<script lang="ts">
  let { restore = false }: { restore?: boolean } = $props();
</script>

<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
  <rect x="3" y="4" width="18" height="16" rx="2.5" />
  <rect x="12" y="12" width="6" height="5" rx="1" fill="currentColor" stroke="none" />
  {#if restore}
    <path d="m11 12-4-4m0 4V8h4" />
  {:else}
    <path d="m7 8 4 4m-4 0h4V8" />
  {/if}
</svg>
