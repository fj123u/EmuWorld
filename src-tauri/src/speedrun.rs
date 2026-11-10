//! Speedrun timer: splits and records per game (`speedruns.json`), and the
//! small always-on-top timer window driven by numpad hotkeys
//! (see `hotkey_action`, fed by the low-level keyboard hook in lib.rs).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::push_log;

pub const MAX_SPLITS: usize = 100;
pub const MAX_NAME_CHARS: usize = 60;
/// 100 hours: anything longer is a broken or tampered record.
pub const MAX_TIME_MS: u64 = 100 * 3600 * 1000;
pub const WINDOW_LABEL: &str = "speedrun";

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SpeedrunData {
    /// Segment names, in order (the last one ends the run).
    #[serde(default)]
    pub splits: Vec<String>,
    /// Personal best: time since the start at each split, in ms.
    #[serde(default)]
    pub pb: Option<Vec<u64>>,
    /// Best duration ever of each segment, in ms (`None`: never timed).
    #[serde(default)]
    pub best_segments: Vec<Option<u64>>,
    #[serde(default)]
    pub attempts: u32,
    #[serde(default)]
    pub completed: u32,
}

/// Cleans data coming from the UI: names trimmed and bounded, and a record
/// that does not fit the splits (count, order, absurd times) is dropped
/// rather than kept wrong.
pub fn sanitize(mut data: SpeedrunData) -> SpeedrunData {
    data.splits = data
        .splits
        .iter()
        .map(|s| s.trim().chars().filter(|c| !c.is_control()).take(MAX_NAME_CHARS).collect::<String>())
        .filter(|s| !s.is_empty())
        .take(MAX_SPLITS)
        .collect();
    let n = data.splits.len();
    let pb_ok = data.pb.as_ref().map_or(true, |pb| {
        pb.len() == n && pb.iter().all(|&t| t > 0 && t <= MAX_TIME_MS) && pb.windows(2).all(|w| w[0] <= w[1])
    });
    if !pb_ok {
        data.pb = None;
    }
    if data.best_segments.len() != n || data.best_segments.iter().flatten().any(|&t| t == 0 || t > MAX_TIME_MS) {
        data.best_segments = vec![None; n];
    }
    data.completed = data.completed.min(data.attempts);
    data
}

pub fn store_path() -> PathBuf {
    crate::emuworld_base_dir().join("speedruns.json")
}

