//! The subject: whose install this is, and where it put its files.

use crate::copy;
use crate::diff::{Change, Delta};
use crate::model::{Item, SourceKind};
use crate::noise::NoiseList;
use crate::publisher;
use serde::{Deserialize, Serialize};

/// Words too common in app and publisher names to tell apps apart.
const GENERIC: &[&str] = &[
    "microsoft",
    "corporation",
    "software",
    "windows",
    "media",
    "player",
    "free",
    "edition",
    "version",
    "setup",
];

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct Subject {
    pub name: String,
    pub version: Option<String>,
    /// Set when the app was already installed and this install changed its version.
    pub previous_version: Option<String>,
    pub publisher: Option<String>,
    /// Normalized folders that are the app's: its install location, plus new top-level folders
    /// that hold it or are named after the app or its publisher.
    pub install_dirs: Vec<String>,
    pub also_installed: Vec<String>,
    /// Keys of the Programs or Packaged entries this install added (or upgraded).
    pub entries: Vec<String>,
    /// The key of the app's own entry among `entries`: the one a real uninstall always removes.
    /// Runtimes installed alongside it can stay behind without meaning the uninstall didn't run.
    pub own_entry: Option<String>,
}

impl Subject {
    /// False for `copy::UNKNOWN_APP`: nothing said which app this was.
    pub fn is_known(&self) -> bool {
        !self.name.is_empty() && self.name != copy::UNKNOWN_APP
    }

    /// What a folder named after the app contains: words of 4+ characters from the name and
    /// publisher that aren't generic, plus the name's first word if it has 3+.
    pub fn folder_tokens(&self) -> Vec<String> {
        if !self.is_known() {
            return Vec::new();
        }
        let publisher = self.publisher.as_deref().map(publisher::normalize).unwrap_or_default();
        let mut tokens: Vec<String> = words(&self.name)
            .chain(words(&publisher))
            .filter(|w| is_token(w, 4))
            .collect();
        tokens.extend(words(&self.name).next().filter(|w| is_token(w, 3)));
        // Short words read together: "Logitech G HUB" is in folders as "ghub" and "lghub".
        let joined: String = words(&self.name)
            .filter(|w| w.chars().count() < 4 && !GENERIC.contains(&w.as_str()))
            .collect();
        tokens.extend(Some(joined).filter(|j| j.chars().count() >= 4));
        tokens.sort();
        tokens.dedup();
        tokens
    }

    /// The word of the name that R6 looks for in keys and labels.
    pub fn name_token(&self) -> Option<String> {
        if !self.is_known() {
            return None;
        }
        words(&self.name).find(|w| is_token(w, 4))
    }
}

fn words(s: &str) -> impl Iterator<Item = String> + '_ {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
}

fn is_token(word: &str, min_chars: usize) -> bool {
    word.chars().count() >= min_chars && !word.chars().all(|c| c.is_ascii_digit()) && !GENERIC.contains(&word)
}

