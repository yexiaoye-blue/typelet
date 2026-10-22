<script lang="ts">
  import SettingsSelect from '../components/SettingsSelect.svelte';

  let {
    restorePracticeTypingFocus,
    chapterPaused,
    t,
    canGoPreviousItem,
    navigateItem,
    restart,
    practicePhase,
    togglePractice,
    canGoNextItem,
    snapshot,
    chapterOptions,
    chooseChapter,
    dismissChapterPicker,
    stats,
    awaitingStart,
    chapterResultTitle,
    chapterTotals,
    chapterResultUnit,
    continueAfterChapter,
    retryChapter,
    textDisplay = $bindable(),
    wordFontSize,
    expected,
    typed,
    compositionText = $bindable(),
    selectedPhonetic,
    selectedPhoneticLabel,
    replayWord,
    input = $bindable(),
    captureX,
    captureY,
    draft,
    handleFocus,
    focused = $bindable(),
    composing = $bindable(),
    handleCompositionEnd,
    handleInput,
    handleKeydown,
    displayWpm,
    displayAccuracy,
    formatDuration,
    displayElapsed,
  }: {
    restorePracticeTypingFocus: any;
    chapterPaused: any;
    t: any;
    canGoPreviousItem: any;
    navigateItem: any;
    restart: any;
    practicePhase: any;
    togglePractice: any;
    canGoNextItem: any;
    snapshot: any;
    chapterOptions: any;
    chooseChapter: any;
    dismissChapterPicker: any;
    stats: any;
    awaitingStart: any;
    chapterResultTitle: any;
    chapterTotals: any;
    chapterResultUnit: any;
    continueAfterChapter: any;
    retryChapter: any;
    textDisplay: any;
    wordFontSize: any;
    expected: any;
    typed: any;
    compositionText: any;
    selectedPhonetic: any;
    selectedPhoneticLabel: any;
    replayWord: any;
    input: any;
    captureX: any;
    captureY: any;
    draft: any;
    handleFocus: any;
    focused: any;
    composing: any;
    handleCompositionEnd: any;
    handleInput: any;
    handleKeydown: any;
    displayWpm: any;
    displayAccuracy: any;
    formatDuration: any;
    displayElapsed: any;
  } = $props();
