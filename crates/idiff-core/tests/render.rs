//! Receipts rendered as text, Markdown and HTML, checked against the files in `tests/golden/`.
//! `UPDATE_GOLDEN=1 cargo test -p idiff-core --test render` rewrites them; read every change
//! against spec §1 and §7 before committing it.

use idiff_core::attribute::attribute;
use idiff_core::diff::diff;
use idiff_core::model::Snapshot;
use idiff_core::noise::NoiseList;
use idiff_core::render::{TextOptions, html, markdown, md_escape, text};
use idiff_core::subject::find_subject;
use idiff_core::verdict::{Level, Line, Receipt, install_receipt};
use std::collections::BTreeMap;
use std::path::Path;

fn discord() -> Receipt {
    let before = Snapshot::load(include_str!("fixtures/discord-before.json")).unwrap();
    let after = Snapshot::load(include_str!("fixtures/discord-after.json")).unwrap();
    let d = diff(&before, &after);
    let subject = find_subject(&d, None, &[]);
    let attributed = attribute(&d, &subject, &NoiseList::builtin(), &after.env);
    install_receipt(&subject, &attributed, &BTreeMap::new(), Some(0), &after.env)
}

/// Compares with a golden file, or rewrites it with `UPDATE_GOLDEN=1`. The version is a
/// placeholder in the files, so a release doesn't change them.
fn golden(name: &str, actual: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden").join(name);
    let actual = actual.replace(env!("CARGO_PKG_VERSION"), "{version}");
    if std::env::var_os("UPDATE_GOLDEN").is_some_and(|v| v == "1") {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &actual).unwrap();
        return;
    }
    let want = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: {e} (run with UPDATE_GOLDEN=1 to create it)", path.display()));
    assert_eq!(
        actual,
        want.replace("\r\n", "\n"),
        "{name} differs from its golden file"
    );
}

const PLAIN: TextOptions = TextOptions {
    color: false,
    all: false,
    why: false,
};

#[test]
fn text_matches_golden() {
    golden("discord.txt", &text(&discord(), &PLAIN));
}

#[test]
fn text_with_all_and_why_matches_golden() {
    let o = TextOptions {
        color: false,
        all: true,
        why: true,
    };
    golden("discord-all-why.txt", &text(&discord(), &o));
}

#[test]
fn markdown_matches_golden() {
    golden("discord.md", &markdown(&discord(), true));
}

#[test]
fn html_matches_golden() {
    golden("discord.html", &html(&discord()));
}

#[test]
fn text_starts_like_the_spec_example() {
    let out = text(&discord(), &PLAIN);
    let spec = "Discord 1.0.9170: installed
⚠ 1 scheduled task: runs an updater every day
⚠ Starts with Windows
✓ No services, drivers or firewall rules
✓ Files only in your user folder (412 MB)
· 3 Windows background changes hidden (idiff show --all)
";
    assert!(out.starts_with(spec), "{out}");
}

#[test]
fn colour_only_when_asked() {
    let r = discord();
    assert!(!text(&r, &PLAIN).contains('\x1b'));
    let coloured = text(&r, &TextOptions { color: true, ..PLAIN });
    assert!(
        coloured.contains("\x1b[33m⚠ Starts with Windows\x1b[0m"),
        "{coloured:?}"
    );
    assert!(coloured.contains("\x1b[32m✓ No services"), "{coloured:?}");
    assert!(coloured.contains("\x1b[2m· 3 Windows background"), "{coloured:?}");
}

#[test]
fn unknown_changes_always_show_and_hidden_ones_only_with_all() {
    let r = discord();
    let plain = text(&r, &PLAIN);
    assert!(plain.contains("Also changed while installing (couldn't tell if it was this app)"));
    assert!(plain.contains("Added service \"Foo Service\""));
    assert!(!plain.contains("Hidden: Windows background changes"));
    let all = text(&r, &TextOptions { all: true, ..PLAIN });
    assert!(all.contains("Hidden: Windows background changes"));
    assert!(all.contains("Added driver \"mpkslabc123\""));
}

fn hostile() -> Receipt {
    let line = |text: &str| Line {
        level: Level::Warn,
        text: text.into(),
        items: vec!["<b>key</b>".into()],
        why: vec!["x".into()],
    };
    Receipt {
        mode: idiff_core::verdict::Mode::Install,
        title: "<script>alert(1)</script> & \"co\"".into(),
        lines: vec![line("Adds 1 background service: <script>evil</script>")],
        unknown: vec![line("Added service \"a | b](javascript:x)\"")],
        hidden: vec![],
    }
}

#[test]
fn html_escapes_text() {
    let out = html(&hostile());
    assert!(out.contains("&lt;script&gt;alert(1)&lt;/script&gt; &amp; &quot;co&quot;"));
    assert!(out.contains("&lt;script&gt;evil&lt;/script&gt;"));
    assert!(!out.contains("<script"));
    assert!(!out.contains("<b>"));
}

#[test]
fn html_has_no_external_resources() {
    for out in [html(&discord()), html(&hostile())] {
        let lower = out.to_lowercase();
        for banned in ["http", "<script", "<link", "<img", "url(", "@import", "src="] {
            assert!(!lower.contains(banned), "{banned}");
        }
        assert!(lower.starts_with("<!doctype html>"));
        assert!(lower.contains("prefers-color-scheme: dark"));
    }
}

#[test]
fn md_escape_covers_markdown_syntax() {
    assert_eq!(
        md_escape(r"a\b|c[d]e(f)g*h_i<j>k`l"),
        r"a\\b\|c\[d\]e\(f\)g\*h\_i\<j\>k\`l"
    );
    assert_eq!(md_escape("plain words, 412 MB."), "plain words, 412 MB.");
    assert_eq!(md_escape("two\nlines"), "two lines");
}

#[test]
fn markdown_can_not_be_injected() {
    let out = markdown(&hostile(), true);
    assert!(out.contains(r"a \| b\]\(javascript:x\)"), "{out}");
    assert!(!out.contains("](javascript:"));
    assert!(out.contains(r"\<script\>alert\(1\)\</script\>"), "{out}");
}
