//! What idiff learns about the changes only, after the diff: who signed the file each changed
//! item runs, and how big each added folder is. Looking at changes alone keeps it cheap.

use crate::sources::folders::folder_size;
use idiff_core::diff::{Change, Delta};
use idiff_core::model::SourceKind;
use std::collections::HashMap;
use std::os::windows::io::AsRawHandle;
use std::path::Path;
use windows::Win32::Foundation::{HANDLE, HWND, INVALID_HANDLE_VALUE};
use windows::Win32::Security::Cryptography::Catalog::{
    CATALOG_INFO, CryptCATAdminAcquireContext2, CryptCATAdminCalcHashFromFileHandle2, CryptCATAdminEnumCatalogFromHash,
    CryptCATAdminReleaseCatalogContext, CryptCATAdminReleaseContext, CryptCATCatalogInfoFromContext,
};
use windows::Win32::Security::Cryptography::{CERT_NAME_SIMPLE_DISPLAY_TYPE, CertGetNameStringW};
use windows::Win32::Security::WinTrust::{
    WINTRUST_ACTION_GENERIC_VERIFY_V2, WINTRUST_DATA, WINTRUST_FILE_INFO, WTD_CACHE_ONLY_URL_RETRIEVAL,
    WTD_CHOICE_FILE, WTD_REVOCATION_CHECK_NONE, WTD_REVOKE_NONE, WTD_STATEACTION_CLOSE, WTD_STATEACTION_VERIFY,
    WTD_UI_NONE, WTHelperGetProvCertFromChain, WTHelperGetProvSignerFromChain, WTHelperProvDataFromStateData,
    WinVerifyTrust,
};
use windows::Win32::Storage::FileSystem::{GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW};
use windows::core::{HSTRING, PCWSTR, w};

/// The signer of Windows' own files, which carry no signature of their own but are listed in a
/// Windows catalog.
pub const WINDOWS_CATALOG_SIGNER: &str = "Microsoft Windows";

/// Who signed the file at `path`: the name on its own valid Authenticode signature, else
/// `Microsoft Windows` when a Windows catalog lists it, else None (unsigned, invalid, missing).
/// Nothing is fetched from the network: no revocation check, cached URLs only.
pub fn signer_of(path: &Path) -> Option<String> {
    if !path.is_file() {
        return None;
    }
    embedded_signer(path).or_else(|| catalog_signer(path))
}

/// The company the file at `path` names in its version details (`CompanyName`), trimmed. None
/// when it has no version details, names no company, or is missing. Only reads the file.
pub fn company_of(path: &Path) -> Option<String> {
    if !path.is_file() {
        return None;
    }
    let wide = HSTRING::from(path);
    let size = unsafe { GetFileVersionInfoSizeW(&wide, None) };
    if size == 0 {
        return None;
    }
    let mut data = vec![0u8; size as usize];
    unsafe { GetFileVersionInfoW(&wide, None, size, data.as_mut_ptr().cast()) }.ok()?;

    // The languages and code pages the strings are in, then the usual US English ones in case
    // the file doesn't list its own.
    let mut tables: Vec<(u16, u16)> = query(&data, r"\VarFileInfo\Translation")
        .map(|bytes| {
            bytes
                .as_chunks::<4>()
                .0
                .iter()
                .map(|[a, b, c, d]| (u16::from_le_bytes([*a, *b]), u16::from_le_bytes([*c, *d])))
                .collect()
        })
        .unwrap_or_default();
    tables.extend([(0x0409, 0x04b0), (0x0409, 0x04e4), (0x0409, 0x0000)]);
    tables.into_iter().find_map(|(lang, code_page)| {
        let bytes = query(
            &data,
            &format!(r"\StringFileInfo\{lang:04x}{code_page:04x}\CompanyName"),
        )?;
        let units: Vec<u16> = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&pair| u16::from_le_bytes(pair))
            .collect();
        let end = units.iter().position(|&u| u == 0).unwrap_or(units.len());
        let company = String::from_utf16_lossy(&units[..end]).trim().to_string();
        (!company.is_empty()).then_some(company)
    })
}

