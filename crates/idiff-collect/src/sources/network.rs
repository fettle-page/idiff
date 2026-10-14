//! Network settings an installer can change for every program: the hosts file, the user's proxy
//! and the Winsock catalog's provider DLLs.

use idiff_core::command::normalize_path;
use idiff_core::model::Item;
use std::collections::BTreeMap;

/// One item per host name on each hosts line; `#` starts a comment.
pub fn hosts_items(text: &str) -> Vec<Item> {
    let mut items = Vec::new();
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("");
        let mut words = line.split_whitespace();
        let Some(ip) = words.next() else { continue };
        for host in words {
            let label = format!("{ip} {host}");
            items.push(Item {
                key: format!("hosts|{}", label.to_lowercase()),
                label,
                target: None,
                props: BTreeMap::new(),
            });
        }
    }
    items
}

/// A Winsock catalog entry's provider DLL: its PackedCatalogItem starts with the DLL path as a
/// null-terminated ANSI string in a 260-byte field.
pub fn winsock_dll(packed: &[u8]) -> Option<String> {
    let field = &packed[..packed.len().min(260)];
    let end = field.iter().position(|&b| b == 0)?;
    let path = match std::str::from_utf8(&field[..end]) {
        Ok(path) => path.to_string(),
        // Not UTF-8: read the bytes as Latin-1, so a non-ASCII path still gives a stable key.
        Err(_) => field[..end].iter().map(|&b| b as char).collect(),
    };
    let path = normalize_path(&path, crate::env());
    (!path.is_empty()).then_some(path)
}

/// The user's proxy settings (Internet Settings): whether it's on, the server, and the
/// auto-config script. Only the values that are set become items.
pub fn proxy_items(enable: Option<u32>, server: Option<&str>, pac: Option<&str>) -> Vec<Item> {
    let enable = enable.map(|e| e.to_string());
    [
        ("proxy|enabled", "ProxyEnable", enable.as_deref()),
        ("proxy|server", "ProxyServer", server),
        ("proxy|pac", "AutoConfigURL", pac),
    ]
    .into_iter()
    .filter_map(|(key, label, value)| {
        let value = value.map(str::trim).filter(|v| !v.is_empty())?;
        Some(Item {
            key: key.to_string(),
            label: label.to_string(),
            target: None,
            props: BTreeMap::from([("value".to_string(), value.to_string())]),
        })
    })
    .collect()
}

#[cfg(windows)]
pub use reader::read;

#[cfg(windows)]
mod reader {
    use super::{hosts_items, proxy_items, winsock_dll};
    use crate::SourceOutput;
    use crate::reg::open_error;
    use idiff_core::model::Item;
    use std::collections::BTreeMap;
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

    const INTERNET_SETTINGS: &str = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";
    const CATALOG: &str = r"SYSTEM\CurrentControlSet\Services\WinSock2\Parameters\Protocol_Catalog9\Catalog_Entries64";

    /// All three parts. One that can't be read makes the result partial; all three, an error.
    pub fn read() -> Result<SourceOutput, String> {
        let mut items = Vec::new();
        let mut errors = Vec::new();
        let mut read_ok = 0;
        for part in [hosts(), proxy(), winsock()] {
            match part {
                Ok(part) => {
                    items.extend(part);
                    read_ok += 1;
                }
                Err(e) => errors.push(e),
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

    fn hosts() -> Result<Vec<Item>, String> {
        let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
        let path = format!(r"{root}\System32\drivers\etc\hosts");
        match std::fs::read(&path) {
            Ok(bytes) => Ok(hosts_items(&String::from_utf8_lossy(&bytes))),
            Err(e) => open_error(&e).map_or(Ok(Vec::new()), |error| Err(format!("{path}: {error}"))),
        }
    }

    fn proxy() -> Result<Vec<Item>, String> {
        let key = match RegKey::predef(HKEY_CURRENT_USER).open_subkey(INTERNET_SETTINGS) {
            Ok(key) => key,
            Err(e) => {
                return open_error(&e).map_or(Ok(Vec::new()), |error| {
                    Err(format!(r"HKCU\{INTERNET_SETTINGS}: {error}"))
                });
            }
        };
        let enable = key.get_value::<u32, _>("ProxyEnable").ok();
        let server = key.get_value::<String, _>("ProxyServer").ok();
        let pac = key.get_value::<String, _>("AutoConfigURL").ok();
        Ok(proxy_items(enable, server.as_deref(), pac.as_deref()))
    }

    /// One item per provider DLL, however many catalog entries use it.
    fn winsock() -> Result<Vec<Item>, String> {
        let catalog = RegKey::predef(HKEY_LOCAL_MACHINE)
            .open_subkey(CATALOG)
            .map_err(|e| format!(r"HKLM\{CATALOG}: {e}"))?;
        let mut dlls = BTreeMap::new();
        for entry in catalog.enum_keys().flatten() {
            let packed = catalog
                .open_subkey(&entry)
                .and_then(|k| k.get_raw_value("PackedCatalogItem"));
            if let Some(dll) = packed.ok().and_then(|v| winsock_dll(&v.bytes)) {
                *dlls.entry(dll).or_insert(0u32) += 1;
            }
        }
        Ok(dlls
            .into_iter()
            .map(|(dll, entries)| Item {
                key: format!("winsock|{dll}"),
                label: dll.clone(),
                target: Some(dll),
                props: BTreeMap::from([("entries".to_string(), entries.to_string())]),
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosts_comments_and_blank_lines_are_ignored() {
        let items = hosts_items("# comment\r\n\r\n   \n127.0.0.1 a b # c\n0.0.0.0\tAds.Example.com\n::1\n");
        let keys: Vec<_> = items.iter().map(|i| i.key.as_str()).collect();
        assert_eq!(
            keys,
            [
                "hosts|127.0.0.1 a",
                "hosts|127.0.0.1 b",
                "hosts|0.0.0.0 ads.example.com"
            ]
        );
        assert_eq!(items[2].label, "0.0.0.0 Ads.Example.com");
    }

    #[test]
    fn winsock_dll_is_read_from_packed_item() {
        let mut packed = br"%SystemRoot%\system32\mswsock.dll".to_vec();
        packed.resize(260, 0);
        packed.extend([0xaa; 100]);
        let dll = winsock_dll(&packed).unwrap();
        assert!(dll.ends_with(r"\system32\mswsock.dll") && !dll.contains('%'), "{dll}");
        assert_eq!(winsock_dll(&[0; 300]), None);
        assert_eq!(winsock_dll(&[b'a'; 300]), None);
    }

    #[test]
    fn proxy_values_become_items() {
        let items = proxy_items(Some(1), Some("127.0.0.1:8080"), None);
        let got: Vec<_> = items
            .iter()
            .map(|i| (i.key.as_str(), i.props["value"].as_str()))
            .collect();
        assert_eq!(got, [("proxy|enabled", "1"), ("proxy|server", "127.0.0.1:8080")]);
        assert_eq!(
            proxy_items(None, Some(""), Some("http://wpad/x.pac"))[0].key,
            "proxy|pac"
        );
    }
}
