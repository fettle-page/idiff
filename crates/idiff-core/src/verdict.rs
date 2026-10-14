//! The verdict: the receipt's lines, in the order and words of spec §7, from attributed changes.

use crate::attribute::{Attributed, Owner, Reason};
use crate::copy;
use crate::diff::Change;
use crate::model::{Env, Item, Snapshot, SourceKind};
use crate::subject::Subject;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Good,
    Note,
    Warn,
    Bad,
}

/// One receipt line, with the keys of the items it's about and why each was the app's.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub level: Level,
    pub text: String,
    pub items: Vec<String>,
    pub why: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Install,
    Leftovers,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    pub mode: Mode,
    pub title: String,
    pub lines: Vec<Line>,
    /// Changes nobody could be told apart as the app's or Windows'.
    pub unknown: Vec<Line>,
    /// Windows' own changes, shown with `--all`.
    pub hidden: Vec<Line>,
}

/// Fills `{name}` parts of a copy template.
fn fill(template: &str, values: &[(&str, &str)]) -> String {
    values.iter().fold(template.to_string(), |text, (name, value)| {
        text.replace(&format!("{{{name}}}"), value)
    })
}

fn plain(level: Level, text: impl Into<String>) -> Line {
    Line {
        level,
        text: text.into(),
        items: Vec::new(),
        why: Vec::new(),
    }
}

fn about(level: Level, text: impl Into<String>, changes: &[&Attributed]) -> Line {
    Line {
        level,
        text: text.into(),
        items: changes.iter().map(|a| a.change.item().key.clone()).collect(),
        why: changes.iter().map(|a| a.reason.text()).collect(),
    }
}

/// One of `one` and `many`, with `{n}` filled in.
fn counted(n: usize, one: &str, many: &str) -> String {
    fill(if n == 1 { one } else { many }, &[("n", &n.to_string())])
}

fn unique_join(parts: impl IntoIterator<Item = String>, separator: &str) -> String {
    let mut seen: Vec<String> = Vec::new();
    for p in parts {
        if !seen.contains(&p) {
            seen.push(p);
        }
    }
    seen.join(separator)
}

/// `KB`, `MB` or `GB` in base 1024. GB has one decimal under 10 GB, so 1.5 GB doesn't read as 2.
pub fn format_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    let b = bytes as f64;
    if b < KB * KB {
        format!("{} KB", (b / KB).ceil() as u64)
    } else if b < KB * KB * KB {
        format!("{} MB", (b / (KB * KB)).round() as u64)
    } else if b < 10.0 * KB * KB * KB {
        format!("{:.1} GB", b / (KB * KB * KB))
    } else {
        format!("{} GB", (b / (KB * KB * KB)).round() as u64)
    }
}

fn prop<'a>(item: &'a Item, name: &str) -> Option<&'a str> {
    item.props.get(name).map(String::as_str)
}

fn file_name(path: &str) -> &str {
    path.rsplit(['\\', '/']).next().unwrap_or(path)
}

/// What a scheduled task runs, and when: "runs an updater every day", "runs foo.exe at sign-in".
pub fn trigger_summary(item: &Item) -> String {
    let exe = item.target.as_deref().map(file_name).filter(|e| !e.is_empty());
    let what = match exe {
        Some(exe) if exe.to_lowercase().contains("update") => copy::RUNS_UPDATER.to_string(),
        Some(exe) => fill(copy::RUNS_FILE, &[("exe", exe)]),
        None => copy::RUNS_SOMETHING.to_string(),
    };
    let first_trigger = prop(item, "triggers").and_then(|t| t.split(',').next()).unwrap_or("");
    let when = match first_trigger {
        "Daily" => copy::WHEN_DAILY,
        "Weekly" => copy::WHEN_WEEKLY,
        "Logon" => copy::WHEN_LOGON,
        "Boot" => copy::WHEN_BOOT,
        "Idle" => copy::WHEN_IDLE,
        "Time" => copy::WHEN_ONCE,
        _ => copy::WHEN_OTHER,
    };
    let user = prop(item, "user").unwrap_or("");
    let as_system = ["S-1-5-18", "SYSTEM", r"NT AUTHORITY\SYSTEM"]
        .iter()
        .any(|s| user.eq_ignore_ascii_case(s));
    format!("{what} {when}{}", if as_system { copy::AS_SYSTEM } else { "" })
}

