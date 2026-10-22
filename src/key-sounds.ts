// Key clicks are the recordings Qwerty Learner ships, originally from
// https://github.com/tplai/kbsim. The desktop app does not play them through
// WebKit: that stream can sit in a PipeWire buffer for seconds. The Rust
// process owns one low-latency output, and each key adds a voice that plays
// to the end. The browser preview has no native stream, so it uses one-shot
// Web Audio voices instead. Word pronunciation stays on its own player.

export type KeySoundPack = { id: string; file: string };

export const keySoundPacks: KeySoundPack[] = [
  { id: 'Default', file: 'Default.wav' },
  { id: 'Alpacas', file: 'Alpacas.wav' },
  { id: 'Buckling Spring', file: 'Buckling Spring.wav' },
  { id: 'Cherry MX Blacks', file: 'Cherry MX Blacks.wav' },
  { id: 'Cherry MX Blues', file: 'Cherry MX Blues.wav' },
  { id: 'Cherry MX Browns', file: 'Cherry MX Browns.wav' },
  { id: 'Gateron Black Inks', file: 'Gateron Black Inks.wav' },
  { id: 'Gateron Red Inks', file: 'Gateron Red Inks.wav' },
  { id: 'Holy Pandas', file: 'Holy Pandas.wav' },
  { id: 'Kailh Box Navies', file: 'Kailh Box Navies.wav' },
  { id: 'NovelKeys Creams', file: 'NovelKeys Creams.wav' },
  { id: 'SKCM Blue Alps', file: 'SKCM Blue Alps.wav' },
  { id: 'Topre', file: 'Topre.wav' },
  { id: 'Turquoise Tealios', file: 'Turquoise Tealios.wav' },
];

export type SoundPreference = {
  enabled: boolean;
  speechMuted: boolean;
  speechVolume: number;
  keyMuted: boolean;
  keyVolume: number;
  pack: string;
};

const storageKey = 'typelet.sound';
const packIds = new Set(keySoundPacks.map(pack => pack.id));
const defaultPreference: SoundPreference = { enabled: true, speechMuted: false, speechVolume: 100, keyMuted: false, keyVolume: 100, pack: 'Default' };

function clampPercent(value: unknown, fallback: number) {
  const number = typeof value === 'number' ? value : Number(value);
  if (!Number.isFinite(number)) return fallback;
  return Math.max(0, Math.min(100, Math.round(number)));
}

type StoredSound = Partial<SoundPreference> & { keyEnabled?: boolean };

export function soundPreferenceFrom(value: unknown): SoundPreference {
  const parsed = value && typeof value === 'object' ? value as StoredSound : {};
  const pack = typeof parsed.pack === 'string' && packIds.has(parsed.pack) ? parsed.pack : defaultPreference.pack;
  const keyMuted = typeof parsed.keyMuted === 'boolean' ? parsed.keyMuted : parsed.keyEnabled === false;
  return {
    enabled: parsed.enabled !== false,
    speechMuted: parsed.speechMuted === true,
    speechVolume: clampPercent(parsed.speechVolume, defaultPreference.speechVolume),
    keyMuted,
    keyVolume: clampPercent(parsed.keyVolume, defaultPreference.keyVolume),
    pack,
  };
}

export function loadSoundPreference(): SoundPreference {
  try {
    return soundPreferenceFrom(JSON.parse(localStorage.getItem(storageKey) ?? ''));
  } catch {
    return { ...defaultPreference };
  }
}

export function readingAudible(sound: SoundPreference) {
  return sound.enabled && !sound.speechMuted && sound.speechVolume > 0;
}

export function keysAudible(sound: SoundPreference) {
  return sound.enabled && !sound.keyMuted && sound.keyVolume > 0;
}

export function persistSoundPreference(value: SoundPreference) {
  try { localStorage.setItem(storageKey, JSON.stringify(value)); } catch { /* Storage may be disabled. */ }
}

