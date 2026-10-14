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
        only: Some(vec![SourceKind::Network]),
        ..Default::default()
    });
    let network = &s.sources[&SourceKind::Network];
    assert!(!network.ok && network.items.is_empty());
    assert_eq!(network.error.as_deref(), Some("not built yet"));
}
