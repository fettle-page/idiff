// Copied from fettle-page/fettle@442e166 crates/fettle-collect/src/sources/windows.rs (reg_value_to_json) and
// crates/fettle-collect/src/sources/startup.rs (open_error) (MIT). Changes: `values` added below them.
//! Registry helpers every source shares.

use serde_json::Value;
use std::collections::BTreeMap;
use winreg::enums::RegType;
use winreg::types::FromRegValue;
use winreg::{RegKey, RegValue};

/// Registry value as JSON: strings and numbers as-is, anything else as hex.
pub fn reg_value_to_json(v: &RegValue) -> Value {
    match v.vtype {
        RegType::REG_SZ | RegType::REG_EXPAND_SZ => String::from_reg_value(v).map(Value::from).unwrap_or(Value::Null),
        RegType::REG_MULTI_SZ => Vec::<String>::from_reg_value(v).map(Value::from).unwrap_or(Value::Null),
        RegType::REG_DWORD => u32::from_reg_value(v).map(Value::from).unwrap_or(Value::Null),
        RegType::REG_QWORD => u64::from_reg_value(v).map(Value::from).unwrap_or(Value::Null),
        _ => Value::from(v.bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()),
    }
}

/// A missing key or folder just means "nothing there". Anything else (access denied)
/// is kept, so it can't masquerade as an empty list.
pub fn open_error(e: &std::io::Error) -> Option<String> {
    (e.kind() != std::io::ErrorKind::NotFound).then(|| e.to_string())
}

/// Every named value of `key` as text: numbers in decimal, multi-strings one per line, anything
/// else as hex. Values that can't be read are left out.
pub fn values(key: &RegKey) -> BTreeMap<String, String> {
    key.enum_values()
        .flatten()
        .filter(|(name, _)| !name.is_empty())
        .map(|(name, value)| {
            let text = match reg_value_to_json(&value) {
                Value::String(s) => s,
                Value::Array(lines) => lines.iter().filter_map(Value::as_str).collect::<Vec<_>>().join("\n"),
                Value::Null => String::new(),
                other => other.to_string(),
            };
            (name, text)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_is_silent_other_errors_are_kept() {
        use std::io::{Error, ErrorKind};
        assert_eq!(open_error(&Error::from(ErrorKind::NotFound)), None);
        assert!(open_error(&Error::from_raw_os_error(5)).is_some());
    }

    #[test]
    fn numbers_stay_numbers_and_binary_is_hex() {
        let dword = RegValue {
            vtype: RegType::REG_DWORD,
            bytes: 412_000u32.to_le_bytes().to_vec().into(),
        };
        let binary = RegValue {
            vtype: RegType::REG_BINARY,
            bytes: vec![0x02, 0xff].into(),
        };
        assert_eq!(reg_value_to_json(&dword), Value::from(412_000));
        assert_eq!(reg_value_to_json(&binary), Value::from("02ff"));
    }
}
