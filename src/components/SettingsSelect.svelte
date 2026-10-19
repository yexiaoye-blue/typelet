<script module lang="ts">
  let nextSelectId = 0;
</script>

<script lang="ts">
  import { onMount, tick } from 'svelte';

  export type SettingsSelectOption = {
    value: string;
    label: string;
    group?: string;
  };

  type Props = {
    value: string;
    options: SettingsSelectOption[];
    ariaLabel: string;
    disabled?: boolean;
    variant?: 'field' | 'pill';
    onSelect: (value: string) => void;
    onClose?: () => void;
  };

  let { value, options, ariaLabel, disabled = false, variant = 'field', onSelect, onClose }: Props = $props();
  let root = $state<HTMLDivElement>();
  let trigger = $state<HTMLButtonElement>();
  let menu = $state<HTMLDivElement>();
  let open = $state(false);
  let highlighted = $state(-1);
  let menuPosition = $state({ top: 0, left: 0, width: 0, maxHeight: 240 });
  const menuId = 'settings-select-menu-' + ++nextSelectId;

  let selectedIndex = $derived(options.findIndex(option => option.value === value));
  let selected = $derived(options[selectedIndex] ?? options[0]);

  function optionId(index: number) {
    return menuId + '-option-' + index;
  }

  function updatePosition() {
    if (!open || !trigger) return;
    const rect = trigger.getBoundingClientRect();
    const margin = 10;
    const gap = 6;
    const availableBelow = Math.max(0, window.innerHeight - rect.bottom - margin);
    const availableAbove = Math.max(0, rect.top - margin);
    const opensAbove = availableBelow < 180 && availableAbove > availableBelow;
    const maxHeight = Math.max(120, Math.min(240, opensAbove ? availableAbove : availableBelow));
    const menuWidth = variant === 'pill' ? Math.max(rect.width, 220) : rect.width;
    const preferredLeft = variant === 'pill' ? rect.right - menuWidth : rect.left;
    menuPosition = {
      top: opensAbove ? Math.max(margin, rect.top - maxHeight - gap) : rect.bottom + gap,
      left: Math.max(margin, Math.min(preferredLeft, window.innerWidth - menuWidth - margin)),
      width: menuWidth,
      maxHeight,
    };
  }

  async function scrollHighlighted() {
    await tick();
    if (!menu || highlighted < 0) return;
    menu.querySelector<HTMLElement>('[data-index="' + highlighted + '"]')?.scrollIntoView({ block: 'nearest' });
  }

  async function openMenu() {
    if (disabled || !options.length) return;
    open = true;
    highlighted = selectedIndex >= 0 ? selectedIndex : 0;
    await tick();
    updatePosition();
    void scrollHighlighted();
  }

  function closeMenu() {
    const wasOpen = open;
    open = false;
    highlighted = -1;
    if (wasOpen) onClose?.();
  }

  function toggleMenu() {
    if (open) closeMenu();
    else void openMenu();
  }

  function choose(option: SettingsSelectOption) {
    onSelect(option.value);
    closeMenu();
    if (!onClose) void tick().then(() => trigger?.focus({ preventScroll: true }));
  }

  function moveHighlight(offset: number) {
    if (!options.length) return;
    highlighted = (highlighted + offset + options.length) % options.length;
    void scrollHighlighted();
  }

  function setHighlight(index: number) {
    highlighted = index;
  }

  function handleKeydown(event: KeyboardEvent) {
    if (disabled) return;
    if (!open && ['ArrowDown', 'ArrowUp', 'Enter', ' '].includes(event.key)) {
      event.preventDefault();
      void openMenu();
      return;
    }
    if (!open) return;
    if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      closeMenu();
      return;
    }
    if (event.key === 'Tab') {
      closeMenu();
      return;
    }
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      moveHighlight(1);
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      moveHighlight(-1);
    } else if (event.key === 'Home') {
      event.preventDefault();
      highlighted = 0;
      void scrollHighlighted();
    } else if (event.key === 'End') {
      event.preventDefault();
      highlighted = options.length - 1;
      void scrollHighlighted();
    } else if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      const option = options[highlighted];
      if (option) choose(option);
    }
  }

  function handleDocumentPointerdown(event: PointerEvent) {
    if (open && root && !root.contains(event.target as Node)) closeMenu();
  }

  function handleViewportChange() {
    if (open) updatePosition();
  }

  $effect(() => {
    if (disabled && open) closeMenu();
  });

  onMount(() => {
    document.addEventListener('pointerdown', handleDocumentPointerdown, true);
    window.addEventListener('resize', handleViewportChange);
    window.addEventListener('scroll', handleViewportChange, true);
    return () => {
      document.removeEventListener('pointerdown', handleDocumentPointerdown, true);
      window.removeEventListener('resize', handleViewportChange);
      window.removeEventListener('scroll', handleViewportChange, true);
    };
  });
</script>