/// The package id in a `winget install|upgrade` command: `--id X`, else the query or first
/// plain argument.
pub fn winget_id(command: &[String]) -> Option<String> {
    /// winget options that take a value, so the value isn't mistaken for the package.
    const VALUE_FLAGS: &[&str] = &[
        "-v",
        "--version",
        "-s",
        "--source",
        "--scope",
        "-a",
        "--architecture",
        "--installer-type",
        "-l",
        "--location",
        "-o",
        "--log",
        "--override",
        "--custom",
        "--locale",
        "--header",
        "--authentication-mode",
        "--authentication-account",
        "-r",
        "--rename",
        "-m",
        "--manifest",
        "-n",
        "--name",
        "--moniker",
        "--tag",
        "--cmd",
        "--command",
        "--dependency-source",
    ];
    let (exe, rest) = command.split_first()?;
    let file = exe.rsplit(['\\', '/']).next().unwrap_or(exe).to_ascii_lowercase();
    if file != "winget" && file != "winget.exe" {
        return None;
    }
    let (verb, args) = rest.split_first()?;
    if !["install", "add", "upgrade", "update"].contains(&verb.to_ascii_lowercase().as_str()) {
        return None;
    }
    let mut query = None;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        if arg.starts_with('-')
            && let Some((flag, value)) = arg.split_once('=')
        {
            match flag.to_ascii_lowercase().as_str() {
                "--id" => return Some(value.to_string()),
                "-q" | "--query" => query = query.or(Some(value.to_string())),
                _ => {}
            }
            continue;
        }
        let lower = arg.to_ascii_lowercase();
        if lower == "--id" {
            return args.next().cloned();
        } else if lower == "-q" || lower == "--query" {
            query = query.or(args.next().cloned());
        } else if VALUE_FLAGS.contains(&lower.as_str()) {
            args.next();
        } else if !arg.starts_with('-') {
            query = query.or(Some(arg.clone()));
        }
    }
    query
}

fn size_kb(item: &Item) -> u64 {
    item.props.get("size_kb").and_then(|s| s.parse().ok()).unwrap_or(0)
}

fn prop(item: &Item, name: &str) -> Option<String> {
    item.props.get(name).filter(|v| !v.trim().is_empty()).cloned()
}

/// Paths in normalized form: lowercase, no trailing backslash.
fn is_under(path: &str, dir: &str) -> bool {
    path == dir || path.strip_prefix(dir).is_some_and(|rest| rest.starts_with('\\'))
}

/// Folder tokens from the winget id's parts, for apps whose name says little: "EA app" is
/// `ElectronicArts.EADesktop`, giving `electronicarts` and `eadesktop`. Same rules as the
/// name's tokens: letters and digits only, 4+ characters, never a generic word.
fn winget_id_tokens(command: &[String]) -> Vec<String> {
    let Some(id) = winget_id(command) else {
        return Vec::new();
    };
    id.split('.')
        .map(|part| {
            part.chars()
                .filter(|c| c.is_alphanumeric())
                .flat_map(char::to_lowercase)
                .collect::<String>()
        })
        .filter(|part| is_token(part, 4))
        .collect()
}

/// A folder's letters-only name `leaf` is the app's when it contains a token (a word of the name
/// is part of the letters-only form, so this covers both), or when it's at least 4 characters
/// and starts a token ("logi" for "logitech"). Tokens never include generic words.
fn is_named_after(leaf: &str, tokens: &[String]) -> bool {
    tokens
        .iter()
        .any(|t| leaf.contains(t.as_str()) || (leaf.chars().count() >= 4 && t.starts_with(leaf)))
}

/// A folder too broad to be one app's, which some installers still write as their location.
pub(crate) fn is_too_broad(dir: &str) -> bool {
    let parts: Vec<&str> = dir.split('\\').filter(|p| !p.is_empty()).collect();
    let tail = |n: usize| parts[parts.len().saturating_sub(n)..].join("\\");
    parts.len() < 2
        || (parts.len() == 2
            && [
                "program files",
                "program files (x86)",
                "programdata",
                "windows",
                "users",
            ]
            .contains(&parts[1]))
        || (parts.len() == 3 && parts[1] == "users")
        || ["appdata\\local", "appdata\\roaming"].contains(&tail(2).as_str())
        || tail(3) == "appdata\\local\\programs"
        // Where installers keep what many apps share: Windows Installer bundles cache themselves
        // in `Package Cache`, and Inno Setup puts per-user shared files in `Programs\Common`.
        || tail(2) == "programdata\\package cache"
        || tail(3) == "appdata\\local\\package cache"
        || tail(4) == "appdata\\local\\programs\\common"
}

