export type Practice = { text: string; attempts: number; correctAttempts: number };
export const freshPractice = (): Practice => ({ text: '', attempts: 0, correctAttempts: 0 });
export const characters = (text: string) => Array.from(text.normalize('NFC'));
export function updatePractice(previous: Practice, value: string, target: string): Practice {
 const before = characters(previous.text), after = characters(value), expected = characters(target);
 let start = 0, endBefore = before.length, endAfter = after.length;
 while (start < endBefore && start < endAfter && before[start] === after[start]) start++;
 while (endBefore > start && endAfter > start && before[endBefore - 1] === after[endAfter - 1]) { endBefore--; endAfter--; }
 let correct = 0;
 for (let i = start; i < endAfter; i++) if (after[i] === expected[i]) correct++;
 return { text: after.join(''), attempts: previous.attempts + endAfter - start, correctAttempts: previous.correctAttempts + correct };
}
export function statistics(practice: Practice, target: string, milliseconds: number) {
 const typed = characters(practice.text), expected = characters(target);
 const correct = typed.filter((char, index) => char === expected[index]).length;
 return {
  correct, percent: expected.length ? Math.floor(correct / expected.length * 100) : 0,
  accuracy: practice.attempts ? Math.round(practice.correctAttempts / practice.attempts * 100) : 100,
  cpm: milliseconds >= 1000 ? Math.round(correct * 60000 / milliseconds) : 0,
  done: expected.length > 0 && practice.text.normalize('NFC') === target.normalize('NFC'),
 };
}
export const passages = {
 en: [
  'Small steps make a difference. Take your time and find your own rhythm.',
  'The quick brown fox jumps over the lazy dog. A little practice goes a long way.',
  'Keep your hands moving and your mind at ease. One word, then another.',
 ],
 zh: [
  '窗外的风轻轻吹过，手指在键盘上慢慢走着。别着急，按照自己的节奏，打完这一小段文字。',
  '等一杯茶变温，等一个想法成形。在等待的片刻，敲下几行文字，找回一点熟悉的手感。',
  '把注意力放在眼前的一个字上。打错了就退格，停下来也没关系，下一次仍然可以继续。',
 ],
};
