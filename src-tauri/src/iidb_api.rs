//! iiDB through its official developer API (https://iidb-api.iisu.network),
//! the only route iiDB's terms allow for automated access (section 8).
//!
//! - Each user brings their own API key (accounts.iisu.network), stored
//!   encrypted with DPAPI and sent only in the `X-API-Key` header, never to
//!   the image host, never logged. Without a key iiDB is simply skipped.
//! - Lookups are queued and sent by `/resolve/batch`, up to 25 games per
//!   request (counted as one request), paced to the user's tier
//!   (`X-IIDB-Requests-Per-Second-Limit`, 1/s by default), waiting
//!   `Retry-After` on 429. A 700-game library is ~28 requests.
//! - Only `match.status == "matched"` is accepted, with box art restricted to
//!   the ROM's console; ambiguous names ("needs_choice") fall through to the
//!   other sources rather than risk a wrong cover.
//! - Public apps must credit iiDB with a link to https://iidb.iisu.network:
//!   done in Settings.

use serde::Serialize;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};

use crate::push_log;

pub const API_BASE: &str = "https://iidb-api.iisu.network/api/v1";
pub const ASSET_HOST: &str = "https://assets.iisu.network/";
const MAX_BATCH: usize = 25;
/// How long the queue waits for more lookups before sending a batch.
const GATHER: Duration = Duration::from_millis(400);

/// iiDB console keys (from `/platforms`) for an EmuWorld console; the first
/// is the preferred one.
pub fn console_keys(console: &str) -> &'static [&'static str] {
    match console {
        "NES" => &["nes", "famicom", "fds"],
        "Super Nintendo" => &["snes", "sfc"],
        "Nintendo 64" => &["n64"],
        "Game Boy Advance" => &["gba", "gbc", "gb"],
        "Nintendo DS" => &["nds"],
        "Nintendo 3DS" => &["3ds"],
        "GameCube / Wii" => &["gamecube", "wii"],
        "Wii U" => &["wiiu"],
        "Nintendo Switch" => &["switch"],
        "Virtual Boy" => &["virtualboy"],
        "PlayStation 1" => &["ps1"],
        "PlayStation 2" => &["ps2"],
        "PlayStation 3" => &["ps3"],
        "PlayStation Portable" => &["psp"],
        "Xbox" => &["xbox"],
        "Xbox 360" => &["xbox360"],
        "Master System" => &["mastersystem"],
        "Mega Drive" => &["megadrive", "genesis"],
        "Sega CD" => &["segacd", "megacd"],
        "Sega 32X" => &["sega32x", "sega32xna", "sega32xjp"],
        "Saturn" => &["saturn"],
        "Dreamcast" => &["dreamcast"],
        "Game Gear" => &["gamegear"],
        "Atari 2600" => &["atari2600"],
        "Atari 5200" => &["atari5200"],
        "Atari 7800" => &["atari7800"],
        "Jaguar" => &["atarijaguar"],
        "Lynx" => &["atarilynx"],
        "Neo-Geo" => &["neogeo"],
        "Arcade" => &["arcade"],
        "PC Engine" => &["pcengine", "tg16"],
        "TurboGrafx-16" => &["tg16", "pcengine"],
        "TurboGrafx-CD" => &["tg_cd", "pcenginecd"],
        "CD-i" => &["cdimono1"],
        "WonderSwan" => &["wonderswan", "wonderswancolor"],
        _ => &[],
    }
}

fn preferred_region(rom_name: &str) -> &'static str {
    let lower = rom_name.to_lowercase();
    if ["(europe", "(france", "(germany", "(spain", "(italy", "(uk"].iter().any(|t| lower.contains(t)) {
        "Europe"
    } else if lower.contains("(japan") {
        "Japan"
    } else if lower.contains("(usa") {
        "USA"
    } else {
        "World"
    }
}

