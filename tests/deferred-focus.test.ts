import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createDeferredFocus } from '../src/deferred-focus';

test('click and release handlers finish before practice focus returns', context => {
  context.mock.timers.enable({ apis: ['setTimeout'] });
  const events: string[] = [];
  const focus = createDeferredFocus(() => events.push('focus'));
  focus.request();
  events.push('button activated');
  assert.deepEqual(events, ['button activated']);
  context.mock.timers.tick(1);
  assert.deepEqual(events, ['button activated', 'focus']);
});

test('release, click and start requests restore focus only once', context => {
  context.mock.timers.enable({ apis: ['setTimeout'] });
  let restores = 0;
  const focus = createDeferredFocus(() => restores++);
  focus.request();
  focus.request();
  focus.request();
  context.mock.timers.tick(1);
  assert.equal(restores, 1);
});

test('leaving the window cancels a pending focus request', context => {
  context.mock.timers.enable({ apis: ['setTimeout'] });
  let restores = 0;
  const focus = createDeferredFocus(() => restores++);
  focus.request();
  focus.cancel();
  context.mock.timers.tick(1);
  assert.equal(restores, 0);
  focus.request();
  context.mock.timers.tick(1);
  assert.equal(restores, 1);
});

test('the main desktop window is permitted to recover native focus', () => {
  const capability = JSON.parse(readFileSync(new URL('../src-tauri/capabilities/default.json', import.meta.url), 'utf8'));
  assert.ok(capability.windows.includes('main'));
  assert.ok(capability.permissions.includes('core:window:allow-set-focus'));
});
