export type PracticeOrder = 'book' | 'random';
export const defaultChapterLength = 20;
export const minimumChapterLength = 10;

let requestedChapterLength = defaultChapterLength;
let practiceOrder: PracticeOrder = 'book';

export function setPreviewLayout(chapterLength: number, order: PracticeOrder) {
  requestedChapterLength = normalizeRequestedChapterLength(chapterLength);
  practiceOrder = order === 'random' ? 'random' : 'book';
}

export function normalizeRequestedChapterLength(value: number) {
  if (!Number.isInteger(value) || value <= 0) return defaultChapterLength;
  return Math.max(minimumChapterLength, value);
}

export function effectiveChapterLength(count: number, requested = requestedChapterLength) {
  const normalized = normalizeRequestedChapterLength(requested);
  if (count <= 0) return normalized;
  const minimum = Math.min(minimumChapterLength, count);
  return Math.min(count, Math.max(minimum, normalized));
}

export function numberedChapterCount(count: number, requested = requestedChapterLength) {
  if (count <= 0) return 1;
  return Math.max(1, Math.ceil(count / effectiveChapterLength(count, requested)));
}

function hash32(id: string) {
  const bytes = new TextEncoder().encode(id);
  let hash = 2166136261;
  for (const byte of bytes) {
    hash ^= byte;
    hash = Math.imul(hash, 16777619);
  }
  return hash >>> 0;
}

function mix32(value: number) {
  let state = value >>> 0;
  state = (state ^ (state << 13)) >>> 0;
  state = (state ^ (state >>> 17)) >>> 0;
  state = (state ^ (state << 5)) >>> 0;
  return state >>> 0;
}

export function chapterOffsets(collectionId: string, chapterStart: number, chapterLength: number) {
  const offsets = Array.from({ length: chapterLength }, (_, index) => index);
  if (chapterLength < 2) return offsets;
  let state = (hash32(collectionId) ^ Math.imul(chapterStart, 0x9E3779B1) ^ Math.imul(chapterLength, 0x85EBCA77)) >>> 0;
  if (state === 0) state = 0xA5A5A5A5;
  for (let index = chapterLength - 1; index > 0; index -= 1) {
    state = mix32(state);
    const swap = state % (index + 1);
    const held = offsets[index];
    offsets[index] = offsets[swap];
    offsets[swap] = held;
  }
  return offsets;
}

export function presentedIndex(collectionId: string, start: number, length: number, cursor: number) {
  const slot = Math.max(0, cursor - start);
  if (practiceOrder !== 'random' || length < 2) return start + Math.min(slot, Math.max(0, length - 1));
  return start + (chapterOffsets(collectionId, start, length)[slot] ?? 0);
}

export function numberedPlacement(count: number, cursor: number) {
  const length = effectiveChapterLength(count);
  const chapterStart = Math.floor(cursor / length) * length;
  return {
    chapterIndex: Math.floor(cursor / length) + 1,
    chapterCount: numberedChapterCount(count),
    chapterStart,
    chapterItemIndex: cursor - chapterStart + 1,
    chapterItemCount: Math.max(1, Math.min(length, count - chapterStart)),
    chapterTitle: '',
    chapterTitles: [] as string[],
  };
}

export function sentencePlacement(article: { sentences: string[]; chapterTitles: string[]; chapterEnds: number[] }, index: number) {
  let start = 0;
  for (let chapter = 0; chapter < article.chapterEnds.length; chapter += 1) {
    const end = article.chapterEnds[chapter];
    if (index < end || chapter === article.chapterEnds.length - 1) {
      return {
        chapterIndex: chapter + 1,
        chapterCount: article.chapterEnds.length,
        chapterStart: start,
        chapterItemIndex: index - start + 1,
        chapterItemCount: Math.max(1, end - start),
        chapterTitle: article.chapterTitles[chapter] ?? '',
        chapterTitles: article.chapterTitles,
      };
    }
    start = end;
  }
  return numberedPlacement(article.sentences.length, index);
}

