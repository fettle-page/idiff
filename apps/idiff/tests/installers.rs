//! Waiting for installers and uninstallers that keep working after the command returns.

use idiff::run::{is_installer, temp_dir_candidates};

const TEMP: &str = r"C:\Users\Niccolò\AppData\Local\Temp";

fn installer(name: &str, image: Option<&str>) -> bool {
    is_installer(name, image, false, &[TEMP.to_string()])
}

#[test]
fn installers_are_known_by_name_or_by_running_from_temp() {
    for name in [
        "Un_A.exe",
        "Un.exe",
        "un_b.EXE",
        "Au_.exe",
        "msiexec.exe",
        "setup.exe",
        "SetupHost.exe",
        "DiscordInstaller.exe",
        "install.exe",
        "unins000.exe",
        "Uninstall.exe",
        "uninst.exe",
        "Update.exe",
    ] {
        assert!(installer(name, None), "{name}");
    }
    for name in [
        "TrustedInstaller.exe",
        "un_ab.exe",
        "vlc.exe",
        "updater.exe",
        "notepad.exe",
        "setup",
        "msiexec",
    ] {
        assert!(!installer(name, None), "{name}");
    }
}

#[test]
fn anything_running_from_temp_counts() {
    // NSIS uninstallers copy themselves to %TEMP% and run from there.
    assert!(installer(
        "vlc.exe",
        Some(r"c:\users\niccolò\appdata\local\temp\~nsu1.tmp\Un_A.exe")
    ));
    assert!(installer("x.exe", Some(r"C:\USERS\NICCOLÒ\APPDATA\LOCAL\TEMP\x.exe")));
    assert!(!installer("x.exe", Some(r"C:\Users\Niccolò\AppData\Local\Temp2\x.exe")));
    assert!(!installer("vlc.exe", Some(r"C:\Program Files\VideoLAN\VLC\vlc.exe")));

    // SYSTEM's temp too: installers started by a service (updaters) run from there.
    let env = |name: &str| match name {
        "TEMP" => Some(TEMP.to_string()),
        "SystemRoot" => Some(r"C:\Windows".to_string()),
        _ => None,
    };
    let temps = temp_dir_candidates(env);
    assert_eq!(temps, [TEMP.to_string(), r"C:\Windows\Temp".to_string()]);
    assert!(is_installer(
        "x.exe",
        Some(r"C:\WINDOWS\Temp\{1A2B}\x.exe"),
        true,
        &temps
    ));
    assert!(!is_installer(
        "x.exe",
        Some(r"C:\Windows\Temporary\x.exe"),
        true,
        &temps
    ));
    assert!(!is_installer("x.exe", Some(r"C:\Windows\System32\x.exe"), true, &temps));
    assert!(temp_dir_candidates(|_| None).is_empty());
}

#[test]
fn the_windows_installer_service_is_left_to_the_msi_wait() {
    // msiexec in session 0 is the Windows Installer service (and its custom action hosts), which
    // stays up for minutes after any install; the Global\_MSIExecute mutex covers its work.
    assert!(!is_installer(
        "msiexec.exe",
        Some(r"C:\Windows\System32\msiexec.exe"),
        true,
        &[TEMP.to_string()]
    ));
    assert!(is_installer(
        "msiexec.exe",
        Some(r"C:\Windows\System32\msiexec.exe"),
        false,
        &[TEMP.to_string()]
    ));
}

#[cfg(windows)]
mod windows_only {
    use idiff::run::wait_for_installers;
    use std::path::Path;
    use std::process::Command;
    use std::sync::{Mutex, MutexGuard};
    use std::time::{Duration, Instant, SystemTime};

    /// These tests watch every process on the PC, so one test's stand-in uninstaller would make
    /// another wait: they take turns.
    static PROCESSES: Mutex<()> = Mutex::new(());

