//! Right-click menu entries: context menu handlers and shell verbs on the roots that show them
//! for files, folders, drives and folder backgrounds.

use idiff_core::command::{extract_exe, normalize_path};
use idiff_core::model::Item;

/// Where Explorer looks for right-click entries, under HKEY_CLASSES_ROOT.
pub const ROOTS: &[&str] = &[
    "*",
    "Directory",
    r"Directory\Background",
    "Folder",
    "Drive",
    "AllFilesystemObjects",
];

/// A context menu handler; `server` is its CLSID's InprocServer32 DLL.
pub fn handler_item(root: &str, name: &str, server: Option<&str>) -> Item {
    Item {
        key: format!("{root}|handler|{name}").to_lowercase(),
        label: name.to_string(),
        target: server
            .map(|s| normalize_path(s, crate::env()))
            .filter(|s| !s.is_empty()),
        props: Default::default(),
    }
}

/// A shell verb; its target is the exe its command runs.
pub fn verb_item(root: &str, verb: &str, command: Option<&str>) -> Item {
    Item {
        key: format!("{root}|verb|{verb}").to_lowercase(),
        label: verb.to_string(),
        target: command.and_then(|c| extract_exe(c, crate::env())),
        props: Default::default(),
    }
}

#[cfg(windows)]
pub use reader::read;

#[cfg(windows)]
mod reader {
    use super::{ROOTS, handler_item, verb_item};
    use crate::SourceOutput;
    use crate::reg::open_error;
    use winreg::RegKey;
    use winreg::enums::HKEY_CLASSES_ROOT;

    /// HKCR is the merged view of the machine's and the user's classes, as Explorer sees them.
    pub fn read() -> Result<SourceOutput, String> {
        let classes = RegKey::predef(HKEY_CLASSES_ROOT);
        let mut items = Vec::new();
        let mut errors = Vec::new();
        let mut open = |path: &str| match classes.open_subkey(path) {
            Ok(key) => Some(key),
            Err(e) => {
                errors.extend(open_error(&e).map(|error| format!(r"HKCR\{path}: {error}")));
                None
            }
        };

        for root in ROOTS {
            if let Some(handlers) = open(&format!(r"{root}\shellex\ContextMenuHandlers")) {
                for name in handlers.enum_keys().flatten() {
                    let clsid = handlers
                        .open_subkey(&name)
                        .ok()
                        .and_then(|k| k.get_value::<String, _>("").ok());
                    // Some handlers are registered under their CLSID as the key name, with no default value.
                    let clsid = clsid.filter(|c| c.starts_with('{')).unwrap_or_else(|| name.clone());
                    let server = classes
                        .open_subkey(format!(r"CLSID\{clsid}\InprocServer32"))
                        .and_then(|k| k.get_value::<String, _>(""))
                        .ok();
                    let mut item = handler_item(root, &name, server.as_deref());
                    if clsid.starts_with('{') {
                        item.props.insert("clsid".into(), clsid.to_lowercase());
                    }
                    items.push(item);
                }
            }
            if let Some(verbs) = open(&format!(r"{root}\shell")) {
                for verb in verbs.enum_keys().flatten() {
                    let command = verbs
                        .open_subkey(format!(r"{verb}\command"))
                        .and_then(|k| k.get_value::<String, _>(""))
                        .ok();
                    items.push(verb_item(root, &verb, command.as_deref()));
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

    #[test]
    fn verb_target_is_the_exe() {
        let item = verb_item(
            "Directory",
            "Open with 7-Zip",
            Some(r#""C:\Program Files\7-Zip\7zFM.exe" "%1""#),
        );
        assert_eq!(item.key, "directory|verb|open with 7-zip");
        assert_eq!(item.label, "Open with 7-Zip");
        assert_eq!(item.target.as_deref(), Some(r"c:\program files\7-zip\7zfm.exe"));
        assert_eq!(verb_item("*", "x", None).target, None);
    }

    #[test]
    fn handler_target_is_the_server() {
        let item = handler_item(
            r"Directory\Background",
            "7-Zip",
            Some(r#""C:\Program Files\7-Zip\7-zip.dll""#),
        );
        assert_eq!(item.key, r"directory\background|handler|7-zip");
        assert_eq!(item.target.as_deref(), Some(r"c:\program files\7-zip\7-zip.dll"));
        assert_eq!(handler_item("*", "x", Some("  ")).target, None);
    }
}
