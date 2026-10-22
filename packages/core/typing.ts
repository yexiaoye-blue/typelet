import cet4Data from '../../src-tauri/typing-core/data/cet4_3.json';
import movieLines from '../../src-tauri/typing-core/data/movie_lines.json';

export type Practice = { text: string; attempts: number; correctAttempts: number };
export type PracticeMode = 'sentence' | 'word';
export type WordEntry = { name: string; trans: string[]; usphone: string; ukphone: string };
export type CollectionSummary = { id: string; title: string; count: number; selected: boolean; source: string; chapterCount: number; progress: number; removable: boolean };
export type ContentCatalogSnapshot = { mode: PracticeMode; articles: CollectionSummary[]; wordBooks: CollectionSummary[] };
export type PracticeSnapshot = {
  mode: PracticeMode;
  collectionId: string;
  collectionTitle: string;
  itemIndex: number;
  itemCount: number;
  chapterIndex: number;
  chapterCount: number;
  chapterItemIndex: number;
  chapterItemCount: number;
  chapterProgress: number;
  isChapterLastItem: boolean;
  isLastItem: boolean;
  target: string;
  word: WordEntry | null;
  chapterTitle: string;
  chapterTitles: string[];
  translation: string;
  practice: Practice;
  stats: Statistics;
};
export type Statistics = { correct: number; percent: number; accuracy: number; cpm: number; done: boolean };
export type ChapterWordResult = { attempts: number; correctAttempts: number; correct: number; milliseconds: number };
export type ChapterSummary = { words: number; accuracy: number; cpm: number; milliseconds: number };

export const freshPractice = (): Practice => ({ text: '', attempts: 0, correctAttempts: 0 });
export const characters = (text: string) => Array.from(text.normalize('NFC'));
export const limitPracticeInput = (value: string, target: string) => characters(value).slice(0, characters(target).length).join('');

export function updatePractice(previous: Practice, value: string, target: string): Practice {
  const before = characters(previous.text), after = characters(value), expected = characters(target);
  let start = 0, endBefore = before.length, endAfter = after.length;
  while (start < endBefore && start < endAfter && before[start] === after[start]) start++;
  while (endBefore > start && endAfter > start && before[endBefore - 1] === after[endAfter - 1]) { endBefore--; endAfter--; }
  let correct = 0;
  for (let index = start; index < endAfter; index++) if (after[index] === expected[index]) correct++;
  return { text: after.join(''), attempts: previous.attempts + endAfter - start, correctAttempts: previous.correctAttempts + correct };
}

export function statistics(practice: Practice, target: string, milliseconds: number): Statistics {
  const typed = characters(practice.text), expected = characters(target);
  const correct = typed.filter((char, index) => char === expected[index]).length;
  return {
    correct,
    percent: expected.length ? Math.floor(correct / expected.length * 100) : 0,
    accuracy: practice.attempts ? Math.round(practice.correctAttempts / practice.attempts * 100) : 100,
    cpm: milliseconds >= 1000 ? Math.round(correct * 60000 / milliseconds) : 0,
    done: expected.length > 0 && practice.text.normalize('NFC') === target.normalize('NFC'),
  };
}

export function summarizeChapter(results: ChapterWordResult[]): ChapterSummary {
  let attempts = 0;
  let correctAttempts = 0;
  let correct = 0;
  let milliseconds = 0;
  for (const result of results) {
    attempts += result.attempts;
    correctAttempts += result.correctAttempts;
    correct += result.correct;
    milliseconds += result.milliseconds;
  }
  return {
    words: results.length,
    accuracy: attempts ? Math.round(correctAttempts / attempts * 100) : 100,
    cpm: milliseconds >= 1000 ? Math.round(correct * 60000 / milliseconds) : 0,
    milliseconds,
  };
}

type MovieLineBook = { title: string; source: string; chapters: { title: string; lines: { en: string; zh: string }[] }[] };

