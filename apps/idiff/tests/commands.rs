//! `leftovers`, `export`, `calibrate` and `list`, through the built binary. Nothing here installs
//! or uninstalls anything or asks for administrator rights: every command that would runs with
//! `--no-elevate --yes` and a harmless `cmd /c` stand-in.

use idiff::session::SessionMeta;
use idiff_core::copy;
use idiff_core::verdict::{Level, Line, Mode, Receipt};
use std::path::Path;
use std::process::{Command, Output};

fn idiff_at(home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_idiff"))
        .args(args)
        .env("IDIFF_HOME", home)
        .output()
        .unwrap()
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

fn line(level: Level, text: &str) -> Line {
    Line {
        level,
        text: text.into(),
        items: vec![],
        why: vec![],
    }
}

fn receipt(mode: Mode, title: &str, lines: Vec<Line>) -> Receipt {
    Receipt {
        mode,
        title: title.into(),
        lines,
        unknown: vec![],
        hidden: vec![line(Level::Note, "Added driver x")],
    }
}

/// A session folder as `run` leaves it: session.json, and the given files.
fn session(home: &Path, id: &str, started_ms: u64, finished: bool, files: &[(&str, String)]) -> std::path::PathBuf {
    let dir = home.join("sessions").join(id);
    std::fs::create_dir_all(&dir).unwrap();
    let meta = SessionMeta {
        id: id.into(),
        command: vec!["winget".into(), "install".into(), "Example.App".into()],
        app_flag: None,
        subject: Some("Example".into()),
        exit_code: finished.then_some(0),
        started_ms,
        finished_ms: finished.then_some(started_ms + 1),
        idiff_version: "0.1.0".into(),
    };
    std::fs::write(dir.join("session.json"), serde_json::to_string(&meta).unwrap()).unwrap();
    for (name, text) in files {
        std::fs::write(dir.join(name), text).unwrap();
    }
    dir
}

fn json(r: &Receipt) -> String {
    serde_json::to_string(r).unwrap()
}

#[cfg(windows)]
mod leftovers {
    use super::*;

    /// A finished session whose before and after are this PC right now, so nothing was installed.
    fn finished_nothing_session(home: &Path) -> std::path::PathBuf {
        let dir = session(home, "20261014-120000-example", 1, true, &[]);
        let snap = idiff_at(
            home,
            &[
                "snapshot",
                "--no-elevate",
                "-o",
                &dir.join("before.json").to_string_lossy(),
            ],
        );
        assert!(snap.status.success(), "{}", stderr(&snap));
        std::fs::copy(dir.join("before.json"), dir.join("after.json")).unwrap();
        let install = receipt(
            Mode::Install,
            "Example 1.0: installed",
            vec![line(Level::Good, "No lasting changes found")],
        );
        std::fs::write(dir.join("receipt.json"), json(&install)).unwrap();
        dir
    }

    #[test]
    fn leftovers_runs_the_uninstall_command_and_saves_its_receipt() {
        let home = tempfile::tempdir().unwrap();
        let dir = finished_nothing_session(home.path());
        let marker = home.path().join("uninstaller-ran");
        let out = idiff_at(
            home.path(),
            &[
                "leftovers",
                "last",
                "--yes",
                "--no-elevate",
                "--settle",
                "0",
                "--",
                "cmd",
                "/c",
                "mkdir",
                &marker.to_string_lossy(),
            ],
        );
        assert!(out.status.success(), "{}", stderr(&out));
        assert!(marker.is_dir(), "the uninstall command ran");
        assert!(stderr(&out).contains(copy::WAITING_FOR_INSTALLER), "{}", stderr(&out));
        assert!(
            // The app's name comes from the session's winget command.
            stdout(&out).starts_with("Example.App: after uninstall, "),
            "{}",
            stdout(&out)
        );
        for file in ["uninstalled.json", "leftovers.json"] {
            assert!(dir.join(file).is_file(), "{file}");
        }
        let saved: Receipt =
            serde_json::from_str(&std::fs::read_to_string(dir.join("leftovers.json")).unwrap()).unwrap();
        assert_eq!(saved.mode, Mode::Leftovers);

        // `show` gives the install receipt, then the leftovers one; `--json` stays the install receipt.
        let shown = stdout(&idiff_at(home.path(), &["show", "last"]));
        let (install_at, leftovers_at) = (shown.find("Example 1.0: installed"), shown.find(&saved.title));
        assert!(
            install_at.is_some() && leftovers_at.is_some() && install_at < leftovers_at,
            "{shown}"
        );
        let shown_json = stdout(&idiff_at(home.path(), &["show", "last", "--json"]));
        assert_eq!(
            serde_json::from_str::<Receipt>(&shown_json).unwrap().mode,
            Mode::Install
        );
    }

    #[test]
    fn a_failed_uninstall_command_still_gives_a_receipt_and_exit_code_2() {
        let home = tempfile::tempdir().unwrap();
        finished_nothing_session(home.path());
        let out = idiff_at(
            home.path(),
            &[
                "leftovers",
                "last",
                "--yes",
                "--no-elevate",
                "--settle",
                "0",
                "--",
                "cmd",
                "/c",
                "exit",
                "5",
            ],
        );
        assert_eq!(out.status.code(), Some(2), "{}", stderr(&out));
        assert!(stdout(&out).contains("after uninstall"), "{}", stdout(&out));
    }

    /// A session whose "install" added one program that's really on this PC: its before snapshot
    /// is this PC without that program. Nothing is installed or removed, so after any "uninstall"
    /// the program is still listed, as when an uninstall didn't run.
    fn session_with_a_program_still_installed(home: &Path) -> std::path::PathBuf {
        let dir = finished_nothing_session(home);
        let mut before: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("after.json")).unwrap()).unwrap();
        let items = before["sources"]["Programs"]["items"].as_array_mut().unwrap();
        let program = items
            .iter()
            .position(|i| i["props"]["system_component"].is_null())
            .expect("this PC lists at least one program");
        items.remove(program);
        std::fs::write(dir.join("before.json"), serde_json::to_string(&before).unwrap()).unwrap();
        dir
    }

    #[test]
    fn an_app_still_listed_after_the_uninstall_gives_an_honest_receipt() {
        let home = tempfile::tempdir().unwrap();
        let dir = session_with_a_program_still_installed(home.path());
        let out = idiff_at(home.path(), &["leftovers", "last", "--yes", "--no-elevate"]);
        assert_eq!(out.status.code(), Some(2), "{}", stderr(&out));
        let saved: Receipt =
            serde_json::from_str(&std::fs::read_to_string(dir.join("leftovers.json")).unwrap()).unwrap();
        assert_eq!(
            (saved.lines[0].level, saved.lines[0].text.as_str()),
            (Level::Warn, copy::UNINSTALL_DIDNT_FINISH)
        );
        assert!(stdout(&out).contains(copy::UNINSTALL_DIDNT_FINISH), "{}", stdout(&out));
    }

    #[test]
    fn a_timeout_is_the_one_warning_when_the_app_is_also_still_listed() {
        let home = tempfile::tempdir().unwrap();
        let dir = session_with_a_program_still_installed(home.path());
        let out = idiff_at(
            home.path(),
            &[
                "leftovers",
                "last",
                "--yes",
                "--no-elevate",
                "--settle",
                "0",
                "--timeout",
                "0",
                "--",
                "ping",
                "-n",
                "3",
                "127.0.0.1",
            ],
        );
        assert_eq!(out.status.code(), Some(2), "{}", stderr(&out));
        let saved: Receipt =
            serde_json::from_str(&std::fs::read_to_string(dir.join("leftovers.json")).unwrap()).unwrap();
        assert_eq!(saved.lines[0].text, copy::UNINSTALL_TIMED_OUT.replace("{n}", "0"));
        assert!(!saved.lines.iter().any(|l| l.text == copy::UNINSTALL_DIDNT_FINISH));
    }

    /// An uninstaller that hangs (at a dialog nobody can click) must not pass for a finished one:
    /// the receipt says the list may hold things it would have removed, and idiff exits 2.
    #[test]
    fn a_hung_uninstall_gives_an_honest_receipt() {
        let home = tempfile::tempdir().unwrap();
        let dir = finished_nothing_session(home.path());
        // --timeout 0: the stand-in "uninstaller" (a short ping) is still running when time is up.
        let out = idiff_at(
            home.path(),
            &[
                "leftovers",
                "last",
                "--yes",
                "--no-elevate",
                "--settle",
                "0",
                "--timeout",
                "0",
                "--",
                "ping",
                "-n",
                "3",
                "127.0.0.1",
            ],
        );
        assert_eq!(out.status.code(), Some(2), "{}", stderr(&out));
        let saved: Receipt =
            serde_json::from_str(&std::fs::read_to_string(dir.join("leftovers.json")).unwrap()).unwrap();
        let first = &saved.lines[0];
        assert_eq!(first.level, Level::Warn);
        assert_eq!(first.text, copy::UNINSTALL_TIMED_OUT.replace("{n}", "0"));
        assert!(stdout(&out).contains(&first.text), "{}", stdout(&out));
    }

    #[test]
    fn without_a_command_yes_just_takes_the_snapshot() {
        let home = tempfile::tempdir().unwrap();
        finished_nothing_session(home.path());
        let out = idiff_at(home.path(), &["leftovers", "last", "--yes", "--no-elevate"]);
        assert!(out.status.success(), "{}", stderr(&out));
        assert!(
            !stderr(&out).contains("Uninstall"),
            "no prompt with --yes: {}",
            stderr(&out)
        );
    }

    #[test]
    fn an_unfinished_session_is_refused_before_anything_runs() {
        let home = tempfile::tempdir().unwrap();
        session(
            home.path(),
            "20261014-120000-example",
            1,
            false,
            &[("before.json", "{}".into())],
        );
        let marker = home.path().join("must-not-run");
        let out = idiff_at(
            home.path(),
            &[
                "leftovers",
                "last",
                "--yes",
                "--no-elevate",
                "--",
                "cmd",
                "/c",
                "mkdir",
                &marker.to_string_lossy(),
            ],
        );
        assert_eq!(out.status.code(), Some(1));
        assert!(stderr(&out).contains(copy::SESSION_UNFINISHED), "{}", stderr(&out));
        assert!(!marker.exists(), "the uninstall command must not run");
    }
}

