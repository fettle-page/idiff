use idiff::session::{self, SessionMeta, slug};
use idiff_core::copy;
use std::sync::{Mutex, MutexGuard};

/// `IDIFF_HOME` is process-wide, so tests that set it take turns.
static HOME_LOCK: Mutex<()> = Mutex::new(());

struct TempHome {
    dir: tempfile::TempDir,
    _lock: MutexGuard<'static, ()>,
}

fn temp_home() -> TempHome {
    let lock = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    // SAFETY: every test that reads or writes IDIFF_HOME holds HOME_LOCK.
    unsafe { std::env::set_var("IDIFF_HOME", dir.path()) };
    TempHome { dir, _lock: lock }
}

#[test]
fn slug_is_safe() {
    assert_eq!(slug("Discord.Discord!"), "discord-discord");
    assert_eq!(slug("  --Niccolò's App--  "), "niccol-s-app");
    assert_eq!(slug("!!!"), "app");
    assert_eq!(slug(""), "app");
    let long = slug(&"a-".repeat(40));
    assert!(long.len() <= 32 && !long.ends_with('-'), "{long}");
}

#[test]
fn home_follows_idiff_home() {
    let h = temp_home();
    assert_eq!(session::home(), h.dir.path());
}

#[test]
fn created_sessions_have_safe_ids_and_metadata() {
    let _h = temp_home();
    let s = session::create("Discord.Discord").unwrap();
    let (stamp, rest) = s.id.split_at(15);
    assert!(
        stamp[..8].bytes().all(|b| b.is_ascii_digit()) && &stamp[8..9] == "-",
        "{}",
        s.id
    );
    assert!(stamp[9..].bytes().all(|b| b.is_ascii_digit()), "{}", s.id);
    assert_eq!(rest, "-discord-discord");
    let meta: SessionMeta = s.read_json("session.json").unwrap();
    assert_eq!(meta.id, s.id);
    assert_eq!(meta.idiff_version, env!("CARGO_PKG_VERSION"));
}

#[test]
fn two_sessions_in_the_same_second_get_different_ids() {
    let _h = temp_home();
    let a = session::create("app").unwrap();
    let b = session::create("app").unwrap();
    assert_ne!(a.id, b.id);
}

#[test]
fn json_round_trips_through_a_session() {
    let _h = temp_home();
    let s = session::create("x").unwrap();
    s.write_json("receipt.json", &vec![1, 2, 3]).unwrap();
    assert_eq!(s.read_json::<Vec<i32>>("receipt.json").unwrap(), [1, 2, 3]);
    assert!(s.read_json::<Vec<i32>>("missing.json").is_err());
}

#[test]
fn last_is_the_newest() {
    let _h = temp_home();
    // Created in the same second, and the newer slug sorts first alphabetically.
    session::create("zoom").unwrap();
    let newest = session::create("anydesk").unwrap();
    assert_eq!(session::open("last").unwrap().id, newest.id);
    let ids: Vec<String> = session::list().into_iter().map(|m| m.id).collect();
    assert_eq!(ids.len(), 2);
    assert_eq!(ids[1], newest.id);
}

#[test]
fn last_with_no_sessions_explains() {
    let _h = temp_home();
    assert_eq!(session::open("last").unwrap_err(), copy::NO_SESSIONS);
}

#[test]
fn open_by_id_finds_it_and_refuses_paths() {
    let _h = temp_home();
    let s = session::create("vlc").unwrap();
    assert_eq!(session::open(&s.id).unwrap().id, s.id);
    for bad in ["nope", "..", r"..\..\windows", "a/b", ""] {
        assert_eq!(session::open(bad).unwrap_err(), copy::NO_SUCH_SESSION, "{bad}");
    }
}
