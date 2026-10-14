//! PATH entries, the user's and the machine's, one item each.

use idiff_core::command::normalize_path;
use idiff_core::model::Item;
use std::collections::BTreeSet;

/// One item per entry of a raw (unexpanded) PATH value. Empty entries are dropped and entries
/// that normalize to the same folder are kept once, first one first.
pub fn path_items(scope: &str, raw: &str) -> Vec<Item> {
    let mut seen = BTreeSet::new();
    raw.split(';')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .filter_map(|entry| {
            let normalized = normalize_path(entry, crate::env());
            seen.insert(normalized.clone()).then(|| Item {
                key: format!("{scope}|{normalized}"),
                label: entry.to_string(),
                target: Some(normalized),
                props: Default::default(),
            })
        })
        .collect()
}

#[cfg(windows)]
pub fn read() -> Result<crate::SourceOutput, String> {
    use crate::reg::open_error;
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

    let scopes = [
        (HKEY_CURRENT_USER, "user", "Environment"),
        (
            HKEY_LOCAL_MACHINE,
            "machine",
            r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment",
        ),
    ];
    let mut items = Vec::new();
    let mut errors = Vec::new();
    for (hive, scope, path) in scopes {
        // The raw value: winreg doesn't expand REG_EXPAND_SZ, so normalize_path does it with `env`.
        match RegKey::predef(hive)
            .open_subkey(path)
            .and_then(|key| key.get_value::<String, _>("Path"))
        {
            Ok(raw) => items.extend(path_items(scope, &raw)),
            Err(e) => errors.extend(open_error(&e).map(|error| format!("{scope} PATH: {error}"))),
        }
    }
    if items.is_empty() && !errors.is_empty() {
        return Err(errors.join("; "));
    }
    let note = (!errors.is_empty()).then(|| format!("Couldn't read {}", errors.join("; ")));
    Ok(crate::SourceOutput { items, note })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_entries_are_split_trimmed_and_deduped() {
        let items = path_items("user", r"C:\A;;c:\a ; %SystemRoot%\x");
        assert_eq!(items.len(), 2, "{items:?}");
        assert_eq!(items[0].key, r"user|c:\a");
        assert_eq!(items[0].label, r"C:\A");
        assert_eq!(items[0].target.as_deref(), Some(r"c:\a"));
        assert_eq!(items[1].label, r"%SystemRoot%\x");
        assert!(items[1].key.starts_with("user|") && items[1].key.ends_with(r"\x"));
        assert!(path_items("machine", " ; ").is_empty());
    }
}
