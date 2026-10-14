//! `idiff list`: one row per session, newest first.

use super::app_name;
use crate::session;
use idiff_core::copy;
use idiff_core::verdict::{Level, Receipt};

pub fn run() -> Result<(), String> {
    print!("{}", text());
    Ok(())
}

/// `id  app  ⚠n ✗n  finished`, in aligned columns. A session without a receipt has no counts.
fn text() -> String {
    let rows: Vec<[String; 4]> = session::list()
        .into_iter()
        .rev()
        .map(|meta| {
            let receipt: Option<Receipt> = session::open(&meta.id)
                .ok()
                .and_then(|s| s.read_json("receipt.json").ok());
            let count = |r: &Receipt, level| r.lines.iter().filter(|l| l.level == level).count();
            let counts = receipt.map_or_else(String::new, |r| {
                format!("⚠{} ✗{}", count(&r, Level::Warn), count(&r, Level::Bad))
            });
            let state = if meta.finished_ms.is_some() {
                copy::LIST_FINISHED
            } else {
                copy::LIST_UNFINISHED
            };
            [meta.id.clone(), app_name(&meta), counts, state.to_string()]
        })
        .collect();
    if rows.is_empty() {
        return format!("{}\n", copy::NO_SESSIONS);
    }
    let width = |col: usize| rows.iter().map(|r| r[col].chars().count()).max().unwrap_or(0);
    let (w0, w1, w2) = (width(0), width(1), width(2));
    rows.iter()
        .map(|[id, app, counts, state]| format!("{id:<w0$}  {app:<w1$}  {counts:<w2$}  {state}\n"))
        .collect()
}