function normalizePracticeText(text: string) {
  return text
    .replaceAll('\u2018', "'")
    .replaceAll('\u2019', "'")
    .replaceAll('\u2032', "'")
    .replaceAll('\u201c', '"')
    .replaceAll('\u201d', '"')
    .replaceAll('\u2013', '-')
    .replaceAll('\u2014', '-')
    .replaceAll('\u2026', '...')
    .replaceAll('\u00a0', ' ');
}

function movieArticle(book: MovieLineBook) {
  const sentences: string[] = [];
  const translations: string[] = [];
  const chapterTitles: string[] = [];
  const chapterEnds: number[] = [];
  for (const chapter of book.chapters) {
    const lines = chapter.lines.filter((line) => line.en.trim());
    if (!lines.length) continue;
    chapterTitles.push(chapter.title);
    for (const line of lines) {
      sentences.push(normalizePracticeText(line.en.trim()));
      translations.push(line.zh.trim());
    }
    chapterEnds.push(sentences.length);
  }
  return {
    id: 'builtin:movie-lines',
    title: book.title.trim() || '电影台词',
    source: book.source,
    sentences,
    translations,
    chapterTitles,
    chapterEnds,
  };
}

const previewArticles = [movieArticle(movieLines as MovieLineBook)];
const previewWords: WordEntry[] = cet4Data as WordEntry[];

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

