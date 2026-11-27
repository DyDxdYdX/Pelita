use super::{
    storage, Capabilities, Control, Conversion, Converter, Error, ErrorKind, Input, Installation,
    Preferences, ProviderId, Result,
};
use std::{
    ffi::OsString,
    fs::{self, File},
    io::{Read, Write},
    os::windows::ffi::OsStrExt,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    time::Duration,
};
use tempfile::TempDir;
use windows::{
    core::{w, BSTR, GUID, PCWSTR},
    Win32::{
        Foundation::{RPC_E_CALL_REJECTED, RPC_E_SERVERCALL_RETRYLATER},
        Storage::FileSystem::GetDriveTypeW,
        System::{
            Com::{
                CLSIDFromProgID, CoCreateInstance, CoInitializeEx, CoUninitialize, IDispatch,
                CLSCTX_LOCAL_SERVER, COINIT_APARTMENTTHREADED, DISPATCH_FLAGS, DISPATCH_METHOD,
                DISPATCH_PROPERTYGET, DISPATCH_PROPERTYPUT, DISPPARAMS,
            },
            Ole::DISPID_PROPERTYPUT,
            Variant::VARIANT,
            WindowsProgramming::DRIVE_REMOTE,
        },
    },
};

static AUTOMATION: Mutex<()> = Mutex::new(());
static SECURITY_RESTORE_FAILED: AtomicBool = AtomicBool::new(false);
const FORCE_DISABLE_MACROS: i32 = 3;
const MSO_TRUE: i32 = -1;
const MSO_FALSE: i32 = 0;

pub struct PowerPoint;

impl Converter for PowerPoint {
    fn id(&self) -> ProviderId {
        ProviderId::PowerPoint
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            slide_progress: true,
            interruptible: false,
            custom_executable: false,
        }
    }

    fn detect(&self, _: &Preferences, _: &Path, control: &Control) -> Result<Installation> {
        worker(&control.limit(Duration::from_secs(10)), |_| {
            let application = application()?;
            let version: BSTR = property(&application, w!("Version"))?;
            let path: BSTR = property(&application, w!("Path"))?;
            let documents: IDispatch = property(&application, w!("Presentations"))?;
            let _: i32 = property(&documents, w!("Count"))?;
            let _: i32 = property(&application, w!("AutomationSecurity"))?;
            Ok(Installation {
                version: version.to_string(),
                path: Some(PathBuf::from(path.to_string()).join("POWERPNT.EXE")),
            })
        })
    }

    fn convert(
        &self,
        input: &Input,
        _: &Installation,
        stage: Arc<TempDir>,
        control: &Control,
    ) -> Result<Conversion> {
        let staged = stage.path().join("import.pptx");
        fs::copy(input.original(), &staged).map_err(Error::storage)?;
        preserve_zone(&input.original(), &staged)?;
        let dimensions = input.format.raster_dimensions();
        let count = input.format.count;
        let marked = input.marked;
        let converted = worker(control, move |control| {
            export(staged, &stage, dimensions, count, marked, control)
        });
        match converted {
            Err(error) if marked && error.kind == ErrorKind::Timeout => {
                Err(protected(error.message))
            }
            other => other,
        }
    }
}

fn protected(error: impl std::fmt::Display) -> Error {
    Error::invalid(format!("PowerPoint could not open or export the security-marked file. Check for an Office security prompt, or import a static PDF. {error}"))
}

fn worker<T: Send + 'static>(
    control: &Control,
    task: impl FnOnce(&Control) -> Result<T> + Send + 'static,
) -> Result<T> {
    control.check()?;
    let (sender, receiver) = mpsc::sync_channel(1);
    let lifetime = control.clone();
    std::thread::Builder::new().name("Pelita PowerPoint STA".into()).spawn(move || {
        let outcome = (|| {
            let _serial = AUTOMATION.try_lock().map_err(|_| Error::provider("PowerPoint is still busy. Close any PowerPoint prompts and retry."))?;
            lifetime.check()?;
            if SECURITY_RESTORE_FAILED.load(Ordering::Relaxed) {
                return Err(Error::provider("PowerPoint could not restore its automation settings. Close PowerPoint after saving your work, then restart Pelita."));
            }
            let _apartment = Apartment::start()?;
            task(&lifetime)
        })();
        let _ = sender.send(outcome);
    }).map_err(Error::storage)?;
    loop {
        control.check()?;
        match receiver.recv_timeout(Duration::from_millis(50)) {
            Ok(result) => {
                control.check()?;
                return result;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(Error::provider(
                    "PowerPoint automation stopped unexpectedly.",
                ))
            }
        }
    }
}

