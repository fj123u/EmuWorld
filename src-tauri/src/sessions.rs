//! Journal of play sessions: one line per session (console, game, start,
//! duration, emulator) in `%LOCALAPPDATA%/EmuWorld/sessions.json`.
//!
//! `playtime.json` only keeps totals, so "3 h on N64 this week" cannot be
//! read from it. The challenges (src/challenges.ts) are computed from this
//! journal over any period, so nothing has to be joined beforehand and a
//! session never counts twice. Kept 400 days, like `active_days`.

use crate::push_log;
use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// History kept: a year of challenges and the month before it.
pub const KEEP_DAYS: i64 = 400;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Session {
    pub console: String,
    pub name: String,
    /// Start of the session, RFC 3339 (UTC).
    pub start: String,
    pub seconds: u64,
    pub emulator: String,
}

fn path() -> PathBuf {
    crate::emuworld_base_dir().join("sessions.json")
}

/// One writer at a time: two games closing together must not lose a line.
fn lock() -> &'static Mutex<()> {
    static LOCK: Mutex<()> = Mutex::new(());
    &LOCK
}

/// The journal in `path`; empty when missing. A damaged file is set aside as
/// `sessions.json.bad` so the next session starts a clean journal instead of
/// failing every time.
pub fn load_from(path: &Path) -> Vec<Session> {
    match std::fs::read_to_string(path) {
        Ok(text) => match serde_json::from_str::<Vec<Session>>(&text) {
            Ok(sessions) => sessions,
            Err(e) => {
                push_log("WARN", &format!("Journal des sessions illisible, mis de côté: {}", e));
                let _ = std::fs::rename(path, path.with_extension("json.bad"));
                vec![]
            }
        },
        Err(_) => vec![],
    }
}

/// Adds `session`, drops what started more than `KEEP_DAYS` before `now`.
pub fn add_to(path: &Path, session: Session, now: DateTime<Utc>) -> Result<(), String> {
    let mut sessions = load_from(path);
    sessions.push(session);
    let oldest = now - TimeDelta::days(KEEP_DAYS);
    sessions.retain(|s| DateTime::parse_from_rfc3339(&s.start).map(|d| d.with_timezone(&Utc) >= oldest).unwrap_or(false));
    let data = serde_json::to_string(&sessions).map_err(|e| e.to_string())?;
    crate::atomic_file::write(path, data.as_bytes()).map_err(|e| format!("Journal des sessions non écrit: {}", e))
}

/// A session that just ended (now) after `seconds`.
pub fn session_ending_now(console: &str, name: &str, seconds: u64, emulator: &str, now: DateTime<Utc>) -> Session {
    let start = now - TimeDelta::seconds(i64::try_from(seconds).unwrap_or(0));
    Session {
        console: console.to_string(),
        name: name.to_string(),
        start: start.to_rfc3339(),
        seconds,
        emulator: emulator.to_string(),
    }
}

/// Records a session that just ended. Errors are logged, never fatal: the
/// play time itself is already saved in playtime.json.
pub fn record(console: &str, name: &str, seconds: u64, emulator: &str) {
    let now = Utc::now();
    record_session(session_ending_now(console, name, seconds, emulator, now), now);
}

/// Records a session recovered after EmuWorld was closed during a game: it
/// started at `start_epoch` (Unix seconds).
pub fn record_started(console: &str, name: &str, seconds: u64, emulator: &str, start_epoch: u64) {
    let start = i64::try_from(start_epoch).ok().and_then(|t| DateTime::from_timestamp(t, 0)).unwrap_or_else(Utc::now);
    let session = Session { console: console.to_string(), name: name.to_string(), start: start.to_rfc3339(), seconds, emulator: emulator.to_string() };
    record_session(session, Utc::now());
}

fn record_session(session: Session, now: DateTime<Utc>) {
    let _guard = lock().lock().unwrap_or_else(|p| p.into_inner());
    let (console, name, seconds) = (session.console.clone(), session.name.clone(), session.seconds);
    match add_to(&path(), session, now) {
        Ok(()) => push_log("INFO", &format!("Journal des sessions: '{}' ({}) — {}s", name, console, seconds)),
        Err(e) => push_log("ERROR", &e),
    }
}