/// The bytes of one value in a version resource, or None when it isn't there. String values
/// come back as UTF-16 characters (`len` counts characters), everything else as bytes.
fn query<'a>(data: &'a [u8], path: &str) -> Option<&'a [u8]> {
    let mut ptr: *mut std::ffi::c_void = std::ptr::null_mut();
    let mut len = 0u32;
    let found = unsafe { VerQueryValueW(data.as_ptr().cast(), &HSTRING::from(path), &mut ptr, &mut len) }.as_bool();
    if !found || ptr.is_null() || len == 0 {
        return None;
    }
    let bytes = if path.starts_with(r"\StringFileInfo") {
        len as usize * 2
    } else {
        len as usize
    };
    let start = ptr as usize - data.as_ptr() as usize;
    data.get(start..start.checked_add(bytes)?)
}

/// Adds `signer` and `company` to the added and modified items whose target is a file, and
/// `bytes` and `files` to added folders. Each path is looked at once, however many items point at it.
pub fn enrich(delta: &mut Delta) {
    // Signer and company per target path, so each file is looked at once.
    let mut files: HashMap<String, (Option<String>, Option<String>)> = HashMap::new();
    for (kind, changes) in delta.sources.iter_mut() {
        for change in changes {
            let (item, added) = match change {
                Change::Added(item) => (item, true),
                Change::Modified { after, .. } => (after, false),
                Change::Removed(_) => continue,
            };
            if let Some(target) = &item.target {
                let (signer, company) = files.entry(target.clone()).or_insert_with(|| {
                    let path = Path::new(target);
                    (signer_of(path), company_of(path))
                });
                if let Some(signer) = signer {
                    item.props.insert("signer".into(), signer.clone());
                }
                if let Some(company) = company {
                    item.props.insert("company".into(), company.clone());
                }
            }
            // Folder keys are unique paths, so each added folder is measured once.
            if added && *kind == SourceKind::Folders && item.props.get("kind").map(String::as_str) == Some("folder") {
                let (bytes, files) = folder_size(Path::new(&item.key));
                item.props.insert("bytes".into(), bytes.to_string());
                item.props.insert("files".into(), files.to_string());
            }
        }
    }
}

/// Catalogs Windows installs for third-party driver packages are renamed `oem<n>.cat`. Their
/// files belong to whoever made the driver, not to Windows.
pub fn is_driver_package_catalog(catalog_path: &str) -> bool {
    let name = catalog_path.rsplit(['\\', '/']).next().unwrap_or(catalog_path);
    name.get(..3).is_some_and(|start| start.eq_ignore_ascii_case("oem"))
}

fn embedded_signer(path: &Path) -> Option<String> {
    let wide = HSTRING::from(path);
    let mut file = WINTRUST_FILE_INFO {
        cbStruct: size_of::<WINTRUST_FILE_INFO>() as u32,
        pcwszFilePath: PCWSTR(wide.as_ptr()),
        ..Default::default()
    };
    let mut data = WINTRUST_DATA {
        cbStruct: size_of::<WINTRUST_DATA>() as u32,
        dwUIChoice: WTD_UI_NONE,
        fdwRevocationChecks: WTD_REVOKE_NONE,
        dwUnionChoice: WTD_CHOICE_FILE,
        dwStateAction: WTD_STATEACTION_VERIFY,
        dwProvFlags: WTD_CACHE_ONLY_URL_RETRIEVAL | WTD_REVOCATION_CHECK_NONE,
        ..Default::default()
    };
    data.Anonymous.pFile = &mut file;
    let mut action = WINTRUST_ACTION_GENERIC_VERIFY_V2;
    let no_window = HWND(INVALID_HANDLE_VALUE.0);
    unsafe {
        let status = WinVerifyTrust(no_window, &mut action, (&mut data as *mut WINTRUST_DATA).cast());
        let signer = if status == 0 {
            signer_name(data.hWVTStateData)
        } else {
            None
        };
        data.dwStateAction = WTD_STATEACTION_CLOSE;
        WinVerifyTrust(no_window, &mut action, (&mut data as *mut WINTRUST_DATA).cast());
        signer
    }
}

