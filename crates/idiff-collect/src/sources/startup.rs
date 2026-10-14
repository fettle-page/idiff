// Copied from fettle-page/fettle@442e166 crates/fettle-collect/src/sources/startup.rs (MIT). Changes: returns Items instead of JSON, with StartupApproved matched here.
//! What starts with Windows: Run and RunOnce values and the Startup folders, each with its
//! enabled state from Task Manager (StartupApproved).

use idiff_core::command::{extract_exe, normalize_path};
use idiff_core::model::Item;
use std::collections::BTreeMap;

/// One StartupApproved value: `hive` is `HKCU` or `HKLM`, `key` is `Run`, `Run32` or `StartupFolder`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Approved {
    pub hive: String,
    pub key: String,
    pub name: String,
    pub enabled: Option<bool>,
}

/// Task Manager's enabled/disabled flag: the low bit of the first byte set means disabled.
pub fn startup_approved_state(bytes: &[u8]) -> Option<bool> {
    bytes.first().map(|b| b & 1 == 0)
}

fn enabled(approved: &[Approved], hive: &str, key: &str, name: &str) -> Option<bool> {
    approved
        .iter()
        .find(|a| a.hive.eq_ignore_ascii_case(hive) && a.key == key && a.name.eq_ignore_ascii_case(name))
        .and_then(|a| a.enabled)
}