fn is_shortcut(item: &Item) -> bool {
    prop(item, "kind") == Some("shortcut") || item.key.ends_with(".lnk")
}

/// "Scheduled task", "Shortcut", … for one item of a source.
fn kind_name(kind: SourceKind, item: &Item) -> &'static str {
    match kind {
        SourceKind::Programs => copy::KIND_PROGRAM,
        SourceKind::Services => copy::KIND_SERVICE,
        SourceKind::Drivers => copy::KIND_DRIVER,
        SourceKind::Tasks => copy::KIND_TASK,
        SourceKind::Startup => copy::KIND_STARTUP,
        SourceKind::Firewall => copy::KIND_FIREWALL,
        SourceKind::ShellExt => copy::KIND_SHELL_EXT,
        SourceKind::Path => copy::KIND_PATH,
        SourceKind::Folders if is_shortcut(item) => copy::KIND_SHORTCUT,
        SourceKind::Folders => copy::KIND_FOLDER,
        SourceKind::Packaged => copy::KIND_PACKAGED,
        SourceKind::Certs => copy::KIND_CERT,
        SourceKind::Defender => copy::KIND_DEFENDER,
        SourceKind::Network => copy::KIND_NETWORK,
    }
}

/// The kind name inside a sentence: "scheduled task", but "PATH entry".
fn kind_in_sentence(kind: SourceKind, item: &Item) -> String {
    let name = kind_name(kind, item);
    let mut chars = name.chars();
    match (chars.next(), chars.next()) {
        (Some(first), Some(second)) if !second.is_uppercase() => {
            first.to_lowercase().chain(name[1..].chars()).collect()
        }
        _ => name.to_string(),
    }
}

/// The name people know an item by: a task's name without its folder, else the label.
fn display_label(kind: SourceKind, item: &Item) -> &str {
    let label = if item.label.is_empty() { &item.key } else { &item.label };
    match kind {
        SourceKind::Tasks => label.rsplit('\\').next().unwrap_or(label),
        _ => label,
    }
}

/// "Added service "Foo Service"" and the like, for the unknown and hidden lists.
fn change_line(a: &Attributed) -> Line {
    let template = match a.change {
        Change::Added(_) => copy::CHANGE_ADDED,
        Change::Removed(_) => copy::CHANGE_REMOVED,
        Change::Modified { .. } => copy::CHANGE_MODIFIED,
    };
    let item = a.change.item();
    let text = fill(
        template,
        &[
            ("kind", &kind_in_sentence(a.kind, item)),
            ("label", display_label(a.kind, item)),
        ],
    );
    about(Level::Note, text, &[a])
}

#[derive(PartialEq)]
enum FolderPlace {
    ProgramFiles,
    ProgramData,
    User,
}

fn folder_place(item: &Item, env: &Env) -> FolderPlace {
    match prop(item, "root") {
        Some("Program Files" | "Program Files (x86)") => FolderPlace::ProgramFiles,
        Some("ProgramData") => FolderPlace::ProgramData,
        Some(_) => FolderPlace::User,
        // Without a root, the environment says where it is.
        None => {
            let under = |var: &str| {
                env.get(var)
                    .map(|v| v.trim_end_matches('\\').to_lowercase())
                    .is_some_and(|dir| item.key.starts_with(&format!("{dir}\\")))
            };
            if under("ProgramFiles") || under("ProgramFiles(x86)") {
                FolderPlace::ProgramFiles
            } else if under("ProgramData") {
                FolderPlace::ProgramData
            } else {
                FolderPlace::User
            }
        }
    }
}

