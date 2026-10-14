use idiff_core::attribute::{Attributed, Owner, Reason, attribute};
use idiff_core::diff::{Change, diff};
use idiff_core::model::{Env, Item, Snapshot, SourceKind};
use idiff_core::noise::NoiseList;
use idiff_core::subject::{Subject, find_subject};
use idiff_core::verdict::{Level, Mode, Receipt, format_size, install_receipt, leftovers_receipt, trigger_summary};
use std::collections::{BTreeMap, BTreeSet};

fn load(json: &str) -> Snapshot {
    Snapshot::load(json).unwrap()
}

/// The Discord install, attributed: (before, after, subject, attributed).
fn discord() -> (Snapshot, Snapshot, Subject, Vec<Attributed>) {
    let before = load(include_str!("fixtures/discord-before.json"));
    let after = load(include_str!("fixtures/discord-after.json"));
    let d = diff(&before, &after);
    let subject = find_subject(&d, None, &[]);
    let attributed = attribute(&d, &subject, &NoiseList::builtin(), &after.env);
    (before, after, subject, attributed)
}

fn texts(r: &Receipt) -> Vec<(Level, &str)> {
    r.lines.iter().map(|l| (l.level, l.text.as_str())).collect()
}

fn item(key: &str, label: &str, target: Option<&str>, props: &[(&str, &str)]) -> Item {
    Item {
        key: key.into(),
        label: label.into(),
        target: target.map(String::from),
        props: props.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
    }
}

fn app_added(kind: SourceKind, item: Item) -> Attributed {
    Attributed {
        kind,
        change: Change::Added(item),
        owner: Owner::App,
        reason: Reason::TargetInInstallDir,
    }
}

#[test]
fn discord_receipt_matches_spec() {
    let (_, after, subject, attributed) = discord();
    let r = install_receipt(
        &subject,
        &attributed,
        &BTreeMap::new(),
        &BTreeSet::new(),
        Some(0),
        &after.env,
    );
    assert_eq!(r.mode, Mode::Install);
    assert_eq!(r.title, "Discord 1.0.9170: installed");
    assert_eq!(
        texts(&r),
        [
            (Level::Warn, "1 scheduled task: runs an updater every day"),
            (Level::Warn, "Starts with Windows"),
            (Level::Good, "No services, drivers or firewall rules"),
            (Level::Good, "Files only in your user folder (412 MB)"),
            (Level::Note, "3 Windows background changes hidden (idiff show --all)"),
        ]
    );
    assert_eq!(r.unknown.len(), 1);
    assert_eq!(r.unknown[0].text, "Added service \"Foo Service\"");
    assert_eq!(r.hidden.len(), 3);
    assert!(r.hidden.iter().all(|l| l.why.len() == 1 && !l.why[0].is_empty()));
    // Each line says which items it's about, for `--all` and `--why`.
    let task = &r.lines[0];
    assert_eq!(task.items, [r"\discordupdate"]);
    assert_eq!(task.why.len(), 1);
}

#[test]
fn restart_exit_codes_are_not_failures() {
    let (_, after, subject, attributed) = discord();
    for code in [3010, 1641] {
        let r = install_receipt(
            &subject,
            &attributed,
            &BTreeMap::new(),
            &BTreeSet::new(),
            Some(code),
            &after.env,
        );
        assert_eq!(texts(&r)[0], (Level::Note, "Needs a restart to finish"), "{code}");
        assert!(!r.lines.iter().any(|l| l.text.contains("exited with code")));
    }
    let r = install_receipt(
        &subject,
        &attributed,
        &BTreeMap::new(),
        &BTreeSet::new(),
        Some(1),
        &after.env,
    );
    assert_eq!(texts(&r)[0], (Level::Warn, "The install command exited with code 1"));
    let r = install_receipt(
        &subject,
        &attributed,
        &BTreeMap::new(),
        &BTreeSet::new(),
        None,
        &after.env,
    );
    assert_eq!(texts(&r)[0].1, "1 scheduled task: runs an updater every day");
}

