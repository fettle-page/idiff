use library::findings::findings_md;
use serde_json::{Value, json};
use std::path::Path;

fn line(level: &str, text: &str) -> Value {
    json!({"level": level, "text": text, "items": [], "why": []})
}

fn receipt(mode: &str, lines: &[Value]) -> Value {
    json!({"mode": mode, "title": "t", "lines": lines, "unknown": [], "hidden": []})
}

/// Writes `<root>/<id>/{meta,receipt,leftovers}.json`; `leftovers` is skipped when `None`.
fn app(
    root: &Path,
    id: &str,
    name: &str,
    version: &str,
    failed: Option<&str>,
    lines: &[Value],
    left: Option<&[Value]>,
) {
    let dir = root.join(id);
    std::fs::create_dir_all(&dir).unwrap();
    let meta = json!({
        "id": id, "name": name, "version": version, "date": "2026-10-15", "idiff_version": "0.9.1",
        "runner_image": "win25 1", "run_url": "https://github.com/fettle-page/idiff/actions/runs/7",
        "failed": failed,
    });
    std::fs::write(dir.join("meta.json"), meta.to_string()).unwrap();
    std::fs::write(dir.join("receipt.json"), receipt("Install", lines).to_string()).unwrap();
    if let Some(left) = left {
        std::fs::write(dir.join("leftovers.json"), receipt("Leftovers", left).to_string()).unwrap();
    }
}

fn sample() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    // Nested like `gh run download`: result-<id>/<id>/.
    let at = |id: &str| {
        let p = dir.path().join(format!("result-{id}"));
        std::fs::create_dir_all(&p).unwrap();
        p
    };
    app(
        &at("A.A"),
        "A.A",
        "Alpha",
        "1.0",
        None,
        &[
            line(
                "Warn",
                "Adds 2 background services: one, two (starts with Windows) (full system rights)",
            ),
            line("Warn", "1 scheduled task: runs an updater every day"),
            line("Warn", "Starts with Windows"),
            line("Warn", "Allows incoming connections for alpha.exe"),
            line("Bad", "Adds a trusted root certificate: \"Alpha CA\""),
            line("Warn", "Installs 1 driver: alphadrv"),
        ],
        Some(&[
            line("Bad", "2 MB in AppData\\Local\\Alpha"),
            line("Bad", "512 KB in ProgramData\\Alpha"),
            line("Bad", "Shortcut \"alpha.lnk\" is still there"),
        ]),
    );
    app(
        &at("B.B"),
        "B.B",
        "Beta+",
        "2.0 beta",
        None,
        &[
            line("Warn", "1 scheduled task: runs beta.exe at sign-in"),
            line("Note", "Adds to PATH: x"),
        ],
        Some(&[
            line("Bad", "4 MB in AppData\\Local\\Beta"),
            line("Bad", "Folder \"x\" is still there (empty)"),
        ]),
    );
    // The uninstall didn't finish: its leftovers must not count.
    app(
        &at("C.C"),
        "C.C",
        "Gamma",
        "3",
        Some("leftovers: exit 2"),
        &[line("Warn", "Starts with Windows")],
        Some(&[
            line(
                "Warn",
                "The uninstall was still running after 10 minutes, so this list may include things it would have removed",
            ),
            line("Bad", "900 MB in AppData\\Local\\Gamma"),
        ]),
    );
    app(
        &at("D.D"),
        "D.D",
        "Delta",
        "4",
        None,
        &[line("Good", "No services, drivers or firewall rules")],
        Some(&[line("Good", "Nothing left behind")]),
    );
    // Install failed: not part of the counts.
    let e = at("E.E").join("E.E");
    std::fs::create_dir_all(&e).unwrap();
    std::fs::write(
        e.join("meta.json"),
        json!({"id": "E.E", "name": "Eps", "failed": "install: exit 1"}).to_string(),
    )
    .unwrap();
    dir
}

