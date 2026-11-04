use serde::{Deserialize, Deserializer, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct GameEntry {
    pub console: String,
    pub name: String,
    pub seconds: u64,
    pub launches: u32,
    pub last_played: Option<String>,
    pub first_played: Option<String>,
    #[serde(default)]
    pub favorite: bool,
    /// Emulator id used the last time this game was launched — used to build
    /// "most used emulator" stats without having to re-scan the catalog.
    #[serde(default)]
    pub last_emulator_id: Option<String>,
    #[serde(default)]
    pub rating: Option<u8>,
    #[serde(default)]
    pub notes: Option<String>,
    /// Local month (`YYYY-MM`) → time, launches and longest session recorded
    /// that month. Empty for history recorded before per-month tracking
    /// existed: only lifetime totals (`seconds`, `launches`) are known then.
    #[serde(default, deserialize_with = "null_as_default")]
    pub monthly: BTreeMap<String, MonthlyStats>,
    /// Local dates (`YYYY-MM-DD`) with play time, sorted, at most
    /// `MAX_ACTIVE_DAYS` entries (oldest dropped first).
    #[serde(default, deserialize_with = "null_as_default")]
    pub active_days: Vec<String>,
    /// Longest single session ever recorded for this game, in seconds.
    #[serde(default, deserialize_with = "null_as_default")]
    pub longest_session: u64,
}

/// Aggregates for one game over one calendar month (local time).
#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
pub struct MonthlyStats {
    #[serde(default)]
    pub seconds: u64,
    #[serde(default)]
    pub launches: u32,
    #[serde(default)]
    pub longest_session: u64,
}

/// Enough to cover the current and previous month plus a year of history,
/// while keeping playtime.json (and the cloud row) small.
const MAX_ACTIVE_DAYS: usize = 400;

/// The cloud pull can hand us `null` for columns written by older clients.
fn null_as_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct GameCollection {
    pub name: String,
    pub games: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct PlaytimeStore {
    #[serde(default)]
    pub games: HashMap<String, GameEntry>,
    /// Emulator id → total seconds spent. Cached so the profile page doesn't
    /// have to recompute it every render.
    #[serde(default)]
    pub emulators: HashMap<String, u64>,
    #[serde(default)]
    pub collections: Vec<GameCollection>,
}

fn store_path() -> PathBuf {
    let mut path = crate::emuworld_base_dir();
    path.push("playtime.json");
    path
}

fn load_from(path: &Path) -> Result<PlaytimeStore, String> {
    match std::fs::read_to_string(path) {
        Ok(data) => serde_json::from_str::<PlaytimeStore>(&data)
            .map_err(|error| format!("Invalid playtime data in {}: {}", path.display(), error)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(PlaytimeStore::default()),
        Err(error) => Err(format!("Cannot read playtime data from {}: {}", path.display(), error)),
    }
}

pub fn load() -> Result<PlaytimeStore, String> {
    let path = store_path();
    load_from(&path).map_err(|error| {
        crate::push_log("ERROR", &error);
        error
    })
}

fn save_to(path: &Path, store: &PlaytimeStore) -> Result<(), String> {
    let data = serde_json::to_string_pretty(store).map_err(|e| e.to_string())?;
    crate::atomic_file::write(path, data.as_bytes())
        .map_err(|error| format!("Cannot save playtime data to {}: {}", path.display(), error))
}

pub fn save(store: &PlaytimeStore) -> Result<(), String> {
    let path = store_path();
    crate::push_log("INFO", "Sauvegarde des données de jeu");
    save_to(&path, store).map_err(|error| {
        crate::push_log("ERROR", &error);
        error
    })?;
    crate::push_log("INFO", "Données de jeu sauvegardées");
    Ok(())
}

/// Reset the local store entirely. Called on sign-out so the next user
/// starts from an empty slate, and on sign-in right before we rehydrate
/// from the cloud — otherwise the previous account's stats would leak
/// into the new one via the next sync.
pub fn clear() -> Result<(), String> {
    save(&PlaytimeStore::default())
}

/// Replace the local store with whatever the frontend gives us. Used to
/// pull the signed-in user's cloud data and use it as the source of truth.
pub fn overwrite(store: PlaytimeStore) -> Result<(), String> {
    save(&store)
}

fn key(console: &str, name: &str) -> String {
    format!("{}::{}", console, name)
}

/// Record a completed play session (when the emulator child process exits).
/// Updates total seconds, launches, last-played timestamp, per-emulator totals.
pub fn record_session(console: &str, name: &str, seconds: u64, emulator_id: &str) -> Result<(), String> {
    crate::push_log("INFO", &format!("Session enregistrée: '{}' ({}) — {}s via {}", name, console, seconds, emulator_id));
    let mut store = load()?;
    let now = chrono::Utc::now().to_rfc3339();
    let entry = store.games.entry(key(console, name)).or_insert(GameEntry {
        console: console.to_string(),
        name: name.to_string(),
        first_played: Some(now.clone()),
        ..Default::default()
    });
    // In case the entry already existed but had no first_played (older schema)
    if entry.first_played.is_none() {
        entry.first_played = Some(now.clone());
    }
    entry.seconds += seconds;
    entry.launches += 1;
    entry.last_played = Some(now);
    entry.last_emulator_id = Some(emulator_id.to_string());
    let local_end = chrono::Local::now().naive_local();
    apply_session_stats(entry, local_end, seconds);
    crate::push_log(
        "INFO",
        &format!(
            "Stats mensuelles mises à jour pour '{}' ({}) — mois {}, plus longue session {}s",
            name,
            console,
            local_end.format("%Y-%m"),
            entry.longest_session
        ),
    );

    *store.emulators.entry(emulator_id.to_string()).or_insert(0) += seconds;
    save(&store)
}

/// Spread a session that ended at `end` (local time) and lasted `seconds`
/// over the days and months it covered, so a session played across midnight
/// counts on both days. The launch and the longest-session record go to the
/// month the session ended in.
fn apply_session_stats(entry: &mut GameEntry, end: chrono::NaiveDateTime, seconds: u64) {
    entry.longest_session = entry.longest_session.max(seconds);
    {
        let month = entry.monthly.entry(end.format("%Y-%m").to_string()).or_default();
        month.launches += 1;
        month.longest_session = month.longest_session.max(seconds);
    }
    if seconds == 0 {
        return;
    }

    let start = i64::try_from(seconds)
        .ok()
        .and_then(chrono::TimeDelta::try_seconds)
        .and_then(|duration| end.checked_sub_signed(duration));
    let Some(start) = start else {
        // Absurd duration: keep the totals right, attribute it to the end day.
        crate::push_log("WARN", &format!("Durée de session hors limites ({}s), attribuée au jour de fin", seconds));
        add_time_on_day(entry, end.date(), seconds);
        return;
    };

    let mut cursor = start;
    while cursor < end {
        let next_midnight = cursor
            .date()
            .succ_opt()
            .and_then(|day| day.and_hms_opt(0, 0, 0))
            .unwrap_or(end);
        let segment_end = next_midnight.min(end);
        if segment_end <= cursor {
            break; // safety: never loop forever
        }
        let segment = (segment_end - cursor).num_seconds().max(0) as u64;
        if segment > 0 {
            add_time_on_day(entry, cursor.date(), segment);
        }
        cursor = segment_end;
    }
}

fn add_time_on_day(entry: &mut GameEntry, day: chrono::NaiveDate, seconds: u64) {
    entry.monthly.entry(day.format("%Y-%m").to_string()).or_default().seconds += seconds;
    let day = day.format("%Y-%m-%d").to_string();
    if let Err(position) = entry.active_days.binary_search(&day) {
        entry.active_days.insert(position, day);
    }
    if entry.active_days.len() > MAX_ACTIVE_DAYS {
        let excess = entry.active_days.len() - MAX_ACTIVE_DAYS;
        entry.active_days.drain(..excess);
    }
}

pub fn toggle_favorite(console: &str, name: &str) -> Result<bool, String> {
    let mut store = load()?;
    let k = key(console, name);
    let entry = store.games.entry(k).or_insert(GameEntry {
        console: console.to_string(),
        name: name.to_string(),
        ..Default::default()
    });
    entry.favorite = !entry.favorite;
    let new_value = entry.favorite;
    save(&store)?;
    Ok(new_value)
}

pub fn set_rating(console: &str, name: &str, rating: u8) -> Result<(), String> {
    let mut store = load()?;
    let k = key(console, name);
    let entry = store.games.entry(k).or_insert(GameEntry {
        console: console.to_string(),
        name: name.to_string(),
        ..Default::default()
    });
    entry.rating = if rating == 0 { None } else { Some(rating.min(5)) };
    save(&store)?;
    Ok(())
}

pub fn set_notes(console: &str, name: &str, notes: &str) -> Result<(), String> {
    let mut store = load()?;
    let k = key(console, name);
    let entry = store.games.entry(k).or_insert(GameEntry {
        console: console.to_string(),
        name: name.to_string(),
        ..Default::default()
    });
    entry.notes = if notes.is_empty() { None } else { Some(notes.to_string()) };
    save(&store)?;
    Ok(())
}

pub fn create_collection(name: &str) -> Result<Vec<GameCollection>, String> {
    let mut store = load()?;
    if store.collections.iter().any(|c| c.name == name) {
        return Err(format!("Collection '{}' already exists", name));
    }
    store.collections.push(GameCollection { name: name.to_string(), games: vec![] });
    save(&store)?;
    Ok(store.collections)
}

pub fn delete_collection(name: &str) -> Result<Vec<GameCollection>, String> {
    let mut store = load()?;
    store.collections.retain(|c| c.name != name);
    save(&store)?;
    Ok(store.collections)
}

pub fn rename_collection(old_name: &str, new_name: &str) -> Result<Vec<GameCollection>, String> {
    let mut store = load()?;
    if let Some(col) = store.collections.iter_mut().find(|c| c.name == old_name) {
        col.name = new_name.to_string();
    }
    save(&store)?;
    Ok(store.collections)
}

pub fn add_to_collection(collection_name: &str, game_key: &str) -> Result<(), String> {
    let mut store = load()?;
    if let Some(col) = store.collections.iter_mut().find(|c| c.name == collection_name) {
        if !col.games.contains(&game_key.to_string()) {
            col.games.push(game_key.to_string());
        }
    }
    save(&store)?;
    Ok(())
}

pub fn remove_from_collection(collection_name: &str, game_key: &str) -> Result<(), String> {
    let mut store = load()?;
    if let Some(col) = store.collections.iter_mut().find(|c| c.name == collection_name) {
        col.games.retain(|g| g != game_key);
    }
    save(&store)?;
    Ok(())
}

#[derive(Debug, Serialize, Clone, Default)]
pub struct ProfileStats {
    pub total_seconds: u64,
    pub total_launches: u32,
    pub games_played: u32,
    pub favorite_count: u32,
    pub most_played: Option<GameEntry>,
    pub favorite_game: Option<GameEntry>,
    pub top_games: Vec<GameEntry>,
    pub top_emulator_id: Option<String>,
    pub top_console: Option<String>,
    pub top_console_seconds: u64,
    pub first_played: Option<String>,
    /// Days in a row with at least one launch, counting from today.
    pub streak_days: u32,
}

pub fn compute_stats() -> Result<ProfileStats, String> {
    let store = load()?;
    Ok(compute_stats_from_store(&store))
}

pub fn compute_stats_from_store(store: &PlaytimeStore) -> ProfileStats {
    let mut stats = ProfileStats::default();

    let mut per_console: HashMap<String, u64> = HashMap::new();
    let mut play_days: Vec<chrono::NaiveDate> = Vec::new();

    for entry in store.games.values() {
        stats.total_seconds += entry.seconds;
        stats.total_launches += entry.launches;
        if entry.seconds > 0 || entry.launches > 0 {
            stats.games_played += 1;
        }
        if entry.favorite {
            stats.favorite_count += 1;
        }
        *per_console.entry(entry.console.clone()).or_insert(0) += entry.seconds;

        if let Some(ts) = entry.last_played.as_ref() {
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts) {
                play_days.push(dt.date_naive());
            }
        }
    }

    // Most played by seconds
    stats.most_played = store
        .games
        .values()
        .max_by_key(|e| e.seconds)
        .filter(|e| e.seconds > 0)
        .cloned();

    // Favorite: first one marked favorite (most played among favorites)
    stats.favorite_game = store
        .games
        .values()
        .filter(|e| e.favorite)
        .max_by_key(|e| e.seconds)
        .cloned();

    // Top 5 games (most time first)
    let mut games: Vec<GameEntry> = store.games.values().cloned().collect();
    games.sort_by(|a, b| b.seconds.cmp(&a.seconds));
    stats.top_games = games.into_iter().filter(|e| e.seconds > 0).take(5).collect();

    // Top emulator
    stats.top_emulator_id = store
        .emulators
        .iter()
        .max_by_key(|(_, v)| *v)
        .filter(|(_, v)| **v > 0)
        .map(|(k, _)| k.clone());

    // Top console
    if let Some((console, seconds)) = per_console.iter().max_by_key(|(_, v)| *v) {
        stats.top_console = Some(console.clone());
        stats.top_console_seconds = *seconds;
    }

    // First played = oldest first_played across all games
    stats.first_played = store
        .games
        .values()
        .filter_map(|e| e.first_played.clone())
        .min();

    // Streak: consecutive days ending today with at least one play
    if !play_days.is_empty() {
        play_days.sort();
        play_days.dedup();
        let today = chrono::Utc::now().date_naive();
        let mut day = today;
        let mut streak = 0u32;
        while play_days.binary_search(&day).is_ok() {
            streak += 1;
            day = day.pred_opt().unwrap_or(day);
            if day == today { break; } // safety
        }
        stats.streak_days = streak;
    }

    stats
}

#[cfg(test)]
mod tests {
    use super::{
        apply_session_stats, compute_stats_from_store, load_from, save_to, GameEntry, MonthlyStats,
        PlaytimeStore, MAX_ACTIVE_DAYS,
    };
    use chrono::{NaiveDate, NaiveDateTime};
    use std::collections::HashMap;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_path() -> PathBuf {
        let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        std::env::temp_dir().join(format!("emuworld-playtime-{}-{}.json", std::process::id(), id))
    }

    #[test]
    fn missing_file_loads_as_empty_store() {
        let path = test_path();
        assert!(load_from(&path).unwrap().games.is_empty());
    }

    #[test]
    fn invalid_file_returns_error_without_changing_original() {
        let path = test_path();
        let original = b"{not valid json";
        fs::write(&path, original).unwrap();

        assert!(load_from(&path).is_err());
        assert_eq!(fs::read(&path).unwrap(), original);

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn saves_and_reloads_store_atomically() {
        let path = test_path();
        let mut store = PlaytimeStore::default();
        store.games.insert(
            "Nintendo::Game".to_string(),
            GameEntry {
                console: "Nintendo".to_string(),
                name: "Game".to_string(),
                seconds: 120,
                ..Default::default()
            },
        );

        save_to(&path, &store).unwrap();
        let loaded = load_from(&path).unwrap();

        assert_eq!(loaded.games["Nintendo::Game"].seconds, 120);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn computes_stats_from_loaded_store() {
        let store = PlaytimeStore {
            games: HashMap::from([(
                "Nintendo::Game".to_string(),
                GameEntry {
                    console: "Nintendo".to_string(),
                    name: "Game".to_string(),
                    seconds: 3600,
                    launches: 2,
                    ..Default::default()
                },
            )]),
            ..Default::default()
        };

        let stats = compute_stats_from_store(&store);

        assert_eq!(stats.total_seconds, 3600);
        assert_eq!(stats.total_launches, 2);
        assert_eq!(stats.games_played, 1);
    }

    fn at(y: i32, m: u32, d: u32, h: u32, min: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d).unwrap().and_hms_opt(h, min, 0).unwrap()
    }

    #[test]
    fn session_is_counted_in_the_month_it_was_played() {
        let mut entry = GameEntry::default();
        apply_session_stats(&mut entry, at(2026, 10, 12, 20, 0), 3600);
        apply_session_stats(&mut entry, at(2026, 11, 3, 21, 0), 600);

        assert_eq!(
            entry.monthly["2026-10"],
            MonthlyStats { seconds: 3600, launches: 1, longest_session: 3600 }
        );
        assert_eq!(
            entry.monthly["2026-11"],
            MonthlyStats { seconds: 600, launches: 1, longest_session: 600 }
        );
        assert_eq!(entry.active_days, vec!["2026-10-12", "2026-11-03"]);
    }

    #[test]
    fn session_across_month_change_is_split_at_midnight() {
        let mut entry = GameEntry::default();
        // 23:30 on Oct 31 → 00:30 on Nov 1
        apply_session_stats(&mut entry, at(2026, 11, 1, 0, 30), 3600);

        assert_eq!(entry.monthly["2026-10"].seconds, 1800);
        assert_eq!(entry.monthly["2026-10"].launches, 0);
        assert_eq!(entry.monthly["2026-11"].seconds, 1800);
        assert_eq!(entry.monthly["2026-11"].launches, 1);
        assert_eq!(entry.monthly["2026-11"].longest_session, 3600);
        assert_eq!(entry.active_days, vec!["2026-10-31", "2026-11-01"]);
    }

    #[test]
    fn longest_session_keeps_the_maximum() {
        let mut entry = GameEntry::default();
        apply_session_stats(&mut entry, at(2026, 11, 1, 12, 0), 100);
        apply_session_stats(&mut entry, at(2026, 11, 2, 12, 0), 500);
        apply_session_stats(&mut entry, at(2026, 11, 2, 18, 0), 200);
        apply_session_stats(&mut entry, at(2026, 12, 1, 18, 0), 300);

        assert_eq!(entry.longest_session, 500);
        assert_eq!(entry.monthly["2026-11"].longest_session, 500);
        assert_eq!(entry.monthly["2026-11"].launches, 3);
        assert_eq!(entry.monthly["2026-11"].seconds, 800);
        assert_eq!(entry.monthly["2026-12"].longest_session, 300);
        // Same day twice is only one active day
        assert_eq!(entry.active_days, vec!["2026-11-01", "2026-11-02", "2026-12-01"]);
    }

    #[test]
    fn zero_second_session_counts_a_launch_but_no_active_day() {
        let mut entry = GameEntry::default();
        apply_session_stats(&mut entry, at(2026, 11, 4, 10, 0), 0);

        assert_eq!(entry.monthly["2026-11"].launches, 1);
        assert_eq!(entry.monthly["2026-11"].seconds, 0);
        assert!(entry.active_days.is_empty());
    }

    #[test]
    fn active_days_are_capped_dropping_oldest() {
        let mut entry = GameEntry::default();
        let first = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        for offset in 0..(MAX_ACTIVE_DAYS as u64 + 10) {
            let day = first + chrono::Days::new(offset);
            apply_session_stats(&mut entry, day.and_hms_opt(12, 0, 0).unwrap(), 60);
        }

        assert_eq!(entry.active_days.len(), MAX_ACTIVE_DAYS);
        assert_eq!(entry.active_days[0], (first + chrono::Days::new(10)).format("%Y-%m-%d").to_string());
    }

    #[test]
    fn legacy_and_null_fields_load_as_empty() {
        let legacy = r#"{"console":"Nintendo","name":"Game","seconds":36000,"launches":40,
            "last_played":"2026-11-04T10:00:00Z","first_played":null}"#;
        let entry: GameEntry = serde_json::from_str(legacy).unwrap();
        assert!(entry.monthly.is_empty());
        assert!(entry.active_days.is_empty());
        assert_eq!(entry.longest_session, 0);

        let nulls = r#"{"console":"Nintendo","name":"Game","seconds":1,"launches":1,
            "last_played":null,"first_played":null,
            "monthly":null,"active_days":null,"longest_session":null}"#;
        let entry: GameEntry = serde_json::from_str(nulls).unwrap();
        assert!(entry.monthly.is_empty());
        assert_eq!(entry.longest_session, 0);
    }
}
