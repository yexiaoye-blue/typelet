import test from 'node:test';
import assert from 'node:assert/strict';
import { freshPractice, updatePractice, statistics } from '../packages/core/typing';
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