</script>

    <main class="practice-page" onmousedown={restorePracticeTypingFocus} onclick={restorePracticeTypingFocus}>
      <section class="practice-tile">
        <div class="passage-heading">
          {#if !chapterPaused}
            <div class="practice-controls">
              <button type="button" class="navigation-button" aria-label={t.previousItem} title={t.previousItem} disabled={!canGoPreviousItem} onclick={() => void navigateItem(-1)}>
                <svg viewBox="0 0 24 24" aria-hidden="true"><path d="m14 5-6 7 6 7" /></svg>
              </button>
              <button type="button" class="icon-button" aria-label={t.restart} title={t.restart} onclick={() => void restart()}>
                <svg viewBox="0 0 24 24" aria-hidden="true"><g transform="translate(12 12) scale(0.72) translate(-12 -12)"><path class="stroke heavy" d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" /><path class="stroke heavy" d="M3 3v5h5" /></g></svg>
              </button>
              <button type="button" class="icon-button" aria-label={practicePhase === 'running' ? t.pausePractice : t.startPractice} title={practicePhase === 'running' ? t.pausePractice : t.startPractice} onclick={togglePractice}>
                {#if practicePhase === 'running'}
                  <svg viewBox="0 0 24 24" aria-hidden="true"><path class="stroke" d="M9 5.4v13.2M15 5.4v13.2" /></svg>
                {:else}
                  <svg viewBox="0 0 24 24" aria-hidden="true"><path class="stroke" d="M9.2 5.6v12.8L18.4 12 9.2 5.6z" /></svg>
                {/if}
              </button>
              <button type="button" class="navigation-button" aria-label={t.nextItem} title={t.nextItem} disabled={!canGoNextItem} onclick={() => void navigateItem(1)}>
                <svg viewBox="0 0 24 24" aria-hidden="true"><path d="m10 5 6 7-6 7" /></svg>
              </button>
            </div>
          {/if}
          <div class="word-navigation">
            {#if snapshot.chapterCount > 1}
              <div class="practice-context">
                <SettingsSelect variant="pill" value={String(snapshot.chapterIndex - 1)} options={chapterOptions} ariaLabel={t.chapter} onSelect={chooseChapter} onClose={dismissChapterPicker} />
                <span class="chapter-position">{snapshot.chapterItemIndex}/{snapshot.chapterItemCount}</span>
              </div>
            {:else}
              <div class="practice-context">
                <span class="chapter-position">{snapshot.itemIndex}/{snapshot.itemCount}</span>
              </div>
            {/if}
          </div>
        </div>


        <label class="typing-surface" class:word-mode={snapshot.mode === 'word'} class:focused class:complete={stats.done} for="typing-capture" ondrop={(event) => event.preventDefault()} ondragover={(event) => event.preventDefault()}>
          <div class="practice-body" class:word-stage={snapshot.mode === 'word'} class:with-gloss={snapshot.mode === 'sentence' && snapshot.translation.length > 0} class:awaiting-start={awaitingStart} class:paused={practicePhase === 'paused' && !chapterPaused}>
            {#if chapterPaused}
              <div class="chapter-result" role="status">
                <p class="chapter-result-title">{chapterResultTitle}</p>
                <p class="chapter-result-count">{chapterTotals.words} {chapterResultUnit}</p>
              </div>
              <div class="chapter-actions">
                <button class="primary" disabled={snapshot.isLastItem} onclick={() => void continueAfterChapter()}>{snapshot.isLastItem ? t.bookComplete : t.nextChapter}</button>
                <button class="chapter-retry" onclick={() => void retryChapter()}>{t.retryChapter}</button>
              </div>
            {:else}
            <div class="sentence-stack">
              <div class="word-row">
                <div class="word-slot">
                  <div bind:this={textDisplay} id="passage-text" class="target" class:word-target={snapshot.mode === 'word'} aria-label={t.target} style:font-size={snapshot.mode === 'word' ? wordFontSize + 'px' : undefined}>
                {#each expected as char, index}
                  {#if index === typed.length && compositionText}<span class="composition-preview">{compositionText}</span>{/if}<span class:correct={index < typed.length && typed[index] === char} class:incorrect={index < typed.length && typed[index] !== char} class:caret={index === typed.length && !stats.done}>{char}</span>
                {/each}
                  </div>
                </div>
              </div>
              {#if snapshot.mode === 'sentence' && snapshot.translation}
                <p class="sentence-gloss">{snapshot.translation}</p>
              {/if}
            </div>
            {#if awaitingStart}
              <p class="start-hint">{t.pressAnyKey}</p>
            {:else if practicePhase === 'paused' && !chapterPaused}
              <p class="start-hint">{t.pressAnyKeyContinue}</p>
            {/if}
            {#if snapshot.mode === 'word' && snapshot.word}
              <div class="phonetic-row">
                <p class="word-phonetic">{selectedPhonetic ? selectedPhoneticLabel + ' ' + selectedPhonetic : t.missing}</p>
                <button type="button" class="speaker speaker-inline" aria-label={t.replayPronunciation} title={t.replayPronunciation} onclick={replayWord}>
                  <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 9.5h3.2L12 5v14l-4.8-4.5H4v-5Z" /><path d="M16 9.2a3.8 3.8 0 0 1 0 5.6M18.4 6.8a7 7 0 0 1 0 10.4" /></svg>
                </button>
              </div>
              <p class="word-translation">{snapshot.word.trans.length ? snapshot.word.trans.join('；') : t.missingTranslation}</p>
            {/if}
            {/if}
          </div>
          <textarea bind:this={input} id="typing-capture" class="typing-capture" aria-label={t.input} aria-describedby="passage-text" style:left={`${captureX}px`} style:top={`${captureY}px`} value={draft} spellcheck="false" autocapitalize="off" autocorrect="off" readonly={stats.done}
          onfocus={handleFocus} onblur={() => focused = false}
          onpaste={(event) => event.preventDefault()}
          oncompositionstart={() => { composing = true; compositionText = ''; }} oncompositionupdate={(event) => compositionText = event.data}
          oncompositionend={handleCompositionEnd} oninput={handleInput} onkeydown={handleKeydown}></textarea>
        </label>
      </section>
      <progress class="practice-progress" aria-label={t.progress} value={snapshot.mode === 'word' || snapshot.chapterCount > 1 ? snapshot.chapterProgress : stats.percent} max="100"></progress>
      <section class="summary" class:chapter-paused={chapterPaused}>
        <div class="stats">
          <div><strong>{displayWpm}</strong><span>{t.speed}</span></div>
          <div><strong>{displayAccuracy}<small>%</small></strong><span>{t.accuracy}</span></div>
          <div><strong>{formatDuration(displayElapsed)}</strong><span>{t.time}</span></div>
        </div>
      </section>
    </main>
