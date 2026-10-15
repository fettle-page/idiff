//! `idiff leftovers`: after an uninstall, what the install left behind.

use crate::session::{Session, SessionMeta};
use idiff_core::attribute::{Attributed, Owner, attribute};
use idiff_core::copy;
use idiff_core::diff::{Change, diff};
use idiff_core::model::{Snapshot, SourceKind};
use idiff_core::noise::NoiseList;
use idiff_core::subject::find_subject;
use idiff_core::verdict::{Level, Line, Receipt, leftovers_receipt};
use std::collections::BTreeMap;

/// What a leftovers check found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Leftovers {
    pub receipt: Receipt,
    /// The app's own entry in Installed programs (or its Store package) is still there, so the
    /// uninstall didn't run or didn't finish. The receipt then opens with a warning saying so.
    pub still_listed: bool,
}

/// The leftovers receipt for a finished install session and a snapshot taken after the uninstall.
/// The install is attributed again with `noise` (today's rules, not the ones it was run with).
/// `size_of` measures a folder by its key; it's asked only about the app's folders still there.
pub fn leftovers_from_files(
    session: &Session,
    uninstalled: &Snapshot,
    noise: &NoiseList,
    size_of: &dyn Fn(&str) -> u64,
) -> Result<Leftovers, String> {
    if !session.dir.join("after.json").is_file() {
        return Err(copy::SESSION_UNFINISHED.to_string());
    }
    let before = load(session, "before.json")?;
    let after = load(session, "after.json")?;
    let meta: SessionMeta = session.read_json("session.json")?;

    let installed = diff(&before, &after);
    let subject = find_subject(&installed, meta.app_flag.as_deref(), &meta.command);
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
    let mut receipt = leftovers_receipt(&subject, &install, uninstalled, &during, &sizes);

    // A real uninstall always removes the app's own Installed programs entry (or Store package).
    let listed = |kind: SourceKind, key: &str| {
        uninstalled
            .sources
            .get(&kind)
            .is_some_and(|s| s.ok && s.items.iter().any(|i| i.key == key))
    };
    let still_listed = subject
        .entries
        .iter()
        .any(|key| listed(SourceKind::Programs, key) || listed(SourceKind::Packaged, key));
    if still_listed {
        receipt.lines.insert(
            0,
            Line {
                level: Level::Warn,
                text: copy::UNINSTALL_DIDNT_FINISH.to_string(),
                items: Vec::new(),
                why: Vec::new(),
            },
        );
    }
    Ok(Leftovers { receipt, still_listed })
}

/// A snapshot from the session, through `Snapshot::load` so a damaged or newer file says so.
fn load(session: &Session, name: &str) -> Result<Snapshot, String> {
    let path = session.dir.join(name);
    let text =
        std::fs::read_to_string(&path).map_err(|e| format!("{} {}: {e}", copy::CANT_READ_FILE, path.display()))?;
    Snapshot::load(&text).map_err(|e| format!("{}: {e}", path.display()))
}
