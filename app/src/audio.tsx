import { useEffect, useRef, useState } from 'react';
import { Icon, IconButton } from './components';
import { adjacentTrack, audioTracks, type Repeat, type Track } from './audio-queue';
import { assetUrl, type Asset, type Library, type Status } from './model';

type Source =
  | { kind: 'queue'; service: string; item: string; asset: Asset; title: string }
  | { kind: 'take' | 'test'; asset: Asset };

function initialVolume() {
  const saved = localStorage.getItem('pelita-volume');
  return saved !== null && Number.isFinite(Number(saved)) ? Math.max(0, Math.min(1, Number(saved))) : 0.8;
}

export function useAudio(library: Library | null, status: Status) {
  const element = useRef<HTMLAudioElement>(null);
  const loaded = useRef<Source | null>(null);
  const seenTake = useRef<string | null>(null);
  const request = useRef(0);
  const [source, setSource] = useState<Source | null>(null);
  const [playing, setPlaying] = useState(false);
  const [time, setTime] = useState(0);
  const [duration, setDuration] = useState(0);
  const [problem, setProblem] = useState('');
  const [repeat, setRepeat] = useState<Repeat>('off');
  const [volume, setVolume] = useState(initialVolume);
  const [videoCut, setVideoCut] = useState(0);
  const blocked = !status.audio_allowed || status.revision < videoCut;
  const queue = source?.kind === 'queue'
    ? audioTracks(library?.playlists.find(service => service.id === source.service)) : [];
  const previous = source?.kind === 'queue' ? adjacentTrack(queue, source.item, -1, repeat) : undefined;
  const next = source?.kind === 'queue' ? adjacentTrack(queue, source.item, 1, repeat) : undefined;

  function release() {
    const node = element.current!;
    request.current++;
    loaded.current = null;
    node.pause();
    node.removeAttribute('src');
    node.load();
    setSource(null);
    setPlaying(false);
    setTime(0);
    setDuration(0);
    setProblem('');
  }

  function play() {
    const attempt = ++request.current;
    void element.current!.play().catch(error => {
      if (attempt === request.current && !(error instanceof DOMException && error.name === 'AbortError')) {
        setPlaying(false);
        setProblem(`Could not play ${loaded.current?.asset.name ?? 'audio'}. Relink the file or try another track.`);
      }
    });
  }

  function start(nextSource: Source) {
    release();
    loaded.current = nextSource;
    setSource(nextSource);
    element.current!.src = assetUrl(nextSource.asset);
    play();
  }

  function startTrack(service: string, track: Track) {
    if (!blocked) start({
      kind: 'queue', service, item: track.id, title: track.title, asset: track.content.audio,
    });
  }

  function stopProjectionAudio() {
    if (loaded.current && loaded.current.kind !== 'queue') release();
  }

  function pauseForVideo() {
    const node = element.current!;
    const current = loaded.current;
    const resume = current && !node.paused;
    setVideoCut(Infinity);
    node.pause();
    return (revision = 0) => {
      setVideoCut(revision);
      if (!revision && resume && loaded.current === current) play();
    };
  }

  useEffect(() => {
    const node = element.current!;
    node.volume = volume;
    localStorage.setItem('pelita-volume', String(volume));
  }, [volume]);

  useEffect(() => {
    const current = loaded.current;
    if (current?.kind === 'queue' && !queue.some(track => track.id === current.item)) release();
  }, [library, source]);

  useEffect(() => {
    if (blocked) element.current!.pause();
    const taken = status.audio;
    if (!taken) {
      seenTake.current = null;
      if (loaded.current?.kind === 'take') release();
      return;
    }
    if (blocked || seenTake.current === taken.token || !library) return;
    seenTake.current = taken.token;
    for (const service of library.playlists) {
      const track = service.items.find((item): item is Track => item.id === taken.item && item.content.kind === 'audio');
      if (track) {
        startTrack(service.id, track);
        return;
      }
    }
    start({ kind: 'take', asset: taken.asset });
  }, [library, status.audio, blocked]);

  function toggle(service?: string, track?: Track) {
    const node = element.current!;
    if (!node.paused) {
      node.pause();
      return;
    }
    if (blocked) return;
    if (!source && service && track) startTrack(service, track);
    else if (source) {
      setProblem('');
      const updated = source.kind === 'queue' ? queue.find(track => track.id === source.item) : undefined;
      if (source.kind === 'queue' && updated && (node.error || updated.content.audio.id !== source.asset.id)) {
        startTrack(source.service, updated);
      } else if (node.error) {
        start(source);
      } else {
        if (node.ended) node.currentTime = 0;
        play();
      }
    }
  }

  return {
    source, playing, blocked, startTrack, stopProjectionAudio, pauseForVideo,
    test(asset: Asset) {
      if (loaded.current?.kind === 'test') release();
      else if (!loaded.current && !blocked) start({ kind: 'test', asset });
    },
    transport(service?: string, track?: Track) {
      return <section className="audio-transport" aria-label="Audio transport">
        <audio ref={element} preload="auto" hidden
          onPlaying={event => setPlaying(!event.currentTarget.paused)}
          onPause={event => setPlaying(!event.currentTarget.paused)}
          onTimeUpdate={event => setTime(event.currentTarget.currentTime)}
          onDurationChange={event => {
            const value = event.currentTarget.duration;
            setDuration(Number.isFinite(value) ? value : 0);
          }}
          onEnded={() => {
            const current = loaded.current;
            setPlaying(false);
            if (current?.kind !== 'queue' || blocked) return;
            const following = adjacentTrack(queue, current.item, 1, repeat, true);
            if (following) startTrack(current.service, following);
          }}
          onError={event => {
            if (!event.currentTarget.error || !loaded.current) return;
            setPlaying(false);
            setProblem(`Could not play ${loaded.current.asset.name}. Relink the file or try another track.`);
          }} />
        <div className="audio-caption">
          <strong title={source?.asset.name}>{source?.kind === 'queue' ? source.title : source?.asset.name ?? track?.title ?? 'Audio'}</strong>
          <small className={problem ? 'problem' : ''} role={problem ? 'alert' : undefined} title={problem}>
            {problem || (blocked ? 'Video has audio priority' : source ? `${formatTime(time)} / ${formatTime(duration)}` : 'No track playing')}
          </small>
        </div>
        <div className="audio-buttons">
          <IconButton icon="left" label="Previous track" disabled={blocked || !previous} onClick={() => {
            if (source?.kind === 'queue' && previous) startTrack(source.service, previous);
          }} />
          <IconButton icon={playing ? 'pause' : 'play'} label={playing ? 'Pause audio' : 'Play audio'}
            disabled={!playing && (blocked || (!source && !track))} onClick={() => toggle(service, track)} />
          <IconButton icon="right" label="Next track" disabled={blocked || !next} onClick={() => {
            if (source?.kind === 'queue' && next) startTrack(source.service, next);
          }} />
          <IconButton icon="repeat" label="Restart audio" disabled={!duration || blocked} onClick={() => {
            element.current!.currentTime = 0;
            play();
          }} />
          <IconButton icon="square" label="Stop audio" disabled={!source} onClick={release} />
        </div>
        <input className="audio-seek" aria-label="Seek audio" type="range" min="0" max={duration || 1} step="0.01"
          value={Math.min(time, duration)} disabled={!duration}
          onChange={event => {
            const value = Number(event.target.value);
            element.current!.currentTime = value;
            setTime(value);
          }} />
        <label className="volume"><Icon name="volume" /><span className="visually-hidden">Audio volume</span>
          <input type="range" min="0" max="1" step="0.01" value={volume} onChange={event => setVolume(Number(event.target.value))} />
        </label>
        <select aria-label="Audio repeat" value={repeat} onChange={event => setRepeat(event.target.value as Repeat)}>
          <option value="off">Repeat off</option><option value="all">Repeat all</option><option value="one">Repeat one</option>
        </select>
      </section>;
    },
  };
}

function formatTime(time: number) {
  const whole = Number.isFinite(time) ? Math.floor(time) : 0;
  return `${Math.floor(whole / 60)}:${String(whole % 60).padStart(2, '0')}`;
}
