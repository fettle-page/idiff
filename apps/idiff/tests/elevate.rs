use idiff::elevate::{helper_path, merge_helper, parse_helper_args};
use idiff_core::model::{SCHEMA, Snapshot, SourceKind, SourceSnap};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const NONCE: u128 = 0x0123_4567_89ab_cdef_0011_2233_4455_6677;

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|a| a.to_string()).collect()
}

#[test]
fn helper_path_names_pid_and_nonce() {
    let path = helper_path(Path::new(r"C:\Users\a\AppData\Local\idiff"), 4242, NONCE);
    assert_eq!(
        path,
        PathBuf::from(r"C:\Users\a\AppData\Local\idiff\helper-4242-0123456789abcdef0011223344556677.json")
    );
}

#[test]
fn helper_args_are_strict() {
    let good = helper_path(Path::new(r"C:\Users\Niccolò\AppData\Local\idiff"), 7, NONCE);
    let good = good.to_string_lossy().into_owned();
    let parsed = parse_helper_args(&args(&["--helper", &good, "--only", "Tasks,Defender"])).unwrap();
    assert_eq!(
        parsed,
        (PathBuf::from(&good), vec![SourceKind::Tasks, SourceKind::Defender])
    );
    assert!(parse_helper_args(&args(&["--helper", &good, "--only", "Defender"])).is_some());

    let bad_name = r"C:\x\helper-7-0123.json";
    let upper_hex = r"C:\x\helper-7-0123456789ABCDEF0011223344556677.json";
    let escaped = r"C:\x\idiff\..\helper-7-0123456789abcdef0011223344556677.json";
    let relative = r"helper-7-0123456789abcdef0011223344556677.json";
    for wrong in [
        args(&["--helper", &good, "--only", "Tasks", "--extra"]),
        args(&["--helper", &good]),
        args(&["--helper", &good, "--kinds", "Tasks"]),
        args(&["--only", "Tasks", "--helper", &good]),
        args(&["--helper", bad_name, "--only", "Tasks"]),
        args(&["--helper", upper_hex, "--only", "Tasks"]),
        args(&["--helper", escaped, "--only", "Tasks"]),
        args(&["--helper", relative, "--only", "Tasks"]),
        args(&["--helper", &good, "--only", ""]),
        args(&["--helper", &good, "--only", "Tasks,Nope"]),
        // The helper only reads what needs an administrator.
        args(&["--helper", &good, "--only", "Programs"]),
        args(&[]),
    ] {
        assert_eq!(parse_helper_args(&wrong), None, "{wrong:?}");
    }
}

fn snap(sources: &[(SourceKind, SourceSnap)]) -> Snapshot {
    Snapshot {
        schema: SCHEMA,
        idiff_version: "0.1.0".into(),
        taken_at_ms: 1,
        elevated: false,
        env: BTreeMap::new(),
        sources: sources.iter().cloned().collect(),
    }
}

fn failed(error: &str) -> SourceSnap {
    SourceSnap {
        ok: false,
        error: Some(error.into()),
        ..Default::default()
    }
}

#[test]
fn only_the_helpers_good_results_replace_the_unelevated_ones() {
    let partial = SourceSnap {
        ok: true,
        note: Some("partial".into()),
        ..Default::default()
    };
    let mut full = snap(&[
        (
            SourceKind::Programs,
            SourceSnap {
                ok: true,
                ..Default::default()
            },
        ),
        (SourceKind::Tasks, partial.clone()),
        (SourceKind::Defender, failed("needs administrator")),
    ]);
    let complete = SourceSnap {
        ok: true,
        ms: 5,
        ..Default::default()
    };
    // The helper may run as another administrator: its view of user sources must never be used.
    let admins_programs = SourceSnap {
        ok: true,
        ms: 99,
        ..Default::default()
    };
    let helper = snap(&[
        (SourceKind::Tasks, failed("boom")),
        (SourceKind::Defender, complete.clone()),
        (SourceKind::Programs, admins_programs),
    ]);
    merge_helper(&mut full, helper);
    assert_eq!(full.sources[&SourceKind::Tasks], partial);
    assert_eq!(full.sources[&SourceKind::Defender], complete);
    assert_eq!(full.sources[&SourceKind::Programs].ms, 0);
}

#[cfg(windows)]
mod windows_only {
    use super::NONCE;
    use idiff::elevate::{helper_path, run_helper, snapshot_for_run};
    use idiff_core::model::{Snapshot, SourceKind};

    #[test]
    fn helper_refuses_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = helper_path(dir.path(), 1, NONCE);
        let existing = tempfile::Builder::new()
            .tempfile_in(dir.path())
            .unwrap()
            .into_temp_path();
        std::fs::rename(&existing, &path).unwrap();
        assert_eq!(run_helper(&path, &[SourceKind::Defender]), 1);
        assert_eq!(
            std::fs::metadata(&path).unwrap().len(),
            0,
            "the existing file is untouched"
        );
    }

    #[test]
    fn helper_writes_a_snapshot_of_what_it_was_asked_for() {
        let dir = tempfile::tempdir().unwrap();
        let path = helper_path(dir.path(), 2, NONCE);
        assert_eq!(run_helper(&path, &[SourceKind::Defender]), 0);
        let written = Snapshot::load(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(written.sources.keys().collect::<Vec<_>>(), [&SourceKind::Defender]);
    }

    #[test]
    fn no_elevate_gives_the_plain_snapshot() {
        let s = snapshot_for_run(true);
        assert_eq!(s.sources.len(), SourceKind::ALL.len());
    }
}
