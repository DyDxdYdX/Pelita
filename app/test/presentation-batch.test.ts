import assert from 'node:assert/strict';
import { test } from 'node:test';
import { processPresentationBatch } from '../src/presentation-batch.ts';

const picks = ['Sermon', 'Unavailable', 'Hymn'].map(id => ({ id, name: `${id}.pptx` }));

test('a failed PowerPoint file leaves later files importable and reports exactly what to retry', async () => {
  const attempts: string[] = [];
  const result = await processPresentationBatch(picks, new AbortController().signal, async ({ id }) => {
    attempts.push(id);
    if (id === 'Unavailable') throw new Error('Converter unavailable');
    return { reused: id === 'Hymn', fallback: id === 'Sermon' };
  });
  assert.deepEqual(attempts, ['Sermon', 'Unavailable', 'Hymn']);
  assert.deepEqual(result, {
    added: 2, reused: 1, fallbacks: 1,
    failures: [{ source: picks[1], message: 'Converter unavailable' }],
  });
});

test('cancelling a PowerPoint batch keeps completed imports and stops later work', async () => {
  const controller = new AbortController();
  const attempts: string[] = [];
  const result = await processPresentationBatch(picks, controller.signal, async ({ id }) => {
    attempts.push(id);
    if (id === 'Unavailable') {
      controller.abort();
      controller.signal.throwIfAborted();
    }
    return { reused: false, fallback: false };
  });
  assert.deepEqual(attempts, ['Sermon', 'Unavailable']);
  assert.deepEqual(result, { added: 1, reused: 0, fallbacks: 0, failures: [] });
});
