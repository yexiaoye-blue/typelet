import test from 'node:test';
import assert from 'node:assert/strict';
import { keySoundMakeup } from '../src/key-sounds';

test('a quiet key recording is normalized to about -1 dBFS at full volume', () => {
  const peak = 10 ** (-21.2 / 20);
  const amplified = peak * keySoundMakeup(peak);
  assert.ok(Math.abs(20 * Math.log10(amplified) + 1) < 0.05);
});

test('an already loud recording is not made louder', () => {
  assert.ok(keySoundMakeup(0.9) < 1);
});

test('silence does not receive an extreme gain', () => {
  assert.equal(keySoundMakeup(0), 1);
  assert.ok(keySoundMakeup(10 ** (-60 / 20)) <= 32);
});