#[test]
fn upgrade_titles_and_modified_items() {
    let (_, after, _, _) = discord();
    // Discord was installed already: the update changed its version and its task's arguments.
    let mut base = after.clone();
    for (kind, key, prop, value) in [
        (SourceKind::Programs, r"hkcu\native\discord", "version", "1.0.9100"),
        (SourceKind::Tasks, r"\discordupdate", "args", "--old"),
    ] {
        let items = &mut base.sources.get_mut(&kind).unwrap().items;
        let entry = items.iter_mut().find(|i| i.key == key).unwrap();
        entry.props.insert(prop.into(), value.into());
    }
    let d = diff(&base, &after);
    let subject = find_subject(&d, None, &[]);
    let attributed = attribute(&d, &subject, &NoiseList::builtin(), &after.env);
    let r = install_receipt(
        &subject,
        &attributed,
        &BTreeMap::new(),
        &BTreeSet::new(),
        Some(0),
        &after.env,
    );
    assert_eq!(r.title, "Discord: updated from 1.0.9100 to 1.0.9170");
    assert!(
        texts(&r).contains(&(Level::Note, "Changes scheduled task \"DiscordUpdate\"")),
        "{r:#?}"
    );
    // The program's own entry changing version is what the title says already.
    assert!(!r.lines.iter().any(|l| l.text.contains("program")), "{r:#?}");
}

#[test]
fn root_certificate_is_bad_and_first() {
    let (_, after, subject, mut attributed) = discord();
    attributed.push(app_added(
        SourceKind::Certs,
        item("lm|abc123", "Evil CA", None, &[("issuer", "Evil CA")]),
    ));
    let r = install_receipt(
        &subject,
        &attributed,
        &BTreeMap::new(),
        &BTreeSet::new(),
        Some(0),
        &after.env,
    );
    assert_eq!(
        texts(&r)[0],
        (Level::Bad, "Adds a trusted root certificate: \"Evil CA\"")
    );
}

#[test]
fn incomplete_sources_are_listed() {
    let (_, after, subject, attributed) = discord();
    let notes = BTreeMap::from([(SourceKind::Defender, "needs administrator".to_string())]);
    let r = install_receipt(&subject, &attributed, &notes, &BTreeSet::new(), Some(0), &after.env);
    let lines = texts(&r);
    let at = lines
        .iter()
        .position(|l| *l == (Level::Note, "Couldn't check antivirus exclusions: needs administrator"))
        .expect("the incomplete source is listed");
    // After the findings, before the hidden count.
    assert_eq!(at, lines.len() - 2);
}

#[test]
fn nothing_found_says_so() {
    let (_, after, subject, _) = discord();
    let r = install_receipt(&subject, &[], &BTreeMap::new(), &BTreeSet::new(), Some(0), &after.env);
    assert_eq!(texts(&r), [(Level::Good, "No lasting changes found")]);
    assert!(r.unknown.is_empty() && r.hidden.is_empty());
}

#[test]
fn leftovers_list_what_is_still_there() {
    let (before, after, subject, install) = discord();
    // The uninstaller removed the program and the startup entry, but not the task or the folder.
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
    let sizes = BTreeMap::from([(r"c:\users\a\appdata\local\discord".to_string(), 340 * 1024 * 1024)]);
    let r = leftovers_receipt(&subject, &install, &uninstalled, &[], &sizes);
    assert_eq!(r.mode, Mode::Leftovers);
    assert_eq!(r.title, "Discord 1.0.9170: after uninstall, 2 leftovers");
    assert_eq!(
        texts(&r),
        [
            (Level::Bad, "Scheduled task \"DiscordUpdate\" is still there"),
            (Level::Bad, r"340 MB in AppData\Local\Discord"),
        ]
    );

    // What the uninstaller itself added for the app is noted.
    let during = vec![app_added(
        SourceKind::Tasks,
        item(r"\discordcleanup", r"\DiscordCleanup", None, &[]),
    )];
    let r = leftovers_receipt(&subject, &install, &before, &during, &BTreeMap::new());
    assert_eq!(r.title, "Discord 1.0.9170: after uninstall, no leftovers");
    assert_eq!(
        texts(&r),
        [
            (Level::Note, "The uninstaller added scheduled task \"DiscordCleanup\""),
            (Level::Good, "Nothing left behind"),
        ]
    );
}

#[test]
fn leftovers_say_when_a_source_couldnt_be_checked() {
    let (before, _, subject, install) = discord();
    let mut uninstalled = before.clone();
    let tasks = uninstalled.sources.get_mut(&SourceKind::Tasks).unwrap();
    tasks.ok = false;
    tasks.error = Some("needs administrator".into());
    tasks.items.clear();
    let r = leftovers_receipt(&subject, &install, &uninstalled, &[], &BTreeMap::new());
    assert!(texts(&r).contains(&(Level::Note, "Couldn't check scheduled tasks: needs administrator")));
    // Unchecked isn't the same as gone.
    assert!(!texts(&r).contains(&(Level::Good, "Nothing left behind")));
    assert_eq!(r.title, "Discord 1.0.9170: after uninstall, no leftovers found");
}

