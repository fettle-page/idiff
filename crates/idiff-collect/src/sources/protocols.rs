//! Link handlers: the URL protocols (`discord://`, `steam://`) registered for the machine and for
//! the user, each with the command Windows runs to open such a link.

use idiff_core::command::extract_exe;
use idiff_core::model::Item;
use std::collections::BTreeMap;

/// A class name that can be a URL protocol. File extensions (`.txt`) never are.
pub fn may_be_protocol(name: &str) -> bool {
    !name.is_empty() && !name.starts_with('.')
}

/// One protocol under `HKLM` or `HKCU` `Software\Classes`; its target is the exe its
/// `shell\open\command` runs, as for a startup entry.
pub fn protocol_item(hive: &str, scheme: &str, command: Option<&str>) -> Item {
    let command = command.map(str::trim).filter(|c| !c.is_empty());
    Item {
        key: format!("protocol|{hive}|{scheme}").to_lowercase(),
        label: scheme.to_string(),
        target: command.and_then(|c| extract_exe(c, crate::env())),
        props: command
            .map(|c| BTreeMap::from([("command".to_string(), c.to_string())]))
            .unwrap_or_default(),
    }
}

#[cfg(windows)]
pub use reader::read;

#[cfg(windows)]
mod reader {
    use super::{may_be_protocol, protocol_item};
    use crate::SourceOutput;
    use crate::reg::open_error;
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_QUERY_VALUE};

    const CLASSES: &str = r"Software\Classes";

    /// The machine's and the user's classes, each read on its own: HKCR merges them, and an app
    /// registering a protocol for the user only would hide which one it changed.
    pub fn read() -> Result<SourceOutput, String> {
        let mut items = Vec::new();
        let mut errors = Vec::new();
        for (hive, hive_name) in [(HKEY_LOCAL_MACHINE, "HKLM"), (HKEY_CURRENT_USER, "HKCU")] {
            let classes = match RegKey::predef(hive).open_subkey(CLASSES) {
                Ok(key) => key,
                Err(e) => {
                    errors.extend(open_error(&e).map(|error| format!(r"{hive_name}\{CLASSES}: {error}")));
                    continue;
                }
            };
            for name in classes.enum_keys().flatten().filter(|n| may_be_protocol(n)) {
                // Thousands of classes, so each is opened only to ask for the one value.
                let is_protocol = classes
                    .open_subkey_with_flags(&name, KEY_QUERY_VALUE)
                    .is_ok_and(|k| k.get_raw_value("URL Protocol").is_ok());
                if !is_protocol {
                    continue;
                }
                let command = classes
                    .open_subkey(format!(r"{name}\shell\open\command"))
                    .and_then(|k| k.get_value::<String, _>(""))
                    .ok();
                items.push(protocol_item(hive_name, &name, command.as_deref()));
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

    #[test]
    fn protocol_target_is_the_command_exe() {
        let item = protocol_item(
            "HKCU",
            "Discord",
            Some(
                r#""C:\Users\a\AppData\Local\Discord\Update.exe" --processStart Discord.exe --process-start-args "%1""#,
            ),
        );
        assert_eq!(item.key, "protocol|hkcu|discord");
        assert_eq!(item.label, "Discord");
        assert_eq!(
            item.target.as_deref(),
            Some(r"c:\users\a\appdata\local\discord\update.exe")
        );
        assert!(item.props["command"].contains("--processStart"));

        let bare = protocol_item("HKLM", "ms-foo", Some("  "));
        assert_eq!((bare.target, bare.props.len()), (None, 0));
        assert!(may_be_protocol("steam") && !may_be_protocol(".txt") && !may_be_protocol(""));
    }
}
