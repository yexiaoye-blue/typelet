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

export const freshPractice = (): Practice => ({ text: '', attempts: 0, correctAttempts: 0 });
