use idiff_core::diff::Change;
use idiff_core::model::{Item, SourceKind};
use idiff_core::noise::{NoiseList, exact_rules};
use idiff_core::subject::Subject;

fn item(key: &str, target: Option<&str>, props: &[(&str, &str)]) -> Item {
    Item {
        key: key.into(),
        label: key.into(),
        target: target.map(String::from),
        props: props.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
    }
}

fn nobody() -> Subject {
    Subject {
        name: "Discord".into(),
        publisher: Some("Discord Inc.".into()),
        ..Default::default()
    }
}

#[test]
fn builtin_list_covers_each_kind_of_noise() {
    let noise = NoiseList::builtin();
    let s = nobody();
    let hit = |kind, key: &str, target: Option<&str>, props: &[(&str, &str)]| {
        noise.matches(kind, &item(key, target, props), &s).is_some()
    };
    assert!(hit(
        SourceKind::Tasks,
        r"\microsoft\windows\defrag\scheduleddefrag",
        None,
        &[]
    ));
    assert!(hit(
        SourceKind::Tasks,
        r"\onedrive standalone update task-s-1-5-21-1-2-3-1001",
        None,
        &[]
    ));
    assert!(hit(SourceKind::Drivers, "mpksl1a2b3c4d", None, &[]));
    assert!(hit(SourceKind::Services, "windefend", None, &[]));
    assert!(hit(
        SourceKind::Startup,
        r"run|hkcu|software\microsoft\windows\currentversion\run|onedrive",
        None,
        &[]
    ));
    assert!(hit(
        SourceKind::Folders,
        r"c:\users\niccolò\appdata\local\temp",
        None,
        &[]
    ));
    assert!(hit(
        SourceKind::Folders,
        r"c:\users\a\appdata\local\peerdistrepub",
        None,
        &[]
    ));
    assert!(hit(
        SourceKind::Packaged,
        "microsoft.winget.source_8wekyb3d8bbwe",
        None,
        &[]
    ));
    assert!(hit(
        SourceKind::Firewall,
        "{x}",
        Some(r"c:\program files\windowsapps\a_1_x64__b\a.exe"),
        &[]
    ));
    assert!(hit(
        SourceKind::Firewall,
        "{y}",
        None,
        &[("pfn", "microsoft.windowsstore_8wekyb3d8bbwe")]
    ));

    assert!(!hit(SourceKind::Tasks, r"\discordupdate", None, &[]));
    assert!(!hit(
        SourceKind::Folders,
        r"c:\users\a\appdata\local\discord",
        None,
        &[]
    ));
    // Under a noise folder, but not the folder itself.
    assert!(!hit(
        SourceKind::Folders,
        r"c:\users\a\appdata\roaming\microsoft\windows\start menu\programs\x.lnk",
        None,
        &[]
    ));
    assert!(!hit(
        SourceKind::Firewall,
        "{z}",
        None,
        &[("pfn", "spotifyab.spotifymusic_zpdnekdrzrea0")]
    ));
    // A rule for one source never matches another.
    assert!(!hit(SourceKind::Services, "mpksl1a2b3c4d", None, &[]));
}

#[test]
fn only_changed_rules_need_a_change_in_just_those_fields() {
    let noise = NoiseList::builtin();
    let s = nobody();
    let ms = "CN=Microsoft Corporation, O=Microsoft Corporation, L=Redmond, S=Washington, C=US";
    let pkg = |version: &str, publisher: &str| {
        item(
            "microsoft.windowsstore_8wekyb3d8bbwe",
            Some(version),
            &[("version", version), ("publisher", publisher)],
        )
    };
    let updated = Change::Modified {
        before: pkg("1", ms),
        after: pkg("2", ms),
        fields: vec!["target".into(), "version".into()],
    };
    assert!(noise.matches_change(SourceKind::Packaged, &updated, &s).is_some());
    // The item alone doesn't say it only changed version.
    assert!(noise.matches(SourceKind::Packaged, &pkg("2", ms), &s).is_none());
    assert!(
        noise
            .matches_change(SourceKind::Packaged, &Change::Added(pkg("2", ms)), &s)
            .is_none()
    );
    let other = Change::Modified {
        before: pkg("1", ms),
        after: pkg("2", ms),
        fields: vec!["extensions".into(), "version".into()],
    };
    assert!(noise.matches_change(SourceKind::Packaged, &other, &s).is_none());
    let spotify = Change::Modified {
        before: pkg("1", "CN=Spotify AB, O=Spotify AB"),
        after: pkg("2", "CN=Spotify AB, O=Spotify AB"),
        fields: vec!["version".into()],
    };
    assert!(noise.matches_change(SourceKind::Packaged, &spotify, &s).is_none());
}

#[test]
fn bad_rules_are_errors() {
    for bad in [
        "[[rule]]\nsource = \"Tasks\"\nkey = '('\nnote = \"x\"",
        "[[rule]]\nsource = \"Nope\"\nkey = 'a'\nnote = \"x\"",
        "[[rule]]\nsource = \"Tasks\"\nnote = \"matches everything\"",
        "[[rule]]\nsource = \"Tasks\"\nkey = 'a'\nnote = \"x\"\nkye = 'typo'",
        "[[rule]]\nsource = \"Tasks\"\nkey = 'a'",
        "not toml at all [",
    ] {
        assert!(NoiseList::parse(bad).is_err(), "{bad}");
    }
    assert!(NoiseList::parse("").is_ok());
}

#[test]
fn merged_adds_local_rules() {
    let local = NoiseList::parse("[[rule]]\nsource = \"Services\"\nkey = '^foosvc$'\nnote = \"mine\"").unwrap();
    let noise = NoiseList::builtin().merged(local);
    let s = nobody();
    assert_eq!(
        noise.matches(SourceKind::Services, &item("foosvc", None, &[]), &s),
        Some("mine")
    );
    assert!(
        noise
            .matches(SourceKind::Drivers, &item("mpksl1", None, &[]), &s)
            .is_some()
    );
}

#[test]
fn exact_rules_round_trip() {
    let keys = vec![
        (
            SourceKind::Folders,
            r"c:\users\niccolò's pc\appdata\local\a+b (x)".to_string(),
        ),
        (SourceKind::Tasks, r#"\say "hi" [now]"#.to_string()),
    ];
    let text = exact_rules(&keys);
    let noise = NoiseList::parse(&text).unwrap_or_else(|e| panic!("{e}\n{text}"));
    let s = nobody();
    for (kind, key) in &keys {
        assert_eq!(noise.matches(*kind, &item(key, None, &[]), &s), Some("calibrate"));
        // Exact: neither a longer key nor the same key in another source.
        assert!(noise.matches(*kind, &item(&format!("{key}x"), None, &[]), &s).is_none());
    }
    assert!(
        noise
            .matches(SourceKind::Services, &item(&keys[1].1, None, &[]), &s)
            .is_none()
    );
    assert!(NoiseList::parse(&exact_rules(&[])).is_ok());
}
