<script lang="ts">
  import type { Messages } from '../i18n';
  import MiniWindowIcon from './MiniWindowIcon.svelte';

  let {
    t, isTauri, pinned, dragWindow, togglePinned, minimize, close, restoreWindow,
  }: {
    t: Messages;
    isTauri: boolean;
    pinned: boolean;
    dragWindow: (event: MouseEvent) => void | Promise<void>;
    togglePinned: () => void | Promise<void>;
    minimize: () => void | Promise<void>;
    close: () => void | Promise<void>;
    restoreWindow: () => void | Promise<void>;
  } = $props();
</script>

<header class="mini-toolbar" role="presentation" onmousedown={dragWindow}>
  <div class="mini-toolbar-group">
    {#if isTauri}
      <button class="mini-toolbar-button mini-toolbar-aux" type="button" aria-label={t.close} title={t.close} onclick={() => void close()}>
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="m8 8 8 8m0-8-8 8" /></svg>
      </button>
      <button class="mini-toolbar-button mini-toolbar-aux" type="button" aria-label={t.minimize} title={t.minimize} onclick={() => void minimize()}>
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M6 12h12" /></svg>
      </button>
    {/if}
  </div>
  <div class="mini-toolbar-group">
    {#if isTauri}
      <button class="mini-toolbar-button mini-toolbar-aux" class:pinned type="button" aria-label={pinned ? t.unpin : t.pin} title={pinned ? t.unpin : t.pin} aria-pressed={pinned} onclick={() => void togglePinned()}>
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M8 4h8l-1.5 7 3.5 4H6l3.5-4L8 4Zm4 11v5" /></svg>
      </button>
    {/if}
    <button class="mini-toolbar-button restore-window" type="button" aria-label={t.exitMiniWindow} title={t.exitMiniWindow} aria-keyshortcuts="Control+Shift+M Meta+Shift+M" onclick={() => void restoreWindow()}>
      <MiniWindowIcon restore />
    </button>
  </div>
</header>
