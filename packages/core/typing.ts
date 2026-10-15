import cet4Data from '../../src-tauri/typing-core/data/cet4_3.json';

export type Practice = { text: string; attempts: number; correctAttempts: number };
export type PracticeMode = 'sentence' | 'word';
export type WordEntry = { name: string; trans: string[]; usphone: string; ukphone: string };
export type CollectionSummary = { id: string; title: string; count: number; selected: boolean; source: string; chapterCount: number };
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

const previewArticles = [
  { id: 'preview:meditations', title: 'Meditations', source: 'Web preview', sentences: ['You have power over your mind—not outside events.', 'Realize this, and you will find strength.'] },
  { id: 'preview:alice', title: "Alice's Adventures in Wonderland", source: 'Web preview', sentences: ['Alice was beginning to get very tired.', 'She had nothing to do.'] },
];
const previewWords: WordEntry[] = cet4Data as WordEntry[];

export function previewCatalog(mode: PracticeMode = 'sentence', articleId = previewArticles[0].id, wordBookId = 'preview:cet4'): ContentCatalogSnapshot {
  return {
    mode,
    articles: previewArticles.map(article => ({ id: article.id, title: article.title, count: article.sentences.length, selected: article.id === articleId, source: article.source, chapterCount: 1 })),
    wordBooks: [{ id: wordBookId, title: 'CET4_3 · preview', count: previewWords.length, selected: true, source: 'Web preview data', chapterCount: Math.ceil(previewWords.length / 20) }],
  };
}

export function previewSnapshot(mode: PracticeMode = 'sentence', collectionId = previewArticles[0].id, itemIndex = 0, practice = freshPractice(), milliseconds = 0): PracticeSnapshot {
  const collection = mode === 'sentence' ? previewArticles.find(article => article.id === collectionId) ?? previewArticles[0] : { id: 'preview:cet4', title: 'CET4_3 · preview', source: 'Web preview data', sentences: previewWords.map(word => word.name) };
  const index = itemIndex % collection.sentences.length;
  const word = mode === 'word' ? previewWords[index] : null;
  const target = word?.name ?? collection.sentences[index];
  const stats = statistics(practice, target, milliseconds);
  const chapterCount = mode === 'word' ? Math.max(1, Math.ceil(collection.sentences.length / 20)) : 1;
  const chapterIndex = mode === 'word' ? Math.floor(index / 20) + 1 : 1;
  const chapterStart = mode === 'word' ? (chapterIndex - 1) * 20 : 0;
  const chapterItemCount = mode === 'word' ? Math.min(20, collection.sentences.length - chapterStart) : collection.sentences.length;
  const chapterItemIndex = mode === 'word' ? index - chapterStart + 1 : index + 1;
  const completed = mode === 'word' ? chapterItemIndex - 1 + (stats.done ? 1 : 0) : stats.percent;
  const chapterProgress = mode === 'word' ? Math.min(100, Math.floor(completed / Math.max(1, chapterItemCount) * 100)) : stats.percent;
  const isChapterLastItem = mode === 'word' ? chapterItemIndex === chapterItemCount : index === collection.sentences.length - 1;
  const isLastItem = index === collection.sentences.length - 1;
  return { mode, collectionId: collection.id, collectionTitle: collection.title, itemIndex: index + 1, itemCount: collection.sentences.length, chapterIndex, chapterCount, chapterItemIndex, chapterItemCount, chapterProgress, isChapterLastItem, isLastItem, target, word, practice, stats };
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
  if (snapshot.mode !== 'word') return snapshot;
  const index = Math.min(chapter * 20, snapshot.itemCount - 1);
  return previewSnapshot(snapshot.mode, snapshot.collectionId, index);
}

export function previewSelectMode(snapshot: PracticeSnapshot, mode: PracticeMode): PracticeSnapshot {
  const id = mode === 'sentence' ? previewArticles[0].id : 'preview:cet4';
  return previewSnapshot(mode, id);
}

export function previewSelectCollection(snapshot: PracticeSnapshot, collectionId: string): PracticeSnapshot {
  return previewSnapshot(snapshot.mode, collectionId);
}
