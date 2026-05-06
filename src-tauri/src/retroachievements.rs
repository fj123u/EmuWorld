use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct RAConfig {
    pub username: String,
    pub api_key: String,
    #[serde(default)]
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RAGameAchievement {
    pub id: u64,
    pub title: String,
    pub description: String,
    pub points: u32,
    pub badge_name: String,
    pub date_earned: Option<String>,
    pub date_earned_hardcore: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RAGameInfo {
    pub game_id: u64,
    pub title: String,
    pub console_name: String,
    pub image_icon: String,
    pub num_achievements: u32,
    pub achievements: Vec<RAGameAchievement>,
    pub num_earned: u32,
    pub num_earned_hardcore: u32,
}

fn config_path() -> PathBuf {
    let base = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("EmuWorld").join("ra_config.json")
}

pub fn load_config() -> RAConfig {
    let path = config_path();
    if let Ok(data) = fs::read_to_string(&path) {
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        RAConfig::default()
    }
}

pub fn save_config(config: &RAConfig) -> Result<(), String> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())
}

pub fn console_to_ra_id(console: &str) -> Option<u32> {
    match console {
        "NES" => Some(7),
        "Super Nintendo" => Some(3),
        "Nintendo 64" => Some(2),
        "Game Boy Advance" => Some(5),
        "Game Boy" | "Game Boy Color" => Some(4),
        "Nintendo DS" => Some(18),
        "Virtual Boy" => Some(28),
        "PlayStation 1" => Some(12),
        "PlayStation 2" => Some(21),
        "PlayStation Portable" => Some(41),
        "Mega Drive" => Some(1),
        "Master System" => Some(11),
        "Game Gear" => Some(15),
        "Saturn" => Some(39),
        "Dreamcast" => Some(40),
        "Arcade" => Some(27),
        "Neo-Geo" => Some(27),
        "PC Engine" => Some(8),
        "Atari 2600" => Some(25),
        "WonderSwan" => Some(53),
        _ => None,
    }
}

pub async fn search_game(game_name: &str, console: &str, api_key: &str) -> Result<Option<u64>, String> {
    let console_id = console_to_ra_id(console)
        .ok_or_else(|| format!("Console '{}' not supported by RetroAchievements", console))?;

    let client = reqwest::Client::builder()
        .user_agent("EmuWorld/0.2.0")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!(
        "https://retroachievements.org/API/API_GetGameList.php?i={}&y={}&f=1&h=1",
        console_id, api_key
    );

    let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("RA API returned {}", resp.status()));
    }

    let games: Vec<serde_json::Value> = resp.json().await.map_err(|e| e.to_string())?;

    let cleaned = clean_name(game_name);
    let mut best_match: Option<(u64, usize)> = None;

    for game in &games {
        let title = game["Title"].as_str().unwrap_or("");
        let id = game["ID"].as_u64().unwrap_or(0);
        if id == 0 { continue; }

        let cleaned_title = clean_name(title);
        if cleaned_title == cleaned {
            return Ok(Some(id));
        }

        if cleaned_title.contains(&cleaned) || cleaned.contains(&cleaned_title) {
            let score = cleaned_title.len().abs_diff(cleaned.len());
            if best_match.is_none() || score < best_match.unwrap().1 {
                best_match = Some((id, score));
            }
        }
    }

    Ok(best_match.map(|(id, _)| id))
}

