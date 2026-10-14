//! Runs the built binary on snapshot files, so `diff` is checked without any collector.

use idiff::session::SessionMeta;
use idiff_core::copy;
use idiff_core::model::{Item, SCHEMA, Snapshot, SourceKind, SourceSnap};
use idiff_core::verdict::{Level, Line, Mode, Receipt};
use std::collections::BTreeMap;
use std::process::{Command, Output};

fn idiff(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_idiff")).args(args).output().unwrap()
}

fn stdout(o: &Output) -> String {
    String::from_utf8(o.stdout.clone()).unwrap()
}

fn stderr(o: &Output) -> String {
    String::from_utf8(o.stderr.clone()).unwrap()
}

fn snapshot(folders: &[&str], defender_error: Option<&str>) -> Snapshot {
    let items = folders
        .iter()
        .map(|k| Item {
            key: k.to_string(),
            label: k.rsplit('\\').next().unwrap().to_string(),
            ..Default::default()
        })
        .collect();
    let mut sources = BTreeMap::new();
    sources.insert(
        SourceKind::Folders,
        SourceSnap {
            ok: true,
            items,
            ..Default::default()
        },
    );
    sources.insert(
        SourceKind::Defender,
        SourceSnap {
            ok: defender_error.is_none(),
            error: defender_error.map(String::from),
            ..Default::default()
        },
    );
    Snapshot {
        schema: SCHEMA,
        idiff_version: "0.1.0".into(),
        taken_at_ms: 0,
        elevated: false,
        env: BTreeMap::new(),
        sources,
    }
}

fn write(dir: &tempfile::TempDir, name: &str, s: &Snapshot) -> String {
    let path = dir.path().join(name);
    std::fs::write(&path, s.to_json()).unwrap();
    path.to_string_lossy().into_owned()
}

#[test]
fn diff_prints_a_receipt() {
    let dir = tempfile::tempdir().unwrap();
    let a = write(
        &dir,
        "a.json",
        &snapshot(&[r"c:\users\niccolò\appdata\local\old"], None),
    );
    let b = write(
        &dir,
        "b.json",
        &snapshot(&[r"c:\users\niccolò\appdata\local\new"], Some("needs administrator")),
    );
    let out = idiff(&["diff", &a, &b]);
    assert!(out.status.success(), "{}", stderr(&out));
    let not_enriched = format!("· {}", copy::DIFF_NOT_ENRICHED);
    assert_eq!(
        stdout(&out).lines().collect::<Vec<_>>(),
        [
            "Unknown app: installed",
            "✓ No lasting changes found",
            "· Couldn't check antivirus exclusions: needs administrator",
            not_enriched.as_str(),
            "",
            "Also changed while installing (couldn't tell if it was this app)",
            "· Added folder \"new\"",
            "· Removed folder \"old\"",
        ]
    );
    // Piped, so no colour.
    assert!(!stdout(&out).contains('\x1b'));
    let why = stdout(&idiff(&["diff", &a, &b, "--why"]));
    let want = format!(r"    c:\users\niccolò\appdata\local\new: {}", copy::WHY_UNATTRIBUTED);
    assert!(why.contains(&want), "{why}");
    let app = stdout(&idiff(&["diff", &a, &b, "--app", "New Thing"]));
    assert!(app.starts_with("New Thing: installed"), "{app}");
}

#[test]
fn diff_of_identical_snapshots_finds_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let a = write(&dir, "a.json", &snapshot(&["c:\\x"], None));
    let out = idiff(&["diff", &a, &a]);
    assert!(out.status.success());
    assert_eq!(
        stdout(&out).lines().take(2).collect::<Vec<_>>(),
        ["No install found", "✓ No lasting changes found"]
    );
}

#[test]
fn diff_of_a_damaged_file_is_an_idiff_error() {
    let dir = tempfile::tempdir().unwrap();
    let bad = dir.path().join("bad.json");
    std::fs::write(&bad, "{not json").unwrap();
    let bad = bad.to_string_lossy().into_owned();
    let out = idiff(&["diff", &bad, &bad]);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains(copy::SNAPSHOT_UNREADABLE), "{}", stderr(&out));
    let missing = dir.path().join("missing.json").to_string_lossy().into_owned();
    let out = idiff(&["diff", &missing, &missing]);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains(copy::CANT_READ_FILE), "{}", stderr(&out));
}

