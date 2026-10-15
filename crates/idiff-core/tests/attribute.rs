use idiff_core::attribute::{Attributed, Owner, Reason, attribute};
use idiff_core::copy;
use idiff_core::diff::{Change, Delta, diff};
use idiff_core::model::{Item, Snapshot, SourceKind};
use idiff_core::noise::NoiseList;
use idiff_core::publisher::{is_microsoft, normalize, same};
use idiff_core::subject::{Subject, find_subject, winget_id};
use std::collections::BTreeMap;

fn load(json: &str) -> Snapshot {
    Snapshot::load(json).unwrap()
}

fn discord() -> (Snapshot, Snapshot) {
    (
        load(include_str!("fixtures/discord-before.json")),
        load(include_str!("fixtures/discord-after.json")),
    )
}

fn vlc() -> (Snapshot, Snapshot) {
    (
        load(include_str!("fixtures/vlc-before.json")),
        load(include_str!("fixtures/vlc-after.json")),
    )
}

fn s<const N: usize>(words: [&str; N]) -> Vec<String> {
    words.iter().map(|w| w.to_string()).collect()
}

fn find<'a>(r: &'a [Attributed], key: &str) -> &'a Attributed {
    r.iter()
        .find(|a| a.change.item().key.contains(key))
        .unwrap_or_else(|| panic!("no change with a key containing {key:?}"))
}

fn item(key: &str, target: Option<&str>, props: &[(&str, &str)]) -> Item {
    Item {
        key: key.into(),
        label: key.into(),
        target: target.map(String::from),
        props: props.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
    }
}

fn added(kind: SourceKind, items: Vec<Item>) -> Delta {
    Delta {
        sources: BTreeMap::from([(kind, items.into_iter().map(Change::Added).collect())]),
        notes: BTreeMap::new(),
        partial: Default::default(),
    }
}

#[test]
fn discord_changes_are_split_correctly() {
    let (before, after) = discord();
    let d = diff(&before, &after);
    let r = attribute(&d, &find_subject(&d, None, &[]), &NoiseList::builtin(), &after.env);
    let owner = |key: &str| find(&r, key).owner;
    assert_eq!(owner("discordupdate"), Owner::App);
    assert_eq!(owner("|discord"), Owner::App); // startup run value
    assert_eq!(owner("mpkslabc123"), Owner::Windows);
    assert_eq!(owner("microsoftedgeupdatetask"), Owner::Windows);
    assert_eq!(owner("windowsapps"), Owner::Windows);
    assert_eq!(owner("foosvc"), Owner::Unknown);

    assert_eq!(owner(r"hkcu\native\discord"), Owner::App);
    assert_eq!(owner(r"appdata\local\discord"), Owner::App);
    assert_eq!(r.len(), 8, "{r:#?}");
}

#[test]
fn a_microsoft_store_app_updating_itself_is_windows() {
    let (_, after) = discord();
    let subject = find_subject(&diff(&after, &after), None, &[]);
    let ms = "CN=Microsoft Corporation, O=Microsoft Corporation, L=Redmond, S=Washington, C=US";
    let store = |version: &str| Item {
        key: "microsoft.windowsstore_8wekyb3d8bbwe".into(),
        label: "Microsoft Store".into(),
        target: Some(format!(
            r"c:\program files\windowsapps\microsoft.windowsstore_{version}_x64__8wekyb3d8bbwe"
        )),
        props: BTreeMap::from([("version".into(), version.into()), ("publisher".into(), ms.into())]),
    };
    let d = Delta {
        sources: BTreeMap::from([(
            SourceKind::Packaged,
            vec![Change::Modified {
                before: store("22409.1401.5.0"),
                after: store("22410.1401.3.0"),
                fields: vec!["target".into(), "version".into()],
            }],
        )]),
        notes: BTreeMap::new(),
        partial: Default::default(),
    };
    let r = attribute(&d, &subject, &NoiseList::builtin(), &after.env);
    assert_eq!(r[0].owner, Owner::Windows);
    assert_eq!(
        r[0].reason,
        Reason::Noise("a Microsoft Store app updated itself".into())
    );
}