/// One `/resolve` lookup asking for the recommended box art.
pub fn build_lookup(rom_name: &str, console: &str) -> Option<Value> {
    let keys = console_keys(console);
    let title = crate::iidb::title_from_rom_name(rom_name);
    if keys.is_empty() || title.chars().count() < 2 {
        return None;
    }
    let mut lookup = json!({
        "title": title,
        "preferred_console": keys[0],
        "preferred_region": preferred_region(rom_name),
        "artwork": "recommended",
        "slots": ["boxart"],
        "candidates_per_slot": 3,
    });
    // Box art from that console only, when the console is unambiguous.
    if keys.len() == 1 {
        lookup["artwork_filters"] = json!({ "console_id": keys[0] });
    }
    Some(lookup)
}

/// Box art URL from one `/resolve` result: only a confident match, on the
/// ROM's console, served by iiDB's asset host.
pub fn pick_boxart(result: &Value, console: &str) -> Option<String> {
    if result["match"]["status"].as_str() != Some("matched") {
        return None;
    }
    let keys = console_keys(console);
    result["artwork"]["boxart"].as_array()?.iter().find_map(|asset| {
        let url = asset["url"].as_str()?;
        let console_ok = asset["console_id"].as_str().map_or(true, |c| keys.contains(&c));
        (url.starts_with(ASSET_HOST) && console_ok).then(|| url.to_string())
    })
}

/// Pause before the next request so the user's per-second allowance holds.
pub fn wait_before_next(last_request: Option<Instant>, requests_per_second: f64, now: Instant) -> Duration {
    let interval = Duration::from_secs_f64(1.0 / requests_per_second.clamp(0.1, 50.0));
    match last_request {
        Some(last) => interval.saturating_sub(now.saturating_duration_since(last)),
        None => Duration::ZERO,
    }
}

/// `Retry-After` in seconds (bounded), defaulting to 2 s.
pub fn retry_after(value: Option<&str>) -> Duration {
    let secs = value.and_then(|v| v.trim().parse::<f64>().ok()).unwrap_or(2.0).clamp(0.5, 120.0);
    Duration::from_secs_f64(secs)
}

/// A pasted key: no spaces or control characters, reasonable length.
pub fn valid_key_format(key: &str) -> bool {
    (16..=256).contains(&key.len()) && key.chars().all(|c| c.is_ascii_graphic())
}

// ---- key storage ----------------------------------------------------------

fn key_path() -> PathBuf {
    crate::emuworld_base_dir().join("iidb_key.dat")
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

fn http() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(format!("EmuWorld/{} (+https://github.com/fj123u/EmuWorld)", env!("CARGO_PKG_VERSION")))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    })
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct IidbStatus {
    pub configured: bool,
    pub username: Option<String>,
    pub tier: Option<String>,
    pub requests_per_second: Option<f64>,
}

fn status_from_access(access: &Value) -> IidbStatus {
    IidbStatus {
        configured: true,
        username: access["username"].as_str().map(str::to_string),
        tier: access["tier"]["code"].as_str().map(str::to_string),
        requests_per_second: access["tier"]["requests_per_second"].as_f64(),
    }
}

async fn fetch_access(key: &str) -> Result<Value, String> {
    let resp = http()
        .get(format!("{}/me/access", API_BASE))
        .header("X-API-Key", key)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("iiDB injoignable: {}", e))?;
    match resp.status().as_u16() {
        200 => resp.json().await.map_err(|e| e.to_string()),
        401 | 403 => Err("Clé API iiDB refusée : vérifie-la sur accounts.iisu.network".to_string()),
        s => Err(format!("iiDB a répondu HTTP {}", s)),
    }
}

/// Checks the key with `/me/access`, then stores it encrypted.
#[tauri::command]
pub async fn set_iidb_api_key(key: String) -> Result<IidbStatus, String> {
    let key = key.trim().to_string();
    push_log("INFO", "iiDB: vérification d'une clé API");
    if !valid_key_format(&key) {
        return Err("Clé API iiDB au mauvais format".to_string());
    }
    let access = fetch_access(&key).await.map_err(|e| {
        push_log("WARN", &format!("iiDB: clé non enregistrée ({})", e));
        e
    })?;
    crate::dpapi::encrypt_and_write(&key_path(), key.as_bytes())?;
    set_cached_key(Some(key));
    let status = status_from_access(&access);
    push_log("INFO", &format!("iiDB: clé enregistrée (palier {:?}, {:?} req/s)", status.tier, status.requests_per_second));
    Ok(status)
}

