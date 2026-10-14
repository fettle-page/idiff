//! `idiff leftovers <session|last> [--yes] [--settle SECS] [--no-elevate] [-- <uninstall command>]`

use idiff_core::noise::NoiseList;
use idiff_core::render::TextOptions;

/// Runs the uninstall command (or waits for the user to uninstall by hand), snapshots the PC,
/// saves `uninstalled.json` and `leftovers.json` in the session and prints the receipt.
/// Returns idiff's exit code: `2` when the uninstall command failed.
#[cfg(windows)]
pub fn run(
    target: &str,
    yes: bool,
    settle: u64,
    no_elevate: bool,
    command: &[String],
    noise: &NoiseList,
    text: &TextOptions,
) -> Result<u8, String> {
    use super::{app_name, fill};
    use crate::elevate::snapshot_for_run;
    use crate::leftovers::leftovers_from_files;
    use crate::run::{exit_status, run_in_job, wait_for_msi_idle};
    use crate::session::{self, SessionMeta};
    use idiff_collect::sources::folders::folder_size;
    use idiff_core::copy;
    use idiff_core::render;
    use std::path::Path;
    use std::time::Duration;

    let s = session::open(target)?;
    // Checked before anything runs: without the install's after snapshot there's nothing to compare.
    if !s.dir.join("after.json").is_file() {
        return Err(copy::SESSION_UNFINISHED.to_string());
    }
    let meta: SessionMeta = s.read_json("session.json")?;

    let mut status = 0;
    if !command.is_empty() {
        let outcome = run_in_job(command, Duration::from_secs(30 * 60)).map_err(|e| e.to_string())?;
        wait_for_msi_idle(Duration::from_secs(10 * 60));
        std::thread::sleep(Duration::from_secs(settle));
        status = exit_status(&outcome);
    } else if !yes {
        eprintln!("{}", fill(copy::UNINSTALL_NOW, &[("app", &app_name(&meta))]));
        let mut answer = String::new();
        std::io::stdin().read_line(&mut answer).map_err(|e| e.to_string())?;
    }

    let uninstalled = snapshot_for_run(no_elevate);
    let saved = |e: std::io::Error| format!("{}: {e}", s.dir.display());
    s.write_json("uninstalled.json", &uninstalled).map_err(saved)?;
    let receipt = leftovers_from_files(&s, &uninstalled, noise, &|key| folder_size(Path::new(key)).0)?;
    s.write_json("leftovers.json", &receipt).map_err(saved)?;
    print!("{}", render::text(&receipt, text));
    Ok(status)
}

#[cfg(not(windows))]
pub fn run(
    _target: &str,
    _yes: bool,
    _settle: u64,
    _no_elevate: bool,
    _command: &[String],
    _noise: &NoiseList,
    _text: &TextOptions,
) -> Result<u8, String> {
    Err(idiff_core::copy::NOT_BUILT_YET.to_string())
}
