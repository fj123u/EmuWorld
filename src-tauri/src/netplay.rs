//! Online multiplayer launch for lobbies.
//!
//! RetroArch (most consoles) goes through libretro's relay ("MITM") servers,
//! so nobody has to open a port:
//! 1. the host starts RetroArch with `--host` and a relay enabled; RetroArch
//!    announces the room on the public lobby list with a unique nickname;
//! 2. EmuWorld finds that room on http://lobby.libretro.com/list/ and reads
//!    the relay address + session id, which the UI stores in the Supabase lobby;
//! 3. friends start RetroArch with `--connect <relay> --port --mitm-session`.
//!
//! Every netplay setting goes in an extra config file passed with
//! `--appendconfig`, with `config_save_on_exit = "false"`, so the user's own
//! retroarch.cfg is never changed. A per-lobby password is not possible:
//! RetroArch ignores the configured password on `--connect`; the host caps
//! connections to the lobby size instead.
//!
//! Verified on 2026-11-06 with RetroArch 1.20 (host + client on one PC through
//! the Madrid relay: player 2 joined in ~10 s, ~56 ms ping).

use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::push_log;

const LOBBY_LIST_URL: &str = "http://lobby.libretro.com/list/";
/// libretro relay closest to France (europe-west1). Others: nyc, montreal, saopaulo.
const RELAY_SERVER: &str = "madrid";
/// RetroArch's `host_method` for a room reached through a relay.
const HOST_METHOD_MITM: u64 = 3;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelaySession {
    pub ip: String,
    pub port: u16,
    pub session: String,
}

