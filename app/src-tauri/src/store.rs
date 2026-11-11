use crate::{describe, operator, Outcome};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
};
use tauri::{ipc::Request, State, WebviewWindow};
use uuid::Uuid;

const MAX_FILE: u64 = 256 * 1024 * 1024;
const MAX_IMPORT: u64 = 512 * 1024 * 1024;
const MAX_CHUNK: usize = 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Png,
    Jpeg,
    Mp3,
    Mp4,
}

impl Kind {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpeg",
            Self::Mp3 => "mp3",
            Self::Mp4 => "mp4",
        }
    }

    pub fn mime(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Mp3 => "audio/mpeg",
            Self::Mp4 => "video/mp4",
        }
    }

    pub fn from_extension(value: &str) -> Option<Self> {
        [Self::Png, Self::Jpeg, Self::Mp3, Self::Mp4]
            .into_iter()
            .find(|kind| kind.extension() == value)
    }

    fn matches_header(self, bytes: &[u8]) -> bool {
        match self {
            Self::Png => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
            Self::Jpeg => bytes.starts_with(b"\xff\xd8\xff"),
            Self::Mp3 => bytes.starts_with(b"ID3") || (bytes[0] == 0xff && bytes[1] & 0xe0 == 0xe0),
            Self::Mp4 => &bytes[4..8] == b"ftyp",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct Asset {
    pub group: Uuid,
    pub id: Uuid,
    pub kind: Kind,
    pub name: String,
    pub size: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Slide {
    Text { text: String },
    Image { asset: Asset },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Content {
    Slides {
        slides: Vec<Slide>,
        audio: Option<Asset>,
    },
    Video {
        video: Asset,
    },
    Audio {
        audio: Asset,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Item {
    pub id: Uuid,
    pub title: String,
    pub content: Content,
}

impl Item {
    pub fn assets(&self) -> Vec<&Asset> {
        match &self.content {
            Content::Slides { slides, audio } => slides
                .iter()
                .filter_map(|slide| match slide {
                    Slide::Image { asset } => Some(asset),
                    Slide::Text { .. } => None,
                })
                .chain(audio)
                .collect(),
            Content::Video { video } => vec![video],
            Content::Audio { audio } => vec![audio],
        }
    }

    fn valid(&self) -> bool {
        let content_valid = match &self.content {
            Content::Slides { slides, audio } => {
                (1..=100).contains(&slides.len())
                    && slides.iter().all(|slide| match slide {
                        Slide::Text { text } => {
                            !text.trim().is_empty() && text.chars().count() <= 360
                        }
                        Slide::Image { asset } => matches!(asset.kind, Kind::Png | Kind::Jpeg),
                    })
                    && audio.as_ref().is_none_or(|asset| asset.kind == Kind::Mp3)
            }
            Content::Video { video } => video.kind == Kind::Mp4,
            Content::Audio { audio } => audio.kind == Kind::Mp3,
        };
        valid_label(&self.title, 160)
            && content_valid
            && self
                .assets()
                .into_iter()
                .all(|asset| valid_label(&asset.name, 300) && (12..=MAX_FILE).contains(&asset.size))
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Playlist {
    id: Uuid,
    title: String,
    pub items: Vec<Item>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Library {
    schema: u32,
    pub revision: u64,
    playlists: Vec<Playlist>,
    selected: Uuid,
}

fn valid_label(value: &str, max: usize) -> bool {
    !value.trim().is_empty() && value.chars().count() <= max
}

impl Library {
    fn empty() -> Self {
        let id = Uuid::new_v4();
        Self {
            schema: 1,
            revision: 0,
            playlists: vec![Playlist {
                id,
                title: "Sabbath Service".into(),
                items: vec![],
            }],
            selected: id,
        }
    }

    fn validate(&self) -> Outcome<()> {
        let mut playlist_ids = HashSet::new();
        let mut item_ids = HashSet::new();
        if self.schema != 1
            || self.playlists.len() > 100
            || !self
                .playlists
                .iter()
                .any(|playlist| playlist.id == self.selected)
            || !self.playlists.iter().all(|playlist| {
                playlist_ids.insert(playlist.id)
                    && valid_label(&playlist.title, 160)
                    && playlist.items.len() <= 500
                    && playlist
                        .items
                        .iter()
                        .all(|item| item_ids.insert(item.id) && item.valid())
            })
        {
            return Err("Invalid service data or service size limit exceeded.".into());
        }
        Ok(())
    }

    pub fn item(&self, id: Uuid) -> Outcome<Item> {
        self.playlists
            .iter()
            .flat_map(|playlist| &playlist.items)
            .find(|item| item.id == id)
            .cloned()
            .ok_or_else(|| "The playlist item no longer exists.".into())
    }
}

pub struct Store {
    pub root: PathBuf,
    pub library: Mutex<Library>,
    upload: Mutex<Option<Upload>>,
    _lock: File,
}

impl Store {
    pub fn open(root: PathBuf) -> Outcome<Self> {
        fs::create_dir_all(&root).map_err(describe)?;
        let root = root.canonicalize().map_err(describe)?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(root.join("profile.lock"))
            .map_err(describe)?;
        lock.try_lock().map_err(|error| format!("Could not lock the Pelita profile. Close the other instance or check the data directory. {error}"))?;
        let library = match fs::read(root.join("services.json")) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(describe)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Library::empty(),
            Err(error) => return Err(describe(error)),
        };
        library.validate()?;
        let staging = root.join("staging");
        if staging.exists() {
            fs::remove_dir_all(&staging).map_err(describe)?;
        }
        fs::create_dir(&staging).map_err(describe)?;
        fs::create_dir_all(root.join("media")).map_err(describe)?;
        Ok(Self {
            root,
            library: Mutex::new(library),
            upload: Mutex::new(None),
            _lock: lock,
        })
    }

    fn save(&self, mut next: Library) -> Outcome<Library> {
        next.validate()?;
        let mut current = self.library.lock().map_err(describe)?;
        if next.revision != current.revision {
            return Err("Service data changed. Reopen the service and try again.".into());
        }
        let existing: HashSet<_> = current
            .playlists
            .iter()
            .flat_map(|playlist| &playlist.items)
            .flat_map(Item::assets)
            .collect();
        for item in next.playlists.iter().flat_map(|playlist| &playlist.items) {
            for asset in item.assets() {
                if !existing.contains(asset) {
                    self.asset(asset)
                        .map_err(|_| format!("{} is unavailable.", asset.name))?;
                }
            }
        }
        next.revision += 1;
        let temporary = self.root.join("services.tmp");
        let mut file = File::create(&temporary).map_err(describe)?;
        serde_json::to_writer(&mut file, &next).map_err(describe)?;
        file.sync_all().map_err(describe)?;
        drop(file);
        fs::rename(temporary, self.root.join("services.json")).map_err(describe)?;
        #[cfg(unix)]
        if let Ok(directory) = File::open(&self.root) {
            let _ = directory.sync_all();
        }
        *current = next.clone();
        Ok(next)
    }

    pub fn asset(&self, asset: &Asset) -> Outcome<PathBuf> {
        let path = media_path(&self.root, asset.group, asset.id, asset.kind)?;
        use std::io::Read;
        let mut file = File::open(&path).map_err(describe)?;
        let mut header = [0; 12];
        if file.metadata().map_err(describe)?.len() != asset.size
            || file.read_exact(&mut header).is_err()
            || !asset.kind.matches_header(&header)
        {
            return Err(format!("{} has changed. Relink this asset.", asset.name));
        }
        Ok(path)
    }
}

pub fn media_path(root: &Path, group: Uuid, id: Uuid, kind: Kind) -> Outcome<PathBuf> {
    let base = root.join("media");
    let path = base.join(group.to_string()).join(filename(id, kind));
    let canonical = path.canonicalize().map_err(describe)?;
    if canonical != path || !canonical.is_file() {
        return Err("Media file unavailable.".into());
    }
    Ok(canonical)
}

fn filename(id: Uuid, kind: Kind) -> String {
    format!("{id}.{}", kind.extension())
}

#[tauri::command]
pub fn library(window: WebviewWindow, store: State<'_, Store>) -> Outcome<Library> {
    operator(&window)?;
    Ok(store.library.lock().map_err(describe)?.clone())
}

#[tauri::command]
pub async fn save_library(
    window: WebviewWindow,
    store: State<'_, Store>,
    next: Library,
) -> Outcome<Library> {
    operator(&window)?;
    store.save(next)
}

#[derive(Serialize)]
pub struct Missing {
    item: Uuid,
    asset: Asset,
}

#[tauri::command]
pub async fn preflight(
    window: WebviewWindow,
    store: State<'_, Store>,
    playlist: Uuid,
) -> Outcome<Vec<Missing>> {
    operator(&window)?;
    let current = store.library.lock().map_err(describe)?;
    let playlist = current
        .playlists
        .iter()
        .find(|candidate| candidate.id == playlist)
        .ok_or("Service no longer exists.")?;
    Ok(playlist
        .items
        .iter()
        .flat_map(|item| {
            item.assets()
                .into_iter()
                .filter(|asset| store.asset(asset).is_err())
                .map(|asset| Missing {
                    item: item.id,
                    asset: asset.clone(),
                })
                .collect::<Vec<_>>()
        })
        .collect())
}

#[derive(Deserialize)]
pub struct Source {
    kind: Kind,
    name: String,
    size: u64,
}

struct Upload {
    group: Uuid,
    files: Vec<Asset>,
    index: usize,
    written: u64,
}

#[derive(Serialize)]
pub struct UploadPlan {
    group: Uuid,
    files: Vec<Asset>,
}

#[tauri::command]
pub async fn begin_import(
    window: WebviewWindow,
    store: State<'_, Store>,
    sources: Vec<Source>,
) -> Outcome<UploadPlan> {
    operator(&window)?;
    if !(1..=100).contains(&sources.len())
        || sources.iter().any(|source| {
            !(12..=MAX_FILE).contains(&source.size) || !valid_label(&source.name, 300)
        })
        || sources.iter().map(|source| source.size).sum::<u64>() > MAX_IMPORT
    {
        return Err("Import exceeds the supported file count or size.".into());
    }
    let mut upload = store.upload.lock().map_err(describe)?;
    if upload.is_some() {
        return Err("Another import is in progress.".into());
    }
    let group = Uuid::new_v4();
    let files: Vec<_> = sources
        .into_iter()
        .map(|source| Asset {
            group,
            id: Uuid::new_v4(),
            kind: source.kind,
            name: source.name,
            size: source.size,
        })
        .collect();
    fs::create_dir(store.root.join("staging").join(group.to_string())).map_err(describe)?;
    *upload = Some(Upload {
        group,
        files: files.clone(),
        index: 0,
        written: 0,
    });
    Ok(UploadPlan { group, files })
}

#[tauri::command]
pub async fn write_import(
    window: WebviewWindow,
    store: State<'_, Store>,
    request: Request<'_>,
) -> Outcome<()> {
    operator(&window)?;
    let group = request
        .headers()
        .get("pelita-import")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| Uuid::parse_str(value).ok())
        .ok_or("Invalid import group.")?;
    let fallback;
    let bytes = match request.body() {
        tauri::ipc::InvokeBody::Raw(bytes) => bytes.as_slice(),
        tauri::ipc::InvokeBody::Json(value) => {
            let values = value
                .as_array()
                .filter(|values| values.len() <= MAX_CHUNK)
                .ok_or("Expected binary import data.")?;
            fallback = values
                .iter()
                .map(|value| {
                    value
                        .as_u64()
                        .and_then(|number| u8::try_from(number).ok())
                        .ok_or("Invalid import byte.")
                })
                .collect::<Result<Vec<_>, _>>()?;
            &fallback
        }
    };
    let mut pending = store.upload.lock().map_err(describe)?;
    let upload = pending
        .as_mut()
        .filter(|upload| upload.group == group)
        .ok_or("Import expired.")?;
    let file = upload
        .files
        .get(upload.index)
        .ok_or("Import is already complete.")?;
    if bytes.is_empty()
        || bytes.len() > MAX_CHUNK
        || upload.written + bytes.len() as u64 > file.size
    {
        return Err("Invalid import chunk size.".into());
    }
    let path = store
        .root
        .join("staging")
        .join(group.to_string())
        .join(filename(file.id, file.kind));
    let mut output = if upload.written == 0 {
        if bytes.len() < 12 || !file.kind.matches_header(bytes) {
            return Err(format!(
                "{} is not a valid {} file.",
                file.name,
                file.kind.extension()
            ));
        }
        File::create_new(path).map_err(describe)?
    } else {
        OpenOptions::new()
            .append(true)
            .open(path)
            .map_err(describe)?
    };
    output.write_all(bytes).map_err(describe)?;
    upload.written += bytes.len() as u64;
    if upload.written == file.size {
        output.sync_all().map_err(describe)?;
        upload.index += 1;
        upload.written = 0;
    }
    Ok(())
}

#[tauri::command]
pub async fn finish_import(
    window: WebviewWindow,
    store: State<'_, Store>,
    group: Uuid,
    commit: bool,
) -> Outcome<()> {
    operator(&window)?;
    let mut pending = store.upload.lock().map_err(describe)?;
    let upload = pending
        .as_ref()
        .filter(|upload| upload.group == group)
        .ok_or("Import expired.")?;
    let stage = store.root.join("staging").join(group.to_string());
    if commit {
        if upload.index != upload.files.len() {
            return Err("Import is incomplete.".into());
        }
        let media = store.root.join("media");
        #[cfg(unix)]
        File::open(&stage)
            .and_then(|directory| directory.sync_all())
            .map_err(describe)?;
        fs::rename(&stage, media.join(group.to_string())).map_err(describe)?;
        #[cfg(unix)]
        if let Ok(directory) = File::open(media) {
            let _ = directory.sync_all();
        }
    } else {
        fs::remove_dir_all(stage).map_err(describe)?;
    }
    *pending = None;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_service_reopens_and_stale_or_invalid_writes_preserve_it() {
        let path = std::env::temp_dir().join(format!("pelita-store-test-{}", Uuid::new_v4()));
        let store = Store::open(path.clone()).unwrap();
        assert!(Store::open(path.clone()).is_err());

        let initial = store.library.lock().unwrap().clone();
        let mut draft = initial.clone();
        let id = Uuid::new_v4();
        draft.playlists[0].items.push(Item {
            id,
            title: "Opening hymn".into(),
            content: Content::Slides {
                slides: vec![Slide::Text {
                    text: "Selamat Sabat".into(),
                }],
                audio: None,
            },
        });
        let saved = store.save(draft).unwrap();
        assert_eq!(saved.revision, 1);
        assert!(store.save(initial).is_err());

        let mut invalid = saved.clone();
        let duplicate = invalid.playlists[0].items[0].clone();
        invalid.playlists[0].items.push(duplicate);
        assert!(store.save(invalid).is_err());
        drop(store);

        let reopened = Store::open(path.clone()).unwrap();
        let current = reopened.library.lock().unwrap();
        assert_eq!(current.revision, 1);
        assert_eq!(current.item(id).unwrap().title, "Opening hymn");
        assert_eq!(current.playlists[0].items.len(), 1);
        drop(current);
        drop(reopened);
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn a_missing_asset_blocks_new_references_and_remains_repairable() {
        let path = std::env::temp_dir().join(format!("pelita-assets-test-{}", Uuid::new_v4()));
        let store = Store::open(path.clone()).unwrap();
        let mut draft = store.library.lock().unwrap().clone();
        let asset = Asset {
            group: Uuid::new_v4(),
            id: Uuid::new_v4(),
            kind: Kind::Mp3,
            name: "accompaniment.mp3".into(),
            size: 12,
        };
        draft.playlists[0].items.push(Item {
            id: Uuid::new_v4(),
            title: "Special music".into(),
            content: Content::Audio {
                audio: asset.clone(),
            },
        });
        assert!(store.save(draft.clone()).is_err());
        let directory = store.root.join("media").join(asset.group.to_string());
        fs::create_dir(&directory).unwrap();
        let media = directory.join(filename(asset.id, asset.kind));
        fs::write(&media, b"ID3\x00\x00\x00\x00\x00\x00\x00\x00\x00").unwrap();
        let mut saved = store.save(draft).unwrap();
        fs::remove_file(media).unwrap();
        assert!(store.asset(&asset).is_err());
        saved.playlists[0].title = "Retitled service".into();
        assert!(store.save(saved).is_ok());
        drop(store);
        fs::remove_dir_all(path).unwrap();
    }
}
