//! Microsoft Defender exclusions: paths, extensions, processes and IP addresses it doesn't scan.

use idiff_core::command::normalize_path;
use idiff_core::model::Item;
use std::collections::BTreeMap;

/// The exclusion lists, each a registry key whose value names are the excluded things.
pub const KINDS: &[&str] = &["Paths", "Extensions", "Processes", "IpAddresses"];

/// One exclusion. Paths and processes get a normalized target; `policy` marks one set by Group
/// Policy rather than in Windows Security.
pub fn exclusion_item(kind: &str, value: &str, policy: bool) -> Item {
    let is_path = kind == "Paths" || kind == "Processes";
    let mut props = BTreeMap::new();
    if policy {
        props.insert("policy".to_string(), "1".to_string());
    }
    Item {
        key: format!("{kind}|{value}").to_lowercase(),
        label: value.to_string(),
        target: is_path
            .then(|| normalize_path(value, crate::env()))
            .filter(|t| !t.is_empty()),
        props,
    }
}

/// Every exclusion, from Windows Security's own lists and the Group Policy ones. Only
/// administrators can read them; anyone else gets `copy::NEEDS_ADMIN`, never an empty list.
#[cfg(windows)]
pub fn read() -> Result<crate::SourceOutput, String> {
    use idiff_core::copy;
    use std::io::ErrorKind;
    use winreg::RegKey;
    use winreg::enums::HKEY_LOCAL_MACHINE;

    let bases = [
        (false, r"SOFTWARE\Microsoft\Windows Defender\Exclusions"),
        (true, r"SOFTWARE\Policies\Microsoft\Windows Defender\Exclusions"),
    ];
    let mut items = Vec::new();
    for (policy, base) in bases {
        for kind in KINDS {
            let key = match RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(format!(r"{base}\{kind}")) {
                Ok(key) => key,
                Err(e) if e.kind() == ErrorKind::NotFound => continue,
                Err(e) if e.kind() == ErrorKind::PermissionDenied => return Err(copy::NEEDS_ADMIN.to_string()),
                Err(e) => return Err(format!(r"HKLM\{base}\{kind}: {e}")),
            };
            for (name, _) in key.enum_values().flatten() {
                items.push(exclusion_item(kind, &name, policy));
            }
        }
    }
    // The same exclusion in both lists is one item: the Windows Security one, read first.
    items.sort_by(|a, b| a.key.cmp(&b.key).then(a.props.len().cmp(&b.props.len())));
    items.dedup_by(|a, b| a.key == b.key);
    Ok(crate::SourceOutput { items, note: None })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exclusions_are_keyed_by_kind_and_value() {
        let path = exclusion_item("Paths", r"C:\Program Files\Example\", false);
        assert_eq!(path.key, r"paths|c:\program files\example\");
        assert_eq!(path.label, r"C:\Program Files\Example\");
        assert_eq!(path.target.as_deref(), Some(r"c:\program files\example\"));
        assert_eq!(path.props.get("policy"), None);

        let ext = exclusion_item("Extensions", ".LOG", true);
        assert_eq!(ext.key, "extensions|.log");
        assert_eq!(ext.target, None);
        assert_eq!(ext.props["policy"], "1");
    }

    #[cfg(windows)]
    #[test]
    fn defender_unelevated_is_an_error_not_empty() {
        if crate::elevation::is_elevated() {
            return;
        }
        assert_eq!(read(), Err(idiff_core::copy::NEEDS_ADMIN.to_string()));
    }
}
