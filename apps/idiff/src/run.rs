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

#[cfg(windows)]
pub use windows_impl::{run_in_job, wait_for_msi_idle};

#[cfg(windows)]
mod windows_impl {
    use super::{RunOutcome, quote_args};
    use idiff_core::copy;
    use std::io;
    use std::time::{Duration, Instant};
    use windows::Win32::Foundation::{CloseHandle, ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND, HANDLE, WAIT_OBJECT_0};
    use windows::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOBOBJECT_BASIC_ACCOUNTING_INFORMATION,
        JobObjectBasicAccountingInformation, QueryInformationJobObject,
    };
    use windows::Win32::System::Threading::{
        CREATE_SUSPENDED, CreateProcessW, GetExitCodeProcess, OpenMutexW, PROCESS_INFORMATION, ResumeThread,
        STARTUPINFOW, SYNCHRONIZATION_SYNCHRONIZE, WaitForSingleObject,
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
}
