<script lang="ts">
  import SettingsSelect from '../components/SettingsSelect.svelte';
  import { fontStack } from '../fonts';

  let {
    settingsPage = $bindable(),
    selectedAgent,
    backButton = $bindable(),
    t,
    navigateBack,
    agentDetail,
    agentDetailLoading,
    agentDetailDraft = $bindable(),
    closeAgentDetail,
    agentDetailSaving,
    saveAgentDetail,
    agentError,
    libraryOpen,
    builtinLibraries,
    importedLibraries,
    selectLibrary,
    snapshot,
    removeLibrary,
    isTauri,
    importLoading,
    importContent,
    pendingImport,
    cancelImport,
    confirmImport,
    importMessage,
    importError,
    libraries,
    themePreference,
    persistThemePreference,
    localePreference,
    persistLocalePreference,
    changeMode,
    chapterLengthMin,
    chapterLengthMax,
    effectiveChapterLength,
    stepChapterLength,
    commitChapterLength,
    onChapterLengthKeydown,
    practiceOrder,
    choosePracticeOrder,
    pronunciationPreference,
    persistPronunciationPreference,
    autoAdvanceWords,
    persistAutoAdvanceWords,
    openLibrary,
    sound,
    setSoundEnabled,
    setSpeechMuted,
    setSpeechVolume,
    setKeyMuted,
    setKeyVolume,
    keySoundOptions,
    setKeyPack,
    keysAudible,
    previewKeySound,
    agents,
    openAgentDetail,
    agentLoading,
    toggleAgent,
    fontDraft,
    fontPreference,
    applyFont,
    fontPreview,
    fontField = $bindable(),
    onFontFocusOut,
    fontMenuOpen,
    fontLabel,
    fontQuery,
    openFontMenu,
    onFontInput,
    onFontKeydown,
    closeFontMenu,
    fontMenuMaxHeight,
    fontChoiceList,
    fontHighlight = $bindable(),
    chooseFont,
    fontsLoading,
    fontsFailed,
    systemFonts,
    loadFonts,
    appVersion,
  }: {
    settingsPage: any;
    selectedAgent: any;
    backButton: any;
    t: any;
    navigateBack: any;
    agentDetail: any;
    agentDetailLoading: any;
    agentDetailDraft: any;
    closeAgentDetail: any;
    agentDetailSaving: any;
    saveAgentDetail: any;
    agentError: any;
    libraryOpen: any;
    builtinLibraries: any;
    importedLibraries: any;
    selectLibrary: any;
    snapshot: any;
    removeLibrary: any;
    isTauri: any;
    importLoading: any;
    importContent: any;
    pendingImport: any;
    cancelImport: any;
    confirmImport: any;
    importMessage: any;
    importError: any;
    libraries: any;
    themePreference: any;
    persistThemePreference: any;
    localePreference: any;
    persistLocalePreference: any;
    changeMode: any;
    chapterLengthMin: any;
    chapterLengthMax: any;
    effectiveChapterLength: any;
    stepChapterLength: any;
    commitChapterLength: any;
    onChapterLengthKeydown: any;
    practiceOrder: any;
    choosePracticeOrder: any;
    pronunciationPreference: any;
    persistPronunciationPreference: any;
    autoAdvanceWords: any;
    persistAutoAdvanceWords: any;
    openLibrary: any;
    sound: any;
    setSoundEnabled: any;
    setSpeechMuted: any;
    setSpeechVolume: any;
    setKeyMuted: any;
    setKeyVolume: any;
    keySoundOptions: any;
    setKeyPack: any;
    keysAudible: any;
    previewKeySound: any;
    agents: any;
    openAgentDetail: any;
    agentLoading: any;
    toggleAgent: any;
    fontDraft: any;
    fontPreference: any;
    applyFont: any;
    fontPreview: any;
    fontField: any;
    onFontFocusOut: any;
    fontMenuOpen: any;
    fontLabel: any;
    fontQuery: any;
    openFontMenu: any;
    onFontInput: any;
    onFontKeydown: any;
    closeFontMenu: any;
    fontMenuMaxHeight: any;
    fontChoiceList: any;
    fontHighlight: any;
    chooseFont: any;
    fontsLoading: any;
    fontsFailed: any;
    systemFonts: any;
    loadFonts: any;
    appVersion: any;
  } = $props();