#[test]
fn trigger_summaries() {
    let task = |target: &str, triggers: &str, user: &str| {
        item(r"\t", r"\t", Some(target), &[("triggers", triggers), ("user", user)])
    };
    let update = r"c:\users\a\appdata\local\discord\update.exe";
    assert_eq!(
        trigger_summary(&task(update, "Daily", "S-1-5-21-1")),
        "runs an updater every day"
    );
    assert_eq!(
        trigger_summary(&task(r"c:\x\foo.exe", "Logon", "")),
        "runs foo.exe at sign-in"
    );
    assert_eq!(
        trigger_summary(&task(r"c:\x\foo.exe", "Boot", "S-1-5-18")),
        "runs foo.exe at startup, as SYSTEM"
    );
    assert_eq!(
        trigger_summary(&task(r"c:\x\foo.exe", "Weekly,Logon", "")),
        "runs foo.exe every week"
    );
    assert_eq!(
        trigger_summary(&task(r"c:\x\foo.exe", "Idle", "")),
        "runs foo.exe when the PC is idle"
    );
    assert_eq!(trigger_summary(&task(r"c:\x\foo.exe", "Time", "")), "runs foo.exe once");
    assert_eq!(
        trigger_summary(&task(r"c:\x\foo.exe", "EventTrigger", "")),
        "runs foo.exe on a schedule"
    );
    assert_eq!(
        trigger_summary(&item(r"\t", r"\t", None, &[("triggers", "Logon")])),
        "runs a program at sign-in"
    );
}

#[test]
fn sizes_use_base_1024() {
    for (bytes, want) in [
        (0, "0 KB"),
        (1, "1 KB"),
        (1023 * 1024, "1023 KB"),
        (432_013_312, "412 MB"),
        (1024 * 1024 * 1024 + 512 * 1024 * 1024, "1.5 GB"),
        (12 * 1024 * 1024 * 1024, "12 GB"),
    ] {
        assert_eq!(format_size(bytes), want, "{bytes}");
    }
}