mod export {
    use super::*;

    fn saved(home: &Path, private: &str) -> std::path::PathBuf {
        let install = receipt(
            Mode::Install,
            "Example 1.0: installed",
            vec![line(Level::Warn, &format!(r"Adds to PATH: C:\Users\{private}\bin"))],
        );
        let leftovers = receipt(
            Mode::Leftovers,
            "Example 1.0: after uninstall, 1 leftover",
            vec![line(Level::Bad, "Leftover <x>")],
        );
        session(
            home,
            "20261014-120000-example",
            1,
            true,
            &[("receipt.json", json(&install)), ("leftovers.json", json(&leftovers))],
        )
    }

    #[test]
    fn export_writes_each_format_with_both_receipts() {
        let home = tempfile::tempdir().unwrap();
        saved(home.path(), "someone");
        let out_dir = tempfile::tempdir().unwrap();
        for (format, starts) in [
            ("md", "## Example 1.0: installed"),
            ("html", "<!doctype html>"),
            ("json", "{"),
        ] {
            let path = out_dir.path().join(format!("r.{format}"));
            let out = idiff_at(
                home.path(),
                &["export", "last", "--format", format, "-o", &path.to_string_lossy()],
            );
            assert!(out.status.success(), "{format}: {}", stderr(&out));
            let text = std::fs::read_to_string(&path).unwrap();
            assert!(text.starts_with(starts), "{format}: {text}");
            assert!(text.contains("after uninstall, 1 leftover"), "{format}: {text}");
        }
        // HTML escapes what it shows, and stays one document.
        let html = std::fs::read_to_string(out_dir.path().join("r.html")).unwrap();
        assert!(
            html.contains("Leftover &lt;x&gt;") && html.matches("<html").count() == 1,
            "{html}"
        );
        let both: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(out_dir.path().join("r.json")).unwrap()).unwrap();
        assert_eq!(both["install"]["mode"], "Install");
        assert_eq!(both["leftovers"]["mode"], "Leftovers");
    }

    #[test]
    fn export_names_its_file_after_the_session() {
        let home = tempfile::tempdir().unwrap();
        saved(home.path(), "someone");
        let cwd = tempfile::tempdir().unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_idiff"))
            .args(["export", "last", "--format", "md"])
            .env("IDIFF_HOME", home.path())
            .current_dir(cwd.path())
            .output()
            .unwrap();
        assert!(out.status.success(), "{}", stderr(&out));
        assert!(cwd.path().join("idiff-20261014-120000-example.md").is_file());
    }

    #[cfg(windows)]
    #[test]
    fn export_hides_this_pcs_names_unless_asked_not_to() {
        let home = tempfile::tempdir().unwrap();
        let me = std::env::var("USERNAME").unwrap();
        saved(home.path(), &me);
        let out_dir = tempfile::tempdir().unwrap();
        let path = out_dir.path().join("r.md");
        let path_arg = path.to_string_lossy();

        let out = idiff_at(home.path(), &["export", "last", "--format", "md", "-o", &path_arg]);
        assert!(out.status.success() && !stderr(&out).contains(copy::NOT_REDACTED));
        let text = std::fs::read_to_string(&path).unwrap();
        // The message never prints the name: it's this PC's.
        assert!(
            !text.to_lowercase().contains(&me.to_lowercase()),
            "the user name is still in the export"
        );
        assert!(
            // Markdown-escaped: `\` and `<>` get a backslash each.
            text.contains(r"C:\\Users\\\<user\>\\bin"),
            "{text}"
        );

        let out = idiff_at(
            home.path(),
            &["export", "last", "--format", "md", "-o", &path_arg, "--no-redact"],
        );
        assert!(out.status.success());
        assert!(stderr(&out).contains(copy::NOT_REDACTED), "{}", stderr(&out));
        assert!(
            std::fs::read_to_string(&path)
                .unwrap()
                .to_lowercase()
                .contains(&me.to_lowercase())
        );
    }

    #[test]
    fn an_unfinished_session_has_nothing_to_export() {
        let home = tempfile::tempdir().unwrap();
        session(home.path(), "20261014-120000-example", 1, false, &[]);
        let out = idiff_at(home.path(), &["export", "last", "--format", "md"]);
        assert_eq!(out.status.code(), Some(1));
        assert!(stderr(&out).contains(copy::SESSION_UNFINISHED), "{}", stderr(&out));
    }
}

