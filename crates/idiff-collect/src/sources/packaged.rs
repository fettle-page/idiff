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

#[cfg(windows)]
pub fn read() -> Result<crate::SourceOutput, String> {
    use idiff_core::command::normalize_path;
    use idiff_core::model::Item;
    use std::collections::BTreeMap;
    use windows::Management::Deployment::PackageManager;
    use windows::core::HSTRING;

    let text = |s: windows::core::Result<HSTRING>| s.ok().map(|s| s.to_string()).filter(|s| !s.trim().is_empty());
    let manager = PackageManager::new().map_err(|e| e.to_string())?;
    // An empty user SID means the current user.
    let packages = manager
        .FindPackagesByUserSecurityId(&HSTRING::new())
        .map_err(|e| e.to_string())?;

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
