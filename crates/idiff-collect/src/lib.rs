//! Reads the parts of Windows that change how the PC behaves, into an `idiff_core` snapshot.
//!
//! Every source runs at once on its own thread (the runner) and only reads. Each source module
//! has pure functions that turn what Windows stores into `Item`s, plus a thin `read()`.

#[cfg(windows)]
mod elevation;
#[cfg(windows)]
pub mod reg;
pub mod runner;
pub mod sources;

pub use runner::{Raw, Source, SourceResult, run_all, run_all_with};

use idiff_core::model::{Env, Item};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use std::time::Duration;

/// Which sources a snapshot reads, and how long any one of them may take.
#[derive(Debug, Clone)]
pub struct SnapshotOptions {
    /// `None` reads every source.
    pub only: Option<Vec<idiff_core::model::SourceKind>>,
    pub timeout: Duration,
}

impl Default for SnapshotOptions {
    fn default() -> Self {
        SnapshotOptions {
            only: None,
            timeout: Duration::from_secs(20),
        }
    }
}

/// What every source's `read()` returns. `note` says the list is only partial.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct SourceOutput {
    pub items: Vec<Item>,
    pub note: Option<String>,
}

/// The environment variables paths are expanded with, as this process sees them.
const ENV_NAMES: &[&str] = &[
    "SystemRoot",
    "ProgramFiles",
    "ProgramFiles(x86)",
    "ProgramData",
    "LOCALAPPDATA",
    "APPDATA",
    "USERPROFILE",
];

/// This process's folders by environment variable name, read once.
pub fn env() -> &'static Env {
    static ENV: OnceLock<Env> = OnceLock::new();
    ENV.get_or_init(|| {
        ENV_NAMES
            .iter()
            .filter_map(|name| std::env::var(name).ok().map(|value| (name.to_string(), value)))
            .collect()
    })
}

/// Reads the chosen sources, all at once, into a snapshot. A source that fails or times out is
/// `ok: false` with its error, never an empty list.
#[cfg(windows)]
pub fn snapshot(opts: &SnapshotOptions) -> idiff_core::model::Snapshot {
    use idiff_core::model::{SCHEMA, Snapshot, SourceKind, SourceSnap};
    use std::time::{SystemTime, UNIX_EPOCH};

    let kinds: Vec<SourceKind> = opts.only.clone().unwrap_or_else(|| SourceKind::ALL.to_vec());
    let taken_at_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64);
    let jobs = kinds
        .iter()
        .map(|&kind| (format!("{kind:?}"), sources::source(kind)))
        .collect();
    let mut raw = run_all(jobs, opts.timeout);

    let sources = kinds
        .into_iter()
        .map(|kind| {
            let result = raw.remove(&format!("{kind:?}"));
            let snap = match result {
                Some(SourceResult {
                    ok: true,
                    ms,
                    data: Some(data),
                    ..
                }) => match serde_json::from_value::<SourceOutput>(data) {
                    Ok(out) => SourceSnap {
                        ok: true,
                        error: None,
                        note: out.note,
                        ms,
                        items: out.items,
                    },
                    Err(e) => SourceSnap {
                        ok: false,
                        error: Some(e.to_string()),
                        ms,
                        ..Default::default()
                    },
                },
                Some(SourceResult { ms, error, .. }) => {
                    let error = error.or_else(|| Some("no result".into()));
                    SourceSnap {
                        ok: false,
                        error,
                        ms,
                        ..Default::default()
                    }
                }
                None => SourceSnap {
                    ok: false,
                    error: Some("no result".into()),
                    ..Default::default()
                },
            };
            (kind, snap)
        })
        .collect();

    Snapshot {
        schema: SCHEMA,
        idiff_version: env!("CARGO_PKG_VERSION").to_string(),
        taken_at_ms,
        elevated: elevation::is_elevated(),
        env: env().clone(),
        sources,
    }
}
