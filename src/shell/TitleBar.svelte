<script lang="ts">
  import MiniWindowIcon from './MiniWindowIcon.svelte';

  let {
    dragWindow,
    t,
    snapshot,
    view,
    navigateBack,
    openSettings,
    isTauri,
    toggleMiniWindow,
    pinned,
    togglePinned,
    minimize,
    close,
  }: {
    dragWindow: any;
    t: any;
    snapshot: any;
    view: any;
    navigateBack: any;
    openSettings: any;
    isTauri: any;
    toggleMiniWindow: any;
    pinned: any;
    togglePinned: any;
    minimize: any;
    close: any;
  } = $props();
</script>

  <header class="global-nav" role="presentation" onmousedown={dragWindow}>
    <span class="brand"><span class="brand-mark">T</span><span>{t.brand}</span>{#if snapshot.collectionTitle}<span class="brand-context" title={snapshot.collectionTitle}>{snapshot.collectionTitle}</span>{/if}</span>
    <div class="window-actions">
      <button class="settings-button" aria-label={t.settings} title={t.settings} aria-pressed={view === 'settings'} onclick={() => view === 'settings' ? navigateBack() : openSettings()}>
        <svg viewBox="0 0 20 20" aria-hidden="true"><path d="m8 3 .5-1h3l.5 1 .5 1 1 .5 1-.5 2 2-.5 1 .5 1 1 .5 1 .5v3l-1 .5-1 .5-.5 1 .5 1-2 2-1-.5-1 .5-.5 1-.5 1h-3l-.5-1-.5-1-1-.5-1 .5-2-2 .5-1-.5-1-1-.5-1-.5v-3l1-.5 1-.5.5-1-.5-1 2-2 1 .5 1-.5.5-1Z" /><circle cx="10" cy="10" r="2.5" /></svg>
      </button>
      <button type="button" aria-label={t.enterMiniWindow} title={t.enterMiniWindow} aria-keyshortcuts="Control+Shift+M Meta+Shift+M" onclick={() => void toggleMiniWindow()}>
        <MiniWindowIcon />
      </button>
      {#if isTauri}
        <button class:pinned aria-label={pinned ? t.unpin : t.pin} title={pinned ? t.unpin : t.pin} aria-pressed={pinned} onclick={togglePinned}>
          <svg viewBox="0 0 20 20" aria-hidden="true"><path d="M7 3h6l-1 6 3 3H5l3-3-1-6ZM10 12v5" /></svg>
        </button>
        <button aria-label={t.minimize} title={t.minimize} onclick={minimize}>−</button>
        <button aria-label={t.close} title={t.close} onclick={close}>×</button>
      {/if}
    </div>
  </header>
