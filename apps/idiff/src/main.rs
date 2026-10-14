use clap::Parser;
use idiff::cli::{Cli, Command, color_enabled};
use idiff::session;
use idiff_core::attribute::attribute;
use idiff_core::copy;
use idiff_core::diff::diff;
use idiff_core::model::Snapshot;
use idiff_core::noise::NoiseList;
use idiff_core::render::{self, TextOptions};
use idiff_core::subject::find_subject;
use idiff_core::verdict::{Level, Line, Receipt, install_receipt};
use std::io::IsTerminal;
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    // An elevated copy started as the helper takes this path, before the normal commands are parsed.
    #[cfg(windows)]
    {
        let args: Vec<String> = std::env::args().skip(1).collect();
        if args.first().is_some_and(|a| a == "--helper") {
            let code = match idiff::elevate::parse_helper_args(&args) {
                Some((path, only)) => idiff::elevate::run_helper(&path, &only),
                None => 1,
            };
            return ExitCode::from(code as u8);
        }
    }
    let result = match Cli::parse().command {
        Command::Snapshot { output, no_elevate } => cmd_snapshot(output.as_deref(), no_elevate),
        Command::Diff {
            before,
            after,
            app,
            all,
            why,
        } => cmd_diff(&before, &after, app.as_deref(), all, why),
        Command::Show {
            session,
            all,
            why,
            json,
        } => cmd_show(&session, all, why, json),
        Command::Leftovers {
            session,
            yes,
            settle,
            no_elevate,
            command,
        } => {
            let text = text_options(false, false);
            match idiff::cmd::leftovers::run(&session, yes, settle, no_elevate, &command, &noise(), &text) {
                Ok(0) => Ok(()),
                // The uninstall command failed; the receipt was still printed.
                Ok(code) => return ExitCode::from(code),
                Err(e) => Err(e),
            }
        }
        Command::Export {
            session,
            format,
            output,
            no_redact,
        } => idiff::cmd::export::run(&session, format, output.as_deref(), no_redact),
        Command::Calibrate {
            minutes,
            save,
            no_elevate,
        } => idiff::cmd::calibrate::run(minutes, save, no_elevate, &noise(), &text_options(true, true)),
        Command::List => idiff::cmd::list::run(),
        _ => Err(copy::NOT_BUILT_YET.to_string()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("idiff: {e}");
            ExitCode::from(1)
        }
    }
}

fn load(path: &Path) -> Result<Snapshot, String> {
    let json =
        std::fs::read_to_string(path).map_err(|e| format!("{} {}: {e}", copy::CANT_READ_FILE, path.display()))?;
    Snapshot::load(&json).map_err(|e| format!("{}: {e}", path.display()))
}

fn text_options(all: bool, why: bool) -> TextOptions {
    let terminal = std::io::stdout().is_terminal() && console_shows_colour();
    TextOptions {
        color: color_enabled(terminal, std::env::var_os("NO_COLOR").as_deref()),
        all,
        why,
    }
}

/// Whether the console already turns ANSI codes into colour (Windows Terminal, VS Code and
/// PowerShell switch it on). idiff only reads the console's mode, never changes it.
#[cfg(windows)]
fn console_shows_colour() -> bool {
    use windows::Win32::System::Console::{
        CONSOLE_MODE, ENABLE_VIRTUAL_TERMINAL_PROCESSING, GetConsoleMode, GetStdHandle, STD_OUTPUT_HANDLE,
    };
    unsafe {
        let Ok(out) = GetStdHandle(STD_OUTPUT_HANDLE) else {
            return false;
        };
        let mut mode = CONSOLE_MODE(0);
        GetConsoleMode(out, &mut mode).is_ok() && mode.0 & ENABLE_VIRTUAL_TERMINAL_PROCESSING.0 != 0
    }
}

#[cfg(not(windows))]
fn console_shows_colour() -> bool {
    true
}

/// The built-in noise list, plus the user's own `noise.local.toml` if there is one. A local
/// file with a mistake is left out, with a warning on stderr.
fn noise() -> NoiseList {
    let (noise, warning) = idiff::noise::load(&session::home());
    if let Some(warning) = warning {
        eprintln!("idiff: {warning}");
    }
    noise
}

/// To the file given with `-o`, else to stdout. Asks through UAC for the administrator-only
/// sources unless `--no-elevate`.
#[cfg(windows)]
fn cmd_snapshot(output: Option<&Path>, no_elevate: bool) -> Result<(), String> {
    let json = idiff::elevate::snapshot_for_run(no_elevate).to_json();
    match output {
        Some(path) => std::fs::write(path, json).map_err(|e| format!("{}: {e}", path.display())),
        None => {
            println!("{json}");
            Ok(())
        }
    }
}

#[cfg(not(windows))]
fn cmd_snapshot(_output: Option<&Path>, _no_elevate: bool) -> Result<(), String> {
    Err(copy::NOT_BUILT_YET.to_string())
}

/// A receipt from two snapshot files. Signers and folder sizes come from the PC the snapshots
/// describe, which may not be this one, so they aren't looked up; the receipt says so.
fn cmd_diff(before: &Path, after: &Path, app: Option<&str>, all: bool, why: bool) -> Result<(), String> {
    let (before, after) = (load(before)?, load(after)?);
    let delta = diff(&before, &after);
    let subject = find_subject(&delta, app, &[]);
    let attributed = attribute(&delta, &subject, &noise(), &after.env);
    let mut receipt = install_receipt(&subject, &attributed, &delta.notes, None, &after.env);
    receipt.lines.push(Line {
        level: Level::Note,
        text: copy::DIFF_NOT_ENRICHED.to_string(),
        items: Vec::new(),
        why: Vec::new(),
    });
    print!("{}", render::text(&receipt, &text_options(all, why)));
    Ok(())
}

/// The install receipt, then the leftovers one if `leftovers` was run. `--json` prints the
/// install receipt only (`export --format json` has both).
fn cmd_show(target: &str, all: bool, why: bool, json: bool) -> Result<(), String> {
    let s = session::open(target)?;
    let receipt: Receipt = s.read_json("receipt.json")?;
    if json {
        let text = serde_json::to_string_pretty(&receipt).map_err(|e| e.to_string())?;
        println!("{text}");
    } else {
        print!("{}", render::text(&receipt, &text_options(all, why)));
        if s.dir.join("leftovers.json").is_file() {
            let leftovers: Receipt = s.read_json("leftovers.json")?;
            print!("\n{}", render::text(&leftovers, &text_options(all, why)));
        }
    }
    Ok(())
}
