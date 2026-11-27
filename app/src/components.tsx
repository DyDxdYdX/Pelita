import { memo, useEffect, useRef, type FormEvent, type ReactNode } from 'react';
import { assetUrl, fitText, type Content, type Slide } from './model';

const paths = {
  flame: 'M12 22a7 7 0 0 0 7-7c0-4-3-6-4-10-1 3-3 4-3 4-.5-3-2-5-3-7-.5 4-4 7-4 13a7 7 0 0 0 7 7Z M12 22a3 3 0 0 0 3-3c0-2-2-3-3-5-1 2-3 3-3 5a3 3 0 0 0 3 3Z',
  monitor: 'M3 4h18v13H3z M8 21h8 M12 17v4',
  layers: 'm12 3 9 5-9 5-9-5 9-5Z M3 12l9 5 9-5 M3 16l9 5 9-5',
  queue: 'M9 6h12 M9 12h12 M9 18h12 M3 6h1 M3 12h1 M3 18h1',
  plus: 'M12 5v14 M5 12h14',
  text: 'M4 6h16 M12 6v13 M9 19h6',
  file: 'M6 3h8l4 4v14H6z M14 3v5h4 M9 13h6 M9 17h6',
  image: 'M3 4h18v16H3z M3 17l5-5 4 4 3-3 6 5 M15 8h.01',
  video: 'M3 5h18v14H3z M10 9l5 3-5 3V9Z',
  music: 'M9 18V6l11-3v12 M9 9l11-3 M9 18c0 1.1-1.3 2-3 2s-3-.9-3-2 1.3-2 3-2 3 .9 3 2Z M20 15c0 1.1-1.3 2-3 2s-3-.9-3-2 1.3-2 3-2 3 .9 3 2Z',
  volume: 'M3 9h4l5-4v14l-5-4H3z M16 9a4 4 0 0 1 0 6 M18 6a8 8 0 0 1 0 12',
  arrow: 'M4 12h15 M13 6l6 6-6 6',
  left: 'm15 6-6 6 6 6',
  right: 'm9 6 6 6-6 6',
  up: 'm6 15 6-6 6 6',
  down: 'm6 9 6 6 6-6',
  play: 'm9 5 11 7-11 7V5Z',
  pause: 'M9 5v14 M15 5v14',
  repeat: 'M4 5v6h6 M4 11a8 8 0 1 1 .7 6',
  check: 'm5 12 4 4L19 6',
  warning: 'm12 3 10 17H2L12 3Z M12 9v4 M12 16h.01',
  pencil: 'm13 6 5 5 M4 20l5-1L20 8l-5-5L4 14v6Z',
  copy: 'M8 8h12v12H8z M4 16V4h12',
  x: 'm6 6 12 12 M18 6 6 18',
  trash: 'M4 6h16 M9 6V3h6v3 M6 6l1 15h10l1-15 M10 10v7 M14 10v7',
  square: 'M5 5h14v14H5z',
  eye: 'M2 12s4-7 10-7 10 7 10 7-4 7-10 7-10-7-10-7Z M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z',
  settings: 'M4 7h16 M4 12h16 M4 17h16 M8 4v6 M16 9v6 M11 14v6',
  keyboard: 'M2 5h20v14H2z M6 9h.01 M10 9h.01 M14 9h.01 M18 9h.01 M7 14h10',
} as const;

export type IconName = keyof typeof paths;

export function Icon({ name, size = 17 }: { name: IconName; size?: number }) {
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor"
    strokeWidth="1.65" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d={paths[name]} /></svg>;
}

export function IconButton({ icon, label, disabled, onClick }: {
  icon: IconName; label: string; disabled?: boolean; onClick: () => void;
}) {
  return <button className="icon-button" type="button" aria-label={label} title={label} disabled={disabled} onClick={onClick}>
    <Icon name={icon} />
  </button>;
}

export function contentIcon(content: Content): IconName {
  if (content.kind === 'video') return 'video';
  if (content.kind === 'audio') return 'music';
  return content.slides.every(slide => slide.kind === 'text') ? 'text' : 'image';
}