#[tauri::command]
pub async fn get_iidb_status() -> IidbStatus {
    match api_key() {
        None => IidbStatus::default(),
        Some(key) => match fetch_access(&key).await {
            Ok(access) => status_from_access(&access),
            // Offline or refused: still configured, details unknown.
            Err(_) => IidbStatus { configured: true, ..Default::default() },
        },
    }
}

#[tauri::command]
pub fn clear_iidb_api_key() -> Result<(), String> {
    let path = key_path();
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    set_cached_key(None);
    push_log("INFO", "iiDB: clé API retirée");
    Ok(())
}

// ---- batched lookups --------------------------------------------------------

struct Job {
    lookup: Value,
    console: String,
    reply: oneshot::Sender<Option<String>>,
}

fn queue() -> &'static mpsc::UnboundedSender<Job> {
    static TX: OnceLock<mpsc::UnboundedSender<Job>> = OnceLock::new();
    TX.get_or_init(|| {
        let (tx, rx) = mpsc::unbounded_channel();
        tauri::async_runtime::spawn(worker(rx));
        tx
    })
}

/// Box art URL for a ROM through the official API, or `None` (no key, no
/// confident match, or iiDB unavailable): the cascade then continues.
pub async fn find_boxart(rom_name: &str, console: &str) -> Option<String> {
    api_key()?;
    let lookup = build_lookup(rom_name, console)?;
    let (reply, answer) = oneshot::channel();
    queue().send(Job { lookup, console: console.to_string(), reply }).ok()?;
    answer.await.ok().flatten()
}

