use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

    *store.emulators.entry(emulator_id.to_string()).or_insert(0) += seconds;
    save(&store)
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
    use super::{compute_stats_from_store, load_from, save_to, GameEntry, PlaytimeStore};
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
}
