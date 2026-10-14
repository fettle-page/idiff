use idiff_core::redact::{Redactor, redact_receipt};
use idiff_core::verdict::{Level, Line, Mode, Receipt};

fn line(text: &str, items: &[&str], why: &[&str]) -> Line {
    Line {
        level: Level::Note,
        text: text.into(),
        items: items.iter().map(|s| s.to_string()).collect(),
        why: why.iter().map(|s| s.to_string()).collect(),
    }
}

fn receipt(lines: Vec<Line>, unknown: Vec<Line>, hidden: Vec<Line>) -> Receipt {
    Receipt {
        mode: Mode::Install,
        title: "Discord 1.0.9170: installed".into(),
        lines,
        unknown,
        hidden,
    }
}

#[test]
fn redacts_profile_paths_in_items() {
    let red = Redactor::new(["alex"], "DESKTOP-ABC");
    let path = r"C:\Users\alex\AppData\Local\Discord";
    let mut r = receipt(
        vec![line(
            r"Files only in your user folder (412 MB)",
            &[path],
            &[&format!(r"under {path}")],
        )],
        vec![],
        vec![line(
            "1 Windows background change",
            &[r"c:\users\alex\appdata\local\temp\x"],
            &[],
        )],
    );
    redact_receipt(&mut r, &red);
    assert_eq!(r.lines[0].items, [r"C:\Users\<user>\AppData\Local\Discord"]);
    assert_eq!(r.lines[0].why, [r"under C:\Users\<user>\AppData\Local\Discord"]);
    assert_eq!(r.hidden[0].items, [r"c:\users\<user>\appdata\local\temp\x"]);
    assert_eq!(r.title, "Discord 1.0.9170: installed");
}

#[test]
fn redacts_non_ascii_profile_folder() {
    let red = Redactor::new(["Niccolò"], "PC-UFFICIO");
    let mut r = receipt(
        vec![line(
            r"Adds to PATH: C:\Users\Niccolò\bin",
            &[r"user|c:\users\niccolò\bin"],
            &[],
        )],
        vec![],
        vec![],
    );
    r.title = "Niccolò's tool 1.0: installed".into();
    redact_receipt(&mut r, &red);
    assert_eq!(r.lines[0].text, r"Adds to PATH: C:\Users\<user>\bin");
    assert_eq!(r.lines[0].items, [r"user|c:\users\<user>\bin"]);
    assert_eq!(r.title, "<user>'s tool 1.0: installed");
    let all = serde_json::to_string(&r).unwrap();
    assert!(!all.to_lowercase().contains("niccolò"), "{all}");
}

#[test]
fn redacts_pc_name_in_unknown_section() {
    let red = Redactor::new(["alex"], "DESKTOP-ABC");
    let mut r = receipt(
        vec![],
        vec![line(
            r#"Firewall rule "DESKTOP-ABC remote access""#,
            &[r"{a1}"],
            &[r"S-1-5-21-111-222-333-1001 on desktop-abc"],
        )],
        vec![],
    );
    redact_receipt(&mut r, &red);
    assert_eq!(r.unknown[0].text, r#"Firewall rule "<pc-name> remote access""#);
    assert_eq!(r.unknown[0].why, ["<sid> on <pc-name>"]);
    assert_eq!(r.unknown[0].items, ["{a1}"]);
}
