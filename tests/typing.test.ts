import test from 'node:test';
import assert from 'node:assert/strict';
import { freshPractice, previewAdvance, previewSnapshot, previewUpdate, summarizeChapter, updatePractice, statistics } from '../packages/core/typing';
test('backspace correction retains earlier mistakes in accuracy', () => {
 let practice = updatePractice(freshPractice(), 'ax', 'abc');
 assert.equal(statistics(practice, 'abc', 1000).accuracy, 50);
 practice = updatePractice(practice, 'a', 'abc');
 practice = updatePractice(practice, 'abc', 'abc');
 const result = statistics(practice, 'abc', 60000);
 assert.equal(result.done, true); assert.equal(result.accuracy, 75); assert.equal(result.cpm, 3);
});
test('middle replacement counts the new character without recounting suffix', () => {
 const wrong = updatePractice(freshPractice(), 'axc', 'abc');
 const fixed = updatePractice(wrong, 'abc', 'abc');
 assert.equal(fixed.attempts, 4); assert.equal(fixed.correctAttempts, 3);
 assert.equal(statistics(fixed, 'abc', 1000).percent, 100);
});
test('committed Chinese and non-BMP characters count as characters', () => {
 const target = '中文𠮷', practice = updatePractice(freshPractice(), target, target);
 assert.equal(practice.attempts, 3); assert.equal(statistics(practice, target, 60000).cpm, 3);
});
test('wrong or extra characters cannot complete a passage', () => {
 assert.equal(statistics(updatePractice(freshPractice(), 'abcx', 'abc'), 'abc', 1000).done, false);
 assert.equal(statistics(updatePractice(freshPractice(), 'abx', 'abc'), 'abc', 1000).done, false);
 assert.equal(statistics(freshPractice(), 'abc', 0).cpm, 0); assert.equal(statistics(freshPractice(), '', 0).done, false);
});
test('web preview uses the same snapshot shape and waits for Enter to advance', () => {
 const initial = previewSnapshot();
 const complete = previewUpdate(initial, initial.target, 60000);
 assert.equal(complete.stats.done, true);
 assert.equal(complete.itemIndex, 1);
 assert.equal(previewAdvance(complete).itemIndex, 2);
});
test('web preview keeps word metadata in the Rust-compatible snapshot shape', () => {
 const initial = previewSnapshot('word', 'preview:cet4');
 assert.equal(initial.word?.name, initial.target);
 assert.equal(initial.itemCount, 2607);
 assert.equal(initial.chapterCount, 131);
 assert.equal(initial.chapterItemCount, 20);
 assert.equal(initial.word?.name, 'cancel');
 assert.equal(initial.word?.usphone, "'kænsl");
 assert.deepEqual(initial.word?.trans, ['取消， 撤销； 删去']);
  const chapterEnd = previewUpdate(previewSnapshot('word', 'preview:cet4', 19), previewSnapshot('word', 'preview:cet4', 19).target, 1000);
  assert.equal(chapterEnd.isChapterLastItem, true);
  assert.equal(chapterEnd.chapterItemIndex, 20);
  assert.equal(previewAdvance(chapterEnd).chapterIndex, 2);
});
test('chapter summary adds the words practiced in that chapter', () => {
  const summary = summarizeChapter([
    { attempts: 5, correctAttempts: 4, correct: 4, milliseconds: 2000 },
    { attempts: 5, correctAttempts: 5, correct: 6, milliseconds: 1000 },
  ]);
  assert.equal(summary.words, 2);
  assert.equal(summary.accuracy, 90);
  assert.equal(summary.cpm, 200);
  assert.equal(summary.milliseconds, 3000);
  assert.deepEqual(summarizeChapter([]), { words: 0, accuracy: 100, cpm: 0, milliseconds: 0 });
});
