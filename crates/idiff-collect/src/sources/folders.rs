//! Folders and shortcuts: what sits at the top of Program Files, ProgramData and the user's
//! AppData, the user's dot-folders, and the shortcuts on the Desktops and in the Start menus.

use idiff_core::command::normalize_path;
use idiff_core::model::{Env, Item};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// `folder_size` stops counting after this many entries, so one huge folder can't stall a run.
pub const MAX_ENTRIES: u64 = 2_000_000;

const START_MENU: &str = r"Microsoft\Windows\Start Menu\Programs";

/// The folders watched, by label, from the environment. A root whose variable isn't set is left out.
pub fn roots(env: &Env) -> Vec<(String, PathBuf)> {
    let var = |name: &str| env.get(name).filter(|v| !v.is_empty()).map(PathBuf::from);
    let profile = var("USERPROFILE");
    let public = var("PUBLIC").or_else(|| profile.as_ref()?.parent().map(|users| users.join("Public")));
    [
        ("Program Files", var("ProgramFiles")),
        ("Program Files (x86)", var("ProgramFiles(x86)")),
        ("ProgramData", var("ProgramData")),
        (r"AppData\Local", var("LOCALAPPDATA")),
        (
            r"AppData\Local\Programs",
            var("LOCALAPPDATA").map(|p| p.join("Programs")),
        ),
        (r"AppData\Roaming", var("APPDATA")),
        ("User folder", profile.clone()),
        ("Desktop", profile.map(|p| p.join("Desktop"))),
        ("Public Desktop", public.map(|p| p.join("Desktop"))),
        ("Start menu", var("APPDATA").map(|p| p.join(START_MENU))),
        ("Start menu (all users)", var("ProgramData").map(|p| p.join(START_MENU))),
    ]
    .into_iter()
    .filter_map(|(label, path)| Some((label.to_string(), path?)))
    .collect()
}

/// What a root contributes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Listing {
    /// Its top-level folders.
    Folders,
    /// Its top-level folders whose names start with a dot (the user folder).
    DotFolders,
    /// Its shortcuts, down to `depth` levels.
    Shortcuts { depth: usize },
}

pub fn listing(root: &str) -> Listing {
    match root {
        "User folder" => Listing::DotFolders,
        "Desktop" | "Public Desktop" => Listing::Shortcuts { depth: 1 },
        "Start menu" | "Start menu (all users)" => Listing::Shortcuts { depth: 3 },
        _ => Listing::Folders,
    }
}

/// One folder or shortcut. `relative` is its path under the root; `target` is a shortcut's.
pub fn folder_item(root: &str, relative: &str, path: &Path, target: Option<String>) -> Item {
    let kind = if relative.to_ascii_lowercase().ends_with(".lnk") {
        "shortcut"
    } else {
        "folder"
    };
    Item {
        key: normalize_path(&path.to_string_lossy(), crate::env()),
        label: format!(r"{root}\{relative}"),
        target,
        props: BTreeMap::from([
            ("root".to_string(), root.to_string()),
            ("kind".to_string(), kind.to_string()),
        ]),
    }
}

/// (bytes, files) under `path`. Junctions and symbolic links inside it are neither counted nor
/// followed: AppData has junctions that point back up the tree or elsewhere on the disk.
pub fn folder_size(path: &Path) -> (u64, u64) {
    let (mut bytes, mut files, mut seen) = (0, 0, 0);
    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            seen += 1;
            if seen > MAX_ENTRIES {
                return (bytes, files);
            }
            // On Windows a junction or symlink is neither `is_dir` nor `is_file`.
            let Ok(kind) = entry.file_type() else { continue };
            if kind.is_dir() {
                stack.push(entry.path());
            } else if kind.is_file() {
                files += 1;
                bytes += entry.metadata().map_or(0, |m| m.len());
            }
        }
    }
    (bytes, files)
}

#[cfg(windows)]
pub use reader::read;

#[cfg(windows)]
mod reader {
    use super::{Listing, folder_item, listing, roots};
    use crate::SourceOutput;
    use crate::com::Com;
    use crate::reg::open_error;
    use crate::shortcut;
    use idiff_core::model::Item;
    use std::path::{Path, PathBuf};
    use windows::Win32::System::Com::CoTaskMemFree;
    use windows::Win32::UI::Shell::{
        FOLDERID_CommonPrograms, FOLDERID_Desktop, FOLDERID_Programs, FOLDERID_PublicDesktop, KF_FLAG_DEFAULT,
        SHGetKnownFolderPath,
    };
    use windows::core::GUID;

    pub fn read() -> Result<SourceOutput, String> {
        let _com = Com::sta();
        let mut items = Vec::new();
        let mut errors = Vec::new();
        for (root, mut path) in roots(crate::env()) {
            // The shell knows where these really are (OneDrive can move the Desktop).
            let known = match root.as_str() {
                "Desktop" => Some(FOLDERID_Desktop),
                "Public Desktop" => Some(FOLDERID_PublicDesktop),
                "Start menu" => Some(FOLDERID_Programs),
                "Start menu (all users)" => Some(FOLDERID_CommonPrograms),
                _ => None,
            };
            if let Some(found) = known.and_then(|id| known_folder(&id)) {
                path = found;
            }
            match listing(&root) {
                Listing::Shortcuts { depth } => shortcuts(&root, &path, "", depth, &mut items, &mut errors),
                Listing::Folders | Listing::DotFolders => {
                    let dots_only = listing(&root) == Listing::DotFolders;
                    let entries = match std::fs::read_dir(&path) {
                        Ok(entries) => entries,
                        Err(e) => {
                            errors.extend(open_error(&e).map(|error| format!("{}: {error}", path.display())));
                            continue;
                        }
                    };
                    for entry in entries.flatten() {
                        let name = entry.file_name().to_string_lossy().into_owned();
                        let is_dir = entry.file_type().is_ok_and(|t| t.is_dir());
                        if is_dir && (!dots_only || name.starts_with('.')) {
                            items.push(folder_item(&root, &name, &entry.path(), None));
                        }
                    }
                }
            }
        }
        if items.is_empty() && !errors.is_empty() {
            return Err(errors.join("; "));
        }
        items.sort_by(|a, b| a.key.cmp(&b.key));
        items.dedup_by(|a, b| a.key == b.key);
        let note = (!errors.is_empty()).then(|| format!("Couldn't read {}", errors.join("; ")));
        Ok(SourceOutput { items, note })
    }

