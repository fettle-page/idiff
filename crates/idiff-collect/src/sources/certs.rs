//! Trusted root certificates: the machine's and the user's Root stores.

use idiff_core::model::Item;
use std::collections::BTreeMap;

/// A FILETIME (100-nanosecond ticks since 1601-01-01 UTC) as `YYYY-MM-DD`.
pub fn filetime_to_date(filetime: u64) -> String {
    // Days since 1601-01-01, shifted to days since 0000-03-01 for the civil calendar formula.
    let days = (filetime / 10_000_000 / 86_400) as i64 + 584_694;
    let era = days.div_euclid(146_097);
    let day_of_era = days.rem_euclid(146_097);
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

/// One certificate in the machine's (`lm`) or the user's (`cu`) Root store.
pub fn cert_item(store: &str, sha1: &[u8], subject: &str, issuer: &str, not_after: u64) -> Item {
    let thumbprint: String = sha1.iter().map(|b| format!("{b:02x}")).collect();
    Item {
        key: format!("{store}|{thumbprint}"),
        label: subject.to_string(),
        target: None,
        props: BTreeMap::from([
            ("issuer".to_string(), issuer.to_string()),
            ("not_after".to_string(), filetime_to_date(not_after)),
        ]),
    }
}

#[cfg(windows)]
pub use reader::read;

#[cfg(windows)]
mod reader {
    use super::cert_item;
    use crate::SourceOutput;
    use idiff_core::model::Item;
    use std::collections::BTreeSet;
    use windows::Win32::Security::Cryptography::{
        CERT_CONTEXT, CERT_NAME_ISSUER_FLAG, CERT_NAME_SIMPLE_DISPLAY_TYPE, CERT_OPEN_STORE_FLAGS,
        CERT_QUERY_ENCODING_TYPE, CERT_SHA1_HASH_PROP_ID, CERT_STORE_OPEN_EXISTING_FLAG, CERT_STORE_PROV_SYSTEM_W,
        CERT_STORE_READONLY_FLAG, CERT_SYSTEM_STORE_CURRENT_USER, CERT_SYSTEM_STORE_LOCAL_MACHINE, CertCloseStore,
        CertEnumCertificatesInStore, CertGetCertificateContextProperty, CertGetNameStringW, CertOpenStore,
    };
    use windows::core::w;

    /// Both Root stores. The user's store also shows the machine's certificates, so only the
    /// ones that are the user's own are listed under `cu`.
    pub fn read() -> Result<SourceOutput, String> {
        let machine = store_items("lm", CERT_SYSTEM_STORE_LOCAL_MACHINE)?;
        let thumbprints: BTreeSet<&str> = machine
            .iter()
            .filter_map(|i| i.key.split_once('|'))
            .map(|(_, t)| t)
            .collect();
        let user: Vec<Item> = store_items("cu", CERT_SYSTEM_STORE_CURRENT_USER)?
            .into_iter()
            .filter(|i| i.key.split_once('|').is_none_or(|(_, t)| !thumbprints.contains(t)))
            .collect();
        let mut items: Vec<Item> = machine.into_iter().chain(user).collect();
        items.sort_by(|a, b| a.key.cmp(&b.key));
        items.dedup_by(|a, b| a.key == b.key);
        Ok(SourceOutput { items, note: None })
    }

    fn store_items(store: &str, location: u32) -> Result<Vec<Item>, String> {
        let flags = CERT_OPEN_STORE_FLAGS(location | CERT_STORE_READONLY_FLAG.0 | CERT_STORE_OPEN_EXISTING_FLAG.0);
        let mut items = Vec::new();
        unsafe {
            let handle = CertOpenStore(
                CERT_STORE_PROV_SYSTEM_W,
                CERT_QUERY_ENCODING_TYPE(0),
                None,
                flags,
                Some(w!("ROOT").as_ptr() as *const _),
            )
            .map_err(|e| format!("{store} Root store: {e}"))?;
            let mut cert = CertEnumCertificatesInStore(handle, None);
            while !cert.is_null() {
                if let Some(item) = cert_to_item(store, &*cert) {
                    items.push(item);
                }
                cert = CertEnumCertificatesInStore(handle, Some(cert));
            }
            let _ = CertCloseStore(Some(handle), 0);
        }
        Ok(items)
    }

    unsafe fn cert_to_item(store: &str, cert: &CERT_CONTEXT) -> Option<Item> {
        unsafe {
            let mut sha1 = [0u8; 20];
            let mut len = sha1.len() as u32;
            CertGetCertificateContextProperty(cert, CERT_SHA1_HASH_PROP_ID, Some(sha1.as_mut_ptr().cast()), &mut len)
                .ok()?;
            let name = |flags: u32| {
                let mut buf = [0u16; 512];
                let n = CertGetNameStringW(cert, CERT_NAME_SIMPLE_DISPLAY_TYPE, flags, None, Some(&mut buf)) as usize;
                String::from_utf16_lossy(&buf[..n.saturating_sub(1)])
            };
            let not_after = cert.pCertInfo.as_ref()?.NotAfter;
            let not_after = (u64::from(not_after.dwHighDateTime) << 32) | u64::from(not_after.dwLowDateTime);
            Some(cert_item(
                store,
                &sha1[..len as usize],
                &name(0),
                &name(CERT_NAME_ISSUER_FLAG),
                not_after,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filetimes_become_dates() {
        assert_eq!(filetime_to_date(0), "1601-01-01");
        assert_eq!(filetime_to_date(125_596_719_000_000_000), "1999-01-01");
        assert_eq!(filetime_to_date(134_364_591_000_000_000), "2026-10-14");
        assert_eq!(filetime_to_date(138_534_255_000_000_000), "2039-12-31");
    }

    #[test]
    fn cert_items_are_keyed_by_store_and_thumbprint() {
        let item = cert_item(
            "lm",
            &[0xAB, 0x01, 0xff],
            "Example Root CA",
            "Example Root CA",
            134_364_591_000_000_000,
        );
        assert_eq!(item.key, "lm|ab01ff");
        assert_eq!(item.label, "Example Root CA");
        assert_eq!(item.target, None);
        assert_eq!(item.props["issuer"], "Example Root CA");
        assert_eq!(item.props["not_after"], "2026-10-14");
    }
}
