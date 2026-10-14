//! Runs the install command the user typed, and waits until it and everything it started is done.
//!
//! The command runs in a Windows job object, so processes it starts (an installer that hands
//! over to a second setup.exe, or exits while its updater keeps working) are waited for too.

/// How the command ended. `exit_code` is the command's own, if it exited; `timed_out` means
/// it or something it started was still running when the time ran out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunOutcome {
    pub exit_code: Option<i32>,
    pub timed_out: bool,
}

/// One command line from separate arguments, quoted so `CommandLineToArgvW` (what nearly every
/// Windows program uses) splits it back into the same arguments.
pub fn quote_args(args: &[String]) -> String {
    args.iter().map(|a| quote_arg(a)).collect::<Vec<_>>().join(" ")
}

fn quote_arg(arg: &str) -> String {
    let quoted = arg.is_empty() || arg.contains([' ', '\t', '\n']);
    let mut out = String::with_capacity(arg.len() + 2);
    if quoted {
        out.push('"');
    }
    let mut backslashes = 0;
    for c in arg.chars() {
        match c {
            '\\' => backslashes += 1,
            '"' => {
                // Backslashes before a quote are escapes, so double them, then escape the quote.
                out.push_str(&"\\".repeat(backslashes * 2 + 1));
                out.push('"');
                backslashes = 0;
            }
            _ => {
                out.push_str(&"\\".repeat(backslashes));
                out.push(c);
                backslashes = 0;
            }
        }
    }
    // Before the closing quote, trailing backslashes would escape it.
    out.push_str(&"\\".repeat(if quoted { backslashes * 2 } else { backslashes }));
    if quoted {
        out.push('"');
    }
    out
}

/// idiff's exit code for a finished run: `0` when the command succeeded, including installers'
/// "restart required" codes 3010 and 1641; `2` when it failed or ran out of time.
pub fn exit_status(outcome: &RunOutcome) -> u8 {
    match outcome {
        RunOutcome {
            timed_out: false,
            exit_code: Some(0 | 3010 | 1641),
        } => 0,
        _ => 2,
    }
}

/// How long no installer may run before an install or uninstall counts as done.
pub const INSTALLERS_QUIET: std::time::Duration = std::time::Duration::from_secs(10);

/// Whether a process is an installer or uninstaller at work: by its file name, or because it runs
/// from a temp folder, where uninstallers copy themselves (NSIS's `Un_A.exe`) so they can delete
/// their own folder. `msiexec.exe` in session 0 is the Windows Installer service and its custom
/// action hosts, which linger for minutes after any install; `wait_for_msi_idle` covers their work.
pub fn is_installer(name: &str, image: Option<&str>, service_session: bool, temp_dirs: &[String]) -> bool {
    let from_temp = image.is_some_and(|image| {
        let image = image.to_lowercase();
        temp_dirs
            .iter()
            .any(|temp| image.starts_with(&format!("{}\\", temp.trim_end_matches('\\').to_lowercase())))
    });
    let name = name.to_lowercase();
    let Some(stem) = name.strip_suffix(".exe") else {
        return from_temp;
    };
    from_temp
        || (stem == "msiexec" && !service_session)
        || stem.starts_with("setup")
        || (stem.contains("install") && stem != "trustedinstaller")
        || stem.starts_with("unins")
        || stem == "au_"
        || (stem.starts_with("un_") && stem.chars().count() == 4)
        || stem == "update"
}

/// The temp folders installers run from, as `var` gives them: the user's (%TEMP%, %TMP%) and
/// SYSTEM's (`%SystemRoot%\Temp`), where installers started by a service, such as updaters, run.
pub fn temp_dir_candidates(var: impl Fn(&str) -> Option<String>) -> Vec<String> {
    let mut dirs: Vec<String> = ["TEMP", "TMP"].iter().filter_map(|name| var(name)).collect();
    dirs.extend(var("SystemRoot").map(|root| format!(r"{}\Temp", root.trim_end_matches('\\'))));
    dirs.dedup();
    dirs
}

#[cfg(windows)]
pub use windows_impl::{run_in_job, wait_for_installers, wait_for_msi_idle};