struct Apartment;

impl Apartment {
    fn start() -> Result<Self> {
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }
            .ok()
            .map_err(Error::provider)?;
        Ok(Self)
    }
}

impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}

fn application() -> Result<IDispatch> {
    let error = |error| {
        Error::new(ErrorKind::Unavailable, format!("PowerPoint desktop automation is unavailable. Install or activate Office and check any PowerPoint prompts. {error}"))
    };
    let class = unsafe { CLSIDFromProgID(w!("PowerPoint.Application")) }.map_err(error)?;
    unsafe { CoCreateInstance(&class, None, CLSCTX_LOCAL_SERVER) }.map_err(error)
}

fn export(
    path: PathBuf,
    stage: &TempDir,
    dimensions: (u32, u32),
    count: usize,
    marked: bool,
    control: &Control,
) -> Result<Conversion> {
    let application = application()?;
    let documents: IDispatch = property(&application, w!("Presentations"))?;
    let original = path_argument(&path)?;
    let previous: i32 = property(&application, w!("AutomationSecurity"))?;
    control.check()?;
    if previous != FORCE_DISABLE_MACROS {
        put_security(&application, FORCE_DISABLE_MACROS).map_err(Error::provider)?;
    }
    let open = call(
        &documents,
        w!("Open"),
        DISPATCH_METHOD,
        &mut [
            original,
            MSO_TRUE.into(),
            MSO_FALSE.into(),
            MSO_FALSE.into(),
        ],
    )
    .and_then(|result| IDispatch::try_from(&result))
    .map(Presentation);
    if previous != FORCE_DISABLE_MACROS {
        if let Err(error) = retry_busy(|| put_security(&application, previous)) {
            SECURITY_RESTORE_FAILED.store(true, Ordering::Relaxed);
            if let Ok(presentation) = open {
                let _ = presentation.close();
            }
            return Err(Error::provider(format!("PowerPoint could not restore its automation settings. Save your work, then restart PowerPoint and Pelita. {error}")));
        }
    }
    let presentation = open.map_err(|error| if marked { protected(error) } else { Error::provider(format!("PowerPoint could not open the presentation. Check Office activation or security prompts. {error}")) })?;
    let result = export_slides(&presentation, stage, dimensions, count, control);
    let closed = presentation.close();
    result.and_then(|conversion| closed.map(|()| conversion))
}

struct Presentation(IDispatch);

impl Presentation {
    fn close(self) -> Result<()> {
        // Never call Quit or set Saved. Close only the returned, staged presentation.
        retry_busy(|| call(&self.0, w!("Close"), DISPATCH_METHOD, &mut []))
            .map_err(Error::provider)?;
        Ok(())
    }
}

fn export_slides(
    presentation: &Presentation,
    stage: &TempDir,
    dimensions: (u32, u32),
    count: usize,
    control: &Control,
) -> Result<Conversion> {
    control.check()?;
    let slides: IDispatch = property(&presentation.0, w!("Slides"))?;
    let actual: i32 = property(&slides, w!("Count"))?;
    if actual as usize != count {
        return Err(Error::provider(
            "PowerPoint opened a different number of slides than the validated file.",
        ));
    }
    let (width, height) = dimensions;
    let mut files = Vec::with_capacity(count);
    let mut total_bytes = 0;
    for number in 1..=count {
        control.check()?;
        control.report(
            format!("Exporting slide {number} of {count}…"),
            Some((number - 1, count)),
        );
        let value = call(
            &slides,
            w!("Item"),
            DISPATCH_METHOD | DISPATCH_PROPERTYGET,
            &mut [(number as i32).into()],
        )
        .map_err(Error::provider)?;
        let slide = IDispatch::try_from(&value).map_err(Error::provider)?;
        let path = stage.path().join(format!("{number:04}.png"));
        call(
            &slide,
            w!("Export"),
            DISPATCH_METHOD,
            &mut [
                path_argument(&path)?,
                "PNG".into(),
                (width as i32).into(),
                (height as i32).into(),
            ],
        )
        .map_err(Error::provider)?;
        let size = fs::metadata(&path).map_err(Error::provider)?.len();
        total_bytes += size;
        if !(12..=storage::MAX_SLIDE_BYTES).contains(&size) || total_bytes > 384 * 1024 * 1024 {
            return Err(Error::provider(
                "PowerPoint's exported slides exceed the output limit.",
            ));
        }
        files.push(path);
    }
    Ok(Conversion { dimensions, slides: files, warnings: vec!["Review the slides for substitutions if your deck uses fonts that are unavailable on this computer.".into()] })
}

