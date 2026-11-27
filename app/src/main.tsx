import { useCallback, useEffect, useMemo, useRef, useState, type ChangeEvent } from 'react';
import { createRoot } from 'react-dom/client';
import { contentDetail, contentIcon, Dialog, Icon, IconButton, SlideNavigator, SlideView } from './components';
import { useAudio } from './audio';
import { audioTracks, moveTrack } from './audio-queue';
import { decodeMedia, importFiles } from './import';
import { processPresentationBatch, type Selection } from './presentation-batch';
import {
  preparePresentation, PresentationSettings, providerLabel, RegeneratePresentation, type Progress,
} from './presentations';
import {
  accompaniment, assetUrl, channel, delay, describe, invoke, newId, type Asset, type Content, type DisplayList,
  type Intent, type Item, type Kind, type Library, type Missing, type Playlist, type Provider, type SharedIntent, type Slide, type Status,
} from './model';
import './projection.css';
import './style.css';

const emptyStatus: Status = {
  revision: 0, message: '', ready: false, opening: false, active: false, shared: false, shared_intent: null,
  black: true, live: null, audio: null, audio_allowed: true, candidate: null,
};
const noDisplays: DisplayList = { displays: [], operator: null, preferred: null };
const noSlides: Slide[] = [];
const sameDisplay = (a: unknown, b: unknown) => JSON.stringify(a) === JSON.stringify(b);

type Editor = { id: string | null; title: string; text: string; textDeck: boolean };
type PlaylistEditor = { mode: 'new' | 'rename' | 'duplicate'; title: string };
type FileIntent = { type: 'add' } | { type: 'audio' } | { type: 'attach'; item: string } | { type: 'relink'; target: Missing };

function importedItems(assets: Asset[]): Item[] {
  const deck = assets.filter(asset => asset.kind === 'png' || asset.kind === 'jpeg');
  const media = assets.filter(asset => asset.kind === 'mp3' || asset.kind === 'mp4');
  const title = (asset: Asset) => asset.name.replace(/ · page \d+$/, '').replace(/\.[^.]+$/, '').slice(0, 160) || 'Imported item';
  const items: Item[] = media.map(asset => ({
    id: newId(), title: title(asset),
    content: asset.kind === 'mp4' ? { kind: 'video', video: asset } : { kind: 'audio', audio: asset },
  }));
  if (deck.length) items.unshift({
    id: newId(), title: title(deck[0]),
    content: { kind: 'slides', slides: deck.map(asset => ({ kind: 'image', asset })), audio: null },
  });
  return items;
}

function replaceAsset(content: Content, target: string, asset: Asset): Content {
  if (content.kind === 'video') return { ...content, video: asset };
  if (content.kind === 'audio') return { ...content, audio: asset };
  return {
    ...content,
    audio: content.audio?.id === target ? asset : content.audio,
    slides: content.slides.map(slide => slide.kind === 'image' && slide.asset.id === target ? { ...slide, asset } : slide),
  };
}

