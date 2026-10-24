import cet4Data from '../../src-tauri/typing-core/data/cet4_3.json';
import movieLines from '../../src-tauri/typing-core/data/movie_lines.json';

import { effectiveChapterLength, numberedChapterCount, numberedPlacement, presentedIndex, sentencePlacement } from './chapter';
import type { ContentCatalogSnapshot, Practice, PracticeMode, PracticeSnapshot, WordEntry } from './model';
import { freshPractice } from './model';
import { statistics, updatePractice } from './scoring';
import { characters } from './text';

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
