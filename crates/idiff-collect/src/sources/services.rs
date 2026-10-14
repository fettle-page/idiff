//! Services and drivers: both live under `HKLM\SYSTEM\CurrentControlSet\Services`, told apart by `Type`.

use idiff_core::command::{extract_exe, normalize_path};
use idiff_core::model::{Item, SourceKind};
use std::collections::{BTreeMap, BTreeSet};

/// Which source a service key belongs to, by its `Type` value.
pub fn kind_of(service_type: u32) -> Option<SourceKind> {
    if service_type & 0x30 != 0 {
        Some(SourceKind::Services)
    } else if service_type & 0x3 != 0 {
        Some(SourceKind::Drivers)
    } else {
        None
    }
}

/// Per-user services (`CDPUserSvc_1a2b3`) get a new random suffix at every sign-in. Only the
/// template they're made from (`Type & 0x40`) says anything about what was installed.
pub fn is_user_instance(name: &str, template_names: &BTreeSet<String>) -> bool {
    let Some((template, suffix)) = name.rsplit_once('_') else {
        return false;
    };
    (4..=8).contains(&suffix.len())
        && suffix.bytes().all(|b| b.is_ascii_hexdigit())
        && template_names.contains(&template.to_lowercase())
}

pub fn service_item(name: &str, values: &BTreeMap<String, String>) -> Item {
    let value = |v: &str| values.get(v).map(|s| s.trim()).filter(|s| !s.is_empty());
    // "@%SystemRoot%\...dll,-101" is a resource reference, not a name anyone would recognise.
    let label = value("DisplayName").filter(|d| !d.starts_with('@')).unwrap_or(name);
    let target = value("ImagePath").and_then(|p| {
        extract_exe(p, crate::env())
            .or_else(|| Some(normalize_path(p, crate::env())))
            .filter(|t| !t.is_empty())
    });

    let mut props = BTreeMap::new();
    for (prop, from) in [("start", "Start"), ("type", "Type")] {
        if let Some(v) = value(from) {
            props.insert(prop.to_string(), v.to_string());
        }
    }
    // Windows runs a service without ObjectName as LocalSystem.
    props.insert(
        "account".into(),
        value("ObjectName").unwrap_or("LocalSystem").to_string(),
    );

    Item {
        key: name.to_lowercase(),
        label: label.to_string(),
        target,
        props,
    }
}

#[cfg(windows)]
pub use reader::{read_drivers, read_services};

#[cfg(windows)]
mod reader {
    use super::{is_user_instance, kind_of, service_item};
    use crate::SourceOutput;
    use crate::reg::{open_error, values};
    use idiff_core::model::SourceKind;
    use std::collections::BTreeSet;
    use winreg::RegKey;
    use winreg::enums::HKEY_LOCAL_MACHINE;

    const SERVICES: &str = r"SYSTEM\CurrentControlSet\Services";

    pub fn read_services() -> Result<SourceOutput, String> {
        read(SourceKind::Services)
    }

    pub fn read_drivers() -> Result<SourceOutput, String> {
        read(SourceKind::Drivers)
    }

