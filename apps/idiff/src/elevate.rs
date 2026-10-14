//! The elevated helper. Scheduled tasks and Defender exclusions are only complete when an
//! administrator reads them, so an unelevated idiff starts a second copy of itself with one UAC
//! prompt. That copy reads only those sources and writes them to a file in idiff's home.
//!
//! Everything about the user (their programs, startup, folders) is still read by the unelevated
//! idiff: when someone else's administrator password approves UAC, the elevated copy runs as
//! that administrator and would see their profile instead.

use idiff_core::model::{Snapshot, SourceKind};
use std::path::{Component, Path, PathBuf};

/// Where the helper writes its snapshot: a name nobody else can guess.
pub fn helper_path(home: &Path, pid: u32, nonce: u128) -> PathBuf {
    home.join(format!("helper-{pid}-{nonce:032x}.json"))
}

/// `--helper <path> --only <Kind,Kind>` and nothing else. The path must be absolute, without
/// `..`, and named like `helper_path` names it; the kinds must be ones that need an administrator.
pub fn parse_helper_args(args: &[String]) -> Option<(PathBuf, Vec<SourceKind>)> {
    let [flag, path, only_flag, only] = args else {
        return None;
    };
    if flag != "--helper" || only_flag != "--only" {
        return None;
    }
    let path = PathBuf::from(path);
    let plain = path.is_absolute() && !path.components().any(|c| c == Component::ParentDir);
    if !plain
        || !path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(is_helper_file_name)
    {
        return None;
    }
    let kinds = only
        .split(',')
        .map(|name| {
            SourceKind::ALL
                .into_iter()
                .find(|k| format!("{k:?}") == name && k.needs_admin())
        })
        .collect::<Option<Vec<_>>>()?;
    Some((path, kinds))
}

/// `helper-<digits>-<32 lowercase hex>.json`.
fn is_helper_file_name(name: &str) -> bool {
    let Some(rest) = name.strip_prefix("helper-").and_then(|r| r.strip_suffix(".json")) else {
        return false;
    };
    let Some((pid, nonce)) = rest.split_once('-') else {
        return false;
    };
    !pid.is_empty()
        && pid.bytes().all(|b| b.is_ascii_digit())
        && nonce.len() == 32
        && nonce.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// Puts the helper's results into the unelevated snapshot: only sources that need an
/// administrator, and only those the helper read without an error.
pub fn merge_helper(full: &mut Snapshot, helper: Snapshot) {
    for (kind, snap) in helper.sources {
        if kind.needs_admin() && snap.ok {
            full.sources.insert(kind, snap);
        }
    }
}

#[cfg(windows)]
pub use windows_impl::{run_helper, snapshot_for_run};

#[cfg(windows)]
mod windows_impl {
    use super::{helper_path, merge_helper};
    use crate::run::quote_args;
    use crate::session;
    use idiff_collect::{SnapshotOptions, snapshot};
    use idiff_core::model::{Snapshot, SourceKind};
    use std::fs::OpenOptions;
    use std::hash::{BuildHasher, RandomState};
    use std::io::Write;
    use std::path::Path;
    use std::sync::atomic::{AtomicBool, Ordering};
    use windows::Win32::Foundation::{CloseHandle, ERROR_CANCELLED, WAIT_OBJECT_0};
    use windows::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject};
    use windows::Win32::UI::Shell::{SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW};
    use windows::core::{HSTRING, PCWSTR, w};

    /// Set once the user says no to UAC, so the after snapshot of the same run doesn't ask again.
    static DECLINED: AtomicBool = AtomicBool::new(false);

    /// How long the helper may take, UAC prompt included.
    const HELPER_TIMEOUT_MS: u32 = 5 * 60 * 1000;

    /// The helper's whole job: read `only` and write it to `path`, which must not exist yet.
    /// Returns the process exit code.
    pub fn run_helper(path: &Path, only: &[SourceKind]) -> i32 {
        // Refuse a file that's already there before reading anything.
        let Ok(mut file) = OpenOptions::new().write(true).create_new(true).open(path) else {
            return 1;
        };
        let snap = snapshot(&SnapshotOptions {
            only: Some(only.to_vec()),
            ..Default::default()
        });
        match file.write_all(snap.to_json().as_bytes()) {
            Ok(()) => 0,
            Err(_) => 1,
        }
    }

    /// The snapshot `run` takes before and after the install. Unelevated, and unless `no_elevate`,
    /// it asks once through UAC for the sources that need an administrator. If the user declines
    /// or the helper fails, those sources keep their unelevated results, which say why.
    pub fn snapshot_for_run(no_elevate: bool) -> Snapshot {
        let mut full = snapshot(&SnapshotOptions::default());
        if full.elevated || no_elevate || DECLINED.load(Ordering::Relaxed) {
            return full;
        }
        let kinds: Vec<SourceKind> = SourceKind::ALL.into_iter().filter(|k| k.needs_admin()).collect();
        match elevated_snapshot(&kinds) {
            Ok(helper) => merge_helper(&mut full, helper),
            Err(Helper::Declined) => DECLINED.store(true, Ordering::Relaxed),
            Err(Helper::Failed) => {}
        }
        full
    }

    enum Helper {
        Declined,
        Failed,
    }

    /// Starts this exe elevated as the helper, waits for it and reads what it wrote.
    fn elevated_snapshot(kinds: &[SourceKind]) -> Result<Snapshot, Helper> {
        let home = session::home();
        std::fs::create_dir_all(&home).map_err(|_| Helper::Failed)?;
        let path = helper_path(&home, std::process::id(), nonce());
        let exe = std::env::current_exe().map_err(|_| Helper::Failed)?;
        let only = kinds.iter().map(|k| format!("{k:?}")).collect::<Vec<_>>().join(",");
        let params = quote_args(&["--helper".into(), path.display().to_string(), "--only".into(), only]);
        let (file, params) = (HSTRING::from(exe.as_path()), HSTRING::from(params));
        let mut info = SHELLEXECUTEINFOW {
            cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC,
            lpVerb: w!("runas"),
            lpFile: PCWSTR(file.as_ptr()),
            lpParameters: PCWSTR(params.as_ptr()),
            nShow: 0, // SW_HIDE: the helper prints nothing
            ..Default::default()
        };
        unsafe {
            if let Err(e) = ShellExecuteExW(&mut info) {
                return Err(if e.code() == ERROR_CANCELLED.to_hresult() {
                    Helper::Declined
                } else {
                    Helper::Failed
                });
            }
            let finished = WaitForSingleObject(info.hProcess, HELPER_TIMEOUT_MS) == WAIT_OBJECT_0;
            let mut code = 1u32;
            let _ = GetExitCodeProcess(info.hProcess, &mut code);
            let _ = CloseHandle(info.hProcess);
            let text = std::fs::read_to_string(&path);
            let _ = std::fs::remove_file(&path);
            if !finished || code != 0 {
                return Err(Helper::Failed);
            }
            text.ok().and_then(|t| Snapshot::load(&t).ok()).ok_or(Helper::Failed)
        }
    }

    /// 128 random bits from the standard library's per-process hash keys.
    fn nonce() -> u128 {
        let half = || u128::from(RandomState::new().hash_one(std::time::SystemTime::now()));
        (half() << 64) | half()
    }
}
