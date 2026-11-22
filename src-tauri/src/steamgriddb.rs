//! SteamGridDB (https://www.steamgriddb.com): community-made 600×900 "grids".
//!
//! An extra cover source, used two ways:
//! - as a fallback in `fetch_boxart`, after the exact sources (game icon,
//!   eShop, libretro, LaunchBox) and before Wikipedia / GameTDB;
//! - as a tab of "Choisir la cover", where its many choices help most.
//!
//! API v2 (openapi.yml, v2.10.0): each user's own key, sent as
//! `Authorization: Bearer <key>`, generated on their SteamGridDB profile. It is
//! stored DPAPI-encrypted like the iiDB key and never logged. The search is by
//! name only (no console filter), so the automatic fallback only takes a game
//! whose name is exactly the ROM's title; anything looser is left to the picker.

use crate::iidb::{normalize_title, title_from_rom_name};
use crate::iidb_api::{CoverChoice, CoverGame};
use crate::push_log;
use serde::Serialize;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

pub const API_BASE: &str = "https://www.steamgriddb.com/api/v2";
pub const SITE: &str = "https://www.steamgriddb.com";

/// Portrait grids only (the cards are 3:4), still images, nothing NSFW,
/// joke or flashing.
pub const GRID_QUERY: &str = "dimensions=600x900,342x482,660x930&types=static&mimes=image/png,image/jpeg,image/webp&nsfw=false&humor=false&epilepsy=false";

/// Most games and grids offered in the picker.
const PICKER_GAMES: usize = 4;
const PICKER_GRIDS: usize = 12;

/// Pause after SteamGridDB answered 429.
const RATE_LIMIT_PAUSE: Duration = Duration::from_secs(60);

pub fn valid_key_format(key: &str) -> bool {
    (16..=128).contains(&key.len()) && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Images are only taken from SteamGridDB's own hosts (https).
pub fn is_image_url(url: &str) -> bool {
    match reqwest::Url::parse(url) {
        Ok(u) => u.scheme() == "https" && u.host_str().map(|h| h == "steamgriddb.com" || h.ends_with(".steamgriddb.com")).unwrap_or(false),
        Err(_) => false,
    }
}

/// A game of a search answer.
#[derive(Debug, Clone, PartialEq)]
pub struct SgdbGame {
    pub id: i64,
    pub name: String,
    pub year: Option<i64>,
    pub verified: bool,
}

/// Games of `/search/autocomplete/{term}`.
pub fn search_games(body: &Value) -> Vec<SgdbGame> {
    body["data"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter_map(|g| {
                    Some(SgdbGame {
                        id: g["id"].as_i64()?,
                        name: g["name"].as_str()?.to_string(),
                        // Unix time of the release, when SteamGridDB knows it.
                        year: g["release_date"].as_i64().filter(|t| *t > 0).map(|t| 1970 + t / 31_556_952),
                        verified: g["verified"].as_bool().unwrap_or(false),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The game for an automatic cover: same title as the ROM (case, accents and
/// punctuation aside), a verified one first. None when no name matches.
pub fn pick_game(games: &[SgdbGame], rom_title: &str) -> Option<i64> {
    let target = normalize_title(rom_title);
    if target.is_empty() {
        return None;
    }
    let mut same: Vec<&SgdbGame> = games.iter().filter(|g| normalize_title(&g.name) == target).collect();
    same.sort_by_key(|g| !g.verified);
    same.first().map(|g| g.id)
}

/// Picker order: games with the ROM's title first, then the others.
pub fn picker_games(games: Vec<SgdbGame>, rom_title: &str) -> Vec<SgdbGame> {
    let target = normalize_title(rom_title);
    let (mut same, others): (Vec<_>, Vec<_>) = games.into_iter().partition(|g| normalize_title(&g.name) == target);
    same.sort_by_key(|g| !g.verified);
    same.into_iter().chain(others).take(PICKER_GAMES).collect()
}

/// Grids of `/grids/game/{id}` that come from SteamGridDB's own hosts.
pub fn grid_choices(body: &Value) -> Vec<CoverChoice> {
    body["data"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter_map(|g| {
                    let url = g["url"].as_str().filter(|u| is_image_url(u))?.to_string();
                    let thumbnail = g["thumb"].as_str().filter(|u| is_image_url(u)).map(String::from);
                    Some(CoverChoice { url, thumbnail, region: None, console_id: None })
                })
                .collect()
        })
        .unwrap_or_default()
}

// ---- key storage ----------------------------------------------------------

fn key_path() -> PathBuf {
    crate::emuworld_base_dir().join("steamgriddb_key.dat")
}

fn key_cache() -> &'static Mutex<Option<Option<String>>> {
    static KEY: OnceLock<Mutex<Option<Option<String>>>> = OnceLock::new();
    KEY.get_or_init(|| Mutex::new(None))
}

/// The user's key, read once from disk (DPAPI-encrypted).
pub fn api_key() -> Option<String> {
    let mut cache = key_cache().lock().unwrap_or_else(|p| p.into_inner());
    if cache.is_none() {
        let key = crate::dpapi::read_and_decrypt(&key_path())
            .ok()
            .and_then(|b| String::from_utf8(b).ok())
            .map(|k| k.trim().to_string())
            .filter(|k| valid_key_format(k));
        *cache = Some(key);
    }
    cache.clone().flatten()
}

fn set_cached_key(key: Option<String>) {
    *key_cache().lock().unwrap_or_else(|p| p.into_inner()) = Some(key);
}

fn paused_until() -> &'static Mutex<Option<Instant>> {
    static PAUSE: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();
    PAUSE.get_or_init(|| Mutex::new(None))
}

fn is_paused() -> bool {
    paused_until().lock().map(|p| p.map(|t| Instant::now() < t).unwrap_or(false)).unwrap_or(false)
}

fn http() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .user_agent(format!("EmuWorld/{} (+https://github.com/fj123u/EmuWorld)", env!("CARGO_PKG_VERSION")))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    })
}

/// GET on the API. Err("refused") for a bad key, Err("unavailable") otherwise.
async fn get(path: &str, key: &str) -> Result<Value, &'static str> {
    if is_paused() {
        return Err("unavailable");
    }
    let resp = http()
        .get(format!("{}{}", API_BASE, path))
        .header("Authorization", format!("Bearer {}", key))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| {
            push_log("WARN", &format!("SteamGridDB injoignable: {}", e));
            "unavailable"
        })?;
    match resp.status().as_u16() {
        200 => resp.json().await.map_err(|_| "unavailable"),
        // A search with no game answers 404.
        404 => Ok(Value::Null),
        401 | 403 => Err("refused"),
        429 => {
            push_log("WARN", "SteamGridDB: trop de requêtes, pause d'une minute");
            if let Ok(mut p) = paused_until().lock() {
                *p = Some(Instant::now() + RATE_LIMIT_PAUSE);
            }
            Err("unavailable")
        }
        s => {
            push_log("WARN", &format!("SteamGridDB a répondu HTTP {}", s));
            Err("unavailable")
        }
    }
}