#[test]
fn reasons_say_which_rule_matched() {
    let (before, after) = discord();
    let d = diff(&before, &after);
    let r = attribute(&d, &find_subject(&d, None, &[]), &NoiseList::builtin(), &after.env);
    assert_eq!(find(&r, "discordupdate").reason, Reason::TargetInInstallDir);
    assert_eq!(find(&r, r"appdata\local\discord").reason, Reason::ItemInInstallDir);
    assert_eq!(find(&r, r"hkcu\native\discord").reason, Reason::SubjectEntry);
    assert_eq!(find(&r, "foosvc").reason, Reason::Unattributed);
    assert_eq!(
        find(&r, "mpkslabc123").reason,
        Reason::Noise("Microsoft Defender's per-update driver".into())
    );
    assert!(Reason::NameMatches.text().contains("name matches"));
    for reason in [
        Reason::Noise("x".into()),
        Reason::SubjectEntry,
        Reason::TargetInInstallDir,
        Reason::SignerMatchesPublisher,
        Reason::ItemInInstallDir,
        Reason::WindowsComponent,
        Reason::NameMatches,
        Reason::Unattributed,
    ] {
        assert!(!reason.text().is_empty());
    }
    assert_eq!(Reason::Noise("x".into()).owner(), Owner::Windows);
    assert_eq!(Reason::NameMatches.owner(), Owner::App);
    assert_eq!(Reason::Unattributed.owner(), Owner::Unknown);
}

#[test]
fn subject_comes_from_the_new_program_entry() {
    let (before, after) = discord();
    let subject = find_subject(&diff(&before, &after), None, &[]);
    assert_eq!(subject.name, "Discord");
    assert_eq!(subject.version.as_deref(), Some("1.0.9170"));
    assert_eq!(subject.publisher.as_deref(), Some("Discord Inc."));
    assert_eq!(subject.previous_version, None);
    assert_eq!(subject.install_dirs, [r"c:\users\a\appdata\local\discord"]);
    assert!(subject.also_installed.is_empty());
}

#[test]
fn app_flag_wins_over_programs() {
    let (before, after) = discord();
    let subject = find_subject(
        &diff(&before, &after),
        Some("Steam"),
        &s(["winget", "install", "Valve.Steam"]),
    );
    assert_eq!(subject.name, "Steam");
}

#[test]
fn several_new_programs_pick_the_largest() {
    let d = added(
        SourceKind::Programs,
        vec![
            item(
                r"hklm\native\redist",
                None,
                &[("size_kb", "20000"), ("version", "14.4")],
            ),
            item(
                r"hklm\native\game",
                Some(r"c:\games\game"),
                &[("size_kb", "9000000"), ("version", "2.0")],
            ),
            item(
                r"hklm\native\hidden",
                None,
                &[("size_kb", "99999999"), ("system_component", "1")],
            ),
        ],
    );
    let subject = find_subject(&d, None, &[]);
    assert_eq!(subject.name, r"hklm\native\game");
    assert_eq!(subject.also_installed, [r"hklm\native\redist"]);
    assert_eq!(subject.install_dirs, [r"c:\games\game"]);
}

#[test]
fn a_too_broad_install_location_is_ignored() {
    // Some installers write their parent folder as InstallLocation; it mustn't claim everything in it.
    for broad in [r"c:\program files", r"c:\users\a\appdata\local", r"c:\users\a", r"c:\"] {
        let d = added(
            SourceKind::Programs,
            vec![item(r"hklm\native\tool", Some(broad), &[("version", "1")])],
        );
        assert!(find_subject(&d, None, &[]).install_dirs.is_empty(), "{broad}");
    }
}

#[test]
fn packaged_and_winget_and_unknown_subjects() {
    let store_app = added(
        SourceKind::Packaged,
        vec![Item {
            key: "spotifyab.spotifymusic_zpdnekdrzrea0".into(),
            label: "Spotify Music".into(),
            target: Some(r"c:\program files\windowsapps\spotifyab.spotifymusic_1.2.0.0_x64__zpdnekdrzrea0".into()),
            props: BTreeMap::from([
                ("version".into(), "1.2.0.0".into()),
                (
                    "publisher".into(),
                    "CN=Spotify AB, O=Spotify AB, L=Stockholm, C=SE".into(),
                ),
            ]),
        }],
    );
    let subject = find_subject(&store_app, None, &[]);
    assert_eq!(subject.name, "Spotify Music");
    assert_eq!(subject.publisher.as_deref(), Some("Spotify AB"));
    assert_eq!(subject.install_dirs.len(), 1);

    let winget = find_subject(
        &Delta::default(),
        None,
        &s(["winget", "install", "--id", "Valve.Steam"]),
    );
    assert_eq!(winget.name, "Valve.Steam");

    let unknown = find_subject(&Delta::default(), None, &s(["setup.exe", "/S"]));
    assert_eq!(unknown.name, copy::UNKNOWN_APP);
    assert!(unknown.install_dirs.is_empty());
}

