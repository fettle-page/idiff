//! Shortcut (`.lnk`) targets, read through the shell's own link object.

use idiff_core::command::normalize_path;
use std::path::Path;
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance, IPersistFile, STGM_READ};
use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};
use windows::core::{HSTRING, Interface};

/// The normalized path a shortcut points at, or None when it can't be loaded or points at no
/// file (advertised installer shortcuts, URLs). Loads the link read-only and never calls
/// `Resolve`, which could search the disk or rewrite the link.
pub fn resolve(path: &Path) -> Option<String> {
    let target = unsafe {
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).ok()?;
        link.cast::<IPersistFile>()
            .ok()?
            .Load(&HSTRING::from(path), STGM_READ)
            .ok()?;
        let mut buf = [0u16; 1024];
        link.GetPath(&mut buf, std::ptr::null_mut(), 0).ok()?;
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        String::from_utf16_lossy(&buf[..len])
    };
    let target = normalize_path(&target, crate::env());
    (!target.is_empty()).then_some(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance, IPersistFile};
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};
    use windows::core::{HSTRING, Interface};

    /// Makes a real shortcut in a temporary folder, the way an installer would.
    fn make_link(lnk: &Path, target: &Path) {
        unsafe {
            let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).unwrap();
            link.SetPath(&HSTRING::from(target)).unwrap();
            link.cast::<IPersistFile>()
                .unwrap()
                .Save(&HSTRING::from(lnk), true)
                .unwrap();
        }
    }

    #[test]
    fn a_shortcut_resolves_to_its_target() {
        let _com = crate::com::Com::sta();
        let dir = tempfile::tempdir().unwrap();
        let target = tempfile::Builder::new().suffix(".exe").tempfile_in(dir.path()).unwrap();
        let lnk = dir.path().join("App.lnk");
        make_link(&lnk, target.path());
        // The temp folder can be an 8.3 short path, so compare the file name.
        let name = target.path().file_name().unwrap().to_string_lossy().to_lowercase();
        let got = resolve(&lnk).unwrap();
        assert!(
            got.ends_with(&format!(r"\{name}")) && got == got.to_lowercase(),
            "{got}"
        );
        assert_eq!(resolve(&dir.path().join("missing.lnk")), None);
    }
}