mod calibrate {
    use super::*;
    use idiff::cmd::calibrate::{calibration_receipt, save_rules};
    use idiff::noise::{LOCAL_FILE, load};
    use idiff_core::model::{Item, Snapshot, SourceKind};
    use idiff_core::noise::NoiseList;

    fn fixture(name: &str) -> Snapshot {
        let text = match name {
            "before" => include_str!("../../../crates/idiff-core/tests/fixtures/discord-before.json"),
            _ => include_str!("../../../crates/idiff-core/tests/fixtures/discord-after.json"),
        };
        Snapshot::load(text).unwrap()
    }

    #[test]
    fn changes_with_no_app_are_windows_or_unknown() {
        let (r, unknown) = calibration_receipt(&fixture("before"), &fixture("after"), &NoiseList::builtin(), 3);
        // No findings: only notes about sources that couldn't be compared.
        assert!(r.lines.iter().all(|l| l.level == Level::Note), "{:?}", r.lines);
        assert!(r.title.contains('3'), "{}", r.title);
        assert!(!r.hidden.is_empty(), "Windows' own changes are listed");
        // With no app to compare against, Discord's own changes can't be told apart either.
        assert!(
            unknown.contains(&(SourceKind::Tasks, r"\discordupdate".to_string())),
            "{unknown:?}"
        );
        assert!(!r.unknown.is_empty());
        // Hidden changes are Windows' own, so they're never saved as new rules.
        assert!(!unknown.iter().any(|(_, key)| key.starts_with("mpksl")), "{unknown:?}");
    }