pub fn load_store(path: &Path) -> HashMap<String, SpeedrunData> {
    fs::read_to_string(path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

pub fn save_to(path: &Path, key: &str, data: SpeedrunData) -> Result<SpeedrunData, String> {
    if key.is_empty() || key.len() > 400 {
        return Err("Jeu invalide".to_string());
    }
    let clean = sanitize(data);
    let mut store = load_store(path);
    store.insert(key.to_string(), clean.clone());
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_vec_pretty(&store).map_err(|e| e.to_string())?;
    crate::atomic_file::write(path, &json).map_err(|e| e.to_string())?;
    Ok(clean)
}

/// Numpad hotkeys (LiveSplit's defaults) → action sent to the timer window.
pub fn hotkey_action(vk: i32) -> Option<&'static str> {
    match vk {
        0x61 => Some("split"), // Numpad 1
        0x62 => Some("skip"),  // Numpad 2
        0x63 => Some("reset"), // Numpad 3
        0x65 => Some("pause"), // Numpad 5
        0x68 => Some("undo"),  // Numpad 8
        _ => None,
    }
}

#[tauri::command]
pub fn get_speedrun(game_key: String) -> SpeedrunData {
    load_store(&store_path()).remove(&game_key).unwrap_or_default()
}

#[tauri::command]
pub fn save_speedrun(game_key: String, data: SpeedrunData) -> Result<SpeedrunData, String> {
    let result = save_to(&store_path(), &game_key, data);
    match &result {
        Ok(d) => push_log("INFO", &format!(
            "Speedrun: '{}' enregistré ({} splits, PB {:?} ms, {}/{} runs finies)",
            game_key, d.splits.len(), d.pb.as_ref().and_then(|p| p.last()), d.completed, d.attempts
        )),
        Err(e) => push_log("ERROR", &format!("Speedrun: enregistrement de '{}' impossible: {}", game_key, e)),
    }
    result
}

/// Opens the timer in the top-right corner of the main screen: always on
/// top, transparent, click-through and never focused, so the game keeps
/// the keyboard. Reopening it switches it to `game_key`.
/// Async on purpose: a synchronous command that builds a window deadlocks
/// the main thread on Windows (the app then ignored every click, "Jouer"
/// included). A test in lib.rs checks it.
#[tauri::command]
pub async fn open_speedrun_window(app_handle: tauri::AppHandle, game_key: String, game_name: String) -> Result<(), String> {
    use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
    push_log("INFO", &format!("Speedrun: ouverture du chrono pour '{}'", game_key));
    if let Some(win) = app_handle.get_webview_window(WINDOW_LABEL) {
        let _ = win.close();
    }
    let query = format!(
        "index.html?speedrun=1&game={}&name={}",
        urlencoding::encode(&game_key),
        urlencoding::encode(&game_name)
    );
    let (width, height) = (280.0, 380.0);
    let (x, y) = app_handle
        .primary_monitor()
        .ok()
        .flatten()
        .map(|m| {
            let scale = m.scale_factor();
            (m.size().width as f64 / scale - width - 24.0, 24.0)
        })
        .unwrap_or((24.0, 24.0));
    let win = WebviewWindowBuilder::new(&app_handle, WINDOW_LABEL, WebviewUrl::App(query.into()))
        .title("EmuWorld Speedrun")
        .inner_size(width, height)
        .position(x, y)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .focusable(false)
        .focused(false)
        .build()
        .map_err(|e| {
            push_log("ERROR", &format!("Speedrun: fenêtre impossible à créer: {}", e));
            e.to_string()
        })?;
    let _ = win.set_ignore_cursor_events(true);
    Ok(())
}

#[tauri::command]
pub async fn close_speedrun_window(app_handle: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    if let Some(win) = app_handle.get_webview_window(WINDOW_LABEL) {
        win.close().map_err(|e| e.to_string())?;
        push_log("INFO", "Speedrun: chrono fermé");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data(splits: &[&str], pb: Option<Vec<u64>>) -> SpeedrunData {
        SpeedrunData { splits: splits.iter().map(|s| s.to_string()).collect(), pb, ..Default::default() }
    }

    #[test]
    fn keeps_a_consistent_record() {
        let d = sanitize(data(&["Monde 1", "Monde 2", "Bowser"], Some(vec![60_000, 150_000, 300_000])));
        assert_eq!(d.pb, Some(vec![60_000, 150_000, 300_000]));
        assert_eq!(d.best_segments, vec![None, None, None]);
    }

    #[test]
    fn drops_a_record_that_does_not_fit_the_splits() {
        // Wrong count (a split was added), going backwards, or absurd.
        assert_eq!(sanitize(data(&["A", "B", "C"], Some(vec![1, 2]))).pb, None);
        assert_eq!(sanitize(data(&["A", "B"], Some(vec![5_000, 4_000]))).pb, None);
        assert_eq!(sanitize(data(&["A"], Some(vec![MAX_TIME_MS + 1]))).pb, None);
        assert_eq!(sanitize(data(&["A"], Some(vec![0]))).pb, None);
    }

    #[test]
    fn cleans_split_names() {
        let long = "x".repeat(200);
        let d = sanitize(data(&["  Monde 1 ", "", "a\nb", &long], None));
        assert_eq!(d.splits[0], "Monde 1");
        assert_eq!(d.splits[1], "ab");
        assert_eq!(d.splits[2].chars().count(), MAX_NAME_CHARS);
        assert_eq!(d.splits.len(), 3, "empty names are removed");
        let many: Vec<String> = (0..150).map(|i| i.to_string()).collect();
        assert_eq!(sanitize(SpeedrunData { splits: many, ..Default::default() }).splits.len(), MAX_SPLITS);
    }

    #[test]
    fn saves_one_game_without_touching_the_others() {
        let dir = std::env::temp_dir().join(format!("emuworld_speedrun_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let path = dir.join("speedruns.json");
        save_to(&path, "NES::Mario", data(&["Fin"], Some(vec![600_000]))).unwrap();
        save_to(&path, "SNES::Zelda", data(&["Fin"], None)).unwrap();
        let store = load_store(&path);
        assert_eq!(store["NES::Mario"].pb, Some(vec![600_000]));
        assert_eq!(store.len(), 2);
        assert!(save_to(&path, "", data(&["Fin"], None)).is_err());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn maps_livesplit_numpad_keys() {
        assert_eq!(hotkey_action(0x61), Some("split"));
        assert_eq!(hotkey_action(0x63), Some("reset"));
        assert_eq!(hotkey_action(0x68), Some("undo"));
        assert_eq!(hotkey_action(0x41), None, "letters stay with the game");
    }
}
