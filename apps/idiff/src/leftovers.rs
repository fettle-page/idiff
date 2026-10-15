//! `idiff leftovers`: after an uninstall, what the install left behind.

use crate::session::{Session, SessionMeta};
use idiff_core::attribute::{Attributed, Owner, attribute};
use idiff_core::copy;
use idiff_core::diff::{Change, diff};
use idiff_core::model::{Snapshot, SourceKind};
use idiff_core::noise::NoiseList;
use idiff_core::subject::find_subject;
use idiff_core::verdict::{Receipt, leftovers_receipt};
use std::collections::BTreeMap;

/// The leftovers receipt for a finished install session and a snapshot taken after the uninstall.
/// The install is attributed again with `noise` (today's rules, not the ones it was run with).
/// `size_of` measures a folder by its key; it's asked only about the app's folders still there.
pub fn leftovers_from_files(
    session: &Session,
    uninstalled: &Snapshot,
    noise: &NoiseList,
    size_of: &dyn Fn(&str) -> u64,
) -> Result<Receipt, String> {
    if !session.dir.join("after.json").is_file() {
        return Err(copy::SESSION_UNFINISHED.to_string());
    }
    let before = load(session, "before.json")?;
    let after = load(session, "after.json")?;
    let meta: SessionMeta = session.read_json("session.json")?;

    let installed = diff(&before, &after);
    let subject = find_subject(&installed, meta.app_flag.as_deref(), &meta.command, noise);
    let install = attribute(&installed, &subject, noise, &after.env);
    let during: Vec<Attributed> = attribute(&diff(&after, uninstalled), &subject, noise, &uninstalled.env)
        .into_iter()
        .filter(|a| a.owner == Owner::App && matches!(a.change, Change::Added(_)))
        .collect();

    let still_there = |key: &str| {
        uninstalled
            .sources
            .get(&SourceKind::Folders)
            .is_some_and(|s| s.ok && s.items.iter().any(|i| i.key == key))
    };
    let sizes: BTreeMap<String, u64> = install
        .iter()
        .filter(|a| a.owner == Owner::App && a.kind == SourceKind::Folders)
        .filter_map(|a| match &a.change {
            Change::Added(item) if item.props.get("kind").map(String::as_str) == Some("folder") => Some(&item.key),
            _ => None,
        })
        .filter(|key| still_there(key))
        .map(|key| (key.clone(), size_of(key)))
        .collect();
    Ok(leftovers_receipt(&subject, &install, uninstalled, &during, &sizes))
}

/// A snapshot from the session, through `Snapshot::load` so a damaged or newer file says so.
fn load(session: &Session, name: &str) -> Result<Snapshot, String> {
    let path = session.dir.join(name);
    let text =
        std::fs::read_to_string(&path).map_err(|e| format!("{} {}: {e}", copy::CANT_READ_FILE, path.display()))?;
    Snapshot::load(&text).map_err(|e| format!("{}: {e}", path.display()))
}