#[test]
fn upgrade_sets_previous_version() {
    let (_, after) = discord();
    let mut base = after.clone();
    // Discord was already installed; the update changed its version.
    let programs = &mut base.sources.get_mut(&SourceKind::Programs).unwrap().items;
    let entry = programs.iter_mut().find(|i| i.key == r"hkcu\native\discord").unwrap();
    entry.props.insert("version".into(), "1.0.9100".into());
    let d = diff(&base, &after);
    let subject = find_subject(&d, None, &[]);
    assert_eq!(subject.name, "Discord");
    assert_eq!(subject.version.as_deref(), Some("1.0.9170"));
    assert_eq!(subject.previous_version.as_deref(), Some("1.0.9100"));
    let r = attribute(&d, &subject, &NoiseList::builtin(), &after.env);
    assert_eq!(find(&r, r"hkcu\native\discord").owner, Owner::App);
}

#[test]
fn removed_changes_use_the_same_rules() {
    let (before, after) = discord();
    let install = diff(&before, &after);
    let subject = find_subject(&install, None, &[]);
    // As after an uninstall: everything the install added is gone again.
    let r = attribute(&diff(&after, &before), &subject, &NoiseList::builtin(), &after.env);
    let task = find(&r, "discordupdate");
    assert!(matches!(task.change, Change::Removed(_)));
    assert_eq!(task.owner, Owner::App);
    assert_eq!(find(&r, "mpkslabc123").owner, Owner::Windows);
    assert_eq!(find(&r, "foosvc").owner, Owner::Unknown);
}

#[test]
fn vlc_runner_install_is_attributed() {
    let (before, after) = vlc();
    let d = diff(&before, &after);
    let subject = find_subject(&d, None, &s(["winget", "install", "--id", "VideoLAN.VLC", "--exact"]));
    assert_eq!(subject.name, "VLC media player");
    assert_eq!(subject.version.as_deref(), Some("3.0.24"));
    assert_eq!(subject.publisher.as_deref(), Some("VideoLAN"));
    assert!(
        subject
            .install_dirs
            .contains(&r"c:\program files\videolan\vlc".to_string())
    );
    assert!(subject.install_dirs.contains(&r"c:\program files\videolan".to_string()));
    assert!(!subject.install_dirs.iter().any(|d| d.contains("peerdistrepub")));

    let r = attribute(&d, &subject, &NoiseList::builtin(), &after.env);
    for verb in ["directory|verb|playwithvlc", "directory|verb|addtoplaylistvlc"] {
        assert_eq!(find(&r, verb).owner, Owner::App);
        assert_eq!(find(&r, verb).reason, Reason::TargetInInstallDir);
    }
    assert_eq!(find(&r, r"c:\program files\videolan").owner, Owner::App);
    assert_eq!(find(&r, r"public\desktop\vlc media player.lnk").owner, Owner::App);
    assert_eq!(find(&r, "vlc media player skinned.lnk").owner, Owner::App);
    assert_ne!(find(&r, "peerdistrepub").owner, Owner::App);
    assert_eq!(find(&r, "microsoft.winget.source").owner, Owner::Windows);
    // BITS and the Program Compatibility Assistant changed their start type meanwhile.
    assert_eq!(find(&r, "bits").owner, Owner::Windows);
    assert_eq!(find(&r, "bits").reason, Reason::WindowsComponent);
    assert_eq!(find(&r, "pcasvc").owner, Owner::Windows);
    assert!(!r.iter().any(|a| a.owner == Owner::Unknown), "{r:#?}");
}

#[test]
fn unrelated_new_folder_is_not_an_install_dir() {
    // Without the noise list, BranchCache's folder must still not count as VLC's.
    let (before, after) = vlc();
    let d = diff(&before, &after);
    let subject = find_subject(&d, None, &[]);
    let r = attribute(&d, &subject, &NoiseList::parse("").unwrap(), &after.env);
    assert_eq!(find(&r, "peerdistrepub").owner, Owner::Unknown);
}

