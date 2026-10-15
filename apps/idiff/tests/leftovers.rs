use idiff::leftovers::leftovers_from_files;
use idiff::session::{self, Session};
use idiff_core::copy;
use idiff_core::model::{Item, Snapshot, SourceKind};
use idiff_core::noise::NoiseList;
use idiff_core::verdict::{Level, Mode, Receipt};
use std::cell::RefCell;
use std::sync::{Mutex, MutexGuard};

/// `IDIFF_HOME` is process-wide, so tests that set it take turns.
static HOME_LOCK: Mutex<()> = Mutex::new(());

struct TempHome {
    _dir: tempfile::TempDir,
    _lock: MutexGuard<'static, ()>,
}

fn temp_home() -> TempHome {
    let lock = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    // SAFETY: every test that reads or writes IDIFF_HOME holds HOME_LOCK.
    unsafe { std::env::set_var("IDIFF_HOME", dir.path()) };
    TempHome { _dir: dir, _lock: lock }
}

fn fixture(name: &str) -> Snapshot {
    let text = match name {
        "before" => include_str!("../../../crates/idiff-core/tests/fixtures/discord-before.json"),
        _ => include_str!("../../../crates/idiff-core/tests/fixtures/discord-after.json"),
    };
    Snapshot::load(text).unwrap()
}

/// A finished Discord install session, as `run` leaves it.
fn discord_session() -> Session {
    let s = session::create("Discord").unwrap();
    s.write_json("before.json", &fixture("before")).unwrap();
    s.write_json("after.json", &fixture("after")).unwrap();
    s
}

/// The uninstaller removed the program and the startup entry, but not the task or the folder.
fn uninstalled_leaving_task_and_folder() -> Snapshot {
    let (before, after) = (fixture("before"), fixture("after"));
    let mut uninstalled = before.clone();
    for kind in [SourceKind::Tasks, SourceKind::Folders] {
        let kept: Vec<Item> = after.sources[&kind]
            .items
            .iter()
            .filter(|i| i.key == r"\discordupdate" || i.key.ends_with(r"\appdata\local\discord"))
            .cloned()
            .collect();
        uninstalled.sources.get_mut(&kind).unwrap().items.extend(kept);
    }
    uninstalled
}

fn texts(r: &Receipt) -> Vec<(Level, &str)> {
    r.lines.iter().map(|l| (l.level, l.text.as_str())).collect()
}

#[test]
fn leftovers_from_session_files_list_what_is_still_there() {
    let _home = temp_home();
    let s = discord_session();
    let measured = RefCell::new(Vec::new());
    let size_of = |key: &str| {
        measured.borrow_mut().push(key.to_string());
        340 * 1024 * 1024
    };
    let result = leftovers_from_files(
        &s,
        &uninstalled_leaving_task_and_folder(),
        &NoiseList::builtin(),
        &size_of,
    )
    .unwrap();
    // Discord's own entry is gone, so these are real leftovers (Squirrel leaves its data behind).
    assert!(!result.still_listed);
    let r = result.receipt;
    assert_eq!(r.mode, Mode::Leftovers);
    assert_eq!(r.title, "Discord 1.0.9170: after uninstall, 2 leftovers");
    assert_eq!(
        texts(&r),
        [
            (Level::Bad, "Scheduled task \"DiscordUpdate\" is still there"),
            (Level::Bad, r"340 MB in AppData\Local\Discord"),
        ]
    );
    // Only the app's folder that's still there is measured.
    assert_eq!(measured.into_inner(), [r"c:\users\a\appdata\local\discord"]);
}

#[test]
fn what_the_uninstaller_added_for_the_app_is_noted() {
    let _home = temp_home();
    let s = discord_session();
    let mut uninstalled = fixture("before");
    uninstalled
        .sources
        .get_mut(&SourceKind::Tasks)
        .unwrap()
        .items
        .push(Item {
            key: r"\discordcleanup".into(),
            label: r"\DiscordCleanup".into(),
            target: Some(r"c:\users\a\appdata\local\discord\cleanup.exe".into()),
            props: Default::default(),
        });
    let r = leftovers_from_files(&s, &uninstalled, &NoiseList::builtin(), &|_| 0)
        .unwrap()
        .receipt;
    assert_eq!(r.title, "Discord 1.0.9170: after uninstall, no leftovers");
    assert_eq!(
        texts(&r),
        [
            (Level::Note, "The uninstaller added scheduled task \"DiscordCleanup\""),
            (Level::Good, "Nothing left behind"),
        ]
    );
}

/// A real uninstall always removes the app's own entry in Installed programs. Still there means
/// the uninstall didn't run or finish (EA app on a runner), not that everything is a leftover.
#[test]
fn an_app_still_listed_after_the_uninstall_is_said_so_first() {
    let _home = temp_home();
    let s = discord_session();
    // Nothing was removed: the "uninstalled" PC is the PC right after the install.
    let result = leftovers_from_files(&s, &fixture("after"), &NoiseList::builtin(), &|_| 0).unwrap();
    assert!(result.still_listed);
    let first = &result.receipt.lines[0];
    assert_eq!(
        (first.level, first.text.as_str()),
        (Level::Warn, copy::UNINSTALL_DIDNT_FINISH)
    );
    // The rest of the receipt is still there to read.
    assert!(result.receipt.lines.len() > 1);
}

#[test]
fn an_unfinished_session_has_no_leftovers_check() {
    let _home = temp_home();
    let s = session::create("Discord").unwrap();
    s.write_json("before.json", &fixture("before")).unwrap();
    let err = leftovers_from_files(&s, &fixture("before"), &NoiseList::builtin(), &|_| 0).unwrap_err();
    assert_eq!(err, copy::SESSION_UNFINISHED);
}

/// Runtimes an installer brings along (Epic's Microsoft GameInput, NordVPN's .NET) often stay after
/// the app's own uninstall. That's a leftover, not a sign the uninstall didn't run: only the app's
/// own entry counts.
#[test]
fn a_runtime_left_behind_doesnt_mean_the_app_is_still_installed() {
    let _home = temp_home();
    let runtime = Item {
        key: r"hklm\native\{runtime}".into(),
        label: "Microsoft .NET Runtime - 8.0.32 (x64)".into(),
        props: [
            ("version".to_string(), "8.0.32".to_string()),
            ("size_kb".to_string(), "1".to_string()),
        ]
        .into(),
        ..Default::default()
    };
    let s = session::create("Discord").unwrap();
    let mut after = fixture("after");
    after
        .sources
        .get_mut(&SourceKind::Programs)
        .unwrap()
        .items
        .push(runtime.clone());
    s.write_json("before.json", &fixture("before")).unwrap();
    s.write_json("after.json", &after).unwrap();
    let mut uninstalled = fixture("before");
    uninstalled
        .sources
        .get_mut(&SourceKind::Programs)
        .unwrap()
        .items
        .push(runtime);

    let result = leftovers_from_files(&s, &uninstalled, &NoiseList::builtin(), &|_| 0).unwrap();
    assert!(!result.still_listed, "{:?}", texts(&result.receipt));
    assert!(
        texts(&result.receipt)
            .iter()
            .any(|(level, text)| *level == Level::Bad && text.contains(".NET Runtime"))
    );
}