</script>

    <main class="settings-page" bind:this={settingsPage}>
      {#if selectedAgent}
        <div class="settings-heading">
          <button bind:this={backButton} class="back-button" aria-label={t.agentBack} title={t.agentBack} onclick={navigateBack}>←</button>
          <h1>{agentDetail?.name ?? t.agentLoading}</h1>
        </div>
        <section class="settings-group agent-detail" aria-label={t.agentDetail}>
          {#if agentDetailLoading}
            <p class="agent-note">{t.agentLoading}</p>
          {:else if agentDetail}
            <p class="agent-note">{agentDetail.event}</p>
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
      {:else if libraryOpen}
        <div class="settings-heading">
          <button bind:this={backButton} class="back-button" aria-label={t.back} title={t.back} onclick={navigateBack}>←</button>
          <h1>{t.library}</h1>
        </div>
        <p class="library-intro">{t.libraryHint}</p>
        {#if builtinLibraries.length}
          {#if importedLibraries.length}<h2 class="library-heading">{t.libraryBuiltin}</h2>{/if}
          <section class="settings-group" aria-label={t.libraryBuiltin}>
            {#each builtinLibraries as item (item.id)}
              <div class="library-row">
                <button class="library-main" style="--progress: {item.progress}" onclick={() => selectLibrary(item.id)}>
                  <strong>{item.title}</strong>
                  <span>{item.count} {snapshot.mode === 'sentence' ? t.sentenceUnit : t.wordUnit} · {item.progress}%</span>
                </button>
                {#if item.selected}<span class="library-check" aria-hidden="true">✓</span>{/if}
              </div>
            {/each}
          </section>
        {/if}
        {#if importedLibraries.length}
          <h2 class="library-heading">{t.libraryImported}</h2>
          <section class="settings-group" aria-label={t.libraryImported}>
            {#each importedLibraries as item (item.id)}
              <div class="library-row">
                <button class="library-main" style="--progress: {item.progress}" onclick={() => selectLibrary(item.id)}>
                  <strong>{item.title}</strong>
                  <span>{item.count} {snapshot.mode === 'sentence' ? t.sentenceUnit : t.wordUnit} · {item.progress}%</span>
                </button>
                {#if item.selected}<span class="library-check" aria-hidden="true">✓</span>{/if}
                <button class="library-remove" aria-label={t.removeLibrary + ' ' + item.title} onclick={() => removeLibrary(item.id)}>{t.removeLibrary}</button>
              </div>
            {/each}
          </section>
        {/if}
        {#if isTauri}
          <div class="detail-actions">
            {#if snapshot.mode === 'sentence'}
              <button class="secondary" disabled={importLoading} onclick={() => importContent('article')}>{t.importArticle}</button>
            {:else}
              <button class="secondary" disabled={importLoading} onclick={() => importContent('wordBook')}>{t.importWordBook}</button>
            {/if}
          </div>
        {:else}
          <p class="agent-note">{t.previewImportHint}</p>
        {/if}
        {#if pendingImport}
          <div class="import-preview">
            <strong>{pendingImport.report.title}</strong>
            <span>{t.importPreview}: {pendingImport.report.count} · {pendingImport.report.firstItem ?? t.missing}</span>
            {#if pendingImport.kind === 'wordBook'}<span>{t.duplicates}: {pendingImport.report.duplicateCount}</span>{/if}
            <div class="detail-actions"><button class="secondary" onclick={cancelImport}>{t.cancel}</button><button class="primary" disabled={importLoading} onclick={() => void confirmImport()}>{t.confirmImport}</button></div>
          </div>
        {/if}
        {#if importMessage}<p class="agent-note" role="status">{importMessage}</p>{/if}
        {#if importError}<p class="agent-error" role="alert">{importError}</p>{/if}
        {#if snapshot.mode === 'word'}
          <p class="agent-note">{t.dictionaryNotice}</p>
        {:else if libraries.find((item) => item.selected)?.source}
          <p class="agent-note">{libraries.find((item) => item.selected)?.source}</p>
        {/if}
      {:else}
      <div class="settings-heading">
        <button bind:this={backButton} class="back-button" aria-label={t.back} title={t.back} onclick={navigateBack}>←</button>
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

      <section class="settings-group content-settings" aria-label={t.contentSettings}>
        <div class="settings-copy"><strong>{t.practiceMode}</strong><span>{t.contentHint}</span></div>
        <div class="segmented-control two-options" role="group" aria-label={t.practiceMode}>
          <button class:selected={snapshot.mode === 'sentence'} aria-pressed={snapshot.mode === 'sentence'} onclick={() => changeMode('sentence')}>{t.sentenceMode}</button>
          <button class:selected={snapshot.mode === 'word'} aria-pressed={snapshot.mode === 'word'} onclick={() => changeMode('word')}>{t.wordMode}</button>
        </div>
        <div class="setting-block">
          <div class="settings-copy">
            <strong>{t.chapterLength}</strong>
            <span>{t.chapterLengthHint}<span class="chapter-length-bounds"> · {chapterLengthMin}–{chapterLengthMax}</span></span>
          </div>
          <div class="chapter-stepper" class:is-disabled={chapterLengthMin === chapterLengthMax} role="group" aria-label={t.chapterLength}>
            <button type="button" aria-label={t.decreaseChapterLength} disabled={chapterLengthMin === chapterLengthMax || effectiveChapterLength <= chapterLengthMin} onclick={() => stepChapterLength(-1)}>−</button>
            <input inputmode="numeric" autocomplete="off" spellcheck="false" value={effectiveChapterLength} disabled={chapterLengthMin === chapterLengthMax} aria-label={t.chapterLength} onchange={(event) => commitChapterLength(event.currentTarget as HTMLInputElement)} onkeydown={onChapterLengthKeydown} />
            <button type="button" aria-label={t.increaseChapterLength} disabled={chapterLengthMin === chapterLengthMax || effectiveChapterLength >= chapterLengthMax} onclick={() => stepChapterLength(1)}>+</button>
          </div>
        </div>
        <div class="setting-block">
          <div class="settings-copy"><strong>{t.practiceOrder}</strong><span>{t.practiceOrderHint}</span></div>
          <div class="segmented-control two-options" role="group" aria-label={t.practiceOrder}>
            <button class:selected={practiceOrder === 'book'} aria-pressed={practiceOrder === 'book'} onclick={() => choosePracticeOrder('book')}>{t.bookOrder}</button>
            <button class:selected={practiceOrder === 'random'} aria-pressed={practiceOrder === 'random'} onclick={() => choosePracticeOrder('random')}>{t.randomOrder}</button>
          </div>
        </div>
        {#if snapshot.mode === 'word'}
          <div class="setting-block">
            <div class="settings-copy"><strong>{t.wordPronunciation}</strong><span>{t.wordPronunciationHint}</span></div>
            <div class="segmented-control two-options" role="group" aria-label={t.wordPronunciation}>
              <button class:selected={pronunciationPreference === 'uk'} aria-pressed={pronunciationPreference === 'uk'} onclick={() => persistPronunciationPreference('uk')}>{t.britishPronunciation}</button>
              <button class:selected={pronunciationPreference === 'us'} aria-pressed={pronunciationPreference === 'us'} onclick={() => persistPronunciationPreference('us')}>{t.americanPronunciation}</button>
            </div>
          </div>
          <div class="setting-row">
            <div class="settings-copy"><strong>{t.autoAdvanceWords}</strong><span>{t.autoAdvanceWordsHint}</span></div>
            <button class="agent-switch" class:on={autoAdvanceWords} role="switch" aria-checked={autoAdvanceWords} aria-label={t.autoAdvanceWords} onclick={() => persistAutoAdvanceWords(!autoAdvanceWords)}><span></span></button>
          </div>
        {/if}
        <button class="nav-row" onclick={openLibrary}>
          <strong>{t.library}</strong>
          <span class="nav-value">{snapshot.collectionTitle}</span>
          <span class="chevron" aria-hidden="true">›</span>
        </button>
      </section>


      <section class="settings-group" aria-label={t.soundSettings}>
        <div class="setting-row sound-header">
          <div class="settings-copy"><strong>{t.soundSettings}</strong><span>{t.soundSettingsHint}</span></div>
          <button type="button" class="agent-switch" class:on={sound.enabled} role="switch" aria-checked={sound.enabled} aria-label={t.soundSettings} onclick={() => setSoundEnabled(!sound.enabled)}><span></span></button>
        </div>
        <div class="setting-block" class:is-disabled={!sound.enabled}>
          <div class="volume-control" class:is-muted={sound.speechMuted} style:--volume={sound.speechVolume + '%'}>
            <span id="reading-volume">{t.speechVolume}</span>
            <button type="button" class="speaker speaker-inline mute-button" class:is-muted={sound.speechMuted} aria-pressed={sound.speechMuted} aria-label={sound.speechMuted ? t.unmuteReading : t.muteReading} disabled={!sound.enabled} onclick={() => setSpeechMuted(!sound.speechMuted)}>
              {#if sound.speechMuted}
                <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 9.5h3.2L12 5v14l-4.8-4.5H4v-5Z" /><path d="M16 9.5 21 14.5M21 9.5 16 14.5" /></svg>
              {:else}
                <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 9.5h3.2L12 5v14l-4.8-4.5H4v-5Z" /><path d="M16 9.2a3.8 3.8 0 0 1 0 5.6M18.4 6.8a7 7 0 0 1 0 10.4" /></svg>
              {/if}
            </button>
            <input type="range" min="0" max="100" step="1" value={sound.speechVolume} aria-labelledby="reading-volume" disabled={!sound.enabled} oninput={setSpeechVolume} />
            <span class="volume-value">{sound.speechVolume}%</span>
          </div>
        </div>
        <div class="setting-block" class:is-disabled={!sound.enabled}>
          <div class="volume-control" class:is-muted={sound.keyMuted} style:--volume={sound.keyVolume + '%'}>
            <span id="key-volume">{t.keySound}</span>
            <button type="button" class="speaker speaker-inline mute-button" class:is-muted={sound.keyMuted} aria-pressed={sound.keyMuted} aria-label={sound.keyMuted ? t.unmuteKeySound : t.muteKeySound} disabled={!sound.enabled} onclick={() => setKeyMuted(!sound.keyMuted)}>
              {#if sound.keyMuted}
                <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 9.5h3.2L12 5v14l-4.8-4.5H4v-5Z" /><path d="M16 9.5 21 14.5M21 9.5 16 14.5" /></svg>
              {:else}
                <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 9.5h3.2L12 5v14l-4.8-4.5H4v-5Z" /><path d="M16 9.2a3.8 3.8 0 0 1 0 5.6M18.4 6.8a7 7 0 0 1 0 10.4" /></svg>
              {/if}
            </button>
            <input type="range" min="0" max="100" step="1" value={sound.keyVolume} aria-labelledby="key-volume" disabled={!sound.enabled} oninput={setKeyVolume} />
            <span class="volume-value">{sound.keyVolume}%</span>
          </div>
          <p class="sound-note">{t.keySoundHint}</p>
          <div class="pack-control">
            <span>{t.keySoundPack}</span>
            <div class="sound-picker-row">
              <SettingsSelect value={sound.pack} options={keySoundOptions} ariaLabel={t.keySoundPack} disabled={!sound.enabled} onSelect={setKeyPack} />
              <button type="button" class="speaker sound-preview-button" aria-label={t.previewKeySound} title={t.previewKeySound} disabled={!keysAudible(sound)} onclick={() => previewKeySound(true)}>
                <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 9.5h3.2L12 5v14l-4.8-4.5H4v-5Z" /><path d="M16 9.2a3.8 3.8 0 0 1 0 5.6M18.4 6.8a7 7 0 0 1 0 10.4" /></svg>
              </button>
            </div>
          </div>
        </div>
      </section>

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

      <section class="settings-group font-settings" aria-label={t.font}>
        <div class="font-heading">
          <div class="settings-copy"><strong>{t.font}</strong><span>{t.fontHint}</span></div>
          <button type="button" class="font-apply" disabled={fontDraft === fontPreference} onclick={applyFont}>{t.applyFont}</button>
        </div>
        <div class="font-sample" aria-label={t.fontSample} style:font-family={fontStack(fontPreview)}>Aa Bb 123 · 中文</div>
        <div class="font-combobox" bind:this={fontField} onfocusout={onFontFocusOut}>
          <div class="font-field">
            <input
              role="combobox"
              aria-label={t.fontSearch}
              aria-expanded={fontMenuOpen}
              aria-controls="font-menu"
              aria-autocomplete="list"
              placeholder={fontMenuOpen ? fontLabel(fontDraft) : t.fontSearch}
              value={fontMenuOpen ? fontQuery : fontLabel(fontDraft)}
              onfocus={openFontMenu}
              oninput={onFontInput}
              onkeydown={onFontKeydown}
            />
            <button type="button" class="font-disclosure" aria-label={t.font} aria-expanded={fontMenuOpen} onclick={() => fontMenuOpen ? closeFontMenu() : openFontMenu()}>{fontMenuOpen ? '⌃' : '⌄'}</button>
          </div>
          {#if fontMenuOpen}
            <div id="font-menu" class="font-menu" role="listbox" aria-label={t.font} style:max-height={fontMenuMaxHeight + 'px'}>
              {#each fontChoiceList as choice, index (choice.value)}
                <button type="button" role="option" aria-selected={choice.value === fontDraft} class:active={index === fontHighlight} onmouseenter={() => fontHighlight = index} onclick={() => chooseFont(choice.value)}>{choice.label}</button>
              {:else}
                <p class="font-empty">{t.fontEmpty}</p>
              {/each}
            </div>
          {/if}
        </div>
        <div class="font-list-status">
          <span role="status">{fontsLoading ? t.fontLoading : fontsFailed ? t.fontError : `${t.fontCount}: ${systemFonts.length}`}</span>
          <button disabled={fontsLoading} onclick={loadFonts}>{t.refresh}</button>
        </div>
      </section>

      <section class="settings-group version-settings" aria-label={t.version}>
        <div class="version-row">
          <span>{t.version}</span>
          <span class="version-value">{appVersion}</span>
        </div>
      </section>
      {/if}
    </main>