impl RelaySession {
    /// Only libretro's own relays, and values that are safe as CLI arguments.
    pub fn is_valid(&self) -> bool {
        let host_ok = self.ip.ends_with(".retroarch.com")
            && self.ip.len() <= 100
            && self.ip.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-');
        let session_ok = !self.session.is_empty()
            && self.session.len() <= 64
            && self.session.chars().all(|c| c.is_ascii_alphanumeric() || "+/=".contains(c));
        host_ok && session_ok && self.port != 0
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct NetplayLaunch {
    /// "retroarch" (relay flow), "ppsspp" or "dolphin".
    pub method: String,
    /// Nickname RetroArch announces (host) or uses (client).
    pub nick: Option<String>,
}

/// Keeps a username usable as a RetroArch nickname (ASCII, no spaces), 16 chars max.
pub fn sanitize_nick(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(16)
        .collect();
    if cleaned.is_empty() { "Joueur".to_string() } else { cleaned }
}

/// Unique nickname of the host's room, found again on the public list:
/// `EW-<pseudo>-<6 chars of the lobby id>` (RetroArch allows 32 chars).
pub fn host_nick(username: &str, lobby_id: &str) -> String {
    let tag: String = lobby_id.chars().filter(|c| c.is_ascii_alphanumeric()).take(6).collect();
    format!("EW-{}-{}", sanitize_nick(username), tag)
}

/// Finds the relay session of the room announced as `nick` in the lobby list JSON.
pub fn find_relay_session(list_json: &str, nick: &str) -> Option<RelaySession> {
    let rooms: Vec<serde_json::Value> = serde_json::from_str(list_json).ok()?;
    rooms.iter().find_map(|room| {
        let f = room.get("fields")?;
        if f.get("username")?.as_str()? != nick || f.get("host_method")?.as_u64()? != HOST_METHOD_MITM {
            return None;
        }
        let relay = RelaySession {
            ip: f.get("mitm_ip")?.as_str()?.to_string(),
            port: u16::try_from(f.get("mitm_port")?.as_u64()?).ok()?,
            session: f.get("mitm_session")?.as_str()?.to_string(),
        };
        relay.is_valid().then_some(relay)
    })
}

/// Settings shared by host and client, on top of EmuWorld's usual RetroArch ones.
fn common_config(nick: &str) -> Vec<(&'static str, String)> {
    let mut cfg: Vec<(&'static str, String)> = crate::retroarch_launch_settings(None)
        .into_iter()
        .map(|(k, v)| (k, v.to_string()))
        .collect();
    cfg.extend([
        // Never write these netplay values into the user's retroarch.cfg.
        ("config_save_on_exit", "false".to_string()),
        ("netplay_nickname", nick.to_string()),
        ("netplay_password", String::new()),
        ("netplay_spectate_password", String::new()),
        ("netplay_start_as_spectator", "false".to_string()),
        ("netplay_allow_pausing", "true".to_string()),
        // Achievements' hardcore mode refuses netplay.
        ("cheevos_hardcore_mode_enable", "false".to_string()),
    ]);
    cfg
}

pub fn host_config(nick: &str, max_clients: u32) -> Vec<(&'static str, String)> {
    let mut cfg = common_config(nick);
    cfg.extend([
        ("netplay_use_mitm_server", "true".to_string()),
        ("netplay_mitm_server", RELAY_SERVER.to_string()),
        // Needed for the room (and its relay session) to appear on the list.
        ("netplay_public_announce", "true".to_string()),
        // Only the lobby's friends: no seat left for a stranger from the list.
        ("netplay_max_connections", max_clients.clamp(1, 3).to_string()),
        ("netplay_max_ping", "0".to_string()),
    ]);
    cfg
}

pub fn client_config(nick: &str) -> Vec<(&'static str, String)> {
    common_config(nick)
}

pub fn host_args(core: &Path, append_cfg: &Path, log_file: &Path, rom: &str) -> Vec<OsString> {
    vec![
        "-L".into(),
        core.into(),
        "--host".into(),
        format!("--appendconfig={}", append_cfg.display()).into(),
        format!("--log-file={}", log_file.display()).into(),
        "--verbose".into(),
        rom.into(),
    ]
}

pub fn client_args(core: &Path, append_cfg: &Path, log_file: &Path, relay: &RelaySession, nick: &str, rom: &str) -> Vec<OsString> {
    vec![
        "-L".into(),
        core.into(),
        format!("--connect={}", relay.ip).into(),
        format!("--port={}", relay.port).into(),
        format!("--mitm-session={}", relay.session).into(),
        format!("--nick={}", nick).into(),
        format!("--appendconfig={}", append_cfg.display()).into(),
        format!("--log-file={}", log_file.display()).into(),
        "--verbose".into(),
        rom.into(),
    ]
}

fn write_append_config(path: &Path, values: &[(&'static str, String)]) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let body: String = values.iter().map(|(k, v)| format!("{} = \"{}\"\n", k, v)).collect();
    fs::write(path, body).map_err(|e| format!("Config netplay impossible à écrire: {}", e))
}

/// Downloads a missing RetroArch core from the libretro buildbot.
async fn ensure_core(cores_dir: &Path, core_name: &str) -> Result<PathBuf, String> {
    let core_path = cores_dir.join(core_name);
    if core_path.exists() {
        return Ok(core_path);
    }
    push_log("INFO", &format!("Netplay: core {} manquant, téléchargement…", core_name));
    fs::create_dir_all(cores_dir).map_err(|e| e.to_string())?;
    let url = format!("https://buildbot.libretro.com/nightly/windows/x86_64/latest/{}.zip", core_name);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client.get(&url).send().await.map_err(|e| format!("Téléchargement du core impossible: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("Téléchargement du core impossible (HTTP {})", resp.status().as_u16()));
    }
    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    let tmp = cores_dir.join(format!("_{}_netplay.zip", core_name));
    fs::write(&tmp, &bytes).map_err(|e| e.to_string())?;
    let extracted = crate::extract_zip(&tmp, &cores_dir.to_path_buf());
    let _ = fs::remove_file(&tmp);
    extracted.map_err(|e| format!("Extraction du core impossible: {}", e))?;
    if core_path.exists() {
        push_log("INFO", &format!("Netplay: core {} installé", core_name));
        Ok(core_path)
    } else {
        Err(format!("Core '{}' introuvable après téléchargement", core_name))
    }
}

fn spawn(mut cmd: Command) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    cmd.spawn().map(|_| ()).map_err(|e| format!("Lancement impossible: {}", e))
}

/// Starts the emulator for a lobby. `role` is "host" or "client"; a RetroArch
/// client needs the host's `relay` (from the lobby). `max_players` caps the
/// host's connections.
#[tauri::command]
pub async fn launch_netplay(
    emulator_id: String,
    rom_path: String,
    role: String,
    lobby_id: String,
    username: String,
    max_players: u32,
    relay: Option<RelaySession>,
) -> Result<NetplayLaunch, String> {
    let is_host = role == "host";
    push_log("INFO", &format!("Netplay: lancement {} ({}) pour le lobby {} — {}", role, emulator_id, lobby_id, rom_path));
    let result = launch_inner(&emulator_id, &rom_path, is_host, &lobby_id, &username, max_players, relay).await;
    match &result {
        Ok(l) => push_log("INFO", &format!("Netplay: {} démarré via {} (pseudo {:?})", role, l.method, l.nick)),
        Err(e) => push_log("ERROR", &format!("Netplay: lancement {} impossible: {}", role, e)),
    }
    result
}

async fn launch_inner(
    emulator_id: &str,
    rom_path: &str,
    is_host: bool,
    lobby_id: &str,
    username: &str,
    max_players: u32,
    relay: Option<RelaySession>,
) -> Result<NetplayLaunch, String> {
    let catalog = crate::emulators::get_catalog();
    let emu = catalog.iter().find(|e| e.id == emulator_id).ok_or("Émulateur inconnu")?;
    let config = crate::get_config();
    let final_path = rom_path.replace(r"\\?\", "").replace('/', "\\");
    if !Path::new(&final_path).is_file() {
        return Err("ROM introuvable".to_string());
    }

    if emu.id == "ppsspp" || emu.id == "dolphin" {
        legacy_launch(&emu.id, &config.emulators_directory, &final_path, is_host)?;
        return Ok(NetplayLaunch { method: emu.id.clone(), nick: None });
    }

    let ra_dir = PathBuf::from(&config.emulators_directory).join("retroarch");
    let ra_exe = crate::find_executable(&ra_dir, "retroarch.exe").ok_or("RetroArch n'est pas installé (nécessaire pour le netplay)")?;
    let ra_root = ra_exe.parent().unwrap_or(&ra_dir).to_path_buf();
    let core_name = emu
        .core_name
        .as_deref()
        .or_else(|| crate::retroachievements::retroarch_core_for_emulator(&emu.id))
        .ok_or("Pas de netplay pour cette console")?;
    let core = ensure_core(&ra_root.join("cores"), core_name).await?;

    let work = crate::emuworld_base_dir().join("netplay");
    let logs = crate::emuworld_base_dir().join("logs");
    let _ = fs::create_dir_all(&logs);
    let (args, nick) = if is_host {
        let nick = host_nick(username, lobby_id);
        let cfg = work.join("host.cfg");
        write_append_config(&cfg, &host_config(&nick, max_players.saturating_sub(1)))?;
        (host_args(&core, &cfg, &logs.join("netplay_host.log"), &final_path), nick)
    } else {
        let relay = relay.filter(RelaySession::is_valid).ok_or("Session de l'hôte manquante ou invalide")?;
        let nick = sanitize_nick(username);
        let cfg = work.join("client.cfg");
        write_append_config(&cfg, &client_config(&nick))?;
        (client_args(&core, &cfg, &logs.join("netplay_client.log"), &relay, &nick, &final_path), nick)
    };
    let mut cmd = Command::new(&ra_exe);
    cmd.current_dir(&ra_root).args(&args);
    spawn(cmd)?;
    Ok(NetplayLaunch { method: "retroarch".to_string(), nick: Some(nick) })
}

/// PPSSPP (ad hoc) and Dolphin, unchanged for now (to be reworked: Dolphin
/// has no netplay command line, PPSSPP's ini editing needs fixing).
fn legacy_launch(emu_id: &str, emulators_directory: &str, rom: &str, is_host: bool) -> Result<(), String> {
    if emu_id == "ppsspp" {
        let dir = PathBuf::from(emulators_directory).join("ppsspp");
        let exe = crate::find_executable(&dir, "PPSSPPWindows64.exe").ok_or("PPSSPP n'est pas installé")?;
        let mut c = Command::new(&exe);
        c.current_dir(exe.parent().unwrap_or(&dir)).arg(rom).arg("--fullscreen");
        return spawn(c);
    }
    let dir = PathBuf::from(emulators_directory).join("dolphin");
    let exe = crate::find_executable(&dir, "Dolphin.exe").ok_or("Dolphin n'est pas installé")?;
    let mut c = Command::new(&exe);
    c.current_dir(exe.parent().unwrap_or(&dir)).arg("-e").arg(rom);
    let _ = is_host;
    spawn(c)
}

/// Waits until the host's room shows on the public list and returns its relay session.
#[tauri::command]
pub async fn find_netplay_session(nick: String, timeout_secs: Option<u64>) -> Result<RelaySession, String> {
    push_log("INFO", &format!("Netplay: recherche de la salle '{}' sur le relais", nick));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent(format!("EmuWorld/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs.unwrap_or(60).min(120));
    let mut last_error = String::new();
    while std::time::Instant::now() < deadline {
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        match client.get(LOBBY_LIST_URL).send().await {
            Ok(resp) => match resp.text().await {
                Ok(body) => {
                    if let Some(relay) = find_relay_session(&body, &nick) {
                        push_log("INFO", &format!("Netplay: salle trouvée via {}:{}", relay.ip, relay.port));
                        return Ok(relay);
                    }
                }
                Err(e) => last_error = e.to_string(),
            },
            Err(e) => last_error = e.to_string(),
        }
    }
    push_log("ERROR", &format!("Netplay: salle '{}' introuvable ({})", nick, last_error));
    Err("La partie de l'hôte n'est pas apparue sur le relais (RetroArch s'est-il bien lancé ?)".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIST: &str = r#"[
      {"fields": {"username": "Airwax", "host_method": 0, "mitm_ip": "", "mitm_port": 0, "mitm_session": ""}},
      {"fields": {"username": "EW-Leo-3f2a91", "host_method": 3, "mitm_ip": "europe-west1.relay.retroarch.com", "mitm_port": 55435, "mitm_session": "g3kjejYF3wcqOkMh"}},
      {"fields": {"username": "EW-Evil-000000", "host_method": 3, "mitm_ip": "attacker.example.com", "mitm_port": 55435, "mitm_session": "abc"}}
    ]"#;

    #[test]
    fn finds_the_relay_session_of_the_host_room() {
        assert_eq!(
            find_relay_session(LIST, "EW-Leo-3f2a91"),
            Some(RelaySession { ip: "europe-west1.relay.retroarch.com".into(), port: 55435, session: "g3kjejYF3wcqOkMh".into() })
        );
        assert_eq!(find_relay_session(LIST, "Airwax"), None, "direct (non relay) rooms are not used");
        assert_eq!(find_relay_session(LIST, "EW-Evil-000000"), None, "only libretro relays");
        assert_eq!(find_relay_session(LIST, "nobody"), None);
        assert_eq!(find_relay_session("not json", "EW-Leo-3f2a91"), None);
    }

    #[test]
    fn rejects_relay_values_unsafe_as_arguments() {
        let ok = RelaySession { ip: "us-east1.relay.retroarch.com".into(), port: 55435, session: "Mf/4xxQvkDmDWUwl".into() };
        assert!(ok.is_valid());
        assert!(!RelaySession { session: "x --verbose".into(), ..ok.clone() }.is_valid());
        assert!(!RelaySession { ip: "retroarch.com.evil.net".into(), ..ok.clone() }.is_valid());
        assert!(!RelaySession { port: 0, ..ok.clone() }.is_valid());
        assert!(!RelaySession { session: String::new(), ..ok }.is_valid());
    }

    #[test]
    fn nicknames_are_unique_and_retroarch_safe() {
        assert_eq!(host_nick("Léo le pro", "3f2a91bc-1111-2222"), "EW-Lolepro-3f2a91");
        assert_eq!(host_nick("fj123u", "ab-cd-ef-12"), "EW-fj123u-abcdef");
        assert!(host_nick(&"x".repeat(40), "123456789").len() <= 32);
        assert_eq!(sanitize_nick("   "), "Joueur");
    }

    #[test]
    fn netplay_settings_never_touch_the_user_config_and_cap_the_room() {
        let host: std::collections::HashMap<_, _> = host_config("EW-a-b", 1).into_iter().collect();
        assert_eq!(host["config_save_on_exit"], "false");
        assert_eq!(host["netplay_use_mitm_server"], "true");
        assert_eq!(host["netplay_mitm_server"], "madrid");
        assert_eq!(host["netplay_public_announce"], "true");
        assert_eq!(host["netplay_max_connections"], "1");
        assert_eq!(host["netplay_nickname"], "EW-a-b");
        assert_eq!(host["pause_nonactive"], "false");
        let client: std::collections::HashMap<_, _> = client_config("Leo").into_iter().collect();
        assert_eq!(client["config_save_on_exit"], "false");
        assert!(!client.contains_key("netplay_use_mitm_server"));
        assert_eq!(host_config("x", 9).into_iter().find(|(k, _)| *k == "netplay_max_connections").unwrap().1, "3");
    }

    /// Real round trip through libretro's relay with the app's own code: host,
    /// room lookup, client. Opens two RetroArch windows; run by hand:
    /// `EMUWORLD_NETPLAY_ROM=<rom> cargo test live_relay -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn live_relay_round_trip() {
        let rom = std::env::var("EMUWORLD_NETPLAY_ROM").expect("EMUWORLD_NETPLAY_ROM");
        let emu = std::env::var("EMUWORLD_NETPLAY_EMU").unwrap_or_else(|_| "mgba".into());
        let lobby = format!("live{:06}", std::process::id() % 1_000_000);
        let host = launch_inner(&emu, &rom, true, &lobby, "EWtest", 2, None).await.expect("host launch");
        let nick = host.nick.expect("host nick");
        let relay = find_netplay_session(nick.clone(), Some(60)).await.expect("room on the relay");
        println!("room {} via {}:{} ({})", nick, relay.ip, relay.port, relay.session);
        launch_inner(&emu, &rom, false, &lobby, "EWfriend", 2, Some(relay)).await.expect("client launch");
        let log = crate::emuworld_base_dir().join("logs").join("netplay_client.log");
        for _ in 0..30 {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            let text = fs::read_to_string(&log).unwrap_or_default();
            if text.contains("joueur 2") || text.contains("player 2") {
                println!("client joined as player 2");
                return;
            }
        }
        panic!("client never joined, see {}", log.display());
    }

    #[test]
    fn builds_the_retroarch_command_lines() {
        let core = Path::new("C:\\RA\\cores\\mgba_libretro.dll");
        let cfg = Path::new("C:\\EW\\netplay\\client.cfg");
        let log = Path::new("C:\\EW\\logs\\netplay_client.log");
        let relay = RelaySession { ip: "europe-west1.relay.retroarch.com".into(), port: 55435, session: "g3kjejYF3wcqOkMh".into() };
        let args: Vec<String> = client_args(core, cfg, log, &relay, "Leo", "D:\\Roms\\Rubis.gba")
            .into_iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            args,
            [
                "-L", "C:\\RA\\cores\\mgba_libretro.dll",
                "--connect=europe-west1.relay.retroarch.com", "--port=55435", "--mitm-session=g3kjejYF3wcqOkMh", "--nick=Leo",
                "--appendconfig=C:\\EW\\netplay\\client.cfg", "--log-file=C:\\EW\\logs\\netplay_client.log", "--verbose",
                "D:\\Roms\\Rubis.gba",
            ]
        );
        let host: Vec<String> = host_args(core, cfg, log, "D:\\Roms\\Rubis.gba").into_iter().map(|a| a.to_string_lossy().into_owned()).collect();
        assert!(host.contains(&"--host".to_string()));
        assert!(!host.iter().any(|a| a.starts_with("--mitm")), "RetroArch has no --mitm option");
    }
}
