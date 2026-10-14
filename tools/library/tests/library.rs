use idiff_core::verdict::{Level, Line, Mode, Receipt};
use library::{Entry, Meta, Summary, apply, changes_since, index_md, version_page, version_slug};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
}

fn index(receipts: &Path) -> Vec<Entry> {
    serde_json::from_str(&std::fs::read_to_string(receipts.join("index.json")).unwrap()).unwrap()
}

fn read(path: impl AsRef<Path>) -> String {
    std::fs::read_to_string(path.as_ref()).unwrap_or_else(|e| panic!("{}: {e}", path.as_ref().display()))
}

fn modified(path: impl AsRef<Path>) -> SystemTime {
    std::fs::metadata(path).unwrap().modified().unwrap()
}

/// A results dir with one app whose `meta.json` is `meta` and nothing else, as a failed or hung
/// install leaves it.
fn meta_only(meta: serde_json::Value) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let app = dir.path().join(meta["id"].as_str().unwrap());
    std::fs::create_dir_all(&app).unwrap();
    std::fs::write(app.join("meta.json"), meta.to_string()).unwrap();
    dir
}

#[test]
fn new_app_gets_a_page_and_an_index_row() {
    let receipts = tempfile::tempdir().unwrap();
    let summary = apply(&fixture("discord-1.0.9170"), receipts.path()).unwrap();
    assert_eq!(summary.added, ["Discord.Discord"]);
    let app = receipts.path().join("discord.discord");
    for file in ["1.0.9170.md", "1.0.9170.json", "README.md"] {
        assert!(app.join(file).is_file(), "{file}");
    }
    assert_eq!(
        index(receipts.path()),
        [Entry {
            id: "Discord.Discord".into(),
            name: "Discord".into(),
            version: "1.0.9170".into(),
            date: "2026-10-12".into(),
            warn: 2,
            bad: 0,
            leftovers: 2,
            changed: false,
            path: "discord.discord/1.0.9170.md".into(),
        }]
    );
    let page = read(app.join("1.0.9170.md"));
    assert!(
        page.starts_with("# Discord 1.0.9170\n\n## Discord 1.0.9170: installed\n"),
        "{page}"
    );
    for part in [
        "- ⚠ 1 scheduled task: runs an updater every day",
        "## After uninstall",
        "### Discord 1.0.9170: after uninstall, 2 leftovers",
        "- ✗ Scheduled task \"DiscordUpdate\" is still there",
        "## How this was made",
        "Installed with winget on a fresh GitHub Actions runner (win25 20261004.1) by idiff 0.9.0 on 2026-10-12, opened once, then uninstalled.",
        "[Workflow run](https://github.com/fettle-page/idiff/actions/runs/123456)",
        "../../README.md#how-it-decides-whats-the-apps",
    ] {
        assert!(page.contains(part), "missing {part:?} in\n{page}");
    }
    assert!(!page.contains("## Changed since the previous version"));
    // The order the plan gives: receipt, leftovers, how it was made.
    let at = |s: &str| page.find(s).unwrap();
    assert!(at("## Discord 1.0.9170: installed") < at("## After uninstall"));
    assert!(at("## After uninstall") < at("## How this was made"));

    let library = read(receipts.path().join("README.md"));
    assert!(library.contains("What popular Windows apps do to your PC when you install them"));
    assert!(library.contains("| App | Version | ⚠ | ✗ | Leftovers | Changed | Checked |"));
    assert!(
        library.contains("| [Discord](discord.discord/1.0.9170.md) | 1.0.9170 | 2 | 0 | 2 |  | 2026-10-12 |"),
        "{library}"
    );
}

#[test]
fn new_version_with_new_finding_is_flagged() {
    let receipts = tempfile::tempdir().unwrap();
    apply(&fixture("discord-1.0.9170"), receipts.path()).unwrap();
    let summary = apply(&fixture("discord-1.0.9171"), receipts.path()).unwrap();
    assert_eq!(summary.added, ["Discord.Discord"]);
    let entries = index(receipts.path());
    assert_eq!(entries.len(), 1);
    assert_eq!((entries[0].version.as_str(), entries[0].changed), ("1.0.9171", true));
    let page = read(receipts.path().join("discord.discord/1.0.9171.md"));
    assert!(page.contains("## Changed since the previous version"), "{page}");
    assert!(
        page.contains("+ 2 scheduled tasks: runs crashreporter.exe at sign-in; runs an updater every day"),
        "{page}"
    );
    assert!(page.contains("- 1 scheduled task: runs an updater every day"), "{page}");
    // The app's own page is the newest version, with the older one linked.
    let readme = read(receipts.path().join("discord.discord/README.md"));
    assert!(readme.starts_with("# Discord 1.0.9171\n"), "{readme}");
    assert!(readme.contains("[1.0.9170](1.0.9170.md)"), "{readme}");
    assert!(receipts.path().join("discord.discord/1.0.9170.md").is_file());
}

