export type ShortcutAction =
  | 'openSettings'
  | 'toggleMiniWindow'
  | 'previousItem'
  | 'nextItem'
  | 'previousChapter'
  | 'nextChapter';

export type ShortcutBinding = {
  code: string;
  mod: boolean;
  shift: boolean;
  alt: boolean;
};

export type Shortcuts = Record<ShortcutAction, ShortcutBinding>;
export type ShortcutPlatform = 'mac' | 'other';

export const shortcutActions: ShortcutAction[] = [
  'openSettings',
  'toggleMiniWindow',
  'previousItem',
  'nextItem',
  'previousChapter',
  'nextChapter',
];

const STORAGE_KEY = 'typelet.shortcuts';
const MODIFIER_KEYS = new Set(['Shift', 'Control', 'Alt', 'Meta', 'CapsLock', 'NumLock', 'ScrollLock']);
const MODIFIER_CODES = /^(Shift|Control|Alt|Meta)(Left|Right)$/;
const PLAIN_NAVIGATION_CODES = /^(Arrow(Up|Down|Left|Right)|Home|End|PageUp|PageDown|F(?:[1-9]|1[0-2]))$/;

type ReadableStorage = Pick<Storage, 'getItem'>;
type WritableStorage = Pick<Storage, 'setItem'>;

function binding(code: string, mod = false, shift = false, alt = false): ShortcutBinding {
  return { code, mod, shift, alt };
}

export function defaultShortcuts(): Shortcuts {
  return {
    openSettings: binding('Comma', true),
    toggleMiniWindow: binding('KeyM', true, true),
    previousItem: binding('ArrowLeft'),
    nextItem: binding('ArrowRight'),
    previousChapter: binding('ArrowLeft', true),
    nextChapter: binding('ArrowRight', true),
  };
}

export function detectPlatform(): ShortcutPlatform {
  if (typeof navigator === 'undefined') return 'other';
  const platform = (navigator as Navigator & { userAgentData?: { platform?: string } }).userAgentData?.platform || navigator.platform;
  return /mac|iphone|ipad|ipod/i.test(platform) || /Macintosh/i.test(navigator.userAgent) ? 'mac' : 'other';
}

function isBinding(value: unknown): value is ShortcutBinding {
  if (!value || typeof value !== 'object') return false;
  const candidate = value as Record<string, unknown>;
  return typeof candidate.code === 'string'
    && candidate.code.length > 0
    && typeof candidate.mod === 'boolean'
    && typeof candidate.shift === 'boolean'
    && typeof candidate.alt === 'boolean';
}

export function normalizeShortcuts(value: unknown): Shortcuts {
  const defaults = defaultShortcuts();
  if (!value || typeof value !== 'object') return defaults;
  const source = value as Record<string, unknown>;
  const next = { ...defaults };
  for (const action of shortcutActions) {
    if (isBinding(source[action])) next[action] = { ...source[action] };
  }
  return next;
}

function browserStorage(): Storage | undefined {
  try {
    return typeof localStorage === 'undefined' ? undefined : localStorage;
  } catch {
    return undefined;
  }
}

export function loadShortcuts(storage: ReadableStorage | undefined = browserStorage()): Shortcuts {
  try {
    return normalizeShortcuts(JSON.parse(storage?.getItem(STORAGE_KEY) ?? 'null'));
  } catch {
    return defaultShortcuts();
  }
}

export function saveShortcuts(shortcuts: Shortcuts, storage: WritableStorage | undefined = browserStorage()) {
  try {
    storage?.setItem(STORAGE_KEY, JSON.stringify(shortcuts));
  } catch {
    // Browser storage may be unavailable in a locked-down webview.
  }
}

export function shortcutSignature(binding: ShortcutBinding): string {
  return [binding.code, binding.mod ? 1 : 0, binding.shift ? 1 : 0, binding.alt ? 1 : 0].join('|');
}

export function shortcutsEqual(left: ShortcutBinding, right: ShortcutBinding): boolean {
  return shortcutSignature(left) === shortcutSignature(right);
}

export function findShortcutConflict(shortcuts: Shortcuts, action: ShortcutAction, candidate: ShortcutBinding): ShortcutAction | null {
  const signature = shortcutSignature(candidate);
  for (const other of shortcutActions) {
    if (other !== action && shortcutSignature(shortcuts[other]) === signature) return other;
  }
  return null;
}

export function shortcutAllowed(action: ShortcutAction, binding: ShortcutBinding): boolean {
  // Mode changes require a modifier to avoid accidental activation during practice.
  if (action === 'openSettings' || action === 'toggleMiniWindow') return binding.mod || binding.alt;
  return true;
}