/// Removes the journal (sign-out: the next user starts from nothing).
pub fn clear() {
    let _guard = lock().lock().unwrap_or_else(|p| p.into_inner());
    let _ = std::fs::remove_file(path());
    push_log("INFO", "Journal des sessions vidé");
}

/// Sessions that ended after `since` (RFC 3339), all of them without it.
pub fn since(sessions: Vec<Session>, since: Option<&str>) -> Vec<Session> {
    let Some(since) = since.and_then(|s| DateTime::parse_from_rfc3339(s).ok()).map(|d| d.with_timezone(&Utc)) else {
        return sessions;
    };
    sessions
        .into_iter()
        .filter(|s| {
            DateTime::parse_from_rfc3339(&s.start)
                .map(|start| start.with_timezone(&Utc) + TimeDelta::seconds(i64::try_from(s.seconds).unwrap_or(0)) >= since)
                .unwrap_or(false)
        })
        .collect()
}

#[tauri::command]
pub fn get_sessions(since: Option<String>) -> Vec<Session> {
    let _guard = lock().lock().unwrap_or_else(|p| p.into_inner());
    let sessions = self::since(load_from(&path()), since.as_deref());
    push_log("INFO", &format!("Journal des sessions lu: {} session(s)", sessions.len()));
    sessions
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("emuworld_sessions_{}_{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("sessions.json")
    }

    fn at(rfc: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(rfc).unwrap().with_timezone(&Utc)
    }

    #[test]
    fn adds_sessions_with_their_start() {
        let path = temp("add");
        let end = at("2026-11-22T20:00:00Z");
        add_to(&path, session_ending_now("NES", "Zelda", 3600, "mesen", end), end).unwrap();
        add_to(&path, session_ending_now("N64", "Mario 64", 60, "mupen", end), end).unwrap();
        let sessions = load_from(&path);
        assert_eq!(sessions.len(), 2);
        assert_eq!(at(&sessions[0].start), at("2026-11-22T19:00:00Z"));
        assert_eq!(sessions[1].emulator, "mupen");
    }

    #[test]
    fn keeps_400_days() {
        let path = temp("keep");
        let now = at("2026-11-22T12:00:00Z");
        let old = Session { console: "NES".into(), name: "Old".into(), start: "2025-10-01T10:00:00+00:00".into(), seconds: 60, emulator: "x".into() };
        let recent = Session { start: "2025-11-01T10:00:00+00:00".into(), name: "Recent".into(), ..old.clone() };
        std::fs::write(&path, serde_json::to_string(&vec![old, recent]).unwrap()).unwrap();
        add_to(&path, session_ending_now("NES", "New", 60, "x", now), now).unwrap();
        let names: Vec<String> = load_from(&path).into_iter().map(|s| s.name).collect();
        assert_eq!(names, vec!["Recent", "New"]);
    }

    #[test]
    fn a_damaged_journal_is_set_aside_not_fatal() {
        let path = temp("bad");
        std::fs::write(&path, "{ not json").unwrap();
        assert!(load_from(&path).is_empty());
        assert!(path.with_extension("json.bad").exists());
        let now = at("2026-11-22T12:00:00Z");
        add_to(&path, session_ending_now("NES", "Zelda", 60, "x", now), now).unwrap();
        assert_eq!(load_from(&path).len(), 1);
    }

    #[test]
    fn filters_the_sessions_that_ended_after_a_date() {
        let s = |start: &str, seconds: u64| Session { console: "NES".into(), name: start.into(), start: start.into(), seconds, emulator: "x".into() };
        let all = vec![s("2026-11-01T10:00:00+00:00", 60), s("2026-11-09T23:30:00+00:00", 3600), s("2026-11-12T10:00:00+00:00", 60)];
        let names = |v: Vec<Session>| v.into_iter().map(|s| s.name).collect::<Vec<_>>();
        // The second one started before the date but ended after it: it counts.
        assert_eq!(names(since(all.clone(), Some("2026-11-10T00:00:00Z"))), vec!["2026-11-09T23:30:00+00:00", "2026-11-12T10:00:00+00:00"]);
        assert_eq!(since(all.clone(), None).len(), 3);
        assert_eq!(since(all, Some("not a date")).len(), 3);
    }
}
