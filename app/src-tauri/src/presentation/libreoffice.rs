use super::{
    process, storage, Capabilities, Control, Conversion, Converter, Error, ErrorKind, Input,
    Installation, Preferences, ProviderId, Result,
};
use hayro::{
    hayro_interpret::{util::TransformExt, InterpreterSettings},
    hayro_syntax::Pdf,
    kurbo::Affine,
    vello_cpu::{
        color::palette::css::WHITE, Pixmap, RasterizerSettings, RenderContext, Resources,
        TargetInit,
    },
    RenderCache, RenderSettings,
};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
    time::Duration,
};
use tempfile::TempDir;

const MAX_PDF: u64 = 64 * 1024 * 1024;
const MAX_OUTPUT: u64 = 384 * 1024 * 1024;
const PROFILE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<oor:items xmlns:oor="http://openoffice.org/2001/registry">
  <item oor:path="/org.openoffice.Office.Common/Security/Scripting">
    <prop oor:name="MacroSecurityLevel" oor:op="fuse"><value>3</value></prop>
    <prop oor:name="DisableMacrosExecution" oor:op="fuse"><value>true</value></prop>
    <prop oor:name="DisableActiveContent" oor:op="fuse"><value>true</value></prop>
    <prop oor:name="BlockUntrustedRefererLinks" oor:op="fuse"><value>true</value></prop>
  </item>
  <item oor:path="/org.openoffice.Office.Common/Filter/PDF/Export">
    <prop oor:name="ExportHiddenSlides" oor:op="fuse"><value>true</value></prop>
  </item>
</oor:items>"#;

pub struct LibreOffice;

impl Converter for LibreOffice {
    fn id(&self) -> ProviderId {
        ProviderId::LibreOffice
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            slide_progress: true,
            interruptible: true,
            custom_executable: true,
        }
    }

    fn detect(
        &self,
        preferences: &Preferences,
        stage: &Path,
        control: &Control,
    ) -> Result<Installation> {
        if let Some(path) = &preferences.libreoffice_path {
            let path = validate_executable(path)?;
            let version = version(&path, stage, control)?;
            return Ok(Installation {
                version,
                path: Some(path),
            });
        }
        let mut last_error = None;
        let mut checked = std::collections::HashSet::new();
        for candidate in candidates() {
            control.check()?;
            let Ok(path) = validate_executable(&candidate) else {
                continue;
            };
            if !checked.insert(path.clone()) {
                continue;
            }
            match version(&path, stage, control) {
                Ok(version) => {
                    return Ok(Installation {
                        version,
                        path: Some(path),
                    })
                }
                Err(error) if error.allows_fallback() => last_error = Some(error),
                Err(error) => return Err(error),
            }
        }
        Err(last_error.unwrap_or_else(|| Error::new(ErrorKind::Unavailable, "LibreOffice was not found. Install LibreOffice with Impress or choose its executable in Settings.")))
    }

    fn convert(
        &self,
        input: &Input,
        installation: &Installation,
        stage: Arc<TempDir>,
        control: &Control,
    ) -> Result<Conversion> {
        let executable = installation
            .path
            .as_ref()
            .ok_or_else(|| Error::provider("Missing LibreOffice executable."))?;
        let scratch = stage.path();
        let mut command = isolated_command(executable, scratch)?;
        let pdf = scratch.join("original.pdf");
        command
            .args(["--convert-to", "pdf:impress_pdf_Export", "--outdir"])
            .arg(dunce::simplified(scratch))
            .arg(dunce::simplified(&input.original()));
        let log = process::run(command, scratch, Some((&pdf, MAX_PDF)), control)?;
        let size = fs::metadata(&pdf)
            .map_err(|_| {
                Error::provider(format!(
                    "LibreOffice could not export a PDF. Ensure Impress is installed. {log}"
                ))
            })?
            .len();
        if !(12..=MAX_PDF).contains(&size) {
            return Err(Error::provider(
                "LibreOffice produced an empty or oversized PDF.",
            ));
        }
        control.check()?;
        // Hayro handles a local static export. Bound pages and raster dimensions before rendering.
        rasterize(
            fs::read(pdf).map_err(Error::storage)?,
            input,
            scratch,
            control,
        )
    }
}

