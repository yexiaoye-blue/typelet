// Key clicks are the files Qwerty Learner ships in public/sounds/key-sound.
// Those recordings come from https://github.com/tplai/kbsim.

// Use PCM WAV copies for reliable decoding in the Tauri WebView; the original
// MP3 files remain in the bundle as the source recordings.
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
  speechVolume: number;
  keyEnabled: boolean;
  keyVolume: number;
  pack: string;
};

const storageKey = 'typelet.sound';
const packIds = new Set(keySoundPacks.map(pack => pack.id));
const defaultPreference: SoundPreference = { speechVolume: 100, keyEnabled: true, keyVolume: 100, pack: 'Default' };

function clampPercent(value: unknown, fallback: number) {
  const number = typeof value === 'number' ? value : Number(value);
  if (!Number.isFinite(number)) return fallback;
  return Math.max(0, Math.min(100, Math.round(number)));
}

export function loadSoundPreference(): SoundPreference {
  try {
    const parsed = JSON.parse(localStorage.getItem(storageKey) ?? '') as Partial<SoundPreference>;
    const pack = typeof parsed.pack === 'string' && packIds.has(parsed.pack) ? parsed.pack : defaultPreference.pack;
    return {
      speechVolume: clampPercent(parsed.speechVolume, defaultPreference.speechVolume),
      keyEnabled: parsed.keyEnabled !== false,
      keyVolume: clampPercent(parsed.keyVolume, defaultPreference.keyVolume),
      pack,
    };
  } catch {
    return { ...defaultPreference };
  }
}

export function persistSoundPreference(value: SoundPreference) {
  try { localStorage.setItem(storageKey, JSON.stringify(value)); } catch { /* Storage may be disabled. */ }
}

const packFiles = new Map(keySoundPacks.map(pack => [pack.id, pack.file]));
// Most packs peak around -21 to -32 dBFS. Default is about -10 dBFS.
// Full volume brings each pack to about -1 dBFS. A click is never stopped
// when the next key arrives; cutting the tail is what made the sound seem
// interrupted and quieter than the recording.
const normalClickPeak = 10 ** (-1 / 20);
const maxMakeup = 32;
const maxVoices = 8;

export function keySoundMakeup(peak: number) {
  if (!Number.isFinite(peak) || peak <= 0.00001) return 1;
  return Math.min(maxMakeup, normalClickPeak / peak);
}

let writeKeySoundLog: Promise<(line: string) => Promise<unknown>> | undefined;

function audioClock() {
  if (!context) return { state: 'none', audioMs: 0, baseMs: 0, outputMs: 0, voices: voices.length };
  const outputLatency = 'outputLatency' in context ? context.outputLatency : 0;
  return {
    state: context.state,
    audioMs: Math.round(context.currentTime * 1000),
    baseMs: Math.round(context.baseLatency * 1000),
    outputMs: Math.round(outputLatency * 1000),
    voices: voices.length,
  };
}

export function traceKeySound(event: string, detail: Record<string, unknown> = {}) {
  const line = JSON.stringify({ t: new Date().toISOString(), wall: Math.round(performance.now()), event, ...audioClock(), ...detail });
  console.info('[typelet-key]', line);
  if (typeof window === 'undefined' || !('__TAURI_INTERNALS__' in window)) return;
  writeKeySoundLog ??= import('@tauri-apps/api/core').then(({ invoke }) => (message: string) => invoke('log_key_sound', { line: message }));
  void writeKeySoundLog.then(write => write(line)).catch(() => { /* Logging must not affect playback. */ });
}

function soundUrl(file: string) {
  return new URL('sounds/key-sound/' + encodeURIComponent(file), document.baseURI).href;
}

let context: AudioContext | undefined;
let loadedPack = '';
let buffer: AudioBuffer | undefined;
let packMakeup = 1;
let loading: Promise<void> | undefined;
let loadingPack = '';
let loadGeneration = 0;
let queued = false;
let queuedPack = '';
let queuedVolume = 0;

let voices: Array<{ node: AudioBufferSourceNode; gain: GainNode }> = [];

function audioContext() {
  if (!context) {
    context = new AudioContext({ latencyHint: 'interactive' });
    context.onstatechange = flushPending;
  }
  return context;
}