pub async fn get_game_progress(game_id: u64, username: &str, api_key: &str) -> Result<RAGameInfo, String> {
    let client = reqwest::Client::builder()
        .user_agent("EmuWorld/0.2.0")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!(
        "https://retroachievements.org/API/API_GetGameInfoAndUserProgress.php?g={}&u={}&y={}",
        game_id, username, api_key
    );

    let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("RA API returned {}", resp.status()));
    }

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    let title = json["Title"].as_str().unwrap_or("Unknown").to_string();
    let console_name = json["ConsoleName"].as_str().unwrap_or("").to_string();
    let image_icon = json["ImageIcon"].as_str().unwrap_or("").to_string();
    let num_achievements = json["NumAchievements"].as_u64().unwrap_or(0) as u32;

    let mut achievements = Vec::new();
    let mut num_earned = 0u32;
    let mut num_earned_hardcore = 0u32;

    if let Some(achs) = json["Achievements"].as_object() {
        for (_, ach) in achs {
            let date_earned = ach["DateEarned"].as_str()
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string());
            let date_earned_hardcore = ach["DateEarnedHardcore"].as_str()
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string());

            if date_earned.is_some() { num_earned += 1; }
            if date_earned_hardcore.is_some() { num_earned_hardcore += 1; }

            achievements.push(RAGameAchievement {
                id: ach["ID"].as_u64().unwrap_or(0),
                title: ach["Title"].as_str().unwrap_or("").to_string(),
                description: ach["Description"].as_str().unwrap_or("").to_string(),
                points: ach["Points"].as_u64().unwrap_or(0) as u32,
                badge_name: ach["BadgeName"].as_str().unwrap_or("").to_string(),
                date_earned,
                date_earned_hardcore,
            });
        }
    }

    achievements.sort_by_key(|a| a.id);

    Ok(RAGameInfo {
        game_id,
        title,
        console_name,
        image_icon,
        num_achievements,
        achievements,
        num_earned,
        num_earned_hardcore,
    })
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RACompletedGame {
    pub game_id: u64,
    pub title: String,
    pub console_name: String,
    pub image_icon: String,
    pub max_possible: u32,
    pub num_awarded: u32,
    pub hardcore_mode: bool,
}

pub async fn get_completed_games(username: &str, api_key: &str) -> Result<Vec<RACompletedGame>, String> {
    let client = reqwest::Client::builder()
        .user_agent("EmuWorld/0.2.0")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!(
        "https://retroachievements.org/API/API_GetUserCompletedGames.php?u={}&y={}",
        username, api_key
    );

    let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("RA API returned {}", resp.status()));
    }

    let games: Vec<serde_json::Value> = resp.json().await.map_err(|e| e.to_string())?;

    let result: Vec<RACompletedGame> = games.iter().map(|g| {
        RACompletedGame {
            game_id: g["GameID"].as_u64().unwrap_or(0),
            title: g["Title"].as_str().unwrap_or("").to_string(),
            console_name: g["ConsoleName"].as_str().unwrap_or("").to_string(),
            image_icon: g["ImageIcon"].as_str().unwrap_or("").to_string(),
            max_possible: g["MaxPossible"].as_u64().unwrap_or(0) as u32,
            num_awarded: g["NumAwarded"].as_u64().unwrap_or(0) as u32,
            hardcore_mode: g["HardcoreMode"].as_str().map(|s| s == "1").unwrap_or(false)
                || g["HardcoreMode"].as_u64().map(|n| n == 1).unwrap_or(false),
        }
    }).collect();

    Ok(result)
}

pub async fn login_and_get_token(username: &str, password: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .user_agent("EmuWorld/0.2.0")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!(
        "https://retroachievements.org/dorequest.php?r=login&u={}&p={}",
        urlencoding::encode(username),
        urlencoding::encode(password)
    );

    let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("RA login returned {}", resp.status()));
    }

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    if json["Success"].as_bool() != Some(true) {
        return Err(json["Error"].as_str().unwrap_or("Login failed").to_string());
    }

    json["Token"].as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "No token in response".to_string())
}