pub fn validate_executable(path: &Path) -> Result<PathBuf> {
    let invalid = || {
        Error::new(
            ErrorKind::Unavailable,
            "Choose an installed LibreOffice soffice executable.",
        )
    };
    if !path.is_absolute() {
        return Err(invalid());
    }
    #[cfg(windows)]
    let allowed = ["soffice.com", "soffice.exe", "libreoffice.exe"];
    #[cfg(not(windows))]
    let allowed = ["soffice", "libreoffice"];
    let valid_name = |path: &Path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| {
                allowed
                    .iter()
                    .any(|allowed| name.eq_ignore_ascii_case(allowed))
            })
    };
    if !valid_name(path) {
        return Err(invalid());
    }
    let path = dunce::canonicalize(path).map_err(|_| invalid())?;
    if !valid_name(&path) || !path.is_file() {
        return Err(invalid());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if fs::metadata(&path)
            .map_err(|_| invalid())?
            .permissions()
            .mode()
            & 0o111
            == 0
        {
            return Err(invalid());
        }
    }
    #[cfg(windows)]
    if path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
    {
        let console = path.with_file_name("soffice.com");
        if console.is_file() {
            return Ok(console);
        }
    }
    Ok(path)
}

fn candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(path) = env::var_os("PATH") {
        for directory in env::split_paths(&path).filter(|path| path.is_absolute()) {
            #[cfg(windows)]
            paths.extend(["soffice.com", "soffice.exe"].map(|name| directory.join(name)));
            #[cfg(not(windows))]
            paths.extend(["soffice", "libreoffice"].map(|name| directory.join(name)));
        }
    }
    #[cfg(windows)]
    for variable in [
        "ProgramFiles",
        "ProgramFiles(x86)",
        "ProgramW6432",
        "LOCALAPPDATA",
    ] {
        if let Some(root) = env::var_os(variable) {
            let program = PathBuf::from(root).join("LibreOffice").join("program");
            paths.extend(["soffice.com", "soffice.exe"].map(|name| program.join(name)));
        }
    }
    #[cfg(target_os = "macos")]
    {
        paths.push(PathBuf::from(
            "/Applications/LibreOffice.app/Contents/MacOS/soffice",
        ));
        if let Some(home) = env::var_os("HOME") {
            paths.push(
                PathBuf::from(home).join("Applications/LibreOffice.app/Contents/MacOS/soffice"),
            );
        }
    }
    #[cfg(target_os = "linux")]
    paths.extend(
        [
            "/usr/bin/soffice",
            "/usr/local/bin/soffice",
            "/usr/lib/libreoffice/program/soffice",
            "/opt/libreoffice/program/soffice",
        ]
        .map(PathBuf::from),
    );
    paths
}

fn isolated_command(executable: &Path, scratch: &Path) -> Result<Command> {
    let profile = scratch.join("profile");
    let config = profile.join("user");
    fs::create_dir_all(&config).map_err(Error::storage)?;
    fs::write(config.join("registrymodifications.xcu"), PROFILE).map_err(Error::storage)?;
    let profile = url::Url::from_directory_path(dunce::simplified(&profile))
        .map_err(|_| Error::storage("Invalid LibreOffice temporary profile path."))?;
    let mut command = Command::new(executable);
    command
        .arg(format!("-env:UserInstallation={profile}"))
        .args(["--headless", "--nologo", "--nodefault", "--norestore"]);
    Ok(command)
}

