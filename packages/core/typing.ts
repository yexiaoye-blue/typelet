import cet4Data from '../../src-tauri/typing-core/data/cet4_3.json';
import movieLines from '../../src-tauri/typing-core/data/movie_lines.json';

export type Practice = { text: string; attempts: number; correctAttempts: number };
export type PracticeMode = 'sentence' | 'word';
export type WordEntry = { name: string; trans: string[]; usphone: string; ukphone: string };
export type CollectionSummary = { id: string; title: string; count: number; selected: boolean; source: string; chapterCount: number; removable: boolean };
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

function articleChapterCount(article: { chapterEnds: number[] }) {
  return article.chapterEnds.length || 1;
}

function sentencePlacement(article: { sentences: string[]; chapterTitles: string[]; chapterEnds: number[] }, index: number) {
  if (!article.chapterEnds.length) {
    return { chapterIndex: 1, chapterCount: 1, chapterStart: 0, chapterItemIndex: index + 1, chapterItemCount: article.sentences.length, chapterTitle: '', chapterTitles: [] as string[] };
  }
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
  return { chapterIndex: 1, chapterCount: 1, chapterStart: 0, chapterItemIndex: index + 1, chapterItemCount: article.sentences.length, chapterTitle: '', chapterTitles: article.chapterTitles };
}

export function previewCatalog(mode: PracticeMode = 'sentence', articleId = previewArticles[0].id, wordBookId = 'preview:cet4'): ContentCatalogSnapshot {
  const selectedArticle = previewArticles.some((article) => article.id === articleId) ? articleId : previewArticles[0].id;
  return {
    mode,
    articles: previewArticles.map(article => ({ id: article.id, title: article.title, count: article.sentences.length, selected: article.id === selectedArticle, source: article.source, chapterCount: articleChapterCount(article), removable: false })),
    wordBooks: [{ id: wordBookId, title: 'CET4_3 · preview', count: previewWords.length, selected: true, source: 'Web preview data', chapterCount: Math.ceil(previewWords.length / 20), removable: false }],
  };
}

export function previewSnapshot(mode: PracticeMode = 'sentence', collectionId = previewArticles[0].id, itemIndex = 0, practice = freshPractice(), milliseconds = 0): PracticeSnapshot {
  const collection = mode === 'sentence' ? previewArticles.find(article => article.id === collectionId) ?? previewArticles[0] : { id: 'preview:cet4', title: 'CET4_3 · preview', source: 'Web preview data', sentences: previewWords.map(word => word.name) };
  const index = itemIndex % collection.sentences.length;
  const word = mode === 'word' ? previewWords[index] : null;
  const target = word?.name ?? collection.sentences[index];
  const stats = statistics(practice, target, milliseconds);
  const sentence = mode === 'sentence' && 'chapterEnds' in collection ? sentencePlacement(collection, index) : null;
  const chapterCount = mode === 'word' ? Math.max(1, Math.ceil(collection.sentences.length / 20)) : sentence?.chapterCount ?? 1;
  const chapterIndex = mode === 'word' ? Math.floor(index / 20) + 1 : sentence?.chapterIndex ?? 1;
  const chapterStart = mode === 'word' ? (chapterIndex - 1) * 20 : sentence?.chapterStart ?? 0;
  const chapterItemCount = mode === 'word' ? Math.min(20, collection.sentences.length - chapterStart) : sentence?.chapterItemCount ?? collection.sentences.length;
  const chapterItemIndex = mode === 'word' ? index - chapterStart + 1 : sentence?.chapterItemIndex ?? index + 1;
  const useChapterProgress = mode === 'word' || chapterCount > 1;
  const chapterProgress = useChapterProgress ? Math.min(100, Math.floor((chapterItemIndex - 1 + (stats.done ? 1 : 0)) / Math.max(1, chapterItemCount) * 100)) : stats.percent;
  const isChapterLastItem = chapterItemIndex === chapterItemCount;
  const isLastItem = index === collection.sentences.length - 1;
  const translation = mode === 'sentence' && 'translations' in collection ? collection.translations[index] ?? '' : '';
  return { mode, collectionId: collection.id, collectionTitle: collection.title, itemIndex: index + 1, itemCount: collection.sentences.length, chapterIndex, chapterCount, chapterItemIndex, chapterItemCount, chapterProgress, isChapterLastItem, isLastItem, target, word, chapterTitle: sentence?.chapterTitle ?? '', chapterTitles: sentence?.chapterTitles ?? [], translation, practice, stats };
}

export function previewUpdate(snapshot: PracticeSnapshot, value: string, milliseconds: number): PracticeSnapshot {
  const practice = updatePractice(snapshot.practice, value, snapshot.target);
  return previewSnapshot(snapshot.mode, snapshot.collectionId, snapshot.itemIndex - 1, practice, milliseconds);
}

export function previewReset(snapshot: PracticeSnapshot): PracticeSnapshot {
  return previewSnapshot(snapshot.mode, snapshot.collectionId, snapshot.itemIndex - 1);
}

export function previewAdvance(snapshot: PracticeSnapshot): PracticeSnapshot {
  if (!snapshot.stats.done) throw new Error('当前练习项尚未完成');
  if (snapshot.mode === 'word' && snapshot.isLastItem) throw new Error('本词书已完成');
  return previewSnapshot(snapshot.mode, snapshot.collectionId, snapshot.itemIndex % snapshot.itemCount);
}

export function previewSelectChapter(snapshot: PracticeSnapshot, chapter: number): PracticeSnapshot {
  if (snapshot.mode === 'sentence') {
    const article = previewArticles.find(item => item.id === snapshot.collectionId) ?? previewArticles[0];
    if (!article.chapterEnds.length) return chapter === 0 ? previewSnapshot(snapshot.mode, article.id, 0) : snapshot;
    if (chapter < 0 || chapter >= article.chapterEnds.length) return snapshot;
    const index = chapter === 0 ? 0 : article.chapterEnds[chapter - 1];
    return previewSnapshot(snapshot.mode, article.id, index);
  }
  const index = Math.min(Math.max(0, chapter) * 20, snapshot.itemCount - 1);
  return previewSnapshot(snapshot.mode, snapshot.collectionId, index);
}

export function previewSelectMode(snapshot: PracticeSnapshot, mode: PracticeMode): PracticeSnapshot {
  const id = mode === 'sentence' ? previewArticles[0].id : 'preview:cet4';
  return previewSnapshot(mode, id);
}

export function previewSelectCollection(snapshot: PracticeSnapshot, collectionId: string): PracticeSnapshot {
  return previewSnapshot(snapshot.mode, collectionId);
}