    #[test]
    fn saved_rules_hide_those_changes_next_time() {
        let home = tempfile::tempdir().unwrap();
        let keys = vec![(SourceKind::Services, "foo service".to_string())];
        let path = save_rules(home.path(), &keys).unwrap();
        assert_eq!(path, home.path().join(LOCAL_FILE));
        save_rules(home.path(), &[(SourceKind::Tasks, r"\x".to_string())]).unwrap();
        let (noise, warning) = load(home.path());
        assert_eq!(warning, None);
        let item = Item {
            key: "foo service".into(),
            ..Default::default()
        };
        assert!(
            noise
                .matches(SourceKind::Services, &item, &Default::default())
                .is_some()
        );
        // Nothing to save writes nothing.
        let empty = tempfile::tempdir().unwrap();
        save_rules(empty.path(), &[]).unwrap();
        assert!(!empty.path().join(LOCAL_FILE).exists());
    }

    #[cfg(windows)]
    #[test]
    fn calibrate_for_zero_minutes_works_without_elevation() {
        let home = tempfile::tempdir().unwrap();
        let out = idiff_at(home.path(), &["calibrate", "--minutes", "0", "--no-elevate", "--save"]);
        assert!(out.status.success(), "{}", stderr(&out));
        assert!(stdout(&out).contains("0 min"), "{}", stdout(&out));
        // Whatever it saved must load cleanly.
        assert_eq!(load(home.path()).1, None);
    }
}

