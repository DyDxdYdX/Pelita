mod libreoffice;
#[cfg(windows)]
mod powerpoint;
mod process;
mod storage;
#[cfg(test)]
mod tests;
mod validation;

use crate::{describe, operator, store::Store, Outcome};
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
pub use storage::{asset_path, damaged_slides, Reference};
use tauri::{ipc::Channel, State, WebviewWindow};
use tempfile::TempDir;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderId {
    PowerPoint,
    LibreOffice,
}

impl ProviderId {
    fn label(self) -> &'static str {
        match self {
            Self::PowerPoint => "Microsoft PowerPoint",
            Self::LibreOffice => "LibreOffice",
        }
    }

    fn guidance(self) -> &'static str {
        match self {
            Self::PowerPoint => {
                "Install or repair the activated PowerPoint desktop app for Windows."
            }
            Self::LibreOffice => {
                "Install LibreOffice with Impress, then rescan or select its soffice executable."
            }
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Preferences {
    // None is Automatic.
    provider: Option<ProviderId>,
    libreoffice_path: Option<PathBuf>,
}

impl Preferences {
    fn update(root: &Path, update: impl FnOnce(&mut Self)) -> Outcome<()> {
        static WRITE: Mutex<()> = Mutex::new(());
        let _write = WRITE.lock().map_err(describe)?;
        let mut preferences = Self::load(root)?;
        update(&mut preferences);
        preferences.save(root)
    }

    fn load(root: &Path) -> Outcome<Self> {
        match fs::read(root.join("presentation-settings.json")) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(describe),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(describe(error)),
        }
    }

    fn save(&self, root: &Path) -> Outcome<()> {
        let mut file = tempfile::NamedTempFile::new_in(root).map_err(describe)?;
        serde_json::to_writer(file.as_file_mut(), self).map_err(describe)?;
        file.as_file().sync_all().map_err(describe)?;
        file.persist(root.join("presentation-settings.json"))
            .map_err(describe)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ErrorKind {
    Cancelled,
    InvalidSource,
    SourceAccess,
    Storage,
    Unavailable,
    Provider,
    Timeout,
    Busy,
}

#[derive(Debug, Serialize)]
pub struct Error {
    kind: ErrorKind,
    message: String,
}

impl Error {
    fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    fn provider(error: impl std::fmt::Display) -> Self {
        Self::new(ErrorKind::Provider, describe(error))
    }

    fn storage(error: impl std::fmt::Display) -> Self {
        Self::new(ErrorKind::Storage, describe(error))
    }

    fn access(error: impl std::fmt::Display) -> Self {
        Self::new(
            ErrorKind::SourceAccess,
            format!("Could not read the presentation. {error}"),
        )
    }

    fn invalid(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::InvalidSource, message)
    }

    fn allows_fallback(&self) -> bool {
        matches!(
            self.kind,
            ErrorKind::Unavailable | ErrorKind::Provider | ErrorKind::Timeout
        )
    }
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Clone)]
struct Control {
    cancelled: Arc<AtomicBool>,
    deadline: Instant,
    progress: Option<Channel<Progress>>,
}

#[derive(Clone, Serialize)]
pub struct Progress {
    message: String,
    completed: Option<usize>,
    total: Option<usize>,
}

impl Control {
    fn new(
        cancelled: Arc<AtomicBool>,
        timeout: Duration,
        progress: Option<Channel<Progress>>,
    ) -> Self {
        Self {
            cancelled,
            deadline: Instant::now() + timeout,
            progress,
        }
    }

    fn limit(&self, timeout: Duration) -> Self {
        Self {
            deadline: self.deadline.min(Instant::now() + timeout),
            ..self.clone()
        }
    }

    fn check(&self) -> Result<()> {
        if self.cancelled.load(Ordering::Relaxed) {
            return Err(Error::new(
                ErrorKind::Cancelled,
                "Presentation import cancelled.",
            ));
        }
        if Instant::now() >= self.deadline {
            return Err(Error::new(
                ErrorKind::Timeout,
                "Presentation conversion timed out.",
            ));
        }
        Ok(())
    }

    fn report(&self, message: impl Into<String>, count: Option<(usize, usize)>) {
        if let Some(channel) = &self.progress {
            let _ = channel.send(Progress {
                message: message.into(),
                completed: count.map(|(done, _)| done),
                total: count.map(|(_, total)| total),
            });
        }
    }
}

