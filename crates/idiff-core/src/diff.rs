//! What changed between two snapshots, source by source.

use crate::copy;
use crate::model::{Item, Snapshot, SourceKind, SourceSnap};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    Added(Item),
    Removed(Item),
    /// `fields` names what differs: `"label"`, `"target"` and prop names, sorted.
    Modified {
        before: Item,
        after: Item,
        fields: Vec<String>,
    },
}

impl Change {
    /// The item as it is now, or as it was last seen if it was removed.
    pub fn item(&self) -> &Item {
        match self {
            Change::Added(i) | Change::Removed(i) => i,
            Change::Modified { after, .. } => after,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Delta {
    /// Only sources with at least one change, each sorted by key.
    pub sources: BTreeMap<SourceKind, Vec<Change>>,
    /// Why a source couldn't be compared, or was only partly read.
    pub notes: BTreeMap<SourceKind, String>,
}

pub fn diff(before: &Snapshot, after: &Snapshot) -> Delta {
    let mut delta = Delta::default();
    for kind in SourceKind::ALL {
        let (b, a) = match (before.sources.get(&kind), after.sources.get(&kind)) {
            (None, None) => continue,
            (Some(b), Some(a)) => (b, a),
            // Diffing against nothing would report every item as added or removed.
            _ => {
                delta.notes.insert(kind, copy::SOURCE_NOT_READ.to_string());
                continue;
            }
        };
        if !a.ok || !b.ok {
            // A source that couldn't be read is never treated as empty, for the same reason.
            let error = [a, b].into_iter().filter(|s| !s.ok).find_map(|s| s.error.clone());
            delta
                .notes
                .insert(kind, error.unwrap_or_else(|| copy::SOURCE_NOT_READ.to_string()));
            continue;
        }
        if let Some(note) = a.note.as_ref().or(b.note.as_ref()) {
            delta.notes.insert(kind, note.clone());
        }
        let changes = diff_items(b, a);
        if !changes.is_empty() {
            delta.sources.insert(kind, changes);
        }
    }
    delta
}

fn by_key(source: &SourceSnap) -> BTreeMap<&str, &Item> {
    let mut map = BTreeMap::new();
    for item in &source.items {
        map.entry(item.key.as_str()).or_insert(item);
    }
    map
}

fn diff_items(before: &SourceSnap, after: &SourceSnap) -> Vec<Change> {
    let (b, a) = (by_key(before), by_key(after));
    let mut keys: Vec<&str> = b.keys().chain(a.keys()).copied().collect();
    keys.sort_unstable();
    keys.dedup();
    keys.into_iter()
        .filter_map(|key| match (b.get(key), a.get(key)) {
            (None, Some(a)) => Some(Change::Added((*a).clone())),
            (Some(b), None) => Some(Change::Removed((*b).clone())),
            (Some(b), Some(a)) => {
                let fields = changed_fields(b, a);
                (!fields.is_empty()).then(|| Change::Modified {
                    before: (*b).clone(),
                    after: (*a).clone(),
                    fields,
                })
            }
            (None, None) => None,
        })
        .collect()
}

fn changed_fields(before: &Item, after: &Item) -> Vec<String> {
    let mut fields: Vec<String> = before
        .props
        .keys()
        .chain(after.props.keys())
        .filter(|k| before.props.get(*k) != after.props.get(*k))
        .cloned()
        .collect();
    if before.label != after.label {
        fields.push("label".into());
    }
    if before.target != after.target {
        fields.push("target".into());
    }
    fields.sort_unstable();
    fields.dedup();
    fields
}
