import { invoke } from '@tauri-apps/api/core';

export const bundledFont = 'builtin:noto-mono';
const fallback = '"Typelet Mono", "Noto Sans CJK SC", system-ui, monospace';

export function loadFontPreference(): string {
  try {
    const value = localStorage.getItem('typelet.font.preference');
    return value === bundledFont || value === 'system' || value?.startsWith('font:') ? value : bundledFont;
  } catch { return bundledFont; }
}

export function fontStack(value: string): string {
  if (value === 'system') return 'system-ui, sans-serif';
  return value.startsWith('font:') ? `${JSON.stringify(value.slice(5))}, ${fallback}` : fallback;
}

export function persistFontPreference(value: string) {
  try { localStorage.setItem('typelet.font.preference', value); } catch { /* Storage may be disabled. */ }
}

export async function getSystemFonts(native: boolean): Promise<string[]> {
  const values: unknown = native
    ? await invoke<string[]>('list_system_fonts')
    : await fetch('/__typelet/fonts', { cache: 'no-store' }).then(response => {
        if (!response.ok) throw new Error('Font list unavailable');
        return response.json();
      });
  if (!Array.isArray(values) || !values.every(value => typeof value === 'string')) throw new Error('Invalid font list');
  return [...new Set(values.map(value => value.trim()).filter(Boolean))].sort((a, b) => a.localeCompare(b));
}
