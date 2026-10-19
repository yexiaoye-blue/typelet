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
const keySoundGain = 1.8;

function soundUrl(file: string) {
  return new URL('sounds/key-sound/' + encodeURIComponent(file), document.baseURI).href;
}

let context: AudioContext | undefined;
let gain: GainNode | undefined;
let compressor: DynamicsCompressorNode | undefined;
let source: AudioBufferSourceNode | undefined;
let loadedPack = '';
let buffer: AudioBuffer | undefined;
let loading: Promise<void> | undefined;
let loadingPack = '';
let loadGeneration = 0;
let appliedVolume = -1;
let queued = false;
let queuedPack = '';
let queuedVolume = 0;

function audioContext() {
  if (!context) {
    context = new AudioContext();
    gain = context.createGain();
    compressor = context.createDynamicsCompressor();
    compressor.threshold.value = -12;
    compressor.knee.value = 6;
    compressor.ratio.value = 4;
    compressor.attack.value = 0.003;
    compressor.release.value = 0.08;
    gain.connect(compressor);
    compressor.connect(context.destination);
    context.onstatechange = flushPending;
  }
  return context;
}

function startBuffer(decoded: AudioBuffer, volumePercent: number) {
  if (!context || !gain || context.state !== 'running') return false;
  const previous = source;
  source = undefined;
  if (previous) {
    previous.onended = null;
    try { previous.stop(); } catch { /* The click already finished. */ }
    previous.disconnect();
  }
  const level = volumePercent * 0.01 * keySoundGain;
  if (level !== appliedVolume) {
    appliedVolume = level;
    gain.gain.value = level;
  }
  const next = context.createBufferSource();
  next.buffer = decoded;
  next.connect(gain);
  next.onended = () => { if (source === next) source = undefined; };
  try { next.start(); } catch { next.disconnect(); return false; }
  source = next;
  return true;
}

function flushPending() {
  if (!queued || !buffer || loadedPack !== queuedPack || context?.state !== 'running') return;
  const volume = queuedVolume;
  queued = false;
  startBuffer(buffer, volume);
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
  if (buffer && loadedPack === pack && ctx.state === 'running') {
    queued = false;
    startBuffer(buffer, volumePercent);
    return;
  }
  queued = true;
  queuedPack = pack;
  queuedVolume = volumePercent;
  if (ctx.state === 'suspended') void ctx.resume().then(flushPending);
  void loadPack(pack).then(flushPending).catch(() => { /* A missing pack stays silent. */ });
}