/// The display name on the leaf certificate of the first signer WinVerifyTrust checked.
unsafe fn signer_name(state: HANDLE) -> Option<String> {
    unsafe {
        let provider = WTHelperProvDataFromStateData(state);
        if provider.is_null() {
            return None;
        }
        let signer = WTHelperGetProvSignerFromChain(provider, 0, false, 0);
        if signer.is_null() {
            return None;
        }
        let cert = WTHelperGetProvCertFromChain(signer, 0);
        let context = cert.as_ref()?.pCert;
        if context.is_null() {
            return None;
        }
        let mut buf = [0u16; 512];
        let n = CertGetNameStringW(context, CERT_NAME_SIMPLE_DISPLAY_TYPE, 0, None, Some(&mut buf)) as usize;
        let name = String::from_utf16_lossy(&buf[..n.saturating_sub(1)]);
        (!name.trim().is_empty()).then_some(name)
    }
}

/// `Microsoft Windows` when a Windows catalog lists the file's hash. Catalogs hold SHA-256
/// hashes on current Windows and SHA-1 on older ones, so both are tried.
fn catalog_signer(path: &Path) -> Option<String> {
    for algorithm in [w!("SHA256"), PCWSTR::null()] {
        // A fresh handle per hash, so the second one reads the file from the start.
        let Ok(file) = std::fs::File::open(path) else {
            return None;
        };
        let handle = HANDLE(file.as_raw_handle());
        unsafe {
            let mut admin = 0isize;
            if CryptCATAdminAcquireContext2(&mut admin, None, algorithm, None, None).is_err() {
                continue;
            }
            let listed = in_windows_catalog(admin, handle);
            let _ = CryptCATAdminReleaseContext(admin, 0);
            if listed {
                return Some(WINDOWS_CATALOG_SIGNER.to_string());
            }
        }
    }
    None
}

unsafe fn in_windows_catalog(admin: isize, file: HANDLE) -> bool {
    unsafe {
        let mut size = 0u32;
        // The first call only says how big the hash is.
        let _ = CryptCATAdminCalcHashFromFileHandle2(admin, file, &mut size, None, None);
        if size == 0 {
            return false;
        }
        let mut hash = vec![0u8; size as usize];
        if CryptCATAdminCalcHashFromFileHandle2(admin, file, &mut size, Some(hash.as_mut_ptr()), None).is_err() {
            return false;
        }
        let mut catalog = CryptCATAdminEnumCatalogFromHash(admin, &hash, None, None);
        while catalog != 0 {
            let mut info = CATALOG_INFO {
                cbStruct: size_of::<CATALOG_INFO>() as u32,
                ..Default::default()
            };
            if CryptCATCatalogInfoFromContext(catalog, &mut info, 0).is_ok() {
                let len = info
                    .wszCatalogFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(info.wszCatalogFile.len());
                if !is_driver_package_catalog(&String::from_utf16_lossy(&info.wszCatalogFile[..len])) {
                    let _ = CryptCATAdminReleaseCatalogContext(admin, catalog, 0);
                    return true;
                }
            }
            // Asking for the next catalog releases this one.
            let mut previous = catalog;
            catalog = CryptCATAdminEnumCatalogFromHash(admin, &hash, None, Some(&mut previous));
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn driver_package_catalogs_are_not_windows() {
        assert!(is_driver_package_catalog(
            r"C:\Windows\System32\CatRoot\{F750E6C3}\oem12.cat"
        ));
        assert!(is_driver_package_catalog("OEM3.CAT"));
        assert!(!is_driver_package_catalog(
            r"C:\Windows\System32\CatRoot\{F750E6C3}\Microsoft-Windows-Client-Features-Package.cat"
        ));
        assert!(!is_driver_package_catalog(""));
    }
}
