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

/// How EmuWorld plays this emulator online: "retroarch" (libretro relay, via
/// the emulator's core), "ppsspp" (ad hoc server), "dolphin" (NetPlay code)
/// or "azahar" (room). `None`: no netplay, so no lobby for its games (a lobby
/// on a PS2 or Switch game would only fail once everyone is ready).
pub fn netplay_method(emu: &crate::emulators::EmulatorInfo) -> Option<&'static str> {
    match emu.id.as_str() {
        "ppsspp" => Some("ppsspp"),
        "dolphin" => Some("dolphin"),
        "azahar" => Some("azahar"),
        id if emu.core_name.is_some() || crate::retroachievements::retroarch_core_for_emulator(id).is_some() => Some("retroarch"),
        _ => None,
    }
}

/// Ids of the catalog emulators that can play online (lobbies are offered
/// only for the consoles they run).
#[tauri::command]
pub fn netplay_emulators() -> Vec<String> {
    let ids: Vec<String> = crate::emulators::get_catalog()
        .into_iter()
        .filter(|e| netplay_method(e).is_some())
        .map(|e| e.id)
        .collect();
    push_log("INFO", &format!("Netplay: {} émulateurs compatibles", ids.len()));
    ids
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
    host_code: Option<String>,
) -> Result<NetplayLaunch, String> {
    let is_host = role == "host";
    push_log("INFO", &format!("Netplay: lancement {} ({}) pour le lobby {} — {}", role, emulator_id, lobby_id, rom_path));
    let result = launch_inner(&emulator_id, &rom_path, is_host, &lobby_id, &username, max_players, relay, host_code).await;
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
    host_code: Option<String>,
) -> Result<NetplayLaunch, String> {
    let catalog = crate::emulators::get_catalog();
    let emu = catalog.iter().find(|e| e.id == emulator_id).ok_or("Émulateur inconnu")?;
    let config = crate::get_config();
    let final_path = rom_path.replace(r"\\?\", "").replace('/', "\\");
    if !Path::new(&final_path).is_file() {
        return Err("ROM introuvable".to_string());
    }

    let emus_dir = PathBuf::from(&config.emulators_directory);
    netplay_method(emu).ok_or("Pas de netplay pour cette console")?;
    match emu.id.as_str() {
        "ppsspp" => return launch_ppsspp(&emus_dir, &final_path, username),
        "dolphin" => return launch_dolphin(&emus_dir, &final_path, username, is_host, host_code.as_deref()),
        "azahar" => return launch_azahar(&emus_dir, &emu.executable_name, &final_path),
        _ => {}
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

/// Sets `key = value` lines inside `[section]` of an INI file, keeping every
/// other line. Keys are matched exactly within that section only (the old
/// code searched "Enable = " anywhere and could rewrite another setting).
pub fn set_ini_values(ini: &str, section: &str, values: &[(&str, String)]) -> String {
    let header = format!("[{}]", section);
    let mut lines: Vec<String> = ini.lines().map(str::to_string).collect();
    let start = match lines.iter().position(|l| l.trim().eq_ignore_ascii_case(&header)) {
        Some(i) => i,
        None => {
            if lines.last().map_or(false, |l| !l.trim().is_empty()) {
                lines.push(String::new());
            }
            lines.push(header.clone());
            lines.len() - 1
        }
    };
    for (key, value) in values {
        let end = lines[start + 1..]
            .iter()
            .position(|l| l.trim_start().starts_with('['))
            .map_or(lines.len(), |i| start + 1 + i);
        let line = format!("{} = {}", key, value);
        match (start + 1..end).find(|&i| lines[i].split_once('=').map_or(false, |(k, _)| k.trim() == *key)) {
            Some(i) => lines[i] = line,
            None => {
                // Before trailing blank lines of the section.
                let mut at = end;
                while at > start + 1 && lines[at - 1].trim().is_empty() {
                    at -= 1;
                }
                lines.insert(at, line);
            }
        }
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

fn read_ini(path: &Path) -> String {
    // PPSSPP/Dolphin may write a UTF-8 BOM.
    fs::read_to_string(path).map(|s| s.trim_start_matches('\u{feff}').to_string()).unwrap_or_default()
}

fn write_ini(path: &Path, content: &str) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    fs::write(path, content).map_err(|e| format!("Impossible d'écrire {}: {}", path.display(), e))
}

/// PPSSPP's memory stick: `memstick/` next to the exe (portable zip), or the
/// folder named in `installed.txt` (Documents\PPSSPP when it is empty).
pub fn ppsspp_memstick(exe_dir: &Path, documents: &Path) -> PathBuf {
    match fs::read_to_string(exe_dir.join("installed.txt")) {
        Ok(text) => {
            let custom = text.trim_start_matches('\u{feff}').trim();
            if custom.is_empty() { documents.join("PPSSPP") } else { PathBuf::from(custom) }
        }
        Err(_) => exe_dir.join("memstick"),
    }
}

/// Ad hoc multiplayer: both players reach the same public ad hoc server
/// (PPSSPP's default), with their EmuWorld name in game. The MAC address is
/// left to PPSSPP (it picks a random one), never forced.
pub fn ppsspp_netplay_ini(ini: &str, nick: &str) -> String {
    let ini = set_ini_values(ini, "Network", &[
        ("EnableWlan", "True".into()),
        ("EnableAdhocServer", "False".into()),
        ("proAdhocServer", PPSSPP_ADHOC_SERVER.into()),
        ("PortOffset", "10000".into()),
    ]);
    set_ini_values(&ini, "SystemParam", &[("NickName", nick.into())])
}

const PPSSPP_ADHOC_SERVER: &str = "socom.cc";

fn launch_ppsspp(emus_dir: &Path, rom: &str, username: &str) -> Result<NetplayLaunch, String> {
    let dir = emus_dir.join("ppsspp");
    let exe = crate::find_executable(&dir.to_path_buf(), "PPSSPPWindows64.exe").ok_or("PPSSPP n'est pas installé")?;
    let exe_dir = exe.parent().unwrap_or(&dir).to_path_buf();
    let documents = dirs::document_dir().unwrap_or_else(|| exe_dir.clone());
    let ini_path = ppsspp_memstick(&exe_dir, &documents).join("PSP").join("SYSTEM").join("ppsspp.ini");
    write_ini(&ini_path, &ppsspp_netplay_ini(&read_ini(&ini_path), &sanitize_nick(username)))?;
    push_log("INFO", &format!("Netplay PPSSPP: ad hoc via {} ({})", PPSSPP_ADHOC_SERVER, ini_path.display()));
    let mut c = Command::new(&exe);
    c.current_dir(&exe_dir).arg(rom).arg("--fullscreen");
    spawn(c)?;
    Ok(NetplayLaunch { method: "ppsspp".into(), nick: None })
}

/// Dolphin's user folder: `User\` next to the exe when portable, else
/// %APPDATA%\Dolphin Emulator (recent builds) or Documents\Dolphin Emulator.
pub fn dolphin_user_dir(exe_dir: &Path, appdata: &Path, documents: &Path) -> PathBuf {
    if exe_dir.join("portable.txt").exists() {
        return exe_dir.join("User");
    }
    let roaming = appdata.join("Dolphin Emulator");
    if roaming.exists() || !documents.join("Dolphin Emulator").exists() {
        roaming
    } else {
        documents.join("Dolphin Emulator")
    }
}

/// A Dolphin traversal host code as typed by the host (letters/digits only).
pub fn valid_dolphin_code(code: &str) -> bool {
    (4..=16).contains(&code.len()) && code.chars().all(|c| c.is_ascii_alphanumeric())
}

/// Dolphin.ini for NetPlay through Dolphin's traversal server: the player's
/// name, the host's code pre-filled on the friend's side, and the ROM folder
/// in the game list (NetPlay only offers games from that list).
pub fn dolphin_netplay_ini(ini: &str, nick: &str, host_code: Option<&str>, rom_dir: &str) -> String {
    let mut netplay = vec![("TraversalChoice", "traversal".to_string()), ("Nickname", nick.to_string())];
    if let Some(code) = host_code {
        netplay.push(("HostCode", code.to_string()));
    }
    let ini = set_ini_values(ini, "NetPlay", &netplay);

    // [General] ISOPaths = N, ISOPath0..N-1
    let general: Vec<(String, String)> = {
        let mut in_general = false;
        ini.lines()
            .filter_map(|l| {
                let t = l.trim();
                if t.starts_with('[') {
                    in_general = t.eq_ignore_ascii_case("[General]");
                    return None;
                }
                if !in_general { return None; }
                let (k, v) = t.split_once('=')?;
                Some((k.trim().to_string(), v.trim().to_string()))
            })
            .collect()
    };
    let count: usize = general.iter().find(|(k, _)| k == "ISOPaths").and_then(|(_, v)| v.parse().ok()).unwrap_or(0);
    let norm = |p: &str| p.trim_end_matches(['\\', '/']).replace('/', "\\").to_lowercase();
    let known = (0..count).any(|i| {
        general.iter().any(|(k, v)| k == &format!("ISOPath{}", i) && norm(v) == norm(rom_dir))
    });
    if known {
        return ini;
    }
    let new_key = format!("ISOPath{}", count);
    set_ini_values(&ini, "General", &[
        ("ISOPaths", (count + 1).to_string()),
        (new_key.as_str(), rom_dir.to_string()),
    ])
}

fn launch_dolphin(emus_dir: &Path, rom: &str, username: &str, is_host: bool, host_code: Option<&str>) -> Result<NetplayLaunch, String> {
    let dir = emus_dir.join("dolphin");
    let exe = crate::find_executable(&dir.to_path_buf(), "Dolphin.exe").ok_or("Dolphin n'est pas installé")?;
    let exe_dir = exe.parent().unwrap_or(&dir).to_path_buf();
    let code = if is_host {
        None
    } else {
        let code = host_code.map(str::trim).filter(|c| valid_dolphin_code(c)).ok_or("Code Dolphin de l'hôte manquant ou invalide")?;
        Some(code)
    };
    let appdata = dirs::config_dir().unwrap_or_else(|| exe_dir.clone());
    let documents = dirs::document_dir().unwrap_or_else(|| exe_dir.clone());
    let ini_path = dolphin_user_dir(&exe_dir, &appdata, &documents).join("Config").join("Dolphin.ini");
    let rom_dir = Path::new(rom).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    write_ini(&ini_path, &dolphin_netplay_ini(&read_ini(&ini_path), &sanitize_nick(username), code, &rom_dir))?;
    push_log("INFO", &format!("Netplay Dolphin: {} via traversal ({})", if is_host { "hôte" } else { "client" }, ini_path.display()));
    // No game argument: NetPlay is started from Dolphin's window
    // (Tools → Start NetPlay), the game would otherwise start solo.
    let mut c = Command::new(&exe);
    c.current_dir(&exe_dir);
    spawn(c)?;
    Ok(NetplayLaunch { method: "dolphin".into(), nick: None })
}

/// Azahar has no multiplayer command line ("--multiplayer ... not yet
/// implemented for the Qt frontend"): start the game, the lobby shows the steps.
fn launch_azahar(emus_dir: &Path, exe_name: &str, rom: &str) -> Result<NetplayLaunch, String> {
    let dir = emus_dir.join("azahar");
    let exe = crate::find_executable(&dir.to_path_buf(), exe_name).ok_or("Azahar n'est pas installé")?;
    let mut c = Command::new(&exe);
    c.current_dir(exe.parent().unwrap_or(&dir)).arg(rom);
    spawn(c)?;
    Ok(NetplayLaunch { method: "azahar".into(), nick: None })
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

    #[test]
    fn only_emulators_with_netplay_get_lobbies() {
        let catalog = crate::emulators::get_catalog();
        let method = |id: &str| netplay_method(catalog.iter().find(|e| e.id == id).unwrap_or_else(|| panic!("{} absent du catalogue", id)));
        assert_eq!(method("ppsspp"), Some("ppsspp"));
        assert_eq!(method("dolphin"), Some("dolphin"));
        assert_eq!(method("azahar"), Some("azahar"));
        assert_eq!(method("mesen"), Some("retroarch"));
        assert_eq!(method("retroarch-nes"), Some("retroarch"));
        assert_eq!(method("flycast"), Some("retroarch"));
        // Every other emulator is a deliberate "no netplay": a new emulator
        // must be added to one list or the other.
        // ("retroarch" alone is the multi-system frontend: no core, no console.)
        let without = ["cemu", "ryubing", "duckstation", "pcsx2", "rpcs3", "xemu", "xenia", "retroarch"];
        for emu in &catalog {
            assert_eq!(netplay_method(emu).is_none(), without.contains(&emu.id.as_str()), "netplay de {} à décider", emu.id);
        }
        let ids = netplay_emulators();
        assert!(ids.contains(&"retroarch-snes".to_string()) && !ids.contains(&"pcsx2".to_string()));
    }

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

    #[test]
    fn ini_values_land_in_their_own_section_only() {
        let ini = "[Graphics]\nEnable = True\nFrameSkip = 0\n\n[Network]\nEnableWlan = False\n\n[SystemParam]\nNickName = PPSSPP\n";
        let out = set_ini_values(ini, "Network", &[("EnableWlan", "True".into()), ("proAdhocServer", "socom.cc".into())]);
        assert_eq!(
            out,
            "[Graphics]\nEnable = True\nFrameSkip = 0\n\n[Network]\nEnableWlan = True\nproAdhocServer = socom.cc\n\n[SystemParam]\nNickName = PPSSPP\n"
        );
        // Missing section is created; running twice changes nothing.
        let fresh = set_ini_values("", "NetPlay", &[("TraversalChoice", "traversal".into())]);
        assert_eq!(fresh, "[NetPlay]\nTraversalChoice = traversal\n");
        assert_eq!(set_ini_values(&out, "Network", &[("EnableWlan", "True".into())]), out);
    }

    #[test]
    fn ppsspp_joins_the_default_adhoc_server_without_forcing_a_mac() {
        let ini = ppsspp_netplay_ini("[SystemParam]\nMacAddress = 1a:2b:3c:4d:5e:6f\nNickName = PPSSPP\n", "Leo");
        assert!(ini.contains("[Network]\nEnableWlan = True\nEnableAdhocServer = False\nproAdhocServer = socom.cc\nPortOffset = 10000"));
        assert!(ini.contains("MacAddress = 1a:2b:3c:4d:5e:6f"), "the player's own MAC is kept");
        assert!(ini.contains("NickName = Leo"));
        assert!(!ini.contains("01:02:03"), "no fixed (multicast) MAC any more");
    }

    #[test]
    fn finds_the_ppsspp_and_dolphin_user_folders() {
        let root = std::env::temp_dir().join(format!("emuworld_np_dirs_{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let exe = root.join("exe");
        let docs = root.join("docs");
        let appdata = root.join("appdata");
        fs::create_dir_all(&exe).unwrap();
        assert_eq!(ppsspp_memstick(&exe, &docs), exe.join("memstick"));
        fs::write(exe.join("installed.txt"), "").unwrap();
        assert_eq!(ppsspp_memstick(&exe, &docs), docs.join("PPSSPP"));
        fs::write(exe.join("installed.txt"), "E:\\PSP\n").unwrap();
        assert_eq!(ppsspp_memstick(&exe, &docs), PathBuf::from("E:\\PSP"));

        assert_eq!(dolphin_user_dir(&exe, &appdata, &docs), appdata.join("Dolphin Emulator"));
        fs::create_dir_all(docs.join("Dolphin Emulator")).unwrap();
        assert_eq!(dolphin_user_dir(&exe, &appdata, &docs), docs.join("Dolphin Emulator"));
        fs::write(exe.join("portable.txt"), "").unwrap();
        assert_eq!(dolphin_user_dir(&exe, &appdata, &docs), exe.join("User"));
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn dolphin_gets_the_host_code_and_the_rom_folder() {
        let ini = "[General]\nISOPaths = 1\nISOPath0 = D:\\Jeux\\Wii\n[NetPlay]\nTraversalChoice = direct\n";
        let client = dolphin_netplay_ini(ini, "Leo", Some("Ab12Cd34"), "D:\\Roms\\GameCube");
        assert!(client.contains("TraversalChoice = traversal"));
        assert!(client.contains("HostCode = Ab12Cd34"));
        assert!(client.contains("Nickname = Leo"));
        assert!(client.contains("ISOPaths = 2") && client.contains("ISOPath1 = D:\\Roms\\GameCube"));
        // Same folder again (any case / trailing slash): not added twice.
        let again = dolphin_netplay_ini(&client, "Leo", None, "d:\\roms\\gamecube\\");
        assert!(again.contains("ISOPaths = 2") && !again.contains("ISOPath2"));
        // The host has no code to pre-fill.
        assert!(!dolphin_netplay_ini("", "Host", None, "D:\\Roms").contains("HostCode"));

        assert!(valid_dolphin_code("Ab12Cd34"));
        assert!(!valid_dolphin_code("ab"));
        assert!(!valid_dolphin_code("abc\nTraversalChoice=direct"));
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
        let host = launch_inner(&emu, &rom, true, &lobby, "EWtest", 2, None, None).await.expect("host launch");
        let nick = host.nick.expect("host nick");
        let relay = find_netplay_session(nick.clone(), Some(60)).await.expect("room on the relay");
        println!("room {} via {}:{} ({})", nick, relay.ip, relay.port, relay.session);
        launch_inner(&emu, &rom, false, &lobby, "EWfriend", 2, Some(relay), None).await.expect("client launch");
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