const packFiles = new Map(keySoundPacks.map(pack => [pack.id, pack.file]));
// Most packs peak around -21 to -32 dBFS. Default is about -10 dBFS.
// Full volume brings each pack to about -1 dBFS. Voices are never stopped
// early; cutting the tail is what made the click sound interrupted.
const normalClickPeak = 10 ** (-1 / 20);
const maxMakeup = 32;
const maxVoices = 32;

export function keySoundMakeup(peak: number) {
  if (!Number.isFinite(peak) || peak <= 0.00001) return 1;
  return Math.min(maxMakeup, normalClickPeak / peak);
}

let writeKeySoundLog: Promise<(line: string) => Promise<unknown>> | undefined;
let nativeBridge: Promise<NativeBridge> | undefined;
let nativeBroken = false;
let nativeReady = false;
let nativePack = '';
let context: AudioContext | undefined;
let webBuffer: AudioBuffer | undefined;
let webPack = '';
let webMakeup = 1;
let loading: Promise<void> | undefined;
let loadingPack = '';
let loadGeneration = 0;
let queued = false;
let queuedPack = '';
let queuedVolume = 0;
let queuedRequest = 0;
let queuedAt = 0;
let nextRequest = 0;
let unlocked = false;
const activeSources = new Set<AudioBufferSourceNode>();

type NativeLoad = {
  frames: number;
  rate: number;
  deviceRate: number;
  channels: number;
  makeup: number;
  bufferFrames: number;
};
type NativePlay = { voice: number };
type NativeBridge = {
  load: (bytes: number[]) => Promise<NativeLoad>;
  play: (volume: number, ipcMs: number, request: number) => Promise<NativePlay>;
};

