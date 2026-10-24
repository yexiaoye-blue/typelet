<script lang="ts">
  import { onMount, tick } from 'svelte';
  import confetti from 'canvas-confetti';
  import { invoke } from '@tauri-apps/api/core';
  import { getVersion } from '@tauri-apps/api/app';
  import { open } from '@tauri-apps/plugin-dialog';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import {
    characters,
    limitPracticeInput,
    previewAdvance,
    previewCatalog,
    previewReset,
    previewSelectCollection,
    previewSelectChapter,
    previewSelectItem,
    previewSnapshot,
    previewUpdate,
    setPreviewLayout,
    summarizeChapter,
    type ContentCatalogSnapshot,
    type ChapterWordResult,
    type PracticeMode,
    type PracticeOrder,
    type PracticeSnapshot,
  } from '../packages/core/typing';
  import { en, zh } from './i18n';
  import { version as packagedVersion } from '../package.json';
  import { bundledFont, fontChoices, fontStack, getSystemFonts, loadFontPreference, persistFontPreference } from './fonts';
  import { ScrollMemory } from './scroll-memory';
  import { keySoundPacks, keysAudible, loadSoundPreference, persistSoundPreference, playKeySound, preloadKeySound, readingAudible, traceKeySound, unlockKeySound, type SoundPreference } from './key-sounds';
  import type { SettingsSelectOption } from './components/SettingsSelect.svelte';
  import {
    detectSystemLocale,
    loadAutoAdvanceWords,
    loadChapterLength,
    loadLocalePreference,
    loadPracticeOrder,
    loadPronunciationPreference,
    loadThemePreference,
    type Locale,
    type LocalePreference,
    type PronunciationPreference,
    type Theme,
    type ThemePreference,
  } from './preferences';
  import PracticePage from './practice/PracticePage.svelte';
  import SettingsPage from './settings/SettingsPage.svelte';
  import TitleBar from './shell/TitleBar.svelte';

  type Statistics = PracticeSnapshot['stats'];
  type AgentId = 'codex' | 'cursor' | 'dsh';
  type AgentSummary = { id: AgentId; name: string; enabled: boolean; supported: boolean; configPath: string; format: string; event: string };
  type AgentDetail = AgentSummary & { editable: boolean; content: string; note: string };
  type ImportReport = ImportResult['report'];
  type PendingImport = { kind: 'article' | 'wordBook'; path: string; report: ImportReport };

  const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
  const appWindow = isTauri ? getCurrentWindow() : undefined;
  type ImportResult = { report: { title: string; count: number; duplicateCount: number; firstItem: string | null }; catalog: ContentCatalogSnapshot; snapshot: PracticeSnapshot };
  setPreviewLayout(loadChapterLength(), loadPracticeOrder());
  const emptySnapshot = previewSnapshot();

  let localePreference = $state<LocalePreference>(loadLocalePreference());
  let systemLocale = $state<Locale>(detectSystemLocale());
  let themePreference = $state<ThemePreference>(loadThemePreference());
  let pronunciationPreference = $state<PronunciationPreference>(loadPronunciationPreference());
  let autoAdvanceWords = $state(loadAutoAdvanceWords());
  let chapterLength = $state(loadChapterLength());
  let practiceOrder = $state<PracticeOrder>(loadPracticeOrder());
  let sound = $state<SoundPreference>(loadSoundPreference());
  let appVersion = $state(packagedVersion);
  let systemTheme = $state<Theme>(window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light');
  let view = $state<'practice' | 'settings'>('practice');
  let snapshot = $state<PracticeSnapshot>(emptySnapshot);
  let libraries = $derived(snapshot.mode === 'sentence' ? catalog.articles : catalog.wordBooks);
  let catalog = $state<ContentCatalogSnapshot>(previewCatalog());
  let draft = $state('');
  let committedDraft = $state('');
  let compositionText = $state('');
  let focused = $state(false);
  let elapsed = $state(0);
  let pinned = $state(false);
  let pinGeneration = 0;
  let composing = $state(false);
  let surfacePressed = $state(false);
  let input = $state<HTMLTextAreaElement | undefined>();
  let backButton = $state<HTMLButtonElement | undefined>();
  let settingsPage = $state<HTMLElement | undefined>();
  const settingsScroll = new ScrollMemory();

  let textDisplay = $state<HTMLDivElement | undefined>();
  let captureX = $state(0);
  let captureY = $state(0);
  let wordFontSize = $state(46);
  let resizeObserver: ResizeObserver | undefined;
  let observedDisplay: HTMLDivElement | undefined;
  let fontPreference = $state(loadFontPreference());
  let fontDraft = $state(loadFontPreference());
  let fontQuery = $state('');
  let fontMenuOpen = $state(false);
  let fontHighlight = $state(-1);
  let fontField = $state<HTMLDivElement>();
  let systemFonts = $state<string[]>([]);
  let fontsLoading = $state(false);
  let fontsLoaded = $state(false);
  let fontsFailed = $state(false);
  let agents = $state<AgentSummary[]>([]);
  let agentsLoaded = $state(false);
  let agentLoading = $state(false);
  let agentError = $state('');
  let selectedAgent = $state<AgentId | null>(null);
  let libraryOpen = $state(false);
  let agentDetail = $state<AgentDetail | null>(null);
  let agentDetailDraft = $state('');
  let agentDetailLoading = $state(false);
  let agentDetailSaving = $state(false);
  let importMessage = $state('');
  let importError = $state('');
  let importLoading = $state(false);
  let pendingImport = $state<PendingImport | null>(null);
  let spokenKey = $state('');
  let spokenState: 'idle' | 'pending' | 'started' = 'idle';
  let autoAdvancedKey = $state('');
  let chapterKey = $state('');
  let chapterWords = $state<Record<number, ChapterWordResult>>({});
  let chapterPaused = $state(false);
  let practicePhase = $state<'idle' | 'running' | 'paused'>('idle');
  let allowIncompleteNext = $state(false);
  let chapterContinuation = false;
  let activeAudio: HTMLAudioElement | undefined;
  let pronunciationGeneration = 0;
  let finishPronunciation: (() => void) | undefined;
  let stats = $derived<Statistics>(snapshot.stats);
  let clock = { total: 0, since: null as number | null };
  let itemElapsedOrigin = 0;
  let requestId = 0;

  let locale = $derived<Locale>(localePreference === 'system' ? systemLocale : localePreference);
  let resolvedTheme = $derived<Theme>(themePreference === 'system' ? systemTheme : themePreference);
  let t = $derived(locale === 'zh' ? zh : en);
  let builtinLibraries = $derived(libraries.filter((item) => !item.removable));
  let importedLibraries = $derived(libraries.filter((item) => item.removable));
  let target = $derived(snapshot.target);
  let expected = $derived(characters(snapshot.target));
  let selectedPhonetic = $derived(snapshot.word ? pronunciationPreference === 'uk' ? snapshot.word.ukphone : snapshot.word.usphone : '');
  let selectedPhoneticLabel = $derived(pronunciationPreference === 'uk' ? t.ukPhone : t.usPhone);
  let chapterTotals = $derived(summarizeChapter(Object.values(chapterWords)));
  let displayAccuracy = $derived(chapterPaused ? chapterTotals.accuracy : stats.accuracy);
  let displayElapsed = $derived(elapsed);
  let displayWpm = $derived.by(() => {
    let correct = stats.correct;
    for (const [index, result] of Object.entries(chapterWords)) {
      if (Number(index) === snapshot.itemIndex) continue;
      correct += result.correct;
    }
    const cpm = elapsed >= 1000 ? Math.round(correct * 60000 / elapsed) : 0;
    return Math.round(cpm / 5);
  });
  let chapterResultTitle = $derived(snapshot.chapterTitle
    ? snapshot.chapterTitle + ' · ' + t.chapterComplete
    : locale === 'zh'
      ? t.chapterPrefix + ' ' + snapshot.chapterIndex + ' ' + t.chapterSuffix + t.chapterComplete
      : t.chapterPrefix + ' ' + snapshot.chapterIndex + ' ' + t.chapterComplete);
  let chapterResultUnit = $derived(snapshot.mode === 'sentence' ? t.chapterSentences : t.chapterWords);
  let typed = $derived(characters(committedDraft));
  const keySoundOptions: SettingsSelectOption[] = keySoundPacks.map(pack => ({ value: pack.id, label: pack.id }));
  let fontChoiceList = $derived(fontChoices({
    families: systemFonts,
    query: fontMenuOpen ? fontQuery : '',
    draft: fontDraft,
    builtinLabel: 'Noto Sans Mono · ' + t.builtIn,
    systemLabel: t.systemFont,
  }));
  let fontPreview = $derived(fontMenuOpen && fontChoiceList[fontHighlight] ? fontChoiceList[fontHighlight].value : fontDraft);
  let fontMenuMaxHeight = $state(168);
  let running = $derived(view === 'practice' && practicePhase === 'running' && focused && snapshot.practice.attempts > 0 && !stats.done);
  let awaitingStart = $derived(practicePhase === 'idle' && !chapterPaused);
  let itemReachedEnd = $derived(snapshot.target.length > 0 && characters(committedDraft).length >= characters(snapshot.target).length);
  let canGoPreviousItem = $derived(!chapterPaused && snapshot.chapterItemIndex > 1);
  let canGoNextItem = $derived(!chapterPaused && snapshot.chapterItemIndex < snapshot.chapterItemCount && (stats.done || itemReachedEnd || allowIncompleteNext));
  let canGoPreviousChapter = $derived(snapshot.chapterIndex > 1);
  let canGoNextChapter = $derived(snapshot.chapterIndex < snapshot.chapterCount);
  let chapterOptions = $derived(Array.from({ length: snapshot.chapterCount }, (_, index) => ({
    value: String(index),
    label: snapshot.chapterTitles[index] || t.chapterPrefix + ' ' + (index + 1) + (t.chapterSuffix ? ' ' + t.chapterSuffix : ''),
  })));
  let chapterLengthMax = $derived(Math.max(1, snapshot.itemCount));
  let chapterLengthMin = $derived(Math.min(10, chapterLengthMax));
  let effectiveChapterLength = $derived(Math.min(chapterLengthMax, Math.max(chapterLengthMin, chapterLength)));
  let celebrationCanvas = $state<HTMLCanvasElement | undefined>();
  let celebratedChapter = '';
  let celebrationBurst: ReturnType<typeof confetti.create> | undefined;

  $effect(() => {
    if (typeof document === 'undefined') return;
    document.documentElement.lang = locale === 'zh' ? 'zh-CN' : 'en';
    document.documentElement.dataset.theme = resolvedTheme;
  });
  $effect(() => {
    if (view === 'practice') void updateCaret(committedDraft, fontPreference, target, compositionText);
  });
  $effect(() => {
    if (view === 'settings' && !agentsLoaded) void loadAgents();
  });
  $effect(() => {
    const finished = view === 'practice' && chapterPaused;
    if (!finished) {
      celebratedChapter = '';
      return;
    }
    const key = snapshot.collectionId + ':' + snapshot.chapterIndex;
    const canvas = celebrationCanvas;
    if (!canvas || celebratedChapter === key) return;
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;
    celebratedChapter = key;
    // The canvas lives for the whole practice view; one burst instance per element.
    celebrationBurst ??= confetti.create(canvas, { resize: true, useWorker: false, disableForReducedMotion: true });
    void celebrationBurst({ particleCount: 16, angle: 62, spread: 36, startVelocity: 16, origin: { x: 0.04, y: 0.7 }, scalar: 0.5, ticks: 100, gravity: 0.9, disableForReducedMotion: true });
    void celebrationBurst({ particleCount: 16, angle: 118, spread: 36, startVelocity: 16, origin: { x: 0.96, y: 0.7 }, scalar: 0.5, ticks: 100, gravity: 0.9, disableForReducedMotion: true });
  });

  function handlePracticeNavigationKey(event: KeyboardEvent) {
    if (event.defaultPrevented || view !== 'practice' || event.repeat || event.isComposing || event.altKey || event.metaKey) return false;
    const target = event.target as HTMLElement | null;
    if (target?.closest('[role="combobox"][aria-expanded="true"], [role="listbox"]')) return false;
    if (target?.closest('select, input:not(#typing-capture), textarea:not(#typing-capture)')) return false;
    if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return false;
    event.preventDefault();
    const direction = event.key === 'ArrowLeft' ? -1 : 1;
    if (event.ctrlKey) void navigateChapter(direction);
    else void navigateItem(direction);
    return true;
  }

  onMount(() => {
    let timer: ReturnType<typeof setInterval> | undefined;
    let unlistenFocus: (() => void) | undefined;
    const colorScheme = window.matchMedia('(prefers-color-scheme: dark)');
    const syncSystemTheme = () => { systemTheme = colorScheme.matches ? 'dark' : 'light'; };
    const syncSystemLocale = () => { systemLocale = detectSystemLocale(); };
    syncSystemTheme();
    syncSystemLocale();

    const leaveApp = () => {
      focused = false;
      pausePractice();
    };
    const releaseSurface = () => {
      if (!surfacePressed) return;
      surfacePressed = false;
      focusInput();
    };
    const resume = () => {
      syncSystemTheme();
      syncSystemLocale();
      if (view === 'practice') focusInput();
    };
    const hidden = () => { if (document.hidden) leaveApp(); else resume(); };

    window.addEventListener('blur', leaveApp);
    window.addEventListener('pointerup', releaseSurface);
    window.addEventListener('focus', resume);
    window.addEventListener('languagechange', syncSystemLocale);
    document.addEventListener('visibilitychange', hidden);
    const startFromKey = (event: KeyboardEvent) => {
      if (view !== 'practice') return;
      if (chapterPaused) {
        if (event.key === 'Enter' && !event.repeat && !event.altKey && !event.ctrlKey && !event.metaKey) {
          // A focused chapter button keeps its own Enter activation.
          const target = event.target as HTMLElement | null;
          if (target?.closest('.practice-page button')) return;
          event.preventDefault();
          void continueAndStart();
        }
        return;
      }
      if (practicePhase === 'running') return;
      if (event.isComposing || event.repeat || event.altKey || event.ctrlKey || event.metaKey) return;
      if (['Shift', 'Control', 'Alt', 'Meta', 'CapsLock', 'Tab', 'Escape'].includes(event.key)) return;
      const target = event.target as HTMLElement | null;
      if (target?.closest('[role="combobox"], [role="listbox"], .settings-select')) return;
      const practiceButton = target?.closest('.practice-page button');
      if (practiceButton) {
        if (event.key === 'Enter' || event.key === ' ') return;
      } else if (target?.closest('button, select, a, input:not(#typing-capture)')) {
        return;
      }
      event.preventDefault();
      beginPractice();
    };
    const handleGlobalKeydown = (event: KeyboardEvent) => {
      if (handlePracticeNavigationKey(event)) return;
      if (event.code === 'Comma' && event.ctrlKey && !event.shiftKey && !event.altKey && !event.metaKey && !event.repeat) {
        event.preventDefault();
        if (view !== 'settings') void openSettings();
        return;
      }
      if (event.key !== 'Escape' || event.repeat || event.altKey || event.ctrlKey || event.metaKey) return;
      if (view === 'settings') {
        // Escape inside a text field must not throw away unsaved edits by leaving settings.
        const origin = event.target as HTMLElement | null;
        if (origin?.closest('input, textarea, select, [role="combobox"], [role="listbox"]')) return;
        event.preventDefault();
        void navigateBack();
        return;
      }
      if (view === 'practice' && practicePhase === 'running') {
        event.preventDefault();
        pausePractice();
      }
    };
    preloadKeySound(sound.pack);
    if (isTauri) {
      void getVersion().then(version => { appVersion = version; }).catch(() => { /* Keep the packaged version. */ });
    }
    const primeKeySound = () => unlockKeySound();
    window.addEventListener('pointerdown', primeKeySound, true);
    window.addEventListener('keydown', primeKeySound, true);
    window.addEventListener('keydown', startFromKey);
    window.addEventListener('keydown', handleGlobalKeydown);
    colorScheme.addEventListener?.('change', syncSystemTheme);
    const fontLayoutChanged = () => { void updateCaret(); };
    resizeObserver = new ResizeObserver(fontLayoutChanged);
    document.fonts.addEventListener('loadingdone', fontLayoutChanged);
    void loadPractice().then(() => tick()).then(() => focusInput());

    if (appWindow) {
      const pinRead = pinGeneration;
      void appWindow.isAlwaysOnTop().then(value => {
        if (pinRead === pinGeneration) pinned = value;
      });
      void appWindow.onFocusChanged(({ payload: active }) => {
        if (!active) pausePractice();
      }).then(unlisten => { unlistenFocus = unlisten; });
    }

    const updateClock = () => {
      if (!running) return;
      const now = performance.now();
      elapsed = clock.total + now - (clock.since ?? now);
      void refreshCore(committedDraft, itemElapsed());
    };

    timer = setInterval(updateClock, 200);
    return () => {
      if (timer) clearInterval(timer);
      window.removeEventListener('blur', leaveApp);
      window.removeEventListener('pointerup', releaseSurface);
      window.removeEventListener('focus', resume);
      window.removeEventListener('languagechange', syncSystemLocale);
      document.removeEventListener('visibilitychange', hidden);
      window.removeEventListener('pointerdown', primeKeySound, true);
      window.removeEventListener('keydown', primeKeySound, true);
      window.removeEventListener('keydown', startFromKey);
      window.removeEventListener('keydown', handleGlobalKeydown);
      colorScheme.removeEventListener?.('change', syncSystemTheme);
      resizeObserver?.disconnect();
      document.fonts.removeEventListener('loadingdone', fontLayoutChanged);
      unlistenFocus?.();
    };
  });

  $effect(() => {
    if (running && clock.since === null) {
      clock.since = performance.now();
    } else if (!running && clock.since !== null) {
      clock.total += performance.now() - clock.since;
      clock.since = null;
      elapsed = clock.total;
    }
  });

  function persistLocalePreference(value: LocalePreference) {
    localePreference = value;
    try { localStorage.setItem('typelet.locale.preference', value); } catch { /* browser storage may be unavailable */ }
  }

  function persistThemePreference(value: ThemePreference) {
    themePreference = value;
    try { localStorage.setItem('typelet.theme.preference', value); } catch { /* browser storage may be unavailable */ }
  }

  function persistPronunciationPreference(value: PronunciationPreference) {
    pronunciationPreference = value;
    try { localStorage.setItem('typelet.word.pronunciation', value); } catch { /* browser storage may be unavailable */ }
  }

  function persistAutoAdvanceWords(value: boolean) {
    if (autoAdvanceWords === value) return;
    autoAdvanceWords = value;
    try { localStorage.setItem('typelet.word.autoAdvance', String(value)); } catch { /* browser storage may be unavailable */ }
    if (value) speakVisibleWord(snapshot, true);
    else stopPronunciation();
  }

  function commitChapterLength(input: HTMLInputElement) {
    const parsed = Number(input.value.trim());
    if (!input.value.trim() || !Number.isInteger(parsed) || chapterLengthMin === chapterLengthMax) {
      input.value = String(effectiveChapterLength);
      return;
    }
    const next = Math.min(chapterLengthMax, Math.max(chapterLengthMin, parsed));
    input.value = String(next);
    if (next === effectiveChapterLength && next === chapterLength) return;
    void applyPracticeLayout(next, practiceOrder);
  }

  function stepChapterLength(direction: -1 | 1) {
    if (chapterLengthMin === chapterLengthMax) return;
    const next = Math.min(chapterLengthMax, Math.max(chapterLengthMin, effectiveChapterLength + direction));
    if (next === effectiveChapterLength) return;
    void applyPracticeLayout(next, practiceOrder);
  }

  function onChapterLengthKeydown(event: KeyboardEvent) {
    if (event.key === 'ArrowUp' || event.key === 'ArrowDown') {
      event.preventDefault();
      stepChapterLength(event.key === 'ArrowUp' ? 1 : -1);
      return;
    }
    if (event.key !== 'Enter') return;
    event.preventDefault();
    const input = event.currentTarget;
    if (input instanceof HTMLInputElement) commitChapterLength(input);
  }

  function choosePracticeOrder(order: PracticeOrder) {
    if (order === practiceOrder) return;
    void applyPracticeLayout(chapterLength, order);
  }

  async function applyPracticeLayout(nextLength: number, nextOrder: PracticeOrder) {
    const previous = snapshot;
    chapterLength = nextLength;
    practiceOrder = nextOrder;
    try {
      localStorage.setItem('typelet.practice.chapterLength', String(nextLength));
      localStorage.setItem('typelet.practice.order', nextOrder);
    } catch { /* browser storage may be unavailable */ }
    try {
      if (isTauri) {
        const next = await invoke<{ catalog: ContentCatalogSnapshot; snapshot: PracticeSnapshot }>('set_practice_layout', { chapterLength: nextLength, order: nextOrder });
        catalog = next.catalog;
        adoptLayoutSnapshot(previous, next.snapshot);
      } else {
        setPreviewLayout(nextLength, nextOrder);
        const cursor = Math.max(0, previous.itemIndex - 1);
        const kept = previewSnapshot(previous.mode, previous.collectionId, cursor, previous.practice);
        const changed = kept.target !== previous.target;
        const applied = changed ? previewSnapshot(previous.mode, previous.collectionId, cursor) : kept;
        const progress = loadWebProgress();
        catalog = previewCatalog(applied.mode, applied.mode === 'sentence' ? applied.collectionId : progress.sentenceId, applied.mode === 'word' ? applied.collectionId : progress.wordBookId);
        adoptLayoutSnapshot(previous, applied);
      }
    } catch (error) {
      importError = String(error);
    }
  }

  function adoptLayoutSnapshot(previous: PracticeSnapshot, next: PracticeSnapshot) {
    const targetChanged = next.target !== previous.target || next.collectionId !== previous.collectionId;
    const chapterChanged = targetChanged || next.chapterIndex !== previous.chapterIndex;
    if (chapterChanged) {
      invalidateCompletion();
      requestId += 1;
      allowIncompleteNext = false;
      resetPracticeClock();
      chapterKey = '';
    }
    if (targetChanged) {
      draft = '';
      committedDraft = '';
    }
    applySnapshot(next, targetChanged);
  }

  function speechLevel() {
    return sound.speechVolume / 100;
  }

  function rememberSound(next: SoundPreference) {
    sound = next;
    persistSoundPreference(next);
    if (!readingAudible(sound)) {
      stopPronunciation();
      return;
    }
    if (activeAudio) activeAudio.volume = speechLevel();
  }

  let keyPreviewAt = 0;
  function previewKeySound(force = false) {
    if (!keysAudible(sound)) return;
    const now = performance.now();
    if (!force && now - keyPreviewAt < 80) return;
    keyPreviewAt = now;
    playKeySound(sound.pack, sound.keyVolume);
  }

  function setSoundEnabled(enabled: boolean) {
    rememberSound({ ...sound, enabled });
    if (enabled) previewKeySound(true);
  }

  function setSpeechMuted(muted: boolean) {
    rememberSound({ ...sound, speechMuted: muted });
  }

  function setSpeechVolume(event: Event) {
    const speechVolume = Math.round((event.currentTarget as HTMLInputElement).valueAsNumber);
    if (!Number.isFinite(speechVolume)) return;
    rememberSound({ ...sound, speechMuted: false, speechVolume: Math.max(0, Math.min(100, speechVolume)) });
  }

  function setKeyMuted(muted: boolean) {
    rememberSound({ ...sound, keyMuted: muted });
    if (!muted) previewKeySound(true);
  }

  function setKeyVolume(event: Event) {
    const keyVolume = Math.round((event.currentTarget as HTMLInputElement).valueAsNumber);
    if (!Number.isFinite(keyVolume)) return;
    rememberSound({ ...sound, keyMuted: false, keyVolume: Math.max(0, Math.min(100, keyVolume)) });
    previewKeySound();
  }

  function setKeyPack(pack: string) {
    if (!keySoundPacks.some(item => item.id === pack)) return;
    rememberSound({ ...sound, pack });
    previewKeySound(true);
  }

  function playPracticeKey(previous: string, next: string, occurredAt: number) {
    if (!keysAudible(sound) || view !== 'practice' || practicePhase !== 'running' || chapterPaused) return;
    if (next.length <= previous.length || !next.startsWith(previous)) return;
    const lag = Math.round(keyEventLag(occurredAt));
    const added = next.length - previous.length;
    if (lag > 70) {
      traceKeySound('practice-drop-late', { lag, added });
      return;
    }
    traceKeySound('practice-key', { lag, added });
    playKeySound(sound.pack, sound.keyVolume);
  }

  function keyEventLag(occurredAt: number) {
    const now = performance.now();
    if (!Number.isFinite(occurredAt) || occurredAt <= 0) return 0;
    // Some WebKit builds report event.timeStamp in seconds. Treat a tiny
    // stamp as seconds so a clock mismatch cannot silence every click.
    const stamp = occurredAt < now / 20 ? occurredAt * 1000 : occurredAt;
    return now - stamp;
  }


  function fontLabel(value: string) {
    if (value === bundledFont) return 'Noto Sans Mono · ' + t.builtIn;
    if (value === 'system') return t.systemFont;
    return value.startsWith('font:') ? value.slice(5) : value;
  }

  function openFontMenu() {
    placeFontMenu();
    fontMenuOpen = true;
    fontHighlight = Math.max(0, fontChoiceList.findIndex(choice => choice.value === fontDraft));
  }

  function placeFontMenu() {
    const field = fontField?.querySelector('.font-field');
    const page = fontField?.closest('.settings-page');
    if (!(field instanceof HTMLElement) || !(page instanceof HTMLElement)) return;
    const minimum = 148;
    const fieldBox = field.getBoundingClientRect();
    const pageBox = page.getBoundingClientRect();
    let room = pageBox.bottom - fieldBox.bottom - 10;
    if (room < minimum) {
      page.scrollTop += minimum - room;
      room = page.getBoundingClientRect().bottom - field.getBoundingClientRect().bottom - 10;
    }
    fontMenuMaxHeight = Math.max(96, Math.min(200, Math.floor(room)));
  }

  function closeFontMenu() {
    fontMenuOpen = false;
    fontQuery = '';
    fontHighlight = -1;
  }

  function chooseFont(value: string) {
    fontDraft = value;
    closeFontMenu();
  }

  function applyFont() {
    if (fontDraft === fontPreference) return;
    fontPreference = fontDraft;
    persistFontPreference(fontDraft);
  }

  function onFontFocusOut(event: FocusEvent) {
    const next = event.relatedTarget;
    if (next instanceof Node && fontField?.contains(next)) return;
    closeFontMenu();
  }

  function onFontInput(event: Event) {
    fontQuery = (event.currentTarget as HTMLInputElement).value;
    fontMenuOpen = true;
    fontHighlight = 0;
  }

  async function moveFontHighlight(next: number) {
    fontHighlight = next;
    await tick();
    const menu = fontField?.querySelector<HTMLElement>('.font-menu');
    const option = menu?.querySelectorAll<HTMLButtonElement>('button')[next];
    if (!menu || !option) return;
    const top = option.offsetTop;
    const bottom = top + option.offsetHeight;
    if (top < menu.scrollTop) menu.scrollTop = top;
    else if (bottom > menu.scrollTop + menu.clientHeight) menu.scrollTop = bottom - menu.clientHeight;
  }

  function onFontKeydown(event: KeyboardEvent) {
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      if (!fontMenuOpen) {
        openFontMenu();
        return;
      }
      void moveFontHighlight(Math.min(fontChoiceList.length - 1, fontHighlight + 1));
    } else if (event.key === 'ArrowUp' && fontMenuOpen) {
      event.preventDefault();
      void moveFontHighlight(Math.max(0, fontHighlight - 1));
    } else if (event.key === 'Enter' && fontMenuOpen && fontChoiceList[fontHighlight]) {
      event.preventDefault();
      chooseFont(fontChoiceList[fontHighlight].value);
    } else if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      closeFontMenu();
    }
  }

  function previewAgents(): AgentSummary[] {
    return [
      { id: 'codex', name: 'Codex', enabled: localStorage.getItem('typelet.agent.codex') === 'true', supported: true, configPath: '~/.codex/config.toml', format: 'TOML', event: 'UserPromptSubmit' },
      { id: 'cursor', name: 'Cursor', enabled: localStorage.getItem('typelet.agent.cursor') === 'true', supported: true, configPath: '~/.cursor/hooks.json', format: 'JSON', event: 'beforeSubmitPrompt' },
      { id: 'dsh', name: 'DeepSeek Harness (dsh)', enabled: false, supported: false, configPath: '', format: 'Plugin', event: '待确认' },
    ];
  }

  async function loadAgents() {
    agentsLoaded = true;
    agentError = '';
    try {
      if (isTauri) {
        agents = await invoke<AgentSummary[]>('get_agents_config');
      } else {
        agents = previewAgents();
      }
    } catch (error) {
      agentError = String(error);
    }
  }

  async function toggleAgent(agent: AgentSummary) {
    if (agentLoading) return;
    if (!agent.supported) return;
    agentLoading = true;
    agentError = '';
    const enabled = !agent.enabled;
    try {
      if (isTauri) {
        agents = await invoke<AgentSummary[]>('set_agent_enabled', { agent: agent.id, enabled });
      } else {
        localStorage.setItem('typelet.agent.' + agent.id, String(enabled));
        agents = agents.map(item => item.id === agent.id ? { ...item, enabled } : item);
      }
    } catch (error) {
      agentError = String(error);
    } finally {
      agentLoading = false;
    }
  }


  function currentSettingsPage() {
    if (selectedAgent) return 'agent:' + selectedAgent;
    if (libraryOpen) return 'library';
    return 'settings';
  }

  function rememberSettingsScroll() {
    settingsScroll.save(currentSettingsPage(), settingsPage);
  }

  async function restoreSettingsScroll(id = currentSettingsPage()) {
    await tick();
    settingsScroll.apply(id, settingsPage);
  }

  async function openAgentDetail(agent: AgentSummary) {
    rememberSettingsScroll();
    selectedAgent = agent.id;
    agentDetail = null;
    agentDetailDraft = '';
    agentDetailLoading = true;
    agentError = '';
    await restoreSettingsScroll('agent:' + agent.id);
    backButton?.focus({ preventScroll: true });
    try {
      if (isTauri) {
        agentDetail = await invoke<AgentDetail>('get_agent_detail', { agent: agent.id });
      } else {
        agentDetail = { ...agent, editable: agent.supported, content: agent.supported ? JSON.stringify({ command: 'nohup typelet >/dev/null 2>&1 </dev/null &' }, null, 2) : '', note: agent.supported ? '网页预览只显示模拟配置。' : 'DeepSeek Harness 的接入方式待确认。' };
      }
      agentDetailDraft = agentDetail?.content ?? '';
    } catch (error) {
      agentError = String(error);
    } finally {
      agentDetailLoading = false;
    }
  }

  function closeAgentDetail() {
    rememberSettingsScroll();
    selectedAgent = null;
    agentDetail = null;
    agentDetailDraft = '';
    agentError = '';
    void restoreSettingsScroll('settings');
  }

  async function saveAgentDetail() {
    if (!agentDetail || !agentDetail.editable || agentDetailSaving) return;
    agentDetailSaving = true;
    agentError = '';
    try {
      if (isTauri) {
        agentDetail = await invoke<AgentDetail>('save_agent_detail', { agent: agentDetail.id, content: agentDetailDraft });
        agents = await invoke<AgentSummary[]>('get_agents_config');
      } else {
        agentDetail = { ...agentDetail, content: agentDetailDraft, enabled: true };
        agents = agents.map(item => item.id === agentDetail?.id ? { ...item, enabled: true } : item);
      }
      agentDetailDraft = agentDetail.content;
    } catch (error) {
      agentError = String(error);
    } finally {
      agentDetailSaving = false;
    }
  }

  async function loadFonts() {
    if (fontsLoading) return;
    fontsLoading = true;
    fontsFailed = false;
    try {
      systemFonts = await getSystemFonts(isTauri);
      fontsLoaded = true;
    } catch { fontsFailed = true; }
    finally { fontsLoading = false; }
  }

  function focusInput() {
    input?.focus({ preventScroll: true });
  }

  function dismissChapterPicker() {
    window.setTimeout(() => {
      if (view === 'practice') focusInput();
    }, 0);
  }

  function practiceButton(event: Event) {
    if (view !== 'practice') return null;
    const target = event.target;
    if (!(target instanceof Element)) return null;
    return target.closest('.practice-page button');
  }

  function restorePracticeTypingFocus(event: MouseEvent) {
    const control = practiceButton(event);
    if (!control || control.closest('.settings-select')) return;
    // Keep the caret in the capture field without cancelling the button click.
    // WebKitGTK drops the click when mousedown is cancelled, so only move focus afterward.
    if (event.type === 'mousedown') return;
    if (document.activeElement === control) control.blur();
    focusInput();
  }

  type WebProgress = { mode: PracticeMode; sentenceId: string; wordBookId: string; sentenceIndex: number; wordIndex: number };

  function defaultWebProgress(): WebProgress {
    return { mode: 'sentence', sentenceId: 'builtin:movie-lines', wordBookId: 'preview:cet4', sentenceIndex: 0, wordIndex: 0 };
  }

  function loadWebProgress(): WebProgress {
    const base = defaultWebProgress();
    try {
      const parsed = JSON.parse(localStorage.getItem('typelet.practice.progress') ?? '') as Partial<WebProgress>;
      const index = (value: unknown) => Number.isInteger(value) && Number(value) >= 0 ? Number(value) : 0;
      return {
        mode: parsed.mode === 'word' || parsed.mode === 'sentence' ? parsed.mode : base.mode,
        sentenceId: typeof parsed.sentenceId === 'string' && parsed.sentenceId ? parsed.sentenceId : base.sentenceId,
        wordBookId: typeof parsed.wordBookId === 'string' && parsed.wordBookId ? parsed.wordBookId : base.wordBookId,
        sentenceIndex: index(parsed.sentenceIndex),
        wordIndex: index(parsed.wordIndex),
      };
    } catch {
      return base;
    }
  }

  function persistWebProgress(next: PracticeSnapshot) {
    const previous = loadWebProgress();
    const progress: WebProgress = {
      mode: next.mode,
      sentenceId: next.mode === 'sentence' ? next.collectionId : previous.sentenceId,
      wordBookId: next.mode === 'word' ? next.collectionId : previous.wordBookId,
      sentenceIndex: next.mode === 'sentence' ? Math.max(0, next.itemIndex - 1) : previous.sentenceIndex,
      wordIndex: next.mode === 'word' ? Math.max(0, next.itemIndex - 1) : previous.wordIndex,
    };
    try { localStorage.setItem('typelet.practice.progress', JSON.stringify(progress)); } catch { /* browser storage may be unavailable */ }
  }

  function previewRestoredMode(mode: PracticeMode) {
    const progress = loadWebProgress();
    return mode === 'word'
      ? previewSnapshot('word', progress.wordBookId, progress.wordIndex)
      : previewSnapshot('sentence', progress.sentenceId, progress.sentenceIndex);
  }

  async function loadPractice() {
    try {
      if (isTauri) {
        const next = await invoke<{ catalog: ContentCatalogSnapshot; snapshot: PracticeSnapshot }>('set_practice_layout', { chapterLength, order: practiceOrder });
        catalog = next.catalog;
        applySnapshot(next.snapshot, true);
      } else {
        setPreviewLayout(chapterLength, practiceOrder);
        const progress = loadWebProgress();
        catalog = previewCatalog(progress.mode, progress.sentenceId, progress.wordBookId);
        const collectionId = progress.mode === 'word' ? progress.wordBookId : progress.sentenceId;
        const itemIndex = progress.mode === 'word' ? progress.wordIndex : progress.sentenceIndex;
        applySnapshot(previewSnapshot(progress.mode, collectionId, itemIndex), true);
      }
    } catch (error) {
      importError = String(error);
    }
  }

  function applySnapshot(next: PracticeSnapshot, resetInput: boolean) {
    snapshot = next;
    if (resetInput) {
      draft = next.practice.text;
      committedDraft = next.practice.text;
      compositionText = '';
    }
    syncChapterSession(next);
    if (!next.stats.done) autoAdvancedKey = '';
    handleItemCompletion(next);
    speakVisibleWord(next);
    if (!isTauri) persistWebProgress(next);
  }

  function syncChapterSession(next: PracticeSnapshot) {
    const key = next.collectionId + ':' + next.chapterIndex;
    if (key === chapterKey) return;
    chapterKey = key;
    chapterWords = {};
    chapterPaused = false;
  }

  function currentElapsed() {
    return clock.since === null ? elapsed : clock.total + performance.now() - clock.since;
  }

  function itemElapsed() {
    return Math.max(0, currentElapsed() - itemElapsedOrigin);
  }

  function resetPracticeClock() {
    clock = { total: 0, since: null };
    elapsed = 0;
    itemElapsedOrigin = 0;
  }

  function rememberCompletedItem(next: PracticeSnapshot) {
    chapterWords = {
      ...chapterWords,
      [next.itemIndex]: {
        attempts: next.practice.attempts,
        correctAttempts: next.practice.correctAttempts,
        correct: next.stats.correct,
        milliseconds: Math.max(0, Math.round(currentElapsed()) - itemElapsedOrigin),
      },
    };
  }

  function invalidateCompletion() {
    stopPronunciation();
    spokenKey = '';
    spokenState = 'idle';
  }

  function shouldAutoAdvance(next: PracticeSnapshot) {
    return next.mode === 'sentence' || autoAdvanceWords;
  }

  function handleItemCompletion(next: PracticeSnapshot) {
    if (!next.stats.done) return;
    const key = next.collectionId + ':' + next.chapterIndex + ':' + next.itemIndex;
    rememberCompletedItem(next);
    if (next.isChapterLastItem) {
      chapterPaused = true;
      return;
    }
    if (!shouldAutoAdvance(next) || autoAdvancedKey === key) return;
    autoAdvancedKey = key;
    void restart(true);
  }

  function waitForAudio(audio: HTMLAudioElement) {
    return new Promise<void>((resolve) => {
      let finished = false;
      const finish = () => {
        if (finished) return;
        finished = true;
        if (finishPronunciation === finish) finishPronunciation = undefined;
        audio.removeEventListener('ended', finish);
        audio.removeEventListener('error', finish);
        resolve();
      };
      finishPronunciation = finish;
      audio.addEventListener('ended', finish, { once: true });
      audio.addEventListener('error', finish, { once: true });
      window.setTimeout(finish, 1800);
    });
  }

  function stopPronunciation() {
    pronunciationGeneration += 1;
    const finish = finishPronunciation;
    finishPronunciation = undefined;
    finish?.();
    activeAudio?.pause();
    activeAudio = undefined;
    window.speechSynthesis?.cancel();
  }

  function wordKey(next: PracticeSnapshot) {
    return next.collectionId + ':' + next.chapterIndex + ':' + next.itemIndex;
  }

  function markSpoken(key: string, generation: number, started: boolean) {
    if (generation !== pronunciationGeneration || spokenKey !== key) return;
    if (started) spokenState = 'started';
    else {
      spokenKey = '';
      spokenState = 'idle';
    }
  }

  function startSpokenWord(name: string | undefined, key: string, force = false) {
    if (!name || !readingAudible(sound)) return;
    if (!force && spokenKey === key && spokenState !== 'idle') return;
    spokenKey = key;
    spokenState = 'pending';
    void playPronunciation(name).then(result => markSpoken(key, result.generation, result.started));
  }

  function speakVisibleWord(next: PracticeSnapshot = snapshot, force = false) {
    if (view !== 'practice' || practicePhase !== 'running' || !autoAdvanceWords || chapterPaused) return;
    if (next.mode !== 'word' || !next.word || next.stats.done) return;
    if (next.practice.text || draft || compositionText) return;
    startSpokenWord(next.word.name, wordKey(next), force);
  }

  function beginPractice() {
    if (chapterPaused || view !== 'practice' || practicePhase === 'running') return;
    unlockKeySound();
    practicePhase = 'running';
    if (snapshot.mode === 'word' && snapshot.word && !snapshot.stats.done) {
      startSpokenWord(snapshot.word.name, wordKey(snapshot), true);
    }
    focusInput();
  }

  function pausePractice() {
    if (practicePhase !== 'running') return;
    practicePhase = 'paused';
    stopPronunciation();
  }

  function togglePractice() {
    if (practicePhase === 'running') pausePractice();
    else beginPractice();
  }

  async function playPronunciation(name: string | undefined): Promise<{ generation: number; started: boolean }> {
    // Checked before constructing the audio element so a mute does not request the pronunciation.
    if (!name || !readingAudible(sound)) return { generation: pronunciationGeneration, started: false };
    const generation = ++pronunciationGeneration;
    const finishPrevious = finishPronunciation;
    finishPronunciation = undefined;
    finishPrevious?.();
    activeAudio?.pause();
    window.speechSynthesis?.cancel();
    const audioType = pronunciationPreference === 'uk' ? '1' : '2';
    const audio = new Audio('https://dict.youdao.com/dictvoice?audio=' + encodeURIComponent(name) + '&type=' + audioType);
    audio.volume = speechLevel();
    activeAudio = audio;
    try {
      await audio.play();
      if (generation !== pronunciationGeneration) return { generation, started: false };
      void waitForAudio(audio);
      return { generation, started: true };
    } catch {
      if (generation !== pronunciationGeneration || activeAudio !== audio) return { generation, started: false };
      return { generation, started: await speakFallback(name, generation) };
    }
  }

  function speakFallback(name: string, generation: number) {
    const speech = window.speechSynthesis;
    if (!speech || generation !== pronunciationGeneration) return Promise.resolve(false);
    return new Promise<boolean>((resolve) => {
      let settled = false;
      const finish = (started: boolean) => {
        if (settled) return;
        settled = true;
        resolve(started);
      };
      const utterance = new SpeechSynthesisUtterance(name);
      utterance.lang = pronunciationPreference === 'uk' ? 'en-GB' : 'en-US';
      utterance.volume = speechLevel();
      utterance.onstart = () => finish(true);
      utterance.onerror = () => finish(false);
      window.setTimeout(() => finish(false), 700);
      try { speech.speak(utterance); } catch { finish(false); }
    });
  }

  function replayWord(event: MouseEvent) {
    event.preventDefault();
    if (snapshot.word) startSpokenWord(snapshot.word.name, wordKey(snapshot), true);
  }
  function markSurfacePress(event: PointerEvent) {
    const origin = event.target as HTMLElement | null;
    const interactive = origin?.closest('button, select, a, input, textarea, summary, [role="button"]');
    surfacePressed = view === 'practice' && !interactive;
    // A click inside the window must keep Typelet above the window underneath.
    void appWindow?.setFocus();
  }

  function handleFocus() {
    focused = true;
    if (input && !composing) input.setSelectionRange(input.value.length, input.value.length);
    if (spokenState === 'idle') speakVisibleWord();
  }

  async function updateCaret(..._changes: unknown[]) {
    await tick();
    if (view !== 'practice' || !textDisplay || !input) return;
    const previousWordSize = wordFontSize;
    fitWordFont();
    if (wordFontSize !== previousWordSize) await tick();
    if (observedDisplay !== textDisplay) {
      resizeObserver?.disconnect();
      resizeObserver?.observe(textDisplay);
      observedDisplay = textDisplay;
    }
    const current = textDisplay.querySelector<HTMLElement>('.caret') ?? textDisplay.lastElementChild as HTMLElement | null;
    if (!current) return;
    const display = textDisplay.getBoundingClientRect();
    let character = current.getBoundingClientRect();
    if (character.top < display.top + 8) textDisplay.scrollTop -= display.top + 8 - character.top;
    else if (character.bottom > display.bottom - 8) textDisplay.scrollTop += character.bottom - display.bottom + 8;
    const passage = textDisplay.closest('.sentence-stack') as HTMLElement | null;
    if (passage && getComputedStyle(passage).display !== 'contents') {
      const box = passage.getBoundingClientRect();
      const caret = current.getBoundingClientRect();
      if (caret.top < box.top + 8) passage.scrollTop -= box.top + 8 - caret.top;
      else if (caret.bottom > box.bottom - 8) passage.scrollTop += caret.bottom - box.bottom + 8;
    }
    character = current.getBoundingClientRect();
    const surface = textDisplay.parentElement!.getBoundingClientRect();
    captureX = Math.max(0, Math.min(character.left - surface.left, surface.width - 2));
    captureY = Math.max(0, Math.min(character.top - surface.top, surface.height - 24));
  }

  function fitWordFont() {
    if (snapshot.mode !== 'word' || !textDisplay) return;
    const stage = textDisplay.closest('.word-stage') as HTMLElement | null;
    const available = stage?.clientWidth ?? 0;
    if (available < 40) return;
    const maxSize = 46;
    const minSize = 16;
    const previousWidth = textDisplay.style.width;
    const previousMaxWidth = textDisplay.style.maxWidth;
    textDisplay.style.width = 'max-content';
    textDisplay.style.maxWidth = 'none';
    const styleSize = parseFloat(getComputedStyle(textDisplay).fontSize);
    const natural = textDisplay.scrollWidth * maxSize / styleSize;
    textDisplay.style.width = previousWidth;
    textDisplay.style.maxWidth = previousMaxWidth;
    if (!styleSize || !natural) return;
    const limit = Math.max(1, available - 12);
    const fitted = natural <= limit ? maxSize : Math.max(minSize, Math.min(maxSize, Math.floor(maxSize * limit / natural)));
    if (fitted !== wordFontSize) wordFontSize = fitted;
  }

  async function refreshCore(value: string, milliseconds: number) {
    const id = ++requestId;
    let nextSnapshot: PracticeSnapshot;

    if (isTauri) {
      nextSnapshot = await invoke<PracticeSnapshot>('typing_snapshot', {
        value,
        milliseconds: Math.round(milliseconds),
      });
    } else {
      nextSnapshot = previewUpdate(snapshot, value, milliseconds);
    }

    if (id === requestId) {
      applySnapshot(nextSnapshot, false);
    }
  }

  function handleInput(event: Event) {
    if (practicePhase !== 'running') {
      if (input && input.value !== draft) input.value = draft;
      return;
    }
    const currentTarget = event.currentTarget as HTMLTextAreaElement;
    const rawValue = currentTarget.value;
    const value = composing ? rawValue : limitPracticeInput(rawValue, target);
    if (value !== rawValue) currentTarget.value = value;
    draft = value;
    if (!composing) {
      const previous = committedDraft;
      committedDraft = value;
      playPracticeKey(previous, value, event.timeStamp);
      void refreshCore(value, itemElapsed());
      input?.setSelectionRange(value.length, value.length);
    }
  }

  function handleCompositionEnd(event: CompositionEvent) {
    if (practicePhase !== 'running') {
      composing = false;
      compositionText = '';
      if (input && input.value !== draft) input.value = draft;
      return;
    }
    composing = false;
    compositionText = '';
    const currentTarget = event.currentTarget as HTMLTextAreaElement;
    const rawValue = currentTarget.value;
    const value = limitPracticeInput(rawValue, target);
    if (value !== rawValue) currentTarget.value = value;
    draft = value;
    const previous = committedDraft;
    committedDraft = value;
    playPracticeKey(previous, value, event.timeStamp);
    void refreshCore(value, itemElapsed());
    input?.setSelectionRange(value.length, value.length);
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.repeat) return;
    if (handlePracticeNavigationKey(event)) return;
    if (event.key === 'Escape') return;
    if (event.key === 'Enter' && chapterPaused) {
      event.preventDefault();
      if (!snapshot.isLastItem) void continueAndStart();
      return;
    }
    if (practicePhase !== 'running') {
      if (chapterPaused || event.altKey || event.ctrlKey || event.metaKey) return;
      if (['Shift', 'Control', 'Alt', 'Meta', 'CapsLock', 'Tab', 'Escape'].includes(event.key)) return;
      event.preventDefault();
      beginPractice();
      return;
    }
    if (event.isComposing || composing || event.keyCode === 229) return;
    speakVisibleWord(snapshot, spokenState !== 'started');
    if (event.key === 'Enter') {
      event.preventDefault();
      const reachedEnd = snapshot.target.length > 0 && characters(committedDraft).length >= characters(snapshot.target).length;
      if (chapterPaused || !stats.done && !reachedEnd) return;
      if (snapshot.isLastItem) {
        practicePhase = 'paused';
        chapterPaused = true;
        return;
      }
      if (snapshot.isChapterLastItem && stats.done) return;
      if (shouldAutoAdvance(snapshot) && stats.done) return;
      void restart(true);
    }
    if (['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Home', 'End'].includes(event.key)) event.preventDefault();
  }

  async function restart(next = false) {
    if (next && itemReachedEnd && !stats.done) {
      // An item left through Enter-with-errors still belongs to the chapter totals.
      rememberCompletedItem(snapshot);
    }
    const keepChapterClock = !(next && snapshot.isChapterLastItem);
    invalidateCompletion();
    requestId += 1;
    allowIncompleteNext = false;
    if (keepChapterClock) itemElapsedOrigin = Math.max(0, Math.round(currentElapsed()));
    else resetPracticeClock();
    composing = false;
    compositionText = '';
    draft = '';
    committedDraft = '';
    try {
      const nextSnapshot = isTauri
        ? await invoke<PracticeSnapshot>(next ? 'advance_practice' : 'reset_practice')
        : next ? previewAdvance(snapshot) : previewReset(snapshot);
      applySnapshot(nextSnapshot, true);
    } catch (error) {
      importError = String(error);
    }
    await tick();
    focusInput();
  }

  async function navigateItem(direction: -1 | 1) {
    if (direction < 0 && !canGoPreviousItem) return;
    if (direction > 0 && (chapterPaused || !canGoNextItem)) return;
    await selectPracticeItem(snapshot.itemIndex - 1 + direction, direction < 0);
  }

  async function selectPracticeItem(itemIndex: number, allowNextAfterLeft: boolean) {
    if (!allowNextAfterLeft && itemReachedEnd && !stats.done) {
      // An item skipped with errors still belongs to the chapter totals.
      rememberCompletedItem(snapshot);
    }
    invalidateCompletion();
    requestId += 1;
    // The chapter clock keeps running across item navigation; only the per-item origin moves.
    itemElapsedOrigin = Math.max(0, Math.round(currentElapsed()));
    composing = false;
    compositionText = '';
    draft = '';
    committedDraft = '';
    chapterPaused = false;
    try {
      const nextSnapshot = isTauri
        ? await invoke<PracticeSnapshot>('select_practice_item', { itemIndex })
        : previewSelectItem(snapshot, itemIndex);
      applySnapshot(nextSnapshot, true);
      allowIncompleteNext = allowNextAfterLeft;
    } catch (error) {
      importError = String(error);
    }
    await tick();
    focusInput();
  }

  async function navigateChapter(direction: -1 | 1) {
    if (direction < 0 && !canGoPreviousChapter) return;
    if (direction > 0 && !canGoNextChapter) return;
    await selectChapter(snapshot.chapterIndex - 1 + direction);
  }

  async function changeMode(mode: PracticeMode) {
    if (snapshot.mode === mode) return;
    invalidateCompletion();
    requestId += 1;
    allowIncompleteNext = false;
    resetPracticeClock();
    try {
      const nextSnapshot = isTauri
        ? await invoke<PracticeSnapshot>('select_practice_mode', { mode })
        : previewRestoredMode(mode);
      if (isTauri) catalog = await invoke<ContentCatalogSnapshot>('get_content_catalog');
      applySnapshot(nextSnapshot, true);
    } catch (error) {
      importError = String(error);
    }
    await tick();
    focusInput();
  }

  async function changeCollection(id: string) {
    invalidateCompletion();
    requestId += 1;
    allowIncompleteNext = false;
    resetPracticeClock();
    importError = '';
    try {
      const nextSnapshot = isTauri
        ? await invoke<PracticeSnapshot>(snapshot.mode === 'sentence' ? 'select_article' : 'select_word_book', { id })
        : previewSelectCollection(snapshot, id);
      if (isTauri) catalog = await invoke<ContentCatalogSnapshot>('get_content_catalog');
      else {
        const key = snapshot.mode === 'sentence' ? 'articles' : 'wordBooks';
        catalog = { ...catalog, [key]: catalog[key].map(item => ({ ...item, selected: item.id === id })) };
      }
      applySnapshot(nextSnapshot, true);
    } catch (error) {
      importError = String(error);
    }
    await tick();
    focusInput();
  }

  async function openLibrary() {
    importError = '';
    rememberSettingsScroll();
    if (isTauri) {
      try {
        catalog = await invoke<ContentCatalogSnapshot>('get_content_catalog');
      } catch (error) {
        importError = String(error);
      }
    }
    libraryOpen = true;
    await restoreSettingsScroll('library');
  }

  function closeLibrary() {
    rememberSettingsScroll();
    libraryOpen = false;
    void restoreSettingsScroll('settings');
  }

  async function selectLibrary(id: string) {
    if (id !== snapshot.collectionId) await changeCollection(id);
    if (!importError) closeLibrary();
  }

  async function removeLibrary(id: string) {
    if (!confirm(t.removeLibraryConfirm)) return;
    if (!isTauri) return;
    invalidateCompletion();
    requestId += 1;
    resetPracticeClock();
    try {
      const next = await invoke<{ catalog: ContentCatalogSnapshot; snapshot: PracticeSnapshot }>(snapshot.mode === 'sentence' ? 'remove_article' : 'remove_word_book', { id });
      catalog = next.catalog;
      applySnapshot(next.snapshot, true);
    } catch (error) {
      importError = String(error);
    }
  }

  function chooseChapter(value: string) {
    const chapter = Number(value);
    if (chapter === snapshot.chapterIndex - 1) {
      dismissChapterPicker();
      return;
    }
    void selectChapter(chapter);
  }

  async function selectChapter(chapter: number) {
    invalidateCompletion();
    requestId += 1;
    allowIncompleteNext = false;
    resetPracticeClock();
    draft = '';
    committedDraft = '';
    try {
      const nextSnapshot = isTauri
        ? await invoke<PracticeSnapshot>('select_word_chapter', { chapter })
        : previewSelectChapter(snapshot, chapter);
      applySnapshot(nextSnapshot, true);
    } catch (error) {
      importError = String(error);
    }
    await tick();
    focusInput();
  }

  async function retryChapter() {
    invalidateCompletion();
    requestId += 1;
    allowIncompleteNext = false;
    resetPracticeClock();
    composing = false;
    compositionText = '';
    draft = '';
    committedDraft = '';
    chapterKey = '';
    try {
      const nextSnapshot = isTauri
        ? await invoke<PracticeSnapshot>('select_word_chapter', { chapter: snapshot.chapterIndex - 1 })
        : previewSelectChapter(snapshot, snapshot.chapterIndex - 1);
      applySnapshot(nextSnapshot, true);
    } catch (error) {
      importError = String(error);
    }
    await tick();
    focusInput();
  }

  async function continueAfterChapter() {
    await continueAndStart();
  }

  async function continueAndStart() {
    if (chapterContinuation || !chapterPaused || snapshot.isLastItem) return;
    chapterContinuation = true;
    practicePhase = 'running';
    try {
      await restart(true);
    } finally {
      chapterContinuation = false;
    }
  }

  function formatDuration(milliseconds: number) {
    const totalSeconds = Math.max(0, Math.floor(milliseconds / 1000));
    return String(Math.floor(totalSeconds / 60)).padStart(2, '0') + ':' + String(totalSeconds % 60).padStart(2, '0');
  }

  async function importContent(kind: 'article' | 'wordBook') {
    if (!isTauri || importLoading) return;
    importLoading = true;
    importMessage = '';
    importError = '';
    try {
      const selected = await open({
        multiple: false,
        directory: false,
        filters: [{ name: kind === 'article' ? 'UTF-8 text' : 'UTF-8 TSV', extensions: kind === 'article' ? ['txt'] : ['tsv', 'txt'] }],
      });
      if (typeof selected !== 'string') return;
      const report = await invoke<ImportReport>(kind === 'article' ? 'preview_article' : 'preview_word_book', { path: selected });
      pendingImport = { kind, path: selected, report };
    } catch (error) {
      importError = String(error);
    } finally {
      importLoading = false;
    }
  }

  async function confirmImport() {
    if (!pendingImport || importLoading) return;
    importLoading = true;
    importError = '';
    try {
      const result = await invoke<ImportResult>(pendingImport.kind === 'article' ? 'import_article' : 'import_word_book', { path: pendingImport.path });
      catalog = result.catalog;
      applySnapshot(result.snapshot, true);
      importMessage = result.report.title + ' · ' + result.report.count + (pendingImport.kind === 'wordBook' ? ' · ' + t.duplicates + ': ' + result.report.duplicateCount : '');
      pendingImport = null;
    } catch (error) {
      importError = String(error);
    } finally {
      importLoading = false;
    }
  }

  function cancelImport() {
    pendingImport = null;
    importError = '';
  }

  async function togglePinned() {
    if (!appWindow) return;
    const next = !pinned;
    pinGeneration += 1;
    pinned = next;
    try {
      await appWindow.setAlwaysOnTop(next);
      if (next) {
        await appWindow.unminimize().catch(() => {});
        await appWindow.setFocus().catch(() => {});
      }
    } catch {
      pinGeneration += 1;
      pinned = !next;
    }
  }

  async function minimize() { await appWindow?.minimize(); }
  async function close() { await appWindow?.close(); }

  async function dragWindow(event: MouseEvent) {
    const targetElement = event.target as HTMLElement | null;
    if (targetElement?.closest('button, select, a, input, textarea')) return;
    await appWindow?.startDragging();
  }

  async function openSettings() {
    focused = false;
    libraryOpen = false;
    view = 'settings';
    if (!fontsLoaded) void loadFonts();
    if (!agentsLoaded) void loadAgents();
    await tick();
    settingsScroll.apply('settings', settingsPage);
    backButton?.focus({ preventScroll: true });
  }
  async function closeSettings() {
    rememberSettingsScroll();
    view = 'practice';
    libraryOpen = false;
    selectedAgent = null;
    agentDetail = null;
    agentDetailDraft = '';
    agentError = '';
    await tick();
    focusInput();
  }

  async function navigateBack() {
    if (selectedAgent) {
      closeAgentDetail();
      return;
    }
    if (libraryOpen) {
      closeLibrary();
      return;
    }
    if (view === 'settings') await closeSettings();
  }
