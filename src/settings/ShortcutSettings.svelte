<script lang="ts">
  import { onDestroy, tick } from 'svelte';
  import type { Messages } from '../i18n';
  import {
    conflictingActions,
    defaultShortcuts,
    detectPlatform,
    findShortcutConflict,
    shortcutActions,
    shortcutAllowed,
    shortcutFromEvent,
    shortcutLabel,
    shortcutParts,
    shortcutsEqual,
    type ShortcutAction,
    type ShortcutBinding,
    type Shortcuts,
  } from '../shortcuts';

  let {
    t,
    bindings,
    onChange,
    onRecordingChange,
  }: {
    t: Messages;
    bindings: Shortcuts;
    onChange: (next: Shortcuts) => void;
    onRecordingChange: (recording: boolean) => void;
  } = $props();

  const defaults = defaultShortcuts();
  const platform = detectPlatform();
  let query = $state('');
  let recording = $state<ShortcutAction | null>(null);
  let error = $state('');
  let announcement = $state('');

  let rows = $derived([
    { action: 'openSettings' as const, title: t.shortcutOpenSettings, description: t.shortcutOpenSettingsHint },
    { action: 'toggleMiniWindow' as const, title: t.shortcutToggleMiniWindow, description: t.shortcutToggleMiniWindowHint },
    { action: 'previousItem' as const, title: t.shortcutPreviousItem, description: t.shortcutPreviousItemHint },
    { action: 'nextItem' as const, title: t.shortcutNextItem, description: t.shortcutNextItemHint },
    { action: 'previousChapter' as const, title: t.shortcutPreviousChapter, description: t.shortcutPreviousChapterHint },
    { action: 'nextChapter' as const, title: t.shortcutNextChapter, description: t.shortcutNextChapterHint },
  ]);
  let visibleRows = $derived.by(() => {
    const needle = query.trim().toLocaleLowerCase();
    if (!needle) return rows;
    return rows.filter(row =>
      row.title.toLocaleLowerCase().includes(needle)
      || row.description.toLocaleLowerCase().includes(needle)
      || shortcutLabel(bindings[row.action], platform).toLocaleLowerCase().includes(needle));
  });
  let conflicts = $derived(conflictingActions(bindings));
  let customized = $derived(shortcutActions.some(action => !shortcutsEqual(bindings[action], defaults[action])));

  onDestroy(() => onRecordingChange(false));

  function buttonId(action: ShortcutAction) {
    return 'shortcut-' + action;
  }

  function titleFor(action: ShortcutAction) {
    return rows.find(row => row.action === action)?.title ?? action;
  }

  function focusButton(action: ShortcutAction) {
    void tick().then(() => document.getElementById(buttonId(action))?.focus({ preventScroll: true }));
  }

  function cancelRecording(restoreFocus = true) {
    const action = recording;
    recording = null;
    error = '';
    onRecordingChange(false);
    if (restoreFocus && action) focusButton(action);
  }

  function startRecording(action: ShortcutAction) {
    if (recording === action) {
      cancelRecording();
      return;
    }
    recording = action;
    error = '';
    announcement = '';
    onRecordingChange(true);
    focusButton(action);
  }

  function finishRecording(action: ShortcutAction, binding: ShortcutBinding) {
    if (!shortcutAllowed(action, binding)) {
      error = t.shortcutNeedsModifier;
      return;
    }
    const conflict = findShortcutConflict(bindings, action, binding);
    if (conflict) {
      error = t.shortcutConflict + ': ' + titleFor(conflict);
      return;
    }
    onChange({ ...bindings, [action]: binding });
    recording = null;
    error = '';
    announcement = t.shortcutUpdated;
    onRecordingChange(false);
    focusButton(action);
  }

  function resetOne(action: ShortcutAction) {
    const conflict = findShortcutConflict(bindings, action, defaults[action]);
    if (conflict) {
      startRecording(action);
      error = t.shortcutConflict + ': ' + titleFor(conflict);
      return;
    }
    cancelRecording();
    onChange({ ...bindings, [action]: defaults[action] });
    announcement = t.shortcutUpdated;
    focusButton(action);
  }

  function resetAll() {
    cancelRecording(false);
    onChange(defaultShortcuts());
    announcement = t.shortcutUpdated;
  }

  function onKeydown(event: KeyboardEvent) {
    if (!recording || event.isComposing) return;
    if (event.code === 'Escape') {
      event.preventDefault();
      event.stopImmediatePropagation();
      cancelRecording();
      return;
    }
    if (event.code === 'Tab') {
      cancelRecording(false);
      return;
    }
    if (['Shift', 'Control', 'Alt', 'Meta', 'CapsLock', 'NumLock', 'ScrollLock'].includes(event.key)
      || /^(Shift|Control|Alt|Meta)(Left|Right)$/.test(event.code)) {
      event.preventDefault();
      event.stopImmediatePropagation();
      return;
    }
    const binding = shortcutFromEvent(event, platform);
    event.preventDefault();
    event.stopImmediatePropagation();
    if (!binding) {
      error = t.shortcutInvalid;
      return;
    }
    finishRecording(recording, binding);
  }
