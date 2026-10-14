//! `idiff calibrate --minutes N [--save] [--no-elevate]`: what Windows changes by itself while
//! nothing is being installed, so it can be told apart from what an installer does.

use super::fill;
use crate::noise::LOCAL_FILE;
use idiff_core::attribute::{Owner, attribute};
use idiff_core::copy;
use idiff_core::diff::diff;
use idiff_core::model::{Snapshot, SourceKind};
use idiff_core::noise::{NoiseList, exact_rules};
use idiff_core::render::TextOptions;
use idiff_core::subject::Subject;
use idiff_core::verdict::{Level, Line, Mode, Receipt, install_receipt};
use std::io::Write;
use std::path::{Path, PathBuf};

/// The changes between two snapshots taken with nothing installed, as a receipt with only the
/// unknown and hidden sections (plus a note for each source that couldn't be compared), and the
/// keys of the unknown changes, which `--save` turns into rules.
pub fn calibration_receipt(
    before: &Snapshot,
    after: &Snapshot,
    noise: &NoiseList,
    minutes: u64,
) -> (Receipt, Vec<(SourceKind, String)>) {
    let delta = diff(before, after);
    // No app: nothing can be the app's, so every change is Windows' or unknown.
    let subject = Subject {
        name: copy::UNKNOWN_APP.to_string(),
        ..Default::default()
    };
    let attributed = attribute(&delta, &subject, noise, &after.env);
    let unknown = attributed
        .iter()
        .filter(|a| a.owner == Owner::Unknown)
        .map(|a| (a.kind, a.change.item().key.clone()))
        .collect();
    let full = install_receipt(&subject, &attributed, &delta.notes, &delta.partial, None, &after.env);
    let couldnt_check = delta.notes.iter().map(|(kind, reason)| Line {
        level: Level::Note,
        text: fill(
            if delta.partial.contains(kind) {
                copy::ONLY_PARTLY_CHECKED
            } else {
                copy::COULDNT_CHECK
            },
            &[("source", kind.label()), ("reason", reason)],
        ),
        items: Vec::new(),
        why: Vec::new(),
    });
    let receipt = Receipt {
        mode: Mode::Install,
        title: fill(copy::CALIBRATE_TITLE, &[("n", &minutes.to_string())]),
        lines: couldnt_check.collect(),
        unknown: full.unknown,
        hidden: full.hidden,
    };
    (receipt, unknown)
}

/// Appends rules matching exactly `keys` to `{home}\noise.local.toml` and returns its path.
/// With no keys nothing is written.
pub fn save_rules(home: &Path, keys: &[(SourceKind, String)]) -> Result<PathBuf, String> {
    let path = home.join(LOCAL_FILE);
    if keys.is_empty() {
        return Ok(path);
    }
    let failed = |e: std::io::Error| format!("{}: {e}", path.display());
    std::fs::create_dir_all(home).map_err(failed)?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(failed)?;
    // Rules from an earlier run stay; a blank line keeps the two runs apart.
    let gap = if file.metadata().map_or(0, |m| m.len()) > 0 {
        "\n"
    } else {
        ""
    };
    file.write_all(format!("{gap}{}", exact_rules(keys)).as_bytes())
        .map_err(failed)?;
    Ok(path)
}

/// Two snapshots `minutes` apart with a countdown on stderr, then the receipt; `--save` adds the
/// unknown changes to the local noise list.
#[cfg(windows)]
pub fn run(minutes: u64, save: bool, no_elevate: bool, noise: &NoiseList, text: &TextOptions) -> Result<(), String> {
    use crate::elevate::snapshot_for_run;
    use crate::session;
    use idiff_core::render;
    use std::time::{Duration, Instant};

    eprintln!("{}", fill(copy::CALIBRATE_WATCHING, &[("n", &minutes.to_string())]));
    let before = snapshot_for_run(no_elevate);
    let end = Instant::now() + Duration::from_secs(minutes * 60);
    while let Some(left) = end.checked_duration_since(Instant::now()).filter(|d| !d.is_zero()) {
        let secs = left.as_secs_f64().ceil() as u64;
        let time = format!("{}:{:02}", secs / 60, secs % 60);
        eprint!("\r{}  ", fill(copy::CALIBRATE_LEFT, &[("time", &time)]));
        std::thread::sleep(left.min(Duration::from_secs(1)));
    }
    if minutes > 0 {
        eprintln!();
    }
    let after = snapshot_for_run(no_elevate);

    let (receipt, unknown) = calibration_receipt(&before, &after, noise, minutes);
    print!("{}", render::text(&receipt, text));
    if save {
        if unknown.is_empty() {
            eprintln!("{}", copy::CALIBRATE_NOTHING_TO_SAVE);
        } else {
            let path = save_rules(&session::home(), &unknown)?;
            let n = unknown.len().to_string();
            eprintln!(
                "{}",
                fill(
                    copy::CALIBRATE_SAVED,
                    &[("n", &n), ("path", &path.display().to_string())]
                )
            );
        }
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn run(
    _minutes: u64,
    _save: bool,
    _no_elevate: bool,
    _noise: &NoiseList,
    _text: &TextOptions,
) -> Result<(), String> {
    Err(copy::NOT_BUILT_YET.to_string())
}
