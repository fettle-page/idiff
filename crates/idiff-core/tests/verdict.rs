use idiff_core::attribute::{Attributed, Owner, Reason, attribute};
use idiff_core::diff::{Change, Delta, diff};
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
    let subject = find_subject(&d, None, &[], &NoiseList::builtin());
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
    let subject = find_subject(&d, None, &[], &NoiseList::builtin());
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
    let nobody = find_subject(&idiff_core::diff::Delta::default(), None, &[], &NoiseList::builtin());
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

#[test]
fn empty_new_folders_show_no_size() {
    let subject = Subject {
        name: "X".into(),
        ..Default::default()
    };
    let folder = |key: &str, root: &str, bytes: &str| {
        app_added(
            SourceKind::Folders,
            item(key, key, None, &[("root", root), ("kind", "folder"), ("bytes", bytes)]),
        )
    };
    let receipt = |folders: Vec<Attributed>| {
        install_receipt(
            &subject,
            &folders,
            &BTreeMap::new(),
            &BTreeSet::new(),
            Some(0),
            &Env::new(),
        )
    };
    let has = |r: &Receipt, line: (Level, &str)| texts(r).contains(&line);

    let r = receipt(vec![folder(r"c:\users\a\appdata\local\x", r"AppData\Local", "0")]);
    assert!(has(&r, (Level::Good, "Files only in your user folder")), "{r:#?}");
    let r = receipt(vec![folder(r"c:\users\a\appdata\local\x", r"AppData\Local", "1")]);
    assert!(
        has(&r, (Level::Good, "Files only in your user folder (1 KB)")),
        "{r:#?}"
    );
    let r = receipt(vec![
        folder(r"c:\program files\x", "Program Files", "0"),
        folder(r"c:\users\a\appdata\local\x", r"AppData\Local", "0"),
    ]);
    assert!(has(&r, (Level::Note, "Files in Program Files")), "{r:#?}");
    assert!(has(&r, (Level::Note, "Files in your user folder")), "{r:#?}");
}

/// Opera and VS Code in catalog run 38742495727: an install into AppData\Local\Programs\<app> made
/// `Programs` too. It was listed as a leftover (VS Code: empty) and its size was Opera's, twice.
#[test]
fn the_shared_programs_folder_is_neither_added_nor_left_over() {
    let programs = r"c:\users\a\appdata\local\programs";
    let app_dir = format!(r"{programs}\opera");
    let folder = |key: &str, bytes: &str| {
        // Folders are labelled by their path under the profile.
        let label = key.replace(r"c:\users\a\appdata\local", r"AppData\Local");
        Change::Added(item(
            key,
            &label,
            None,
            &[("root", r"AppData\Local"), ("kind", "folder"), ("bytes", bytes)],
        ))
    };
    let opera = Item {
        label: "Opera Stable".into(),
        ..item(
            r"hkcu\native\opera",
            "Opera Stable",
            Some(&app_dir),
            &[("version", "1"), ("publisher", "Opera Software")],
        )
    };
    let d = Delta {
        sources: BTreeMap::from([
            (SourceKind::Programs, vec![Change::Added(opera)]),
            (
                SourceKind::Folders,
                vec![folder(programs, "468713472"), folder(&app_dir, "468713472")],
            ),
        ]),
        ..Default::default()
    };
    let subject = find_subject(&d, None, &[], &NoiseList::builtin());
    let attributed = attribute(&d, &subject, &NoiseList::builtin(), &BTreeMap::new());

    let r = install_receipt(
        &subject,
        &attributed,
        &BTreeMap::new(),
        &BTreeSet::new(),
        Some(0),
        &Env::new(),
    );
    let files = r.lines.iter().find(|l| l.text.starts_with("Files only in")).unwrap();
    assert_eq!(files.items, std::slice::from_ref(&app_dir), "{r:#?}");
    assert_eq!(files.text, "Files only in your user folder (447 MB)");
    // Not "couldn't tell if it was this app" either: it's hidden as shared.
    assert!(r.unknown.is_empty(), "{:#?}", r.unknown);
    assert!(
        r.lines
            .iter()
            .chain(&r.unknown)
            .all(|l| !l.items.iter().any(|k| k == programs)),
        "{r:#?}"
    );
    assert_eq!(r.hidden.len(), 1);
    assert_eq!(r.hidden[0].items, [programs]);

    // Both folders are still there after the uninstall; only the app's own is a leftover.
    let mut uninstalled = Snapshot::load(include_str!("fixtures/discord-before.json")).unwrap();
    let folders = uninstalled.sources.get_mut(&SourceKind::Folders).unwrap();
    folders.items.extend([
        item(programs, "", None, &[("kind", "folder")]),
        item(&app_dir, "", None, &[("kind", "folder")]),
    ]);
    let sizes = BTreeMap::from([(programs.to_string(), 0), (app_dir.clone(), 468713472)]);
    let r = leftovers_receipt(&subject, &attributed, &uninstalled, &[], &sizes);
    let folders: Vec<&str> = r
        .lines
        .iter()
        .filter(|l| l.text.to_lowercase().contains("programs"))
        .map(|l| l.text.as_str())
        .collect();
    assert_eq!(folders, [r"447 MB in AppData\Local\programs\opera"], "{r:#?}");
    assert_eq!(r.title, "Opera Stable 1: after uninstall, 1 leftover");
}

