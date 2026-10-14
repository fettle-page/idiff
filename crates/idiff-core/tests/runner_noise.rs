//! The catalog's runner-only noise list (`catalog/runner-noise.toml`) loads with idiff's own
//! parser and hides only what GitHub's runner image adds by itself.

use idiff_core::model::{Item, SourceKind};
use idiff_core::noise::NoiseList;
use idiff_core::subject::Subject;

fn runner_noise() -> NoiseList {
    NoiseList::parse(include_str!("../../../catalog/runner-noise.toml")).unwrap_or_else(|e| panic!("{e}"))
}

fn folder(key: &str) -> Item {
    Item {
        key: key.into(),
        ..Default::default()
    }
}

#[test]
fn the_preinstalled_firefox_folders_are_runner_noise() {
    let noise = runner_noise();
    let subject = Subject::default();
    for key in [
        r"c:\users\runneradmin\appdata\local\mozilla",
        r"c:\users\runneradmin\appdata\roaming\mozilla",
    ] {
        let note = noise.matches(SourceKind::Folders, &folder(key), &subject);
        assert_eq!(note, Some("GitHub runner: preinstalled Firefox"), "{key}");
    }
}

#[test]
fn nothing_else_is() {
    let noise = runner_noise();
    let subject = Subject::default();
    for key in [
        r"c:\users\runneradmin\appdata\local\mozilla firefox",
        r"c:\users\runneradmin\appdata\local\mozilla\firefox",
        r"c:\users\runneradmin\appdata\local\thunderbird",
        r"c:\program files\mozilla firefox",
        r"c:\users\runneradmin\appdata\locallow\mozilla",
    ] {
        assert_eq!(
            noise.matches(SourceKind::Folders, &folder(key), &subject),
            None,
            "{key}"
        );
    }
    // Only folders: a task or service named after Mozilla is still looked at.
    let task = Item {
        key: r"\mozilla\firefox default browser agent".into(),
        ..Default::default()
    };
    assert_eq!(noise.matches(SourceKind::Tasks, &task, &subject), None);
}

#[test]
fn mozilla_apps_keep_their_own_folders() {
    let thunderbird = Subject {
        name: "Thunderbird".into(),
        publisher: Some("Mozilla".into()),
        ..Default::default()
    };
    let key = r"c:\users\runneradmin\appdata\local\mozilla";
    assert_eq!(
        runner_noise().matches(SourceKind::Folders, &folder(key), &thunderbird),
        None
    );
}