/// `{size}` for folders that enrichment measured; folders it didn't measure add nothing.
fn folders_text(template: &str, folders: &[&Attributed]) -> String {
    let sizes: Vec<u64> = folders
        .iter()
        .filter_map(|a| prop(a.change.item(), "bytes").and_then(|b| b.parse().ok()))
        .collect();
    if sizes.is_empty() {
        template.replace(" ({size})", "")
    } else {
        fill(template, &[("size", &format_size(sizes.iter().sum()))])
    }
}

fn title(subject: &Subject) -> String {
    match (&subject.previous_version, &subject.version) {
        (Some(previous), Some(version)) => fill(
            copy::TITLE_UPDATED,
            &[("name", &subject.name), ("previous", previous), ("version", version)],
        ),
        _ => fill(copy::TITLE_INSTALLED, &[("name", &name_and_version(subject))]),
    }
}

/// The name with its version, unless the name has it already ("7-Zip 24.08 (x64)").
fn name_and_version(subject: &Subject) -> String {
    match &subject.version {
        Some(v) if !subject.name.contains(v.as_str()) => format!("{} {v}", subject.name),
        _ => subject.name.clone(),
    }
}

/// "a, b or c"
fn join_or(parts: &[&str]) -> String {
    match parts.split_last() {
        Some((last, [])) => last.to_string(),
        Some((last, rest)) => format!("{}{}{last}", rest.join(", "), copy::NONE_LAST_JOIN),
        None => String::new(),
    }
}