#[test]
fn an_unknown_app_claims_nothing_by_name() {
    let (before, after) = vlc();
    let d = diff(&before, &after);
    let unknown = find_subject(&Delta::default(), None, &[]);
    let r = attribute(&d, &unknown, &NoiseList::builtin(), &after.env);
    // Only the Programs entry, which any install's own entry is (InstalledAlongside).
    let app: Vec<&Attributed> = r.iter().filter(|a| a.owner == Owner::App).collect();
    assert!(app.iter().all(|a| a.reason == Reason::InstalledAlongside), "{app:#?}");
    assert!(!r.iter().any(|a| a.reason == Reason::NameMatches), "{r:#?}");
    assert_eq!(find(&r, "directory|verb|playwithvlc").owner, Owner::Unknown);
}

#[test]
fn name_matches_are_whole_words() {
    let subject = Subject {
        name: "Notion".into(),
        ..Default::default()
    };
    let d = added(
        SourceKind::Tasks,
        vec![
            item(r"\notion update", None, &[]),
            item(r"\notionally unrelated", None, &[]),
        ],
    );
    let r = attribute(&d, &subject, &NoiseList::parse("").unwrap(), &BTreeMap::new());
    assert_eq!(find(&r, r"\notion update").reason, Reason::NameMatches);
    assert_eq!(find(&r, "notionally").owner, Owner::Unknown);
}

#[test]
fn signer_rules_use_the_publisher() {
    let subject = Subject {
        name: "Zoom Workplace".into(),
        publisher: Some("Zoom Video Communications, Inc.".into()),
        ..Default::default()
    };
    let env = BTreeMap::from([("SystemRoot".to_string(), r"C:\Windows".to_string())]);
    let d = added(
        SourceKind::Services,
        vec![
            item(
                "zoomsvc",
                Some(r"c:\program files\common files\z\svc.exe"),
                &[("signer", "Zoom Video Communications, Inc")],
            ),
            item(
                "mssvc",
                Some(r"c:\program files\x\y.exe"),
                &[("signer", "Microsoft Windows")],
            ),
            item("winsvc", Some(r"c:\windows\system32\svchost.exe"), &[]),
        ],
    );
    let r = attribute(&d, &subject, &NoiseList::parse("").unwrap(), &env);
    assert_eq!(find(&r, "zoomsvc").reason, Reason::SignerMatchesPublisher);
    // A new service isn't Windows' for its signature alone: installers bring Microsoft's (catalog).
    assert_eq!(find(&r, "mssvc").reason, Reason::Unattributed);
    // Something new in the Windows folder isn't Windows' without Windows' signature (kill test).
    assert_eq!(find(&r, "winsvc").reason, Reason::Unattributed);

    // A Microsoft app's own Windows-folder and Microsoft-signed changes aren't hidden as Windows'.
    let teams = Subject {
        name: "Microsoft Teams".into(),
        publisher: Some("Microsoft Corporation".into()),
        ..Default::default()
    };
    let r = attribute(&d, &teams, &NoiseList::parse("").unwrap(), &env);
    assert_eq!(find(&r, "mssvc").owner, Owner::App);
    assert_ne!(find(&r, "winsvc").owner, Owner::Windows);
}

#[test]
fn winget_id_is_read_from_the_command() {
    assert_eq!(
        winget_id(&s(["winget", "install", "--id", "Discord.Discord", "--silent"])),
        Some("Discord.Discord".into())
    );
    assert_eq!(
        winget_id(&s(["winget", "install", "Valve.Steam"])),
        Some("Valve.Steam".into())
    );
    assert_eq!(
        winget_id(&s([
            r"C:\Users\a\AppData\Local\Microsoft\WindowsApps\winget.exe",
            "upgrade",
            "--id=Zoom.Zoom"
        ])),
        Some("Zoom.Zoom".into())
    );
    assert_eq!(
        winget_id(&s([
            "winget", "install", "--source", "winget", "--scope", "user", "-e", "Git.Git"
        ])),
        Some("Git.Git".into())
    );
    assert_eq!(winget_id(&s(["winget", "install", "-q", "vlc"])), Some("vlc".into()));
    assert_eq!(winget_id(&s(["winget", "search", "vlc"])), None);
    assert_eq!(winget_id(&s(["msiexec", "/i", "x.msi"])), None);
    assert_eq!(winget_id(&[]), None);
}

