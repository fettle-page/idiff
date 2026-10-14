#![cfg(windows)]

use idiff_collect::{SnapshotOptions, snapshot};
use idiff_core::model::{SCHEMA, SourceKind};
use std::time::{Duration, Instant};

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
fn a_full_snapshot_reads_every_source_in_time() {
    let start = Instant::now();
    let s = snapshot(&SnapshotOptions::default());
    let took = start.elapsed();
    assert!(took < Duration::from_secs(15), "took {took:?}");
    assert_eq!(s.sources.keys().copied().collect::<Vec<_>>(), SourceKind::ALL);
    for (kind, snap) in &s.sources {
        if *kind == SourceKind::Defender {
            // Only administrators can read it: anyone else gets an error, never an empty list.
            if !s.elevated {
                assert!(!snap.ok && snap.error.is_some() && snap.items.is_empty(), "{snap:?}");
            }
        } else {
            assert!(snap.ok, "{kind:?}: {:?}", snap.error);
        }
    }
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
fn real_snapshot_has_services_and_drivers() {
    let s = snapshot(&SnapshotOptions {
        only: Some(vec![SourceKind::Services, SourceKind::Drivers]),
        ..Default::default()
    });
    let services = &s.sources[&SourceKind::Services];
    let drivers = &s.sources[&SourceKind::Drivers];
    assert!(services.ok, "{:?}", services.error);
    assert!(drivers.ok, "{:?}", drivers.error);
    let has = |items: &[idiff_core::model::Item], key: &str| items.iter().any(|i| i.key == key);
    assert!(has(&services.items, "eventlog"));
    assert!(has(&drivers.items, "tcpip"));
    assert!(!has(&services.items, "tcpip") && !has(&drivers.items, "eventlog"));
    // Per-user instances change at every sign-in, so only their templates are listed.
    assert!(!services.items.iter().any(|i| i.key.starts_with("cdpusersvc_")));
}

#[test]
fn real_snapshot_has_tasks() {
    let s = snapshot(&SnapshotOptions {
        only: Some(vec![SourceKind::Tasks]),
        ..Default::default()
    });
    let tasks = &s.sources[&SourceKind::Tasks];
    assert!(tasks.ok, "{:?}", tasks.error);
    assert!(tasks.items.len() > 100, "{} tasks", tasks.items.len());
    assert!(tasks.items.iter().any(|i| i.key.starts_with(r"\microsoft\windows\")));
    if !s.elevated {
        assert_eq!(tasks.note.as_deref(), Some(idiff_core::copy::TASKS_PARTIAL));
    }
}

#[test]
fn real_snapshot_has_packaged_apps() {
    let packaged = read_one(SourceKind::Packaged);
    assert!(packaged.ok, "{:?}", packaged.error);
}

#[test]
fn real_snapshot_has_root_certificates() {
    let certs = read_one(SourceKind::Certs);
    assert!(certs.ok, "{:?}", certs.error);
    assert!(!certs.items.is_empty());
}

/// Scheduled tasks, startup shortcuts and Store apps each use COM on their own thread. Read
/// together, a Store app read that hadn't joined COM itself crashed the whole process.
#[test]
fn sources_using_com_can_read_together() {
    for _ in 0..5 {
        let s = snapshot(&SnapshotOptions {
            only: Some(vec![
                SourceKind::Tasks,
                SourceKind::Startup,
                SourceKind::Packaged,
                SourceKind::Folders,
            ]),
            ..Default::default()
        });
        assert!(s.sources.values().all(|v| v.ok), "{:?}", s.sources);
    }
}