function bufferPeak(decoded: AudioBuffer) {
  let peak = 0;
  for (let channel = 0; channel < decoded.numberOfChannels; channel += 1) {
    const samples = decoded.getChannelData(channel);
    for (let index = 0; index < samples.length; index += 1) {
      const value = Math.abs(samples[index]);
      if (value > peak) peak = value;
    }
  }
  return peak;
}

function startVoice(decoded: AudioBuffer, volumePercent: number) {
  const startedWall = performance.now();
  if (!context || context.state !== 'running') {
    traceKeySound('skip-not-running', { volume: volumePercent });
    return false;
  }
  if (voices.length >= maxVoices) {
    traceKeySound('skip-full', { volume: volumePercent });
    return false;
  }
  const voiceGain = context.createGain();
  voiceGain.gain.value = volumePercent * 0.01 * packMakeup;
  const node = context.createBufferSource();
  node.buffer = decoded;
  node.connect(voiceGain);
  voiceGain.connect(context.destination);
  const voice = { node, gain: voiceGain };
  node.onended = () => {
    traceKeySound('ended', { ageMs: Math.round(performance.now() - startedWall), bufferMs: Math.round(decoded.duration * 1000) });
    const index = voices.findIndex(item => item.node === node);
    if (index >= 0) voices.splice(index, 1);
    try { node.disconnect(); } catch { /* The node is already detached. */ }
    try { voiceGain.disconnect(); } catch { /* The node is already detached. */ }
  };
  try { node.start(); } catch {
    try { node.disconnect(); voiceGain.disconnect(); } catch { /* The node never started. */ }
    traceKeySound('start-failed', { bufferMs: Math.round(decoded.duration * 1000) });
    return false;
  }
  traceKeySound('started', { bufferMs: Math.round(decoded.duration * 1000), volume: volumePercent, makeup: Number(packMakeup.toFixed(2)) });
  voices.push(voice);
  return true;
}

function flushPending() {
  if (!queued) return;
  if (!buffer || loadedPack !== queuedPack || context?.state !== 'running') {
    traceKeySound('flush-wait', { hasBuffer: Boolean(buffer), loadedPack, queuedPack });
    return;
  }
  const volume = queuedVolume;
  queued = false;
  traceKeySound('flush', { volume });
  startVoice(buffer, volume);
}

function loadPack(pack: string) {
  if (loadedPack === pack && buffer) return Promise.resolve();
  if (loading && loadingPack === pack) return loading;
  const generation = ++loadGeneration;
  const ctx = audioContext();
  const file = packFiles.get(pack) ?? keySoundPacks[0].file;
  loadingPack = pack;
  const task = (async () => {
    const response = await fetch(soundUrl(file));
    if (!response.ok) throw new Error('Key sound unavailable');
    const decoded = await ctx.decodeAudioData((await response.arrayBuffer()).slice(0));
    if (generation !== loadGeneration) return;
    buffer = decoded;
    packMakeup = keySoundMakeup(bufferPeak(decoded));
    loadedPack = pack;
  })();
  loading = task.finally(() => {
    if (loading === task) loading = undefined;
  });
  return loading;
}

export function preloadKeySound(pack: string) {
  if (typeof AudioContext === 'undefined') return;
  void loadPack(pack).then(flushPending).catch(() => { /* Retry on the next key. */ });
}

export function unlockKeySound() {
  if (typeof AudioContext === 'undefined') return;
  const ctx = audioContext();
  if (ctx.state === 'suspended') void ctx.resume().then(flushPending);
}

export function playKeySound(pack: string, volumePercent: number) {
  if (volumePercent <= 0 || typeof AudioContext === 'undefined') return;
  const ctx = context ?? audioContext();
  const ready = Boolean(buffer && loadedPack === pack);
  traceKeySound('request', { pack, volume: volumePercent, ready });
  if (buffer && loadedPack === pack && ctx.state === 'running') {
    queued = false;
    startVoice(buffer, volumePercent);
    return;
  }
  traceKeySound('queued', { pack, volume: volumePercent, ready });
  queued = true;
  queuedPack = pack;
  queuedVolume = volumePercent;
  if (ctx.state === 'suspended') void ctx.resume().then(flushPending);
  void loadPack(pack).then(flushPending).catch(() => { /* A missing pack stays silent. */ });
}
