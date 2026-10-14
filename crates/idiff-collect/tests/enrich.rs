#![cfg(windows)]

use idiff_collect::enrich::{company_of, enrich, signer_of};
use idiff_core::diff::{Change, Delta};
use idiff_core::model::{Item, SourceKind};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

fn system32(file: &str) -> PathBuf {
    PathBuf::from(std::env::var("SystemRoot").unwrap())
        .join("System32")
        .join(file)
}

/// A closed file of `n` bytes: while a file is open, its folder listing can show an old size.
fn file_of(dir: &Path, n: usize, suffix: &str) -> tempfile::TempPath {
    let mut f = tempfile::Builder::new().suffix(suffix).tempfile_in(dir).unwrap();
    f.write_all(&vec![b'x'; n]).unwrap();
    f.into_temp_path()
}

fn item(key: &str, target: Option<&Path>, props: &[(&str, &str)]) -> Item {
    Item {
        key: key.into(),
        label: key.into(),
        target: target.map(|t| t.display().to_string().to_lowercase()),
        props: props.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
    }
}

#[test]
fn signer_of_notepad_is_microsoft() {
    let signer = signer_of(&system32("notepad.exe")).unwrap();
    assert!(signer.contains("Microsoft"), "{signer}");
}

#[test]
fn an_unsigned_file_has_no_signer() {
    let dir = tempfile::tempdir().unwrap();
    let file = file_of(dir.path(), 64, ".exe");
    assert_eq!(signer_of(&file), None);
    assert_eq!(signer_of(&dir.path().join("missing.exe")), None);
}

#[test]
fn company_comes_from_the_file_s_version_details() {
    for file in ["notepad.exe", "kernel32.dll", r"drivers\tcpip.sys"] {
        assert_eq!(
            company_of(&system32(file)).as_deref(),
            Some("Microsoft Corporation"),
            "{file}"
        );
    }
    // No version resource, or no file at all: no company, and no error.
    let dir = tempfile::tempdir().unwrap();
    let plain = file_of(dir.path(), 64, ".exe");
    assert_eq!(company_of(&plain), None);
    assert_eq!(company_of(&dir.path().join("missing.exe")), None);
}

#[test]
fn enrich_sets_signer_and_size() {
    let dir = tempfile::tempdir().unwrap();
    let added_folder = tempfile::tempdir_in(dir.path()).unwrap();
    let inner = tempfile::tempdir_in(added_folder.path()).unwrap();
    let _files = [
        file_of(added_folder.path(), 10, ".dat"),
        file_of(inner.path(), 15, ".dat"),
    ];
    let folder_key = added_folder.path().display().to_string().to_lowercase();
    let notepad = system32("notepad.exe");

    let mut delta = Delta {
        sources: BTreeMap::from([
            (
                SourceKind::Services,
                vec![Change::Added(item("svc", Some(&notepad), &[]))],
            ),
            (
                SourceKind::Folders,
                vec![
                    Change::Added(item(&folder_key, None, &[("root", "ProgramData"), ("kind", "folder")])),
                    Change::Added(item(r"c:\x\app.lnk", Some(&notepad), &[("kind", "shortcut")])),
                ],
            ),
        ]),
        notes: BTreeMap::new(),
        partial: Default::default(),
    };
    enrich(&mut delta);

    let svc = delta.sources[&SourceKind::Services][0].item();
    assert!(svc.props["signer"].contains("Microsoft"), "{svc:?}");
    assert_eq!(svc.props["company"], "Microsoft Corporation");
    let folders = &delta.sources[&SourceKind::Folders];
    let folder = folders[0].item();
    assert_eq!(
        (folder.props["bytes"].as_str(), folder.props["files"].as_str()),
        ("25", "2")
    );
    assert_eq!(folder.props.get("signer"), None);
    let shortcut = folders[1].item();
    assert!(
        shortcut.props.contains_key("signer") && !shortcut.props.contains_key("bytes"),
        "{shortcut:?}"
    );
}

#[test]
fn modified_items_are_enriched_and_removed_ones_are_not() {
    let notepad = system32("notepad.exe");
    let before = item("svc", Some(&notepad), &[("start", "3")]);
    let after = item("svc", Some(&notepad), &[("start", "2")]);
    let mut delta = Delta {
        sources: BTreeMap::from([(
            SourceKind::Services,
            vec![
                Change::Modified {
                    before,
                    after,
                    fields: vec!["start".into()],
                },
                Change::Removed(item("gone", Some(&notepad), &[])),
            ],
        )]),
        notes: BTreeMap::new(),
        partial: Default::default(),
    };
    enrich(&mut delta);
    let changes = &delta.sources[&SourceKind::Services];
    assert!(matches!(&changes[0], Change::Modified { after, before, fields }
        if after.props.contains_key("signer") && !before.props.contains_key("signer") && fields == &["start"]));
    assert!(!changes[1].item().props.contains_key("signer"));
}

#[test]
fn a_missing_target_gets_no_signer_and_no_error() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("gone.exe");
    let mut delta = Delta {
        sources: BTreeMap::from([(
            SourceKind::Tasks,
            vec![
                Change::Added(item("task", Some(&missing), &[])),
                Change::Added(item("none", None, &[])),
            ],
        )]),
        notes: BTreeMap::new(),
        partial: Default::default(),
    };
    enrich(&mut delta);
    assert!(
        delta.sources[&SourceKind::Tasks]
            .iter()
            .all(|c| c.item().props.is_empty())
    );
}
