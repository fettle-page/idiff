//! Paths and command lines as Windows stores them (services, tasks, Run keys, shortcuts), turned
//! into one comparable form: expanded, absolute where Windows implies it, and lowercase.

use crate::model::Env;

/// What a command line can run directly.
const RUNNABLE: &[&str] = &[".exe", ".dll", ".sys", ".cmd", ".bat", ".ps1", ".com", ".scr"];
/// What wscript and cscript run.
const SCRIPTS: &[&str] = &[".vbs", ".vbe", ".js", ".jse", ".wsf"];

/// Trims quotes and spaces, expands `%VAR%` from `env`, resolves the prefixes the kernel and the
/// service manager use (`\SystemRoot\`, `\??\`, a bare `system32\`), and lowercases.
pub fn normalize_path(p: &str, env: &Env) -> String {
    let p = p
        .trim_matches(|c: char| c == '"' || c.is_whitespace())
        .replace('/', "\\");
    let p = expand_vars(&p, env);
    let p = p.strip_prefix(r"\??\").unwrap_or(&p);
    let system_root = lookup(env, "SystemRoot");
    let p = match system_root {
        Some(root) if starts_with_ignore_case(p, r"\SystemRoot\") => format!(r"{root}\{}", &p[r"\SystemRoot\".len()..]),
        Some(root) if starts_with_ignore_case(p, r"system32\") => format!(r"{root}\{p}"),
        _ => p.to_string(),
    };
    p.to_lowercase()
}

/// The file a command line runs, normalized. For launchers (`rundll32`, `regsvr32`, `cmd /c`,
/// `powershell -File`, `wscript`, `cscript`) it's the file they're given, not the launcher.
pub fn extract_exe(cmd: &str, env: &Env) -> Option<String> {
    let (first, rest) = split_first(cmd, RUNNABLE)?;
    let inner = match file_stem(&first).as_str() {
        "rundll32" => rest.split(',').next().and_then(|r| split_first(r, RUNNABLE)),
        "regsvr32" => split_first(skip_flags(rest, |f| f.starts_with('/') || f.starts_with('-')), RUNNABLE),
        "cmd" => after_flag(rest, &["/c", "/k"]).and_then(|r| {
            split_first(
                r.strip_prefix('"').filter(|r| r.starts_with('"')).unwrap_or(r),
                RUNNABLE,
            )
        }),
        "powershell" | "pwsh" => after_flag(rest, &["-file", "-f"]).and_then(|r| split_first(r, RUNNABLE)),
        "wscript" | "cscript" => split_first(skip_flags(rest, |f| f.starts_with("//")), SCRIPTS),
        // `conhost [--headless] <command>` hosts another command line, which may be a launcher too.
        // Windows starts it as `conhost 0xffffffff -ForceV1`, with nothing inside.
        "conhost" => {
            let inner = skip_flags(rest, |f| f.starts_with("--"));
            if !inner.is_empty() && !inner.starts_with('-') && !inner.to_ascii_lowercase().starts_with("0x") {
                return extract_exe(inner, env);
            }
            None
        }
        _ => None,
    };
    let (file, _) = inner.unwrap_or((first, rest));
    Some(normalize_path(&file, env))
}

/// Splits off the first path in `s`: a quoted string, a bare name, else the shortest run of words
/// that ends in one of `exts` (so unquoted `C:\Program Files\x.exe -a` works), else the first word.
fn split_first<'a>(s: &'a str, exts: &[&str]) -> Option<(String, &'a str)> {
    let s = s.trim_start();
    if s.is_empty() {
        return None;
    }
    if let Some(quoted) = s.strip_prefix('"') {
        let end = quoted.find('"').unwrap_or(quoted.len());
        let rest = quoted.get(end + 1..).unwrap_or("");
        return Some((quoted[..end].to_string(), rest));
    }
    // A bare name (`cmd`, `pwsh`) is looked up on PATH, so it can't contain a space.
    let first_end = s.find(' ').unwrap_or(s.len());
    if !s[..first_end].contains(['\\', '/']) {
        return Some((s[..first_end].to_string(), &s[first_end..]));
    }
    let ends = s.match_indices(' ').map(|(i, _)| i).chain([s.len()]);
    for end in ends {
        let candidate = s[..end].to_ascii_lowercase();
        if exts.iter().any(|ext| candidate.ends_with(ext)) {
            return Some((s[..end].to_string(), &s[end..]));
        }
    }
    Some((s[..first_end].to_string(), &s[first_end..]))
}

/// The lowercase file name without folder or extension: `C:\X\RunDll32.EXE` → `rundll32`.
fn file_stem(path: &str) -> String {
    let name = path.rsplit(['\\', '/']).next().unwrap_or(path).to_ascii_lowercase();
    match name.rsplit_once('.') {
        Some((stem, _)) => stem.to_string(),
        None => name,
    }
}

/// Drops leading words that `is_flag` accepts.
fn skip_flags(s: &str, is_flag: impl Fn(&str) -> bool) -> &str {
    let mut s = s.trim_start();
    while let Some(word) = s.split(' ').next().filter(|w| !w.is_empty() && is_flag(w)) {
        s = s[word.len()..].trim_start();
    }
    s
}

/// What follows the first word that is one of `flags` (case-insensitive), or None without one.
fn after_flag<'a>(s: &'a str, flags: &[&str]) -> Option<&'a str> {
    let mut offset = 0;
    for word in s.split(' ') {
        offset += word.len() + 1;
        if flags.iter().any(|f| word.eq_ignore_ascii_case(f)) {
            return Some(s.get(offset..).unwrap_or(""));
        }
    }
    None
}

/// Expands `%NAME%` from `env`, ignoring case. Unknown names are left as they are.
fn expand_vars(s: &str, env: &Env) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('%') {
            Some(end) => match lookup(env, &after[..end]) {
                Some(value) => {
                    out.push_str(value);
                    rest = &after[end + 1..];
                }
                None => {
                    out.push('%');
                    rest = after;
                }
            },
            None => {
                out.push_str(&rest[start..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

/// `%windir%` is the same folder as `%SystemRoot%`, and commands use both.
fn lookup<'a>(env: &'a Env, name: &str) -> Option<&'a str> {
    let name = if name.eq_ignore_ascii_case("windir") {
        "SystemRoot"
    } else {
        name
    };
    env.iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.trim_end_matches('\\'))
}

fn starts_with_ignore_case(s: &str, prefix: &str) -> bool {
    s.get(..prefix.len())
        .is_some_and(|start| start.eq_ignore_ascii_case(prefix))
}
