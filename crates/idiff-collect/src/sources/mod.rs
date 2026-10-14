//! One module per source. Each has pure functions from what Windows stores to `Item`s, and a
//! `pub fn read() -> Result<SourceOutput, String>` that does the reading.

pub mod programs;
pub mod services;
pub mod tasks;

#[cfg(windows)]
use crate::SourceOutput;
#[cfg(windows)]
use idiff_core::model::SourceKind;

/// The runner job for one source: its `read()`, with the output turned into JSON.
#[cfg(windows)]
pub fn source(kind: SourceKind) -> crate::runner::Source {
    let read: fn() -> Result<SourceOutput, String> = match kind {
        SourceKind::Programs => programs::read,
        SourceKind::Services => services::read_services,
        SourceKind::Drivers => services::read_drivers,
        SourceKind::Tasks => tasks::read,
        _ => || Err("not built yet".into()),
    };
    Box::new(move || read().and_then(|out| serde_json::to_value(out).map_err(|e| e.to_string())))
}