/// Zoom in catalog run 38742495727: four firewall rules, two named "Airhost service for Zoom
/// Video Meetings" (different rule ids, as with one for TCP and one for UDP), made four lines.
#[test]
fn leftovers_with_the_same_name_are_one_line_with_a_count() {
    let rule = |id: &str, name: &str| {
        app_added(
            SourceKind::Firewall,
            item(id, name, Some(r"c:\program files\zoom\bin\airhost.exe"), &[]),
        )
    };
    let install = vec![
        rule("{a}", "Airhost service for Zoom Video Meetings"),
        rule("{b}", "Airhost service for Zoom Video Meetings"),
        rule("{c}", "Hybrid Conference for Zoom Video Meetings"),
        rule("{d}", "Zoom Video Meeting"),
    ];
    let mut uninstalled = Snapshot::load(include_str!("fixtures/discord-before.json")).unwrap();
    let firewall = uninstalled.sources.get_mut(&SourceKind::Firewall).unwrap();
    for a in &install {
        firewall.items.push(a.change.item().clone());
    }
    let subject = Subject {
        name: "Zoom".into(),
        ..Default::default()
    };
    let r = leftovers_receipt(&subject, &install, &uninstalled, &[], &BTreeMap::new());
    assert_eq!(r.title, "Zoom: after uninstall, 4 leftovers");
    assert_eq!(
        texts(&r),
        [
            (
                Level::Bad,
                r#"Firewall rule "Airhost service for Zoom Video Meetings" is still there (2 rules)"#
            ),
            (
                Level::Bad,
                r#"Firewall rule "Hybrid Conference for Zoom Video Meetings" is still there"#
            ),
            (Level::Bad, r#"Firewall rule "Zoom Video Meeting" is still there"#),
        ]
    );
    // Nothing is dropped: both rules' ids and reasons are on the one line.
    assert_eq!(r.lines[0].items, ["{a}", "{b}"]);
    assert_eq!(r.lines[0].why.len(), 2);
}

#[test]
fn an_empty_leftover_folder_says_so() {
    let (before, after, subject, install) = discord();
    let mut uninstalled = before.clone();
    let folder = after.sources[&SourceKind::Folders]
        .items
        .iter()
        .find(|i| i.key.ends_with(r"\appdata\local\discord"))
        .unwrap()
        .clone();
    uninstalled
        .sources
        .get_mut(&SourceKind::Folders)
        .unwrap()
        .items
        .push(folder);
    let key = r"c:\users\a\appdata\local\discord".to_string();
    let r = leftovers_receipt(
        &subject,
        &install,
        &uninstalled,
        &[],
        &BTreeMap::from([(key.clone(), 0)]),
    );
    assert_eq!(
        texts(&r),
        [(Level::Bad, r#"Folder "AppData\Local\Discord" is still there (empty)"#)]
    );
    let r = leftovers_receipt(&subject, &install, &uninstalled, &[], &BTreeMap::from([(key, 1)]));
    assert_eq!(texts(&r), [(Level::Bad, r"1 KB in AppData\Local\Discord")]);
}

/// Catalog run 38772924882: NordVPN's uninstall left only the .NET it brought along, and its page
/// said "5 leftovers". Microsoft's runtimes that came with an app and stay are one Note.
#[test]
fn microsoft_runtimes_left_installed_are_a_note_not_a_leftover() {
    let program = |key: &str, label: &str, publisher: &str| {
        app_added(
            SourceKind::Programs,
            item(key, label, None, &[("publisher", publisher)]),
        )
    };
    let install = vec![
        program(r"hklm\native\app", "NordVPN", "Nord Security"),
        program(
            r"hklm\native\a",
            "Microsoft .NET Runtime - 8.0.32 (x64)",
            "Microsoft Corporation",
        ),
        program(
            r"hklm\native\b",
            "Microsoft .NET Host - 8.0.32 (x64)",
            "Microsoft Corporation",
        ),
        program(r"hklm\native\c", "NordUpdater", "Nord Security"),
    ];
    let mut uninstalled = Snapshot::load(include_str!("fixtures/discord-before.json")).unwrap();
    let programs = uninstalled.sources.get_mut(&SourceKind::Programs).unwrap();
    for a in install.iter().skip(1) {
        programs.items.push(a.change.item().clone());
    }
    let subject = Subject {
        name: "NordVPN".into(),
        publisher: Some("Nord Security".into()),
        own_entry: Some(r"hklm\native\app".into()),
        ..Default::default()
    };
    let r = leftovers_receipt(&subject, &install, &uninstalled, &[], &BTreeMap::new());
    // A non-Microsoft extra entry stays a leftover; the runtimes don't count.
    assert_eq!(r.title, "NordVPN: after uninstall, 1 leftover");
    assert_eq!(
        texts(&r),
        [
            (Level::Bad, r#"Program "NordUpdater" is still there"#),
            (
                Level::Note,
                "Left installed: Microsoft .NET Runtime - 8.0.32 (x64), Microsoft .NET Host - 8.0.32 (x64). \
                 They came with the app, and other apps may use them."
            ),
        ]
    );
    assert_eq!(r.lines[1].items, [r"hklm\native\a", r"hklm\native\b"]);

    // One runtime reads in the singular; with nothing else left, it is "no leftovers".
    let one = vec![install[0].clone(), install[1].clone()];
    let mut only_one = Snapshot::load(include_str!("fixtures/discord-before.json")).unwrap();
    only_one
        .sources
        .get_mut(&SourceKind::Programs)
        .unwrap()
        .items
        .push(install[1].change.item().clone());
    let r = leftovers_receipt(&subject, &one, &only_one, &[], &BTreeMap::new());
    assert_eq!(r.title, "NordVPN: after uninstall, no leftovers");
    assert!(texts(&r).contains(&(
        Level::Note,
        "Left installed: Microsoft .NET Runtime - 8.0.32 (x64). It came with the app, and other apps may use it."
    )));

    // The app's own entry is a leftover even from Microsoft, and so is anything of a Microsoft app.
    let microsoft = Subject {
        name: "PowerToys".into(),
        publisher: Some("Microsoft Corporation".into()),
        own_entry: Some(r"hklm\native\app".into()),
        ..Default::default()
    };
    let r = leftovers_receipt(&microsoft, &one, &only_one, &[], &BTreeMap::new());
    assert_eq!(r.title, "PowerToys: after uninstall, 1 leftover");
}

/// ShareX: the Startup-folder shortcut was both "Starts with Windows" and "Adds a Start menu entry".
#[test]
fn a_startup_folder_shortcut_is_not_a_start_menu_entry() {
    let shortcut = |key: &str| {
        app_added(
            SourceKind::Folders,
            item(key, key, None, &[("kind", "shortcut"), ("root", "Start menu")]),
        )
    };
    let attributed = vec![
        shortcut(r"c:\users\a\appdata\roaming\microsoft\windows\start menu\programs\sharex\sharex.lnk"),
        shortcut(r"c:\users\a\appdata\roaming\microsoft\windows\start menu\programs\startup\sharex.lnk"),
    ];
    let subject = Subject {
        name: "ShareX".into(),
        ..Default::default()
    };
    let r = install_receipt(
        &subject,
        &attributed,
        &BTreeMap::new(),
        &BTreeSet::new(),
        Some(0),
        &BTreeMap::new(),
    );
    let line = r.lines.iter().find(|l| l.text == "Adds a Start menu entry").unwrap();
    assert_eq!(line.items.len(), 1, "{line:?}");
    assert!(line.items[0].contains(r"\sharex\sharex.lnk"));
    // Only the Startup folder: no Start menu line at all.
    let r = install_receipt(
        &subject,
        &attributed[1..],
        &BTreeMap::new(),
        &BTreeSet::new(),
        Some(0),
        &BTreeMap::new(),
    );
    assert!(!r.lines.iter().any(|l| l.text == "Adds a Start menu entry"), "{r:#?}");
}

/// Zoom's `RunOnce` value runs one time at the next sign-in; it isn't a program that starts with Windows.
#[test]
fn a_runonce_entry_runs_once_and_is_not_a_startup_program() {
    let entry = |key: &str, label: &str| app_added(SourceKind::Startup, item(key, label, None, &[]));
    let attributed = vec![
        entry(r"run|hkcu|software\microsoft\windows\currentversion\run|zoom", "Zoom"),
        entry(
            r"run|hkcu|software\microsoft\windows\currentversion\runonce|zoommsirepair",
            "ZoomMsiRepair",
        ),
    ];
    let subject = Subject {
        name: "Zoom".into(),
        ..Default::default()
    };
    let r = install_receipt(
        &subject,
        &attributed,
        &BTreeMap::new(),
        &BTreeSet::new(),
        Some(0),
        &BTreeMap::new(),
    );
    let startup = r.lines.iter().find(|l| l.text == "Starts with Windows").unwrap();
    assert_eq!(startup.level, Level::Warn);
    assert_eq!(startup.items.len(), 1, "{startup:?}");
    let once = r
        .lines
        .iter()
        .find(|l| l.text == "Runs once at the next sign-in")
        .unwrap();
    assert_eq!(once.level, Level::Note);
    assert!(once.items[0].ends_with("|zoommsirepair"));
    // Alone, it is not "Starts with Windows" at all.
    let r = install_receipt(
        &subject,
        &attributed[1..],
        &BTreeMap::new(),
        &BTreeSet::new(),
        Some(0),
        &BTreeMap::new(),
    );
    assert!(!r.lines.iter().any(|l| l.text == "Starts with Windows"), "{r:#?}");
}

/// EA app: the registry name "Microsoft Visual C++ 2010  x86 Redistributable" has two spaces.
#[test]
fn runs_of_spaces_in_program_names_are_one_space() {
    let subject = Subject {
        name: "EA app".into(),
        also_installed: vec!["Microsoft Visual C++ 2010  x86 Redistributable - 10.0.40219".into()],
        ..Default::default()
    };
    let attributed = vec![app_added(
        SourceKind::Services,
        item("svc", "Some   Update\tService", None, &[]),
    )];
    let r = install_receipt(
        &subject,
        &attributed,
        &BTreeMap::new(),
        &BTreeSet::new(),
        Some(0),
        &BTreeMap::new(),
    );
    let text: Vec<&str> = r.lines.iter().map(|l| l.text.as_str()).collect();
    assert!(
        text.contains(&"Also installed: Microsoft Visual C++ 2010 x86 Redistributable - 10.0.40219"),
        "{text:?}"
    );
    assert!(text.iter().any(|t| t.ends_with("Some Update Service")), "{text:?}");
}