async fn search(title: &str, key: &str) -> Result<Vec<SgdbGame>, &'static str> {
    let body = get(&format!("/search/autocomplete/{}", urlencoding::encode(title)), key).await?;
    Ok(search_games(&body))
}

async fn grids(game_id: i64, key: &str) -> Result<Vec<CoverChoice>, &'static str> {
    let body = get(&format!("/grids/game/{}?{}", game_id, GRID_QUERY), key).await?;
    Ok(grid_choices(&body))
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct SgdbStatus {
    pub configured: bool,
}

/// Checks the key with a search, then stores it encrypted.
#[tauri::command]
pub async fn set_sgdb_api_key(key: String) -> Result<SgdbStatus, String> {
    let key = key.trim().to_string();
    push_log("INFO", "SteamGridDB: vérification d'une clé API");
    if !valid_key_format(&key) {
        return Err("Clé API SteamGridDB au mauvais format".to_string());
    }
    match get("/search/autocomplete/zelda", &key).await {
        Ok(_) => {}
        Err("refused") => {
            push_log("WARN", "SteamGridDB: clé refusée");
            return Err("Clé API SteamGridDB refusée : vérifie-la dans tes préférences SteamGridDB".to_string());
        }
        Err(_) => return Err("SteamGridDB injoignable, réessaie plus tard".to_string()),
    }
    crate::dpapi::encrypt_and_write(&key_path(), key.as_bytes())?;
    set_cached_key(Some(key));
    push_log("INFO", "SteamGridDB: clé enregistrée");
    Ok(SgdbStatus { configured: true })
}

#[tauri::command]
pub fn get_sgdb_status() -> SgdbStatus {
    SgdbStatus { configured: api_key().is_some() }
}

#[tauri::command]
pub fn clear_sgdb_api_key() -> Result<(), String> {
    let path = key_path();
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    set_cached_key(None);
    push_log("INFO", "SteamGridDB: clé API retirée");
    Ok(())
}

/// Automatic fallback: the best grid of the game named exactly like the ROM.
/// None without a key, without an exact match, or when SteamGridDB is down.
pub async fn find_cover(rom_name: &str, console: &str) -> Option<String> {
    let key = api_key()?;
    let title = title_from_rom_name(rom_name);
    let games = search(&title, &key).await.ok()?;
    let Some(id) = pick_game(&games, &title) else {
        push_log("INFO", &format!("SteamGridDB: pas de jeu au nom exact pour '{}' ({})", rom_name, console));
        return None;
    };
    let url = grids(id, &key).await.ok()?.into_iter().next().map(|c| c.url);
    if url.is_none() {
        push_log("INFO", &format!("SteamGridDB: aucune grid portrait pour '{}' ({})", rom_name, console));
    }
    url
}

