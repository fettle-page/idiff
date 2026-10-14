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

/// The built-in noise list, plus the user's own `noise.local.toml` if there is one.
fn noise() -> Result<NoiseList, String> {
    let path = session::home().join("noise.local.toml");
    match std::fs::read_to_string(&path) {
        Ok(text) => NoiseList::parse(&text)
            .map(|local| NoiseList::builtin().merged(local))
            .map_err(|e| format!("{}: {e}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(NoiseList::builtin()),
        Err(e) => Err(format!("{} {}: {e}", copy::CANT_READ_FILE, path.display())),
    }
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
    let attributed = attribute(&delta, &subject, &noise()?, &after.env);
    let mut receipt = install_receipt(&subject, &attributed, &delta.notes, &delta.partial, None, &after.env);
    receipt.lines.push(Line {
        level: Level::Note,
        text: copy::DIFF_NOT_ENRICHED.to_string(),
        items: Vec::new(),
        why: Vec::new(),
    });
    print!("{}", render::text(&receipt, &text_options(all, why)));
    Ok(())
}

fn cmd_show(target: &str, all: bool, why: bool, json: bool) -> Result<(), String> {
    let receipt: Receipt = session::open(target)?.read_json("receipt.json")?;
    if json {
        let text = serde_json::to_string_pretty(&receipt).map_err(|e| e.to_string())?;
        println!("{text}");
    } else {
        print!("{}", render::text(&receipt, &text_options(all, why)));
    }
    Ok(())
}
