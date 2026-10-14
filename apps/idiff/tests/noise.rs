use idiff::noise::{LOCAL_FILE, load};
use idiff_core::copy;
use idiff_core::model::{Item, SourceKind};
use idiff_core::noise::exact_rules;
use idiff_core::subject::Subject;
use std::io::Write;
use std::path::Path;

fn item(key: &str) -> Item {
    Item {
        key: key.into(),
        ..Default::default()
    }
}

fn nobody() -> Subject {
    Subject::default()
}

fn append(home: &Path, text: &str) {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(home.join(LOCAL_FILE))
        .unwrap();
    file.write_all(text.as_bytes()).unwrap();
}

#[test]
fn without_a_local_file_the_builtin_list_is_used() {
    let home = tempfile::tempdir().unwrap();
    let (noise, warning) = load(home.path());
    assert_eq!(warning, None);
    assert!(noise.matches(SourceKind::Drivers, &item("mpksl1"), &nobody()).is_some());
    // A home that doesn't exist yet is the same as an empty one.
    assert_eq!(load(&home.path().join("not-yet")).1, None);
}

#[test]
fn saved_rules_are_added_and_can_be_appended_again() {
    let home = tempfile::tempdir().unwrap();
    // `calibrate --save` twice, as two runs would.
    append(home.path(), &exact_rules(&[(SourceKind::Services, "foosvc".into())]));
    append(home.path(), &exact_rules(&[(SourceKind::Tasks, r"\bar task".into())]));
    let (noise, warning) = load(home.path());
    assert_eq!(warning, None);
    let s = nobody();
    assert!(noise.matches(SourceKind::Services, &item("foosvc"), &s).is_some());
    assert!(noise.matches(SourceKind::Tasks, &item(r"\bar task"), &s).is_some());
    assert!(noise.matches(SourceKind::Services, &item("foosvc2"), &s).is_none());
    assert!(
        noise.matches(SourceKind::Drivers, &item("mpksl1"), &s).is_some(),
        "built-in rules stay"
    );
}

#[test]
fn bad_local_noise_file_falls_back() {
    let home = tempfile::tempdir().unwrap();
    append(home.path(), &exact_rules(&[(SourceKind::Services, "foosvc".into())]));
    append(
        home.path(),
        "[[rule]]\nsource = \"Tasks\"\nkey = '('\nnote = \"broken\"\n",
    );
    let (noise, warning) = load(home.path());
    let warning = warning.expect("a warning");
    assert!(warning.starts_with(copy::NOISE_LOCAL_BAD), "{warning}");
    assert!(warning.contains(LOCAL_FILE), "{warning}");
    let s = nobody();
    // None of the local file is used, not even its good rules; the built-in list still is.
    assert!(noise.matches(SourceKind::Services, &item("foosvc"), &s).is_none());
    assert!(noise.matches(SourceKind::Drivers, &item("mpksl1"), &s).is_some());
}
