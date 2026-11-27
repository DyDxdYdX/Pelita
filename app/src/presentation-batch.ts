export interface Selection { id: string; name: string }

export async function processPresentationBatch(
  sources: Selection[],
  signal: AbortSignal,
  importFile: (source: Selection) => Promise<{ reused: boolean; fallback: boolean }>,
) {
  const failures: { source: Selection; message: string }[] = [];
  let added = 0;
  let reused = 0;
  let fallbacks = 0;
  for (const source of sources) {
    if (signal.aborted) break;
    try {
      const prepared = await importFile(source);
      added++;
      if (prepared.reused) reused++;
      if (prepared.fallback) fallbacks++;
    } catch (error) {
      if (signal.aborted) break;
      failures.push({ source, message: error instanceof Error ? error.message : String(error) });
    }
  }
  return { added, reused, fallbacks, failures };
}