export function contentDetail(content: Content) {
  if (content.kind === 'video') return 'MP4 video';
  if (content.kind === 'audio') return 'MP3 audio';
  return `${content.presentation ? 'PPTX · ' : ''}${content.slides.length} ${content.slides.length === 1 ? 'slide' : 'slides'}${content.audio ? ' · MP3' : ''}`;
}

export const SlideView = memo(function SlideView({ slide, thumbnail = false }: { slide: Slide; thumbnail?: boolean }) {
  const text = useRef<HTMLSpanElement>(null);
  const words = slide.kind === 'text' ? slide.text : null;
  useEffect(() => {
    if (text.current) return fitText(text.current);
  }, [words]);
  return slide.kind === 'image'
    ? <img className="frame-image" src={assetUrl(thumbnail && slide.thumbnail ? slide.thumbnail : slide.asset)} alt={thumbnail ? '' : slide.asset.name}
      loading={thumbnail ? 'lazy' : undefined} decoding={thumbnail ? 'async' : undefined} />
    : <span ref={text} className="frame-text">{words}</span>;
});

export const SlideNavigator = memo(function SlideNavigator({ slides, previewIndex, liveIndex, choose }: {
  slides: Slide[]; previewIndex: number; liveIndex?: number; choose: (index: number) => void;
}) {
  const strip = useRef<HTMLOListElement>(null);
  const currentIndex = liveIndex ?? previewIndex;
  useEffect(() => {
    const list = strip.current!;
    const thumb = list.children[currentIndex] as HTMLElement | undefined;
    if (!thumb) return;
    const start = thumb.offsetLeft - 6;
    const end = thumb.offsetLeft + thumb.offsetWidth + 6;
    if (start < list.scrollLeft) list.scrollLeft = start;
    else if (end > list.scrollLeft + list.clientWidth) list.scrollLeft = end - list.clientWidth;
  }, [slides, currentIndex]);
  return <section className="panel slide-navigator" aria-label="Slide navigator">
    <div className="panel-heading">
      <h2><Icon name="layers" />Slides</h2>
      <span className="count">{slides.length}</span>
    </div>
    <ol className="thumbnail-strip" ref={strip}>
      {slides.map((slide, index) => <li key={index}>
        <button type="button" className={index === liveIndex ? 'live-thumb' : index === previewIndex ? 'selected-thumb' : ''}
          aria-label={`Slide ${index + 1}${index === liveIndex ? ', live' : ''}`}
          aria-current={index === currentIndex ? 'step' : undefined}
          title={slide.kind === 'text' ? slide.text : slide.asset.name} onClick={() => choose(index)}>
          <span className="thumbnail-frame" aria-hidden="true">
            {slide.kind === 'text' ? <span className="thumbnail-text-canvas"><SlideView slide={slide} thumbnail /></span>
              : <SlideView slide={slide} thumbnail />}
          </span>
          <span>{index + 1}{index === liveIndex && ' · Live'}</span>
        </button>
      </li>)}
    </ol>
  </section>;
});

export function Dialog({ title, children, close, onSubmit, submitLabel, disabled }: {
  title: string; children: ReactNode; close: () => void;
  onSubmit?: (event: FormEvent<HTMLFormElement>) => void; submitLabel?: string; disabled?: boolean;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    const dialog = ref.current!;
    dialog.showModal();
    return () => dialog.close();
  }, []);
  return <dialog ref={ref} onCancel={event => { event.preventDefault(); close(); }} aria-labelledby="dialog-title">
    <form onSubmit={event => { event.preventDefault(); onSubmit?.(event); }}>
      <header className="dialog-head"><h2 id="dialog-title">{title}</h2><IconButton icon="x" label="Close dialog" onClick={close} /></header>
      <div className="dialog-body">{children}</div>
      {onSubmit && <footer className="dialog-foot">
        <button type="button" onClick={close}>Cancel</button>
        <button className="primary" type="submit" disabled={disabled}>{submitLabel ?? 'Save'}</button>
      </footer>}
    </form>
  </dialog>;
}
