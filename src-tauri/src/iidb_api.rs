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
//! - Switch ROMs also send their Title ID (`nintendo-switch.title-id`).
//! - Only `match.status == "matched"` is accepted, preferring box art from
//!   the ROM's console; ambiguous names ("needs_choice") fall through to the
//!   other sources rather than risk a wrong cover. Each batch logs what
//!   happened to its lookups (matched, not_found, invalid…), never the key.
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
    // The library can carry short names ("SNES", "PSP", "Genesis"...): map
    // them to catalog names the way the ROM scan does.
    match catalog_console_keys(console) {
        [] => catalog_console_keys(&crate::normalize_console_folder(console)),
        keys => keys,
    }
}

fn catalog_console_keys(console: &str) -> &'static [&'static str] {
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
        "candidates_per_slot": 6,
    });
    // No `artwork_filters.console_id`: iiDB wants the console ids found on
    // its assets, which are not documented to equal `/platforms` keys, and a
    // wrong one filters out every cover. The console is preferred in
    // `pick_boxart` instead.
    if console == "Nintendo Switch" {
        if let Some(id) = switch_title_id(rom_name) {
            lookup["identity"] = json!({ "kind": "title_id", "value": id, "namespace": "nintendo-switch.title-id" });
        }
    }
    Some(lookup)
}

/// Switch Title ID written in the ROM name ("Celeste [01002B30028F6000][v0]").
pub fn switch_title_id(rom_name: &str) -> Option<String> {
    rom_name.split(['[', ']', '(', ')', ' ']).find_map(|token| {
        let ok = token.len() == 16 && token.starts_with("01") && token.chars().all(|c| c.is_ascii_hexdigit());
        ok.then(|| token.to_uppercase())
    })
}

/// Box art URL from one `/resolve` result: only a confident match, served by
/// iiDB's asset host, from the ROM's console when iiDB has one for it.
pub fn pick_boxart(result: &Value, console: &str) -> Option<String> {
    if result["match"]["status"].as_str() != Some("matched") {
        return None;
    }
    pick_from_artwork(result, console)
}

/// First usable box art of a result's `artwork.boxart` list (resolve or
/// `/assets/batch`), the ROM's console first.
pub fn pick_from_artwork(result: &Value, console: &str) -> Option<String> {
    let keys = console_keys(console);
    let assets: Vec<(&str, Option<&str>)> = result["artwork"]["boxart"]
        .as_array()?
        .iter()
        .filter_map(|a| Some((a["url"].as_str()?, a["console_id"].as_str())))
        .filter(|(url, _)| url.starts_with(ASSET_HOST))
        .collect();
    let on_console = |c: Option<&str>| c.is_some_and(|c| keys.iter().any(|k| k.eq_ignore_ascii_case(c)));
    assets
        .iter()
        .find(|(_, c)| on_console(*c))
        .or_else(|| assets.first())
        .map(|(url, _)| url.to_string())
}

/// For a `needs_choice` answer, the one candidate EmuWorld can stand behind:
/// the only one iiDB matched by identifier (Switch Title ID), else the only
/// one whose name is the ROM's title (same words, ignoring case, accents and
/// punctuation). Several equal names stay undecided: no guessing.
pub fn choose_candidate(result: &Value, title: &str) -> Option<i64> {
    if result["match"]["status"].as_str() != Some("needs_choice") {
        return None;
    }
    let candidates = result["candidates"].as_array()?;
    let only = |hits: Vec<&Value>| if hits.len() == 1 { hits[0]["id"].as_i64() } else { None };
    let by_identity: Vec<&Value> = candidates.iter().filter(|c| c["reason"].as_str() == Some("identity")).collect();
    if !by_identity.is_empty() {
        return only(by_identity);
    }
    let wanted = crate::iidb::normalize_title(title);
    if wanted.is_empty() {
        return None;
    }
    only(candidates
        .iter()
        .filter(|c| c["name"].as_str().is_some_and(|n| crate::iidb::normalize_title(n) == wanted))
        .collect())
}

/// `/assets/batch` row for a chosen iiDB parent.
fn parent_artwork_lookup(parent_id: i64, lookup: &Value) -> Value {
    json!({
        "parent_id": parent_id,
        "slots": ["boxart"],
        "candidates_per_slot": 6,
        "preferred_region": lookup["preferred_region"].as_str().unwrap_or("World"),
    })
}

