import test from 'node:test';
import assert from 'node:assert/strict';
import {
  conflictingActions,
  defaultShortcuts,
  findShortcutConflict,
  loadShortcuts,
  normalizeShortcuts,
  saveShortcuts,
  shortcutAllowed,
  shortcutAriaLabel,
  shortcutFromEvent,
  shortcutLabel,
  shortcutMatches,
  shortcutParts,
  type ShortcutBinding,
} from '../src/shortcuts';

function key(code: string, options: { key?: string; mod?: boolean; shift?: boolean; alt?: boolean; platform?: 'mac' | 'other' } = {}) {
  const platform = options.platform ?? 'other';
  return {
    code,
    key: options.key ?? code,
    ctrlKey: platform === 'other' && !!options.mod,
    metaKey: platform === 'mac' && !!options.mod,
    shiftKey: !!options.shift,
    altKey: !!options.alt,
    repeat: false,
    isComposing: false,
    defaultPrevented: false,
  } as KeyboardEvent;
}

test('default shortcuts keep the existing practice controls', () => {
  const shortcuts = defaultShortcuts();
  assert.deepEqual(shortcutParts(shortcuts.toggleMiniWindow), ['Ctrl', 'Shift', 'M']);
  assert.deepEqual(shortcutParts(shortcuts.previousItem), ['←']);
  assert.deepEqual(shortcutParts(shortcuts.previousChapter), ['Ctrl', '←']);
  assert.equal(conflictingActions(shortcuts).size, 0);
});

test('shortcut labels adapt to macOS without changing stored bindings', () => {
  const shortcuts = defaultShortcuts();
  assert.deepEqual(shortcutParts(shortcuts.toggleMiniWindow, 'mac'), ['⌘', '⇧', 'M']);
  assert.equal(shortcutLabel(shortcuts.toggleMiniWindow, 'mac'), '⌘ ⇧ M');
  assert.equal(shortcutAriaLabel(shortcuts.toggleMiniWindow, 'mac'), 'Meta+Shift+M');
  assert.equal(shortcutAriaLabel(shortcuts.toggleMiniWindow, 'other'), 'Control+Shift+M');
});

test('shortcut matching is strict about modifiers and platform', () => {
  const binding = defaultShortcuts().toggleMiniWindow;
  assert.equal(shortcutMatches(key('KeyM', { key: 'M', mod: true, shift: true }), binding, 'other'), true);
  assert.equal(shortcutMatches(key('KeyM', { key: 'M', mod: true, shift: true, platform: 'mac' }), binding, 'other'), false);
  assert.equal(shortcutMatches(key('KeyM', { key: 'M', mod: true }), binding, 'other'), false);
  assert.equal(shortcutMatches(key('KeyM', { key: 'M', mod: true, shift: true, alt: true }), binding, 'other'), false);
});

test('recording rejects typing keys without a modifier but accepts navigation keys', () => {
  assert.equal(shortcutFromEvent(key('KeyA', { key: 'a' }), 'other'), null);
  assert.equal(shortcutFromEvent(key('Escape', { key: 'Escape' }), 'other'), null);
  assert.equal(shortcutFromEvent(key('Tab', { key: 'Tab' }), 'other'), null);
  assert.deepEqual(shortcutFromEvent(key('ArrowLeft', { key: 'ArrowLeft' }), 'other'), {
    code: 'ArrowLeft', mod: false, shift: false, alt: false,
  });
  assert.deepEqual(shortcutFromEvent(key('KeyP', { key: 'p', mod: true, shift: true }), 'other'), {
    code: 'KeyP', mod: true, shift: true, alt: false,
  });
  assert.deepEqual(shortcutFromEvent(key('F5', { key: 'F5' }), 'other'), {
    code: 'F5', mod: false, shift: false, alt: false,
  });
});

test('conflicts block a shortcut already used by another action', () => {
  const shortcuts = defaultShortcuts();
  const duplicate = shortcuts.previousItem;
  assert.equal(findShortcutConflict(shortcuts, 'nextItem', duplicate), 'previousItem');
  shortcuts.nextItem = duplicate;
  assert.deepEqual(conflictingActions(shortcuts), new Set(['previousItem', 'nextItem']));
});

test('stored shortcuts merge with defaults and reject malformed entries', () => {
  const incomplete = { openSettings: { code: 'KeyO', mod: true, shift: true, alt: false }, previousItem: { code: 7 } };
  const shortcuts = normalizeShortcuts(incomplete);
  assert.deepEqual(shortcuts.openSettings, { code: 'KeyO', mod: true, shift: true, alt: false });
  assert.deepEqual(shortcuts.previousItem, defaultShortcuts().previousItem);
  assert.deepEqual(normalizeShortcuts(null), defaultShortcuts());
});

test('shortcut preferences survive storage and fall back safely', () => {
  const store = new Map<string, string>();
  const storage = {
    getItem: (key: string) => store.get(key) ?? null,
    setItem: (key: string, value: string) => { store.set(key, value); },
  };
  const shortcuts = defaultShortcuts();
  shortcuts.openSettings = { code: 'KeyK', mod: true, shift: false, alt: false };
  saveShortcuts(shortcuts, storage);
  assert.deepEqual(loadShortcuts(storage), shortcuts);
  assert.deepEqual(loadShortcuts(undefined), defaultShortcuts());
  store.set('typelet.shortcuts', '{broken');
  assert.deepEqual(loadShortcuts(storage), defaultShortcuts());
});

test('a default shortcut can be restored without altering other bindings', () => {
  const shortcuts = defaultShortcuts();
  const original = shortcuts.previousItem;
  shortcuts.previousItem = { code: 'KeyB', mod: true, shift: false, alt: false } as ShortcutBinding;
  const restored = { ...shortcuts, previousItem: original };
  assert.deepEqual(shortcutParts(restored.previousItem), ['←']);
  assert.deepEqual(shortcutParts(restored.previousItem), shortcutParts(defaultShortcuts().previousItem));
});

test('global mode actions require a command modifier', () => {
  assert.equal(shortcutAllowed('openSettings', { code: 'F2', mod: false, shift: false, alt: false }), false);
  assert.equal(shortcutAllowed('toggleMiniWindow', { code: 'KeyM', mod: false, shift: true, alt: false }), false);
  assert.equal(shortcutAllowed('openSettings', { code: 'KeyO', mod: true, shift: true, alt: false }), true);
  assert.equal(shortcutAllowed('previousItem', { code: 'F2', mod: false, shift: false, alt: false }), true);
});
