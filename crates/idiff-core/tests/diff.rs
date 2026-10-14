use idiff_core::copy;
use idiff_core::diff::{Change, diff};
use idiff_core::model::{Item, SCHEMA, Snapshot, SourceKind, SourceSnap};
use std::collections::BTreeMap;

fn item(key: &str, props: &[(&str, &str)]) -> Item {
    Item {
        key: key.into(),
        label: key.into(),
        target: None,
        props: props.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
    }
}

fn snap_of(items: Vec<Item>) -> Snapshot {
    let mut sources = BTreeMap::new();
    sources.insert(
        SourceKind::Services,
        SourceSnap {
            ok: true,
            items,
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

/// One `Services` source with those keys and props.
fn snap(items: &[(&str, &[(&str, &str)])]) -> Snapshot {
    snap_of(items.iter().map(|(k, p)| item(k, p)).collect())
}

#[test]
fn identical_snapshots_give_an_empty_delta() {
    let s = snap(&[("svc-a", &[])]);
    let d = diff(&s, &s);
    assert!(d.sources.is_empty() && d.notes.is_empty());
}

#[test]
fn added_removed_and_modified_items_are_found() {
    let b = snap(&[("a", &[("Start", "3")]), ("gone", &[])]);
    let a = snap(&[("a", &[("Start", "2")]), ("new", &[])]);
    let c = &diff(&b, &a).sources[&SourceKind::Services];
    assert_eq!(c.len(), 3);
    assert!(matches!(&c[0], Change::Modified { fields, .. } if fields == &["Start"]));
    assert!(matches!(&c[1], Change::Removed(i) if i.key == "gone"));
    assert!(matches!(&c[2], Change::Added(i) if i.key == "new"));
}

#[test]
fn a_failed_source_is_noted_not_empty() {
    let b = snap(&[("a", &[])]);
    let mut a = snap(&[]);
    a.sources.insert(
        SourceKind::Services,
        SourceSnap {
            ok: false,
            error: Some("access denied".into()),
            ..Default::default()
        },
    );
    let d = diff(&b, &a);
    assert!(d.sources.is_empty());
    assert_eq!(d.notes[&SourceKind::Services], "access denied");
}

#[test]
fn unicode_keys_diff_cleanly() {
    let b = snap(&[]);
    let a = snap(&[(r"c:\users\niccolò\appdata\local\à", &[])]);
    assert_eq!(diff(&b, &a).sources[&SourceKind::Services].len(), 1);
}

#[test]
fn the_after_error_wins_over_the_before_error() {
    let mut b = snap(&[]);
    let mut a = snap(&[]);
    b.sources.get_mut(&SourceKind::Services).unwrap().ok = false;
    b.sources.get_mut(&SourceKind::Services).unwrap().error = Some("before".into());
    assert_eq!(diff(&b, &a).notes[&SourceKind::Services], "before");
    a.sources.get_mut(&SourceKind::Services).unwrap().ok = false;
    a.sources.get_mut(&SourceKind::Services).unwrap().error = Some("after".into());
    assert_eq!(diff(&b, &a).notes[&SourceKind::Services], "after");
}

#[test]
fn a_partial_source_keeps_its_changes_and_its_note() {
    let b = snap(&[]);
    let mut a = snap(&[("new", &[])]);
    a.sources.get_mut(&SourceKind::Services).unwrap().note = Some("not elevated".into());
    let d = diff(&b, &a);
    assert_eq!(d.sources[&SourceKind::Services].len(), 1);
    assert_eq!(d.notes[&SourceKind::Services], "not elevated");
}

#[test]
fn a_source_missing_from_one_snapshot_is_noted_not_diffed() {
    let b = snap(&[("a", &[])]);
    let mut a = snap(&[]);
    a.sources.clear();
    let d = diff(&b, &a);
    assert!(d.sources.is_empty());
    assert_eq!(d.notes[&SourceKind::Services], copy::SOURCE_NOT_READ);
}

#[test]
fn a_duplicate_key_keeps_the_first_item() {
    let b = snap(&[("a", &[("Start", "3")])]);
    let a = snap(&[("a", &[("Start", "3")]), ("a", &[("Start", "2")])]);
    assert!(diff(&b, &a).sources.is_empty());
}

#[test]
fn fields_name_label_target_and_props_sorted() {
    let mut before = item("a", &[("Start", "3"), ("Type", "16"), ("Gone", "x")]);
    let mut after = item("a", &[("Start", "2"), ("Type", "16"), ("Account", "LocalSystem")]);
    before.target = Some(r"c:\old.exe".into());
    after.target = Some(r"c:\new.exe".into());
    after.label = "A service".into();
    let d = diff(&snap_of(vec![before.clone()]), &snap_of(vec![after.clone()]));
    let c = &d.sources[&SourceKind::Services];
    assert_eq!(c.len(), 1);
    match &c[0] {
        Change::Modified {
            before: b,
            after: a,
            fields,
        } => {
            assert_eq!(fields, &["Account", "Gone", "Start", "label", "target"]);
            assert_eq!((b, a), (&before, &after));
        }
        other => panic!("expected Modified, got {other:?}"),
    }
    assert_eq!(c[0].item(), &after);
}
