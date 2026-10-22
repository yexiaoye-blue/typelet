import test from 'node:test';
import assert from 'node:assert/strict';
import { keySoundMakeup, keysAudible, readingAudible, soundPreferenceFrom } from '../src/key-sounds';

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

test('an older key-sound switch stays muted without changing its volume', () => {
  const sound = soundPreferenceFrom({ keyEnabled: false, keyVolume: 80, speechVolume: 40 });
  assert.equal(sound.enabled, true);
  assert.equal(sound.keyMuted, true);
  assert.equal(sound.keyVolume, 80);
  assert.equal(sound.speechMuted, false);
  assert.equal(keysAudible(sound), false);
});

test('mute and the master switch stop playback without requesting speech', () => {
  const audible = soundPreferenceFrom({ speechVolume: 100, keyVolume: 100 });
  assert.equal(readingAudible(audible), true);
  assert.equal(keysAudible(audible), true);
  assert.equal(readingAudible({ ...audible, speechMuted: true }), false);
  assert.equal(readingAudible({ ...audible, enabled: false }), false);
  assert.equal(keysAudible({ ...audible, keyMuted: true }), false);
  assert.equal(keysAudible({ ...audible, keyVolume: 0 }), false);
});
