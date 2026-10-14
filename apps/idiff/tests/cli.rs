//! Runs the built binary on snapshot files, so `diff` is checked without any collector.

use idiff_core::copy;
use idiff_core::model::{Item, SCHEMA, Snapshot, SourceKind, SourceSnap};
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
fn diff_prints_the_raw_delta() {
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
    assert_eq!(
        stdout(&out).lines().collect::<Vec<_>>(),
        [
            r"+ Folders new [c:\users\niccolò\appdata\local\new]",
            r"- Folders old [c:\users\niccolò\appdata\local\old]",
            "! Defender: needs administrator",
        ]
    );
}

#[test]
fn diff_of_identical_snapshots_says_so() {
    let dir = tempfile::tempdir().unwrap();
    let a = write(&dir, "a.json", &snapshot(&["c:\\x"], None));
    let out = idiff(&["diff", &a, &a]);
    assert!(out.status.success());
    assert_eq!(stdout(&out).trim(), copy::NO_CHANGES);
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
        &["run", "--", "winget", "install", "x"][..],
        &["leftovers", "last"],
        &["show", "last", "--all"],
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
    let out = idiff(&["snapshot", "-o", &path.to_string_lossy()]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stdout(&out).is_empty());
    let s = Snapshot::load(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(s.sources.len(), SourceKind::ALL.len());
    let programs = &s.sources[&SourceKind::Programs];
    assert!(programs.ok && !programs.items.is_empty(), "{:?}", programs.error);
}

#[cfg(windows)]
#[test]
fn snapshot_without_output_prints_it() {
    let out = idiff(&["snapshot"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(Snapshot::load(&stdout(&out)).is_ok());
}