#[test]
fn every_finding_in_spec_order_and_words() {
    let subject = Subject {
        name: "Zoom Workplace".into(),
        version: Some("6.2.0".into()),
        also_installed: vec!["Zoom Outlook Plugin".into()],
        ..Default::default()
    };
    let mut attributed = vec![
        app_added(
            SourceKind::Folders,
            item(
                r"c:\programdata\zoom",
                r"ProgramData\Zoom",
                None,
                &[("root", "ProgramData"), ("kind", "folder"), ("bytes", "2048")],
            ),
        ),
        app_added(
            SourceKind::Folders,
            item(
                r"c:\program files\zoom",
                r"Program Files\Zoom",
                None,
                &[("root", "Program Files"), ("kind", "folder"), ("bytes", "1048576")],
            ),
        ),
        app_added(
            SourceKind::Folders,
            item(
                r"c:\users\a\appdata\roaming\zoom",
                r"AppData\Roaming\Zoom",
                None,
                &[("root", r"AppData\Roaming"), ("kind", "folder"), ("bytes", "3072")],
            ),
        ),
        app_added(
            SourceKind::Folders,
            item(
                r"c:\users\public\desktop\zoom.lnk",
                r"Public Desktop\Zoom.lnk",
                Some(r"c:\program files\zoom\zoom.exe"),
                &[("root", "Public Desktop"), ("kind", "shortcut")],
            ),
        ),
        app_added(
            SourceKind::Folders,
            item(
                r"c:\programdata\microsoft\windows\start menu\programs\zoom\zoom.lnk",
                r"Start menu (all users)\Zoom\Zoom.lnk",
                Some(r"c:\program files\zoom\zoom.exe"),
                &[("root", "Start menu (all users)"), ("kind", "shortcut")],
            ),
        ),
        app_added(
            SourceKind::Path,
            item(
                r"machine|c:\program files\zoom\bin",
                r"C:\Program Files\Zoom\bin",
                Some(r"c:\program files\zoom\bin"),
                &[],
            ),
        ),
        app_added(
            SourceKind::ShellExt,
            item(
                r"*|handler|zoomshare",
                "ZoomShare",
                Some(r"c:\program files\zoom\share.dll"),
                &[],
            ),
        ),
        app_added(
            SourceKind::Firewall,
            item(
                "zoom-out",
                "Zoom",
                Some(r"c:\program files\zoom\updater.exe"),
                &[("dir", "out"), ("action", "allow")],
            ),
        ),
        app_added(
            SourceKind::Firewall,
            item(
                "zoom-in",
                "Zoom",
                Some(r"c:\program files\zoom\zoom.exe"),
                &[("dir", "in"), ("action", "allow")],
            ),
        ),
        app_added(
            SourceKind::Firewall,
            item(
                "zoom-out2",
                "Zoom",
                Some(r"c:\program files\zoom\zoom.exe"),
                &[("dir", "out"), ("action", "allow")],
            ),
        ),
        app_added(
            SourceKind::Packaged,
            item(
                "zoom.plugin_x",
                "Zoom Plugin",
                None,
                &[("extensions", "startup,context-menu")],
            ),
        ),
        app_added(
            SourceKind::Startup,
            item(r"run|hklm|x|zoom", "Zoom", Some(r"c:\program files\zoom\zoom.exe"), &[]),
        ),
        app_added(
            SourceKind::Tasks,
            item(
                r"\zoomupdate",
                r"\ZoomUpdate",
                Some(r"c:\program files\zoom\zoomupdater.exe"),
                &[("triggers", "Daily"), ("user", "S-1-5-18")],
            ),
        ),
        app_added(
            SourceKind::Services,
            item(
                "zoomsvc",
                "Zoom Sharing Service",
                Some(r"c:\program files\zoom\svc.exe"),
                &[("start", "2"), ("account", "LocalSystem")],
            ),
        ),
        app_added(
            SourceKind::Drivers,
            item(
                "zoomaudio",
                "Zoom Audio Device",
                Some(r"c:\windows\system32\drivers\zoomaudio.sys"),
                &[],
            ),
        ),
        app_added(
            SourceKind::Drivers,
            item(
                "zoomcam",
                "Zoom Camera",
                Some(r"c:\windows\system32\drivers\zoomcam.sys"),
                &[],
            ),
        ),
        app_added(
            SourceKind::Network,
            item("hosts|127.0.0.1 zoom.local", "127.0.0.1 zoom.local", None, &[]),
        ),
        app_added(
            SourceKind::Network,
            item("proxy|server", "ProxyServer", None, &[("value", "127.0.0.1:8080")]),
        ),
        app_added(
            SourceKind::Defender,
            item(r"paths|c:\program files\zoom", r"C:\Program Files\Zoom", None, &[]),
        ),
        app_added(SourceKind::Certs, item("lm|1", "Zoom Root CA", None, &[])),
    ];
    attributed.push(Attributed {
        kind: SourceKind::Drivers,
        change: Change::Added(item("mpksl1", "mpksl1", None, &[])),
        owner: Owner::Windows,
        reason: Reason::Noise("Microsoft Defender's per-update driver".into()),
    });
    let r = install_receipt(
        &subject,
        &attributed,
        &BTreeMap::new(),
        &BTreeSet::new(),
        Some(0),
        &Env::new(),
    );
    assert_eq!(r.title, "Zoom Workplace 6.2.0: installed");
    assert_eq!(
        texts(&r),
        [
            (Level::Bad, "Adds a trusted root certificate: \"Zoom Root CA\""),
            (Level::Bad, r"Excludes C:\Program Files\Zoom from antivirus scans"),
            (Level::Bad, "Changes your network settings: hosts file, proxy"),
            (Level::Warn, "Installs 2 drivers: Zoom Audio Device, Zoom Camera"),
            (
                Level::Warn,
                "Adds 1 background service: Zoom Sharing Service (starts with Windows) (full system rights)"
            ),
            (Level::Warn, "1 scheduled task: runs an updater every day, as SYSTEM"),
            (Level::Warn, "Starts with Windows"),
            (Level::Warn, "Allows incoming connections for zoom.exe"),
            (Level::Note, "Adds outbound firewall rules for updater.exe"),
            (Level::Note, "Adds right-click menu entries"),
            (Level::Note, r"Adds to PATH: C:\Program Files\Zoom\bin"),
            (Level::Note, "Files in Program Files (1 MB)"),
            (Level::Note, "Files in ProgramData (2 KB)"),
            (Level::Note, "Files in your user folder (3 KB)"),
            (Level::Note, "Adds a desktop shortcut"),
            (Level::Note, "Adds a Start menu entry"),
            (Level::Note, "Also installed: Zoom Outlook Plugin"),
            (Level::Note, "1 Windows background change hidden (idiff show --all)"),
        ]
    );
}