#[test]
fn counts_and_links_from_receipt_lines() {
    let dir = sample();
    let md = findings_md(dir.path(), None).unwrap();
    for part in [
        "Installed silently with winget on a fresh GitHub Actions runner (win25 1) by idiff 0.9.1 on 2026-10-15",
        "[Workflow run](https://github.com/fettle-page/idiff/actions/runs/7)",
        "What 4 popular Windows apps did",
        "- **1 of 4** added a background service: [Alpha](a.a/1.0.md)",
        "- **2 of 4** added a scheduled task: [Alpha](a.a/1.0.md), [Beta+](b.b/2.0-beta.md)",
    ] {
        assert!(md.contains(part), "missing {part:?} in\n{md}");
    }
    for part in [
        "- **1 of 4** added a scheduled task that runs every day: [Alpha](a.a/1.0.md)",
        "- **2 of 4** added a startup entry: [Alpha](a.a/1.0.md), [Gamma](c.c/3.md)",
        "- **1 of 4** allowed incoming connections through the firewall: [Alpha](a.a/1.0.md)",
        "- **1 of 4** installed a driver: [Alpha](a.a/1.0.md)",
        "- **1 of 4** added a trusted root certificate: [Alpha](a.a/1.0.md)",
        // Gamma's uninstall didn't finish, so only three apps have leftovers checked.
        "- **2 of 3** left something behind: [Alpha](a.a/1.0.md), [Beta+](beta-plus.md)",
    ] {
        let want = part.replace("beta-plus.md", "b.b/2.0-beta.md");
        assert!(md.contains(&want), "missing {want:?} in\n{md}");
    }
    assert!(
        md.contains("- **1 of 4** could not be checked after uninstall: [Gamma](c.c/3.md)"),
        "{md}"
    );
    // Alpha: 2 MB + 512 KB = 2.5 MB; Beta: 4 MB. Median of the two is 3.25 MB, shown as 3 MB.
    assert!(md.contains("median"), "{md}");
    assert!(md.contains("3 MB"), "{md}");
    assert!(md.contains("not counted: Eps"), "{md}");
    assert!(md.contains("1 app did not finish installing"), "{md}");
    for banned in ["bloat", "worst", "best", "score", "rank"] {
        assert!(!md.to_lowercase().contains(banned), "{banned}");
    }
}

#[test]
fn links_follow_the_index_when_a_receipts_dir_is_given() {
    let dir = sample();
    let receipts = tempfile::tempdir().unwrap();
    let index = json!([{
        "id": "A.A", "name": "Alpha", "version": "1.0", "date": "2026-10-15", "warn": 1, "bad": 0,
        "leftovers": 0, "leftovers_checked": true, "changed": false, "path": "a.a/1.0-2.md"
    }]);
    std::fs::write(receipts.path().join("index.json"), index.to_string()).unwrap();
    let md = findings_md(dir.path(), Some(receipts.path())).unwrap();
    assert!(md.contains("[Alpha](a.a/1.0-2.md)"), "{md}");
    assert!(md.contains("[Gamma](c.c/3.md)"), "{md}");
}

#[test]
fn flat_results_dir_and_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    assert!(findings_md(dir.path(), None).is_err());
    app(
        dir.path(),
        "A.A",
        "Alpha",
        "1",
        None,
        &[line("Good", "No drivers")],
        Some(&[line("Good", "Nothing left behind")]),
    );
    let md = findings_md(dir.path(), None).unwrap();
    assert!(md.contains("- **0 of 1** added a background service: none"), "{md}");
    assert!(md.contains("- **0 of 1** left something behind: none"), "{md}");
    // No sizes anywhere: no median claimed.
    assert!(!md.contains("median"), "{md}");
}

#[test]
fn missing_leftovers_file_counts_as_not_checked() {
    let dir = tempfile::tempdir().unwrap();
    app(
        dir.path(),
        "A.A",
        "Alpha",
        "1",
        None,
        &[line("Good", "No drivers")],
        None,
    );
    let md = findings_md(dir.path(), None).unwrap();
    assert!(
        md.contains("could not be checked after uninstall: [Alpha](a.a/1.md)"),
        "{md}"
    );
}