mod list {
    use super::*;

    #[test]
    fn list_shows_sessions_newest_first_with_their_counts() {
        let home = tempfile::tempdir().unwrap();
        let warn_bad = receipt(
            Mode::Install,
            "Example 1.0: installed",
            vec![
                line(Level::Warn, "a"),
                line(Level::Warn, "b"),
                line(Level::Bad, "c"),
                line(Level::Good, "d"),
            ],
        );
        session(
            home.path(),
            "20261014-100000-old",
            1,
            true,
            &[("receipt.json", json(&warn_bad))],
        );
        session(home.path(), "20261014-110000-new", 2, false, &[]);
        let out = idiff_at(home.path(), &["list"]);
        assert!(out.status.success(), "{}", stderr(&out));
        let text = stdout(&out);
        let rows: Vec<&str> = text.lines().collect();
        assert_eq!(rows.len(), 2, "{text}");
        assert!(
            rows[0].starts_with("20261014-110000-new") && rows[0].ends_with(copy::LIST_UNFINISHED),
            "{text}"
        );
        assert!(
            rows[1].starts_with("20261014-100000-old") && rows[1].contains("⚠2 ✗1"),
            "{text}"
        );
        assert!(
            rows[1].contains("Example") && rows[1].ends_with(copy::LIST_FINISHED),
            "{text}"
        );
    }

    #[test]
    fn list_with_no_sessions_says_what_to_do() {
        let home = tempfile::tempdir().unwrap();
        let out = idiff_at(home.path(), &["list"]);
        assert!(out.status.success());
        assert_eq!(stdout(&out).trim(), copy::NO_SESSIONS);
    }
}