#[test]
fn the_none_line_names_only_what_is_absent() {
    let subject = Subject {
        name: "X".into(),
        ..Default::default()
    };
    let svc = app_added(
        SourceKind::Services,
        item(
            "xsvc",
            "X Service",
            None,
            &[("start", "3"), ("account", r"NT AUTHORITY\LocalService")],
        ),
    );
    let r = install_receipt(
        &subject,
        &[svc],
        &BTreeMap::new(),
        &BTreeSet::new(),
        Some(0),
        &Env::new(),
    );
    assert_eq!(
        texts(&r),
        [
            (Level::Warn, "Adds 1 background service: X Service"),
            (Level::Good, "No drivers or firewall rules"),
        ]
    );
}

#[test]
fn receipts_round_trip_as_json() {
    let (_, after, subject, attributed) = discord();
    let r = install_receipt(
        &subject,
        &attributed,
        &BTreeMap::new(),
        &BTreeSet::new(),
        Some(3010),
        &after.env,
    );
    let json = serde_json::to_string(&r).unwrap();
    assert_eq!(serde_json::from_str::<Receipt>(&json).unwrap(), r);
}

#[test]
fn partial_reads_are_worded_apart_from_failed_ones() {
    let (_, after, subject, attributed) = discord();
    let notes = BTreeMap::from([
        (
            SourceKind::Tasks,
            "some tasks are only visible to administrators".to_string(),
        ),
        (SourceKind::Defender, "needs administrator".to_string()),
    ]);
    let partial = BTreeSet::from([SourceKind::Tasks]);
    let r = install_receipt(&subject, &attributed, &notes, &partial, Some(0), &after.env);
    let lines = texts(&r);
    assert!(lines.contains(&(
        Level::Note,
        "Only partly checked scheduled tasks: some tasks are only visible to administrators"
    )));
    assert!(lines.contains(&(Level::Note, "Couldn't check antivirus exclusions: needs administrator")));
}

#[test]
fn no_subject_and_nothing_found_is_no_install() {
    let (_, after, _, attributed) = discord();
    let nobody = find_subject(&idiff_core::diff::Delta::default(), None, &[]);
    let r = install_receipt(&nobody, &[], &BTreeMap::new(), &BTreeSet::new(), Some(0), &after.env);
    assert_eq!(r.title, "No install found");
    assert_eq!(texts(&r), [(Level::Good, "No lasting changes found")]);
    // Something changed that nobody could place: still an unknown app's install.
    let unknown: Vec<Attributed> = attributed.into_iter().filter(|a| a.owner == Owner::Unknown).collect();
    let r = install_receipt(
        &nobody,
        &unknown,
        &BTreeMap::new(),
        &BTreeSet::new(),
        Some(0),
        &after.env,
    );
    assert_eq!(r.title, "Unknown app: installed");
}

#[test]
fn leftovers_in_a_partly_read_source_are_not_claimed_gone() {
    let (before, _, subject, install) = discord();
    let mut uninstalled = before.clone();
    uninstalled.sources.get_mut(&SourceKind::Tasks).unwrap().note =
        Some("some tasks are only visible to administrators".into());
    let r = leftovers_receipt(&subject, &install, &uninstalled, &[], &BTreeMap::new());
    assert!(texts(&r).contains(&(
        Level::Note,
        "Only partly checked scheduled tasks: some tasks are only visible to administrators"
    )));
    assert!(!texts(&r).contains(&(Level::Good, "Nothing left behind")));
    assert_eq!(r.title, "Discord 1.0.9170: after uninstall, no leftovers found");
}

#[test]
fn the_hidden_count_has_two_hints() {
    use idiff_core::verdict::hidden_count_text;
    assert_eq!(
        hidden_count_text(1, false),
        "1 Windows background change hidden (idiff show --all)"
    );
    assert_eq!(
        hidden_count_text(3, false),
        "3 Windows background changes hidden (idiff show --all)"
    );
    assert_eq!(
        hidden_count_text(1, true),
        "1 Windows background change hidden (listed below)"
    );
    assert_eq!(
        hidden_count_text(3, true),
        "3 Windows background changes hidden (listed below)"
    );
    // The receipt's own line is the `show --all` one.
    let (_, after, subject, attributed) = discord();
    let r = install_receipt(
        &subject,
        &attributed,
        &BTreeMap::new(),
        &BTreeSet::new(),
        Some(0),
        &after.env,
    );
    assert!(r.lines.iter().any(|l| l.text == hidden_count_text(3, false)));
}