fn props(pairs: impl IntoIterator<Item = (&'static str, Option<String>)>) -> BTreeMap<String, String> {
    pairs
        .into_iter()
        .filter_map(|(k, v)| Some((k.to_string(), v?)))
        .collect()
}

/// A Run or RunOnce value. RunOnce values have no enabled state; Task Manager doesn't list them.
pub fn run_item(hive: &str, key_path: &str, name: &str, command: &str, approved: &[Approved]) -> Item {
    let lower = key_path.to_ascii_lowercase();
    let state = if lower.ends_with("runonce") {
        None
    } else {
        let approved_key = if lower.contains("wow6432node") { "Run32" } else { "Run" };
        enabled(approved, hive, approved_key, name)
    };
    Item {
        key: format!("run|{hive}|{key_path}|{name}").to_lowercase(),
        label: name.to_string(),
        target: extract_exe(command, crate::env()),
        props: props([
            ("command", Some(command.to_string())),
            ("enabled", state.map(|e| e.to_string())),
        ]),
    }
}

/// A file in the user's (`user`) or everyone's (`common`) Startup folder. Other files (scripts,
/// programs) are their own target; a shortcut's target is filled in by `read()`.
pub fn folder_item(scope: &str, file: &str, path: &str, approved: &[Approved]) -> Item {
    let hive = if scope == "user" { "HKCU" } else { "HKLM" };
    let is_shortcut = file.to_ascii_lowercase().ends_with(".lnk");
    Item {
        key: format!("folder|{scope}|{file}").to_lowercase(),
        label: file.to_string(),
        target: (!is_shortcut).then(|| normalize_path(path, crate::env())),
        props: props([(
            "enabled",
            enabled(approved, hive, "StartupFolder", file).map(|e| e.to_string()),
        )]),
    }
}

#[cfg(windows)]
pub use reader::read;

#[cfg(windows)]
mod reader {
    use super::{Approved, folder_item, run_item, startup_approved_state};
    use crate::SourceOutput;
    use crate::com::Com;
    use crate::reg::open_error;
    use crate::shortcut;
    use std::path::PathBuf;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    use winreg::{HKEY, RegKey};

    const RUN_KEYS: &[(HKEY, &str, &str)] = &[
        (
            HKEY_CURRENT_USER,
            "HKCU",
            r"Software\Microsoft\Windows\CurrentVersion\Run",
        ),
        (
            HKEY_CURRENT_USER,
            "HKCU",
            r"Software\Microsoft\Windows\CurrentVersion\RunOnce",
        ),
        (
            HKEY_LOCAL_MACHINE,
            "HKLM",
            r"Software\Microsoft\Windows\CurrentVersion\Run",
        ),
        (
            HKEY_LOCAL_MACHINE,
            "HKLM",
            r"Software\Microsoft\Windows\CurrentVersion\RunOnce",
        ),
        (
            HKEY_LOCAL_MACHINE,
            "HKLM",
            r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Run",
        ),
        (
            HKEY_LOCAL_MACHINE,
            "HKLM",
            r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\RunOnce",
        ),
    ];
    const APPROVED_BASE: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved";
    const APPROVED_SUBKEYS: &[&str] = &["Run", "Run32", "StartupFolder"];

    pub fn read() -> Result<SourceOutput, String> {
        let _com = Com::sta();
        let mut errors = Vec::new();
        let approved = startup_approved(&mut errors);
        let mut items = Vec::new();

        for &(hive, hive_name, path) in RUN_KEYS {
            let key = match RegKey::predef(hive).open_subkey(path) {
                Ok(key) => key,
                Err(e) => {
                    errors.extend(open_error(&e).map(|error| format!(r"{hive_name}\{path}: {error}")));
                    continue;
                }
            };
            for (name, value) in crate::reg::values(&key) {
                items.push(run_item(hive_name, path, &name, &value, &approved));
            }
        }

        let folders = [
            (
                "user",
                std::env::var("APPDATA")
                    .map(|p| PathBuf::from(p).join(r"Microsoft\Windows\Start Menu\Programs\Startup")),
            ),
            (
                "common",
                std::env::var("ProgramData")
                    .map(|p| PathBuf::from(p).join(r"Microsoft\Windows\Start Menu\Programs\StartUp")),
            ),
        ];
        for (scope, dir) in folders {
            let Ok(dir) = dir else { continue };
            let entries = match std::fs::read_dir(&dir) {
                Ok(entries) => entries,
                Err(e) => {
                    errors.extend(open_error(&e).map(|error| format!("{}: {error}", dir.display())));
                    continue;
                }
            };
            for entry in entries.flatten() {
                let file = entry.file_name().to_string_lossy().into_owned();
                if !file.eq_ignore_ascii_case("desktop.ini") {
                    let mut item = folder_item(scope, &file, &entry.path().to_string_lossy(), &approved);
                    if item.target.is_none() {
                        item.target = shortcut::resolve(&entry.path());
                    }
                    items.push(item);
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

    /// Every StartupApproved value. Without them items just have no enabled state, so a key that
    /// can't be read only adds to `errors`.
    fn startup_approved(errors: &mut Vec<String>) -> Vec<Approved> {
        let mut out = Vec::new();
        for (hive, hive_name) in [(HKEY_CURRENT_USER, "HKCU"), (HKEY_LOCAL_MACHINE, "HKLM")] {
            for sub in APPROVED_SUBKEYS {
                let path = format!(r"{APPROVED_BASE}\{sub}");
                let key = match RegKey::predef(hive).open_subkey(&path) {
                    Ok(key) => key,
                    Err(e) => {
                        errors.extend(open_error(&e).map(|error| format!(r"{hive_name}\{path}: {error}")));
                        continue;
                    }
                };
                for (name, value) in key.enum_values().flatten() {
                    out.push(Approved {
                        hive: hive_name.into(),
                        key: sub.to_string(),
                        name,
                        enabled: startup_approved_state(&value.bytes),
                    });
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

    fn approved(hive: &str, key: &str, name: &str, bytes: &[u8]) -> Approved {
        Approved {
            hive: hive.into(),
            key: key.into(),
            name: name.into(),
            enabled: startup_approved_state(bytes),
        }
    }

    #[test]
    fn startup_approved_first_byte_decides() {
        assert_eq!(startup_approved_state(&[0x02, 0, 0]), Some(true));
        assert_eq!(startup_approved_state(&[0x06, 0, 0]), Some(true));
        assert_eq!(startup_approved_state(&[0x03, 0, 0]), Some(false));
        assert_eq!(startup_approved_state(&[0x07, 0, 0]), Some(false));
        assert_eq!(startup_approved_state(&[]), None);
    }

    #[test]
    fn startup_items_take_enabled_from_approved() {
        let list = [
            approved("HKCU", "Run", "Discord", &[0x03]),
            approved("HKLM", "Run32", "Steam", &[0x02]),
            approved("HKCU", "StartupFolder", "Spotify.lnk", &[0x03]),
            approved("HKLM", "Run", "Discord", &[0x02]),
        ];
        let discord = run_item(
            "HKCU",
            RUN,
            "Discord",
            r"%LOCALAPPDATA%\Discord\Update.exe --processStart Discord.exe",
            &list,
        );
        assert_eq!(
            discord.key,
            r"run|hkcu|software\microsoft\windows\currentversion\run|discord"
        );
        assert_eq!(discord.label, "Discord");
        assert!(
            discord
                .target
                .as_deref()
                .is_some_and(|t| t.ends_with(r"\discord\update.exe")),
            "{discord:?}"
        );
        assert_eq!(discord.props["enabled"], "false");
        assert_eq!(
            discord.props["command"],
            r"%LOCALAPPDATA%\Discord\Update.exe --processStart Discord.exe"
        );

        let wow = r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Run";
        assert_eq!(
            run_item("HKLM", wow, "steam", r#""C:\Steam\steam.exe" -silent"#, &list).props["enabled"],
            "true"
        );

        let once = run_item(
            "HKCU",
            r"Software\Microsoft\Windows\CurrentVersion\RunOnce",
            "Discord",
            "x.exe",
            &list,
        );
        assert_eq!(once.props.get("enabled"), None);

        let unknown = run_item("HKCU", RUN, "Other", "x.exe", &list);
        assert_eq!(unknown.props.get("enabled"), None);

        let lnk = folder_item("user", "Spotify.lnk", r"C:\Users\a\Startup\Spotify.lnk", &list);
        assert_eq!(lnk.key, "folder|user|spotify.lnk");
        assert_eq!(lnk.props["enabled"], "false");
        assert_eq!(lnk.target, None);

        let bat = folder_item("common", "Start.bat", r"C:\ProgramData\Startup\Start.bat", &list);
        assert_eq!(bat.target.as_deref(), Some(r"c:\programdata\startup\start.bat"));
        assert_eq!(bat.props.get("enabled"), None);
    }
}