export function conflictingActions(shortcuts: Shortcuts): Set<ShortcutAction> {
  const seen = new Map<string, ShortcutAction>();
  const conflicts = new Set<ShortcutAction>();
  for (const action of shortcutActions) {
    const signature = shortcutSignature(shortcuts[action]);
    const previous = seen.get(signature);
    if (previous) {
      conflicts.add(previous);
      conflicts.add(action);
    } else {
      seen.set(signature, action);
    }
  }
  return conflicts;
}

export function shortcutMatches(event: KeyboardEvent, binding: ShortcutBinding, platform: ShortcutPlatform = detectPlatform()): boolean {
  if (event.repeat || event.isComposing || event.defaultPrevented) return false;
  const modPressed = platform === 'mac' ? event.metaKey : event.ctrlKey;
  const foreignModPressed = platform === 'mac' ? event.ctrlKey : event.metaKey;
  return !foreignModPressed
    && binding.mod === modPressed
    && binding.shift === event.shiftKey
    && binding.alt === event.altKey
    && binding.code === event.code;
}

export function shortcutFromEvent(event: KeyboardEvent, platform: ShortcutPlatform = detectPlatform()): ShortcutBinding | null {
  if (event.repeat || event.isComposing || event.defaultPrevented) return null;
  if (!event.code) return null;
  if (MODIFIER_KEYS.has(event.key) || MODIFIER_CODES.test(event.code) || event.code === 'Escape' || event.code === 'Tab') return null;
  const mod = platform === 'mac' ? event.metaKey : event.ctrlKey;
  const foreignMod = platform === 'mac' ? event.ctrlKey : event.metaKey;
  if (foreignMod) return null;
  if (!mod && !event.altKey && !PLAIN_NAVIGATION_CODES.test(event.code)) return null;
  return binding(event.code, mod, event.shiftKey, event.altKey);
}

export function shortcutParts(binding: ShortcutBinding, platform: ShortcutPlatform = detectPlatform()): string[] {
  const parts: string[] = [];
  if (binding.mod) parts.push(platform === 'mac' ? '⌘' : 'Ctrl');
  if (binding.shift) parts.push(platform === 'mac' ? '⇧' : 'Shift');
  if (binding.alt) parts.push(platform === 'mac' ? '⌥' : 'Alt');
  parts.push(shortcutKeyLabel(binding.code));
  return parts;
}

export function shortcutLabel(binding: ShortcutBinding, platform: ShortcutPlatform = detectPlatform()): string {
  return shortcutParts(binding, platform).join(' ');
}

export function shortcutAriaLabel(binding: ShortcutBinding, platform: ShortcutPlatform = detectPlatform()): string {
  const parts: string[] = [];
  if (binding.mod) parts.push(platform === 'mac' ? 'Meta' : 'Control');
  if (binding.shift) parts.push('Shift');
  if (binding.alt) parts.push('Alt');
  parts.push(shortcutAriaKeyName(binding.code));
  return parts.join('+');
}

function shortcutKeyLabel(code: string): string {
  const labels: Record<string, string> = {
    ArrowUp: '↑',
    ArrowDown: '↓',
    ArrowLeft: '←',
    ArrowRight: '→',
    Comma: ',',
    Period: '.',
    Slash: '/',
    Semicolon: ';',
    Quote: "'",
    BracketLeft: '[',
    BracketRight: ']',
    Backslash: '\\',
    Minus: '-',
    Equal: '=',
    Backquote: '`',
    Space: 'Space',
    Enter: 'Enter',
    Backspace: '⌫',
    Delete: 'Del',
    Home: 'Home',
    End: 'End',
    PageUp: 'PgUp',
    PageDown: 'PgDn',
  };
  if (labels[code]) return labels[code];
  if (code.startsWith('Key')) return code.slice(3);
  if (code.startsWith('Digit')) return code.slice(5);
  return code;
}

function shortcutAriaKeyName(code: string): string {
  const names: Record<string, string> = {
    ArrowUp: 'ArrowUp',
    ArrowDown: 'ArrowDown',
    ArrowLeft: 'ArrowLeft',
    ArrowRight: 'ArrowRight',
    Comma: ',',
    Period: '.',
    Slash: '/',
    Semicolon: ';',
    Quote: "'",
    BracketLeft: '[',
    BracketRight: ']',
    Backslash: '\\',
    Minus: '-',
    Equal: '=',
    Backquote: '`',
    Space: 'Space',
    Enter: 'Enter',
    Backspace: 'Backspace',
    Delete: 'Delete',
  };
  return names[code] ?? shortcutKeyLabel(code);
}