#[cfg(windows)]
mod windows_impl {
    use super::{RunOutcome, is_installer, quote_args, temp_dir_candidates};
    use idiff_core::copy;
    use std::io;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
    use windows::Win32::Foundation::{CloseHandle, ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND, HANDLE, WAIT_OBJECT_0};
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
    };
    use windows::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOBOBJECT_BASIC_ACCOUNTING_INFORMATION,
        JobObjectBasicAccountingInformation, QueryInformationJobObject,
    };
    use windows::Win32::System::RemoteDesktop::ProcessIdToSessionId;
    use windows::Win32::System::Threading::{
        CREATE_SUSPENDED, CreateProcessW, GetExitCodeProcess, GetProcessTimes, OpenMutexW, OpenProcess,
        PROCESS_INFORMATION, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
        ResumeThread, STARTUPINFOW, SYNCHRONIZATION_SYNCHRONIZE, WaitForSingleObject,
    };
    use windows::core::{PCWSTR, PWSTR, w};

    const POLL: Duration = Duration::from_millis(500);

    /// Closes the handle when dropped.
    struct Owned(HANDLE);

    impl Drop for Owned {
        fn drop(&mut self) {
            if !self.0.is_invalid() {
                let _ = unsafe { CloseHandle(self.0) };
            }
        }
    }

    /// Starts `args` suspended inside a new job, then waits for the process and for every process
    /// in the job, at most `timeout` in all. Processes still running then are left running.
    pub fn run_in_job(args: &[String], timeout: Duration) -> io::Result<RunOutcome> {
        if args.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, copy::COMMAND_NOT_FOUND));
        }
        let deadline = Instant::now() + timeout;
        let mut command_line: Vec<u16> = quote_args(args).encode_utf16().chain([0]).collect();
        let startup = STARTUPINFOW {
            cb: size_of::<STARTUPINFOW>() as u32,
            ..Default::default()
        };
        let mut info = PROCESS_INFORMATION::default();
        unsafe {
            let job = Owned(CreateJobObjectW(None, PCWSTR::null())?);
            let started = CreateProcessW(
                PCWSTR::null(),
                Some(PWSTR(command_line.as_mut_ptr())),
                None,
                None,
                false,
                CREATE_SUSPENDED,
                None,
                PCWSTR::null(),
                &startup,
                &mut info,
            );
            if let Err(e) = started {
                let missing =
                    e.code() == ERROR_FILE_NOT_FOUND.to_hresult() || e.code() == ERROR_PATH_NOT_FOUND.to_hresult();
                return Err(if missing {
                    io::Error::new(io::ErrorKind::NotFound, copy::COMMAND_NOT_FOUND)
                } else {
                    e.into()
                });
            }
            let (process, thread) = (Owned(info.hProcess), Owned(info.hThread));
            // Without the job only the process itself can be waited for; still better than not running it.
            let in_job = AssignProcessToJobObject(job.0, process.0).is_ok();
            ResumeThread(thread.0);

            let left = deadline.saturating_duration_since(Instant::now());
            let exited =
                WaitForSingleObject(process.0, left.as_millis().min(u32::MAX as u128 - 1) as u32) == WAIT_OBJECT_0;
            let mut code = 0u32;
            let exit_code = (exited && GetExitCodeProcess(process.0, &mut code).is_ok()).then_some(code as i32);
            if !exited {
                return Ok(RunOutcome {
                    exit_code: None,
                    timed_out: true,
                });
            }
            while in_job && active_processes(&job) > 0 {
                if Instant::now() >= deadline {
                    return Ok(RunOutcome {
                        exit_code,
                        timed_out: true,
                    });
                }
                std::thread::sleep(POLL);
            }
            Ok(RunOutcome {
                exit_code,
                timed_out: false,
            })
        }
    }

    fn active_processes(job: &Owned) -> u32 {
        let mut info = JOBOBJECT_BASIC_ACCOUNTING_INFORMATION::default();
        let read = unsafe {
            QueryInformationJobObject(
                Some(job.0),
                JobObjectBasicAccountingInformation,
                (&mut info as *mut JOBOBJECT_BASIC_ACCOUNTING_INFORMATION).cast(),
                size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() as u32,
                None,
            )
        };
        // If the job can't be asked, don't wait on it forever.
        if read.is_ok() { info.ActiveProcesses } else { 0 }
    }

    /// Waits while Windows Installer is running an install anywhere on the PC (it holds the
    /// `Global\_MSIExecute` mutex), at most `timeout`. True once it's idle.
    pub fn wait_for_msi_idle(timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            match unsafe { OpenMutexW(SYNCHRONIZATION_SYNCHRONIZE, false, w!(r"Global\_MSIExecute")) } {
                Err(_) => return true,
                Ok(handle) => drop(Owned(handle)),
            }
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    }

    /// Waits until no installer process (`is_installer`) that started after `started_after` has
    /// been running for `quiet` in a row, at most `timeout`. True once it's quiet.
    ///
    /// The job object doesn't cover these: an uninstaller that copies itself to %TEMP% and
    /// restarts from there, or anything winget starts through its App Execution Alias, runs
    /// outside the job, and the command returns while they're still working.
    pub fn wait_for_installers(started_after: SystemTime, quiet: Duration, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        let since = filetime(started_after);
        let temps = temp_dirs();
        let mut quiet_since = Instant::now();
        loop {
            if installer_running(since, &temps) {
                quiet_since = Instant::now();
            } else if quiet_since.elapsed() >= quiet {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(POLL);
        }
    }

    /// 100-nanosecond ticks since 1601, as process start times are given.
    fn filetime(t: SystemTime) -> u64 {
        const UNIX_EPOCH_AS_FILETIME: u64 = 116_444_736_000_000_000;
        let since_unix = t.duration_since(UNIX_EPOCH).unwrap_or_default();
        UNIX_EPOCH_AS_FILETIME + (since_unix.as_nanos() / 100) as u64
    }

    /// The temp folders as full long paths, the form process image paths come in.
    fn temp_dirs() -> Vec<String> {
        let mut dirs: Vec<String> = temp_dir_candidates(|name| std::env::var(name).ok())
            .into_iter()
            .filter_map(|p| std::fs::canonicalize(p).ok())
            .map(|p| {
                let p = p.to_string_lossy().into_owned();
                p.strip_prefix(r"\\?\").map(str::to_string).unwrap_or(p)
            })
            .collect();
        dirs.sort();
        dirs.dedup();
        dirs
    }

    /// Any installer process that started at or after `since`, other than idiff itself. Only
    /// reads: the process list, and each process's start time, path and session.
    fn installer_running(since: u64, temps: &[String]) -> bool {
        let Ok(snapshot) = (unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }) else {
            return false;
        };
        let snapshot = Owned(snapshot);
        let me = std::process::id();
        let mut entry = PROCESSENTRY32W {
            dwSize: size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut more = unsafe { Process32FirstW(snapshot.0, &mut entry) }.is_ok();
        while more {
            let pid = entry.th32ProcessID;
            if pid != me && pid != 0 {
                let len = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());
                let name = String::from_utf16_lossy(&entry.szExeFile[..len]);
                if let Some((started, image)) = start_and_image(pid)
                    && started >= since
                {
                    let mut session = 0u32;
                    let service = unsafe { ProcessIdToSessionId(pid, &mut session) }.is_ok() && session == 0;
                    if is_installer(&name, image.as_deref(), service, temps) {
                        return true;
                    }
                }
            }
            more = unsafe { Process32NextW(snapshot.0, &mut entry) }.is_ok();
        }
        false
    }

    /// When the process started, and its full path if Windows tells us.
    fn start_and_image(pid: u32) -> Option<(u64, Option<String>)> {
        let process = Owned(unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?);
        let (mut created, mut exited, mut kernel, mut user) = Default::default();
        unsafe { GetProcessTimes(process.0, &mut created, &mut exited, &mut kernel, &mut user) }.ok()?;
        let started = (u64::from(created.dwHighDateTime) << 32) | u64::from(created.dwLowDateTime);
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let image =
            unsafe { QueryFullProcessImageNameW(process.0, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len) }
                .ok()
                .map(|()| String::from_utf16_lossy(&buf[..len as usize]));
        Some((started, image))
    }
}
