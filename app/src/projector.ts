import { assetUrl, fitText, invoke, type Frame, type Intent, type Video } from './model';
import './projection.css';

const output = document.querySelector<HTMLElement>('#output')!;
const surface = document.createElement('div');
surface.className = 'surface';
const mask = document.createElement('div');
mask.className = 'mask';
output.append(surface, mask);

interface Player {
  token: string;
  node: HTMLVideoElement;
  ready: boolean;
  failed: boolean;
}

let candidate: Player | null = null;
let live: Player | null = null;
let removeFit: (() => void) | undefined;
let slideKey = '';
let seen = 0;
let shared = false;

function release(player: Player | null) {
  if (!player) return;
  player.node.pause();
  player.node.removeAttribute('src');
  player.node.load();
  player.node.remove();
}

function load(video: Video): Player {
  const node = document.createElement('video');
  const player = { token: video.token, node, ready: false, failed: false };
  node.className = 'frame-image';
  node.hidden = true;
  node.playsInline = true;
  node.preload = 'auto';
  node.addEventListener('loadeddata', () => {
    player.ready = node.videoWidth > 0 && Number.isFinite(node.duration) && node.duration > 0;
    player.failed = !player.ready;
  });
  node.addEventListener('error', () => { player.failed = true; });
  node.src = assetUrl(video.asset);
  surface.append(node);
  return player;
}

function render(frame: Frame) {
  if (frame.shared && !shared) output.focus({ preventScroll: true });
  shared = frame.shared;
  if (frame.video?.token !== live?.token) {
    release(live);
    live = null;
    if (frame.video) {
      live = candidate?.token === frame.video.token ? candidate : load(frame.video);
      if (live === candidate) candidate = null;
      live.node.hidden = false;
      const player = live;
      void player.node.play().catch(() => { player.failed = true; });
    }
  }
  if (frame.candidate?.token !== candidate?.token) {
    release(candidate);
    candidate = frame.candidate ? load(frame.candidate) : null;
  }
  const key = JSON.stringify(frame.slide);
  if (key !== slideKey) {
    slideKey = key;
    removeFit?.();
    surface.querySelector('.slide')?.remove();
    if (frame.slide?.kind === 'image') {
      const image = document.createElement('img');
      image.className = 'slide frame-image';
      image.alt = '';
      image.src = assetUrl(frame.slide.asset);
      surface.prepend(image);
    } else if (frame.slide?.kind === 'text') {
      const text = document.createElement('div');
      text.className = 'slide frame-text';
      text.textContent = frame.slide.text;
      surface.prepend(text);
      removeFit = fitText(text);
    }
  }
  mask.hidden = !frame.black;
  seen = frame.revision;
}

async function poll() {
  try {
    const frame = await invoke<Frame | null>('projector', {
      feedback: {
        seen,
        ready: candidate?.ready ? candidate.token : null,
        failed: live?.failed ? live.token : candidate?.failed ? candidate.token : null,
      },
    });
    if (frame) render(frame);
    else {
      mask.hidden = false;
      release(live);
      release(candidate);
      live = null;
      candidate = null;
    }
  } catch {
    mask.hidden = false;
    release(live);
    release(candidate);
    live = null;
    candidate = null;
  } finally {
    window.setTimeout(poll, 90);
  }
}

const keys: Record<string, Intent> = {
  ArrowRight: 'next', ArrowLeft: 'previous', Space: 'next',
  KeyB: 'toggle_black', KeyR: 'restore', KeyC: 'clear', Escape: 'close',
  Enter: 'take', PageDown: 'next_item', PageUp: 'previous_item',
};
window.addEventListener('keydown', event => {
  const intent = keys[event.code];
  if (!shared || !intent || event.metaKey || event.ctrlKey || event.altKey) return;
  event.preventDefault();
  if (!event.repeat) void invoke('action', { intent }).catch(() => undefined);
});
void poll();
