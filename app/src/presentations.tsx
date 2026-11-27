import { useEffect, useState, type FormEvent } from 'react';
import { Dialog } from './components';
import { channel, describe, invoke, newId, type Presentation, type Provider, type Slide } from './model';

export interface Progress { message: string; completed: number | null; total: number | null }

interface ProviderStatus {
  id: Provider;
  label: string;
  installation: { path: string | null; version: string } | null;
  error: string | null;
  guidance: string;
}

interface Settings {
  preferences: { provider: Provider | null; libreoffice_path: string | null };
  providers: ProviderStatus[];
}

interface Prepared {
  title: string;
  presentation: Presentation;
  slides: Slide[];
  reused: boolean;
  fallback: boolean;
}

export const providerLabel: Record<Provider, string> = {
  powerpoint: 'Microsoft PowerPoint',
  libreoffice: 'LibreOffice',
};

export async function preparePresentation(
  request: { kind: 'source'; id: string } | { kind: 'regenerate'; item: string; provider: Provider | null },
  update: (progress: Progress) => void,
  signal: AbortSignal,
) {
  signal.throwIfAborted();
  const job = newId();
  const cancel = () => { void invoke('cancel_presentation', { job }).catch(() => undefined); };
  const progress = channel<Progress>(next => {
    if (signal.aborted) cancel();
    else update(next);
  });
  signal.addEventListener('abort', cancel, { once: true });
  try {
    return await invoke<Prepared>('import_presentation', { request, job, progress });
  } catch (error) {
    signal.throwIfAborted();
    if (error && typeof error === 'object' && 'message' in error) throw new Error(String(error.message));
    throw error;
  } finally {
    signal.removeEventListener('abort', cancel);
  }
}

export function PresentationSettings({ close }: { close: () => void }) {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [provider, setProvider] = useState<Provider | ''>('');
  const [busy, setBusy] = useState(true);
  const [error, setError] = useState('');

  async function scan(action?: () => Promise<unknown>) {
    setBusy(true);
    setError('');
    try {
      await action?.();
      const current = await invoke<Settings>('presentation_settings');
      setSettings(current);
      setProvider(current.preferences.provider ?? '');
    } catch (failure) {
      setError(describe(failure));
    } finally {
      setBusy(false);
    }
  }

  useEffect(() => { void scan(); }, []);

  async function save(event: FormEvent<HTMLFormElement>) {
    if (!settings) return;
    event.preventDefault();
    setBusy(true);
    setError('');
    try {
      await invoke('save_presentation_provider', { provider: provider || null });
      close();
    } catch (failure) {
      setError(describe(failure));
      setBusy(false);
    }
  }

  return <Dialog title="Presentation conversion" close={close} onSubmit={save} disabled={busy || !settings}>
    <label>Default provider
      <select value={provider} onChange={event => setProvider(event.target.value as Provider | '')} disabled={busy}>
        <option value="">Automatic (recommended)</option>
        {settings?.providers.map(candidate => <option key={candidate.id} value={candidate.id}>{candidate.label}</option>)}
      </select>
    </label>
    {busy && <p role="status">Checking conversion settings…</p>}
    {settings?.providers.map(candidate => <section className="converter-status" key={candidate.id}>
      <h3>{candidate.label}</h3>
      {candidate.installation
        ? <><p>Available · {candidate.installation.version}</p>{candidate.installation.path && <small>{candidate.installation.path}</small>}</>
        : <><p>Unavailable. {candidate.error}</p><small>{candidate.guidance}</small></>}
    </section>)}
    {settings?.providers.every(candidate => !candidate.installation) &&
      <p>Install one of the converters above to import new PowerPoint files. Saved presentations can still be projected.</p>}
    <p><strong>Custom LibreOffice executable</strong><br />{settings?.preferences.libreoffice_path ?? 'Automatic discovery'}</p>
    <div className="utility-actions">
      <button type="button" disabled={busy} onClick={() => void scan()}>Rescan</button>
      <button type="button" disabled={busy} onClick={() => void scan(() => invoke('choose_libreoffice', { reset: false }))}>Choose executable</button>
      {settings?.preferences.libreoffice_path && <button type="button" disabled={busy}
        onClick={() => void scan(() => invoke('choose_libreoffice', { reset: true }))}>Reset path</button>}
    </div>
    {error && <p role="alert" className="problem">{error}</p>}
  </Dialog>;
}

export function RegeneratePresentation({ title, close, regenerate }: {
  title: string; close: () => void; regenerate: (provider: Provider | null) => void;
}) {
  const [providers, setProviders] = useState<ProviderStatus[]>([]);
  const [provider, setProvider] = useState<Provider | ''>('');
  useEffect(() => {
    void invoke<Settings>('presentation_settings').then(settings => setProviders(settings.providers)).catch(() => undefined);
  }, []);

  return <Dialog title="Regenerate PowerPoint slides" close={close} onSubmit={() => regenerate(provider || null)} submitLabel="Regenerate">
    <p>{title} keeps its saved slides until the replacement is ready.</p>
    <label>Conversion provider
      <select value={provider} onChange={event => setProvider(event.target.value as Provider | '')}>
        <option value="">Use application setting</option>
        {providers.map(candidate => <option key={candidate.id} value={candidate.id}>{candidate.label}</option>)}
      </select>
    </label>
  </Dialog>;
}
