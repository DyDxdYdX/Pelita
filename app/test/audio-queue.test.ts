import assert from 'node:assert/strict';
import { test } from 'node:test';
import { adjacentTrack, audioTracks, moveTrack, type Track } from '../src/audio-queue.ts';
import type { Item } from '../src/model.ts';

const track = (id: string): Track => ({
  id, title: id, content: { kind: 'audio', audio: { id, group: 'fixture', kind: 'mp3', name: `${id}.mp3`, size: 512 } },
});

test('reordering MP3s preserves slide positions and queue order follows the saved service', () => {
  const opening = track('Opening');
  const closing = track('Closing');
  const slides: Item = {
    id: 'hymn', title: 'Hymn', content: { kind: 'slides', slides: [{ kind: 'text', text: 'Verse' }], audio: null },
  };
  const items = [opening, slides, closing];
  assert.deepEqual(audioTracks({ id: 'service', title: 'Sabbath', items }), [opening, closing]);
  assert.deepEqual(moveTrack(items, closing.id, -1), [closing, slides, opening]);
  assert.deepEqual(moveTrack(items, opening.id, 1), [closing, slides, opening]);
  assert.equal(moveTrack(items, opening.id, -1), items);
  assert.equal(moveTrack(items, slides.id, 1), items);
});

test('progression respects repeat mode, track removal, and manual skips', () => {
  const opening = track('Opening');
  const closing = track('Closing');
  const tracks = [opening, closing];
  assert.equal(adjacentTrack(tracks, opening.id, 1, 'off', true), closing);
  assert.equal(adjacentTrack(tracks, closing.id, 1, 'off', true), undefined);
  assert.equal(adjacentTrack(tracks, closing.id, 1, 'all', true), opening);
  assert.equal(adjacentTrack(tracks, opening.id, -1, 'all'), closing);
  assert.equal(adjacentTrack(tracks, closing.id, 1, 'one', true), closing);
  assert.equal(adjacentTrack(tracks, closing.id, -1, 'one'), opening);
  assert.equal(adjacentTrack([closing], opening.id, 1, 'all', true), undefined);
});
