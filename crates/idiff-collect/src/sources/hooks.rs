//! Hooks that make Windows run something in place of, or inside, other programs: Image File
//! Execution Options debuggers, Winlogon's `Shell` and `Userinit`, and `AppInit_DLLs`.

use idiff_core::command::{extract_exe, normalize_path};
use idiff_core::model::Item;
use std::collections::BTreeMap;

/// An Image File Execution Options key with a `Debugger`: Windows starts the debugger instead
/// of `exe`. `view` is `64` or `32` (WOW6432Node). An empty debugger does nothing, so it's no item.
pub fn ifeo_item(view: &str, exe: &str, debugger: &str) -> Option<Item> {
    let debugger = debugger.trim();
    (!debugger.is_empty()).then(|| Item {
        key: format!("ifeo|{view}|{exe}").to_lowercase(),
        label: exe.to_string(),
        target: extract_exe(debugger, crate::env()),
        props: BTreeMap::from([("debugger".to_string(), debugger.to_string())]),
    })
}

/// Winlogon's `Shell` or `Userinit` under `hive`. Both can list several programs, separated by
/// commas; the target is the first one that isn't Windows' own (outside the Windows folder, or
/// given as a full path), so a program added after `userinit.exe` is what attribution looks at.
pub fn winlogon_item(hive: &str, name: &str, value: &str) -> Option<Item> {
    let value = value.trim();
    if value.trim_matches(',').trim().is_empty() {
        return None;
    }
    let env = crate::env();
    let system_root = env
        .get("SystemRoot")
        .map(|root| format!("{}\\", root.trim_end_matches('\\').to_lowercase()));
    let programs: Vec<String> = value.split(',').filter_map(|part| extract_exe(part, env)).collect();
    let not_windows = programs
        .iter()
        .find(|p| p.contains('\\') && !system_root.as_deref().is_some_and(|root| p.starts_with(root)));
    Some(Item {
        key: format!("winlogon|{hive}|{name}").to_lowercase(),
        label: name.to_string(),
        target: not_windows.or(programs.first()).cloned(),
        props: BTreeMap::from([("value".to_string(), value.to_string())]),
    })
}

/// One item per DLL in an `AppInit_DLLs` value (separated by spaces or commas), with
/// `LoadAppInit_DLLs`, which says whether Windows loads them at all. `view` is `64` or `32`.
pub fn appinit_items(view: &str, dlls: &str, load: Option<u32>) -> Vec<Item> {
    dlls.split([' ', ',', '\0'])
        .filter(|dll| !dll.is_empty())
        .map(|dll| {
            let path = normalize_path(dll, crate::env());
            Item {
                key: format!("appinit|{view}|{path}"),
                label: dll.to_string(),
                target: Some(path),
                props: load
                    .map(|l| BTreeMap::from([("load".to_string(), l.to_string())]))
                    .unwrap_or_default(),
            }
        })
        .collect()
}

#[cfg(windows)]
pub use reader::read;

#[cfg(windows)]
mod reader {
    use super::{appinit_items, ifeo_item, winlogon_item};
    use crate::SourceOutput;
    use crate::reg::open_error;
    use idiff_core::model::Item;
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

    const NT: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";
    const NT_32: &str = r"SOFTWARE\WOW6432Node\Microsoft\Windows NT\CurrentVersion";

