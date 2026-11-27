use super::*;
use crate::store::{Kind, Slide};
use image::{ImageFormat, Rgb, RgbImage};
use std::{
    io::{Cursor, Read, Write},
    sync::atomic::AtomicUsize,
};
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../spikes/phase-0/fixtures/fidelity.pptx"
);

fn control() -> Control {
    Control::new(Arc::default(), Duration::from_secs(60), None)
}

fn profile() -> (TempDir, Store) {
    let profile = TempDir::new().unwrap();
    let store = Store::open(profile.path().to_owned()).unwrap();
    (profile, store)
}

fn source(root: &Path) -> Input {
    storage::stage(
        root,
        Path::new(FIXTURE),
        "Original slides.pptx".into(),
        &control(),
    )
    .unwrap()
}

fn fixture(
    folder: &Path,
    width: u64,
    height: u64,
    extra: Option<(&str, &[u8])>,
    visuals: bool,
) -> PathBuf {
    let path = folder.join("variant.pptx");
    let mut input = ZipArchive::new(fs::File::open(FIXTURE).unwrap()).unwrap();
    let mut output = ZipWriter::new(fs::File::create(&path).unwrap());
    let options = SimpleFileOptions::default();
    for index in 0..input.len() {
        let mut entry = input.by_index(index).unwrap();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();
        output
            .start_file(entry.name().unwrap().as_ref(), options)
            .unwrap();
        let name = entry.name().unwrap();
        let xml = match name.as_ref() {
            "ppt/presentation.xml" => Some(String::from_utf8(bytes.clone()).unwrap()
                .replace("9144000", &width.to_string()).replace("5143500", &height.to_string())),
            "ppt/slides/slide1.xml" if visuals => Some(String::from_utf8(bytes.clone()).unwrap()
                .replace("</p:spTree>", &format!("{PICTURE}</p:spTree>"))),
            "ppt/slides/slide2.xml" if visuals => Some(String::from_utf8(bytes.clone()).unwrap()
                .replace("<p:sld ", "<p:sld show=\"0\" ")),
            "ppt/slides/_rels/slide1.xml.rels" if visuals => Some(String::from_utf8(bytes.clone()).unwrap()
                .replace("</Relationships>", r#"<Relationship Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/pelita.png" Id="rIdPelitaImage"/></Relationships>"#)),
            "[Content_Types].xml" if visuals => Some(String::from_utf8(bytes.clone()).unwrap()
                .replace("</Types>", r#"<Default Extension="png" ContentType="image/png"/></Types>"#)),
            _ => None,
        };
        output
            .write_all(xml.as_deref().map_or(bytes.as_slice(), str::as_bytes))
            .unwrap();
    }
    if visuals {
        let mut png = Cursor::new(Vec::new());
        RgbImage::from_pixel(16, 16, Rgb([251, 16, 243]))
            .write_to(&mut png, ImageFormat::Png)
            .unwrap();
        output.start_file("ppt/media/pelita.png", options).unwrap();
        output.write_all(&png.into_inner()).unwrap();
    }
    if let Some((name, content)) = extra {
        output.start_file(name, options).unwrap();
        output.write_all(content).unwrap();
    }
    output.finish().unwrap();
    path
}

const PICTURE: &str = r#"<p:pic xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
  xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:nvPicPr><p:cNvPr id="20" name="Pelita test image"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr>
  <p:blipFill><a:blip r:embed="rIdPelitaImage"/><a:stretch><a:fillRect/></a:stretch></p:blipFill>
  <p:spPr><a:xfrm><a:off x="457200" y="457200"/><a:ext cx="457200" cy="457200"/></a:xfrm>
  <a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr>
</p:pic>"#;

#[test]
fn rejects_bad_packages_before_conversion_and_preserves_aspect_ratio() {
    let files = TempDir::new().unwrap();
    let control = control();
    let valid = validation::validate(Path::new(FIXTURE), &control).unwrap();
    assert_eq!(valid.count, 3);
    assert_eq!(valid.raster_dimensions(), (1920, 1080));
    for (emu, pixels) in [
        ((9_144_000, 6_858_000), (1440, 1080)),
        ((9_144_000, 3_810_000), (1920, 800)),
        ((3_810_000, 9_144_000), (450, 1080)),
    ] {
        let changed = fixture(files.path(), emu.0, emu.1, None, false);
        assert_eq!(
            validation::validate(&changed, &control)
                .unwrap()
                .raster_dimensions(),
            pixels
        );
    }

    let broken = files.path().join("broken.pptx");
    fs::write(&broken, "not an archive").unwrap();
    assert_eq!(
        validation::validate(&broken, &control).unwrap_err().kind,
        ErrorKind::InvalidSource
    );
    for extra in [
        ("../escape", "bad"),
        ("ppt/activeX/control.xml", "<active/>"),
        ("ppt/vbaProject.bin", "macro"),
        ("ppt/doctype.xml", "<!DOCTYPE fake><fake/>"),
        (
            "ppt/_rels/link.xml.rels",
            r#"<Relationships><Relationship Type="x/image" TargetMode="External" Target="https://example.com/a"/></Relationships>"#,
        ),
        (
            "ppt/_rels/link.xml.rels",
            r#"<Relationships><Relationship Type="x/image" Target="../../leak"/></Relationships>"#,
        ),
    ] {
        let bad = fixture(
            files.path(),
            9_144_000,
            5_143_500,
            Some((extra.0, extra.1.as_bytes())),
            false,
        );
        assert_eq!(
            validation::validate(&bad, &control).unwrap_err().kind,
            ErrorKind::InvalidSource
        );
    }
    let internal = fixture(
        files.path(),
        9_144_000,
        5_143_500,
        Some((
            "ppt/_rels/link.xml.rels",
            br#"<Relationships><Relationship Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" TargetMode="External" Target="https://example.com/"/></Relationships>"#,
        )),
        false,
    );
    assert!(validation::validate(&internal, &control).is_ok());
}

#[test]
fn validates_nested_chart_data_and_caps_expanded_archive_parts() {
    let files = TempDir::new().unwrap();
    let control = control();
    for external in [false, true] {
        let mut workbook = ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default();
        for (name, xml) in [
            ("[Content_Types].xml", "<Types/>"),
            ("xl/workbook.xml", "<workbook/>"),
        ] {
            workbook.start_file(name, options).unwrap();
            workbook.write_all(xml.as_bytes()).unwrap();
        }
        if external {
            workbook
                .start_file("xl/externalLinks/externalLink1.xml", options)
                .unwrap();
            workbook.write_all(b"<external/>").unwrap();
        }
        let workbook = workbook.finish().unwrap().into_inner();
        let path = fixture(
            files.path(),
            9_144_000,
            5_143_500,
            Some(("ppt/embeddings/chart-data.xlsx", &workbook)),
            false,
        );
        assert_eq!(validation::validate(&path, &control).is_ok(), !external);
    }

    let path = files.path().join("expanded.pptx");
    let mut zip = ZipWriter::new(fs::File::create(&path).unwrap());
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    zip.start_file("oversized.xml", options).unwrap();
    std::io::copy(&mut std::io::repeat(b'a').take(17 * 1024 * 1024), &mut zip).unwrap();
    zip.finish().unwrap();
    let error = validation::validate(&path, &control).unwrap_err();
    assert_eq!(error.kind, ErrorKind::InvalidSource);
    assert!(error.message.contains("expanded size"));
}

struct Mock {
    id: ProviderId,
    detects: AtomicUsize,
    converts: AtomicUsize,
    detection_failure: Option<ErrorKind>,
    conversion_failure: Option<ErrorKind>,
}

impl Mock {
    fn new(id: ProviderId) -> Self {
        Self {
            id,
            detects: AtomicUsize::new(0),
            converts: AtomicUsize::new(0),
            detection_failure: None,
            conversion_failure: None,
        }
    }

    fn conversions(&self) -> usize {
        self.converts.load(Ordering::Relaxed)
    }
}

impl Converter for Mock {
    fn id(&self) -> ProviderId {
        self.id
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            slide_progress: true,
            interruptible: true,
            custom_executable: false,
        }
    }

    fn detect(&self, _: &Preferences, _: &Path, _: &Control) -> Result<Installation> {
        self.detects.fetch_add(1, Ordering::Relaxed);
        if let Some(kind) = self.detection_failure {
            return Err(Error::new(kind, "Mock discovery failure."));
        }
        Ok(Installation {
            version: "mock 1.0".into(),
            path: None,
        })
    }

    fn convert(
        &self,
        source: &Input,
        _: &Installation,
        stage: Arc<TempDir>,
        _: &Control,
    ) -> Result<Conversion> {
        self.converts.fetch_add(1, Ordering::Relaxed);
        if let Some(kind) = self.conversion_failure {
            return Err(Error::new(kind, "Mock conversion failure."));
        }
        let dimensions = source.format.raster_dimensions();
        let slides = (0..source.format.count)
            .map(|index| {
                let path = stage.path().join(format!("{index}.png"));
                RgbImage::from_pixel(dimensions.0, dimensions.1, Rgb(color(index)))
                    .save_with_format(&path, ImageFormat::Png)
                    .unwrap();
                path
            })
            .collect();
        Ok(Conversion {
            dimensions,
            slides,
            warnings: vec![],
        })
    }
}

fn color(index: usize) -> [u8; 3] {
    [32 + index as u8 * 32, 10, 60]
}

fn json(prepared: storage::Prepared) -> serde_json::Value {
    serde_json::to_value(prepared).unwrap()
}

#[test]
fn provider_policy_prefers_operational_provider_and_only_falls_back_for_provider_errors() {
    assert_eq!(
        providers().first().unwrap().id(),
        if cfg!(windows) {
            ProviderId::PowerPoint
        } else {
            ProviderId::LibreOffice
        }
    );
    assert_eq!(providers().last().unwrap().id(), ProviderId::LibreOffice);
    let (_profile, store) = profile();
    let input = source(&store.root);
    let control = control();
    let auto = Preferences::default();
    let mut preferred = Mock::new(ProviderId::PowerPoint);
    let fallback = Mock::new(ProviderId::LibreOffice);
    let prepared = convert(
        &store.root,
        &[&preferred, &fallback],
        &auto,
        &input,
        &control,
    )
    .unwrap();
    assert_eq!(json(prepared)["presentation"]["provider"], "powerpoint");
    assert_eq!(fallback.conversions(), 0);

    preferred.detection_failure = Some(ErrorKind::Unavailable);
    let prepared = convert(
        &store.root,
        &[&preferred, &fallback],
        &auto,
        &input,
        &control,
    )
    .unwrap();
    assert_eq!(json(prepared)["presentation"]["provider"], "libreoffice");
    preferred.detection_failure = None;

    preferred.conversion_failure = Some(ErrorKind::Provider);
    let prepared = json(
        convert(
            &store.root,
            &[&preferred, &fallback],
            &auto,
            &input,
            &control,
        )
        .unwrap(),
    );
    assert_eq!(prepared["fallback"], true);
    assert_eq!(prepared["presentation"]["provider"], "libreoffice");
    assert_eq!(fallback.conversions(), 2);

    let explicit = Preferences {
        provider: Some(ProviderId::PowerPoint),
        ..auto.clone()
    };
    let error = convert(
        &store.root,
        &[&preferred, &fallback],
        &explicit,
        &input,
        &control,
    )
    .err()
    .unwrap();
    assert!(error.message.contains("Select LibreOffice"));
    assert_eq!(fallback.conversions(), 2);

    for kind in [
        ErrorKind::Cancelled,
        ErrorKind::InvalidSource,
        ErrorKind::SourceAccess,
        ErrorKind::Storage,
    ] {
        preferred.conversion_failure = Some(kind);
        assert_eq!(
            convert(
                &store.root,
                &[&preferred, &fallback],
                &auto,
                &input,
                &control
            )
            .err()
            .unwrap()
            .kind,
            kind
        );
        assert_eq!(fallback.conversions(), 2);
    }
    preferred.detection_failure = Some(ErrorKind::Unavailable);
    assert_eq!(
        convert(&store.root, &[&preferred], &auto, &input, &control)
            .err()
            .unwrap()
            .kind,
        ErrorKind::Unavailable,
    );
    let flag = Arc::new(AtomicBool::new(true));
    let cancelled = Control::new(flag.clone(), Duration::from_secs(1), None);
    assert_eq!(
        convert(&store.root, &[&preferred], &auto, &input, &cancelled)
            .err()
            .unwrap()
            .kind,
        ErrorKind::Cancelled
    );
    flag.store(false, Ordering::Relaxed);
    let expired = Control::new(flag, Duration::ZERO, None);
    assert_eq!(expired.check().unwrap_err().kind, ErrorKind::Timeout);
}

#[test]
fn published_slides_are_ordered_reusable_and_survive_reopen_and_regeneration() {
    let (data, store) = profile();
    let input = source(&store.root);
    let control = control();
    let auto = Preferences::default();
    let provider = Mock::new(ProviderId::LibreOffice);
    let prepared = json(convert(&store.root, &[&provider], &auto, &input, &control).unwrap());
    let id: Uuid = serde_json::from_value(prepared["presentation"]["id"].clone()).unwrap();
    let slides: Vec<Slide> = serde_json::from_value(prepared["slides"].clone()).unwrap();
    let mut first = None;
    for (index, slide) in slides.iter().enumerate() {
        let Slide::Image {
            asset,
            thumbnail: Some(thumb),
        } = slide
        else {
            panic!("Expected saved slide and thumbnail.");
        };
        let path = store.asset(asset).unwrap();
        let image = image::open(&path).unwrap().to_rgb8();
        assert_eq!(image.dimensions(), (1920, 1080));
        assert_eq!(image.get_pixel(10, 10).0, color(index));
        let thumbnail = image::open(store.asset(thumb).unwrap()).unwrap();
        assert_eq!((thumbnail.width(), thumbnail.height()), (240, 135));
        if index == 0 {
            first = Some(path);
        }
    }
    let dir = storage::directory(&store.root, id);
    let meta: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.join("metadata.json")).unwrap()).unwrap();
    assert_eq!(meta["source_hash"], input.hash);
    assert_eq!(meta["provider_version"], "mock 1.0");
    assert_eq!(meta["slide_count"], 3);
    assert_eq!(meta["aspect_ratio"], 16.0 / 9.0);
    assert_eq!(
        fs::read(dir.join("original.pptx")).unwrap(),
        fs::read(FIXTURE).unwrap()
    );
    drop(input);
    assert_eq!(fs::read_dir(store.root.join("staging")).unwrap().count(), 0);
    drop(store);

    let reopened = Store::open(data.path().to_owned()).unwrap();
    let input = source(&reopened.root);
    let cached = json(
        storage::find(&reopened.root, &input, None, &control)
            .unwrap()
            .unwrap(),
    );
    assert_eq!(cached["reused"], true);
    assert_eq!(cached["presentation"]["id"], prepared["presentation"]["id"]);
    assert!(storage::find(
        &reopened.root,
        &input,
        Some(ProviderId::PowerPoint),
        &control
    )
    .unwrap()
    .is_none());

    let other = Mock::new(ProviderId::PowerPoint);
    let replacement = json(convert(&reopened.root, &[&other], &auto, &input, &control).unwrap());
    assert_ne!(
        replacement["presentation"]["id"],
        prepared["presentation"]["id"]
    );
    let first = first.unwrap();
    assert!(first.exists());
    let mut damaged = fs::read(&first).unwrap();
    let end = damaged.len() - 1;
    damaged[end] ^= 1;
    fs::write(first, damaged).unwrap();
    let reference = serde_json::from_value(prepared["presentation"].clone()).unwrap();
    assert_eq!(storage::damaged_slides(&reopened.root, &reference).len(), 1);
    assert!(storage::find(
        &reopened.root,
        &input,
        Some(ProviderId::LibreOffice),
        &control
    )
    .unwrap()
    .is_none());
    assert!(storage::find(&reopened.root, &input, None, &control)
        .unwrap()
        .is_some());
    assert!(crate::store::media_path(&reopened.root, id, id, Kind::Png).is_err());
}

#[test]
fn settings_survive_reopen_and_cancel_only_the_active_job() {
    let (_data, store) = profile();
    let mut preferred = Preferences::default();
    assert!(Preferences::load(&store.root).unwrap().provider.is_none());
    preferred.provider = Some(ProviderId::LibreOffice);
    preferred.save(&store.root).unwrap();
    assert_eq!(
        Preferences::load(&store.root).unwrap().provider,
        preferred.provider
    );
    preferred.provider = None;
    preferred.save(&store.root).unwrap();
    assert!(Preferences::load(&store.root).unwrap().provider.is_none());
    let jobs = Jobs::default();
    let (job, _) = jobs.start(Uuid::new_v4()).unwrap();
    assert_eq!(
        jobs.start(Uuid::new_v4()).err().unwrap().kind,
        ErrorKind::Busy
    );
    drop(job);
    assert!(jobs.start(Uuid::new_v4()).is_ok());
}

#[cfg(unix)]
#[test]
fn converter_process_obeys_timeout_and_cancellation() {
    use std::process::Command;
    let scratch = TempDir::new().unwrap();
    let cancel = Arc::new(AtomicBool::new(false));
    for cancelled in [false, true] {
        cancel.store(cancelled, Ordering::Relaxed);
        let control = Control::new(cancel.clone(), Duration::from_millis(250), None);
        let mut command = Command::new("/bin/sleep");
        command.arg("20");
        let start = Instant::now();
        let error = process::run(command, scratch.path(), None, &control).unwrap_err();
        assert_eq!(
            error.kind,
            if cancelled {
                ErrorKind::Cancelled
            } else {
                ErrorKind::Timeout
            }
        );
        assert!(start.elapsed() < Duration::from_secs(2));
    }
}

#[test]
#[ignore = "requires installed LibreOffice with Impress"]
fn libreoffice_smoke() {
    let (_data, store) = profile();
    let control = control();
    let libreoffice = libreoffice::LibreOffice;
    let settings = Preferences::default();
    let scratch = TempDir::new().unwrap();
    let installed = libreoffice
        .detect(&settings, scratch.path(), &control)
        .unwrap();
    eprintln!(
        "LibreOffice smoke test using {} at {:?}",
        installed.version, installed.path
    );
    let path = installed.path.unwrap();
    assert!(libreoffice
        .detect(
            &Preferences {
                provider: None,
                libreoffice_path: Some(path)
            },
            scratch.path(),
            &control
        )
        .is_ok());
    let invalid = Preferences {
        provider: None,
        libreoffice_path: Some(scratch.path().join("invalid")),
    };
    assert_eq!(
        libreoffice
            .detect(&invalid, scratch.path(), &control)
            .err()
            .unwrap()
            .kind,
        ErrorKind::Unavailable
    );

    for (width, height, dims) in [
        (9_144_000, 5_143_500, (1920, 1080)),
        (9_144_000, 6_858_000, (1440, 1080)),
        (9_144_000, 3_810_000, (1920, 800)),
    ] {
        let path = fixture(scratch.path(), width, height, None, true);
        let input =
            storage::stage(&store.root, &path, "Song slides.pptx".into(), &control).unwrap();
        let start = Instant::now();
        let prepared =
            json(convert(&store.root, &[&libreoffice], &settings, &input, &control).unwrap());
        let slides: Vec<Slide> = serde_json::from_value(prepared["slides"].clone()).unwrap();
        assert_eq!(slides.len(), 3);
        for (index, slide) in slides.iter().enumerate() {
            let Slide::Image {
                asset,
                thumbnail: Some(_),
            } = slide
            else {
                panic!("Expected saved slide.");
            };
            let image = image::open(store.asset(asset).unwrap()).unwrap().to_rgb8();
            assert_eq!(image.dimensions(), dims);
            let reference =
                Path::new(FIXTURE).with_file_name(format!("reference-{}.png", index + 1));
            let reference = image::open(reference).unwrap().to_rgb8();
            if index == 0 {
                let x = (dims.0 as f64 * 0.075).round() as u32;
                let expected = [251, 16, 243];
                let image_pixel = image.get_pixel(x, x);
                assert!(
                    image_pixel
                        .0
                        .iter()
                        .zip(expected)
                        .all(|(a, b)| a.abs_diff(b) <= 5),
                    "Embedded image was not rendered: {image_pixel:?}"
                );
            }
            let rendered = image.get_pixel(20, 20);
            let expected = reference.get_pixel(5, 5);
            assert!(
                rendered
                    .0
                    .iter()
                    .zip(expected.0)
                    .all(|(a, b)| a.abs_diff(b) <= 5),
                "Slide {} background mismatch: {rendered:?} != {expected:?}",
                index + 1
            );
        }
        eprintln!(
            "  {}×{}, three slides in {:.2?}",
            dims.0,
            dims.1,
            start.elapsed()
        );
    }
}

#[cfg(windows)]
#[test]
#[ignore = "requires activated Microsoft PowerPoint desktop"]
fn powerpoint_smoke() {
    let (_data, store) = profile();
    let input = source(&store.root);
    let control = control();
    let powerpoint = powerpoint::PowerPoint;
    let preferences = Preferences {
        provider: Some(ProviderId::PowerPoint),
        ..Preferences::default()
    };
    let installed = powerpoint
        .detect(&preferences, input.stage.path(), &control)
        .unwrap();
    eprintln!(
        "PowerPoint smoke test using {} at {:?}",
        installed.version, installed.path
    );
    let prepared =
        json(convert(&store.root, &[&powerpoint], &preferences, &input, &control).unwrap());
    assert_eq!(prepared["presentation"]["provider"], "powerpoint");
    assert_eq!(prepared["slides"].as_array().unwrap().len(), 3);
}
