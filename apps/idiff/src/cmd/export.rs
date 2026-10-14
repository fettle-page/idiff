//! `idiff export <session|last> --format md|html|json [-o FILE] [--no-redact]`

use super::fill;
use crate::cli::Format;
use crate::session;
use idiff_core::copy;
use idiff_core::redact::{Redactor, redact_receipt};
use idiff_core::render;
use idiff_core::verdict::Receipt;
use serde::Serialize;
use std::path::{Path, PathBuf};

/// What `--format json` writes: the install receipt and, after `leftovers`, that receipt too.
#[derive(Serialize)]
struct Exported<'a> {
    install: &'a Receipt,
    leftovers: Option<&'a Receipt>,
}

/// Writes the session's receipts (install, then leftovers if checked) to one file, with this
/// PC's names taken out unless `no_redact`. The file is `idiff-{id}.{ext}` unless `-o` names one.
pub fn run(target: &str, format: Format, output: Option<&Path>, no_redact: bool) -> Result<(), String> {
    let s = session::open(target)?;
    if !s.dir.join("receipt.json").is_file() {
        return Err(copy::SESSION_UNFINISHED.to_string());
    }
    let mut install: Receipt = s.read_json("receipt.json")?;
    let mut leftovers: Option<Receipt> = match s.dir.join("leftovers.json").is_file() {
        true => Some(s.read_json("leftovers.json")?),
        false => None,
    };
    if no_redact {
        eprintln!("{}", copy::NOT_REDACTED);
    } else {
        let red = redactor();
        redact_receipt(&mut install, &red);
        if let Some(l) = &mut leftovers {
            redact_receipt(l, &red);
        }
    }

    let receipts: Vec<&Receipt> = std::iter::once(&install).chain(leftovers.as_ref()).collect();
    let (text, ext) = match format {
        Format::Md => (render::markdown_receipts(&receipts, true), "md"),
        Format::Html => (render::html_receipts(&receipts), "html"),
        Format::Json => {
            let both = Exported {
                install: &install,
                leftovers: leftovers.as_ref(),
            };
            (serde_json::to_string_pretty(&both).map_err(|e| e.to_string())?, "json")
        }
    };
    let path = output.map_or_else(|| PathBuf::from(format!("idiff-{}.{ext}", s.id)), Path::to_path_buf);
    std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    eprintln!("{}", fill(copy::EXPORTED, &[("path", &path.display().to_string())]));
    Ok(())
}

/// Every name this PC goes by.
#[cfg(windows)]
fn redactor() -> Redactor {
    idiff_collect::names::redactor()
}

/// Off Windows there are no names to gather; profile folders, SIDs and addresses still go.
#[cfg(not(windows))]
fn redactor() -> Redactor {
    Redactor::default()
}
