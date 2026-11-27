use super::{Control, Error, Result};
use std::{
    fs::{self, File},
    io::Read,
    path::Path,
    process::{Child, Command, Stdio},
    time::Duration,
};

pub fn run(
    mut command: Command,
    scratch: &Path,
    output_file: Option<(&Path, u64)>,
    control: &Control,
) -> Result<String> {
    control.check()?;
    let log_path = scratch.join("converter.log");
    let log = File::create(&log_path).map_err(Error::storage)?;
    command
        .current_dir(scratch)
        .stdin(Stdio::null())
        .stdout(log.try_clone().map_err(Error::storage)?)
        .stderr(log);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
        // Limit a private converter process and the children that inherit its limits.
        unsafe { command.pre_exec(limit_resources) };
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        use windows::Win32::System::Threading::CREATE_NO_WINDOW;
        command.creation_flags(CREATE_NO_WINDOW.0);
    }
    let mut child = command.spawn().map_err(Error::provider)?;
    #[cfg(windows)]
    let _job = match windows_job::assign(&child) {
        Ok(job) => job,
        Err(error) => {
            stop(&mut child);
            return Err(Error::provider(error));
        }
    };
    let status = loop {
        let result = control.check().and_then(|()| {
            if let Some((path, limit)) = output_file {
                match fs::metadata(path) {
                    Ok(metadata) if metadata.len() > limit => {
                        return Err(Error::provider(
                            "The intermediate PDF exceeds the size limit.",
                        ))
                    }
                    Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                        return Err(Error::storage(error))
                    }
                    _ => {}
                }
            }
            child.try_wait().map_err(Error::provider)
        });
        match result {
            Ok(Some(status)) => break status,
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(error) => {
                stop(&mut child);
                return Err(error);
            }
        }
    };
    #[cfg(unix)]
    terminate_group(&mut child);
    let mut bytes = Vec::new();
    File::open(log_path)
        .map_err(Error::storage)?
        .take(4096)
        .read_to_end(&mut bytes)
        .map_err(Error::storage)?;
    let log = String::from_utf8_lossy(&bytes).trim().to_owned();
    if !status.success() {
        return Err(Error::provider(format!(
            "The converter exited with {status}. {log}"
        )));
    }
    Ok(log)
}

fn stop(child: &mut Child) {
    #[cfg(unix)]
    terminate_group(child);
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(unix)]
fn terminate_group(child: &mut Child) {
    // The child was started in its own process group.
    unsafe { libc::kill(-(child.id() as libc::pid_t), libc::SIGKILL) };
}

#[cfg(unix)]
fn limit_resources() -> std::io::Result<()> {
    for (resource, limit) in [
        (libc::RLIMIT_FSIZE, 192 * 1024 * 1024),
        (libc::RLIMIT_CPU, 120),
    ] {
        let bounds = libc::rlimit {
            rlim_cur: limit,
            rlim_max: limit,
        };
        if unsafe { libc::setrlimit(resource, &bounds) } != 0 {
            return Err(std::io::Error::last_os_error());
        }
    }
    #[cfg(target_os = "linux")]
    {
        let limit = 3 * 1024 * 1024 * 1024;
        let bounds = libc::rlimit {
            rlim_cur: limit,
            rlim_max: limit,
        };
        if unsafe { libc::setrlimit(libc::RLIMIT_AS, &bounds) } != 0 {
            return Err(std::io::Error::last_os_error());
        }
    }
    Ok(())
}

#[cfg(windows)]
mod windows_job {
    use std::{mem::size_of, os::windows::io::AsRawHandle, process::Child};
    use windows::{
        core::PCWSTR,
        Win32::{
            Foundation::{CloseHandle, HANDLE},
            System::JobObjects::{
                AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
                SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
                JOB_OBJECT_LIMIT_JOB_MEMORY, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            },
        },
    };

    pub struct Job(HANDLE);

    impl Drop for Job {
        fn drop(&mut self) {
            let _ = unsafe { CloseHandle(self.0) };
        }
    }

    pub fn assign(child: &Child) -> windows::core::Result<Job> {
        unsafe {
            let job = Job(CreateJobObjectW(None, PCWSTR::null())?);
            let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            limits.BasicLimitInformation.LimitFlags =
                JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_JOB_MEMORY;
            limits.JobMemoryLimit = 3 * 1024 * 1024 * 1024;
            SetInformationJobObject(
                job.0,
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )?;
            AssignProcessToJobObject(job.0, HANDLE(child.as_raw_handle()))?;
            Ok(job)
        }
    }
}
