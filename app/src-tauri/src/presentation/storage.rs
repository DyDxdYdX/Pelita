use super::{validation, Control, Conversion, Error, Input, Installation, ProviderId, Result};
use crate::store::{Asset, Kind, Slide};
use image::{ImageFormat, ImageReader, Limits};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};
use tempfile::TempDir;
use uuid::Uuid;

const CACHE_VERSION: u32 = 1;
const MAX_OUTPUT: u64 = 384 * 1024 * 1024;
pub const MAX_SLIDE_BYTES: u64 = 12 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Reference {
    pub id: Uuid,
    format: SourceFormat,
    provider: ProviderId,
    warnings: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum SourceFormat {
    Pptx,
}

#[derive(Deserialize, Serialize)]
pub(super) struct Manifest {
    reference: Reference,
    title: String,
    pub original_filename: String,
    pub source_hash: String,
    slide_count: usize,
    original_emu: (u64, u64),
    dimensions: (u32, u32),
    aspect_ratio: f64,
    provider_version: String,
    conversion: Settings,
    cache_version: u32,
    imported_at: u64,
    slides: Vec<Slide>,
    assets: Vec<StoredAsset>,
}

#[derive(Deserialize, Serialize)]
struct Settings {
    max_dimensions: (u32, u32),
    thumbnail_dimensions: (u32, u32),
    include_hidden_slides: bool,
}

#[derive(Deserialize, Serialize)]
struct StoredAsset {
    asset: Asset,
    sha256: String,
}

#[derive(Serialize)]
pub struct Prepared {
    title: String,
    presentation: Reference,
    slides: Vec<Slide>,
    reused: bool,
    fallback: bool,
}

impl Manifest {
    fn prepared(self, reused: bool, fallback: bool) -> Prepared {
        Prepared {
            title: self.title,
            presentation: self.reference,
            slides: self.slides,
            reused,
            fallback,
        }
    }
}

pub(super) fn directory(root: &Path, id: Uuid) -> PathBuf {
    root.join("presentations").join(id.to_string())
}

pub fn asset_path(root: &Path, group: Uuid, id: Uuid, kind: Kind) -> Option<PathBuf> {
    let folder = match kind {
        Kind::Png => "slides",
        Kind::Webp => "thumbnails",
        _ => return None,
    };
    Some(directory(root, group).join(folder).join(filename(id, kind)))
}

fn filename(id: Uuid, kind: Kind) -> String {
    format!("{id}.{}", kind.extension())
}

pub(super) fn read(directory: &Path) -> std::result::Result<Manifest, String> {
    let path = directory.join("metadata.json");
    if path.canonicalize().map_err(crate::describe)? != path {
        return Err("Managed presentation directory is unavailable.".into());
    }
    let input = File::open(path).map_err(crate::describe)?;
    if input.metadata().map_err(crate::describe)?.len() > 1024 * 1024 {
        return Err("Managed presentation metadata is too large.".into());
    }
    serde_json::from_reader(input).map_err(crate::describe)
}

pub(super) fn stage(
    root: &Path,
    path: &Path,
    original_name: String,
    control: &Control,
) -> Result<Input> {
    if !super::is_pptx(Path::new(&original_name)) {
        return Err(Error::invalid("Choose a file with the .pptx extension."));
    }
    control.check()?;
    let metadata = fs::metadata(path).map_err(Error::access)?;
    if !metadata.is_file() {
        return Err(Error::access("Choose a regular file."));
    }
    if !(22..=validation::MAX_SOURCE).contains(&metadata.len()) {
        return Err(Error::invalid(
            "PowerPoint files must be 128 MiB or smaller and contain a valid presentation.",
        ));
    }
    let stage = TempDir::new_in(root.join("staging")).map_err(Error::storage)?;
    let original = stage.path().join("original.pptx");
    let mut input = File::open(path).map_err(Error::access)?;
    let mut output = File::create(&original).map_err(Error::storage)?;
    let mut buffer = [0; 64 * 1024];
    let mut size = 0;
    let mut hash = Sha256::new();
    loop {
        control.check()?;
        let count = input.read(&mut buffer).map_err(Error::access)?;
        if count == 0 {
            break;
        }
        size += count as u64;
        if size > validation::MAX_SOURCE {
            return Err(Error::invalid(
                "PowerPoint files must be 128 MiB or smaller.",
            ));
        }
        hash.update(&buffer[..count]);
        output.write_all(&buffer[..count]).map_err(Error::storage)?;
    }
    if size != metadata.len() {
        return Err(Error::access(
            "The file changed while it was being copied. Try again.",
        ));
    }
    output.sync_all().map_err(Error::storage)?;
    drop(output);
    #[cfg(windows)]
    let marked = super::powerpoint::preserve_zone(path, &original)?;
    let format = validation::validate(&original, control)?;
    Ok(Input {
        stage,
        original_name,
        hash: format!("{:x}", hash.finalize()),
        format,
        #[cfg(windows)]
        marked,
    })
}

fn hash_file(path: &Path, max: u64, control: &Control) -> Result<String> {
    let mut input = File::open(path).map_err(Error::storage)?;
    let mut size = 0;
    let mut buffer = [0; 64 * 1024];
    let mut hash = Sha256::new();
    loop {
        control.check()?;
        let count = input.read(&mut buffer).map_err(Error::storage)?;
        if count == 0 {
            return Ok(format!("{:x}", hash.finalize()));
        }
        size += count as u64;
        if size > max {
            return Err(Error::storage("Managed file is larger than expected."));
        }
        hash.update(&buffer[..count]);
    }
}

pub(super) fn find(
    root: &Path,
    input: &Input,
    provider: Option<ProviderId>,
    control: &Control,
) -> Result<Option<Prepared>> {
    let mut matches = Vec::new();
    let presentations = root.join("presentations");
    for entry in fs::read_dir(&presentations).map_err(Error::storage)? {
        control.check()?;
        let path = entry.map_err(Error::storage)?.path();
        if let Ok(manifest) = read(&path) {
            if manifest.cache_version == CACHE_VERSION
                && manifest.source_hash == input.hash
                && provider.is_none_or(|id| manifest.reference.provider == id)
                && path == directory(root, manifest.reference.id)
            {
                matches.push(manifest);
            }
        }
    }
    matches.sort_by_key(|manifest| std::cmp::Reverse(manifest.imported_at));
    for manifest in matches {
        control.check()?;
        if reusable(root, input, &manifest, control)? {
            control.report("Reusing saved PowerPoint slides…", None);
            let mut prepared = manifest.prepared(true, false);
            prepared.title = title(&input.original_name);
            return Ok(Some(prepared));
        }
    }
    Ok(None)
}

fn reusable(root: &Path, input: &Input, manifest: &Manifest, control: &Control) -> Result<bool> {
    let group = manifest.reference.id;
    let source = directory(root, group).join("original.pptx");
    let source_valid = source.canonicalize().is_ok_and(|path| path == source)
        && hash_file(&source, validation::MAX_SOURCE, control).is_ok_and(|hash| hash == input.hash);
    control.check()?;
    if !source_valid
        || manifest.slide_count != input.format.count
        || manifest.slides.len() != manifest.slide_count
        || manifest.assets.len() != 2 * manifest.slide_count
        || manifest.original_emu != input.format.emu
    {
        return Ok(false);
    }
    let assets = manifest.slides.iter().flat_map(|slide| match slide {
        Slide::Image {
            asset,
            thumbnail: Some(thumbnail),
        } => [Some(asset), Some(thumbnail)],
        _ => [None, None],
    });
    for (asset, saved) in assets.zip(&manifest.assets) {
        control.check()?;
        let valid = asset.is_some_and(|asset| *asset == saved.asset && asset.group == group)
            && matches_asset(root, saved, control);
        control.check()?;
        if !valid {
            return Ok(false);
        }
    }
    Ok(true)
}

fn matches_asset(root: &Path, saved: &StoredAsset, control: &Control) -> bool {
    let asset = &saved.asset;
    crate::store::media_path(root, asset.group, asset.id, asset.kind).is_ok_and(|path| {
        fs::metadata(&path).is_ok_and(|metadata| metadata.len() == asset.size)
            && hash_file(&path, MAX_SLIDE_BYTES, control).is_ok_and(|hash| hash == saved.sha256)
    })
}

pub fn damaged_slides(root: &Path, reference: &Reference) -> HashSet<Uuid> {
    let Ok(manifest) = read(&directory(root, reference.id)) else {
        return HashSet::new();
    };
    let control = Control::new(Default::default(), Duration::from_secs(120), None);
    manifest
        .assets
        .into_iter()
        .filter(|saved| saved.asset.group == reference.id && saved.asset.kind == Kind::Png)
        .filter(|saved| !matches_asset(root, saved, &control))
        .map(|saved| saved.asset.id)
        .collect()
}

fn title(original_name: &str) -> String {
    let stem = Path::new(original_name)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .trim();
    if stem.is_empty() {
        "PowerPoint presentation".into()
    } else {
        stem.chars().take(160).collect()
    }
}

pub(super) fn prepare(
    root: &Path,
    input: &Input,
    provider: ProviderId,
    installation: &Installation,
    conversion: Conversion,
    control: &Control,
    fallback: bool,
) -> Result<Prepared> {
    let (width, height) = conversion.dimensions;
    let expected = input.format.raster_dimensions();
    if conversion.slides.len() != input.format.count
        || width.abs_diff(expected.0) > 1
        || height.abs_diff(expected.1) > 1
        || width == 0
        || height == 0
    {
        return Err(Error::provider("The converter changed the slide count or aspect ratio. Try another provider or export a static PDF."));
    }
    let group = Uuid::new_v4();
    let output = TempDir::new_in(root.join("staging")).map_err(Error::storage)?;
    let folder = output.path();
    fs::create_dir(folder.join("slides")).map_err(Error::storage)?;
    fs::create_dir(folder.join("thumbnails")).map_err(Error::storage)?;
    let original = folder.join("original.pptx");
    fs::copy(input.original(), &original).map_err(Error::storage)?;
    #[cfg(windows)]
    super::powerpoint::preserve_zone(&input.original(), &original)?;
    let original_file = File::options()
        .write(true)
        .open(&original)
        .map_err(Error::storage)?;
    original_file.sync_all().map_err(Error::storage)?;
    drop(original_file);
    let mut assets = Vec::with_capacity(2 * conversion.slides.len());
    let mut slides = Vec::with_capacity(conversion.slides.len());
    let mut total_bytes = 0;
    for (index, path) in conversion.slides.iter().enumerate() {
        control.check()?;
        control.report(
            format!("Preparing slide {} of {}…", index + 1, input.format.count),
            Some((index, input.format.count)),
        );
        let asset = Asset {
            group,
            id: Uuid::new_v4(),
            kind: Kind::Png,
            name: format!("Slide {}", index + 1),
            size: fs::metadata(path).map_err(Error::provider)?.len(),
        };
        total_bytes += asset.size;
        if !(12..=MAX_SLIDE_BYTES).contains(&asset.size) || total_bytes > MAX_OUTPUT {
            return Err(Error::provider(
                "The converted slide images exceed the output limit.",
            ));
        }
        let mut reader = ImageReader::open(path).map_err(Error::provider)?;
        reader.set_format(ImageFormat::Png);
        let mut limits = Limits::default();
        limits.max_image_width = Some(1921);
        limits.max_image_height = Some(1081);
        limits.max_alloc = Some(32 * 1024 * 1024);
        reader.limits(limits);
        let image = reader.decode().map_err(Error::provider)?;
        if image.width() != width || image.height() != height {
            return Err(Error::provider(
                "The converter produced inconsistent slide dimensions.",
            ));
        }
        let slide_path = folder.join("slides").join(filename(asset.id, asset.kind));
        fs::rename(path, &slide_path).map_err(Error::storage)?;
        let mut thumbnail = Asset {
            id: Uuid::new_v4(),
            kind: Kind::Webp,
            size: 0,
            ..asset.clone()
        };
        let thumb_path = folder
            .join("thumbnails")
            .join(filename(thumbnail.id, thumbnail.kind));
        image
            .thumbnail(240, 135)
            .save_with_format(&thumb_path, ImageFormat::WebP)
            .map_err(Error::storage)?;
        thumbnail.size = fs::metadata(&thumb_path).map_err(Error::storage)?.len();
        for (asset, path) in [(&asset, slide_path), (&thumbnail, thumb_path)] {
            File::options()
                .write(true)
                .open(&path)
                .and_then(|file| file.sync_all())
                .map_err(Error::storage)?;
            assets.push(StoredAsset {
                asset: asset.clone(),
                sha256: hash_file(&path, MAX_SLIDE_BYTES, control)?,
            });
        }
        slides.push(Slide::Image {
            asset,
            thumbnail: Some(thumbnail),
        });
    }
    let mut warnings = conversion.warnings;
    if fallback {
        warnings.insert(
            0,
            format!(
                "PowerPoint conversion failed. Pelita used {} instead.",
                provider.label()
            ),
        );
    }
    let original_emu = input.format.emu;
    let manifest = Manifest {
        reference: Reference {
            id: group,
            format: SourceFormat::Pptx,
            provider,
            warnings,
        },
        title: title(&input.original_name),
        original_filename: input.original_name.clone(),
        source_hash: input.hash.clone(),
        slide_count: input.format.count,
        original_emu,
        dimensions: conversion.dimensions,
        aspect_ratio: original_emu.0 as f64 / original_emu.1 as f64,
        provider_version: installation.version.clone(),
        conversion: Settings {
            max_dimensions: (1920, 1080),
            thumbnail_dimensions: (240, 135),
            include_hidden_slides: true,
        },
        cache_version: CACHE_VERSION,
        imported_at: SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(Error::storage)?
            .as_secs(),
        slides,
        assets,
    };
    let mut metadata = File::create(folder.join("metadata.json")).map_err(Error::storage)?;
    serde_json::to_writer(&mut metadata, &manifest).map_err(Error::storage)?;
    metadata.sync_all().map_err(Error::storage)?;
    drop(metadata);
    #[cfg(unix)]
    for path in [
        folder.join("slides"),
        folder.join("thumbnails"),
        folder.to_owned(),
    ] {
        File::open(path)
            .and_then(|file| file.sync_all())
            .map_err(Error::storage)?;
    }
    control.check()?;
    fs::rename(folder, directory(root, group)).map_err(Error::storage)?;
    #[cfg(unix)]
    if let Ok(parent) = File::open(root.join("presentations")) {
        let _ = parent.sync_all();
    }
    control.report(
        "PowerPoint slides ready.",
        Some((input.format.count, input.format.count)),
    );
    Ok(manifest.prepared(false, fallback))
}
