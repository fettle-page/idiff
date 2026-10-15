//! `idiff leftovers <session|last> [--yes] [--settle SECS] [--timeout MINS] [--no-elevate] [-- <uninstall command>]`

use idiff_core::noise::NoiseList;
use idiff_core::render::TextOptions;

/// How `leftovers` runs, from its command line.
#[derive(Debug, Clone, Copy)]
pub struct Options {
    /// Don't ask to uninstall by hand; take the snapshot now.
    pub yes: bool,
    /// Seconds to wait after the uninstall for what it left running.
    pub settle: u64,
    /// How long the uninstall command and its installers may take, in minutes.
    pub timeout_mins: u64,
    pub no_elevate: bool,
}

/// Runs the uninstall command (or waits for the user to uninstall by hand), snapshots the PC,
/// saves `uninstalled.json` and `leftovers.json` in the session and prints the receipt.
/// Returns idiff's exit code: `2` when the uninstall command failed or was still running when
/// time ran out (the receipt then says its list may include things the uninstall would have removed).
#[cfg(windows)]
pub fn run(
    target: &str,
    options: &Options,
    command: &[String],
    noise: &NoiseList,
    text: &TextOptions,
) -> Result<u8, String> {
    use super::{app_name, fill};
    use crate::elevate::snapshot_for_run;
    use crate::leftovers::leftovers_from_files;
    use crate::run::{INSTALLERS_QUIET, exit_status, run_in_job, wait_for_installers, wait_for_msi_idle};
    use crate::session::{self, SessionMeta};
    use idiff_collect::sources::folders::folder_size;
    use idiff_core::copy;
    use idiff_core::render;
    use idiff_core::verdict::{Level, Line};
    use std::path::Path;
    use std::time::{Duration, Instant, SystemTime};

    let s = session::open(target)?;
    // Checked before anything runs: without the install's after snapshot there's nothing to compare.
    if !s.dir.join("after.json").is_file() {
        return Err(copy::SESSION_UNFINISHED.to_string());
    }
    let meta: SessionMeta = s.read_json("session.json")?;

    let mut status = 0;
    let mut timed_out = false;
    if !command.is_empty() {
        let timeout = Duration::from_secs(options.timeout_mins * 60);
        let deadline = Instant::now() + timeout;
        let left = || deadline.saturating_duration_since(Instant::now());
        let started = SystemTime::now();
        let outcome = run_in_job(command, timeout).map_err(|e| e.to_string())?;
        eprintln!("{}", copy::WAITING_FOR_INSTALLER);
        // Uninstallers often copy themselves to %TEMP% and restart from there, outside the job,
        // so the command (winget included) reports success while they're still at work.
        let quiet = wait_for_installers(started, INSTALLERS_QUIET, left());
        wait_for_msi_idle(left().min(Duration::from_secs(10 * 60)));
        timed_out = outcome.timed_out || !quiet;
        if !timed_out {
            std::thread::sleep(Duration::from_secs(options.settle));
        }
        status = if timed_out { 2 } else { exit_status(&outcome) };
    } else if !options.yes {
        eprintln!("{}", fill(copy::UNINSTALL_NOW, &[("app", &app_name(&meta))]));
        let mut answer = String::new();
        std::io::stdin().read_line(&mut answer).map_err(|e| e.to_string())?;
    }

    let uninstalled = snapshot_for_run(options.no_elevate);
    let saved = |e: std::io::Error| format!("{}: {e}", s.dir.display());
    s.write_json("uninstalled.json", &uninstalled).map_err(saved)?;
    let found = leftovers_from_files(&s, &uninstalled, noise, &|key| folder_size(Path::new(key)).0)?;
    let mut receipt = found.receipt;
    // Still listed: the uninstall didn't run or finish, so these aren't all leftovers.
    if found.still_listed {
        status = 2;
    }
    if timed_out {
        // An uninstaller that hangs (often at a dialog nobody can click) hasn't removed things yet.
        // That says more than "still listed", so it's the one warning.
        if found.still_listed {
            receipt.lines.remove(0);
        }
        let n = options.timeout_mins.to_string();
        receipt.lines.insert(
            0,
            Line {
                level: Level::Warn,
                text: fill(copy::UNINSTALL_TIMED_OUT, &[("n", &n)]),
                items: Vec::new(),
                why: Vec::new(),
            },
        );
    }
    s.write_json("leftovers.json", &receipt).map_err(saved)?;
    print!("{}", render::text(&receipt, text));
    Ok(status)
}

#[cfg(not(windows))]
pub fn run(
    _target: &str,
    _options: &Options,
    _command: &[String],
    _noise: &NoiseList,
    _text: &TextOptions,
) -> Result<u8, String> {
    Err(idiff_core::copy::NOT_BUILT_YET.to_string())
}