function numberedChapterCount(count: number, requested = requestedChapterLength) {
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

function presentedIndex(collectionId: string, start: number, length: number, cursor: number) {
  const slot = Math.max(0, cursor - start);
  if (practiceOrder !== 'random' || length < 2) return start + Math.min(slot, Math.max(0, length - 1));
  return start + (chapterOffsets(collectionId, start, length)[slot] ?? 0);
}

function numberedPlacement(count: number, cursor: number) {
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

function sentencePlacement(article: { sentences: string[]; chapterTitles: string[]; chapterEnds: number[] }, index: number) {
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

export function previewCatalog(mode: PracticeMode = 'sentence', articleId = previewArticles[0].id, wordBookId = 'preview:cet4'): ContentCatalogSnapshot {
  const selectedArticle = previewArticles.some((article) => article.id === articleId) ? articleId : previewArticles[0].id;
  return {
    mode,
    articles: previewArticles.map(article => ({
      id: article.id,
      title: article.title,
      count: article.sentences.length,
      selected: article.id === selectedArticle,
      source: article.source,
      chapterCount: article.chapterEnds.length || numberedChapterCount(article.sentences.length),
      progress: 0,
      removable: false,
    })),
    wordBooks: [{
      id: wordBookId,
      title: 'CET4_3 · preview',
      count: previewWords.length,
      selected: true,
      source: 'Web preview data',
      chapterCount: numberedChapterCount(previewWords.length),
      progress: 0,
      removable: false,
    }],
  };
}

export function previewSnapshot(mode: PracticeMode = 'sentence', collectionId = previewArticles[0].id, itemIndex = 0, practice = freshPractice(), milliseconds = 0): PracticeSnapshot {
  const article = mode === 'sentence' ? previewArticles.find(item => item.id === collectionId) ?? previewArticles[0] : null;
  const sentences = article ? article.sentences : previewWords.map(word => word.name);
  const collection = article ?? {
    id: 'preview:cet4',
    title: 'CET4_3 · preview',
    source: 'Web preview data',
    sentences,
    translations: [] as string[],
    chapterTitles: [] as string[],
    chapterEnds: [] as number[],
  };
  const cursor = ((itemIndex % sentences.length) + sentences.length) % sentences.length;
  const placement = article && article.chapterEnds.length ? sentencePlacement(article, cursor) : numberedPlacement(sentences.length, cursor);
  const contentIndex = presentedIndex(collection.id, placement.chapterStart, placement.chapterItemCount, cursor);
  const word = mode === 'word' ? previewWords[contentIndex] ?? null : null;
  const target = word?.name ?? sentences[contentIndex] ?? '';
  const stats = statistics(practice, target, milliseconds);
  const useChapterProgress = mode === 'word' || placement.chapterCount > 1;
  const chapterProgress = useChapterProgress
    ? Math.min(100, Math.floor((placement.chapterItemIndex - 1 + (stats.done ? 1 : 0)) / Math.max(1, placement.chapterItemCount) * 100))
    : stats.percent;
  const translation = article ? article.translations[contentIndex] ?? '' : '';
  return {
    mode,
    collectionId: collection.id,
    collectionTitle: collection.title,
    itemIndex: cursor + 1,
    itemCount: sentences.length,
    chapterIndex: placement.chapterIndex,
    chapterCount: placement.chapterCount,
    chapterItemIndex: placement.chapterItemIndex,
    chapterItemCount: placement.chapterItemCount,
    chapterProgress,
    isChapterLastItem: placement.chapterItemIndex === placement.chapterItemCount,
    isLastItem: cursor === sentences.length - 1,
    target,
    word,
    chapterTitle: placement.chapterTitle,
    chapterTitles: placement.chapterTitles,
    translation,
    practice,
    stats,
  };
}

export function previewUpdate(snapshot: PracticeSnapshot, value: string, milliseconds: number): PracticeSnapshot {
  const practice = updatePractice(snapshot.practice, value, snapshot.target);
  return previewSnapshot(snapshot.mode, snapshot.collectionId, snapshot.itemIndex - 1, practice, milliseconds);
}

export function previewReset(snapshot: PracticeSnapshot): PracticeSnapshot {
  return previewSnapshot(snapshot.mode, snapshot.collectionId, snapshot.itemIndex - 1);
}

export function previewAdvance(snapshot: PracticeSnapshot): PracticeSnapshot {
  const reachedEnd = snapshot.target.length > 0 && characters(snapshot.practice.text).length >= characters(snapshot.target).length;
  if (!snapshot.stats.done && !reachedEnd) throw new Error('当前练习项尚未完成');
  if (snapshot.mode === 'word' && snapshot.isLastItem) throw new Error('本词书已完成');
  return previewSnapshot(snapshot.mode, snapshot.collectionId, snapshot.itemIndex % snapshot.itemCount);
}

export function previewSelectItem(snapshot: PracticeSnapshot, itemIndex: number): PracticeSnapshot {
  if (!Number.isInteger(itemIndex) || itemIndex < 0 || itemIndex >= snapshot.itemCount) return snapshot;
  return previewSnapshot(snapshot.mode, snapshot.collectionId, itemIndex);
}

export function previewSelectChapter(snapshot: PracticeSnapshot, chapter: number): PracticeSnapshot {
  if (snapshot.mode === 'sentence') {
    const article = previewArticles.find(item => item.id === snapshot.collectionId) ?? previewArticles[0];
    if (article.chapterEnds.length) {
      if (chapter < 0 || chapter >= article.chapterEnds.length) return snapshot;
      const index = chapter === 0 ? 0 : article.chapterEnds[chapter - 1];
      return previewSnapshot(snapshot.mode, article.id, index);
    }
  }
  const length = effectiveChapterLength(snapshot.itemCount);
  const chapters = numberedChapterCount(snapshot.itemCount);
  if (!Number.isInteger(chapter) || chapter < 0 || chapter >= chapters) return snapshot;
  return previewSnapshot(snapshot.mode, snapshot.collectionId, Math.min(chapter * length, snapshot.itemCount - 1));
}

export function previewSelectMode(snapshot: PracticeSnapshot, mode: PracticeMode): PracticeSnapshot {
  const id = mode === 'sentence' ? previewArticles[0].id : 'preview:cet4';
  return previewSnapshot(mode, id);
}

export function previewSelectCollection(snapshot: PracticeSnapshot, collectionId: string): PracticeSnapshot {
  return previewSnapshot(snapshot.mode, collectionId);
}