#[test]
fn same_version_is_skipped() {
    let receipts = tempfile::tempdir().unwrap();
    apply(&fixture("discord-1.0.9170"), receipts.path()).unwrap();
    let files = [
        receipts.path().join("discord.discord/1.0.9170.md"),
        receipts.path().join("discord.discord/README.md"),
        receipts.path().join("README.md"),
        receipts.path().join("index.json"),
    ];
    let before: Vec<SystemTime> = files.iter().map(modified).collect();
    std::thread::sleep(std::time::Duration::from_millis(50));
    let summary = apply(&fixture("discord-1.0.9170"), receipts.path()).unwrap();
    assert_eq!(summary.skipped_same, ["Discord.Discord"]);
    assert!(summary.added.is_empty());
    let after: Vec<SystemTime> = files.iter().map(modified).collect();
    assert_eq!(before, after, "no file was rewritten");
}

#[test]
fn failed_app_keeps_previous_entry() {
    let receipts = tempfile::tempdir().unwrap();
    apply(&fixture("discord-1.0.9170"), receipts.path()).unwrap();
    let before = index(receipts.path());
    let results = meta_only(serde_json::json!({
        "id": "Discord.Discord", "name": "Discord", "version": "1.0.9171", "date": "2026-10-19",
        "idiff_version": "0.9.0", "runner_image": "win25 20261004.1",
        "run_url": "https://github.com/fettle-page/idiff/actions/runs/2", "failed": "run: exit 2"
    }));
    let summary = apply(results.path(), receipts.path()).unwrap();
    assert_eq!(
        summary.failed,
        [("Discord.Discord".to_string(), "run: exit 2".to_string())]
    );
    assert_eq!(index(receipts.path()), before);
    assert!(!receipts.path().join("discord.discord/1.0.9171.md").exists());
}

#[test]
fn a_hung_install_leaves_only_meta_and_counts_as_failed() {
    let receipts = tempfile::tempdir().unwrap();
    apply(&fixture("discord-1.0.9170"), receipts.path()).unwrap();
    let before = index(receipts.path());
    // The job was cancelled before the runner script could write anything but meta.json.
    let results = meta_only(serde_json::json!({ "id": "Discord.Discord", "name": "Discord" }));
    let summary = apply(results.path(), receipts.path()).unwrap();
    assert_eq!(summary.failed.len(), 1);
    assert_eq!(summary.failed[0].0, "Discord.Discord");
    assert!(summary.failed[0].1.contains("receipt.json"), "{:?}", summary.failed);
    assert_eq!(index(receipts.path()), before);
}

#[test]
fn a_result_dir_without_meta_is_reported_not_fatal() {
    let receipts = tempfile::tempdir().unwrap();
    let results = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(results.path().join("Broken.App")).unwrap();
    let summary = apply(results.path(), receipts.path()).unwrap();
    assert_eq!(summary.failed.len(), 1);
    assert_eq!(summary.failed[0].0, "Broken.App");
    assert!(summary.failed[0].1.contains("meta.json"));
}

#[test]
fn index_is_sorted_by_name() {
    let receipts = tempfile::tempdir().unwrap();
    // A second app: the Discord result under another id and name.
    let results = tempfile::tempdir().unwrap();
    let src = fixture("discord-1.0.9170").join("Discord.Discord");
    let dst = results.path().join("Audacity.Audacity");
    std::fs::create_dir_all(&dst).unwrap();
    for file in ["receipt.json", "leftovers.json"] {
        std::fs::copy(src.join(file), dst.join(file)).unwrap();
    }
    let mut meta: serde_json::Value = serde_json::from_str(&read(src.join("meta.json"))).unwrap();
    meta["id"] = "Audacity.Audacity".into();
    meta["name"] = "Audacity".into();
    std::fs::write(dst.join("meta.json"), meta.to_string()).unwrap();
    apply(&fixture("discord-1.0.9170"), receipts.path()).unwrap();
    apply(results.path(), receipts.path()).unwrap();
    let names: Vec<String> = index(receipts.path()).into_iter().map(|e| e.name).collect();
    assert_eq!(names, ["Audacity", "Discord"]);
}

#[test]
fn version_paths_are_safe() {
    assert_eq!(version_slug("1.0.0+build/7"), "1.0.0-build-7");
    assert_eq!(version_slug(""), "unknown");
    assert_eq!(version_slug("Latest"), "latest");
    assert_eq!(version_slug("1.0.9170"), "1.0.9170");
    // Never a path, a hidden file or a name Windows trims.
    assert_eq!(version_slug(".."), "unknown");
    let up = version_slug("../../etc");
    assert!(
        !up.contains('/') && !up.starts_with('.') && !up.starts_with('-'),
        "{up}"
    );
    assert_eq!(version_slug("2.0."), "2.0");
}

