import { invoke, type Asset, type Kind } from './model';

const maxFile = 256 * 1024 * 1024;
const maxImage = 24 * 1024 * 1024;
const maxImport = 512 * 1024 * 1024;
const chunkSize = 1024 * 1024;

type Source = { file: Blob; kind: Kind; name: string };
type Progress = (message: string) => void;

export async function decodeMedia(kind: 'mp3' | 'mp4', url: string, signal?: AbortSignal) {
  signal?.throwIfAborted();
  const element = document.createElement(kind === 'mp4' ? 'video' : 'audio');
  try {
    element.preload = 'auto';
    element.muted = true;
    await new Promise<void>((resolve, reject) => {
      const finish = (error?: unknown) => {
        clearTimeout(timeout);
        signal?.removeEventListener('abort', abort);
        if (error) reject(error);
        else resolve();
      };
      const abort = () => finish(signal?.reason);
      const timeout = window.setTimeout(() => finish(new Error('Media preparation timed out.')), 10000);
      element.onloadeddata = () => {
        if (!Number.isFinite(element.duration) || element.duration <= 0
          || (element instanceof HTMLVideoElement && !element.videoWidth)) {
          finish(new Error('This file has no playable media.'));
        } else {
          finish();
        }
      };
      element.onerror = () => finish(new Error('The webview could not decode this file.'));
      signal?.addEventListener('abort', abort, { once: true });
      element.src = url;
    });
  } finally {
    element.onloadeddata = null;
    element.onerror = null;
    element.removeAttribute('src');
    element.load();
  }
}

async function rasterize(file: File, progress: Progress, signal: AbortSignal): Promise<Source[]> {
  if (file.size > 64 * 1024 * 1024) throw new Error('PDF files must be 64 MiB or smaller.');
  const { rasterizePdf } = await import('./pdf');
  return rasterizePdf(file, progress, signal);
}

async function inspect(file: File, progress: Progress, signal: AbortSignal, only?: Kind): Promise<Source[]> {
  const suffix = file.name.split('.').pop()?.toLowerCase();
  if (file.size > maxFile) throw new Error(`${file.name} exceeds the 256 MiB file limit.`);
  if (suffix === 'pdf' && !only) return rasterize(file, progress, signal);
  const kind = suffix === 'jpg' ? 'jpeg' : suffix;
  if (kind !== 'png' && kind !== 'jpeg' && kind !== 'mp3' && kind !== 'mp4') {
    throw new Error('Choose PNG, JPEG, PDF, MP3 or MP4 files. Export PowerPoint as PDF first.');
  }
  if (only && only !== kind && !(only === 'jpeg' && kind === 'png') && !(only === 'png' && kind === 'jpeg')) {
    throw new Error(`Choose a replacement of the same media type as ${only.toUpperCase()}.`);
  }
  if (kind === 'png' || kind === 'jpeg') {
    if (file.size > maxImage) throw new Error('Each image must be 24 MiB or smaller.');
    const bitmap = await createImageBitmap(file);
    try {
      if (bitmap.width * bitmap.height > 32_000_000) throw new Error('Each image must be 32 megapixels or smaller.');
    } finally {
      bitmap.close();
    }
  } else {
    const url = URL.createObjectURL(file);
    try {
      await decodeMedia(kind, url, signal);
    } finally {
      URL.revokeObjectURL(url);
    }
  }
  return [{ file, kind, name: file.name }];
}

export async function importFiles(files: File[], progress: Progress, signal: AbortSignal, only?: Kind) {
  if (!files.length || files.length > 100 || files.reduce((total, file) => total + file.size, 0) > maxImport) {
    throw new Error('Select up to 100 files totaling 512 MiB or less.');
  }
  const sources: Source[] = [];
  for (const file of files) {
    signal.throwIfAborted();
    progress(`Checking ${file.name}…`);
    sources.push(...await inspect(file, progress, signal, only));
    if (sources.length > 100 || sources.reduce((sum, source) => sum + source.file.size, 0) > maxImport) {
      throw new Error('An import can contain up to 100 prepared files totaling 512 MiB.');
    }
  }
  signal.throwIfAborted();
  const plan = await invoke<{ group: string; files: Asset[] }>('begin_import', {
    sources: sources.map(({ file, name, kind }) => ({ name, kind, size: file.size })),
  });
  try {
    for (const [index, { file, name }] of sources.entries()) {
      progress(`Saving ${index + 1} of ${sources.length} · ${name}`);
      for (let start = 0; start < file.size; start += chunkSize) {
        signal.throwIfAborted();
        const chunk = await file.slice(start, start + chunkSize).arrayBuffer();
        await invoke('write_import', chunk, { headers: { 'pelita-import': plan.group } });
      }
    }
    signal.throwIfAborted();
    await invoke('finish_import', { group: plan.group, commit: true });
    return plan.files;
  } catch (error) {
    await invoke('finish_import', { group: plan.group, commit: false }).catch(() => undefined);
    throw error;
  }
}