/// The receipt for an install: the app's changes as findings, then what couldn't be checked and
/// how much was hidden as Windows'.
pub fn install_receipt(
    subject: &Subject,
    attributed: &[Attributed],
    notes: &BTreeMap<SourceKind, String>,
    partial: &BTreeSet<SourceKind>,
    exit_code: Option<i32>,
    env: &Env,
) -> Receipt {
    let app: Vec<&Attributed> = attributed.iter().filter(|a| a.owner == Owner::App).collect();
    let added = |kind: SourceKind| -> Vec<&Attributed> {
        app.iter()
            .copied()
            .filter(|a| a.kind == kind && matches!(a.change, Change::Added(_)))
            .collect()
    };
    let labels = |list: &[&Attributed]| {
        unique_join(
            list.iter().map(|a| display_label(a.kind, a.change.item()).to_string()),
            ", ",
        )
    };
    let mut lines = Vec::new();

    match exit_code {
        None | Some(0) => {}
        // Windows Installer's "restart required" codes: the install worked.
        Some(3010 | 1641) => lines.push(plain(Level::Note, copy::RESTART_NEEDED)),
        Some(code) => lines.push(plain(Level::Warn, fill(copy::EXIT_CODE, &[("n", &code.to_string())]))),
    }

    for a in added(SourceKind::Certs) {
        let subject_name = display_label(a.kind, a.change.item());
        lines.push(about(
            Level::Bad,
            fill(copy::CERT_ADDED, &[("subject", subject_name)]),
            &[a],
        ));
    }
    for a in added(SourceKind::Defender) {
        let path = display_label(a.kind, a.change.item());
        lines.push(about(
            Level::Bad,
            fill(copy::DEFENDER_EXCLUSION, &[("path", path)]),
            &[a],
        ));
    }
    let network: Vec<&Attributed> = app
        .iter()
        .copied()
        .filter(|a| a.kind == SourceKind::Network && !matches!(a.change, Change::Removed(_)))
        .collect();
    if !network.is_empty() {
        let what = unique_join(
            network.iter().map(|a| {
                match a.change.item().key.split('|').next() {
                    Some("hosts") => copy::NETWORK_HOSTS,
                    Some("winsock") => copy::NETWORK_WINSOCK,
                    _ => copy::NETWORK_PROXY,
                }
                .to_string()
            }),
            ", ",
        );
        lines.push(about(
            Level::Bad,
            fill(copy::NETWORK_CHANGED, &[("what", &what)]),
            &network,
        ));
    }

    let drivers = added(SourceKind::Drivers);
    if !drivers.is_empty() {
        let text = fill(
            &counted(drivers.len(), copy::DRIVERS_ONE, copy::DRIVERS_MANY),
            &[("names", &labels(&drivers))],
        );
        lines.push(about(Level::Warn, text, &drivers));
    }
    let services = added(SourceKind::Services);
    if !services.is_empty() {
        let mut text = fill(
            &counted(services.len(), copy::SERVICES_ONE, copy::SERVICES_MANY),
            &[("names", &labels(&services))],
        );
        let any = |test: fn(&Item) -> bool| services.iter().any(|a| test(a.change.item()));
        if any(|i| prop(i, "start") == Some("2")) {
            text.push_str(copy::SERVICE_STARTS_WITH_WINDOWS);
        }
        if any(|i| prop(i, "account").is_some_and(|acc| acc.eq_ignore_ascii_case("LocalSystem"))) {
            text.push_str(copy::SERVICE_FULL_RIGHTS);
        }
        lines.push(about(Level::Warn, text, &services));
    }
    let tasks = added(SourceKind::Tasks);
    if !tasks.is_empty() {
        let summary = unique_join(tasks.iter().map(|a| trigger_summary(a.change.item())), "; ");
        let text = fill(
            &counted(tasks.len(), copy::TASKS_ONE, copy::TASKS_MANY),
            &[("summary", &summary)],
        );
        lines.push(about(Level::Warn, text, &tasks));
    }
    let packaged = added(SourceKind::Packaged);
    let with_extension = |ext: &str| -> Vec<&Attributed> {
        packaged
            .iter()
            .copied()
            .filter(|a| prop(a.change.item(), "extensions").is_some_and(|e| e.split(',').any(|x| x == ext)))
            .collect()
    };
    let startup: Vec<&Attributed> = added(SourceKind::Startup)
        .into_iter()
        .chain(with_extension("startup"))
        .collect();
    if !startup.is_empty() {
        lines.push(about(Level::Warn, copy::STARTS_WITH_WINDOWS, &startup));
    }

    let firewall = added(SourceKind::Firewall);
    let program = |a: &Attributed| {
        let item = a.change.item();
        item.target.as_deref().map(file_name).unwrap_or(&item.label).to_string()
    };
    let inbound: Vec<&Attributed> = firewall
        .iter()
        .copied()
        .filter(|a| prop(a.change.item(), "dir") == Some("in") && prop(a.change.item(), "action") == Some("allow"))
        .collect();
    let inbound_programs: Vec<String> = inbound.iter().map(|a| program(a)).collect();
    let outbound: Vec<&Attributed> = firewall
        .iter()
        .copied()
        .filter(|a| prop(a.change.item(), "dir") == Some("out") && !inbound_programs.contains(&program(a)))
        .collect();
    if !inbound.is_empty() {
        let names = unique_join(inbound_programs.iter().cloned(), ", ");
        lines.push(about(
            Level::Warn,
            fill(copy::FIREWALL_INBOUND, &[("program", &names)]),
            &inbound,
        ));
    }
    if !outbound.is_empty() {
        let names = unique_join(outbound.iter().map(|a| program(a)), ", ");
        lines.push(about(
            Level::Note,
            fill(copy::FIREWALL_OUTBOUND, &[("program", &names)]),
            &outbound,
        ));
    }
    // Where those three findings would be, say which of them there are none of.
    if !app.is_empty() {
        let absent: Vec<&str> = [
            (services.is_empty(), copy::NONE_SERVICES),
            (drivers.is_empty(), copy::NONE_DRIVERS),
            (firewall.is_empty(), copy::NONE_FIREWALL),
        ]
        .into_iter()
        .filter_map(|(none, name)| none.then_some(name))
        .collect();
        if !absent.is_empty() {
            lines.push(plain(
                Level::Good,
                fill(copy::NONE_OF, &[("things", &join_or(&absent))]),
            ));
        }
    }

    let shell: Vec<&Attributed> = added(SourceKind::ShellExt)
        .into_iter()
        .chain(with_extension("context-menu"))
        .collect();
    if !shell.is_empty() {
        lines.push(about(Level::Note, copy::SHELL_EXT_ADDED, &shell));
    }
    let path = added(SourceKind::Path);
    if !path.is_empty() {
        lines.push(about(
            Level::Note,
            fill(copy::PATH_ADDED, &[("entries", &labels(&path))]),
            &path,
        ));
    }

    let folders: Vec<&Attributed> = added(SourceKind::Folders)
        .into_iter()
        .filter(|a| !is_shortcut(a.change.item()))
        .collect();
    let in_place = |place: FolderPlace| -> Vec<&Attributed> {
        folders
            .iter()
            .copied()
            .filter(|a| folder_place(a.change.item(), env) == place)
            .collect()
    };
    let (program_files, program_data, user) = (
        in_place(FolderPlace::ProgramFiles),
        in_place(FolderPlace::ProgramData),
        in_place(FolderPlace::User),
    );
    if !program_files.is_empty() {
        let text = folders_text(copy::FILES_PROGRAM_FILES, &program_files);
        lines.push(about(Level::Note, text, &program_files));
    }
    if !program_data.is_empty() {
        let text = folders_text(copy::FILES_PROGRAM_DATA, &program_data);
        lines.push(about(Level::Note, text, &program_data));
    }
    if !user.is_empty() {
        let line = if program_files.is_empty() && program_data.is_empty() {
            about(Level::Good, folders_text(copy::FILES_ONLY_USER, &user), &user)
        } else {
            about(Level::Note, folders_text(copy::FILES_USER, &user), &user)
        };
        lines.push(line);
    }

    let shortcuts: Vec<&Attributed> = added(SourceKind::Folders)
        .into_iter()
        .filter(|a| is_shortcut(a.change.item()))
        .collect();
    for (roots, text) in [
        (["Desktop", "Public Desktop"], copy::SHORTCUT_DESKTOP),
        (["Start menu", "Start menu (all users)"], copy::SHORTCUT_START_MENU),
    ] {
        let these: Vec<&Attributed> = shortcuts
            .iter()
            .copied()
            .filter(|a| prop(a.change.item(), "root").is_some_and(|r| roots.contains(&r)))
            .collect();
        if !these.is_empty() {
            lines.push(about(Level::Note, text, &these));
        }
    }

    if !subject.also_installed.is_empty() {
        let names = subject.also_installed.join(", ");
        lines.push(plain(Level::Note, fill(copy::ALSO_INSTALLED, &[("names", &names)])));
    }
    // The app's own entry changing version is what the title says already.
    for a in app
        .iter()
        .filter(|a| matches!(a.change, Change::Modified { .. }) && a.reason != Reason::SubjectEntry)
    {
        let item = a.change.item();
        let text = fill(
            copy::CHANGES_ITEM,
            &[
                ("kind", &kind_in_sentence(a.kind, item)),
                ("label", display_label(a.kind, item)),
            ],
        );
        lines.push(about(Level::Note, text, &[a]));
    }

    if app.is_empty() {
        lines.push(plain(Level::Good, copy::NOTHING_FOUND));
    }
    for (kind, reason) in notes {
        lines.push(check_line(*kind, reason, partial.contains(kind)));
    }
    let hidden: Vec<Line> = owned_by(attributed, Owner::Windows);
    if !hidden.is_empty() {
        lines.push(plain(Level::Note, hidden_count_text(hidden.len(), false)));
    }

    let unknown = owned_by(attributed, Owner::Unknown);
    // Nothing named the app and nothing changed that could be one: no install to speak of.
    let title = if !subject.is_known() && app.is_empty() && unknown.is_empty() {
        copy::TITLE_NO_INSTALL.to_string()
    } else {
        title(subject)
    };
    Receipt {
        mode: Mode::Install,
        title,
        lines,
        unknown,
        hidden,
    }
}