async fn worker(mut rx: mpsc::UnboundedReceiver<Job>) {
    let mut last_request: Option<Instant> = None;
    let mut rps = 1.0f64;
    while let Some(first) = rx.recv().await {
        let mut jobs = vec![first];
        let deadline = tokio::time::Instant::now() + GATHER;
        while jobs.len() < MAX_BATCH {
            match tokio::time::timeout_at(deadline, rx.recv()).await {
                Ok(Some(job)) => jobs.push(job),
                _ => break,
            }
        }
        let Some(key) = api_key() else {
            jobs.into_iter().for_each(|j| { let _ = j.reply.send(None); });
            continue;
        };
        let body = json!({ "lookups": jobs.iter().map(|j| j.lookup.clone()).collect::<Vec<_>>() });
        let mut results: Option<Vec<Value>> = None;
        for attempt in 0..4 {
            tokio::time::sleep(wait_before_next(last_request, rps, Instant::now())).await;
            last_request = Some(Instant::now());
            let resp = http()
                .post(format!("{}/resolve/batch", API_BASE))
                .header("X-API-Key", &key)
                .header("Accept", "application/json")
                .json(&body)
                .send()
                .await;
            let resp = match resp {
                Ok(r) => r,
                Err(e) => {
                    push_log("WARN", &format!("iiDB: requête échouée ({}), essai {}", e, attempt + 1));
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    continue;
                }
            };
            if let Some(limit) = resp.headers().get("X-IIDB-Requests-Per-Second-Limit").and_then(|v| v.to_str().ok()).and_then(|v| v.parse::<f64>().ok()) {
                rps = limit;
            }
            match resp.status().as_u16() {
                200 => {
                    results = resp.json::<Value>().await.ok().and_then(|v| v["results"].as_array().cloned());
                    break;
                }
                429 => {
                    let wait = retry_after(resp.headers().get("Retry-After").and_then(|v| v.to_str().ok()));
                    push_log("WARN", &format!("iiDB: limite atteinte, attente de {:?}", wait));
                    tokio::time::sleep(wait).await;
                }
                401 | 403 => {
                    push_log("WARN", "iiDB: clé API refusée, iiDB ignoré jusqu'à une nouvelle clé");
                    set_cached_key(None);
                    break;
                }
                s if s >= 500 => tokio::time::sleep(Duration::from_secs(2 * (attempt + 1) as u64)).await,
                s => {
                    push_log("WARN", &format!("iiDB: lot refusé (HTTP {})", s));
                    break;
                }
            }
        }
        let results = results.unwrap_or_default();
        let found = jobs
            .into_iter()
            .enumerate()
            .map(|(i, job)| {
                let url = results.get(i).and_then(|r| pick_boxart(r, &job.console));
                let hit = url.is_some();
                let _ = job.reply.send(url);
                hit
            })
            .filter(|h| *h)
            .count();
        push_log("INFO", &format!("iiDB: lot de {} jeux, {} covers trouvées", results.len().max(1), found));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_catalog_console_has_iidb_console_keys() {
        let missing: Vec<String> = crate::emulators::get_catalog()
            .into_iter()
            .map(|e| e.console)
            .filter(|c| c != "Multi-System" && console_keys(c).is_empty())
            .collect();
        assert!(missing.is_empty(), "add them to iidb_api::console_keys: {missing:?}");
    }

    #[test]
    fn builds_a_boxart_lookup_for_the_rom() {
        let l = build_lookup("Super Mario Bros. 3 (Europe)", "NES").unwrap();
        assert_eq!(l["title"], "Super Mario Bros. 3");
        assert_eq!(l["preferred_console"], "nes");
        assert_eq!(l["preferred_region"], "Europe");
        assert_eq!(l["artwork"], "recommended");
        assert_eq!(l["slots"], json!(["boxart"]));
        assert!(l.get("artwork_filters").is_none(), "NES spans nes/famicom/fds");
        let switch = build_lookup("Celeste [01002B30028F6000][v0]", "Nintendo Switch").unwrap();
        assert_eq!(switch["artwork_filters"]["console_id"], "switch");
        assert!(build_lookup("Game", "Multi-System").is_none());
    }

    #[test]
    fn accepts_only_a_confident_match_on_the_right_console() {
        let ok = json!({"match": {"status": "matched"}, "artwork": {"boxart": [
            {"url": "https://evil.example/x.png", "console_id": "nes"},
            {"url": "https://assets.iisu.network/a.png", "console_id": "snes"},
            {"url": "https://assets.iisu.network/b.png", "console_id": "nes"}
        ]}});
        assert_eq!(pick_boxart(&ok, "NES").as_deref(), Some("https://assets.iisu.network/b.png"));
        let choice = json!({"match": {"status": "needs_choice"}, "artwork": {"boxart": [{"url": "https://assets.iisu.network/c.png"}]}});
        assert_eq!(pick_boxart(&choice, "NES"), None, "never guess between candidates");
        let error = json!({"status": 422, "errors": []});
        assert_eq!(pick_boxart(&error, "NES"), None);
    }

    #[test]
    fn paces_requests_to_the_tier_and_honours_retry_after() {
        let now = Instant::now();
        assert_eq!(wait_before_next(None, 1.0, now), Duration::ZERO);
        assert_eq!(wait_before_next(Some(now), 1.0, now), Duration::from_secs(1));
        assert_eq!(wait_before_next(Some(now), 4.0, now + Duration::from_millis(100)), Duration::from_millis(150));
        assert_eq!(wait_before_next(Some(now), 2.0, now + Duration::from_secs(3)), Duration::ZERO);
        assert_eq!(retry_after(Some("3")), Duration::from_secs(3));
        assert_eq!(retry_after(Some("garbage")), Duration::from_secs(2));
        assert_eq!(retry_after(Some("99999")), Duration::from_secs(120));
    }

    #[test]
    fn rejects_malformed_keys() {
        assert!(valid_key_format("iidb_live_0123456789abcdef"));
        assert!(!valid_key_format("short"));
        assert!(!valid_key_format("has space 0123456789abcdef"));
        assert!(!valid_key_format(&"x".repeat(300)));
    }
}
