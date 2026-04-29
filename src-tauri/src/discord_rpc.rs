// Discord Rich Presence — shows EmuWorld as a "brand" always, with the
// current game as the state when one is running.
//
// The client is wrapped in a Mutex<Option<DiscordIpcClient>> so we can
// start/stop the connection cleanly and update presence on game launch /
// exit without leaking IPC handles.

use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

// Replace with your own app ID from https://discord.com/developers/applications.
// For now we ship a placeholder — the user must create the app themselves and
// drop the ID in the UI if they want rich presence to light up.
// The default ID below is EmuWorld's public demo app (owned by the project).
const DEFAULT_APP_ID: &str = "1334862011723284510";

// Global presence state. Discord IPC clients are !Sync-unfriendly so we gate
// access behind a plain std::sync::Mutex.
pub struct RpcState {
    client: Mutex<Option<DiscordIpcClient>>,
    connected: Mutex<bool>,
    start_ts: Mutex<Option<i64>>,
}

impl RpcState {
    pub fn new() -> Self {
        Self {
            client: Mutex::new(None),
            connected: Mutex::new(false),
            start_ts: Mutex::new(None),
        }
    }
}

fn unix_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// Connect to Discord (idempotent — safe to call again if already connected).
fn ensure_connected(state: &RpcState) -> Result<(), String> {
    let mut connected = state.connected.lock().map_err(|e| e.to_string())?;
    if *connected {
        return Ok(());
    }
    let mut client_slot = state.client.lock().map_err(|e| e.to_string())?;
    let mut client = DiscordIpcClient::new(DEFAULT_APP_ID);
    client.connect().map_err(|e| format!("discord connect failed: {}", e))?;
    *client_slot = Some(client);
    *connected = true;
    Ok(())
}

fn with_client<F>(state: &RpcState, f: F) -> Result<(), String>
where
    F: FnOnce(&mut DiscordIpcClient) -> Result<(), String>,
{
    ensure_connected(state)?;
    let mut client_slot = state.client.lock().map_err(|e| e.to_string())?;
    let client = client_slot
        .as_mut()
        .ok_or_else(|| "discord client not initialised".to_string())?;
    f(client)
}

// Idle presence — shown when the user is in the launcher but not playing.
// This is the "pub" slot: EmuWorld logo + "Browsing the library".
#[tauri::command]
pub fn discord_set_idle(state: tauri::State<'_, RpcState>) -> Result<(), String> {
    let mut start_lock = state.start_ts.lock().map_err(|e| e.to_string())?;
    let start = *start_lock.get_or_insert_with(unix_ts);
    drop(start_lock);

    with_client(&state, |client| {
        let activity = activity::Activity::new()
            .state("Browsing the library")
            .details("In the launcher")
            .assets(
                activity::Assets::new()
                    .large_image("emuworld_logo")
                    .large_text("EmuWorld — retro emulation launcher"),
            )
            .timestamps(activity::Timestamps::new().start(start));
        client
            .set_activity(activity)
            .map_err(|e| format!("set_activity failed: {}", e))
    })
}

// Playing presence — shown while a game is running.
// Large image stays EmuWorld (brand persistence). Small image is the console
// or game-specific asset if available; fallback to "playing" generic.
#[tauri::command]
pub fn discord_set_playing(
    state: tauri::State<'_, RpcState>,
    game_name: String,
    console: String,
) -> Result<(), String> {
    // Reset session start so the elapsed timer begins now.
    let now = unix_ts();
    {
        let mut start_lock = state.start_ts.lock().map_err(|e| e.to_string())?;
        *start_lock = Some(now);
    }

    // Console-specific small icons match asset keys uploaded to the Discord
    // dev portal. Fall back to a generic gamepad if unknown.
    let small_key = console_icon_key(&console);
    let console_label = console.clone();

    with_client(&state, move |client| {
        let mut assets = activity::Assets::new()
            .large_image("emuworld_logo")
            .large_text("via EmuWorld");
        assets = assets
            .small_image(small_key)
            .small_text(console_label.as_str());

        // Clone `game_name` into owned strings the activity can borrow from.
        let state_line = format!("via EmuWorld · {}", console_label);
        let details_line = game_name.clone();

        let activity = activity::Activity::new()
            .state(state_line.as_str())
            .details(details_line.as_str())
            .assets(assets)
            .timestamps(activity::Timestamps::new().start(now));

        client
            .set_activity(activity)
            .map_err(|e| format!("set_activity failed: {}", e))
    })
}

#[tauri::command]
pub fn discord_clear(state: tauri::State<'_, RpcState>) -> Result<(), String> {
    let connected = { *state.connected.lock().map_err(|e| e.to_string())? };
    if !connected {
        return Ok(());
    }
    with_client(&state, |client| {
        client
            .clear_activity()
            .map_err(|e| format!("clear_activity failed: {}", e))
    })
}

// Map our console names to Discord asset keys. The user will need to upload
// matching assets to their Discord application; missing keys silently fall
// back to `playing_generic` (which should always exist).
fn console_icon_key(console: &str) -> &'static str {
    let c = console.to_lowercase();
    if c.contains("switch") {
        "console_switch"
    } else if c.contains("wii u") {
        "console_wiiu"
    } else if c.contains("wii") {
        "console_wii"
    } else if c.contains("3ds") {
        "console_3ds"
    } else if c.contains("ds") {
        "console_ds"
    } else if c.contains("gamecube") {
        "console_gamecube"
    } else if c.contains("n64") || c.contains("64") {
        "console_n64"
    } else if c.contains("gba") || c.contains("advance") {
        "console_gba"
    } else if c.contains("gbc") || c.contains("color") {
        "console_gbc"
    } else if c.contains("gb") || c.contains("game boy") {
        "console_gb"
    } else if c.contains("nes") {
        "console_nes"
    } else if c.contains("snes") {
        "console_snes"
    } else if c.contains("ps5") {
        "console_ps5"
    } else if c.contains("ps4") {
        "console_ps4"
    } else if c.contains("ps3") {
        "console_ps3"
    } else if c.contains("ps2") {
        "console_ps2"
    } else if c.contains("psp") {
        "console_psp"
    } else if c.contains("ps") {
        "console_ps1"
    } else if c.contains("xbox") {
        "console_xbox"
    } else if c.contains("dreamcast") {
        "console_dreamcast"
    } else if c.contains("saturn") {
        "console_saturn"
    } else if c.contains("mega drive") || c.contains("genesis") {
        "console_megadrive"
    } else {
        "playing_generic"
    }
}
