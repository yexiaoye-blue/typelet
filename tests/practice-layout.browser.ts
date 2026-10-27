// Run against the real practice page in the Vite preview. These checks measure
// rendered boxes, so a competing CSS rule cannot silently reintroduce overlap.
// Use a translated long sentence, e.g. movie-lines item 19, in small-window mode.
export function checkSentenceLayout() {
  function element(selector: string) {
    const match = document.querySelector<HTMLElement>(selector);
    if (!match) throw new Error('Missing practice element: ' + selector);
    return match;
  }

  const surface = element('.typing-surface').getBoundingClientRect();
  const scroller = element('.sentence-stack');
  const viewport = scroller.getBoundingClientRect();
  const row = element('.word-row').getBoundingClientRect();
  const target = element('.target').getBoundingClientRect();
  const translation = element('.sentence-gloss').getBoundingClientRect();
  const gap = translation.top - target.bottom;
  if (row.bottom < target.bottom - 1) {
    throw new Error('The original text extends beyond its layout row.');
  }
  if (gap < 8) {
    throw new Error('The translation overlaps or touches the original text.');
  }
  const contentHeight = translation.bottom - row.top;
  if (contentHeight > surface.height && viewport.height < surface.height - 1) {
    throw new Error('Long content does not use the available practice height.');
  }
  const lastVisibleBottom = translation.bottom - (scroller.scrollHeight - scroller.clientHeight - scroller.scrollTop);
  if (lastVisibleBottom > viewport.bottom + 1) {
    throw new Error('The translation cannot be fully reached by scrolling.');
  }
  return {
    rowHeight: row.height,
    textHeight: target.height,
    translationGap: gap,
    viewportHeight: viewport.height,
    availableHeight: surface.height,
    scrollHeight: scroller.scrollHeight,
  };
}

// In word mode, measure the whole group (word, phonetics and meaning). For a
// long sentence, the visible scrolling viewport should remain centered.
export function checkPracticeCentering() {
  const widget = document.querySelector<HTMLElement>('.widget.mini-window');
  if (!widget) throw new Error('Load the small practice window before checking centering.');
  const bounds = widget.getBoundingClientRect();
  const wordMode = widget.querySelector('.word-stage');
  const nodes = widget.querySelectorAll<HTMLElement>(wordMode
    ? '.word-row, .phonetic-row, .word-translation'
    : '.sentence-stack');
  if (!nodes.length) throw new Error('No practice content is visible.');
  const rects = Array.from(nodes, node => node.getBoundingClientRect());
  const left = Math.min(...rects.map(rect => rect.left));
  const right = Math.max(...rects.map(rect => rect.right));
  const top = Math.min(...rects.map(rect => rect.top));
  const bottom = Math.max(...rects.map(rect => rect.bottom));
  const offsetX = (left + right - bounds.left - bounds.right) / 2;
  const offsetY = (top + bottom - bounds.top - bounds.bottom) / 2;
  if (wordMode) {
    const target = widget.querySelector<HTMLElement>('.word-target')?.getBoundingClientRect();
    if (!target || Math.abs((target.left + target.right - bounds.left - bounds.right) / 2) > 1) {
      throw new Error('The word itself is not horizontally centered.');
    }
  }
  if (Math.abs(offsetX) > 1 || Math.abs(offsetY) > 1) {
    throw new Error('Practice content is off center: x=' + offsetX + ', y=' + offsetY);
  }
  return { mode: wordMode ? 'word' : 'sentence', offsetX, offsetY, contentHeight: bottom - top };
}