pub fn inject_ra_config_into_emulators(emulators_dir: &str, username: &str, token: &str) -> Vec<String> {
    let base = PathBuf::from(emulators_dir);
    let mut configured = Vec::new();

    // RetroArch (and all retroarch-* variants share the same install)
    let retroarch_cfg = base.join("retroarch").join("retroarch.cfg");
    if let Some(parent) = retroarch_cfg.parent() {
        if parent.exists() {
            let cfg_content = fs::read_to_string(&retroarch_cfg).unwrap_or_default();
            let new_cfg = inject_retroarch_cheevos(&cfg_content, username, token);
            if fs::write(&retroarch_cfg, new_cfg).is_ok() {
                configured.push("RetroArch".to_string());
            }
        }
    }

    // DuckStation — settings.ini
    let duck_ini = base.join("duckstation").join("settings.ini");
    if let Some(parent) = duck_ini.parent() {
        if parent.exists() {
            let content = fs::read_to_string(&duck_ini).unwrap_or_default();
            let new_content = inject_duckstation_cheevos(&content, username, token);
            if fs::write(&duck_ini, new_content).is_ok() {
                configured.push("DuckStation".to_string());
            }
        }
    }

    // PCSX2 — inis/PCSX2.ini or portable.ini
    let pcsx2_ini = base.join("pcsx2").join("inis").join("PCSX2.ini");
    if let Some(parent) = pcsx2_ini.parent() {
        if parent.exists() {
            let content = fs::read_to_string(&pcsx2_ini).unwrap_or_default();
            let new_content = inject_ini_section_cheevos(&content, username, token);
            if fs::write(&pcsx2_ini, new_content).is_ok() {
                configured.push("PCSX2".to_string());
            }
        }
    }

    // Dolphin — User/Config/Dolphin.ini
    let dolphin_ini = base.join("dolphin").join("User").join("Config").join("Dolphin.ini");
    if let Some(parent) = dolphin_ini.parent() {
        if parent.exists() {
            let content = fs::read_to_string(&dolphin_ini).unwrap_or_default();
            let new_content = inject_ini_section_cheevos(&content, username, token);
            if fs::write(&dolphin_ini, new_content).is_ok() {
                configured.push("Dolphin".to_string());
            }
        }
    }

    // PPSSPP — memstick/PSP/SYSTEM/ppsspp.ini
    let ppsspp_ini = base.join("ppsspp").join("memstick").join("PSP").join("SYSTEM").join("ppsspp.ini");
    if !ppsspp_ini.exists() {
        // Try alternate location
        let alt = base.join("ppsspp").join("ppsspp.ini");
        if alt.exists() {
            let content = fs::read_to_string(&alt).unwrap_or_default();
            let new_content = inject_ini_section_cheevos(&content, username, token);
            if fs::write(&alt, new_content).is_ok() {
                configured.push("PPSSPP".to_string());
            }
        }
    } else if let Some(parent) = ppsspp_ini.parent() {
        if parent.exists() {
            let content = fs::read_to_string(&ppsspp_ini).unwrap_or_default();
            let new_content = inject_ini_section_cheevos(&content, username, token);
            if fs::write(&ppsspp_ini, new_content).is_ok() {
                configured.push("PPSSPP".to_string());
            }
        }
    }

    configured
}

pub fn inject_retroarch_cheevos_pub(cfg: &str, username: &str, token: &str) -> String {
    inject_retroarch_cheevos(cfg, username, token)
}

fn inject_retroarch_cheevos(cfg: &str, username: &str, token: &str) -> String {
    let mut lines: Vec<String> = cfg.lines().map(|l| l.to_string()).collect();

    let keys = [
        ("cheevos_enable", "\"true\""),
        ("cheevos_username", &format!("\"{}\"", username)),
        ("cheevos_token", &format!("\"{}\"", token)),
        ("cheevos_hardcore_mode_enable", "\"false\""),
    ];

    for (key, value) in &keys {
        let prefix = format!("{} ", key);
        let found = lines.iter_mut().find(|l| l.starts_with(&prefix) || l.starts_with(&format!("{}=", key)));
        if let Some(line) = found {
            *line = format!("{} = {}", key, value);
        } else {
            lines.push(format!("{} = {}", key, value));
        }
    }

    lines.join("\n")
}

