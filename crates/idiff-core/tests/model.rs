use idiff_core::copy;
use idiff_core::model::{Item, SCHEMA, Snapshot, SourceKind, SourceSnap};
use std::collections::BTreeMap;

fn fixture_snapshot() -> Snapshot {
    let item = Item {
        key: "steam client service".into(),
        label: "Steam Client Service".into(),
        target: Some(r"c:\program files (x86)\common files\steam\steamservice.exe".into()),
        props: BTreeMap::from([("start".into(), "3".into()), ("account".into(), "LocalSystem".into())]),
    };
    let mut sources = BTreeMap::new();
    sources.insert(
        SourceKind::Services,
        SourceSnap {
            ok: true,
            ms: 12,
            items: vec![item],
            ..Default::default()
        },
    );
    Snapshot {
        schema: SCHEMA,
        idiff_version: "0.1.0".into(),
        taken_at_ms: 1_791_000_000_000,
        elevated: false,
        env: BTreeMap::from([("SystemRoot".into(), r"C:\Windows".into())]),
        sources,
    }
}

#[test]
fn snapshot_round_trips() {
    let s = fixture_snapshot();
    assert_eq!(Snapshot::load(&s.to_json()).unwrap(), s);
}

#[test]
fn load_rejects_other_schema_and_garbage() {
    let mut s = fixture_snapshot();
    s.schema = 2;
    assert_eq!(Snapshot::load(&s.to_json()).unwrap_err(), copy::SNAPSHOT_TOO_NEW);
    assert!(
        Snapshot::load("{not json")
            .unwrap_err()
            .starts_with(copy::SNAPSHOT_UNREADABLE)
    );
    assert!(
        Snapshot::load(r#"{"schema": 1}"#)
            .unwrap_err()
            .starts_with(copy::SNAPSHOT_UNREADABLE)
    );
}

#[test]
fn only_tasks_and_defender_need_admin() {
    let admin: Vec<_> = SourceKind::ALL.iter().filter(|k| k.needs_admin()).collect();
    assert_eq!(admin, [&SourceKind::Tasks, &SourceKind::Defender]);
}

#[test]
fn every_source_has_a_label() {
    assert!(SourceKind::ALL.iter().all(|k| !k.label().is_empty()));
    assert_eq!(SourceKind::Tasks.label(), "scheduled tasks");
}
