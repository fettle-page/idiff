#![cfg(windows)]

use idiff_collect::{SnapshotOptions, snapshot};
use idiff_core::model::{SCHEMA, SourceKind};

#[test]
fn real_snapshot_has_programs() {
    let s = snapshot(&SnapshotOptions {
        only: Some(vec![SourceKind::Programs]),
        ..Default::default()
    });
    assert_eq!(s.schema, SCHEMA);
    assert_eq!(s.sources.keys().collect::<Vec<_>>(), [&SourceKind::Programs]);
    let programs = &s.sources[&SourceKind::Programs];
    assert!(programs.ok, "{:?}", programs.error);
    assert!(!programs.items.is_empty());
    assert!(s.env.contains_key("SystemRoot"));
}

#[test]
fn sources_not_built_yet_are_failed_not_empty() {
    let s = snapshot(&SnapshotOptions {
        only: Some(vec![SourceKind::Services]),
        ..Default::default()
    });
    let services = &s.sources[&SourceKind::Services];
    assert!(!services.ok && services.items.is_empty());
    assert_eq!(services.error.as_deref(), Some("not built yet"));
}

fn read_one(kind: SourceKind) -> idiff_core::model::SourceSnap {
    let mut s = snapshot(&SnapshotOptions {
        only: Some(vec![kind]),
        ..Default::default()
    });
    s.sources.remove(&kind).unwrap()
}

#[test]
fn real_snapshot_has_startup_path_shell_extensions_and_folders() {
    for kind in [
        SourceKind::Startup,
        SourceKind::Path,
        SourceKind::ShellExt,
        SourceKind::Folders,
    ] {
        let snap = read_one(kind);
        assert!(snap.ok, "{kind:?}: {:?}", snap.error);
        assert!(!snap.items.is_empty(), "{kind:?} has no items");
    }
}

#[test]
fn real_snapshot_has_firewall_rules_and_network() {
    let firewall = read_one(SourceKind::Firewall);
    assert!(firewall.ok, "{:?}", firewall.error);
    assert!(!firewall.items.is_empty());
    let network = read_one(SourceKind::Network);
    assert!(network.ok, "{:?}", network.error);
}

#[test]
fn real_snapshot_has_packaged_apps() {
    let packaged = read_one(SourceKind::Packaged);
    assert!(packaged.ok, "{:?}", packaged.error);
}