#[test]
fn publisher_suffixes_are_ignored() {
    assert!(same("Discord Inc.", "Discord, Inc"));
    assert!(!same("Microsoft Corporation", "Discord Inc."));
    assert!(same("Mozilla Corporation", "MOZILLA CORP."));
    assert!(same("Foo GmbH", "foo"));
    assert!(same("Bar S.r.l.", "Bar"));
    assert!(same("CN=Spotify AB, O=Spotify AB, L=Stockholm, C=SE", "Spotify AB"));
    assert!(!same("", ""));
    assert_eq!(
        normalize("Zoom Video Communications, Inc."),
        "zoom video communications"
    );
    assert!(is_microsoft("Microsoft Corporation"));
    assert!(is_microsoft("Microsoft Windows"));
    assert!(is_microsoft(
        "CN=Microsoft Windows, O=Microsoft Corporation, L=Redmond, S=Washington, C=US"
    ));
    assert!(!is_microsoft("Discord Inc."));
    assert!(!is_microsoft("Not Microsoft Tools Ltd"));
}

#[test]
fn microsoft_rule_is_skipped_for_microsoft_apps() {
    let d = added(
        SourceKind::Tasks,
        vec![item(
            r"\microsoft\office\office automatic updates 2.0",
            Some(r"c:\program files\common files\microsoft shared\clicktorun\officec2rclient.exe"),
            &[],
        )],
    );
    let office = Subject {
        name: "Microsoft 365 Apps for enterprise".into(),
        publisher: Some("Microsoft Corporation".into()),
        ..Default::default()
    };
    let discord = Subject {
        name: "Discord".into(),
        publisher: Some("Discord Inc.".into()),
        ..Default::default()
    };
    let noise = NoiseList::builtin();
    let env = BTreeMap::new();
    assert_ne!(attribute(&d, &office, &noise, &env)[0].owner, Owner::Windows);
    assert_eq!(attribute(&d, &discord, &noise, &env)[0].owner, Owner::Windows);
}

/// Cases the 30-app kill test found: added items in Windows' folders, or signed by Microsoft for a
/// vendor, that are the app's.
mod kill_test {
    use super::*;

    fn subject(name: &str, publisher: &str, dir: &str) -> Subject {
        Subject {
            name: name.into(),
            publisher: Some(publisher.into()),
            install_dirs: vec![dir.into()],
            ..Default::default()
        }
    }

    fn env() -> BTreeMap<String, String> {
        BTreeMap::from([("SystemRoot".to_string(), r"C:\Windows".to_string())])
    }

    fn one(kind: SourceKind, change: Change, subject: &Subject) -> Attributed {
        let d = Delta {
            sources: BTreeMap::from([(kind, vec![change])]),
            notes: BTreeMap::new(),
            partial: Default::default(),
        };
        attribute(&d, subject, &NoiseList::builtin(), &env()).remove(0)
    }

    #[test]
    fn a_whql_signed_driver_in_system32_is_not_windows() {
        let malwarebytes = subject(
            "Malwarebytes",
            "Malwarebytes",
            r"c:\program files\malwarebytes\anti-malware",
        );
        let driver = item(
            "mbamswissarmy",
            Some(r"c:\windows\system32\drivers\mbamswissarmy.sys"),
            &[("signer", "Microsoft Windows Hardware Compatibility Publisher")],
        );
        let a = one(SourceKind::Drivers, Change::Added(driver), &malwarebytes);
        assert_ne!(a.owner, Owner::Windows, "{a:?}");
    }

    #[test]
    fn a_vendor_service_in_system32_is_the_apps_by_signer() {
        let dropbox = subject("Dropbox", "Dropbox, Inc.", r"c:\program files (x86)\dropbox\client");
        let service = item(
            "dbxsvc",
            Some(r"c:\windows\system32\dbxsvc.exe"),
            &[("signer", "Dropbox, Inc")],
        );
        let a = one(SourceKind::Services, Change::Added(service), &dropbox);
        assert_eq!(a.owner, Owner::App);
        assert_eq!(a.reason, Reason::SignerMatchesPublisher);
    }

    #[test]
    fn an_uninstall_shortcut_to_msiexec_is_not_windows() {
        let audacity = subject("Audacity 4.0", "Audacity Team", r"c:\program files\audacity");
        let shortcut = Item {
            key: r"c:\programdata\microsoft\windows\start menu\programs\audacity 4.0\uninstall audacity 4.0.lnk".into(),
            label: r"Start menu (all users)\Audacity 4.0\Uninstall Audacity 4.0.lnk".into(),
            target: Some(r"c:\windows\system32\msiexec.exe".into()),
            props: BTreeMap::from([
                ("root".into(), "Start menu (all users)".into()),
                ("kind".into(), "shortcut".into()),
                ("signer".into(), "Microsoft Windows".into()),
            ]),
        };
        let a = one(SourceKind::Folders, Change::Added(shortcut), &audacity);
        assert_ne!(a.owner, Owner::Windows, "{a:?}");
    }

