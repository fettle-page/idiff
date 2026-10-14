//! The commands that do more than print a file, one module each, so `main.rs` only picks one.

pub mod calibrate;
pub mod export;
pub mod leftovers;
pub mod list;

use crate::session::SessionMeta;
use idiff_core::copy;

/// Fills `{name}` parts of a copy template.
fn fill(template: &str, values: &[(&str, &str)]) -> String {
    values.iter().fold(template.to_string(), |text, (name, value)| {
        text.replace(&format!("{{{name}}}"), value)
    })
}

/// The app a session is about: what `run` found, else the `--app` name, else "Unknown app".
fn app_name(meta: &SessionMeta) -> String {
    meta.subject
        .clone()
        .or_else(|| meta.app_flag.clone())
        .unwrap_or_else(|| copy::UNKNOWN_APP.to_string())
}