    fn one_at_a_time() -> MutexGuard<'static, ()> {
        PROCESSES.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// A copy of cmd.exe named like an NSIS uninstaller, in a temp folder, started through WMI so
    /// it isn't idiff's child or in any job of ours, the way an uninstaller's self-copy runs.
    fn start_detached_uninstaller(dir: &Path, name: &str, pings: u32) {
        let exe = dir.join(name);
        std::fs::copy(r"C:\Windows\System32\cmd.exe", &exe).unwrap();
        let command_line = format!(r#""{}" /c ping -n {pings} 127.0.0.1"#, exe.display());
        let created = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command"])
            .arg(format!(
                "$r = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{{ CommandLine = '{command_line}' }}; exit $r.ReturnValue"
            ))
            .status()
            .unwrap();
        assert!(created.success(), "Win32_Process.Create failed");
    }

    /// Stops our stand-in if it's still running, so its temp folder can be deleted.
    fn stop(name: &str) {
        let _ = Command::new("taskkill").args(["/F", "/IM", name]).output();
        std::thread::sleep(Duration::from_millis(500));
    }

    #[test]
    fn waits_while_a_detached_uninstaller_runs() {
        let _turn = one_at_a_time();
        let dir = tempfile::tempdir().unwrap();
        let started = SystemTime::now();
        start_detached_uninstaller(dir.path(), "Un_A.exe", 4);
        let t = Instant::now();
        assert!(wait_for_installers(
            started,
            Duration::from_secs(1),
            Duration::from_secs(30)
        ));
        // ping -n 4 runs about 3 seconds.
        assert!(t.elapsed() >= Duration::from_secs(2), "took {:?}", t.elapsed());
        stop("Un_A.exe");
    }

    #[test]
    fn returns_once_quiet_when_nothing_installs() {
        let _turn = one_at_a_time();
        let t = Instant::now();
        assert!(wait_for_installers(
            SystemTime::now(),
            Duration::from_secs(1),
            Duration::from_secs(10)
        ));
        assert!(t.elapsed() < Duration::from_secs(3), "took {:?}", t.elapsed());
    }

    #[test]
    fn gives_up_at_the_timeout() {
        let _turn = one_at_a_time();
        let dir = tempfile::tempdir().unwrap();
        let started = SystemTime::now();
        start_detached_uninstaller(dir.path(), "Un_T.exe", 30);
        let t = Instant::now();
        assert!(!wait_for_installers(
            started,
            Duration::from_secs(1),
            Duration::from_secs(2)
        ));
        assert!(t.elapsed() < Duration::from_secs(5), "took {:?}", t.elapsed());
        stop("Un_T.exe");
    }

    /// A path in the other form: 8.3 (`C:\Users\RUNNER~1\…`) when `long` is true it's the long
    /// one, and the reverse. None when Windows can't say.
    fn path_form(path: &Path, long: bool) -> Option<std::path::PathBuf> {
        use windows::Win32::Storage::FileSystem::{GetLongPathNameW, GetShortPathNameW};
        use windows::core::HSTRING;
        let wide = HSTRING::from(path);
        let mut buf = [0u16; 1024];
        let n = unsafe {
            if long {
                GetLongPathNameW(&wide, Some(&mut buf))
            } else {
                GetShortPathNameW(&wide, Some(&mut buf))
            }
        } as usize;
        (n > 0 && n < buf.len()).then(|| String::from_utf16_lossy(&buf[..n]).into())
    }

    /// Uninstallers started from the 8.3 form of %TEMP% (as on GitHub's runners, where TEMP is
    /// `C:\Users\RUNNER~1\…`) report their path in that form too. A name no rule knows, so only
    /// the temp folder can match it.
    #[test]
    fn waits_for_a_process_started_from_the_short_temp_path() {
        let _turn = one_at_a_time();
        let dir = tempfile::tempdir().unwrap();
        let (Some(short), Some(long)) = (path_form(dir.path(), false), path_form(dir.path(), true)) else {
            panic!("Windows gave no short or long form for {}", dir.path().display());
        };
        if short == long {
            eprintln!("skipped: this volume has no 8.3 names");
            return;
        }
        let started = SystemTime::now();
        start_detached_uninstaller(&short, "probe.exe", 4);
        let t = Instant::now();
        assert!(wait_for_installers(
            started,
            Duration::from_secs(1),
            Duration::from_secs(30)
        ));
        assert!(t.elapsed() >= Duration::from_secs(2), "took {:?}", t.elapsed());
        stop("probe.exe");
    }
}

#[test]
fn windows_helpers_in_its_temp_folder_are_not_installers() {
    let temps = vec![r"C:\Windows\Temp".to_string()];
    for name in ["DismHost.exe", "MpSigStub.exe", "mpam-fe.exe"] {
        let image = format!(r"C:\Windows\Temp\{{5B1A}}\{name}");
        assert!(!idiff::run::is_installer(name, Some(&image), true, &temps), "{name}");
    }
    assert!(idiff::run::is_installer(
        "x.exe",
        Some(r"C:\Windows\Temp\{5B1A}\x.exe"),
        true,
        &temps
    ));
}