    #[test]
    fn an_added_item_signed_as_windows_is_windows() {
        // Only for the kinds of new item installers don't bring Microsoft's own of (see
        // added_microsoft_signed_services_and_drivers_are_not_hidden).
        let dropbox = subject("Dropbox", "Dropbox, Inc.", r"c:\program files (x86)\dropbox\client");
        for signer in ["Microsoft Windows", "microsoft corporation"] {
            let handler = item("*|handler|x", Some(r"c:\program files\x\y.dll"), &[("signer", signer)]);
            let a = one(SourceKind::ShellExt, Change::Added(handler), &dropbox);
            assert_eq!(a.reason, Reason::WindowsComponent, "{signer}");
        }
    }

    #[test]
    fn a_windows_folder_alone_hides_only_changes_to_existing_items() {
        let dropbox = subject("Dropbox", "Dropbox, Inc.", r"c:\program files (x86)\dropbox\client");
        let svchost = |start: &str| item("bits", Some(r"c:\windows\system32\svchost.exe"), &[("start", start)]);
        let added = one(SourceKind::Services, Change::Added(svchost("2")), &dropbox);
        assert_ne!(added.owner, Owner::Windows, "{added:?}");
        let modified = Change::Modified {
            before: svchost("2"),
            after: svchost("3"),
            fields: vec!["start".into()],
        };
        assert_eq!(
            one(SourceKind::Services, modified, &dropbox).reason,
            Reason::WindowsComponent
        );
        let removed = one(SourceKind::Services, Change::Removed(svchost("3")), &dropbox);
        assert_eq!(removed.reason, Reason::WindowsComponent);
    }
}

/// Drivers Microsoft signs for other companies (WHQL, attestation) say whose they are in the
/// file's own details: Malwarebytes' three drivers and Google Drive's file-system driver.
mod company_fallback {
    use super::*;

    const WHQL: &str = "Microsoft Windows Hardware Compatibility Publisher";

    fn subject(publisher: &str) -> Subject {
        Subject {
            name: "Malwarebytes".into(),
            publisher: Some(publisher.into()),
            ..Default::default()
        }
    }

    fn driver(signer: Option<&str>, company: &str) -> Item {
        let mut props = vec![("company", company)];
        if let Some(signer) = signer {
            props.push(("signer", signer));
        }
        item(
            "mbamswissarmy",
            Some(r"c:\windows\system32\drivers\mbamswissarmy.sys"),
            &props,
        )
    }

    fn one(item: Item, subject: &Subject) -> Attributed {
        attribute(
            &added(SourceKind::Drivers, vec![item]),
            subject,
            &NoiseList::builtin(),
            &BTreeMap::new(),
        )
        .remove(0)
    }

    #[test]
    fn a_whql_signed_driver_is_the_app_s_by_its_company() {
        let a = one(driver(Some(WHQL), "Malwarebytes"), &subject("Malwarebytes Inc."));
        assert_eq!(
            (a.owner, a.reason.clone()),
            (Owner::App, Reason::CompanyMatchesPublisher)
        );
        assert_eq!(a.reason.text(), copy::WHY_COMPANY_MATCHES);
        // No signature at all: the company still says whose it is.
        assert_eq!(
            one(driver(None, "Malwarebytes Corporation"), &subject("Malwarebytes")).reason,
            Reason::CompanyMatchesPublisher
        );
    }

    #[test]
    fn a_microsoft_company_on_another_app_s_driver_stays_unknown() {
        let a = one(driver(Some(WHQL), "Microsoft Corporation"), &subject("Malwarebytes"));
        assert_eq!(a.owner, Owner::Unknown, "{:?}", a.reason);
    }

    #[test]
    fn a_vendor_signature_wins_over_the_company() {
        // Signed by someone else: the signature is the stronger evidence, so the company isn't asked.
        let a = one(
            driver(Some("Other Vendor Ltd"), "Malwarebytes"),
            &subject("Malwarebytes"),
        );
        assert_eq!(a.owner, Owner::Unknown, "{:?}", a.reason);
    }
}