</script>

<svelte:head>
  <title>{t.brand} · {t.subtitle}</title>
</svelte:head>

<section class="widget" role="application" aria-label={t.brand} style:--practice-font={fontStack(fontPreference)} onpointerdown={markSurfacePress}>
  <TitleBar
    {dragWindow}
    {t}
    {snapshot}
    {view}
    {navigateBack}
    {openSettings}
    {isTauri}
    {pinned}
    {togglePinned}
    {minimize}
    {close}
  />

  {#if view === 'settings'}
    <SettingsPage
      bind:settingsPage
      {selectedAgent}
      bind:backButton
      {t}
      {navigateBack}
      {agentDetail}
      {agentDetailLoading}
      bind:agentDetailDraft
      {closeAgentDetail}
      {agentDetailSaving}
      {saveAgentDetail}
      {agentError}
      {libraryOpen}
      {builtinLibraries}
      {importedLibraries}
      {selectLibrary}
      {snapshot}
      {removeLibrary}
      {isTauri}
      {importLoading}
      {importContent}
      {pendingImport}
      {cancelImport}
      {confirmImport}
      {importMessage}
      {importError}
      {libraries}
      {themePreference}
      {persistThemePreference}
      {localePreference}
      {persistLocalePreference}
      {changeMode}
      {chapterLengthMin}
      {chapterLengthMax}
      {effectiveChapterLength}
      {stepChapterLength}
      {commitChapterLength}
      {onChapterLengthKeydown}
      {practiceOrder}
      {choosePracticeOrder}
      {pronunciationPreference}
      {persistPronunciationPreference}
      {autoAdvanceWords}
      {persistAutoAdvanceWords}
      {openLibrary}
      {sound}
      {setSoundEnabled}
      {setSpeechMuted}
      {setSpeechVolume}
      {setKeyMuted}
      {setKeyVolume}
      {keySoundOptions}
      {setKeyPack}
      {keysAudible}
      {previewKeySound}
      {agents}
      {openAgentDetail}
      {agentLoading}
      {toggleAgent}
      {fontDraft}
      {fontPreference}
      {applyFont}
      {fontPreview}
      bind:fontField
      {onFontFocusOut}
      {fontMenuOpen}
      {fontLabel}
      {fontQuery}
      {openFontMenu}
      {onFontInput}
      {onFontKeydown}
      {closeFontMenu}
      {fontMenuMaxHeight}
      {fontChoiceList}
      bind:fontHighlight
      {chooseFont}
      {fontsLoading}
      {fontsFailed}
      {systemFonts}
      {loadFonts}
      {appVersion}
    />
  {:else}
    <PracticePage
      {restorePracticeTypingFocus}
      {chapterPaused}
      {t}
      {canGoPreviousItem}
      {navigateItem}
      {restart}
      {practicePhase}
      {togglePractice}
      {canGoNextItem}
      {snapshot}
      {chapterOptions}
      {chooseChapter}
      {dismissChapterPicker}
      {stats}
      {awaitingStart}
      {chapterResultTitle}
      {chapterTotals}
      {chapterResultUnit}
      {continueAfterChapter}
      {retryChapter}
      bind:textDisplay
      {wordFontSize}
      {expected}
      {typed}
      bind:compositionText
      {selectedPhonetic}
      {selectedPhoneticLabel}
      {replayWord}
      bind:input
      {captureX}
      {captureY}
      {draft}
      {handleFocus}
      bind:focused
      bind:composing
      {handleCompositionEnd}
      {handleInput}
      {handleKeydown}
      {displayWpm}
      {displayAccuracy}
      {formatDuration}
      {displayElapsed}
    />
  {/if}
  <canvas bind:this={celebrationCanvas} class="celebration" aria-hidden="true"></canvas>
</section>