/// Grids SteamGridDB has for a game, to choose from ("Choisir la cover").
/// Errors: "no_key" without an API key, "unavailable" when unreachable.
#[tauri::command]
pub async fn sgdb_cover_choices(game_name: String, console: String) -> Result<Vec<CoverGame>, String> {
    push_log("INFO", &format!("SteamGridDB: choix de cover pour '{}' ({})", game_name, console));
    let key = api_key().ok_or("no_key")?;
    let title = title_from_rom_name(&game_name);
    let games = picker_games(search(&title, &key).await.map_err(|_| "unavailable")?, &title);
    let mut out = vec![];
    for game in games {
        let covers: Vec<CoverChoice> = grids(game.id, &key).await.map_err(|_| "unavailable")?.into_iter().take(PICKER_GRIDS).collect();
        if !covers.is_empty() {
            out.push(CoverGame { parent_id: game.id, name: game.name, platform: None, year: game.year, covers, remembered: false });
        }
    }
    push_log("INFO", &format!("SteamGridDB: {} jeu(x), {} grids proposées pour '{}'", out.len(), out.iter().map(|g| g.covers.len()).sum::<usize>(), game_name));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Shapes from SteamGridDB's openapi.yml (v2.10.0).
    fn zelda_search() -> Value {
        json!({"success": true, "data": [
            {"id": 5248155, "name": "The Legend of Zelda: Ocarina of Time 3D", "release_date": 1308700800, "types": ["steam"], "verified": true},
            {"id": 13136, "name": "The Legend of Zelda: Ocarina of Time", "release_date": 911347200, "types": [], "verified": false},
            {"id": 36097, "name": "The Legend of Zelda - Ocarina of Time", "types": [], "verified": true},
            {"name": "no id"}
        ]})
    }

    #[test]
    fn reads_the_games_of_a_search() {
        let games = search_games(&zelda_search());
        assert_eq!(games.len(), 3, "a row without id is skipped");
        assert_eq!(games[1].year, Some(1998));
        assert_eq!(games[2].year, None);
        assert!(search_games(&Value::Null).is_empty(), "404: no game");
    }

    #[test]
    fn takes_only_the_game_named_like_the_rom_a_verified_one_first() {
        let games = search_games(&zelda_search());
        let title = title_from_rom_name("Legend of Zelda, The - Ocarina of Time (Europe) (En,Fr,De)");
        assert_eq!(pick_game(&games, &title), Some(36097), "same name, verified");
        // "3D" is another game: no automatic cover from it.
        assert_eq!(pick_game(&games[..1], &title), None);
        assert_eq!(pick_game(&games, ""), None);
    }

    #[test]
    fn offers_the_rom_title_first_in_the_picker() {
        let games = picker_games(search_games(&zelda_search()), "The Legend of Zelda Ocarina of Time");
        let ids: Vec<i64> = games.iter().map(|g| g.id).collect();
        assert_eq!(ids, vec![36097, 13136, 5248155]);
    }

    #[test]
    fn keeps_grids_from_steamgriddb_hosts_only() {
        let body = json!({"success": true, "page": 0, "total": 3, "limit": 50, "data": [
            {"id": 1, "score": 3, "style": "alternate", "url": "https://cdn2.steamgriddb.com/grid/abc.png", "thumb": "https://cdn2.steamgriddb.com/thumb/abc.jpg"},
            {"id": 2, "url": "https://evil.example.com/grid.png", "thumb": "https://cdn2.steamgriddb.com/thumb/x.jpg"},
            {"id": 3, "url": "http://cdn2.steamgriddb.com/grid/plain-http.png"}
        ]});
        let grids = grid_choices(&body);
        assert_eq!(grids.len(), 1);
        assert_eq!(grids[0].url, "https://cdn2.steamgriddb.com/grid/abc.png");
        assert_eq!(grids[0].thumbnail.as_deref(), Some("https://cdn2.steamgriddb.com/thumb/abc.jpg"));
    }

    #[test]
    fn asks_for_portrait_still_safe_grids() {
        for part in ["600x900", "types=static", "nsfw=false", "humor=false", "epilepsy=false"] {
            assert!(GRID_QUERY.contains(part), "{}", part);
        }
        assert!(is_image_url("https://cdn2.steamgriddb.com/grid/a.png"));
        assert!(!is_image_url("https://steamgriddb.com.evil.net/a.png"));
        assert!(!is_image_url("not a url"));
        assert!(valid_key_format("0123456789abcdef0123456789abcdef"));
        assert!(!valid_key_format("short"));
        assert!(!valid_key_format("0123456789abcdef 0123456789abcdef"));
    }
}