/// One word on what happened to a lookup, for the batch log line.
pub fn outcome(result: Option<&Value>) -> String {
    let Some(r) = result else { return "absent".into() };
    if r["status"].as_str() == Some("invalid_request") {
        let why = r["errors"][0]["msg"].as_str().or(r["errors"][0]["message"].as_str()).unwrap_or("?");
        return format!("invalide ({})", why.chars().take(60).collect::<String>());
    }
    match r["match"]["status"].as_str().unwrap_or("?") {
        "matched" => match r["missing_artwork"]["boxart"].as_str() {
            Some(reason) => format!("trouvé sans boxart ({})", reason),
            None => format!("trouvé ({})", r["artwork_status"].as_str().unwrap_or("?")),
        },
        other => other.to_string(),
    }
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

/// Pacing shared by every request of this user.
struct Pace {
    last_request: Option<Instant>,
    rps: f64,
}

/// POST a batch to the official API, paced to the user's tier, waiting
/// `Retry-After` on 429 and retrying 5xx. `None` when it never completed.
async fn post_batch(path: &str, body: &Value, key: &str, pace: &mut Pace) -> Option<Vec<Value>> {
    for attempt in 0..4 {
        tokio::time::sleep(wait_before_next(pace.last_request, pace.rps, Instant::now())).await;
        pace.last_request = Some(Instant::now());
        let resp = http()
            .post(format!("{}{}", API_BASE, path))
            .header("X-API-Key", key)
            .header("Accept", "application/json")
            .json(body)
            .send()
            .await;
        let resp = match resp {
            Ok(r) => r,
            Err(e) => {
                push_log("WARN", &format!("iiDB: requête {} échouée ({}), essai {}", path, e, attempt + 1));
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }
        };
        if let Some(limit) = resp.headers().get("X-IIDB-Requests-Per-Second-Limit").and_then(|v| v.to_str().ok()).and_then(|v| v.parse::<f64>().ok()) {
            pace.rps = limit;
        }
        match resp.status().as_u16() {
            200 => return resp.json::<Value>().await.ok().and_then(|v| v["results"].as_array().cloned()),
            429 => {
                let wait = retry_after(resp.headers().get("Retry-After").and_then(|v| v.to_str().ok()));
                push_log("WARN", &format!("iiDB: limite atteinte, attente de {:?}", wait));
                tokio::time::sleep(wait).await;
            }
            401 | 403 => {
                push_log("WARN", "iiDB: clé API refusée, iiDB ignoré jusqu'à une nouvelle clé");
                set_cached_key(None);
                return None;
            }
            s if s >= 500 => tokio::time::sleep(Duration::from_secs(2 * (attempt + 1) as u64)).await,
            s => {
                push_log("WARN", &format!("iiDB: lot {} refusé (HTTP {})", path, s));
                return None;
            }
        }
    }
    None
}

async fn worker(mut rx: mpsc::UnboundedReceiver<Job>) {
    let mut pace = Pace { last_request: None, rps: 1.0 };
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
        let results = post_batch("/resolve/batch", &body, &key, &mut pace).await.unwrap_or_default();

        let mut urls: Vec<Option<String>> = Vec::with_capacity(jobs.len());
        let mut outcomes: std::collections::BTreeMap<String, usize> = Default::default();
        let mut chosen: Vec<(usize, i64)> = Vec::new();
        for (i, job) in jobs.iter().enumerate() {
            let result = results.get(i);
            urls.push(result.and_then(|r| pick_boxart(r, &job.console)));
            if let Some(id) = result.and_then(|r| choose_candidate(r, job.lookup["title"].as_str().unwrap_or(""))) {
                chosen.push((i, id));
            } else {
                *outcomes.entry(outcome(result)).or_default() += 1;
            }
        }

        // needs_choice with one exact candidate: its box art by parent id.
        if !chosen.is_empty() {
            let body = json!({ "lookups": chosen.iter().map(|(i, id)| parent_artwork_lookup(*id, &jobs[*i].lookup)).collect::<Vec<_>>() });
            let art = post_batch("/assets/batch", &body, &key, &mut pace).await.unwrap_or_default();
            for (n, (i, _)) in chosen.iter().enumerate() {
                let url = art.get(n).and_then(|r| pick_from_artwork(r, &jobs[*i].console));
                let label = if url.is_some() { "choisi (nom exact)" } else { "choisi, sans boxart" };
                *outcomes.entry(label.to_string()).or_default() += 1;
                urls[*i] = url;
            }
        }

        let count = jobs.len();
        let found = urls.iter().filter(|u| u.is_some()).count();
        for (job, url) in jobs.into_iter().zip(urls) {
            let _ = job.reply.send(url);
        }
        let detail = outcomes.iter().map(|(k, n)| format!("{} ×{}", k, n)).collect::<Vec<_>>().join(", ");
        push_log("INFO", &format!("iiDB: lot de {} jeux, {} covers trouvées [{}]", count, found, detail));
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
        assert!(l.get("artwork_filters").is_none());
        assert!(l.get("identity").is_none());
        let switch = build_lookup("Celeste [01002b30028f6000][v0]", "Nintendo Switch").unwrap();
        assert!(switch.get("artwork_filters").is_none(), "a guessed console_id filtered out every cover");
        assert_eq!(switch["identity"], json!({"kind": "title_id", "value": "01002B30028F6000", "namespace": "nintendo-switch.title-id"}));
        assert_eq!(switch["title"], "Celeste");
        assert!(build_lookup("Game", "Multi-System").is_none());
    }

    #[test]
    fn reads_switch_title_ids() {
        assert_eq!(switch_title_id("Pokemon Scarlet [0100A3D008C5C000]").as_deref(), Some("0100A3D008C5C000"));
        assert_eq!(switch_title_id("Zelda (01007EF00011E000) (nsw2u.com)").as_deref(), Some("01007EF00011E000"));
        assert_eq!(switch_title_id("SR ROM [0400380020056000]"), None, "not a Switch application id");
        assert_eq!(switch_title_id("The Legend of Zelda Echoes of Wisdom"), None);
    }

    #[test]
    fn accepts_only_a_confident_match_on_the_right_console() {
        let ok = json!({"match": {"status": "matched"}, "artwork": {"boxart": [
            {"url": "https://evil.example/x.png", "console_id": "nes"},
            {"url": "https://assets.iisu.network/a.png", "console_id": "snes"},
            {"url": "https://assets.iisu.network/b.png", "console_id": "NES"}
        ]}});
        assert_eq!(pick_boxart(&ok, "NES").as_deref(), Some("https://assets.iisu.network/b.png"));
        let other_console = json!({"match": {"status": "matched"}, "artwork": {"boxart": [
            {"url": "https://assets.iisu.network/only.png", "console_id": "nintendo-switch"}
        ]}});
        assert_eq!(pick_boxart(&other_console, "Nintendo Switch").as_deref(), Some("https://assets.iisu.network/only.png"),
            "the game matched: its box art beats no cover");
        let choice = json!({"match": {"status": "needs_choice"}, "artwork": {"boxart": [{"url": "https://assets.iisu.network/c.png"}]}});
        assert_eq!(pick_boxart(&choice, "NES"), None, "never guess between candidates");
        let error = json!({"status": 422, "errors": []});
        assert_eq!(pick_boxart(&error, "NES"), None);
    }

    #[test]
    fn understands_the_short_console_names_of_the_library() {
        assert_eq!(console_keys("SNES"), &["snes", "sfc"]);
        assert_eq!(console_keys("PSP"), &["psp"]);
        assert_eq!(console_keys("Genesis"), &["megadrive", "genesis"]);
        assert_eq!(console_keys("GameCube"), &["gamecube", "wii"]);
        assert!(build_lookup("Super Mario World", "SNES").is_some(), "SNES games were never sent to iiDB");
        assert!(console_keys("Not a console").is_empty());
    }

    #[test]
    fn picks_the_one_exact_candidate_of_a_needs_choice() {
        let answer = |cands: Value| json!({"match": {"status": "needs_choice"}, "candidates": cands});
        let r = answer(json!([
            {"id": 1, "name": "Pokémon Scarlet and Violet Double Pack", "reason": "name_prefix"},
            {"id": 2, "name": "Pokémon Scarlet", "reason": "fuzzy"}
        ]));
        assert_eq!(choose_candidate(&r, "Pokemon Scarlet"), Some(2), "same words, accents ignored");
        let r = answer(json!([
            {"id": 7, "name": "Celeste", "reason": "identity"},
            {"id": 8, "name": "Celeste", "reason": "name_exact"}
        ]));
        assert_eq!(choose_candidate(&r, "Celeste"), Some(7), "the Title ID decides");
        let r = answer(json!([
            {"id": 3, "name": "Tomb Raider", "reason": "name_exact"},
            {"id": 4, "name": "Tomb Raider", "reason": "name_exact"}
        ]));
        assert_eq!(choose_candidate(&r, "Tomb Raider"), None, "two equal names: never guess");
        let r = answer(json!([{"id": 5, "name": "Super Mario Party", "reason": "name_prefix"}]));
        assert_eq!(choose_candidate(&r, "Super Mario Party Jamboree"), None);
        let matched = json!({"match": {"status": "matched"}, "candidates": [{"id": 6, "name": "Celeste"}]});
        assert_eq!(choose_candidate(&matched, "Celeste"), None);
    }

    #[test]
    fn reads_box_art_from_an_assets_batch_row() {
        let row = json!({"parent_id": 2, "artwork_status": "available", "artwork": {"boxart": [
            {"url": "https://assets.iisu.network/p.png", "console_id": "switch"}
        ]}});
        assert_eq!(pick_from_artwork(&row, "Nintendo Switch").as_deref(), Some("https://assets.iisu.network/p.png"));
        assert_eq!(parent_artwork_lookup(2, &json!({"preferred_region": "Europe"})),
            json!({"parent_id": 2, "slots": ["boxart"], "candidates_per_slot": 6, "preferred_region": "Europe"}));
    }

    #[test]
    fn describes_each_lookup_outcome_for_the_log() {
        assert_eq!(outcome(None), "absent");
        assert_eq!(outcome(Some(&json!({"status": "invalid_request", "errors": [{"msg": "Extra inputs are not permitted"}]}))), "invalide (Extra inputs are not permitted)");
        assert_eq!(outcome(Some(&json!({"match": {"status": "not_found"}, "artwork_status": "not_resolved"}))), "not_found");
        assert_eq!(outcome(Some(&json!({"match": {"status": "matched"}, "artwork_status": "empty", "missing_artwork": {"boxart": "filtered"}}))), "trouvé sans boxart (filtered)");
        assert_eq!(outcome(Some(&json!({"match": {"status": "matched"}, "artwork_status": "available"}))), "trouvé (available)");
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
