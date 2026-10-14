//! Sessions: one folder per run under idiff's home, holding its snapshots and receipt.

use idiff_core::copy;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const META: &str = "session.json";

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SessionMeta {
    pub id: String,
    pub command: Vec<String>,
    pub app_flag: Option<String>,
    pub subject: Option<String>,
    pub exit_code: Option<i32>,
    pub started_ms: u64,
    pub finished_ms: Option<u64>,
    pub idiff_version: String,
}

#[derive(Debug, Clone)]
pub struct Session {
    pub id: String,
    pub dir: PathBuf,
}

impl Session {
    pub fn write_json(&self, name: &str, value: &impl Serialize) -> io::Result<()> {
        let json = serde_json::to_string_pretty(value).map_err(io::Error::other)?;
        fs::write(self.file(name), json)
    }

    pub fn read_json<T: DeserializeOwned>(&self, name: &str) -> Result<T, String> {
        let path = self.file(name);
        let failed = |e: &dyn std::fmt::Display| format!("{} {}: {e}", copy::CANT_READ_FILE, path.display());
        let text = fs::read_to_string(&path).map_err(|e| failed(&e))?;
        serde_json::from_str(&text).map_err(|e| failed(&e))
    }

    fn file(&self, name: &str) -> PathBuf {
        debug_assert!(!name.contains(['/', '\\']), "a session file name, not a path: {name}");
        self.dir.join(name)
    }
}

/// `IDIFF_HOME` if set, else `%LOCALAPPDATA%\idiff`.
pub fn home() -> PathBuf {
    match std::env::var_os("IDIFF_HOME") {
        Some(h) if !h.is_empty() => PathBuf::from(h),
        _ => std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("idiff"),
    }
}

fn sessions_dir() -> PathBuf {
    home().join("sessions")
}

/// A session id is also a folder name, so it may only hold these characters.
fn is_safe_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// The app part of a session id: lowercase `[a-z0-9-]`, at most 32 characters, `app` if nothing is left.
pub fn slug(hint: &str) -> String {
    let mut s = String::new();
    for c in hint.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            s.push(c);
        } else if !s.is_empty() && !s.ends_with('-') {
            s.push('-');
        }
    }
    // Only ASCII is pushed, so cutting at a byte count can't split a character.
    s.truncate(32);
    let s = s.trim_end_matches('-');
    if s.is_empty() { "app".into() } else { s.into() }
}

pub fn create(app_hint: &str) -> io::Result<Session> {
    let root = sessions_dir();
    fs::create_dir_all(&root)?;
    // Strictly after every existing session, so `last` is right even for two runs in one millisecond.
    let started_ms = list()
        .iter()
        .map(|m| m.started_ms + 1)
        .max()
        .map_or(now_ms(), |next| next.max(now_ms()));
    let base = format!("{}-{}", utc_stamp(started_ms / 1000), slug(app_hint));
    let mut n = 1;
    loop {
        let id = if n == 1 { base.clone() } else { format!("{base}-{n}") };
        let dir = root.join(&id);
        // create_dir, not create_dir_all: an existing folder means the id is taken.
        match fs::create_dir(&dir) {
            Ok(()) => {
                let session = Session { id: id.clone(), dir };
                let meta = SessionMeta {
                    id,
                    command: Vec::new(),
                    app_flag: None,
                    subject: None,
                    exit_code: None,
                    started_ms,
                    finished_ms: None,
                    idiff_version: env!("CARGO_PKG_VERSION").into(),
                };
                session.write_json(META, &meta)?;
                return Ok(session);
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => n += 1,
            Err(e) => return Err(e),
        }
    }
}

/// `"last"` is the newest session; anything else must be an existing session's id.
pub fn open(id_or_last: &str) -> Result<Session, String> {
    let id = if id_or_last == "last" {
        list().pop().ok_or(copy::NO_SESSIONS)?.id
    } else {
        id_or_last.to_string()
    };
    let dir = sessions_dir().join(&id);
    if is_safe_id(&id) && dir.join(META).is_file() {
        Ok(Session { id, dir })
    } else {
        Err(copy::NO_SUCH_SESSION.into())
    }
}

/// Every readable session, oldest first.
pub fn list() -> Vec<SessionMeta> {
    let Ok(entries) = fs::read_dir(sessions_dir()) else {
        return Vec::new();
    };
    let mut metas: Vec<SessionMeta> = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let session = Session {
                id: name.clone(),
                dir: entry.path(),
            };
            let meta: SessionMeta = session.read_json(META).ok()?;
            // A session.json can't point anywhere but its own folder.
            (meta.id == name && is_safe_id(&name)).then_some(meta)
        })
        .collect();
    metas.sort_by(|a, b| (a.started_ms, &a.id).cmp(&(b.started_ms, &b.id)));
    metas
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

/// `YYYYMMDD-HHMMSS` in UTC, from seconds since 1970.
fn utc_stamp(secs: u64) -> String {
    // Days to a civil date: Howard Hinnant's algorithm.
    let days = (secs / 86_400) as i64 + 719_468;
    let (era, doe) = (days.div_euclid(146_097), days.rem_euclid(146_097));
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    let rem = secs % 86_400;
    format!(
        "{year:04}{month:02}{day:02}-{:02}{:02}{:02}",
        rem / 3_600,
        rem / 60 % 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    use super::utc_stamp;

    #[test]
    fn utc_stamps_are_calendar_dates() {
        assert_eq!(utc_stamp(0), "19700101-000000");
        assert_eq!(utc_stamp(951_868_799), "20000229-235959");
        assert_eq!(utc_stamp(1_791_981_296), "20261014-123456");
    }
}