#[test]
fn commands_not_built_yet_say_so() {
    for args in [
        &["leftovers", "last"][..],
        &["export", "last", "--format", "md"],
        &["calibrate", "--minutes", "5"],
        &["list"],
    ] {
        let out = idiff(args);
        assert_eq!(out.status.code(), Some(1), "{args:?}");
        assert!(stderr(&out).contains(copy::NOT_BUILT_YET), "{args:?}: {}", stderr(&out));
    }
}

#[cfg(windows)]
#[test]
fn snapshot_writes_a_file_that_loads() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.json");
    // --no-elevate: a test must never show a UAC prompt.
    let out = idiff(&["snapshot", "--no-elevate", "-o", &path.to_string_lossy()]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stdout(&out).is_empty());
    let s = Snapshot::load(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(s.sources.len(), SourceKind::ALL.len());
    let programs = &s.sources[&SourceKind::Programs];
    assert!(programs.ok && !programs.items.is_empty(), "{:?}", programs.error);
}

/// The elevated helper's entry, run here unelevated: it needs no UAC to write its file.
#[cfg(windows)]
#[test]
fn helper_mode_writes_only_its_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = idiff::elevate::helper_path(dir.path(), 9, 0xabc);
    let path_arg = path.to_string_lossy();
    let out = idiff(&["--helper", &path_arg, "--only", "Defender"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stdout(&out).is_empty());
    let s = Snapshot::load(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(s.sources.keys().collect::<Vec<_>>(), [&SourceKind::Defender]);

    let refused = idiff(&["--helper", &path_arg, "--only", "Programs"]);
    assert_eq!(refused.status.code(), Some(1));
    let again = idiff(&["--helper", &path_arg, "--only", "Defender"]);
    assert_eq!(again.status.code(), Some(1), "an existing file is never overwritten");
}

#[cfg(windows)]
#[test]
fn snapshot_without_output_prints_it() {
    let out = idiff(&["snapshot", "--no-elevate"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(Snapshot::load(&stdout(&out)).is_ok());
}

fn idiff_at(home: &std::path::Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_idiff"))
        .args(args)
        .env("IDIFF_HOME", home)
        .output()
        .unwrap()
}

/// A saved session with a receipt, as `run` leaves one.
fn saved_session(home: &std::path::Path) -> Receipt {
    let id = "20261014-123456-discord";
    let dir = home.join("sessions").join(id);
    std::fs::create_dir_all(&dir).unwrap();
    let meta = SessionMeta {
        id: id.into(),
        command: vec!["winget".into(), "install".into(), "Discord.Discord".into()],
        app_flag: None,
        subject: Some("Discord".into()),
        exit_code: Some(0),
        started_ms: 1,
        finished_ms: Some(2),
        idiff_version: "0.1.0".into(),
    };
    let line = |level, text: &str| Line {
        level,
        text: text.into(),
        items: vec!["mykey".into()],
        why: vec![copy::WHY_TARGET_IN_INSTALL_DIR.into()],
    };
    let receipt = Receipt {
        mode: Mode::Install,
        title: "Discord 1.0.9170: installed".into(),
        lines: vec![line(Level::Warn, "1 scheduled task: runs an updater every day")],
        unknown: vec![],
        hidden: vec![line(Level::Note, "Added driver x")],
    };
    std::fs::write(dir.join("session.json"), serde_json::to_string(&meta).unwrap()).unwrap();
    std::fs::write(dir.join("receipt.json"), serde_json::to_string(&receipt).unwrap()).unwrap();
    receipt
}

#[test]
fn show_prints_a_saved_receipt() {
    let home = tempfile::tempdir().unwrap();
    let receipt = saved_session(home.path());
    for target in ["last", "20261014-123456-discord"] {
        let out = idiff_at(home.path(), &["show", target]);
        assert!(out.status.success(), "{}", stderr(&out));
        assert_eq!(
            stdout(&out),
            "Discord 1.0.9170: installed\n⚠ 1 scheduled task: runs an updater every day\n"
        );
    }
    let all = stdout(&idiff_at(home.path(), &["show", "last", "--all", "--why"]));
    assert!(
        all.contains(copy::HIDDEN_HEADING) && all.contains("Added driver x"),
        "{all}"
    );
    assert!(all.contains(copy::WHY_TARGET_IN_INSTALL_DIR), "{all}");
    let json = stdout(&idiff_at(home.path(), &["show", "last", "--json"]));
    assert_eq!(serde_json::from_str::<Receipt>(&json).unwrap(), receipt);
}

#[test]
fn show_explains_what_to_do_first() {
    let home = tempfile::tempdir().unwrap();
    let out = idiff_at(home.path(), &["show", "last"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains(copy::NO_SESSIONS), "{}", stderr(&out));
    let out = idiff_at(home.path(), &["show", "nope"]);
    assert!(stderr(&out).contains(copy::NO_SUCH_SESSION), "{}", stderr(&out));
}

#[test]
fn show_says_when_a_session_didnt_finish() {
    let home = tempfile::tempdir().unwrap();
    saved_session(home.path());
    let dir = home.path().join("sessions").join("20261014-123456-discord");
    std::fs::remove_file(dir.join("receipt.json")).unwrap();
    let out = idiff_at(home.path(), &["show", "last"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains(copy::SESSION_UNFINISHED), "{}", stderr(&out));
}

/// `idiff run` with a harmless command: nothing is installed. Never elevated, never prompts.
#[cfg(windows)]
fn run_at(home: &std::path::Path, command: &[&str]) -> Output {
    let mut args = vec!["run", "--yes", "--settle", "0", "--no-elevate", "--"];
    args.extend(command);
    idiff_at(home, &args)
}

#[cfg(windows)]
fn only_session(home: &std::path::Path) -> std::path::PathBuf {
    let sessions: Vec<_> = std::fs::read_dir(home.join("sessions"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(sessions.len(), 1, "{sessions:?}");
    sessions[0].clone()
}

#[cfg(windows)]
#[test]
fn run_records_every_step_and_prints_the_receipt() {
    let home = tempfile::tempdir().unwrap();
    let made = home.path().join("made-by-the-command");
    let out = run_at(home.path(), &["cmd", "/c", "mkdir", &made.to_string_lossy()]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert!(made.is_dir(), "the command ran");
    let dir = only_session(home.path());
    for file in ["session.json", "before.json", "after.json", "receipt.json"] {
        assert!(dir.join(file).is_file(), "{file}");
    }
    let meta: SessionMeta = serde_json::from_str(&std::fs::read_to_string(dir.join("session.json")).unwrap()).unwrap();
    assert_eq!(meta.command, ["cmd", "/c", "mkdir", &made.to_string_lossy()]);
    assert_eq!(meta.exit_code, Some(0));
    assert!(meta.finished_ms.is_some_and(|f| f >= meta.started_ms));
    assert!(meta.subject.is_some());
    let receipt: Receipt = serde_json::from_str(&std::fs::read_to_string(dir.join("receipt.json")).unwrap()).unwrap();
    assert_eq!(stdout(&out).lines().next(), Some(receipt.title.as_str()));
    // What `show` prints is the same receipt.
    assert_eq!(stdout(&idiff_at(home.path(), &["show", "last"])), stdout(&out));
}

#[cfg(windows)]
#[test]
fn run_exit_codes_follow_the_command() {
    let home = tempfile::tempdir().unwrap();
    let out = run_at(home.path(), &["cmd", "/c", "exit", "3010"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert!(
        stdout(&out).contains(&format!("· {}", copy::RESTART_NEEDED)),
        "{}",
        stdout(&out)
    );

    let home = tempfile::tempdir().unwrap();
    let out = run_at(home.path(), &["cmd", "/c", "exit", "5"]);
    assert_eq!(out.status.code(), Some(2), "{}", stderr(&out));
    assert!(
        stdout(&out).contains("⚠ The install command exited with code 5"),
        "{}",
        stdout(&out)
    );
}

#[cfg(windows)]
#[test]
fn run_of_a_missing_command_leaves_an_unfinished_session() {
    let home = tempfile::tempdir().unwrap();
    let out = run_at(home.path(), &["no-such-installer-7d1f.exe", "/S"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains(copy::COMMAND_NOT_FOUND), "{}", stderr(&out));
    let dir = only_session(home.path());
    assert!(dir.join("before.json").is_file());
    assert!(!dir.join("receipt.json").exists());
    let show = idiff_at(home.path(), &["show", "last"]);
    assert!(stderr(&show).contains(copy::SESSION_UNFINISHED), "{}", stderr(&show));
}