</script>

<svelte:window onkeydowncapture={onKeydown} />

<section class="settings-group shortcut-settings" aria-label={t.shortcuts}>
  <div class="settings-copy"><strong>{t.shortcuts}</strong><span>{t.shortcutsHint}</span></div>
  <div class="shortcut-search-wrap">
    <svg viewBox="0 0 16 16" aria-hidden="true"><circle cx="7" cy="7" r="4.25" /><path d="m10.2 10.2 3.1 3.1" /></svg>
    <input class="shortcut-search" type="search" placeholder={t.shortcutSearch} aria-label={t.shortcutSearch} bind:value={query} onfocus={() => cancelRecording(false)} />
  </div>

  {#if visibleRows.length}
    <div class="shortcut-list">
      {#each visibleRows as row (row.action)}
        <div class="shortcut-row" class:recording={recording === row.action} class:conflicted={conflicts.has(row.action)}>
          <div class="shortcut-copy">
            <strong>{row.title}</strong>
            <span>{row.description}</span>
          </div>
          <div class="shortcut-actions">
            <button
              id={buttonId(row.action)}
              type="button"
              class="shortcut-key"
              class:recording={recording === row.action}
              aria-label={t.shortcutChange + ': ' + row.title + '. ' + shortcutLabel(bindings[row.action], platform)}
              aria-pressed={recording === row.action}
              onclick={() => startRecording(row.action)}
            >
              {#if recording === row.action}
                <span class="shortcut-recording-label">{t.shortcutRecording}</span>
              {:else}
                {#each shortcutParts(bindings[row.action], platform) as part, index (part + index)}
                  <kbd>{part}</kbd>
                {/each}
              {/if}
            </button>
            {#if !shortcutsEqual(bindings[row.action], defaults[row.action])}
              <button type="button" class="shortcut-reset" aria-label={t.shortcutResetOne + ': ' + row.title} title={t.shortcutResetOne} onclick={() => resetOne(row.action)}>
                <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3 6a5 5 0 1 1 0 4M3 2v4h4" /></svg>
              </button>
            {/if}
          </div>
          {#if recording === row.action}
            <p class="shortcut-record-hint">{t.shortcutRecordHint}</p>
            {#if error}<p class="shortcut-error" role="alert">{error}</p>{/if}
            <button type="button" class="shortcut-cancel" onclick={cancelRecording}>{t.shortcutCancel}</button>
          {/if}
        </div>
      {/each}
    </div>
  {:else}
    <p class="shortcut-empty">{t.shortcutNoMatches}</p>
  {/if}

  {#if conflicts.size}
    <p class="shortcut-error" role="alert">{t.shortcutConflicts}</p>
  {/if}
  <div class="shortcut-footer">
    <button type="button" class="shortcut-reset-all" disabled={!customized} onclick={resetAll}>{t.shortcutResetAll}</button>
  </div>
  <p class="shortcut-fixed-hint">{t.shortcutFixedHint}</p>
  <p class="shortcut-announcement" aria-live="polite">{announcement}</p>
</section>

<style>
  .shortcut-search-wrap { position: relative; margin-top: 12px; }
  .shortcut-search-wrap svg { position: absolute; top: 50%; left: 10px; width: 14px; height: 14px; color: var(--muted); fill: none; stroke: currentColor; stroke-width: 1.3; stroke-linecap: round; transform: translateY(-50%); pointer-events: none; }
  .shortcut-search { width: 100%; height: 34px; padding: 0 10px 0 31px; color: var(--ink); border: 1px solid var(--field-border); border-radius: 8px; outline: none; background: var(--canvas); font-size: 12px; }
  .shortcut-search::placeholder { color: var(--muted); }
  .shortcut-search:focus { border-color: var(--focus); box-shadow: 0 0 0 2px color-mix(in srgb, var(--focus) 22%, transparent); }
  .shortcut-list { display: flex; flex-direction: column; margin: 10px -4px 0; }
  .shortcut-row { display: grid; grid-template-columns: minmax(0, 1fr) auto; align-items: center; gap: 6px 8px; padding: 8px 4px; border-radius: 8px; transition: background .15s ease-out; }
  .shortcut-row + .shortcut-row { border-top: 1px solid var(--hairline); border-radius: 0; }
  .shortcut-row:hover { background: color-mix(in srgb, var(--ink) 4%, transparent); }
  .shortcut-row.recording { background: color-mix(in srgb, var(--action) 8%, transparent); }
  .shortcut-copy { display: flex; min-width: 0; flex-direction: column; gap: 3px; }
  .shortcut-copy strong { color: var(--ink); font-size: 12px; font-weight: 600; }
  .shortcut-copy span { color: var(--muted); font-size: 10px; line-height: 1.45; }
  .shortcut-actions { display: flex; align-items: center; gap: 3px; }
  .shortcut-key { display: flex; align-items: center; justify-content: center; gap: 2px; min-height: 29px; padding: 3px 7px; border: 1px solid var(--field-border); border-radius: 6px; outline: none; background: var(--parchment); color: var(--ink); font-size: 11px; font-weight: 600; }
  .shortcut-key:hover { border-color: var(--focus); }
  .shortcut-key:focus-visible { border-color: var(--focus); box-shadow: 0 0 0 2px color-mix(in srgb, var(--focus) 24%, transparent); }
  .shortcut-key.recording { min-width: 112px; color: var(--action); border-color: var(--focus); background: var(--canvas); }
  .shortcut-key kbd { display: inline-flex; align-items: center; justify-content: center; min-width: 17px; height: 19px; padding: 0 4px; border: 1px solid var(--hairline); border-radius: 4px; background: var(--canvas); color: var(--ink); font-family: var(--practice-font); font-size: 10px; font-weight: 600; line-height: 1; }
  .shortcut-recording-label { font-size: 10px; font-weight: 600; white-space: nowrap; }
  .shortcut-reset { display: grid; place-items: center; width: 24px; height: 24px; color: var(--muted); border-radius: 50%; font-size: 14px; }
  .shortcut-reset svg { width: 14px; height: 14px; fill: none; stroke: currentColor; stroke-width: 1.3; stroke-linecap: round; stroke-linejoin: round; }
  .shortcut-reset:hover { color: var(--action); background: color-mix(in srgb, var(--action) 12%, transparent); }
  .shortcut-record-hint, .shortcut-error { grid-column: 1 / -1; margin: 0; font-size: 10px; line-height: 1.4; }
  .shortcut-record-hint { color: var(--muted); }
  .shortcut-error { color: var(--error); }
  .shortcut-cancel { grid-column: 1 / -1; justify-self: end; min-height: 25px; padding: 2px 8px; color: var(--action); border-radius: 6px; font-size: 11px; }
  .shortcut-cancel:hover { background: color-mix(in srgb, var(--action) 10%, transparent); }
  .shortcut-empty { margin: 12px 4px 0; color: var(--muted); font-size: 11px; }
  .shortcut-footer { display: flex; justify-content: flex-end; margin-top: 10px; padding-top: 10px; border-top: 1px solid var(--hairline); }
  .shortcut-reset-all { min-height: 27px; padding: 2px 7px; color: var(--action); border-radius: 6px; font-size: 11px; }
  .shortcut-reset-all:hover:not(:disabled) { background: color-mix(in srgb, var(--action) 10%, transparent); }
  .shortcut-reset-all:disabled { color: var(--muted); cursor: default; }
  .shortcut-fixed-hint { margin: 8px 4px 0; color: var(--muted); font-size: 10px; line-height: 1.45; }
  .shortcut-announcement { position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px; overflow: hidden; clip: rect(0 0 0 0); white-space: nowrap; border: 0; }
  @media (prefers-reduced-motion: reduce) { .shortcut-row { transition: none; } }
</style>
