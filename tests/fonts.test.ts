import test from 'node:test';
import assert from 'node:assert/strict';
import { fontChoices } from '../src/fonts';

test('font search keeps the built-in and system choices in the same list', () => {
  const choices = fontChoices({
    families: ['Noto Serif', 'Source Code Pro'],
    query: '',
    draft: 'builtin:noto-mono',
    builtinLabel: 'Noto Sans Mono · Built-in',
    systemLabel: 'System default font',
  });
  assert.deepEqual(choices.map(choice => choice.value), ['builtin:noto-mono', 'system', 'font:Noto Serif', 'font:Source Code Pro']);
});

test('a font query filters the shared list', () => {
  const choices = fontChoices({
    families: ['Noto Serif', 'Source Code Pro'],
    query: 'source',
    draft: 'font:Missing',
    builtinLabel: 'Noto Sans Mono · Built-in',
    systemLabel: 'System default font',
  });
  assert.deepEqual(choices.map(choice => choice.label), ['Source Code Pro']);
});
