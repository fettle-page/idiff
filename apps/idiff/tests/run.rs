use idiff::run::{RunOutcome, exit_status, quote_args, uninstall_exit_status};

fn strings(args: &[&str]) -> Vec<String> {
    args.iter().map(|a| a.to_string()).collect()
}

#[test]
fn quote_args_follows_windows_rules() {
    assert_eq!(quote_args(&strings(&["a b", "c\"d", "e\\", ""])), r#""a b" c\"d e\ """#);
    // Backslashes before a quote are doubled, including the closing quote of a quoted arg.
    assert_eq!(
        quote_args(&strings(&[r"C:\Program Files\x\", r#"a\"b"#, "tab\there"])),
        "\"C:\\Program Files\\x\\\\\" a\\\\\\\"b \"tab\there\""
    );
    assert_eq!(
        quote_args(&strings(&["winget", "install", "--id", "VideoLAN.VLC"])),
        "winget install --id VideoLAN.VLC"
    );
}

#[test]
fn chromium_uninstall_success_code_counts_only_for_its_setup() {
    let exited = |code| RunOutcome {
        exit_code: Some(code),
        timed_out: false,
    };
    let brave = strings(&[
        r"C:\Program Files\BraveSoftware\Brave-Browser\Application\155.1.97.56\Installer\setup.exe",
        "--uninstall",
        "--system-level",
        "--force-uninstall",
    ]);
    assert_eq!(uninstall_exit_status(&brave, &exited(19)), 0);
    assert_eq!(uninstall_exit_status(&brave, &exited(20)), 2);
    assert_eq!(uninstall_exit_status(&brave, &exited(0)), 0);
    // 19 from anything else, or from a setup.exe that isn't uninstalling, is still a failure.
    let winget = strings(&["winget", "uninstall", "--id", "Brave.Brave"]);
    assert_eq!(uninstall_exit_status(&winget, &exited(19)), 2);
    assert_eq!(
        uninstall_exit_status(&strings(&[r"C:\x\setup.exe", "/S"]), &exited(19)),
        2
    );
    let timed_out = RunOutcome {
        exit_code: Some(19),
        timed_out: true,
    };
    assert_eq!(uninstall_exit_status(&brave, &timed_out), 2);
}

#[test]
fn exit_status_maps_codes() {
    let exited = |code| RunOutcome {
        exit_code: Some(code),
        timed_out: false,
    };
    assert_eq!(exit_status(&exited(0)), 0);
    assert_eq!(exit_status(&exited(3010)), 0);
    assert_eq!(exit_status(&exited(1641)), 0);
    assert_eq!(exit_status(&exited(1)), 2);
    assert_eq!(exit_status(&exited(-1978335212)), 2);
    assert_eq!(
        exit_status(&RunOutcome {
            exit_code: Some(0),
            timed_out: true
        }),
        2
    );
    assert_eq!(
        exit_status(&RunOutcome {
            exit_code: None,
            timed_out: true
        }),
        2
    );
}

#[cfg(windows)]
mod windows_only {
    use super::strings;
    use idiff::run::{run_in_job, wait_for_msi_idle};
    use idiff_core::copy;
    use std::time::{Duration, Instant};

    #[test]
    fn waits_for_child_processes() {
        // cmd exits at once; the ping it starts keeps running in the job for about 2 s.
        let start = Instant::now();
        let outcome = run_in_job(
            &strings(&["cmd", "/c", "start", "/b", "ping", "-n", "3", "127.0.0.1"]),
            Duration::from_secs(30),
        )
        .unwrap();
        assert!(
            start.elapsed() >= Duration::from_millis(1500),
            "took {:?}",
            start.elapsed()
        );
        assert_eq!(outcome.exit_code, Some(0));
        assert!(!outcome.timed_out);
    }

    #[test]
    fn the_exit_code_is_kept() {
        let outcome = run_in_job(&strings(&["cmd", "/c", "exit", "3010"]), Duration::from_secs(30)).unwrap();
        assert_eq!(outcome.exit_code, Some(3010));
    }

    #[test]
    fn a_command_past_its_timeout_is_timed_out() {
        let start = Instant::now();
        let outcome = run_in_job(&strings(&["ping", "-n", "6", "127.0.0.1"]), Duration::from_secs(1)).unwrap();
        assert!(outcome.timed_out);
        assert!(start.elapsed() < Duration::from_secs(4), "took {:?}", start.elapsed());
    }

    #[test]
    fn missing_command_is_reported() {
        let err = run_in_job(
            &strings(&["idiff-no-such-command-x7", "--flag"]),
            Duration::from_secs(5),
        )
        .unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
        assert_eq!(err.to_string(), copy::COMMAND_NOT_FOUND);
        assert!(run_in_job(&[], Duration::from_secs(5)).is_err());
    }

    #[test]
    fn msi_idle_is_immediate_when_nothing_installs() {
        let start = Instant::now();
        assert!(wait_for_msi_idle(Duration::from_secs(10)));
        assert!(start.elapsed() < Duration::from_secs(2), "took {:?}", start.elapsed());
    }
}
