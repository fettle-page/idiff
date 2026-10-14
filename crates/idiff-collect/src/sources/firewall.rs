//! Windows Firewall rules, as the firewall service stores them in the registry.

use idiff_core::command::normalize_path;
use idiff_core::model::Item;

/// The rule fields kept as props, by their name in the rule string.
const PROPS: &[(&str, &str)] = &[
    ("Action", "action"),
    ("Dir", "dir"),
    ("Active", "active"),
    ("Protocol", "protocol"),
    ("LPort", "lport"),
    ("Svc", "svc"),
    // The package a Store app's rule is for; those rules have no App.
    ("PFN", "pfn"),
];

/// One rule. The data is `v2.33|Field=value|Field=value|…`; a field that appears more than once
/// (remote addresses, for example) keeps its first value.
pub fn parse_rule(value_name: &str, data: &str) -> Item {
    let fields: Vec<(&str, &str)> = data.split('|').filter_map(|f| f.split_once('=')).collect();
    let field = |name: &str| {
        fields
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.trim())
    };

    let props = PROPS
        .iter()
        .filter_map(|(from, prop)| Some((prop.to_string(), field(from)?.to_lowercase())))
        .collect();
    Item {
        key: value_name.to_lowercase(),
        label: field("Name")
            .filter(|n| !n.is_empty())
            .unwrap_or(value_name)
            .to_string(),
        target: field("App")
            .filter(|a| !a.is_empty())
            .map(|a| normalize_path(a, crate::env())),
        props,
    }
}

#[cfg(windows)]
pub fn read() -> Result<crate::SourceOutput, String> {
    use winreg::RegKey;
    use winreg::enums::HKEY_LOCAL_MACHINE;

    const RULES: &str = r"SYSTEM\CurrentControlSet\Services\SharedAccess\Parameters\FirewallPolicy\FirewallRules";
    let key = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(RULES)
        .map_err(|e| format!(r"HKLM\{RULES}: {e}"))?;
    let mut items: Vec<Item> = crate::reg::values(&key)
        .iter()
        .map(|(name, data)| parse_rule(name, data))
        .collect();
    items.sort_by(|a, b| a.key.cmp(&b.key));
    Ok(crate::SourceOutput { items, note: None })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn firewall_rule_fields_are_parsed() {
        let item = parse_rule(
            "{8A2B-Steam}",
            r"v2.33|Action=Allow|Active=TRUE|Dir=In|Protocol=6|LPort=27036|App=C:\Program Files (x86)\Steam\steam.exe|Name=Steam|Desc=Steam|",
        );
        assert_eq!(item.key, "{8a2b-steam}");
        assert_eq!(item.label, "Steam");
        assert_eq!(item.target.as_deref(), Some(r"c:\program files (x86)\steam\steam.exe"));
        let props: Vec<_> = item.props.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        assert_eq!(
            props,
            [
                ("action", "allow"),
                ("active", "true"),
                ("dir", "in"),
                ("lport", "27036"),
                ("protocol", "6")
            ]
        );

        let service = parse_rule(
            "CoreNet-DHCP-In",
            r"v2.10|Action=Allow|Dir=In|Svc=dhcp|App=%SystemRoot%\system32\svchost.exe|",
        );
        assert_eq!(service.label, "CoreNet-DHCP-In");
        assert_eq!(service.props["svc"], "dhcp");
        assert!(
            service
                .target
                .as_deref()
                .is_some_and(|t| t.ends_with(r"\system32\svchost.exe"))
        );
        assert_eq!(parse_rule("x", "v2.10|Action=Block|Dir=Out|").target, None);

        // Rules for Store apps name the package instead of a program.
        let store = parse_rule(
            "{78E1CD88-49E3-476E-B926-580E596AD309}",
            "v2.31|Action=Allow|Active=TRUE|Dir=In|PFN=Microsoft.WindowsStore_8wekyb3d8bbwe|Name=@{Microsoft.WindowsStore}|",
        );
        assert_eq!(store.target, None);
        assert_eq!(store.props["pfn"], "microsoft.windowsstore_8wekyb3d8bbwe");
    }

    #[test]
    fn store_app_rules_keep_their_package_family() {
        let item = parse_rule(
            "5319275A.WhatsAppDesktop-Out-Allow-AllCapabilities",
            "v2.33|Action=Allow|Active=TRUE|Dir=Out|PFN=5319275A.WhatsAppDesktop_cv1g1gvanyjgm|Name=WhatsApp|",
        );
        assert_eq!(item.target, None);
        assert_eq!(item.props["pfn"], "5319275a.whatsappdesktop_cv1g1gvanyjgm");
    }
}
