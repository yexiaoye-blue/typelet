import { characters } from './text';
import type { Practice, Statistics } from './model';

export type ChapterWordResult = { attempts: number; correctAttempts: number; correct: number; milliseconds: number };
export type ChapterSummary = { words: number; accuracy: number; cpm: number; milliseconds: number };


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