fn inject_duckstation_cheevos(ini: &str, username: &str, token: &str) -> String {
    let mut lines: Vec<String> = ini.lines().map(|l| l.to_string()).collect();
    let mut in_section = false;
    let mut section_found = false;
    let mut enabled_set = false;
    let mut username_set = false;
    let mut token_set = false;

    for line in lines.iter_mut() {
        if line.trim().starts_with('[') {
            if in_section { break; }
            if line.trim() == "[Cheevos]" || line.trim() == "[Achievements]" {
                in_section = true;
                section_found = true;
            }
        } else if in_section {
            if line.starts_with("Enabled") {
                *line = "Enabled = true".to_string();
                enabled_set = true;
            } else if line.starts_with("Username") {
                *line = format!("Username = {}", username);
                username_set = true;
            } else if line.starts_with("Token") {
                *line = format!("Token = {}", token);
                token_set = true;
            } else if line.starts_with("ChallengeMode") || line.starts_with("HardcoreMode") {
                *line = format!("{} = false", line.split('=').next().unwrap().trim());
            }
        }
    }

    if !section_found {
        lines.push(String::new());
        lines.push("[Cheevos]".to_string());
        lines.push("Enabled = true".to_string());
        lines.push(format!("Username = {}", username));
        lines.push(format!("Token = {}", token));
        lines.push("ChallengeMode = false".to_string());
    } else if in_section {
        // Find where the section ends and insert missing keys
        let section_idx = lines.iter().position(|l| l.trim() == "[Cheevos]" || l.trim() == "[Achievements]").unwrap();
        let mut insert_at = section_idx + 1;
        while insert_at < lines.len() && !lines[insert_at].trim().starts_with('[') {
            insert_at += 1;
        }
        if !token_set { lines.insert(insert_at, format!("Token = {}", token)); }
        if !username_set { lines.insert(insert_at, format!("Username = {}", username)); }
        if !enabled_set { lines.insert(insert_at, "Enabled = true".to_string()); }
    }

    lines.join("\n")
}

fn inject_ini_section_cheevos(ini: &str, username: &str, token: &str) -> String {
    let mut lines: Vec<String> = ini.lines().map(|l| l.to_string()).collect();
    let mut in_section = false;
    let mut section_found = false;
    let mut enabled_set = false;
    let mut username_set = false;
    let mut token_set = false;

    for line in lines.iter_mut() {
        if line.trim().starts_with('[') {
            if in_section { break; }
            if line.trim() == "[Achievements]" {
                in_section = true;
                section_found = true;
            }
        } else if in_section {
            if line.starts_with("Enabled") {
                *line = "Enabled = true".to_string();
                enabled_set = true;
            } else if line.starts_with("Username") {
                *line = format!("Username = {}", username);
                username_set = true;
            } else if line.starts_with("Token") {
                *line = format!("Token = {}", token);
                token_set = true;
            } else if line.starts_with("ChallengeMode") || line.starts_with("HardcoreMode") {
                *line = format!("{} = false", line.split('=').next().unwrap().trim());
            }
        }
    }

    if !section_found {
        lines.push(String::new());
        lines.push("[Achievements]".to_string());
        lines.push("Enabled = true".to_string());
        lines.push(format!("Username = {}", username));
        lines.push(format!("Token = {}", token));
        lines.push("ChallengeMode = false".to_string());
    } else {
        let section_idx = lines.iter().position(|l| l.trim() == "[Achievements]").unwrap();
        let mut insert_at = section_idx + 1;
        while insert_at < lines.len() && !lines[insert_at].trim().starts_with('[') {
            insert_at += 1;
        }
        if !token_set { lines.insert(insert_at, format!("Token = {}", token)); }
        if !username_set { lines.insert(insert_at, format!("Username = {}", username)); }
        if !enabled_set { lines.insert(insert_at, "Enabled = true".to_string()); }
    }

    lines.join("\n")
}

/// Returns the RetroArch core DLL name for a given emulator ID if it should be
/// redirected to RetroArch when RA is enabled (standalone has no RA support).
pub fn retroarch_core_for_emulator(emulator_id: &str) -> Option<&'static str> {
    match emulator_id {
        "mesen" => Some("mesen_libretro.dll"),
        "mgba" => Some("mgba_libretro.dll"),
        "snes9x" => Some("snes9x_libretro.dll"),
        "project64" => Some("mupen64plus_next_libretro.dll"),
        "melonds" => Some("melonds_ds_libretro.dll"),
        "flycast" => Some("flycast_libretro.dll"),
        _ => None,
    }
}

fn clean_name(name: &str) -> String {
    let lower = name.to_lowercase();
    let mut cleaned = String::new();
    let mut in_parens = false;
    let mut in_brackets = false;
    for c in lower.chars() {
        match c {
            '(' => { in_parens = true; }
            ')' => { in_parens = false; continue; }
            '[' => { in_brackets = true; }
            ']' => { in_brackets = false; continue; }
            _ if in_parens || in_brackets => {}
            _ if c.is_alphanumeric() || c == ' ' => { cleaned.push(c); }
            _ => {}
        }
    }
    cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
}