fn retry_busy<T>(mut action: impl FnMut() -> windows::core::Result<T>) -> windows::core::Result<T> {
    for _ in 0..20 {
        match action() {
            Err(error)
                if matches!(
                    error.code(),
                    RPC_E_CALL_REJECTED | RPC_E_SERVERCALL_RETRYLATER
                ) =>
            {
                std::thread::sleep(Duration::from_millis(100));
            }
            result => return result,
        }
    }
    action()
}

fn path_argument(path: &Path) -> Result<VARIANT> {
    let units = dunce::simplified(path)
        .as_os_str()
        .encode_wide()
        .collect::<Vec<_>>();
    if units.len() >= 260 {
        return Err(Error::provider(
            "PowerPoint cannot use the application data path because it is too long.",
        ));
    }
    Ok(BSTR::from_wide(&units).into())
}

fn property<T>(object: &IDispatch, name: PCWSTR) -> Result<T>
where
    T: for<'a> TryFrom<&'a VARIANT, Error = windows::core::Error>,
{
    let value = call(object, name, DISPATCH_PROPERTYGET, &mut []).map_err(Error::provider)?;
    T::try_from(&value).map_err(Error::provider)
}

fn put_security(object: &IDispatch, value: i32) -> windows::core::Result<VARIANT> {
    call(
        object,
        w!("AutomationSecurity"),
        DISPATCH_PROPERTYPUT,
        &mut [value.into()],
    )
}

fn call(
    object: &IDispatch,
    name: PCWSTR,
    flags: DISPATCH_FLAGS,
    args: &mut [VARIANT],
) -> windows::core::Result<VARIANT> {
    let mut id = 0;
    let interface = GUID::zeroed();
    let mut named = DISPID_PROPERTYPUT;
    args.reverse();
    let mut parameters = DISPPARAMS {
        rgvarg: args.as_mut_ptr(),
        cArgs: args.len() as u32,
        ..Default::default()
    };
    if flags == DISPATCH_PROPERTYPUT {
        parameters.rgdispidNamedArgs = &mut named;
        parameters.cNamedArgs = 1;
    }
    let mut value = VARIANT::default();
    unsafe {
        object.GetIDsOfNames(&interface, &name, 1, 0, &mut id)?;
        object.Invoke(
            id,
            &interface,
            0,
            flags,
            &parameters,
            Some(&mut value),
            None,
            None,
        )?;
    }
    Ok(value)
}

pub fn preserve_zone(source: &Path, destination: &Path) -> Result<bool> {
    fn zone(path: &Path) -> OsString {
        let mut stream = path.as_os_str().to_owned();
        stream.push(":Zone.Identifier");
        stream
    }

    let data = match File::open(zone(source)) {
        Ok(file) => {
            let mut data = Vec::new();
            file.take(64 * 1024 + 1)
                .read_to_end(&mut data)
                .map_err(Error::access)?;
            if data.len() > 64 * 1024 {
                return Err(Error::invalid(
                    "The source has an invalid Windows security marker.",
                ));
            }
            data
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let root = source.ancestors().find(|path| path.parent().is_none());
            let remote = root.is_some_and(|root| {
                let root: Vec<_> = root.as_os_str().encode_wide().chain([0]).collect();
                unsafe { GetDriveTypeW(PCWSTR(root.as_ptr())) == DRIVE_REMOTE }
            });
            if !remote {
                return Ok(false);
            }
            b"[ZoneTransfer]\r\nZoneId=3\r\n".to_vec()
        }
        Err(error) => return Err(Error::access(error)),
    };
    let mut output = File::create(zone(destination)).map_err(Error::storage)?;
    output.write_all(&data).map_err(Error::storage)?;
    output.sync_all().map_err(Error::storage)?;
    Ok(true)
}
