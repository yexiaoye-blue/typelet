<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import {
    characters,
    freshPractice,
    passages,
    statistics as browserStatistics,
    updatePractice as browserUpdatePractice,
    type Practice,
  } from '../packages/core/typing';
  import { en, zh } from './i18n';
  import { bundledFont, fontStack, getSystemFonts, loadFontPreference, persistFontPreference } from './fonts';

  type Locale = 'zh' | 'en';
  type LocalePreference = 'system' | Locale;
  type Theme = 'light' | 'dark';
  type ThemePreference = 'system' | Theme;
  type TextLanguage = 'zh' | 'en';
  type Statistics = {
    correct: number;
    percent: number;
    accuracy: number;
    cpm: number;
    done: boolean;
  };
  type Snapshot = { practice: Practice; stats: Statistics };
  type AgentId = 'codex' | 'cursor' | 'dsh';
  type AgentSummary = { id: AgentId; name: string; enabled: boolean; supported: boolean; configPath: string; format: string; event: string };
  type AgentDetail = AgentSummary & { editable: boolean; content: string; note: string };

  const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
  const appWindow = isTauri ? getCurrentWindow() : undefined;
  const emptyStatistics: Statistics = { correct: 0, percent: 0, accuracy: 100, cpm: 0, done: false };

  let localePreference: LocalePreference = loadLocalePreference();
  let systemLocale: Locale = detectSystemLocale();
  let locale: Locale = 'en';
  let themePreference: ThemePreference = loadThemePreference();
  let systemTheme: Theme = window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  let resolvedTheme: Theme = 'light';
  let view: 'practice' | 'settings' = 'practice';
  let textLanguage: TextLanguage = 'en';
  let passageIndex = 0;
  let practice: Practice = freshPractice();
  let draft = '';
  let committedDraft = '';
  let compositionText = '';
  let focused = false;
  let elapsed = 0;
  let pinned = false;
  let composing = false;
  let surfacePressed = false;
  let input: HTMLTextAreaElement;
  let backButton: HTMLButtonElement;
  let textDisplay: HTMLDivElement;
  let captureX = 0;
  let captureY = 0;
  let resizeObserver: ResizeObserver | undefined;
  let observedDisplay: HTMLDivElement | undefined;
  let fontPreference = loadFontPreference();
  let fontSearch = '';
  let systemFonts: string[] = [];
  let fontsLoading = false;
  let fontsLoaded = false;
  let fontsFailed = false;
  let agents: AgentSummary[] = [];
  let agentsLoaded = false;
  let agentLoading = false;
  let agentError = '';
  let selectedAgent: AgentId | null = null;
  let agentDetail: AgentDetail | null = null;
  let agentDetailDraft = '';
  let agentDetailLoading = false;
  let agentDetailSaving = false;
  let stats: Statistics = emptyStatistics;
  let clock = { total: 0, since: null as number | null };
  let requestId = 0;

  $: t = locale === 'zh' ? zh : en;
  $: locale = localePreference === 'system' ? systemLocale : localePreference;
  $: resolvedTheme = themePreference === 'system' ? systemTheme : themePreference;
  $: if (typeof document !== 'undefined') {
    document.documentElement.lang = locale === 'zh' ? 'zh-CN' : 'en';
    document.documentElement.dataset.theme = resolvedTheme;
  }
  $: target = passages[textLanguage][passageIndex % passages[textLanguage].length];
  $: expected = characters(target);
  $: typed = characters(committedDraft);
  $: filteredFonts = systemFonts.filter(family => family.toLocaleLowerCase().includes(fontSearch.trim().toLocaleLowerCase()));
  $: running = view === 'practice' && focused && practice.attempts > 0 && !stats.done;
  $: if (view === 'practice') void updateCaret(committedDraft, fontPreference, target, compositionText);
  $: if (view === 'settings' && !agentsLoaded) void loadAgents();

  onMount(() => {
    let timer: ReturnType<typeof setInterval> | undefined;
    const colorScheme = window.matchMedia('(prefers-color-scheme: dark)');
    const syncSystemTheme = () => { systemTheme = colorScheme.matches ? 'dark' : 'light'; };
    const syncSystemLocale = () => { systemLocale = detectSystemLocale(); };
    syncSystemTheme();
    syncSystemLocale();

    const stop = () => { focused = false; };
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
    const hidden = () => { if (document.hidden) stop(); else resume(); };

    window.addEventListener('blur', stop);
    window.addEventListener('pointerup', releaseSurface);
    window.addEventListener('focus', resume);
    window.addEventListener('languagechange', syncSystemLocale);
    document.addEventListener('visibilitychange', hidden);
    colorScheme.addEventListener?.('change', syncSystemTheme);
    const fontLayoutChanged = () => { void updateCaret(); };
    resizeObserver = new ResizeObserver(fontLayoutChanged);
    document.fonts.addEventListener('loadingdone', fontLayoutChanged);
    void tick().then(() => focusInput());

    if (appWindow) {
      void appWindow.isAlwaysOnTop().then(value => { pinned = value; });
    }

    const updateClock = () => {
      if (!running) return;
      const now = performance.now();
      elapsed = clock.total + now - (clock.since ?? now);
      void refreshCore(committedDraft, elapsed);
    };

    timer = setInterval(updateClock, 200);
    return () => {
      if (timer) clearInterval(timer);
      window.removeEventListener('blur', stop);
      window.removeEventListener('pointerup', releaseSurface);
      window.removeEventListener('focus', resume);
      window.removeEventListener('languagechange', syncSystemLocale);
      document.removeEventListener('visibilitychange', hidden);
      colorScheme.removeEventListener?.('change', syncSystemTheme);
      resizeObserver?.disconnect();
      document.fonts.removeEventListener('loadingdone', fontLayoutChanged);
    };
  });

  $: if (running && clock.since === null) {
    clock.since = performance.now();
  } else if (!running && clock.since !== null) {
    clock.total += performance.now() - clock.since;
    clock.since = null;
    elapsed = clock.total;
  }

  function detectSystemLocale(): Locale {
    const language = typeof navigator !== 'undefined' ? navigator.language.toLowerCase() : 'en';
    return language.startsWith('zh') ? 'zh' : 'en';
  }

  function loadLocalePreference(): LocalePreference {
    try {
      const value = localStorage.getItem('typelet.locale.preference');
      return value === 'zh' || value === 'en' || value === 'system' ? value : 'system';
    } catch { return 'system'; }
  }

  function persistLocalePreference(value: LocalePreference) {
    localePreference = value;
    try { localStorage.setItem('typelet.locale.preference', value); } catch { /* browser storage may be unavailable */ }
  }

  function loadThemePreference(): ThemePreference {
    try {
      const value = localStorage.getItem('typelet.theme.preference');
      return value === 'light' || value === 'dark' || value === 'system' ? value : 'system';
    } catch { return 'system'; }
  }

  function persistThemePreference(value: ThemePreference) {
    themePreference = value;
    try { localStorage.setItem('typelet.theme.preference', value); } catch { /* browser storage may be unavailable */ }
  }

  function selectFont(value: string) {
    fontPreference = value;
    persistFontPreference(value);
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

  async function openAgentDetail(agent: AgentSummary) {
    selectedAgent = agent.id;
    agentDetail = null;
    agentDetailDraft = '';
    agentDetailLoading = true;
    agentError = '';
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
    selectedAgent = null;
    agentDetail = null;
    agentDetailDraft = '';
    agentError = '';
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

  function markSurfacePress(event: PointerEvent) {
    const origin = event.target as HTMLElement | null;
    surfacePressed = view === 'practice' && !origin?.closest('select');
  }

  function handleFocus() {
    focused = true;
    if (!composing) input.setSelectionRange(input.value.length, input.value.length);
  }

  async function updateCaret(..._changes: unknown[]) {
    await tick();
    if (view !== 'practice' || !textDisplay || !input) return;
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
    character = current.getBoundingClientRect();
    const surface = textDisplay.parentElement!.getBoundingClientRect();
    captureX = Math.max(0, Math.min(character.left - surface.left, surface.width - 2));
    captureY = Math.max(0, Math.min(character.top - surface.top, surface.height - 24));
  }

  async function refreshCore(value: string, milliseconds: number) {
    const id = ++requestId;
    let snapshot: Snapshot;

    if (isTauri) {
      snapshot = await invoke<Snapshot>('typing_snapshot', {
        previous: practice,
        value,
        target,
        milliseconds: Math.round(milliseconds),
      });
    } else {
      const nextPractice = browserUpdatePractice(practice, value, target);
      snapshot = { practice: nextPractice, stats: browserStatistics(nextPractice, target, milliseconds) };
    }

    if (id === requestId) {
      practice = snapshot.practice;
      stats = snapshot.stats;
    }
  }

  function handleInput(event: Event) {
    const value = (event.currentTarget as HTMLTextAreaElement).value;
    draft = value;
    if (!composing) {
      committedDraft = value;
      void refreshCore(value, elapsed);
      input.setSelectionRange(value.length, value.length);
    }
  }

  function handleCompositionEnd(event: CompositionEvent) {
    composing = false;
    compositionText = '';
    const value = (event.currentTarget as HTMLTextAreaElement).value;
    draft = value;
    committedDraft = value;
    void refreshCore(value, elapsed);
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.isComposing || composing || event.keyCode === 229 || event.repeat) return;
    const complete = committedDraft.normalize('NFC') === target.normalize('NFC');
    if (event.key === 'Enter') {
      event.preventDefault();
      if (complete) restart(true);
    }
    if (['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Home', 'End'].includes(event.key)) event.preventDefault();
  }

  function restart(next = false, language = textLanguage) {
    const previousLanguage = textLanguage;
    requestId += 1;
    clock = { total: 0, since: null };
    composing = false;
    compositionText = '';
    elapsed = 0;
    draft = '';
    committedDraft = '';
    practice = freshPractice();
    stats = emptyStatistics;
    textLanguage = language;
    passageIndex = next ? passageIndex + 1 : language !== previousLanguage ? 0 : passageIndex;
    void tick().then(() => focusInput());
  }

  async function togglePinned() {
    if (!appWindow) return;
    try {
      pinned = !pinned;
      await appWindow.setAlwaysOnTop(pinned);
    } catch {
      pinned = !pinned;
    }
  }

  async function minimize() { await appWindow?.minimize(); }
  async function close() { await appWindow?.close(); }

  async function dragWindow(event: MouseEvent) {
    const targetElement = event.target as HTMLElement;
    if (targetElement.closest('button, select')) return;
    await appWindow?.startDragging();
  }

  async function openSettings() {
    focused = false;
    view = 'settings';
    if (!fontsLoaded) void loadFonts();
    if (!agentsLoaded) void loadAgents();
    await tick();
    backButton?.focus();
  }
  async function closeSettings() {
    view = 'practice';
    await tick();
    focusInput();
  }
</script>

<svelte:head>
  <title>{t.brand} · {t.subtitle}</title>
</svelte:head>

<section class="widget" role="application" aria-label={t.brand} style:--practice-font={fontStack(fontPreference)} onpointerdown={markSurfacePress}>
  <header class="global-nav" role="presentation" onmousedown={dragWindow}>
    <span class="brand"><span class="brand-mark">T</span><span>{t.brand}</span></span>
    <div class="window-actions">
      <button class="settings-button" aria-label={t.settings} title={t.settings} aria-pressed={view === 'settings'} onclick={() => view === 'settings' ? closeSettings() : openSettings()}>
        <svg viewBox="0 0 20 20" aria-hidden="true"><path d="m8 3 .5-1h3l.5 1 .5 1 1 .5 1-.5 2 2-.5 1 .5 1 1 .5 1 .5v3l-1 .5-1 .5-.5 1 .5 1-2 2-1-.5-1 .5-.5 1-.5 1h-3l-.5-1-.5-1-1-.5-1 .5-2-2 .5-1-.5-1-1-.5-1-.5v-3l1-.5 1-.5.5-1-.5-1 2-2 1 .5 1-.5.5-1Z" /><circle cx="10" cy="10" r="2.5" /></svg>
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

  {#if view === 'settings'}
    <main class="settings-page">
      <div class="settings-heading">
        <button bind:this={backButton} class="back-button" aria-label={t.back} title={t.back} onclick={closeSettings}>←</button>
        <h1>{t.settingsTitle}</h1>
      </div>

      <section class="settings-group" aria-label={t.theme}>
        <div class="settings-copy"><strong>{t.theme}</strong><span>{t.themeHint}</span></div>
        <div class="segmented-control">
          <button class:selected={themePreference === 'system'} aria-pressed={themePreference === 'system'} onclick={() => persistThemePreference('system')}>{t.system}</button>
          <button class:selected={themePreference === 'light'} aria-pressed={themePreference === 'light'} onclick={() => persistThemePreference('light')}>{t.light}</button>
          <button class:selected={themePreference === 'dark'} aria-pressed={themePreference === 'dark'} onclick={() => persistThemePreference('dark')}>{t.dark}</button>
        </div>
      </section>

      <section class="settings-group" aria-label={t.interfaceLanguage}>
        <div class="settings-copy"><strong>{t.interfaceLanguage}</strong><span>{t.languageHint}</span></div>
        <div class="segmented-control">
          <button class:selected={localePreference === 'system'} aria-pressed={localePreference === 'system'} onclick={() => persistLocalePreference('system')}>{t.system}</button>
          <button class:selected={localePreference === 'zh'} aria-pressed={localePreference === 'zh'} onclick={() => persistLocalePreference('zh')}>{t.chinese}</button>
          <button class:selected={localePreference === 'en'} aria-pressed={localePreference === 'en'} onclick={() => persistLocalePreference('en')}>{t.english}</button>
        </div>
      </section>

      {#if selectedAgent}
        <section class="settings-group agent-detail" aria-label={t.agentDetail}>
          <div class="detail-heading">
            <button class="detail-back" aria-label={t.agentBack} onclick={closeAgentDetail}>←</button>
            <div class="settings-copy"><strong>{agentDetail?.name ?? t.agentLoading}</strong><span>{agentDetail?.event ?? ''}</span></div>
          </div>
          {#if agentDetailLoading}
            <p class="agent-note">{t.agentLoading}</p>
          {:else if agentDetail}
            <div class="detail-meta"><span>{t.agentConfigPath}</span><code>{agentDetail.configPath || t.agentUnavailable}</code></div>
            <div class="detail-meta"><span>{t.agentFormat}</span><code>{agentDetail.format}</code></div>
            <p class="agent-note">{agentDetail.note}</p>
            {#if agentDetail.editable}
              <textarea class="agent-editor" aria-label={t.agentEditor} bind:value={agentDetailDraft} spellcheck="false"></textarea>
              <div class="detail-actions"><button class="secondary" onclick={closeAgentDetail}>{t.cancel}</button><button class="primary" disabled={agentDetailSaving} onclick={saveAgentDetail}>{agentDetailSaving ? t.saving : t.save}</button></div>
            {:else}
              <p class="agent-note">{t.agentUnavailable}</p>
            {/if}
          {/if}
          {#if agentError}<span class="agent-error" role="status">{agentError}</span>{/if}
        </section>
      {:else}
        <section class="settings-group agent-settings" aria-label={t.agents}>
          <div class="settings-copy"><strong>{t.agents}</strong><span>{t.agentsHint}</span></div>
          {#each agents as agent}
            <div class="agent-row" class:unsupported={!agent.supported}>
              <button class="agent-copy agent-open" aria-label={t.agentDetails + ': ' + agent.name} onclick={() => openAgentDetail(agent)}>
                <strong>{agent.name}</strong><span>{agent.supported ? agent.event + ' · ' + agent.format : t.agentUnavailable}</span>
              </button>
              <button class="agent-switch" class:on={agent.enabled} role="switch" aria-checked={agent.enabled} aria-label={agent.name + ' ' + (agent.enabled ? t.enabled : t.disabled)} disabled={!agent.supported || agentLoading} onclick={() => toggleAgent(agent)}><span></span></button>
            </div>
          {/each}
          {#if agentError}<span class="agent-error" role="status">{agentError}</span>{/if}
        </section>
      {/if}

      <section class="settings-group font-settings" aria-label={t.font}>
        <div class="settings-copy"><strong>{t.font}</strong><span>{t.fontHint}</span></div>
        <input class="font-search" type="search" aria-label={t.fontSearch} placeholder={t.fontSearch} bind:value={fontSearch} />
        <select class="font-select" aria-label={t.font} value={fontPreference} onchange={(event) => selectFont((event.currentTarget as HTMLSelectElement).value)}>
          <option value={bundledFont}>Noto Sans Mono · {t.builtIn}</option>
          <option value="system">{t.systemFont}</option>
          {#if fontPreference.startsWith('font:') && !filteredFonts.includes(fontPreference.slice(5))}
            <option value={fontPreference}>{fontPreference.slice(5)}</option>
          {/if}
          <optgroup label={t.systemFonts}>
            {#each filteredFonts as family}
              <option value={`font:${family}`}>{family}</option>
            {/each}
          </optgroup>
        </select>
        <div class="font-sample" aria-label={t.fontSample}>Aa Bb 123 · 中文</div>
        <div class="font-list-status">
          <span role="status">{fontsLoading ? t.fontLoading : fontsFailed ? t.fontError : `${t.fontCount}: ${systemFonts.length}`}</span>
          <button disabled={fontsLoading} onclick={loadFonts}>{t.refresh}</button>
        </div>
      </section>
    </main>
  {:else}
    <main class="practice-page">
      <section class="summary">
        <div class="stats">
          <div><strong>{stats.cpm}</strong><span>{t.speed}</span></div>
          <div><strong>{stats.accuracy}<small>%</small></strong><span>{t.accuracy}</span></div>
          <div><strong>{String(Math.floor(elapsed / 60000)).padStart(2, '0')}:{String(Math.floor(elapsed / 1000) % 60).padStart(2, '0')}</strong><span>{t.time}</span></div>
        </div>
      </section>

      <section class="practice-tile">
        <div class="passage-heading">
          <select aria-label={t.practiceLanguage} value={textLanguage} onchange={(event) => restart(false, (event.currentTarget as HTMLSelectElement).value as TextLanguage)}>
            <option value="en">{t.english}</option>
            <option value="zh">{t.chinese}</option>
          </select>
        </div>

        <label class="typing-surface" class:focused class:complete={stats.done} for="typing-capture" ondrop={(event) => event.preventDefault()} ondragover={(event) => event.preventDefault()}>
          <div bind:this={textDisplay} id="passage-text" class="target" aria-label={t.target}>
            {#each expected as char, index}
              {#if index === typed.length && compositionText}<span class="composition-preview">{compositionText}</span>{/if}<span class:correct={index < typed.length && typed[index] === char} class:incorrect={index < typed.length && typed[index] !== char} class:caret={index === typed.length && !stats.done}>{char}</span>
            {/each}
            {#if typed.length > expected.length}<span class="incorrect caret">{typed.slice(expected.length).join('')}</span>{/if}
          </div>
          <textarea bind:this={input} id="typing-capture" class="typing-capture" aria-label={t.input} aria-describedby="passage-text" style:left={`${captureX}px`} style:top={`${captureY}px`} value={draft} spellcheck="false" autocapitalize="off" autocorrect="off" readonly={stats.done}
          onfocus={handleFocus} onblur={() => focused = false}
          onpaste={(event) => event.preventDefault()}
          oncompositionstart={() => { composing = true; compositionText = ''; }} oncompositionupdate={(event) => compositionText = event.data}
          oncompositionend={handleCompositionEnd} oninput={handleInput} onkeydown={handleKeydown}></textarea>
        </label>
        <progress class="practice-progress" aria-label={t.progress} value={stats.percent} max="100"></progress>
      </section>

      <footer><button class="secondary" onclick={() => restart()}>{t.restart}</button><button class="primary" title={stats.done ? t.enterNext : t.next} onclick={() => restart(true)}>{t.next}{#if stats.done}<kbd>↵</kbd>{:else}<span aria-hidden="true">→</span>{/if}</button></footer>
    </main>
  {/if}
</section>
