//! Installed programs: the Uninstall keys Settings and Control Panel list.

use idiff_core::command::normalize_path;
use idiff_core::model::Item;
use std::collections::BTreeMap;

/// One Uninstall subkey as an item, or None when it has no display name (updates, leftovers and
/// helper entries that no list shows).
pub fn program_item(hive: &str, view: &str, subkey: &str, values: &BTreeMap<String, String>) -> Option<Item> {
    let name = values.get("DisplayName").map(|n| n.trim()).filter(|n| !n.is_empty())?;
    let value = |v: &str| values.get(v).map(|s| s.trim()).filter(|s| !s.is_empty());
    let location = value("InstallLocation");

    let mut props = BTreeMap::new();
    for (prop, from) in [
        ("version", "DisplayVersion"),
        ("publisher", "Publisher"),
        ("size_kb", "EstimatedSize"),
        ("install_location", "InstallLocation"),
    ] {
        if let Some(v) = value(from) {
            props.insert(prop.to_string(), v.to_string());
        }
    }
    if value("SystemComponent") == Some("1") {
        props.insert("system_component".into(), "1".into());
    }

    Some(Item {
        key: format!(r"{hive}\{view}\{subkey}").to_lowercase(),
        label: name.to_string(),
        target: location
            .map(|l| normalize_path(l, crate::env()))
            .filter(|l| !l.is_empty()),
        props,
    })
}

#[cfg(windows)]
pub use reader::read;

#[cfg(windows)]
mod reader {
    use super::program_item;
    use crate::SourceOutput;
    use crate::reg::{open_error, values};
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

    const UNINSTALL: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";
    const UNINSTALL_32: &str = r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall";

    /// Every program in the machine's 64- and 32-bit lists and the user's own list. A list that
    /// can't be opened (other than not existing) makes the result partial; all three failing is
    /// an error.
    pub fn read() -> Result<crate::SourceOutput, String> {
        let roots = [
            (HKEY_LOCAL_MACHINE, "HKLM", "native", UNINSTALL),
            (HKEY_LOCAL_MACHINE, "HKLM", "WOW6432Node", UNINSTALL_32),
            (HKEY_CURRENT_USER, "HKCU", "native", UNINSTALL),
        ];
        let mut items = Vec::new();
        let mut errors = Vec::new();
        for (hive, hive_name, view, path) in roots {
            let root = match RegKey::predef(hive).open_subkey(path) {
                Ok(root) => root,
                Err(e) => {
                    if let Some(error) = open_error(&e) {
                        errors.push(format!(r"{hive_name}\{path}: {error}"));
                    }
                    continue;
                }
            };
            for subkey in root.enum_keys().flatten() {
                match root.open_subkey(&subkey) {
                    Ok(key) => items.extend(program_item(hive_name, view, &subkey, &values(&key))),
                    Err(e) => {
                        errors.extend(open_error(&e).map(|error| format!(r"{hive_name}\{path}\{subkey}: {error}")))
                    }
                }
            }
        }
        if items.is_empty() && !errors.is_empty() {
            return Err(errors.join("; "));
        }
        items.sort_by(|a, b| a.key.cmp(&b.key));
        let note = (!errors.is_empty()).then(|| format!("Couldn't read {}", errors.join("; ")));
        Ok(SourceOutput { items, note })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn program_without_name_is_skipped() {
        assert_eq!(
            program_item("HKLM", "native", "KB123", &values(&[("DisplayVersion", "1.0")])),
            None
        );
        assert_eq!(
            program_item("HKLM", "native", "KB123", &values(&[("DisplayName", " ")])),
            None
        );
    }

    #[test]
    fn program_key_includes_hive_and_view() {
        let item = program_item("HKLM", "WOW6432Node", "{ABC}", &values(&[("DisplayName", "VLC")])).unwrap();
        assert_eq!(item.key, r"hklm\wow6432node\{abc}");
        assert_eq!(item.label, "VLC");
        assert_eq!(item.target, None);
    }

    #[test]
    fn program_props_are_mapped() {
        let item = program_item(
            "HKCU",
            "native",
            "Discord",
            &values(&[
                ("DisplayName", "Discord"),
                ("DisplayVersion", "1.0.9170"),
                ("Publisher", "Discord Inc."),
                ("SystemComponent", "0"),
                ("EstimatedSize", "412000"),
                ("InstallLocation", r#""C:\Users\Niccolò\AppData\Local\Discord""#),
                ("UninstallString", "ignored"),
            ]),
        )
        .unwrap();
        assert_eq!(item.target.as_deref(), Some(r"c:\users\niccolò\appdata\local\discord"));
        let want = values(&[
            ("version", "1.0.9170"),
            ("publisher", "Discord Inc."),
            ("size_kb", "412000"),
            ("install_location", r#""C:\Users\Niccolò\AppData\Local\Discord""#),
        ]);
        assert_eq!(item.props, want);

        let hidden = program_item(
            "HKLM",
            "native",
            "x",
            &values(&[("DisplayName", "Runtime"), ("SystemComponent", "1")]),
        );
        assert_eq!(hidden.unwrap().props["system_component"], "1");
    }
}
