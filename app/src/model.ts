export type Kind = 'png' | 'jpeg' | 'mp3' | 'mp4';

export interface Asset {
  group: string;
  id: string;
  kind: Kind;
  name: string;
  size: number;
}

export type Slide = { kind: 'text'; text: string } | { kind: 'image'; asset: Asset };
export type Content =
  | { kind: 'slides'; slides: Slide[]; audio: Asset | null }
  | { kind: 'audio'; audio: Asset }
  | { kind: 'video'; video: Asset };

export interface Item { id: string; title: string; content: Content }
export interface Playlist { id: string; title: string; items: Item[] }
export interface Library { schema: number; revision: number; playlists: Playlist[]; selected: string }
export interface Missing { item: string; asset: Asset }

export interface Display { name: string; x: number; y: number; width: number; height: number; scale: number }
export interface DisplayList { displays: Display[]; operator: Display | null; preferred: number | null }

export interface Video { asset: Asset; token: string }
export interface Frame {
  revision: number;
  black: boolean;
  slide: Slide | null;
  video: Video | null;
  candidate: Video | null;
  shared: boolean;
}
export interface Live {
  id: string;
  title: string;
  kind: Content['kind'];
  index: number;
  count: number;
  slide: Slide | null;
}

export type SharedIntent = 'take' | 'next_item' | 'previous_item';
export type Intent = SharedIntent | 'next' | 'previous' | 'black' | 'restore' | 'toggle_black' | 'clear' | 'close';

export interface Status {
  message: string;
  ready: boolean;
  opening: boolean;
  active: boolean;
  shared: boolean;
  shared_intent: SharedIntent | null;
  black: boolean;
  live: Live | null;
  audio: { item: string; asset: Asset; token: string } | null;
  audio_allowed: boolean;
  candidate: [string, boolean, boolean] | null;
}

interface TauriCore {
  invoke<T>(command: string, args?: Record<string, unknown> | ArrayBuffer, options?: { headers: Record<string, string> }): Promise<T>;
  convertFileSrc(path: string, protocol: string): string;
}

declare global {
  interface Window { __TAURI__?: { core: TauriCore } }
}

export function invoke<T>(command: string, args?: Record<string, unknown> | ArrayBuffer, options?: { headers: Record<string, string> }) {
  if (!window.__TAURI__) return Promise.reject(new Error('Open Pelita in the desktop app.'));
  return window.__TAURI__.core.invoke<T>(command, args, options);
}

export function assetUrl(asset: Asset) {
  return window.__TAURI__?.core.convertFileSrc(`${asset.group}_${asset.id}.${asset.kind}`, 'pelita') ?? '';
}

export function accompaniment(item: Item | undefined) {
  return item && item.content.kind !== 'video' ? item.content.audio : null;
}

export function describe(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

export const delay = (ms: number) => new Promise<void>(resolve => window.setTimeout(resolve, ms));

export function fitText(element: HTMLElement) {
  const resize = () => {
    const height = element.clientHeight;
    let size = Math.min(element.clientWidth * 0.078, height * 0.18);
    element.style.fontSize = `${size}px`;
    while (element.scrollHeight > height + 1 && size > 6) {
      size *= 0.88;
      element.style.fontSize = `${size}px`;
    }
  };
  const observer = new ResizeObserver(resize);
  observer.observe(element);
  resize();
  return () => observer.disconnect();
}

export function newId() {
  const bytes = crypto.getRandomValues(new Uint8Array(16));
  bytes[6] = (bytes[6] & 0x0f) | 0x40;
  bytes[8] = (bytes[8] & 0x3f) | 0x80;
  const hex = Array.from(bytes, byte => byte.toString(16).padStart(2, '0')).join('');
  return [hex.slice(0, 8), hex.slice(8, 12), hex.slice(12, 16), hex.slice(16, 20), hex.slice(20)].join('-');
}