function App() {
  const [library, setLibrary] = useState<Library | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [previewIndex, setPreviewIndex] = useState(0);
  const [status, setStatus] = useState(emptyStatus);
  const [screens, setScreens] = useState(noDisplays);
  const [displayIndex, setDisplayIndex] = useState<number | null>(null);
  const [normalBusy, setBusy] = useState('');
  const [presentationProgress, setPresentationProgress] = useState<Progress | null>(null);
  const [failedPresentations, setFailedPresentations] = useState<{ source: Selection; message: string }[]>([]);
  const [regenerateTarget, setRegenerateTarget] = useState<Item | null>(null);
  const busy = presentationProgress?.message || normalBusy;
  const [notice, setNotice] = useState('');
  const [problem, setProblem] = useState('');
  const [missing, setMissing] = useState<Missing[] | null>(null);
  const [editor, setEditor] = useState<Editor | null>(null);
  const [playlistEditor, setPlaylistEditor] = useState<PlaylistEditor | null>(null);
  const [dialog, setDialog] = useState<'projector' | 'shared' | 'shortcuts' | 'remove' | 'settings' | null>(null);
  const [panel, setPanel] = useState<'service' | 'audio'>('service');
  const [selectedTrack, setSelectedTrack] = useState<string | null>(null);
  const [removeTarget, setRemoveTarget] = useState<Item | null>(null);
  const [takenDeck, setTakenDeck] = useState<{ token: string; slides: Slide[] } | null>(null);
  const audio = useAudio(library, status);
  const previewVideo = useRef<HTMLVideoElement>(null);
  const [previewPlaying, setPreviewPlaying] = useState(false);
  const latestStatus = useRef(emptyStatus);
  const operation = useRef(false);
  const stopImport = useRef<AbortController | null>(null);
  const presentationAbort = useRef<AbortController | null>(null);
  const receiveDrop = useRef<(sources: Selection[]) => void>(() => {});
  const fileInput = useRef<HTMLInputElement>(null);
  const issuesPanel = useRef<HTMLElement>(null);
  const fileIntent = useRef<FileIntent>({ type: 'add' });
  const takeShortcut = useRef<(intent: SharedIntent) => void>(() => {});
  const playlist = library?.playlists.find(candidate => candidate.id === library.selected);
  const item = playlist?.items.find(candidate => candidate.id === selected);
  const preview = item?.content.kind === 'slides' ? item.content.slides : noSlides;
  const currentDeck = useMemo(() => takenDeck && JSON.stringify(takenDeck.slides) === JSON.stringify(preview), [takenDeck, preview]);
  const liveIndex = currentDeck && status.live?.id === item?.id && status.live?.token === takenDeck?.token
    ? status.live?.index : undefined;
  const jumpToken = liveIndex === undefined ? undefined : status.live?.token;
  const tracks = audioTracks(playlist);
  const track = tracks.find(candidate => candidate.id === selectedTrack) ?? tracks[0];
  const trackIndex = tracks.findIndex(candidate => candidate.id === track?.id);
  const testing = audio.source?.kind === 'test';
  const previewAudio = accompaniment(item);
  const presentation = item?.content.kind === 'slides' ? item.content.presentation : undefined;
  const hasModal = Boolean(editor || playlistEditor || dialog || regenerateTarget);

  useEffect(() => {
    if (missing?.length) issuesPanel.current?.scrollIntoView({ block: 'nearest' });
  }, [missing]);

  function clearFeedback() {
    setProblem('');
    setNotice('');
  }

  async function run(label: string, task: () => Promise<unknown>) {
    if (operation.current) return;
    operation.current = true;
    clearFeedback();
    setBusy(label);
    try {
      await task();
    } catch (error) {
      if (error instanceof DOMException && error.name === 'AbortError') setNotice('Import cancelled.');
      else setProblem(describe(error));
    } finally {
      setBusy('');
      operation.current = false;
    }
  }

  async function refreshDisplays() {
    const next = await invoke<DisplayList>('displays');
    setDisplayIndex(previous => {
      const old = previous === null ? null : screens.displays[previous];
      const index = old ? next.displays.findIndex(candidate => sameDisplay(candidate, old)) : -1;
      if (index >= 0 && !sameDisplay(next.displays[index], next.operator)) return index;
      return next.preferred !== null && !sameDisplay(next.displays[next.preferred], next.operator) ? next.preferred : null;
    });
    setScreens(next);
  }

  function receiveStatus(next: Status) {
    if (JSON.stringify(latestStatus.current) !== JSON.stringify(next)) setStatus(next);
    latestStatus.current = next;
    if (next.shared_intent) takeShortcut.current(next.shared_intent);
  }

  useEffect(() => {
    void invoke<Library>('library').then(next => {
      setLibrary(next);
      setSelected(next.playlists.find(list => list.id === next.selected)?.items[0]?.id ?? null);
    }).catch(error => setProblem(describe(error)));
    void refreshDisplays().catch(error => setProblem(describe(error)));
    if (window.__TAURI__) {
      const drops = channel<Selection[]>(sources => receiveDrop.current(sources));
      void invoke('presentation_drops', { drops }).catch(error => setProblem(describe(error)));
    }
    let cancelled = false;
    let timer: number;
    async function poll() {
      try {
        const next = await invoke<Status>('status');
        if (!cancelled) receiveStatus(next);
      } catch (error) {
        if (!cancelled && window.__TAURI__) setProblem(describe(error));
      } finally {
        if (!cancelled && window.__TAURI__) timer = window.setTimeout(poll, 200);
      }
    }
    if (window.__TAURI__) void poll();
    return () => { cancelled = true; clearTimeout(timer); };
  }, []);

  function choose(id: string | null) {
    if (testing && id !== selected) audio.stopProjectionAudio();
    setSelected(id);
    setPreviewIndex(0);
    setPreviewPlaying(false);
  }

  async function save(next: Library, nextSelection?: string | null) {
    const saved = await invoke<Library>('save_library', { next });
    setLibrary(saved);
    setMissing(null);
    if (nextSelection !== undefined) choose(nextSelection);
    return saved;
  }

  function withItems(items: Item[]): Library {
    return { ...library!, playlists: library!.playlists.map(candidate => candidate.id === playlist!.id ? { ...candidate, items } : candidate) };
  }

  async function updateItem(updated: Item) {
    await save(withItems(playlist!.items.map(candidate => candidate.id === updated.id ? updated : candidate)));
    setPreviewIndex(0);
  }

  function openFiles(intent: FileIntent) {
    if (operation.current || presentationAbort.current) return;
    const input = fileInput.current!;
    fileIntent.current = intent;
    input.accept = intent.type === 'attach' || intent.type === 'audio' ? '.mp3'
      : intent.type === 'relink' ? acceptKind(intent.target.asset.kind)
      : '.pdf,.png,.jpg,.jpeg,.mp3,.mp4';
    input.multiple = intent.type === 'add' || intent.type === 'audio';
    input.value = '';
    input.click();
  }

  function filesChosen(event: ChangeEvent<HTMLInputElement>) {
    const files = Array.from(event.currentTarget.files ?? []);
    if (!files.length || !playlist) return;
    const intent = fileIntent.current;
    const target = intent.type === 'attach' ? intent.item : intent.type === 'relink' ? intent.target.item : null;
    const chosenItem = playlist.items.find(candidate => candidate.id === target);
    void run('Preparing files…', async () => {
      const abort = new AbortController();
      stopImport.current = abort;
      try {
        const only = intent.type === 'attach' || intent.type === 'audio' ? 'mp3' : intent.type === 'relink' ? intent.target.asset.kind : undefined;
        const assets = await importFiles(files, setBusy, abort.signal, only);
        if (intent.type === 'add' || intent.type === 'audio') {
          const additions = importedItems(assets);
          await save(withItems([...playlist.items, ...additions]), intent.type === 'add' ? additions[0].id : undefined);
          if (intent.type === 'audio') setSelectedTrack(additions[0].id);
          setNotice(`Added ${additions.length} ${additions.length === 1 ? 'item' : 'items'}.`);
        } else if (chosenItem && intent.type === 'attach' && chosenItem.content.kind === 'slides') {
          await updateItem({ ...chosenItem, content: { ...chosenItem.content, audio: assets[0] } });
          setNotice('Accompaniment attached.');
        } else if (chosenItem && intent.type === 'relink') {
          await updateItem({ ...chosenItem, content: replaceAsset(chosenItem.content, intent.target.asset.id, assets[0]) });
          setMissing(await invoke<Missing[]>('preflight', { playlist: playlist.id }));
          setNotice('Asset relinked.');
        }
      } finally {
        stopImport.current = null;
      }
    });
  }

  async function withPresentationJob(task: (signal: AbortSignal) => Promise<void>) {
    if (operation.current || presentationAbort.current) return;
    clearFeedback();
    const abort = new AbortController();
    presentationAbort.current = abort;
    setPresentationProgress({ message: 'Preparing PowerPoint…', completed: null, total: null });
    try {
      await task(abort.signal);
    } catch (error) {
      if (abort.signal.aborted) setNotice('PowerPoint import cancelled.');
      else setProblem(describe(error));
    } finally {
      presentationAbort.current = null;
      setPresentationProgress(null);
    }
  }

  function importPresentations(sources: Selection[]) {
    if (!library || !playlist || !sources.length) return;
    void withPresentationJob(async signal => {
      setFailedPresentations([]);
      let draft = library;
      let entries = playlist.items;
      const { added, reused, fallbacks, failures } = await processPresentationBatch(sources, signal, async source => {
        const update = (progress: Progress) => setPresentationProgress({
          ...progress, message: `${source.name} · ${progress.message}`,
        });
        update({ message: 'Reading file…', completed: null, total: null });
        const prepared = await preparePresentation({ kind: 'source', id: source.id }, update, signal);
        const addition: Item = { id: newId(), title: prepared.title, content: {
          kind: 'slides', slides: prepared.slides, presentation: prepared.presentation, audio: null,
        } };
        const items = [...entries, addition];
        const next = { ...draft, playlists: draft.playlists.map(list => list.id === playlist.id ? { ...list, items } : list) };
        draft = await save(next, entries.length ? undefined : addition.id);
        entries = items;
        return prepared;
      });
      setFailedPresentations(failures);
      const summary = [
        signal.aborted ? 'PowerPoint import cancelled.' : '',
        added ? `Added ${added} PowerPoint ${added === 1 ? 'presentation' : 'presentations'}.` : '',
        reused ? `${reused} reused saved slides.` : '',
        fallbacks ? `LibreOffice was used for ${fallbacks} after PowerPoint conversion failed.` : '',
      ].filter(Boolean).join(' ');
      if (failures.length) {
        const count = `${failures.length} PowerPoint ${failures.length === 1 ? 'file failed' : 'files failed'}.`;
        setProblem([summary, count, 'Open PowerPoint import results for details.'].filter(Boolean).join(' '));
      } else {
        setNotice(summary);
      }
    });
  }

  receiveDrop.current = sources => {
    if (!hasModal && !busy) importPresentations(sources);
  };

  async function pickPresentations() {
    if (operation.current || presentationAbort.current) return;
    try {
      importPresentations(await invoke<Selection[]>('pick_presentations'));
    } catch (error) {
      setProblem(describe(error));
    }
  }

  function regeneratePresentation(provider: Provider | null) {
    const target = regenerateTarget;
    setRegenerateTarget(null);
    if (target?.content.kind !== 'slides' || !playlist) return;
    const content = target.content;
    void withPresentationJob(async signal => {
      const prepared = await preparePresentation({ kind: 'regenerate', item: target.id, provider }, setPresentationProgress, signal);
      await save(withItems(playlist.items.map(entry => entry.id === target.id
        ? { ...entry, content: { ...content, presentation: prepared.presentation, slides: prepared.slides } }
        : entry)));
      setNotice(prepared.fallback
        ? 'Regenerated with LibreOffice after PowerPoint conversion failed.'
        : `Regenerated with ${providerLabel[prepared.presentation.provider]}.`);
    });
  }

  function editItem() {
    if (!item) return;
    const slides = item.content.kind === 'slides' ? item.content.slides : [];
    const textDeck = slides.length > 0 && slides.every(slide => slide.kind === 'text');
    setEditor({
      id: item.id, title: item.title, textDeck,
      text: slides.map(slide => slide.kind === 'text' ? slide.text : '').join('\n---\n'),
    });
  }

  function saveItem() {
    if (!editor || !playlist) return;
    const current = playlist.items.find(candidate => candidate.id === editor.id);
    const title = editor.title.trim();
    const blocks = editor.text.split(/^\s*---\s*$/m).map(text => text.trim()).filter(Boolean);
    if (editor.textDeck && (!blocks.length || blocks.length > 100 || blocks.some(text => [...text].length > 360))) {
      setProblem('Enter 1–100 text slides with up to 360 characters per slide.');
      return;
    }
    const content: Content = editor.textDeck
      ? { kind: 'slides', slides: blocks.map(text => ({ kind: 'text', text })), audio: current ? accompaniment(current) ?? null : null }
      : current!.content;
    const updated = { id: current?.id ?? newId(), title, content };
    void run('Saving item…', async () => {
      if (current) await updateItem(updated);
      else await save(withItems([...playlist.items, updated]), updated.id);
      setEditor(null);
    });
  }

  function savePlaylist() {
    if (!playlistEditor || !library || !playlist) return;
    const { mode, title } = playlistEditor;
    const next: Playlist = {
      id: mode === 'rename' ? playlist.id : newId(),
      title: title.trim(),
      items: mode === 'duplicate' ? playlist.items.map(candidate => ({ ...candidate, id: newId() }))
        : mode === 'rename' ? playlist.items : [],
    };
    void run('Saving service…', async () => {
      const playlists = mode === 'rename' ? library.playlists.map(candidate => candidate.id === next.id ? next : candidate)
        : [...library.playlists, next];
      await save({ ...library, selected: next.id, playlists }, mode === 'rename' ? undefined : next.items[0]?.id ?? null);
      setPlaylistEditor(null);
    });
  }

  function move(offset: number) {
    if (!playlist || !item) return;
    const index = playlist.items.findIndex(candidate => candidate.id === item.id);
    const next = index + offset;
    if (next < 0 || next >= playlist.items.length) return;
    const items = [...playlist.items];
    [items[index], items[next]] = [items[next], items[index]];
    void run('Saving order…', () => save(withItems(items)));
  }

  function removeItem() {
    if (!playlist || !removeTarget) return;
    const index = playlist.items.findIndex(candidate => candidate.id === removeTarget.id);
    const remaining = playlist.items.filter(candidate => candidate.id !== removeTarget.id);
    const nextSelection = removeTarget.id === selected ? remaining[Math.min(index, remaining.length - 1)]?.id ?? null : undefined;
    void run('Removing item…', async () => {
      await save(withItems(remaining), nextSelection);
      setDialog(null);
    });
  }

  function askRemove(target: Item) {
    clearFeedback();
    setRemoveTarget(target);
    setDialog('remove');
  }

  function preflight() {
    if (!playlist) return;
    void run('Checking service…', async () => {
      const issues = await invoke<Missing[]>('preflight', { playlist: playlist.id });
      setMissing(issues);
      if (!issues.length) setNotice('Preflight passed. All service files are available.');
    });
  }

  function skip(id: string) {
    if (!playlist) return;
    const index = playlist.items.findIndex(candidate => candidate.id === id);
    const blocked = new Set(missing?.map(issue => issue.item) ?? []);
    const next = playlist.items.slice(index + 1).find(candidate => !blocked.has(candidate.id));
    choose(next?.id ?? null);
    setNotice(next ? `Selected ${next.title}.` : 'No later item is available.');
  }

  function confirm(mode: 'extended' | 'shared') {
    setDialog(null);
    void run('Opening output…', () => invoke('confirm', { topology: screens.displays, index: displayIndex, mode }));
  }

  function act(intent: Intent) {
    void invoke('action', { intent }).then(() => {
      if (intent === 'clear') audio.stopProjectionAudio();
    }).catch(error => setProblem(describe(error)));
  }

  function take(target: Item | undefined, index = 0) {
    if (!target) return;
    previewVideo.current?.pause();
    void run('Preparing Take…', async () => {
      const token = await invoke<string>('prepare_take', { id: target.id });
      const track = accompaniment(target);
      const before = latestStatus.current;
      const keepAudio = Boolean(track && !before.live && before.audio?.item === target.id && before.audio.asset.id === track.id);
      let finishVideoCut: ((revision?: number) => void) | undefined;
      try {
        if (track && !keepAudio) await decodeMedia('mp3', assetUrl(track));
        if (target.content.kind === 'video') {
          const deadline = performance.now() + 10000;
          for (;;) {
            const candidate = latestStatus.current.candidate;
            if (candidate?.[0] === token && candidate[2]) throw new Error('The projector could not decode this video.');
            if (candidate?.[0] === token && candidate[1]) break;
            if (!latestStatus.current.ready || performance.now() > deadline) throw new Error('The projector could not prepare this video. Check the display and file.');
            await delay(120);
          }
        }
        if (target.content.kind === 'video') finishVideoCut = audio.pauseForVideo();
        const revision = await invoke<number>('take', { token, index });
        finishVideoCut?.(revision);
        if (!track && !finishVideoCut) audio.stopProjectionAudio();
        setTakenDeck(target.content.kind === 'slides' ? { token, slides: target.content.slides } : null);
      } catch (error) {
        await invoke('cancel_take', { token }).catch(() => undefined);
        finishVideoCut?.();
        throw error;
      }
    });
  }

  takeShortcut.current = intent => {
    if (!playlist || operation.current) return;
    if (intent === 'take') {
      take(item, previewIndex);
      return;
    }
    const liveId = latestStatus.current.live?.id ?? selected;
    const index = playlist.items.findIndex(candidate => candidate.id === liveId);
    const adjacent = playlist.items[index + (intent === 'next_item' ? 1 : -1)];
    if (adjacent) {
      choose(adjacent.id);
      take(adjacent);
    }
  };

  const chooseSlide = useCallback((index: number) => {
    setPreviewIndex(index);
    if (jumpToken) void invoke('action', { intent: { jump: { token: jumpToken, index } } })
      .catch(error => setProblem(describe(error)));
  }, [jumpToken]);

  useEffect(() => {
    function keydown(event: KeyboardEvent) {
      const inControl = event.target instanceof HTMLElement && event.target.closest('input,textarea,select,button,[contenteditable]');
      const slideArrow = (event.code === 'ArrowLeft' || event.code === 'ArrowRight') && inControl && inControl.closest('.thumbnail-strip');
      if (hasModal || event.altKey || event.metaKey || event.ctrlKey || event.repeat || (inControl && !slideArrow)) return;
      const keys: Record<string, Intent> = { ArrowRight: 'next', ArrowLeft: 'previous', KeyB: 'toggle_black', KeyR: 'restore', KeyC: 'clear' };
      if (event.code === 'Enter' && status.ready && item && !normalBusy) {
        event.preventDefault();
        take(item, previewIndex);
      } else if (keys[event.code] && (status.ready || event.code === 'KeyC')) {
        event.preventDefault();
        act(keys[event.code]);
      }
    }
    window.addEventListener('keydown', keydown);
    return () => window.removeEventListener('keydown', keydown);
  });

  const selectedIndex = playlist?.items.findIndex(candidate => candidate.id === selected) ?? -1;
  const readyState = status.opening ? 'Connecting' : !status.ready ? 'Standby'
    : !status.live ? 'Ready' : status.black ? 'Black' : 'Live';

  return <div className="workspace">
    <header className="topbar">
      <div className="brand"><span className="brand-mark"><img src="/brand/pelita-symbol.png" alt="" /></span><span>Pelita</span></div>
      <div className="top-actions">
        <span className={`pill ${status.ready ? 'online' : ''}`}><span className="dot" />{status.ready ? 'Output connected' : status.opening ? 'Connecting' : 'Output offline'}</span>
        <button onClick={() => { if (!busy) clearFeedback(); setDialog('projector'); }}><Icon name="monitor" />Projector</button>
        <IconButton icon="settings" label="Presentation conversion settings" disabled={!!busy} onClick={() => setDialog('settings')} />
        <IconButton icon="keyboard" label="Keyboard shortcuts" onClick={() => setDialog('shortcuts')} />
      </div>
    </header>

    <main className="workspace-body">
      <div className="service-header">
        <h1 title={playlist?.title}>{playlist?.title ?? 'Pelita'}</h1>
        <button disabled={!playlist || !!busy} onClick={preflight}><Icon name="check" />Run preflight</button>
      </div>

      <div className="workspace-grid">
        <aside className="panel queue-panel" aria-label="Service and audio lists">
          <div className="list-switch" role="group" aria-label="Playlist view">
            <button aria-pressed={panel === 'service'} onClick={() => setPanel('service')}><Icon name="queue" />Playlist</button>
            <button aria-pressed={panel === 'audio'} onClick={() => setPanel('audio')}><Icon name="music" />Audio playlist</button>
          </div>
          {panel === 'service' ? <>
            <div className="playlist-tools">
              <label className="visually-hidden" htmlFor="service">Saved service</label>
              <select id="service" value={library?.selected ?? ''} disabled={!library || !!busy} onChange={event => {
                const next = library!.playlists.find(candidate => candidate.id === event.target.value)!;
                void run('Opening service…', () => save({ ...library!, selected: next.id }, next.items[0]?.id ?? null));
              }}>
                {library?.playlists.map(candidate => <option key={candidate.id} value={candidate.id}>{candidate.title}</option>)}
              </select>
              <IconButton icon="plus" label="New service" disabled={!library || !!busy} onClick={() => { clearFeedback(); setPlaylistEditor({ mode: 'new', title: '' }); }} />
              <IconButton icon="copy" label="Duplicate service" disabled={!playlist || !!busy} onClick={() => { clearFeedback(); setPlaylistEditor({ mode: 'duplicate', title: `${playlist!.title.slice(0, 150)} copy` }); }} />
              <IconButton icon="pencil" label="Rename service" disabled={!playlist || !!busy} onClick={() => { clearFeedback(); setPlaylistEditor({ mode: 'rename', title: playlist!.title }); }} />
            </div>
            <ol className="queue">
              {playlist?.items.map((entry, index) => <li key={entry.id}>
                <button className={`queue-item ${entry.id === selected ? 'selected' : ''} ${audio.source?.kind === 'queue' && audio.source.item === entry.id ? 'active-track' : ''}`}
                  aria-pressed={entry.id === selected} disabled={!!busy}
                  onClick={() => choose(entry.id)}>
                  <span className="queue-index">{index + 1}</span>
                  <span className="item-icon"><Icon name={contentIcon(entry.content)} /></span>
                  <span className="queue-label"><strong>{entry.title}</strong><small>{contentDetail(entry.content)}</small></span>
                  {status.live?.id === entry.id && <span className="queue-live" aria-label="On output"><Icon name="monitor" size={14} /></span>}
                  {missing?.some(issue => issue.item === entry.id) && <span className="warning-icon" aria-label="Missing asset"><Icon name="warning" size={15} /></span>}
                </button>
              </li>)}
            </ol>
            {!playlist?.items.length && <div className="queue-empty"><Icon name="layers" size={29} /><p>{library ? 'Your playlist is empty' : 'Opening service library…'}</p></div>}
            <div className="queue-actions service-imports">
              <button disabled={!playlist || !!busy} onClick={() => { clearFeedback(); setEditor({ id: null, title: '', text: '', textDeck: true }); }}><Icon name="plus" />Text slides</button>
              <button disabled={!playlist || !!busy} onClick={() => openFiles({ type: 'add' })}><Icon name="file" />Import files</button>
              <button disabled={!playlist || !!busy} onClick={() => void pickPresentations()}><Icon name="file" />Import PowerPoint</button>
            </div>
            <div className="item-actions">
              <IconButton icon="up" label="Move item up" disabled={!!busy || selectedIndex < 1} onClick={() => move(-1)} />
              <IconButton icon="down" label="Move item down" disabled={!!busy || selectedIndex < 0 || selectedIndex >= playlist!.items.length - 1} onClick={() => move(1)} />
              <span className="spacer" />
              <IconButton icon="pencil" label="Edit item" disabled={!item || !!busy} onClick={() => { clearFeedback(); editItem(); }} />
              <IconButton icon="trash" label="Remove item" disabled={!item || !!busy} onClick={() => { if (item) askRemove(item); }} />
            </div>
            {(item?.content.kind === 'slides' || previewAudio) && <div className="item-actions accompaniment-tools">
              {item?.content.kind === 'slides' && <button disabled={!!busy} onClick={() => openFiles({ type: 'attach', item: item.id })}>{previewAudio ? 'Replace MP3' : 'Attach MP3'}</button>}
              {item?.content.kind === 'slides' && previewAudio && <IconButton icon="x" label="Remove accompaniment" disabled={!!busy} onClick={() => {
                const content = item.content;
                if (content.kind === 'slides') void run('Removing accompaniment…', () => updateItem({ ...item, content: { ...content, audio: null } }));
              }} />}
              {presentation && item && <button disabled={!!busy} onClick={() => setRegenerateTarget(item)}>Regenerate slides</button>}
              {previewAudio && <button disabled={!!busy || (!testing && (!!audio.source || audio.blocked))}
                onClick={() => audio.test(previewAudio)}>{testing ? 'Stop test' : 'Test audio'}</button>}
            </div>}
          </> : <>
            <ol className="queue" aria-label="Audio tracks">
              {tracks.map((entry, index) => {
                const current = audio.source?.kind === 'queue' && audio.source.item === entry.id;
                return <li key={entry.id}>
                  <button className={`queue-item ${entry.id === track?.id ? 'selected' : ''} ${current ? 'active-track' : ''}`}
                    aria-pressed={entry.id === track?.id} aria-current={current ? 'true' : undefined} onClick={() => setSelectedTrack(entry.id)}>
                    <span className="queue-index">{index + 1}</span>
                    <span className="queue-label"><strong title={entry.title}>{entry.title}</strong><small>{current ? audio.playing ? 'Playing' : 'Current track' : entry.content.audio.name}</small></span>
                    {missing?.some(issue => issue.item === entry.id) && <span className="warning-icon" aria-label="Missing asset"><Icon name="warning" size={15} /></span>}
                  </button>
                </li>;
              })}
            </ol>
            {!tracks.length && <div className="queue-empty"><Icon name="music" size={29} /><p>Add MP3 tracks to this service.</p></div>}
            <div className="queue-actions">
              <button disabled={!playlist || !!busy} onClick={() => openFiles({ type: 'audio' })}><Icon name="plus" />Add tracks</button>
            </div>
            <div className="item-actions">
              <IconButton icon="play" label="Play selected track" disabled={!track || audio.blocked} onClick={() => { if (playlist && track) audio.startTrack(playlist.id, track); }} />
              <IconButton icon="up" label="Move track up" disabled={!!busy || trackIndex < 1} onClick={() => {
                if (playlist && track) void run('Saving order…', () => save(withItems(moveTrack(playlist.items, track.id, -1))));
              }} />
              <IconButton icon="down" label="Move track down" disabled={!!busy || trackIndex < 0 || trackIndex >= tracks.length - 1} onClick={() => {
                if (playlist && track) void run('Saving order…', () => save(withItems(moveTrack(playlist.items, track.id, 1))));
              }} />
              <span className="spacer" />
              <IconButton icon="trash" label="Remove track" disabled={!track || !!busy} onClick={() => { if (track) askRemove(track); }} />
            </div>
          </>}
        </aside>

        <section className="projection-column" aria-label="Projection controls">
          <div className="panel control-strip">
            <button className="primary take" disabled={!item || !status.ready || !!normalBusy} onClick={() => take(item, previewIndex)}><Icon name="arrow" />Take</button>
            <div className="output-buttons">
              <button className={status.live && status.black ? 'engaged' : ''} aria-pressed={Boolean(status.live && status.black)} disabled={!status.ready} onClick={() => act('black')}><Icon name="square" />Black</button>
              <button disabled={!status.live || !status.black} onClick={() => act('restore')}><Icon name="eye" />Restore</button>
              <button disabled={!status.live && !status.audio && !status.candidate && !testing} onClick={() => act('clear')}><Icon name="x" />Clear</button>
            </div>
          </div>

          <div className="projection-scroll">
            <div className="view-grid">
              <section className="panel view-panel">
                <div className="panel-heading"><h2><Icon name="eye" />Preview</h2><span className="pill preview-pill">Private</span></div>
                <div className="preview-screen">
                  {preview[previewIndex] ? <SlideView slide={preview[previewIndex]} />
                    : item?.content.kind === 'video' ? <video ref={previewVideo} key={item.content.video.id} className="frame-image" src={assetUrl(item.content.video)}
                      muted preload="metadata" playsInline aria-label="Private video preview"
                      onPlay={() => setPreviewPlaying(true)} onPause={() => setPreviewPlaying(false)} onEnded={() => setPreviewPlaying(false)} />
                    : <div className="screen-empty"><Icon name={item ? 'music' : 'layers'} size={32} /><span>{item ? 'Audio only' : 'Select a playlist item'}</span></div>}
                </div>
                <div className="view-footer">
                  <span className="view-caption" title={item?.title}>{item?.title ?? 'No item selected'}</span>
                  {item?.content.kind === 'video' ? <IconButton icon={previewPlaying ? 'pause' : 'play'}
                    label={previewPlaying ? 'Pause video preview' : 'Play muted video preview'}
                    onClick={() => {
                      const video = previewVideo.current;
                      if (!video) return;
                      if (video.paused) void video.play().catch(() => setProblem('The video preview could not play.'));
                      else video.pause();
                    }} /> : <div className="inline-controls">
                    <IconButton icon="left" label="Previous preview slide" disabled={previewIndex <= 0} onClick={() => setPreviewIndex(index => index - 1)} />
                    <span className="slide-count">{preview.length ? `${previewIndex + 1} / ${preview.length}` : '—'}</span>
                    <IconButton icon="right" label="Next preview slide" disabled={previewIndex >= preview.length - 1} onClick={() => setPreviewIndex(index => index + 1)} />
                  </div>}
                </div>
              </section>

              <section className="panel view-panel">
                <div className="panel-heading"><h2><Icon name="monitor" />Live output</h2><span className={`pill ${readyState === 'Live' ? 'live-pill' : ''}`}><span className="dot" />{readyState}</span></div>
                <div className="preview-screen output-screen">
                  {!status.black && status.live?.slide ? <SlideView slide={status.live.slide} />
                    : <div className="screen-empty"><Icon name={status.live?.kind === 'video' && !status.black ? 'video' : 'monitor'} size={32} />
                      <span>{!status.ready ? 'No confirmed output' : !status.live ? 'Output is clear'
                        : status.black ? 'Output is black' : status.live.kind === 'video' ? 'Video on projector' : 'Audio only'}</span></div>}
                </div>
                <div className="view-footer">
                  <span className="view-caption" title={status.live?.title}>{status.live?.title ?? 'Nothing on air'}</span>
                  <div className="inline-controls">
                    <IconButton icon="left" label="Previous live slide" disabled={!status.live?.count || status.live.index === 0} onClick={() => act('previous')} />
                    <span className="slide-count">{status.live?.count ? `${status.live.index + 1} / ${status.live.count}` : '—'}</span>
                    <IconButton icon="right" label="Next live slide" disabled={!status.live?.count || status.live.index >= status.live.count - 1} onClick={() => act('next')} />
                  </div>
                </div>
              </section>
            </div>

            {!!preview.length && <SlideNavigator slides={preview} previewIndex={previewIndex}
              liveIndex={liveIndex} choose={chooseSlide} />}

            {presentation && <details className="panel presentation-details">
              <summary>PPTX · Converted with {providerLabel[presentation.provider]}</summary>
              {presentation.warnings.map(warning => <p key={warning}>{warning}</p>)}
            </details>}

            {!!failedPresentations.length && <details className="panel presentation-details" open>
              <summary>PowerPoint import results</summary>
              <ul>{failedPresentations.map(({ source, message }) =>
                <li key={source.id}><strong>{source.name}</strong><p>{message}</p></li>)}</ul>
              <button disabled={!!busy} onClick={() => importPresentations(failedPresentations.map(({ source }) => source))}>Retry failed files</button>
            </details>}

            {!!missing?.length && <section ref={issuesPanel} className="panel issues" aria-label="Preflight issues">
              <div className="panel-heading"><h2><Icon name="warning" />Missing assets</h2><span className="count">{missing.length}</span></div>
              <ul>{missing.map(issue => <li key={`${issue.item}-${issue.asset.id}`}>
                <div><strong>{playlist?.items.find(candidate => candidate.id === issue.item)?.title}</strong><small>{issue.asset.name}</small></div>
                <button disabled={!!busy} onClick={() => {
                  const target = playlist?.items.find(entry => entry.id === issue.item);
                  if (target?.content.kind === 'slides' && target.content.presentation && issue.asset.kind === 'png') {
                    setRegenerateTarget(target);
                  } else {
                    openFiles({ type: 'relink', target: issue });
                  }
                }}>{issue.asset.kind === 'png' && playlist?.items.some(entry => entry.id === issue.item
                  && entry.content.kind === 'slides' && entry.content.presentation) ? 'Regenerate' : 'Relink'}</button>
                <button disabled={!!busy} onClick={() => skip(issue.item)}>Skip</button>
              </li>)}</ul>
            </section>}
          </div>
        </section>
      </div>
    </main>

    {audio.transport(playlist?.id, track)}

    <footer className="statusbar">
      <span role={problem && !hasModal ? 'alert' : 'status'} className={problem && !hasModal ? 'problem' : ''}>
        {presentationProgress?.message || (!hasModal && problem) || normalBusy || notice || status.message || (library ? 'Local service workspace' : 'Opening Pelita…')}
      </span>
      {presentationProgress?.total != null && <progress aria-label="PowerPoint slides prepared"
        value={presentationProgress.completed ?? 0} max={presentationProgress.total} />}
      {busy && (stopImport.current || presentationAbort.current) &&
        <button onClick={() => (presentationAbort.current ?? stopImport.current)?.abort()}>Cancel import</button>}
    </footer>

    <input ref={fileInput} type="file" hidden onChange={filesChosen} />
    {editor && <Dialog title={editor.id ? 'Edit playlist item' : 'Add text slides'} close={() => { if (!busy) { setEditor(null); clearFeedback(); } }}
      onSubmit={saveItem} submitLabel={editor.id ? 'Save item' : 'Add slides'} disabled={!!busy || !editor.title.trim()}>
      <label>Item title<input required maxLength={160} value={editor.title} onChange={event => setEditor({ ...editor, title: event.target.value })} /></label>
      {editor.textDeck && <label>Slide text<textarea required rows={9} value={editor.text} onChange={event => setEditor({ ...editor, text: event.target.value })} />
        <small>Put three hyphens (---) on a line by themselves to start a new slide.</small></label>}
      {problem && <p role="alert" className="problem">{problem}</p>}
    </Dialog>}

    {playlistEditor && <Dialog title={playlistEditor.mode === 'rename' ? 'Rename service' : playlistEditor.mode === 'duplicate' ? 'Duplicate service' : 'New service'}
      close={() => { if (!busy) { setPlaylistEditor(null); clearFeedback(); } }} onSubmit={savePlaylist}
      disabled={!!busy || !playlistEditor.title.trim()}>
      <label>Service name<input required maxLength={160} value={playlistEditor.title} onChange={event => setPlaylistEditor({ ...playlistEditor, title: event.target.value })} /></label>
      {problem && <p role="alert" className="problem">{problem}</p>}
    </Dialog>}

    {dialog === 'settings' && <PresentationSettings close={() => setDialog(null)} />}
    {regenerateTarget && <RegeneratePresentation title={regenerateTarget.title}
      close={() => setRegenerateTarget(null)} regenerate={regeneratePresentation} />}

    {dialog === 'projector' && <Dialog title="Projector" close={() => setDialog(null)}>
      <label className="visually-hidden" htmlFor="display">Extended display</label>
      <select id="display" value={displayIndex ?? ''} disabled={!!busy || status.active} onChange={event => setDisplayIndex(event.target.value === '' ? null : Number(event.target.value))}>
        <option value="">{screens.displays.some(candidate => !sameDisplay(candidate, screens.operator)) ? 'Choose a display' : 'No extended display detected'}</option>
        {screens.displays.map((candidate, index) => <option key={index} value={index} disabled={sameDisplay(candidate, screens.operator)}>
          {candidate.name || `Display ${index + 1}`} · {candidate.width} × {candidate.height}{sameDisplay(candidate, screens.operator) ? ' · Operator' : ''}
        </option>)}
      </select>
      <div className="utility-actions">
        <button disabled={!!busy} onClick={() => void run('Refreshing displays…', refreshDisplays)}><Icon name="repeat" />Refresh displays</button>
        {status.active
          ? <button onClick={() => { act('close'); setDialog(null); }}><Icon name="x" />Close output</button>
          : <><button disabled={displayIndex === null || !!busy} onClick={() => confirm('extended')}>Confirm display</button>
            <button disabled={!screens.operator || !!busy} onClick={() => setDialog('shared')}>Single / mirrored</button></>}
      </div>
      {problem && <p className="problem" role="alert">{problem}</p>}
    </Dialog>}

    {dialog === 'shared' && <Dialog title="Single or mirrored presentation" close={() => setDialog(null)}
      onSubmit={() => confirm('shared')} submitLabel="Open in black" disabled={!item}>
      <p>Anyone viewing this desktop can see the operator window before you enter or after you exit presentation. Pelita hides the operator window while presenting.</p>
      <p>Press <kbd>Enter</kbd> to Take the selected item. Use <kbd>Page Up</kbd> and <kbd>Page Down</kbd> to Take adjacent items and <kbd>Esc</kbd> to exit.</p>
      {!item && <p className="problem">Select a playlist item before opening presentation.</p>}
    </Dialog>}

    {dialog === 'shortcuts' && <Dialog title="Keyboard shortcuts" close={() => setDialog(null)}>
      <dl className="shortcuts">
        <div><dt>Take selected item</dt><dd><kbd>Enter</kbd></dd></div>
        <div><dt>Previous or next live slide</dt><dd><kbd>←</kbd> <kbd>→</kbd></dd></div>
        <div><dt>Toggle Black</dt><dd><kbd>B</kbd></dd></div>
        <div><dt>Restore</dt><dd><kbd>R</kbd></dd></div>
        <div><dt>Clear</dt><dd><kbd>C</kbd></dd></div>
        <div><dt>Take adjacent item in single / mirrored mode</dt><dd><kbd>Page Up</kbd> <kbd>Page Down</kbd></dd></div>
        <div><dt>Exit single / mirrored mode</dt><dd><kbd>Esc</kbd></dd></div>
      </dl>
    </Dialog>}

    {dialog === 'remove' && <Dialog title="Remove playlist item?" close={() => setDialog(null)} onSubmit={removeItem}
      submitLabel="Remove item" disabled={!!busy}>
      <p>{removeTarget?.title} will be removed from this service.</p>
      {problem && <p role="alert" className="problem">{problem}</p>}
    </Dialog>}
  </div>;
}

function acceptKind(kind: Kind) {
  return kind === 'jpeg' || kind === 'png' ? '.png,.jpg,.jpeg' : `.${kind}`;
}

createRoot(document.getElementById('root')!).render(<App />);
