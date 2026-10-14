//! Who made each change: the app, Windows, or nobody can tell (spec rules R1–R7, first match wins).

use crate::copy;
use crate::diff::{Change, Delta};
use crate::model::{Env, Item, SourceKind};
use crate::noise::NoiseList;
use crate::publisher;
use crate::subject::Subject;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Owner {
    App,
    Windows,
    Unknown,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum Reason {
    /// R1, with the matching rule's note.
    Noise(String),
    /// The Programs or Store entry the install added or upgraded.
    SubjectEntry,
    /// R2
    TargetInInstallDir,
    /// R3
    SignerMatchesPublisher,
    /// R4
    ItemInInstallDir,
    /// R5
    WindowsComponent,
    /// R6: weak, so `--why` says so.
    NameMatches,
    /// R7
    Unattributed,
}

impl Reason {
    pub fn owner(&self) -> Owner {
        match self {
            Reason::Noise(_) | Reason::WindowsComponent => Owner::Windows,
            Reason::Unattributed => Owner::Unknown,
            _ => Owner::App,
        }
    }

    /// For `--why`.
    pub fn text(&self) -> String {
        match self {
            Reason::Noise(note) => format!("{}: {note}", copy::WHY_NOISE),
            Reason::SubjectEntry => copy::WHY_SUBJECT_ENTRY.into(),
            Reason::TargetInInstallDir => copy::WHY_TARGET_IN_INSTALL_DIR.into(),
            Reason::SignerMatchesPublisher => copy::WHY_SIGNER_MATCHES.into(),
            Reason::ItemInInstallDir => copy::WHY_ITEM_IN_INSTALL_DIR.into(),
            Reason::WindowsComponent => copy::WHY_WINDOWS_COMPONENT.into(),
            Reason::NameMatches => copy::WHY_NAME_MATCHES.into(),
            Reason::Unattributed => copy::WHY_UNATTRIBUTED.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attributed {
    pub kind: SourceKind,
    pub change: Change,
    pub owner: Owner,
    pub reason: Reason,
}

/// Every change in `delta`, in the same order, with its owner. Removed items go through the same
/// rules, so leftovers are judged like installs.
pub fn attribute(delta: &Delta, subject: &Subject, noise: &NoiseList, env: &Env) -> Vec<Attributed> {
    let rules = Rules {
        subject,
        noise,
        system_root: env
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("SystemRoot"))
            .map(|(_, v)| v.trim_end_matches('\\').to_lowercase()),
        subject_is_microsoft: subject.publisher.as_deref().is_some_and(publisher::is_microsoft),
        token: subject.name_token(),
    };
    delta
        .sources
        .iter()
        .flat_map(|(kind, changes)| {
            changes.iter().map(|change| {
                let reason = rules.reason(*kind, change);
                Attributed {
                    kind: *kind,
                    change: change.clone(),
                    owner: reason.owner(),
                    reason,
                }
            })
        })
        .collect()
}

struct Rules<'a> {
    subject: &'a Subject,
    noise: &'a NoiseList,
    system_root: Option<String>,
    subject_is_microsoft: bool,
    token: Option<String>,
}

impl Rules<'_> {
    fn reason(&self, kind: SourceKind, change: &Change) -> Reason {
        if let Some(note) = self.noise.matches_change(kind, change, self.subject) {
            return Reason::Noise(note.to_string());
        }
        let item = change.item();
        if matches!(kind, SourceKind::Programs | SourceKind::Packaged) && self.subject.entries.contains(&item.key) {
            return Reason::SubjectEntry;
        }
        let target = item.target.as_deref();
        let signer = item.props.get("signer").map(String::as_str);
        if target.is_some_and(|t| self.in_install_dir(t)) {
            return Reason::TargetInInstallDir;
        }
        if let (Some(signer), Some(publisher)) = (signer, self.subject.publisher.as_deref())
            && publisher::same(signer, publisher)
        {
            return Reason::SignerMatchesPublisher;
        }
        if item_path(kind, item).is_some_and(|p| self.in_install_dir(p)) {
            return Reason::ItemInInstallDir;
        }
        if !self.subject_is_microsoft && is_windows_component(kind, change, target, signer, self.system_root.as_deref())
        {
            return Reason::WindowsComponent;
        }
        if let Some(token) = &self.token
            && (has_word(&item.key, token) || has_word(&item.label.to_lowercase(), token))
        {
            return Reason::NameMatches;
        }
        Reason::Unattributed
    }

    fn in_install_dir(&self, path: &str) -> bool {
        self.subject.install_dirs.iter().any(|dir| is_under(path, dir))
    }
}

/// R5. A change to something that already existed is Windows' if it runs from the Windows folder
/// or is signed as Windows. Something new needs Windows' own signature: apps put drivers and
/// services in System32 too (Malwarebytes, Dropbox), and their uninstall shortcuts point at
/// msiexec, so a new folder or shortcut is never Windows' by this rule.
fn is_windows_component(
    kind: SourceKind,
    change: &Change,
    target: Option<&str>,
    signer: Option<&str>,
    system_root: Option<&str>,
) -> bool {
    let windows_signed = signer.is_some_and(publisher::is_windows_signer);
    match change {
        Change::Added(_) => kind != SourceKind::Folders && windows_signed,
        Change::Modified { .. } | Change::Removed(_) => {
            windows_signed || target.zip(system_root).is_some_and(|(t, root)| is_under(t, root))
        }
    }
}

/// Where the item itself is, for items that are places: folders, shortcuts and PATH entries.
fn item_path(kind: SourceKind, item: &Item) -> Option<&str> {
    match kind {
        SourceKind::Folders => Some(&item.key),
        SourceKind::Path => item.key.split_once('|').map(|(_, p)| p),
        _ => None,
    }
}

/// Paths in normalized form: lowercase, no trailing backslash.
fn is_under(path: &str, dir: &str) -> bool {
    path == dir || path.strip_prefix(dir).is_some_and(|rest| rest.starts_with('\\'))
}

/// `word` appears in `text` with no letter or digit right before or after it.
fn has_word(text: &str, word: &str) -> bool {
    text.match_indices(word).any(|(i, _)| {
        let before = text[..i].chars().next_back();
        let after = text[i + word.len()..].chars().next();
        !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
    })
}