/// Whose install this is (spec order: `--app`, new Programs entries, new Store apps, the winget
/// id, else unknown), with its install folders. Entries the noise list matches are never the
/// app: winget refreshing its own source package during a failed install once became "the app",
/// and its Microsoft publisher then made a Windows change the app's.
pub fn find_subject(delta: &Delta, app_flag: Option<&str>, command: &[String], noise: &NoiseList) -> Subject {
    let nobody = Subject::default();
    let changes = |kind| -> Vec<&Change> {
        delta
            .sources
            .get(&kind)
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .filter(|c| noise.matches_change(kind, c, &nobody).is_none())
            .collect()
    };
    let visible = |i: &Item| i.props.get("system_component").map(String::as_str) != Some("1");

    let mut candidates: Vec<(&Item, Option<String>)> = changes(SourceKind::Programs)
        .iter()
        .filter_map(|c| match c {
            Change::Added(i) if visible(i) => Some((i, None)),
            _ => None,
        })
        .collect();
    if candidates.is_empty() {
        // An upgrade: the program was there already, and its version changed.
        candidates = changes(SourceKind::Programs)
            .iter()
            .filter_map(|c| match c {
                Change::Modified { before, after, fields }
                    if visible(after) && fields.iter().any(|f| f == "version") =>
                {
                    Some((after, prop(before, "version")))
                }
                _ => None,
            })
            .collect();
    }
    candidates.sort_by_key(|(i, _)| std::cmp::Reverse(size_kb(i)));
    if candidates.is_empty() {
        candidates = changes(SourceKind::Packaged)
            .iter()
            .filter_map(|c| match c {
                Change::Added(i) => Some((i, None)),
                _ => None,
            })
            .collect();
    }

    let primary = app_flag
        .and_then(|flag| {
            let flag = flag.to_lowercase();
            candidates.iter().find(|(i, _)| i.label.to_lowercase().contains(&flag))
        })
        .or(candidates.first());

    let name = app_flag
        .map(String::from)
        .or_else(|| primary.map(|(i, _)| i.label.clone()))
        .or_else(|| winget_id(command))
        .unwrap_or_else(|| copy::UNKNOWN_APP.to_string());
    let mut subject = Subject {
        name,
        version: primary.and_then(|(i, _)| prop(i, "version")),
        previous_version: primary.and_then(|(_, prev)| prev.clone()),
        publisher: primary
            .and_then(|(i, _)| prop(i, "publisher"))
            .map(|p| publisher::display(&p)),
        entries: candidates.iter().map(|(i, _)| i.key.clone()).collect(),
        own_entry: primary.map(|(i, _)| i.key.clone()),
        also_installed: candidates
            .iter()
            .filter(|c| Some(*c) != primary)
            .map(|(i, _)| i.label.clone())
            .collect(),
        ..Default::default()
    };

    let location = primary
        .and_then(|(i, _)| i.target.as_deref())
        .map(|t| t.trim_end_matches('\\').to_string())
        .filter(|t| !is_too_broad(t));
    subject.install_dirs.extend(location.clone());
    // A new top-level folder is the app's only if it holds the install location or is named
    // after the app. Windows makes folders during installs too (BranchCache's PeerDistRepub).
    let mut tokens = subject.folder_tokens();
    tokens.extend(winget_id_tokens(command));
    for change in changes(SourceKind::Folders) {
        let Change::Added(folder) = change else { continue };
        if folder.props.get("kind").map(String::as_str) == Some("shortcut") || folder.key.ends_with(".lnk") {
            continue;
        }
        let dir = folder.key.trim_end_matches('\\');
        let leaf: String = dir
            .rsplit('\\')
            .next()
            .unwrap_or(dir)
            .chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect();
        let named = is_named_after(&leaf, &tokens);
        let holds_install = location.as_deref().is_some_and(|l| is_under(l, dir));
        if (named || holds_install) && !is_too_broad(dir) && !subject.install_dirs.iter().any(|d| d == dir) {
            subject.install_dirs.push(dir.to_string());
        }
    }
    subject
}