/// "Couldn't check …" for a source that wasn't read, "Only partly checked …" for one read in part.
fn check_line(kind: SourceKind, reason: &str, partial: bool) -> Line {
    let template = if partial {
        copy::ONLY_PARTLY_CHECKED
    } else {
        copy::COULDNT_CHECK
    };
    plain(
        Level::Note,
        fill(template, &[("source", kind.label()), ("reason", reason)]),
    )
}

/// "3 Windows background changes hidden", with the hint to `idiff show --all`, or with "listed
/// below" where the page shows them.
pub fn hidden_count_text(n: usize, listed: bool) -> String {
    if listed {
        counted(n, copy::HIDDEN_LISTED_BELOW_ONE, copy::HIDDEN_LISTED_BELOW)
    } else {
        counted(n, copy::HIDDEN_ONE, copy::HIDDEN_MANY)
    }
}

fn owned_by(attributed: &[Attributed], owner: Owner) -> Vec<Line> {
    attributed
        .iter()
        .filter(|a| a.owner == owner)
        .map(change_line)
        .collect()
}

/// After an uninstall: what the install added for the app that's still there, and what the
/// uninstaller itself added. `sizes` has the bytes of leftover folders, by key.
pub fn leftovers_receipt(
    subject: &Subject,
    install: &[Attributed],
    uninstalled: &Snapshot,
    during: &[Attributed],
    sizes: &BTreeMap<String, u64>,
) -> Receipt {
    let mut lines = Vec::new();
    // Sources the leftovers couldn't be fully checked in: (reason, read in part).
    let mut unchecked: BTreeMap<SourceKind, (String, bool)> = BTreeMap::new();
    let mut left = 0;
    for a in install
        .iter()
        .filter(|a| a.owner == Owner::App && matches!(a.change, Change::Added(_)))
    {
        let item = a.change.item();
        let present = match uninstalled.sources.get(&a.kind) {
            Some(source) if source.ok => {
                let found = source.items.iter().any(|i| i.key == item.key);
                // Read in part: not finding it doesn't mean it's gone.
                if let (false, Some(note)) = (found, &source.note) {
                    unchecked.entry(a.kind).or_insert((note.clone(), true));
                }
                found
            }
            // Couldn't be read: neither gone nor left, so say it wasn't checked.
            source => {
                let reason = source
                    .and_then(|s| s.error.clone())
                    .unwrap_or_else(|| copy::SOURCE_NOT_READ.to_string());
                unchecked.entry(a.kind).or_insert((reason, false));
                continue;
            }
        };
        if !present {
            continue;
        }
        left += 1;
        let size = sizes
            .get(&item.key)
            .filter(|_| a.kind == SourceKind::Folders && !is_shortcut(item));
        let text = match size {
            Some(&bytes) => fill(
                copy::FOLDER_LEFT,
                &[("size", &format_size(bytes)), ("path", display_label(a.kind, item))],
            ),
            None => fill(
                copy::STILL_THERE,
                &[
                    ("kind", kind_name(a.kind, item)),
                    ("label", display_label(a.kind, item)),
                ],
            ),
        };
        lines.push(about(Level::Bad, text, &[a]));
    }
    for a in during
        .iter()
        .filter(|a| a.owner == Owner::App && matches!(a.change, Change::Added(_)))
    {
        let item = a.change.item();
        let text = fill(
            copy::UNINSTALLER_ADDED,
            &[
                ("kind", &kind_in_sentence(a.kind, item)),
                ("label", display_label(a.kind, item)),
            ],
        );
        lines.push(about(Level::Note, text, &[a]));
    }
    if left == 0 && unchecked.is_empty() {
        lines.push(plain(Level::Good, copy::NOTHING_LEFT));
    }
    for (kind, (reason, partial)) in &unchecked {
        lines.push(check_line(*kind, reason, *partial));
    }

    let name = name_and_version(subject);
    let title = match left {
        0 if unchecked.is_empty() => fill(copy::TITLE_LEFTOVERS_NONE, &[("name", &name)]),
        0 => fill(copy::TITLE_LEFTOVERS_NONE_FOUND, &[("name", &name)]),
        n => fill(
            &counted(n, copy::TITLE_LEFTOVERS_ONE, copy::TITLE_LEFTOVERS_MANY),
            &[("name", &name)],
        ),
    };
    Receipt {
        mode: Mode::Leftovers,
        title,
        lines,
        unknown: owned_by(during, Owner::Unknown),
        hidden: owned_by(during, Owner::Windows),
    }
}
