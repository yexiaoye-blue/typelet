import type { PracticeOrder } from '../packages/core/typing';

export type Locale = 'zh' | 'en';
export type LocalePreference = 'system' | Locale;
export type Theme = 'light' | 'dark';
export type ThemePreference = 'system' | Theme;
export type PronunciationPreference = 'uk' | 'us';

export function detectSystemLocale(): Locale {
  const language = typeof navigator !== 'undefined' ? navigator.language.toLowerCase() : 'en';
  return language.startsWith('zh') ? 'zh' : 'en';
}

export function loadLocalePreference(): LocalePreference {
  try {
    const value = localStorage.getItem('typelet.locale.preference');
    return value === 'zh' || value === 'en' || value === 'system' ? value : 'system';
  } catch { return 'system'; }
}

export function loadThemePreference(): ThemePreference {
  try {
    const value = localStorage.getItem('typelet.theme.preference');
    return value === 'light' || value === 'dark' || value === 'system' ? value : 'system';
  } catch { return 'system'; }
}

export function loadPronunciationPreference(): PronunciationPreference {
  try {
    return localStorage.getItem('typelet.word.pronunciation') === 'us' ? 'us' : 'uk';
  } catch { return 'uk'; }
}

export function loadAutoAdvanceWords(): boolean {
  try {
    return localStorage.getItem('typelet.word.autoAdvance') !== 'false';
  } catch { return true; }
}

export function loadChapterLength() {
  try {
    const value = Number(localStorage.getItem('typelet.practice.chapterLength'));
    return Number.isInteger(value) && value >= 10 ? value : 20;
  } catch { return 20; }
}

export function loadPracticeOrder(): PracticeOrder {
  try {
    return localStorage.getItem('typelet.practice.order') === 'random' ? 'random' : 'book';
  } catch { return 'book'; }
}