    /// The `.lnk` files in `dir` and its subfolders, `depth` levels down, with their targets.
    fn shortcuts(root: &str, dir: &Path, prefix: &str, depth: usize, items: &mut Vec<Item>, errors: &mut Vec<String>) {
        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(e) => {
                errors.extend(open_error(&e).map(|error| format!("{}: {error}", dir.display())));
                return;
            }
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let relative = if prefix.is_empty() {
                name.clone()
            } else {
                format!(r"{prefix}\{name}")
            };
            let Ok(kind) = entry.file_type() else { continue };
            if kind.is_dir() && depth > 1 {
                shortcuts(root, &entry.path(), &relative, depth - 1, items, errors);
            } else if kind.is_file() && name.to_ascii_lowercase().ends_with(".lnk") {
                let path = entry.path();
                items.push(folder_item(root, &relative, &path, shortcut::resolve(&path)));
            }
        }
    }

    fn known_folder(id: &GUID) -> Option<PathBuf> {
        unsafe {
            let path = SHGetKnownFolderPath(id, KF_FLAG_DEFAULT, None).ok()?;
            let text = path.to_string().ok();
            CoTaskMemFree(Some(path.0 as *const _));
            text.map(PathBuf::from)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn env() -> Env {
        Env::from([
            ("ProgramFiles".into(), r"C:\Program Files".into()),
            ("ProgramFiles(x86)".into(), r"C:\Program Files (x86)".into()),
            ("ProgramData".into(), r"C:\ProgramData".into()),
            ("LOCALAPPDATA".into(), r"C:\Users\Niccolò\AppData\Local".into()),
            ("APPDATA".into(), r"C:\Users\Niccolò\AppData\Roaming".into()),
            ("USERPROFILE".into(), r"C:\Users\Niccolò".into()),
        ])
    }

    #[test]
    fn roots_cover_spec_locations() {
        let roots = roots(&env());
        let got: Vec<(&str, String)> = roots
            .iter()
            .map(|(l, p)| (l.as_str(), p.display().to_string()))
            .collect();
        let want = [
            ("Program Files", r"C:\Program Files"),
            ("Program Files (x86)", r"C:\Program Files (x86)"),
            ("ProgramData", r"C:\ProgramData"),
            (r"AppData\Local", r"C:\Users\Niccolò\AppData\Local"),
            (r"AppData\Local\Programs", r"C:\Users\Niccolò\AppData\Local\Programs"),
            (r"AppData\Roaming", r"C:\Users\Niccolò\AppData\Roaming"),
            ("User folder", r"C:\Users\Niccolò"),
            ("Desktop", r"C:\Users\Niccolò\Desktop"),
            ("Public Desktop", r"C:\Users\Public\Desktop"),
            (
                "Start menu",
                r"C:\Users\Niccolò\AppData\Roaming\Microsoft\Windows\Start Menu\Programs",
            ),
            (
                "Start menu (all users)",
                r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs",
            ),
        ];
        let want: Vec<(&str, String)> = want.iter().map(|(l, p)| (*l, p.to_string())).collect();
        assert_eq!(got, want);
        assert_eq!(roots_of_missing_env(), 0);
    }

    fn roots_of_missing_env() -> usize {
        roots(&Env::new()).len()
    }

    /// Closed before it's measured: while a file is open, its folder listing can show an old size.
    fn ten_byte_file(dir: &Path) -> tempfile::TempPath {
        let mut f = tempfile::Builder::new().tempfile_in(dir).unwrap();
        f.write_all(b"0123456789").unwrap();
        f.into_temp_path()
    }

    #[test]
    fn folder_size_counts_files() {
        let dir = tempfile::tempdir().unwrap();
        let sub = tempfile::tempdir_in(dir.path()).unwrap();
        let _files = [ten_byte_file(dir.path()), ten_byte_file(sub.path())];
        assert_eq!(folder_size(dir.path()), (20, 2));
        assert_eq!(folder_size(&dir.path().join("missing")), (0, 0));
    }

    #[cfg(windows)]
    #[test]
    fn folder_size_does_not_follow_junctions() {
        let dir = tempfile::tempdir().unwrap();
        let inside = tempfile::tempdir_in(dir.path()).unwrap();
        let outside = tempfile::tempdir().unwrap();
        let _files = [
            ten_byte_file(inside.path()),
            ten_byte_file(outside.path()),
            ten_byte_file(outside.path()),
        ];
        let link = inside.path().join("junction");
        let made = std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(&link)
            .arg(outside.path())
            .output()
            .unwrap();
        assert!(made.status.success(), "{}", String::from_utf8_lossy(&made.stderr));
        assert_eq!(
            folder_size(&link),
            (20, 2),
            "the junction itself does point at two files"
        );
        assert_eq!(folder_size(dir.path()), (10, 1));
    }
}