    /// Every part. A key that's missing has nothing in it; one that can't be opened makes the
    /// result partial, and all of them, an error. A single IFEO key that can't be opened is
    /// skipped, as other sources skip one unreadable entry: Defender protects its own
    /// (`DefenderAgentScan.exe`) from everyone but itself.
    pub fn read() -> Result<SourceOutput, String> {
        let mut items: Vec<Item> = Vec::new();
        let mut errors = Vec::new();
        let mut read_ok = 0;
        let mut open = |hive, hive_name: &str, path: &str| match RegKey::predef(hive).open_subkey(path) {
            Ok(key) => {
                read_ok += 1;
                Some(key)
            }
            Err(e) => match open_error(&e) {
                Some(error) => {
                    errors.push(format!(r"{hive_name}\{path}: {error}"));
                    None
                }
                None => {
                    read_ok += 1;
                    None
                }
            },
        };

        for (view, base) in [("64", NT), ("32", NT_32)] {
            let Some(ifeo) = open(
                HKEY_LOCAL_MACHINE,
                "HKLM",
                &format!(r"{base}\Image File Execution Options"),
            ) else {
                continue;
            };
            for exe in ifeo.enum_keys().flatten() {
                let debugger = ifeo
                    .open_subkey(&exe)
                    .and_then(|k| k.get_value::<String, _>("Debugger"))
                    .ok();
                let Some(item) = debugger.and_then(|d| ifeo_item(view, &exe, &d)) else {
                    continue;
                };
                // The two views are often one key: list a debugger once unless they differ.
                let same_in_64 = view == "32"
                    && items
                        .iter()
                        .any(|i| i.key == item.key.replacen("ifeo|32|", "ifeo|64|", 1) && i.props == item.props);
                if !same_in_64 {
                    items.push(item);
                }
            }
        }

        for (hive, hive_name) in [(HKEY_LOCAL_MACHINE, "HKLM"), (HKEY_CURRENT_USER, "HKCU")] {
            let Some(winlogon) = open(hive, hive_name, &format!(r"{NT}\Winlogon")) else {
                continue;
            };
            for name in ["Shell", "Userinit"] {
                if let Ok(value) = winlogon.get_value::<String, _>(name) {
                    items.extend(winlogon_item(hive_name, name, &value));
                }
            }
        }

        for (view, base) in [("64", NT), ("32", NT_32)] {
            let Some(windows) = open(HKEY_LOCAL_MACHINE, "HKLM", &format!(r"{base}\Windows")) else {
                continue;
            };
            if let Ok(dlls) = windows.get_value::<String, _>("AppInit_DLLs") {
                let load = windows.get_value::<u32, _>("LoadAppInit_DLLs").ok();
                items.extend(appinit_items(view, &dlls, load));
            }
        }

        if read_ok == 0 {
            return Err(errors.join("; "));
        }
        items.sort_by(|a, b| a.key.cmp(&b.key));
        items.dedup_by(|a, b| a.key == b.key);
        let note = (!errors.is_empty()).then(|| format!("Couldn't read {}", errors.join("; ")));
        Ok(SourceOutput { items, note })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ifeo_debugger_is_the_target() {
        let item = ifeo_item("64", "Notepad.exe", r#""C:\Program Files\Tool\tool.exe" -z"#).unwrap();
        assert_eq!(item.key, "ifeo|64|notepad.exe");
        assert_eq!(item.label, "Notepad.exe");
        assert_eq!(item.target.as_deref(), Some(r"c:\program files\tool\tool.exe"));
        assert_eq!(ifeo_item("32", "x.exe", "  "), None);
    }

    #[test]
    fn winlogon_target_is_the_program_that_isnt_windows() {
        let root = crate::env()["SystemRoot"].to_lowercase();
        let stock = winlogon_item("HKLM", "Userinit", r"C:\Windows\system32\userinit.exe,").unwrap();
        assert_eq!(stock.key, "winlogon|hklm|userinit");
        assert_eq!(stock.target, Some(format!(r"{root}\system32\userinit.exe")));
        let added = winlogon_item(
            "HKLM",
            "Userinit",
            r#"%SystemRoot%\system32\userinit.exe,"C:\Program Files\Kiosk\kiosk.exe","#,
        )
        .unwrap();
        assert_eq!(added.target.as_deref(), Some(r"c:\program files\kiosk\kiosk.exe"));
        assert_eq!(
            winlogon_item("HKCU", "Shell", "explorer.exe")
                .unwrap()
                .target
                .as_deref(),
            Some("explorer.exe")
        );
        assert_eq!(winlogon_item("HKCU", "Shell", " , "), None);
    }

    #[test]
    fn appinit_lists_each_dll_with_the_load_flag() {
        let items = appinit_items("32", r"C:\PROGRA~1\Hook\hook.dll, c:\x\y.dll", Some(1));
        let keys: Vec<_> = items.iter().map(|i| i.key.as_str()).collect();
        assert_eq!(
            keys,
            [r"appinit|32|c:\progra~1\hook\hook.dll", r"appinit|32|c:\x\y.dll"]
        );
        assert_eq!(items[0].label, r"C:\PROGRA~1\Hook\hook.dll");
        assert_eq!(items[0].props["load"], "1");
        assert!(appinit_items("64", "", Some(0)).is_empty());
    }
}
