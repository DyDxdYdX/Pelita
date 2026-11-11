import { getDocument, GlobalWorkerOptions } from 'pdfjs-dist';
import workerUrl from 'pdfjs-dist/build/pdf.worker.min.mjs?url';

GlobalWorkerOptions.workerSrc = workerUrl;
const resources = new URL(import.meta.env.DEV ? '/node_modules/pdfjs-dist/' : './pdf/', window.location.href);

export async function rasterizePdf(file: File, progress: (message: string) => void, signal: AbortSignal) {
  const data = new Uint8Array(await file.arrayBuffer());
  signal.throwIfAborted();
  const task = getDocument({
    data,
    stopAtErrors: true,
    cMapUrl: new URL('cmaps/', resources).href,
    cMapPacked: true,
    iccUrl: new URL('iccs/', resources).href,
    standardFontDataUrl: new URL('standard_fonts/', resources).href,
    wasmUrl: new URL('wasm/', resources).href,
    useSystemFonts: true,
    maxImageSize: 32_000_000,
  });
  const abort = () => { void task.destroy().catch(() => undefined); };
  signal.addEventListener('abort', abort, { once: true });
  const pages: { file: Blob; kind: 'png'; name: string }[] = [];
  let preparedBytes = 0;
  try {
    const pdf = await task.promise;
    if (pdf.numPages < 1 || pdf.numPages > 100) throw new Error('A PDF must have 1–100 pages.');
    for (let number = 1; number <= pdf.numPages; number++) {
      signal.throwIfAborted();
      progress(`Rendering page ${number} of ${pdf.numPages} · ${file.name}`);
      const page = await pdf.getPage(number);
      const natural = page.getViewport({ scale: 1 });
      const viewport = page.getViewport({ scale: Math.min(1920 / natural.width, 1080 / natural.height) });
      const canvas = document.createElement('canvas');
      canvas.width = Math.max(1, Math.ceil(viewport.width));
      canvas.height = Math.max(1, Math.ceil(viewport.height));
      try {
        await page.render({ canvas, viewport, background: 'white' }).promise;
        const image = await new Promise<Blob>((resolve, reject) => {
          canvas.toBlob(blob => blob ? resolve(blob) : reject(new Error('Could not encode a PDF page.')), 'image/png');
        });
        preparedBytes += image.size;
        if (preparedBytes > 256 * 1024 * 1024) throw new Error('The rendered PDF exceeds 256 MiB.');
        pages.push({ file: image, kind: 'png', name: `${file.name} · page ${number}` });
      } finally {
        page.cleanup();
        canvas.width = 0;
        canvas.height = 0;
      }
    }
    return pages;
  } finally {
    signal.removeEventListener('abort', abort);
    await task.destroy();
  }
}
