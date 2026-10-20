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
// A fixed 1.8 gain is only +5 dB, and the old compressor pulled Default
// back to about -10 dBFS, so the boost was not actually heard. Full volume
// now lands each pack near -3 dBFS. Clicks that arrive late, or while the
// output is already behind, are dropped instead of playing as a burst.
const normalClickPeak = 10 ** (-3 / 20);
const maxMakeup = 32;
const maxVoices = 4;
const voiceOverrunMs = 160;

export function keySoundMakeup(peak: number) {
  if (!Number.isFinite(peak) || peak <= 0.00001) return 1;
  return Math.min(maxMakeup, normalClickPeak / peak);
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

type Voice = {
  node: AudioBufferSourceNode;
  gain: GainNode;
  startedAt: number;
  durationMs: number;
};

let voices: Voice[] = [];

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

function detachVoice(voice: Voice) {
  voice.node.onended = null;
  try { voice.node.stop(); } catch { /* The click already finished. */ }
  try { voice.node.disconnect(); } catch { /* The node is already detached. */ }
  try { voice.gain.disconnect(); } catch { /* The node is already detached. */ }
}

function outputBackedUp(now: number) {
  const healthy: Voice[] = [];
  let backedUp = false;
  for (const voice of voices) {
    if (now - voice.startedAt <= voice.durationMs + voiceOverrunMs) {
      healthy.push(voice);
      continue;
    }
    backedUp = true;
    detachVoice(voice);
  }
  voices = healthy;
  return backedUp || voices.length >= maxVoices;
}

function startVoice(decoded: AudioBuffer, volumePercent: number) {
  if (!context || context.state !== 'running' || voices.length >= maxVoices) return false;
  const voiceGain = context.createGain();
  voiceGain.gain.value = volumePercent * 0.01 * packMakeup;
  const node = context.createBufferSource();
  node.buffer = decoded;
  node.connect(voiceGain);
  voiceGain.connect(context.destination);
  const voice: Voice = { node, gain: voiceGain, startedAt: performance.now(), durationMs: decoded.duration * 1000 };
  node.onended = () => {
    const index = voices.findIndex(item => item.node === node);
    if (index >= 0) voices.splice(index, 1);
    try { node.disconnect(); } catch { /* The node is already detached. */ }
    try { voiceGain.disconnect(); } catch { /* The node is already detached. */ }
  };
  try { node.start(); } catch {
    try { node.disconnect(); voiceGain.disconnect(); } catch { /* The node never started. */ }
    return false;
  }
  voices.push(voice);
  return true;
}

function flushPending() {
  if (!queued || !buffer || loadedPack !== queuedPack || context?.state !== 'running') return;
  if (outputBackedUp(performance.now())) {
    queued = false;
    return;
  }
  const volume = queuedVolume;
  queued = false;
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
  if (outputBackedUp(performance.now())) {
    queued = false;
    if (ctx.state === 'suspended') void ctx.resume();
    return;
  }
  if (buffer && loadedPack === pack && ctx.state === 'running') {
    queued = false;
    startVoice(buffer, volumePercent);
    return;
  }
  queued = true;
  queuedPack = pack;
  queuedVolume = volumePercent;
  if (ctx.state === 'suspended') void ctx.resume().then(flushPending);
  void loadPack(pack).then(flushPending).catch(() => { /* A missing pack stays silent. */ });
}