#[derive(Clone, Serialize)]
struct Installation {
    version: String,
    path: Option<PathBuf>,
}

#[derive(Clone, Copy, Serialize)]
struct Capabilities {
    slide_progress: bool,
    interruptible: bool,
    custom_executable: bool,
}

trait Converter: Sync {
    fn id(&self) -> ProviderId;
    fn capabilities(&self) -> Capabilities;
    fn detect(
        &self,
        preferences: &Preferences,
        stage: &Path,
        control: &Control,
    ) -> Result<Installation>;
    fn convert(
        &self,
        input: &Input,
        installation: &Installation,
        stage: Arc<TempDir>,
        control: &Control,
    ) -> Result<Conversion>;
}

struct Input {
    stage: TempDir,
    original_name: String,
    hash: String,
    format: validation::Format,
    #[cfg(windows)]
    marked: bool,
}

impl Input {
    fn original(&self) -> PathBuf {
        self.stage.path().join("original.pptx")
    }
}

struct Conversion {
    dimensions: (u32, u32),
    slides: Vec<PathBuf>,
    warnings: Vec<String>,
}

fn providers() -> &'static [&'static dyn Converter] {
    &[
        #[cfg(windows)]
        &powerpoint::PowerPoint,
        &libreoffice::LibreOffice,
    ]
}

#[derive(Serialize)]
pub struct ProviderStatus {
    id: ProviderId,
    label: &'static str,
    capabilities: Capabilities,
    installation: Option<Installation>,
    error: Option<String>,
    guidance: &'static str,
}

#[derive(Serialize)]
pub struct Settings {
    preferences: Preferences,
    providers: Vec<ProviderStatus>,
}

#[derive(Clone, Serialize)]
pub struct Selection {
    id: Uuid,
    name: String,
}

#[derive(Default)]
struct Sources {
    paths: VecDeque<(Uuid, PathBuf)>,
    drops: Option<Channel<Vec<Selection>>>,
}

impl Sources {
    fn remember(&mut self, paths: Vec<PathBuf>) -> Outcome<Vec<Selection>> {
        if paths.len() > 100 {
            return Err("Select up to 100 PowerPoint files at a time.".into());
        }
        let mut selections = Vec::with_capacity(paths.len());
        for path in paths {
            let Some(name) = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
            else {
                continue;
            };
            let id = Uuid::new_v4();
            self.paths.push_back((id, path));
            selections.push(Selection { id, name });
        }
        self.paths.drain(..self.paths.len().saturating_sub(300));
        Ok(selections)
    }
}

#[derive(Default)]
pub struct Jobs {
    sources: Mutex<Sources>,
    running: Mutex<Option<(Uuid, Arc<AtomicBool>)>>,
}

struct Job<'a>(&'a Jobs);

impl Drop for Job<'_> {
    fn drop(&mut self) {
        if let Ok(mut running) = self.0.running.lock() {
            *running = None;
        }
    }
}

impl Jobs {
    fn start(&self, id: Uuid) -> Result<(Job<'_>, Arc<AtomicBool>)> {
        let mut running = self.running.lock().map_err(Error::storage)?;
        if running.is_some() {
            return Err(Error::new(
                ErrorKind::Busy,
                "Another presentation is being prepared.",
            ));
        }
        let cancelled = Arc::new(AtomicBool::new(false));
        *running = Some((id, cancelled.clone()));
        Ok((Job(self), cancelled))
    }