pub(super) fn version(path: &Path, stage: &Path, control: &Control) -> Result<String> {
    let scratch = TempDir::new_in(stage).map_err(Error::storage)?;
    let mut command = isolated_command(path, scratch.path())?;
    command.arg("--version");
    let output = process::run(
        command,
        scratch.path(),
        None,
        &control.limit(Duration::from_secs(8)),
    )?;
    output.lines().find(|line| line.starts_with("LibreOffice ") && line.chars().any(|c| c.is_ascii_digit()))
        .map(|line| line.chars().take(200).collect())
        .ok_or_else(|| Error::new(ErrorKind::Unavailable, "The executable did not identify itself as LibreOffice. Try soffice.com on Windows."))
}

fn rasterize(
    bytes: Vec<u8>,
    input: &Input,
    scratch: &Path,
    control: &Control,
) -> Result<Conversion> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        rasterize_pdf(bytes, input, scratch, control)
    }))
    .unwrap_or_else(|_| {
        Err(Error::provider(
            "The PDF renderer could not process this LibreOffice export.",
        ))
    })
}

fn rasterize_pdf(
    bytes: Vec<u8>,
    input: &Input,
    scratch: &Path,
    control: &Control,
) -> Result<Conversion> {
    let pdf = Pdf::new(bytes).map_err(|error| {
        Error::provider(format!("LibreOffice produced an unreadable PDF. {error:?}"))
    })?;
    if pdf.pages().len() != input.format.count {
        return Err(Error::provider("LibreOffice did not export all slides. Verify the deck in Impress or try another provider."));
    }
    let dimensions = input.format.raster_dimensions();
    let (width, height) = dimensions;
    let expected_ratio = input.format.emu.0 as f64 / input.format.emu.1 as f64;
    let cache = RenderCache::new();
    let interpreter = InterpreterSettings::default();
    let settings = RenderSettings::default();
    let mut resources = Resources::default();
    let mut slides = Vec::new();
    let mut size = 0;
    for page in pdf.pages().iter() {
        control.check()?;
        let (page_width, page_height) = page.render_dimensions();
        let (page_width, page_height) = (f64::from(page_width), f64::from(page_height));
        let scale = (f64::from(width) / page_width).min(f64::from(height) / page_height);
        if !scale.is_finite()
            || scale <= 0.0
            || (page_width / page_height / expected_ratio - 1.0).abs() > 0.015
        {
            return Err(Error::provider(
                "LibreOffice changed the slide aspect ratio.",
            ));
        }
        control.report(
            format!(
                "Rendering slide {} of {}…",
                slides.len() + 1,
                input.format.count
            ),
            Some((slides.len(), input.format.count)),
        );
        let mut context = RenderContext::new(width as u16, height as u16);
        let mut pixmap = Pixmap::new(width as u16, height as u16);
        let offset = (
            (f64::from(width) - page_width * scale) / 2.0,
            (f64::from(height) - page_height * scale) / 2.0,
        );
        let transform = Affine::translate(offset)
            * Affine::scale(scale)
            * page.initial_transform(true).to_kurbo();
        hayro::render_into(
            page,
            &cache,
            &interpreter,
            &settings,
            &mut context,
            transform,
        );
        context.flush();
        context.render_with(
            &mut pixmap,
            &mut resources,
            RasterizerSettings {
                target_init: TargetInit::Clear(WHITE),
                ..Default::default()
            },
        );
        control.check()?;
        let png = pixmap.into_png().map_err(Error::provider)?;
        size += png.len() as u64;
        if png.len() as u64 > storage::MAX_SLIDE_BYTES || size > MAX_OUTPUT {
            return Err(Error::provider(
                "The rendered presentation exceeds the output limit.",
            ));
        }
        let output = scratch.join(format!("{:04}.png", slides.len() + 1));
        fs::write(&output, png).map_err(Error::storage)?;
        slides.push(output);
    }
    Ok(Conversion { dimensions, slides, warnings: vec!["Review the slides for font substitution and changes to effects in the LibreOffice export.".into()] })
}