/// Logitech G HUB in the catalog: its service and drivers were the app's by signer, but its new
/// folders weren't, because "lghub" doesn't contain "logitech".
#[test]
fn a_new_folder_holding_an_app_program_is_the_apps() {
    let ghub = Subject {
        name: "Logitech G HUB".into(),
        publisher: Some("Logitech".into()),
        install_dirs: vec![r"c:\program files\logitech".into()],
        ..Default::default()
    };
    let folder = |key: &str, root: &str| item(key, None, &[("root", root), ("kind", "folder")]);
    let d = Delta {
        sources: BTreeMap::from([
            (
                SourceKind::Services,
                vec![Change::Added(item(
                    "lghub_updater",
                    Some(r"c:\program files\lghub\lghub_updater.exe"),
                    &[("signer", "Logitech")],
                ))],
            ),
            (
                SourceKind::Folders,
                vec![
                    Change::Added(folder(r"c:\program files\lghub", "Program Files")),
                    Change::Added(folder(r"c:\users\a\appdata\local\sentry", r"AppData\Local")),
                    // Too broad to be one app's, however it got on the list.
                    Change::Added(folder(r"c:\program files", "Program Files")),
                ],
            ),
        ]),
        notes: BTreeMap::new(),
        partial: Default::default(),
    };
    let r = attribute(&d, &ghub, &NoiseList::builtin(), &BTreeMap::new());
    assert_eq!(find(&r, "lghub_updater").reason, Reason::SignerMatchesPublisher);
    let lghub = r
        .iter()
        .find(|a| a.change.item().key == r"c:\program files\lghub")
        .unwrap();
    assert_eq!((lghub.owner, &lghub.reason), (Owner::App, &Reason::HoldsAppProgram));
    assert!(
        Reason::HoldsAppProgram
            .text()
            .contains("holds one of the app's programs")
    );
    assert_eq!(find(&r, "sentry").owner, Owner::Unknown);
    let broad = r.iter().find(|a| a.change.item().key == r"c:\program files").unwrap();
    assert_eq!(broad.owner, Owner::Unknown);
}

/// Install dirs for a subject found from one new Programs entry, with these new folders.
fn dirs_with_folders(name: &str, publisher: &str, folders: &[&str]) -> Vec<String> {
    let program = Item {
        label: name.into(),
        ..item(r"hklm\native\app", None, &[("version", "1"), ("publisher", publisher)])
    };
    let changes = vec![Change::Added(program)];
    let d = Delta {
        sources: BTreeMap::from([
            (SourceKind::Programs, changes),
            (
                SourceKind::Folders,
                folders
                    .iter()
                    .map(|f| Change::Added(item(f, None, &[("kind", "folder")])))
                    .collect(),
            ),
        ]),
        notes: BTreeMap::new(),
        partial: Default::default(),
    };
    let subject = find_subject(&d, None, &[]);
    assert_eq!(subject.name, name);
    subject.install_dirs
}

#[test]
fn g_hub_folders_are_found_by_short_words_and_prefixes() {
    let theirs = [
        r"c:\program files\logi",
        r"c:\programdata\logi",
        r"c:\programdata\lghub",
        r"c:\users\a\appdata\local\lghub",
        r"c:\programdata\lghubdata",
        r"c:\users\a\appdata\roaming\g hub",
        r"c:\users\a\appdata\roaming\lghub",
    ];
    let not_theirs = [
        r"c:\programdata\logishrd",
        r"c:\users\a\appdata\local\sentry",
        r"c:\users\a\appdata\roaming\mozilla",
    ];
    let all: Vec<&str> = theirs.iter().chain(&not_theirs).copied().collect();
    let dirs = dirs_with_folders("Logitech G HUB", "Logitech", &all);
    for f in theirs {
        assert!(dirs.iter().any(|d| d == f), "{f} should be G HUB's: {dirs:?}");
    }
    for f in not_theirs {
        assert!(!dirs.iter().any(|d| d == f), "{f} isn't G HUB's: {dirs:?}");
    }
    let tokens = Subject {
        name: "Logitech G HUB".into(),
        publisher: Some("Logitech".into()),
        ..Default::default()
    }
    .folder_tokens();
    assert!(
        tokens.contains(&"ghub".to_string()) && tokens.contains(&"logitech".to_string()),
        "{tokens:?}"
    );
}