    pub fn cancel_active(&self) -> bool {
        let running = self.running.lock().ok();
        if let Some((_, cancelled)) = running.as_deref().and_then(Option::as_ref) {
            cancelled.store(true, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    pub fn drop_files(&self, paths: &[PathBuf]) {
        if let Ok(mut sources) = self.sources.lock() {
            if let Some(channel) = sources.drops.clone() {
                let presentations: Vec<_> =
                    paths.iter().filter(|path| is_pptx(path)).cloned().collect();
                if let Ok(selections) = sources.remember(presentations) {
                    if !selections.is_empty() {
                        let _ = channel.send(selections);
                    }
                }
            }
        }
    }
}

fn is_pptx(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("pptx"))
}

#[tauri::command]
pub async fn pick_presentations(
    window: WebviewWindow,
    jobs: State<'_, Jobs>,
) -> Outcome<Vec<Selection>> {
    operator(&window)?;
    let files = rfd::AsyncFileDialog::new()
        .set_parent(&window)
        .add_filter("PowerPoint presentations", &["pptx"])
        .pick_files()
        .await;
    jobs.sources.lock().map_err(describe)?.remember(
        files
            .unwrap_or_default()
            .into_iter()
            .map(|file| file.path().to_owned())
            .collect(),
    )
}

#[tauri::command]
pub fn presentation_drops(
    window: WebviewWindow,
    jobs: State<'_, Jobs>,
    drops: Channel<Vec<Selection>>,
) -> Outcome<()> {
    operator(&window)?;
    jobs.sources.lock().map_err(describe)?.drops = Some(drops);
    Ok(())
}

#[tauri::command]
pub fn cancel_presentation(window: WebviewWindow, jobs: State<'_, Jobs>, job: Uuid) -> Outcome<()> {
    operator(&window)?;
    if let Some((_, cancelled)) = jobs
        .running
        .lock()
        .map_err(describe)?
        .as_ref()
        .filter(|(id, _)| *id == job)
    {
        cancelled.store(true, Ordering::Relaxed);
    }
    Ok(())
}

#[tauri::command]
pub async fn presentation_settings(
    window: WebviewWindow,
    store: State<'_, Store>,
) -> Outcome<Settings> {
    operator(&window)?;
    let root = store.root.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let preferences = Preferences::load(&root)?;
        let scratch = TempDir::new_in(root.join("staging")).map_err(describe)?;
        let control = Control::new(Arc::default(), Duration::from_secs(30), None);
        let providers = providers()
            .iter()
            .map(|provider| {
                let (installation, error) =
                    match provider.detect(&preferences, scratch.path(), &control) {
                        Ok(installation) => (Some(installation), None),
                        Err(error) => (None, Some(error.message)),
                    };
                let id = provider.id();
                ProviderStatus {
                    id,
                    label: id.label(),
                    capabilities: provider.capabilities(),
                    installation,
                    error,
                    guidance: id.guidance(),
                }
            })
            .collect();
        Ok(Settings {
            preferences,
            providers,
        })
    })
    .await
    .map_err(describe)?
}

#[tauri::command]
pub async fn save_presentation_provider(
    window: WebviewWindow,
    store: State<'_, Store>,
    provider: Option<ProviderId>,
) -> Outcome<()> {
    operator(&window)?;
    if provider.is_some_and(|id| !providers().iter().any(|candidate| candidate.id() == id)) {
        return Err("That conversion provider is unavailable on this platform.".into());
    }
    Preferences::update(&store.root, |preferences| preferences.provider = provider)
}