#[test]
fn versions_with_the_same_slug_get_their_own_pages() {
    let receipts = tempfile::tempdir().unwrap();
    for (version, date) in [("1.0+a", "2026-10-12"), ("1.0/a", "2026-10-19")] {
        let results = tempfile::tempdir().unwrap();
        let src = fixture("discord-1.0.9170").join("Discord.Discord");
        let dst = results.path().join("Discord.Discord");
        std::fs::create_dir_all(&dst).unwrap();
        for file in ["receipt.json", "leftovers.json"] {
            std::fs::copy(src.join(file), dst.join(file)).unwrap();
        }
        let mut meta: serde_json::Value = serde_json::from_str(&read(src.join("meta.json"))).unwrap();
        meta["version"] = version.into();
        meta["date"] = date.into();
        std::fs::write(dst.join("meta.json"), meta.to_string()).unwrap();
        let summary = apply(results.path(), receipts.path()).unwrap();
        assert_eq!(summary.added, ["Discord.Discord"], "{version}");
    }
    let app = receipts.path().join("discord.discord");
    assert!(app.join("1.0-a.md").is_file());
    assert!(app.join("1.0-a-2.md").is_file());
    assert_eq!(index(receipts.path())[0].path, "discord.discord/1.0-a-2.md");
}

fn line(level: Level, text: &str) -> Line {
    Line {
        level,
        text: text.into(),
        items: vec![],
        why: vec![],
    }
}

fn receipt(lines: Vec<Line>) -> Receipt {
    Receipt {
        mode: Mode::Install,
        title: "X 1: installed".into(),
        lines,
        unknown: vec![],
        hidden: vec![],
    }
}

#[test]
fn changes_compare_findings_not_runner_noise() {
    let prev = receipt(vec![
        line(Level::Warn, "1 scheduled task: runs an updater every day"),
        line(Level::Note, "3 Windows background changes hidden (idiff show --all)"),
        line(Level::Note, "Couldn't check antivirus exclusions: needs administrator"),
    ]);
    let cur = receipt(vec![
        line(Level::Warn, "1 scheduled task: runs an updater every day"),
        line(Level::Warn, "Starts with Windows"),
        line(Level::Note, "7 Windows background changes hidden (idiff show --all)"),
        line(
            Level::Note,
            "Only partly checked scheduled tasks: some tasks are only visible to administrators",
        ),
    ]);
    assert_eq!(changes_since(&prev, &cur), ["+ Starts with Windows"]);
    assert!(changes_since(&cur, &cur).is_empty());
    assert_eq!(changes_since(&cur, &prev), ["- Starts with Windows"]);
}

#[test]
fn labels_are_escaped_in_markdown() {
    let evil = "a | b](javascript:x)";
    let table = index_md(&[Entry {
        id: "Evil.App".into(),
        name: evil.into(),
        version: "1|2".into(),
        date: "2026-10-12".into(),
        warn: 1,
        bad: 0,
        leftovers: 0,
        changed: true,
        path: "evil.app/1-2.md".into(),
    }]);
    assert!(table.contains(r"a \| b\]\(javascript:x\)"), "{table}");
    assert!(table.contains(r"1\|2"), "{table}");
    assert!(!table.contains("](javascript:"), "{table}");

    let meta = Meta {
        id: "Evil.App".into(),
        name: evil.into(),
        version: "1".into(),
        date: "2026-10-12".into(),
        idiff_version: "0.9.0".into(),
        runner_image: "win25".into(),
        run_url: "javascript:alert(1)".into(),
        failed: None,
    };
    let page = version_page(
        &meta,
        &receipt(vec![line(Level::Warn, evil)]),
        None,
        &[format!("+ {evil}")],
    );
    assert!(!page.contains("](javascript:"), "{page}");
    assert!(!page.contains("(javascript:alert"), "{page}");
    assert!(page.contains(r"a \| b\]\(javascript:x\)"), "{page}");
}

#[test]
fn the_tool_prints_the_summary_for_the_pr() {
    let receipts = tempfile::tempdir().unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_library"))
        .arg(fixture("discord-1.0.9170"))
        .arg(receipts.path())
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("Discord 1.0.9170"), "{text}");
    let summary = Summary {
        added: vec!["A.B".into()],
        skipped_same: vec!["C.D".into()],
        failed: vec![("E.F".into(), "run: exit 2".into())],
    };
    let md = summary.to_markdown();
    assert!(
        md.contains("C.D") && md.contains("E.F") && md.contains("run: exit 2"),
        "{md}"
    );
}
