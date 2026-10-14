//! Packaged (MSIX / Store) apps installed for the user, and what their manifests make them do.

/// What a package manifest declares that changes how Windows behaves, in a fixed order:
/// `startup`, `context-menu`, `service`. Any namespace prefix on `Extension` counts.
pub fn manifest_extensions(xml: &str) -> Vec<&'static str> {
    let Ok(doc) = roxmltree::Document::parse(xml) else {
        return Vec::new();
    };
    let categories: Vec<&str> = doc
        .descendants()
        .filter(|n| n.tag_name().name() == "Extension")
        .filter_map(|n| n.attribute("Category"))
        .collect();
    [
        ("windows.startupTask", "startup"),
        ("windows.fileExplorerContextMenus", "context-menu"),
        ("windows.service", "service"),
    ]
    .into_iter()
    .filter(|(category, _)| categories.contains(category))
    .map(|(_, name)| name)
    .collect()
}

/// Errors from the package manager's COM server that mean it was busy or restarting (an install
/// still finishing, say), not that the list can't be read: CO_E_SERVER_STOPPING and
/// RPC_E_DISCONNECTED. Worth one more try.
pub fn is_retryable(hresult: i32) -> bool {
    matches!(hresult as u32, 0x8008_0008 | 0x8001_0108)
}

/// The user's packages. A busy package manager is asked once more after 2 seconds; if it
/// fails again, its error is kept.
#[cfg(windows)]
pub fn read() -> Result<crate::SourceOutput, String> {
    // WinRT needs the thread in the multithreaded apartment for the whole read (see com.rs).
    let _com = crate::com::Com::mta();
    match read_once() {
        Err(e) if is_retryable(e.code().0) => {
            std::thread::sleep(std::time::Duration::from_secs(2));
            read_once().map_err(|e| e.to_string())
        }
        result => result.map_err(|e| e.to_string()),
    }
}

#[cfg(windows)]
fn read_once() -> windows::core::Result<crate::SourceOutput> {
    use idiff_core::command::normalize_path;
    use idiff_core::model::Item;
    use std::collections::BTreeMap;
    use windows::Management::Deployment::PackageManager;
    use windows::core::HSTRING;

    let text = |s: windows::core::Result<HSTRING>| s.ok().map(|s| s.to_string()).filter(|s| !s.trim().is_empty());
    let manager = PackageManager::new()?;
    // An empty user SID means the current user.
    let packages = manager.FindPackagesByUserSecurityId(&HSTRING::new())?;

    // A family can have one package per architecture (frameworks): keep the first by full name.
    let mut by_family: BTreeMap<String, (String, Item)> = BTreeMap::new();
    for package in packages {
        let Ok(id) = package.Id() else { continue };
        let (Some(family), Some(full_name)) = (text(id.FamilyName()), text(id.FullName())) else {
            continue;
        };
        let location = text(package.InstalledPath());
        let mut props = BTreeMap::new();
        if let Ok(v) = id.Version() {
            props.insert(
                "version".into(),
                format!("{}.{}.{}.{}", v.Major, v.Minor, v.Build, v.Revision),
            );
        }
        if let Some(publisher) = text(package.PublisherDisplayName()).or_else(|| text(id.Publisher())) {
            props.insert("publisher".into(), publisher);
        }
        let manifest = location
            .as_ref()
            .and_then(|l| std::fs::read_to_string(format!(r"{l}\AppxManifest.xml")).ok());
        let extensions = manifest.map(|xml| manifest_extensions(&xml)).unwrap_or_default();
        if !extensions.is_empty() {
            props.insert("extensions".into(), extensions.join(","));
        }
        let item = Item {
            key: family.to_lowercase(),
            label: text(package.DisplayName())
                .or_else(|| text(id.Name()))
                .unwrap_or_else(|| family.clone()),
            target: location.map(|l| normalize_path(&l, crate::env())),
            props,
        };
        let slot = by_family
            .entry(item.key.clone())
            .or_insert_with(|| (full_name.clone(), item.clone()));
        if full_name < slot.0 {
            *slot = (full_name, item);
        }
    }
    let items = by_family.into_values().map(|(_, item)| item).collect();
    Ok(crate::SourceOutput { items, note: None })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_busy_package_manager_is_asked_again() {
        // CO_E_SERVER_STOPPING and RPC_E_DISCONNECTED: the COM server was busy or restarting.
        assert!(is_retryable(0x8008_0008_u32 as i32));
        assert!(is_retryable(0x8001_0108_u32 as i32));
        // Anything else (access denied, not found, class not registered) won't change on a retry.
        for code in [0x8007_0005_u32, 0x8007_0002, 0x8004_0154, 0] {
            assert!(!is_retryable(code as i32), "{code:#x}");
        }
    }

    #[test]
    fn manifest_startup_task_is_found() {
        let xml = include_str!("../../tests/fixtures/appx-startup.xml");
        assert_eq!(manifest_extensions(xml), ["startup"]);
    }

    #[test]
    fn manifest_extensions_ignore_the_namespace_prefix() {
        let xml = r#"<Package xmlns="urn:a" xmlns:desktop4="urn:d4" xmlns:desktop6="urn:d6">
            <Extensions>
              <desktop6:Extension Category="windows.service"><desktop6:Service Name="x" /></desktop6:Extension>
              <desktop4:Extension Category="windows.fileExplorerContextMenus" />
              <desktop4:Extension Category="windows.fileExplorerContextMenus" />
              <Extension Category="windows.startupTask" />
            </Extensions>
          </Package>"#;
        assert_eq!(manifest_extensions(xml), ["startup", "context-menu", "service"]);
        assert!(manifest_extensions("not xml").is_empty());
    }
}