#[tauri::command]
pub async fn choose_libreoffice(
    window: WebviewWindow,
    store: State<'_, Store>,
    reset: bool,
) -> Outcome<()> {
    operator(&window)?;
    let path = if reset {
        None
    } else {
        let Some(file) = rfd::AsyncFileDialog::new()
            .set_parent(&window)
            .set_title("Choose the LibreOffice soffice executable")
            .pick_file()
            .await
        else {
            return Ok(());
        };
        let path = libreoffice::validate_executable(file.path()).map_err(|error| error.message)?;
        let staging = store.root.join("staging");
        Some(
            tauri::async_runtime::spawn_blocking(move || {
                let scratch = TempDir::new_in(staging).map_err(describe)?;
                let control = Control::new(Arc::default(), Duration::from_secs(8), None);
                libreoffice::version(&path, scratch.path(), &control)
                    .map_err(|error| error.message)?;
                Ok::<_, String>(path)
            })
            .await
            .map_err(describe)??,
        )
    };
    Preferences::update(&store.root, |preferences| {
        preferences.libreoffice_path = path
    })
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum PresentationRequest {
    Source {
        id: Uuid,
    },
    Regenerate {
        item: Uuid,
        provider: Option<ProviderId>,
    },
}

#[tauri::command]
pub async fn import_presentation(
    window: WebviewWindow,
    store: State<'_, Store>,
    jobs: State<'_, Jobs>,
    job: Uuid,
    request: PresentationRequest,
    progress: Channel<Progress>,
) -> Result<storage::Prepared> {
    operator(&window).map_err(Error::access)?;
    let (_job, cancelled) = jobs.start(job)?;
    let (path, name, expected_hash, provider) = match request {
        PresentationRequest::Source { id } => {
            let sources = jobs.sources.lock().map_err(Error::storage)?;
            let path = sources
                .paths
                .iter()
                .find(|(candidate, _)| *candidate == id)
                .map(|(_, path)| path.clone())
                .ok_or_else(|| Error::access("Select the file again."))?;
            let name = path
                .file_name()
                .ok_or_else(|| Error::access("Missing filename."))?
                .to_string_lossy()
                .into_owned();
            (path, name, None, None)
        }
        PresentationRequest::Regenerate { item: id, provider } => {
            let item = store
                .library
                .lock()
                .map_err(Error::storage)?
                .item(id)
                .map_err(Error::access)?;
            let reference = item
                .presentation()
                .ok_or_else(|| Error::access("Select an imported PowerPoint item."))?;
            let directory = storage::directory(&store.root, reference.id);
            let manifest = storage::read(&directory).map_err(Error::access)?;
            let original = directory.join("original.pptx");
            if original.canonicalize().map_err(Error::access)? != original {
                return Err(Error::access("The managed original is unavailable."));
            }
            (
                original,
                manifest.original_filename,
                Some(manifest.source_hash),
                provider,
            )
        }
    };
    let root = store.root.clone();
    let control = Control::new(cancelled, Duration::from_secs(300), Some(progress));
    control.report("Checking PowerPoint file…", None);
    tauri::async_runtime::spawn_blocking(move || {
        let mut preferences = Preferences::load(&root).map_err(Error::storage)?;
        if provider.is_some() {
            preferences.provider = provider;
        }
        let input = storage::stage(&root, &path, name, &control)?;
        if let Some(expected) = &expected_hash {
            if *expected != input.hash {
                return Err(Error::invalid(
                    "The managed original has changed. Import the original PowerPoint file again.",
                ));
            }
        } else if let Some(prepared) = storage::find(&root, &input, preferences.provider, &control)?
        {
            return Ok(prepared);
        }
        convert(&root, providers(), &preferences, &input, &control)
    })
    .await
    .map_err(Error::storage)?
}

fn convert(
    root: &Path,
    providers: &[&dyn Converter],
    preferences: &Preferences,
    input: &Input,
    control: &Control,
) -> Result<storage::Prepared> {
    let mut failures = Vec::new();
    let mut fallback = false;
    for provider in providers
        .iter()
        .filter(|provider| preferences.provider.is_none_or(|id| id == provider.id()))
    {
        control.check()?;
        let id = provider.id();
        let stage = Arc::new(TempDir::new_in(root.join("staging")).map_err(Error::storage)?);
        control.report(format!("Checking {}…", id.label()), None);
        let installation = match provider.detect(preferences, stage.path(), control) {
            Ok(installation) => installation,
            Err(error) if error.allows_fallback() => {
                fallback |= error.kind != ErrorKind::Unavailable;
                failures.push((id, error));
                continue;
            }
            Err(error) => return Err(error),
        };
        let attempt = control.limit(Duration::from_secs(120));
        control.report(format!("Converting with {}…", id.label()), None);
        match provider
            .convert(input, &installation, stage.clone(), &attempt)
            .and_then(|result| {
                storage::prepare(root, input, id, &installation, result, &attempt, fallback)
            }) {
            Ok(prepared) => return Ok(prepared),
            Err(error) if error.allows_fallback() => {
                failures.push((id, error));
                fallback = true;
            }
            Err(error) => return Err(error),
        }
    }
    control.check()?;
    if let Some(selected) = preferences.provider {
        if let Some((_, mut error)) = failures.pop() {
            if let Some(alternative) = providers.iter().find(|candidate| {
                candidate.id() != selected
                    && candidate
                        .detect(preferences, input.stage.path(), control)
                        .is_ok()
            }) {
                error.message.push_str(&format!(
                    " Select {} in Settings and retry.",
                    alternative.id().label()
                ));
            } else {
                error.message.push_str(&format!(" {}", selected.guidance()));
            }
            control.check()?;
            return Err(error);
        }
        return Err(Error::new(
            ErrorKind::Unavailable,
            "The selected provider is unsupported on this platform.",
        ));
    }
    let kind = failures
        .iter()
        .find(|(_, error)| error.kind != ErrorKind::Unavailable)
        .map_or(ErrorKind::Unavailable, |(_, error)| error.kind);
    let detail = failures
        .into_iter()
        .map(|(id, error)| format!("{}: {}", id.label(), error.message))
        .collect::<Vec<_>>()
        .join(" ");
    Err(Error::new(kind, format!("{detail} Install an available converter, or import a static PDF exported from PowerPoint.")))
}
