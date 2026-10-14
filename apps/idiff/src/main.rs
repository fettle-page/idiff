use clap::Parser;
use idiff::cli::{Cli, Command};
use idiff_core::copy;
use idiff_core::diff::{Change, Delta, diff};
use idiff_core::model::Snapshot;
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let result = match Cli::parse().command {
        Command::Snapshot { output, .. } => cmd_snapshot(output.as_deref()),
        Command::Diff { before, after, .. } => cmd_diff(&before, &after),
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

/// To the file given with `-o`, else to stdout.
#[cfg(windows)]
fn cmd_snapshot(output: Option<&Path>) -> Result<(), String> {
    let json = idiff_collect::snapshot(&idiff_collect::SnapshotOptions::default()).to_json();
    match output {
        Some(path) => std::fs::write(path, json).map_err(|e| format!("{}: {e}", path.display())),
        None => {
            println!("{json}");
            Ok(())
        }
    }
}

#[cfg(not(windows))]
fn cmd_snapshot(_output: Option<&Path>) -> Result<(), String> {
    Err(copy::NOT_BUILT_YET.to_string())
}

fn cmd_diff(before: &Path, after: &Path) -> Result<(), String> {
    print!("{}", raw_delta(&diff(&load(before)?, &load(after)?)));
    Ok(())
}

/// One line per change (`+` added, `-` removed, `~` modified), then a `!` line per note.
fn raw_delta(delta: &Delta) -> String {
    let mut out = String::new();
    for (kind, changes) in &delta.sources {
        for change in changes {
            let (sign, fields) = match change {
                Change::Added(_) => ('+', String::new()),
                Change::Removed(_) => ('-', String::new()),
                Change::Modified { fields, .. } => ('~', format!(" ({})", fields.join(", "))),
            };
            let item = change.item();
            let name = if item.label.is_empty() || item.label == item.key {
                item.key.clone()
            } else {
                format!("{} [{}]", item.label, item.key)
            };
            out.push_str(&format!("{sign} {kind:?} {name}{fields}\n"));
        }
    }
    for (kind, note) in &delta.notes {
        out.push_str(&format!("! {kind:?}: {note}\n"));
    }
    if out.is_empty() {
        out = format!("{}\n", copy::NO_CHANGES);
    }
    out
}
