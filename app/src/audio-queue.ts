import type { Content, Item, Playlist } from './model';

export type Track = Item & { content: Extract<Content, { kind: 'audio' }> };
export type Repeat = 'off' | 'all' | 'one';

export function audioTracks(service: Playlist | undefined): Track[] {
  return service?.items.filter((item): item is Track => item.content.kind === 'audio') ?? [];
}

export function adjacentTrack(tracks: Track[], id: string, step: -1 | 1, repeat: Repeat, ended = false) {
  const index = tracks.findIndex(track => track.id === id);
  if (index < 0) return;
  if (ended && repeat === 'one') return tracks[index];
  return tracks[index + step] ?? (repeat === 'all' ? tracks[step === 1 ? 0 : tracks.length - 1] : undefined);
}

export function moveTrack(items: Item[], id: string, step: -1 | 1) {
  const index = items.findIndex(item => item.id === id && item.content.kind === 'audio');
  if (index < 0) return items;
  let adjacent = index + step;
  while (items[adjacent] && items[adjacent].content.kind !== 'audio') adjacent += step;
  if (!items[adjacent]) return items;
  const ordered = [...items];
  [ordered[index], ordered[adjacent]] = [ordered[adjacent], ordered[index]];
  return ordered;
}
