//! Publisher names as installers, signatures and Store packages write them, made comparable:
//! "Discord Inc.", "Discord, Inc" and "CN=Discord Inc., O=Discord Inc., …" are one publisher.

/// Company-form words that say nothing about who the publisher is.
const SUFFIXES: &[&str] = &["inc", "ltd", "llc", "corporation", "corp", "gmbh", "srl"];

/// The organization in a certificate-style name (`CN=…, O=Spotify AB, …`), else the text itself.
pub fn display(s: &str) -> String {
    let fields = dn_fields(s);
    let field = |name: &str| {
        fields
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.clone())
    };
    field("O")
        .or_else(|| field("CN"))
        .unwrap_or_else(|| s.trim().to_string())
}

/// Lowercase words without punctuation or company-form suffixes.
pub fn normalize(s: &str) -> String {
    let name = display(s).to_lowercase().replace("s.r.l.", "srl");
    name.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty() && !SUFFIXES.contains(w))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The same publisher. Microsoft signs as "Microsoft Corporation", "Microsoft Windows" and
/// others, so any two Microsoft names count as the same.
pub fn same(a: &str, b: &str) -> bool {
    let a_norm = normalize(a);
    !a_norm.is_empty() && (a_norm == normalize(b) || (is_microsoft(a) && is_microsoft(b)))
}

pub fn is_microsoft(s: &str) -> bool {
    normalize(s).split(' ').next() == Some("microsoft")
}

/// `KEY=value` parts of a certificate-style name, or nothing if `s` isn't one. Quoted values may
/// contain commas.
fn dn_fields(s: &str) -> Vec<(String, String)> {
    let looks_like_dn = s
        .trim_start()
        .split_once('=')
        .is_some_and(|(k, _)| !k.is_empty() && k.len() <= 6 && k.chars().all(|c| c.is_ascii_alphabetic() || c == '.'));
    if !looks_like_dn {
        return Vec::new();
    }
    let mut parts = Vec::new();
    let (mut current, mut quoted) = (String::new(), false);
    for c in s.chars() {
        match c {
            '"' => quoted = !quoted,
            ',' if !quoted => parts.push(std::mem::take(&mut current)),
            _ => current.push(c),
        }
    }
    parts.push(current);
    parts
        .iter()
        .filter_map(|p| p.split_once('='))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .collect()
}