    /// Each call reads the registry afresh. A pass shared across calls would hand the snapshot
    /// after an install the services from before it.
    fn read(kind: SourceKind) -> Result<SourceOutput, String> {
        let root = RegKey::predef(HKEY_LOCAL_MACHINE)
            .open_subkey(SERVICES)
            .map_err(|e| format!(r"HKLM\{SERVICES}: {e}"))?;
        let mut keys = Vec::new();
        let mut errors = Vec::new();
        for name in root.enum_keys().flatten() {
            match root.open_subkey(&name) {
                Ok(key) => {
                    let values = values(&key);
                    let service_type = values.get("Type").and_then(|t| t.parse::<u32>().ok());
                    keys.push((name, values, service_type));
                }
                Err(e) => errors.extend(open_error(&e).map(|error| format!("{name}: {error}"))),
            }
        }
        let templates: BTreeSet<String> = keys
            .iter()
            .filter(|(_, _, t)| t.is_some_and(|t| t & 0x40 != 0))
            .map(|(name, _, _)| name.to_lowercase())
            .collect();
        let mut items: Vec<_> = keys
            .iter()
            .filter(|(name, _, t)| t.and_then(kind_of) == Some(kind) && !is_user_instance(name, &templates))
            .map(|(name, values, _)| service_item(name, values))
            .collect();
        items.sort_by(|a, b| a.key.cmp(&b.key));
        let note = (!errors.is_empty()).then(|| format!(r"Couldn't read HKLM\{SERVICES}\{}", errors.join("; ")));
        Ok(SourceOutput { items, note })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use idiff_core::model::SourceKind;
    use std::collections::{BTreeMap, BTreeSet};

    fn values(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn kind_of_maps_types() {
        for t in [0x10, 0x20, 0x110, 0x50, 0x60] {
            assert_eq!(kind_of(t), Some(SourceKind::Services), "{t:#x}");
        }
        for t in [0x1, 0x2] {
            assert_eq!(kind_of(t), Some(SourceKind::Drivers), "{t:#x}");
        }
        for t in [0x0, 0x4, 0x8] {
            assert_eq!(kind_of(t), None, "{t:#x}");
        }
    }

    #[test]
    fn user_instances_are_recognised() {
        let templates = BTreeSet::from(["cdpusersvc".to_string()]);
        assert!(is_user_instance("CDPUserSvc_1a2b3", &templates));
        assert!(is_user_instance("cdpusersvc_1A2B3C4D", &templates));
        assert!(!is_user_instance("Steam Client Service", &templates));
        assert!(!is_user_instance("foo_zz12", &templates));
        assert!(!is_user_instance("CDPUserSvc_123", &templates));
        assert!(!is_user_instance("CDPUserSvc_123456789", &templates));
        assert!(!is_user_instance("OtherSvc_1a2b3", &templates));
    }

    #[test]
    fn resource_display_names_fall_back_to_the_name() {
        let item = service_item(
            "AarSvc",
            &values(&[("DisplayName", "@%SystemRoot%\\system32\\AarSvc.dll,-101")]),
        );
        assert_eq!(item.label, "AarSvc");
        let item = service_item("Foo", &values(&[("DisplayName", "  ")]));
        assert_eq!(item.label, "Foo");
        let item = service_item(
            "Steam Client Service",
            &values(&[("DisplayName", "Steam Client Service")]),
        );
        assert_eq!(item.label, "Steam Client Service");
    }

    #[test]
    fn service_item_maps_key_target_and_props() {
        let item = service_item(
            "Steam Client Service",
            &values(&[
                (
                    "ImagePath",
                    r#""C:\Program Files (x86)\Common Files\Steam\steamservice.exe" /RunAsService"#,
                ),
                ("Start", "3"),
                ("Type", "16"),
                ("Description", "ignored"),
            ]),
        );
        assert_eq!(item.key, "steam client service");
        assert_eq!(
            item.target.as_deref(),
            Some(r"c:\program files (x86)\common files\steam\steamservice.exe")
        );
        assert_eq!(
            item.props,
            values(&[("account", "LocalSystem"), ("start", "3"), ("type", "16")])
        );

        let svc = service_item(
            "Dnscache",
            &values(&[
                ("ImagePath", r"%SystemRoot%\system32\svchost.exe -k NetworkService -p"),
                ("ObjectName", r"NT AUTHORITY\NetworkService"),
            ]),
        );
        assert_eq!(svc.props["account"], r"NT AUTHORITY\NetworkService");
        assert!(svc.target.unwrap().ends_with(r"\system32\svchost.exe"));

        let driver = service_item(
            "foo",
            &values(&[("ImagePath", r"\SystemRoot\System32\drivers\foo.sys")]),
        );
        assert!(driver.target.unwrap().ends_with(r"\system32\drivers\foo.sys"));

        assert_eq!(service_item("bare", &values(&[])).target, None);
    }
}
