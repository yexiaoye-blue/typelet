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

export type FontChoice = { value: string; label: string };

export function fontChoices(input: {
  families: string[];
  query: string;
  draft: string;
  builtinLabel: string;
  systemLabel: string;
}): FontChoice[] {
  const query = input.query.trim().toLocaleLowerCase();
  const matches = (label: string) => !query || label.toLocaleLowerCase().includes(query);
  const choices: FontChoice[] = [];
  if (matches(input.builtinLabel) || matches('noto sans mono')) choices.push({ value: bundledFont, label: input.builtinLabel });
  if (matches(input.systemLabel)) choices.push({ value: 'system', label: input.systemLabel });
  const draftFamily = input.draft.startsWith('font:') ? input.draft.slice(5) : '';
  if (draftFamily && !input.families.includes(draftFamily) && matches(draftFamily)) {
    choices.push({ value: input.draft, label: draftFamily });
  }
  for (const family of input.families) {
    if (matches(family)) choices.push({ value: 'font:' + family, label: family });
  }
  return choices;
}