<div class="settings-select" class:pill={variant === 'pill'} bind:this={root}>
  <button
    bind:this={trigger}
    class="settings-select-trigger"
    type="button"
    disabled={disabled}
    role="combobox"
    aria-label={ariaLabel}
    aria-haspopup="listbox"
    aria-expanded={open}
    aria-controls={menuId}
    aria-activedescendant={open && highlighted >= 0 ? optionId(highlighted) : undefined}
    onclick={toggleMenu}
    onkeydown={handleKeydown}
  >
    <span class="settings-select-value">{selected?.label ?? ''}</span>
    <svg class="settings-select-chevron" viewBox="0 0 16 16" aria-hidden="true"><path d="m4 6 4 4 4-4" /></svg>
  </button>

  {#if open}
    <div
      bind:this={menu}
      id={menuId}
      class="settings-select-menu"
      role="listbox"
      aria-label={ariaLabel}
      style:top={menuPosition.top + 'px'}
      style:left={menuPosition.left + 'px'}
      style:width={menuPosition.width + 'px'}
      style:max-height={menuPosition.maxHeight + 'px'}
    >
      {#each options as option, index (option.value)}
        {#if option.group && (index === 0 || options[index - 1].group !== option.group)}
          <div class="settings-select-group" role="presentation">{option.group}</div>
        {/if}
        <button
          class:highlighted={index === highlighted}
          class:selected={option.value === value}
          class="settings-select-option"
          type="button"
          role="option"
          tabindex="-1"
          data-index={index}
          aria-selected={option.value === value}
          onclick={() => choose(option)}
          onpointermove={() => setHighlight(index)}
        >
          <span>{option.label}</span>
          {#if option.value === value}<svg class="settings-select-check" viewBox="0 0 16 16" aria-hidden="true"><path d="m3 8.5 3 3 7-7" /></svg>{/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .settings-select { position: relative; width: 100%; min-width: 0; }
  .settings-select-trigger { display: flex; align-items: center; justify-content: space-between; gap: 10px; width: 100%; min-height: 36px; padding: 0 10px 0 12px; color: var(--ink); border: 1px solid var(--field-border); border-radius: 11px; background: var(--field); font-size: 12px; text-align: left; transition: border-color .16s ease, box-shadow .16s ease, background .16s ease; }
  .settings-select-trigger:hover { border-color: color-mix(in srgb, var(--action) 55%, var(--field-border)); }
  .settings-select-trigger:focus-visible, .settings-select-trigger[aria-expanded='true'] { outline: 2px solid color-mix(in srgb, var(--focus) 35%, transparent); outline-offset: 2px; border-color: var(--focus); }
  .settings-select-trigger:disabled { cursor: default; opacity: .45; }
  .settings-select-value { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .settings-select-chevron { flex: none; width: 14px; height: 14px; color: var(--muted); fill: none; stroke: currentColor; stroke-linecap: round; stroke-linejoin: round; stroke-width: 1.6; transition: transform .16s ease; }
  .settings-select-trigger[aria-expanded='true'] .settings-select-chevron { transform: rotate(180deg); }
  .settings-select-menu { position: fixed; z-index: 100; display: flex; flex-direction: column; gap: 2px; min-width: 180px; overflow: auto; padding: 6px; border: 1px solid var(--hairline); border-radius: 11px; background: var(--field); box-shadow: 0 12px 32px rgb(0 0 0 / 24%); overscroll-behavior: contain; }
  .settings-select-group { padding: 7px 10px 4px; color: var(--muted); font-size: 10px; font-weight: 600; letter-spacing: .02em; text-transform: uppercase; }
  .settings-select-option { display: flex; align-items: center; justify-content: space-between; gap: 10px; width: 100%; min-height: 32px; padding: 6px 10px; color: var(--ink); border-radius: 8px; font-size: 12px; text-align: left; transition: background .12s ease, color .12s ease; }
  .settings-select-option:hover, .settings-select-option.highlighted { color: #fff; background: var(--action); }
  .settings-select-option.selected:not(.highlighted) { color: var(--action); background: color-mix(in srgb, var(--action) 12%, var(--field)); font-weight: 600; }
  .settings-select-option span { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .settings-select-check { flex: none; width: 14px; height: 14px; fill: none; stroke: currentColor; stroke-linecap: round; stroke-linejoin: round; stroke-width: 1.8; }
  .settings-select.pill { width: 112px; flex: none; }
  .settings-select.pill .settings-select-trigger { height: 27px; min-height: 27px; gap: 4px; padding: 0 7px 0 9px; color: var(--muted); border-color: var(--field-border); border-radius: 999px; background: var(--tile); box-shadow: none; font-size: 11px; font-weight: 400; line-height: 1; }
  .settings-select.pill .settings-select-trigger:hover,
  .settings-select.pill .settings-select-trigger:focus-visible,
  .settings-select.pill .settings-select-trigger[aria-expanded='true'] { border-color: var(--field-border); background: var(--tile); outline: none; }
  .settings-select.pill .settings-select-trigger:focus-visible { outline: 2px solid var(--focus); outline-offset: 3px; }
  .settings-select.pill .settings-select-trigger:active { transform: none; }
  .settings-select.pill .settings-select-chevron { width: 10px; height: 10px; }
</style>
