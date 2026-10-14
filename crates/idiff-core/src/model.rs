//! A snapshot: what every source found at one moment.

use crate::copy;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The snapshot format version. Anything else is refused rather than half-read.
pub const SCHEMA: u32 = 1;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SourceKind {
    Programs,
    Services,
    Drivers,
    Tasks,
    Startup,
    Firewall,
    ShellExt,
    Path,
    Folders,
    Packaged,
    Certs,
    Defender,
    Network,
}

impl SourceKind {
    pub const ALL: [SourceKind; 13] = [
        SourceKind::Programs,
        SourceKind::Services,
        SourceKind::Drivers,
        SourceKind::Tasks,
        SourceKind::Startup,
        SourceKind::Firewall,
        SourceKind::ShellExt,
        SourceKind::Path,
        SourceKind::Folders,
        SourceKind::Packaged,
        SourceKind::Certs,
        SourceKind::Defender,
        SourceKind::Network,
    ];

    /// Only complete when read by an administrator.
    pub fn needs_admin(self) -> bool {
        matches!(self, SourceKind::Tasks | SourceKind::Defender)
    }

    pub fn label(self) -> &'static str {
        match self {
            SourceKind::Programs => copy::SOURCE_PROGRAMS,
            SourceKind::Services => copy::SOURCE_SERVICES,
            SourceKind::Drivers => copy::SOURCE_DRIVERS,
            SourceKind::Tasks => copy::SOURCE_TASKS,
            SourceKind::Startup => copy::SOURCE_STARTUP,
            SourceKind::Firewall => copy::SOURCE_FIREWALL,
            SourceKind::ShellExt => copy::SOURCE_SHELL_EXT,
            SourceKind::Path => copy::SOURCE_PATH,
            SourceKind::Folders => copy::SOURCE_FOLDERS,
            SourceKind::Packaged => copy::SOURCE_PACKAGED,
            SourceKind::Certs => copy::SOURCE_CERTS,
            SourceKind::Defender => copy::SOURCE_DEFENDER,
            SourceKind::Network => copy::SOURCE_NETWORK,
        }
    }
}

/// One thing a source found. `key` is stable and lowercase; `target` is the normalized path of
/// what it runs or points at.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct Item {
    pub key: String,
    pub label: String,
    pub target: Option<String>,
    #[serde(default)]
    pub props: BTreeMap<String, String>,
}

/// What one source returned. `ok == false` means it couldn't be read (never "empty"); `note`
/// means it was read, but only in part.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct SourceSnap {
    pub ok: bool,
    pub error: Option<String>,
    pub note: Option<String>,
    pub ms: u64,
    pub items: Vec<Item>,
}

/// The folders attribution needs to know, by environment variable name.
pub type Env = BTreeMap<String, String>;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub schema: u32,
    pub idiff_version: String,
    pub taken_at_ms: u64,
    pub elevated: bool,
    pub env: Env,
    pub sources: BTreeMap<SourceKind, SourceSnap>,
}

impl Snapshot {
    pub fn load(json: &str) -> Result<Snapshot, String> {
        // The version first, so a newer format gets the right message instead of a parse error.
        #[derive(Deserialize)]
        struct Version {
            schema: u32,
        }
        let unreadable = |e: serde_json::Error| format!("{}: {e}", copy::SNAPSHOT_UNREADABLE);
        let version: Version = serde_json::from_str(json).map_err(unreadable)?;
        if version.schema != SCHEMA {
            return Err(copy::SNAPSHOT_TOO_NEW.to_string());
        }
        serde_json::from_str(json).map_err(unreadable)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("a snapshot always serializes")
    }
}