function desktop() {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

function usesNative() {
  return desktop() && !nativeBroken;
}

function traceKeySound(event: string, detail: Record<string, unknown> = {}) {
  const line = JSON.stringify({
    t: new Date().toISOString(),
    wall: typeof performance === 'undefined' ? 0 : Math.round(performance.now()),
    event,
    path: usesNative() ? 'native' : 'webaudio',
    ...detail,
  });
  console.info('[typelet-key]', line);
  if (!desktop()) return;
  writeKeySoundLog ??= import('@tauri-apps/api/core').then(({ invoke }) => (message: string) => invoke('log_key_sound', { line: message }));
  void writeKeySoundLog.then(write => write(line)).catch(() => { /* Logging must not affect playback. */ });
}

export { traceKeySound };

function errorText(error: unknown) {
  if (typeof error === 'string') return error;
  if (error && typeof error === 'object' && 'message' in error) return String((error as { message: unknown }).message);
  return String(error);
}

function bridge() {
  nativeBridge ??= import('@tauri-apps/api/core').then(({ invoke }) => ({
    load: (bytes: number[]) => invoke<NativeLoad>('load_key_sound', { bytes }),
    play: (volume: number, ipcMs: number, request: number) => invoke<NativePlay>('play_key_sound', { volume, ipcMs, request }),
  }));
  return nativeBridge;
}

function soundUrl(file: string) {
  return new URL('sounds/key-sound/' + encodeURIComponent(file), document.baseURI).href;
}

type Clip = { channels: number; rate: number; samples: Float32Array };

function decodeWav(bytes: ArrayBuffer): Clip {
  const view = new DataView(bytes);
  if (view.byteLength < 44 || view.getUint32(0, false) !== 0x52494646) throw new Error('Key sound is not a WAV');
  let offset = 12;
  let format = 1;
  let channels = 1;
  let rate = 48000;
  let bits = 16;
  let dataOffset = 0;
  let dataSize = 0;
  while (offset + 8 <= view.byteLength) {
    const id = view.getUint32(offset, false);
    const size = view.getUint32(offset + 4, true);
    const start = offset + 8;
    if (id === 0x666d7420) {
      format = view.getUint16(start, true);
      channels = view.getUint16(start + 2, true);
      rate = view.getUint32(start + 4, true);
      bits = view.getUint16(start + 14, true);
    } else if (id === 0x64617461) {
      dataOffset = start;
      dataSize = size;
      break;
    }
    offset = start + size + (size & 1);
  }
  if (!dataSize || channels < 1) throw new Error('Key sound WAV is empty');
  const count = Math.floor(dataSize / (bits / 8));
  const samples = new Float32Array(count);
  if (format === 1 && bits === 16) {
    for (let index = 0; index < count; index += 1) samples[index] = view.getInt16(dataOffset + index * 2, true) / 32768;
  } else if (format === 3 && bits === 32) {
    for (let index = 0; index < count; index += 1) samples[index] = view.getFloat32(dataOffset + index * 4, true);
  } else {
    throw new Error('Unsupported key sound WAV');
  }
  return { channels, rate, samples };
}

function samplePeak(samples: Float32Array) {
  let peak = 0;
  for (let index = 0; index < samples.length; index += 1) peak = Math.max(peak, Math.abs(samples[index]));
  return peak;
}

function toMono(clip: Clip): Clip {
  if (clip.channels === 1) return clip;
  const frames = Math.floor(clip.samples.length / clip.channels);
  const mono = new Float32Array(frames);
  for (let frame = 0; frame < frames; frame += 1) {
    let sum = 0;
    for (let channel = 0; channel < clip.channels; channel += 1) sum += clip.samples[frame * clip.channels + channel];
    mono[frame] = sum / clip.channels;
  }
  return { channels: 1, rate: clip.rate, samples: mono };
}

function audioContext() {
  if (context) return context;
  const WebkitContext = (window as Window & { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
  const Ctor = window.AudioContext ?? WebkitContext;
  if (!Ctor) throw new Error('Web Audio is unavailable');
  context = new Ctor();
  return context;
}

function wavBuffer(bytes: Uint8Array) {
  const copy = new ArrayBuffer(bytes.byteLength);
  new Uint8Array(copy).set(bytes);
  return copy;
}

function rememberWeb(pack: string, bytes: Uint8Array) {
  const clip = toMono(decodeWav(wavBuffer(bytes)));
  const ctx = audioContext();
  const buffer = ctx.createBuffer(1, clip.samples.length, clip.rate);
  const channel = new Float32Array(clip.samples.length);
  channel.set(clip.samples);
  buffer.copyToChannel(channel as Float32Array<ArrayBuffer>, 0);
  webMakeup = keySoundMakeup(samplePeak(clip.samples));
  webBuffer = buffer;
  webPack = pack;
  traceKeySound('loaded', {
    pack,
    path: 'webaudio',
    bufferMs: Math.round(buffer.duration * 1000),
    channels: 1,
    makeup: Number(webMakeup.toFixed(2)),
  });
}

async function loadBackend(pack: string, generation: number) {
  const file = packFiles.get(pack) ?? keySoundPacks[0].file;
  const response = await fetch(soundUrl(file));
  if (!response.ok) throw new Error('Key sound unavailable');
  const bytes = new Uint8Array(await response.arrayBuffer());
  if (generation !== loadGeneration) return;
  if (desktop() && !nativeBroken) {
    try {
      const info = await (await bridge()).load(Array.from(bytes));
      if (generation !== loadGeneration) return;
      nativeReady = true;
      nativePack = pack;
      traceKeySound('loaded', {
        pack,
        path: 'native',
        frames: info.frames,
        rate: info.rate,
        deviceRate: info.deviceRate,
        makeup: Number(info.makeup.toFixed(2)),
        bufferFrames: info.bufferFrames,
      });
      return;
    } catch (error) {
      nativeBroken = true;
      nativeReady = false;
      traceKeySound('native-fallback', { pack, message: errorText(error) });
    }
  }
  if (generation !== loadGeneration) return;
  rememberWeb(pack, bytes);
}

function loadPack(pack: string) {
  const ready = usesNative() ? nativeReady && nativePack === pack : Boolean(webBuffer) && webPack === pack;
  if (ready) return Promise.resolve();
  if (loading && loadingPack === pack) return loading;
  const generation = ++loadGeneration;
  loadingPack = pack;
  const task = loadBackend(pack, generation);
  loading = task.finally(() => {
    if (loading === task) loading = undefined;
  });
  return loading;
}

function flushPending() {
  if (!queued) return;
  const native = usesNative();
  const ready = native ? nativeReady && nativePack === queuedPack : Boolean(webBuffer) && webPack === queuedPack;
  if (!ready) {
    traceKeySound('flush-wait', { loadedPack: native ? nativePack : webPack, queuedPack });
    return;
  }
  const volume = queuedVolume;
  const request = queuedRequest;
  const requestedAt = queuedAt;
  queued = false;
  traceKeySound('flush', { request, volume });
  if (native) void scheduleNative(volume, requestedAt, request);
  else startWeb(volume, request);
}

function scheduleNative(volumePercent: number, requestedAt: number, request: number) {
  const ipcMs = Math.max(0, Math.round(performance.now() - requestedAt));
  void bridge().then(api => api.play(volumePercent, ipcMs, request)).then(result => {
    traceKeySound('scheduled', { request, voice: result.voice, ipcMs: Math.round(performance.now() - requestedAt) });
  }).catch(error => {
    const message = errorText(error);
    traceKeySound('start-failed', { request, message });
    if (message.includes('not-ready') || message.includes('queue-full')) return;
    nativeBroken = true;
    nativeReady = false;
    traceKeySound('native-fallback', { request, message });
    const pack = nativePack || queuedPack || keySoundPacks[0].id;
    void loadPack(pack).then(() => startWeb(volumePercent, request)).catch(() => { /* The web fallback logs its own failure. */ });
  });
}

function startWeb(volumePercent: number, request: number) {
  if (!webBuffer) {
    traceKeySound('skip-not-ready', { request, path: 'webaudio' });
    return;
  }
  let ctx: AudioContext;
  try {
    ctx = audioContext();
  } catch (error) {
    traceKeySound('start-failed', { request, path: 'webaudio', message: errorText(error) });
    return;
  }
  if (ctx.state === 'suspended') void ctx.resume();
  if (activeSources.size >= maxVoices) {
    traceKeySound('skip-full', { request, path: 'webaudio' });
    return;
  }
  const source = ctx.createBufferSource();
  source.buffer = webBuffer;
  const gain = ctx.createGain();
  gain.gain.value = Math.max(0, Math.min(1, volumePercent / 100)) * webMakeup;
  source.connect(gain);
  gain.connect(ctx.destination);
  const started = performance.now();
  activeSources.add(source);
  source.onended = () => {
    activeSources.delete(source);
    traceKeySound('ended', { request, path: 'webaudio', ageMs: Math.round(performance.now() - started) });
  };
  traceKeySound('started', {
    request,
    path: 'webaudio',
    bufferMs: Math.round(webBuffer.duration * 1000),
    makeup: Number(webMakeup.toFixed(2)),
    gain: Number(gain.gain.value.toFixed(2)),
  });
  source.start();
}

export function preloadKeySound(pack: string) {
  void loadPack(pack).then(flushPending).catch(() => { traceKeySound('load-failed', { pack }); });
}

export function unlockKeySound() {
  if (unlocked) return;
  unlocked = true;
  traceKeySound('unlock');
  if (context?.state === 'suspended') void context.resume();
}

export function playKeySound(pack: string, volumePercent: number) {
  if (volumePercent <= 0) return;
  const request = ++nextRequest;
  const requestedAt = performance.now();
  const native = usesNative();
  const ready = native ? nativeReady && nativePack === pack : Boolean(webBuffer) && webPack === pack;
  traceKeySound('request', { request, pack, volume: volumePercent, ready });
  if (ready) {
    queued = false;
    if (native) scheduleNative(volumePercent, requestedAt, request);
    else startWeb(volumePercent, request);
    return;
  }
  traceKeySound('queued', { request, pack, volume: volumePercent, ready });
  queued = true;
  queuedPack = pack;
  queuedVolume = volumePercent;
  queuedRequest = request;
  queuedAt = requestedAt;
  void loadPack(pack).then(flushPending).catch(() => { traceKeySound('load-failed', { pack }); });
}