#[test]
fn generic_words_take_no_part_in_folder_names() {
    // "media" is generic, so a folder "medi" isn't VLC's even though it starts the word.
    let dirs = dirs_with_folders("VLC media player", "VideoLAN", &[r"c:\users\a\appdata\local\medi"]);
    assert!(dirs.is_empty(), "{dirs:?}");
    // Nor does Microsoft: "micro" doesn't start any of Teams' tokens.
    let dirs = dirs_with_folders(
        "Microsoft Teams",
        "Microsoft Corporation",
        &[r"c:\programdata\micro", r"c:\users\a\appdata\local\micr"],
    );
    assert!(dirs.is_empty(), "{dirs:?}");
    // A short folder name is never a prefix match: "log" is too short to stand for "logitech".
    let dirs = dirs_with_folders("Logitech G HUB", "Logitech", &[r"c:\programdata\log"]);
    assert!(dirs.is_empty(), "{dirs:?}");
}

/// Installers bring Microsoft's redistributables along: Epic's adds "GameInput Redist Service",
/// signed by Microsoft. A new service, driver or task isn't Windows' just for its signature.
#[test]
fn added_microsoft_signed_services_and_drivers_are_not_hidden() {
    let epic = Subject {
        name: "Epic Games Launcher".into(),
        publisher: Some("Epic Games, Inc.".into()),
        ..Default::default()
    };
    let env = BTreeMap::from([("SystemRoot".to_string(), r"C:\Windows".to_string())]);
    let one = |kind: SourceKind, signer: &str| {
        let d = added(
            kind,
            vec![item(
                "gameinputredist",
                Some(r"c:\program files\microsoft gameinput\gameinputredist.exe"),
                &[("signer", signer)],
            )],
        );
        attribute(&d, &epic, &NoiseList::parse("").unwrap(), &env).remove(0)
    };
    for kind in [
        SourceKind::Services,
        SourceKind::Drivers,
        SourceKind::Tasks,
        SourceKind::Startup,
        SourceKind::Certs,
        SourceKind::Defender,
        SourceKind::Network,
    ] {
        for signer in ["Microsoft Corporation", "Microsoft Windows"] {
            assert_ne!(one(kind, signer).owner, Owner::Windows, "{kind:?} {signer}");
        }
    }
    // The other kinds of new item can still be Windows' by Windows' own signature.
    for kind in [
        SourceKind::Firewall,
        SourceKind::ShellExt,
        SourceKind::Path,
        SourceKind::Packaged,
    ] {
        assert_eq!(
            one(kind, "Microsoft Windows").reason,
            Reason::WindowsComponent,
            "{kind:?}"
        );
    }
}

/// EA's installer adds its own Visual C++ runtime entries (system components) next to "EA app".
#[test]
fn programs_added_by_the_install_are_its_own() {
    let entry = |key: &str, label: &str, props: &[(&str, &str)]| Item {
        label: label.into(),
        ..item(key, None, props)
    };
    let vc = entry(
        r"hklm\wow6432node\{vc2013}",
        "Microsoft Visual C++ 2013 x86 Minimum Runtime - 12.0.40664",
        &[("system_component", "1"), ("publisher", "Microsoft Corporation")],
    );
    let firefox_service = |version: &str| {
        entry(
            r"hklm\native\mozillamaintenanceservice",
            "Mozilla Maintenance Service",
            &[("version", version), ("system_component", "1")],
        )
    };
    let d = Delta {
        sources: BTreeMap::from([(
            SourceKind::Programs,
            vec![
                Change::Added(entry(
                    r"hklm\native\{ea-app}",
                    "EA app",
                    &[("size_kb", "900000"), ("publisher", "Electronic Arts")],
                )),
                Change::Added(vc),
                // Firefox was on the runner already; its updater service changing isn't EA's.
                Change::Modified {
                    before: firefox_service("140.0"),
                    after: firefox_service("141.0"),
                    fields: vec!["version".into()],
                },
            ],
        )]),
        notes: BTreeMap::new(),
        partial: Default::default(),
    };
    let subject = find_subject(&d, None, &[]);
    assert_eq!(subject.name, "EA app");
    assert!(
        subject.also_installed.is_empty(),
        "system components count but aren't named"
    );
    let r = attribute(&d, &subject, &NoiseList::builtin(), &BTreeMap::new());
    let vc = find(&r, "{vc2013}");
    assert_eq!((vc.owner, &vc.reason), (Owner::App, &Reason::InstalledAlongside));
    assert!(Reason::InstalledAlongside.text().contains("added by this install"));
    assert_eq!(find(&r, "{ea-app}").reason, Reason::SubjectEntry);
    assert_eq!(find(&r, "mozillamaintenanceservice").owner, Owner::Unknown);
}
