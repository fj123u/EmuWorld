use serde::{Deserialize, Serialize};
use regex::Regex;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;
use tauri::Emitter;
use reqwest;
use urlencoding;
use base64::Engine;
use std::io::Write;
use image::ImageReader;
use std::io::Cursor;

/// Returns the base data directory for EmuWorld.
/// In portable mode (portable.txt next to exe), uses the exe's directory.
/// Otherwise uses %LOCALAPPDATA%/EmuWorld.
pub fn emuworld_base_dir() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            if dir.join("portable.txt").exists() {
                let base = dir.join("EmuWorld_Data");
                let _ = fs::create_dir_all(&base);
                return base;
            }
        }
    }
    let base = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("EmuWorld");
    let _ = fs::create_dir_all(&base);
    base
}

#[derive(Default)]
struct CurrentPlayingState {
    game_name: Option<String>,
    console: Option<String>,
}

use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};

static APP_LOGS: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
static BANDWIDTH_LIMIT_KBPS: AtomicU64 = AtomicU64::new(0);
static COVER_URLS: OnceLock<Mutex<std::collections::HashMap<String, String>>> = OnceLock::new();
static CANCELLED_DOWNLOADS: OnceLock<Mutex<std::collections::HashSet<String>>> = OnceLock::new();

fn cancelled_downloads() -> &'static Mutex<std::collections::HashSet<String>> {
    CANCELLED_DOWNLOADS.get_or_init(|| Mutex::new(std::collections::HashSet::new()))
}

fn cover_urls() -> &'static Mutex<std::collections::HashMap<String, String>> {
    COVER_URLS.get_or_init(|| {
        let path = emuworld_base_dir().join("cover_urls.json");
        let map = if let Ok(data) = fs::read_to_string(&path) {
            serde_json::from_str::<std::collections::HashMap<String, String>>(&data).unwrap_or_default()
        } else {
            std::collections::HashMap::new()
        };
        Mutex::new(map)
    })
}

fn store_cover_url(key: &str, url: &str) {
    if let Ok(mut map) = cover_urls().lock() {
        if map.get(key).map(|v| v.as_str()) != Some(url) {
            map.insert(key.to_string(), url.to_string());
            let path = emuworld_base_dir().join("cover_urls.json");
            let _ = fs::write(&path, serde_json::to_string(&*map).unwrap_or_default());
        }
    }
}

fn app_logs() -> &'static Mutex<Vec<String>> {
    APP_LOGS.get_or_init(|| Mutex::new(Vec::new()))
}

fn log_file_path() -> PathBuf {
    let logs_dir = emuworld_base_dir().join("logs");
    fs::create_dir_all(&logs_dir).ok();
    let date = chrono::Local::now().format("%Y-%m-%d").to_string();
    logs_dir.join(format!("emuworld_{}.log", date))
}

pub fn push_log(level: &str, msg: &str) {
    let now = chrono::Local::now();
    let timestamp = now.format("%H:%M:%S").to_string();
    let entry = format!("[{}] {} — {}", level, timestamp, msg);
    println!("[LOG] {}", entry);

    // In-memory logs for UI
    if let Ok(mut logs) = app_logs().lock() {
        logs.push(entry.clone());
        if logs.len() > 500 { logs.drain(0..100); }
    }

    // Write to file
    let log_path = log_file_path();
    let file_entry = format!("[{}] [{}] {}\n", now.format("%Y-%m-%d %H:%M:%S"), level, msg);
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&log_path) {
        let _ = file.write_all(file_entry.as_bytes());
    }

    // Auto-cleanup: delete log files older than 7 days
    if let Ok(entries) = fs::read_dir(emuworld_base_dir().join("logs")) {
        let cutoff = now - chrono::Duration::days(7);
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if let Ok(modified) = meta.modified() {
                    let modified_dt: chrono::DateTime<chrono::Local> = modified.into();
                    if modified_dt < cutoff {
                        fs::remove_file(entry.path()).ok();
                    }
                }
            }
        }
    }
}

// Low-level keyboard hook for overlay shortcut (works in fullscreen)
static OVERLAY_APP_HANDLE: OnceLock<Mutex<Option<tauri::AppHandle>>> = OnceLock::new();

fn overlay_app_handle() -> &'static Mutex<Option<tauri::AppHandle>> {
    OVERLAY_APP_HANDLE.get_or_init(|| Mutex::new(None))
}

#[cfg(target_os = "windows")]
fn start_keyboard_hook(app_handle: tauri::AppHandle) {
    use winapi::um::winuser::*;
    use winapi::um::libloaderapi::GetModuleHandleW;
    use winapi::shared::windef::HHOOK;
    use std::sync::atomic::{AtomicBool, Ordering};

    static SHIFT_DOWN: AtomicBool = AtomicBool::new(false);

    match overlay_app_handle().lock() {
        Ok(mut guard) => *guard = Some(app_handle),
        Err(poisoned) => *poisoned.into_inner() = Some(app_handle),
    };

    unsafe extern "system" fn hook_proc(
        code: i32,
        w_param: usize,
        l_param: isize,
    ) -> isize {
        if code >= 0 {
            let kb = &*(l_param as *const KBDLLHOOKSTRUCT);
            let vk = kb.vkCode as i32;

            match w_param as u32 {
                WM_KEYDOWN | WM_SYSKEYDOWN => {
                    if vk == VK_SHIFT || vk == VK_LSHIFT || vk == VK_RSHIFT {
                        SHIFT_DOWN.store(true, Ordering::Relaxed);
                    }
                    if vk == VK_TAB && SHIFT_DOWN.load(Ordering::Relaxed) {
                        println!("[Overlay] LOW-LEVEL HOOK: Shift+Tab detected!");
                        push_log("INFO", "Overlay: Shift+Tab détecté (hook)");
                        if let Ok(guard) = overlay_app_handle().lock() {
                            if let Some(ref handle) = *guard {
                                use tauri::Manager;
                                if let Some(overlay_win) = handle.get_webview_window("overlay") {
                                    let _ = overlay_win.close();
                                    println!("[Overlay] Closed overlay window");
                                } else {
                                    let handle_clone = handle.clone();
                                    std::thread::spawn(move || {
                                        use tauri::WebviewWindowBuilder;
                                        let url = tauri::WebviewUrl::App("index.html?overlay=1".into());
                                        let builder = WebviewWindowBuilder::new(&handle_clone, "overlay", url)
                                            .title("EmuWorld Overlay")
                                            .decorations(false)
                                            .transparent(true)
                                            .always_on_top(true)
                                            .fullscreen(true)
                                            .skip_taskbar(true)
                                            .focused(true);
                                        match builder.build() {
                                            Ok(_) => println!("[Overlay] Window created"),
                                            Err(e) => println!("[Overlay] Failed to create window: {}", e),
                                        }
                                    });
                                }
                            }
                        }
                    }
                },
                WM_KEYUP | WM_SYSKEYUP => {
                    if vk == VK_SHIFT || vk == VK_LSHIFT || vk == VK_RSHIFT {
                        SHIFT_DOWN.store(false, Ordering::Relaxed);
                    }
                },
                _ => {}
            }
        }
        CallNextHookEx(std::ptr::null_mut(), code, w_param, l_param)
    }

    unsafe {
        let hook: HHOOK = SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(hook_proc),
            GetModuleHandleW(std::ptr::null()),
            0,
        );
        if hook.is_null() {
            println!("[Overlay] FAILED to install keyboard hook");
            return;
        }
        println!("[Overlay] Low-level keyboard hook installed successfully");
        // Message loop required for the hook to work
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn start_keyboard_hook(_app_handle: tauri::AppHandle) {}

mod emulators;
mod playtime;
mod discord_rpc;
mod achievements;
mod gamepad;
mod retroachievements;
mod cloud_backup;
mod dpapi;

fn write_to_boxart_log(message: &str) {
    let mut path = emuworld_base_dir();
    path.push("boxart_fetch.log");
    
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path) {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        // Compose line in memory to minimize interleaving
        let line = format!("[{}] {}\n", timestamp, message);
        let _ = file.write_all(line.as_bytes());
    }
}

/// App configuration stored as JSON
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub roms_directory: String,
    pub emulators_directory: String,
    pub covers_directory: String,
    #[serde(default)]
    pub bandwidth_limit_kbps: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        let base = emuworld_base_dir();
        Self {
            roms_directory: base.join("ROMs").to_string_lossy().to_string(),
            emulators_directory: base.join("Emulators").to_string_lossy().to_string(),
            covers_directory: base.join("Covers").to_string_lossy().to_string(),
            bandwidth_limit_kbps: 0,
        }
    }
}

fn config_path() -> PathBuf {
    emuworld_base_dir().join("config.json")
}

#[tauri::command]
fn is_portable_mode() -> bool {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            return dir.join("portable.txt").exists();
        }
    }
    false
}

#[tauri::command]
fn get_config() -> AppConfig {
    let path = config_path();
    let config = if path.exists() {
        let data = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        push_log("INFO", "Aucune config existante, utilisation des valeurs par défaut");
        AppConfig::default()
    };

    fs::create_dir_all(&config.roms_directory).ok();
    fs::create_dir_all(&config.emulators_directory).ok();
    fs::create_dir_all(&config.covers_directory).ok();
    BANDWIDTH_LIMIT_KBPS.store(config.bandwidth_limit_kbps, AtomicOrdering::Relaxed);

    config
}

#[tauri::command]
fn save_config(config: AppConfig) -> Result<(), String> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    BANDWIDTH_LIMIT_KBPS.store(config.bandwidth_limit_kbps, AtomicOrdering::Relaxed);
    let data = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    fs::write(&path, data).map_err(|e| e.to_string())?;
    push_log("INFO", &format!("Config sauvegardée (ROMs: {}, Émulateurs: {}, BW limit: {} KB/s)", config.roms_directory, config.emulators_directory, config.bandwidth_limit_kbps));
    Ok(())
}

#[tauri::command]
fn get_emulator_catalog() -> Vec<emulators::EmulatorInfo> {
    emulators::get_catalog()
}

#[tauri::command]
fn get_installed_emulators() -> Vec<String> {
    let config = get_config();
    let emu_dir = PathBuf::from(&config.emulators_directory);
    if !emu_dir.exists() {
        return vec![];
    }
    let mut installed = vec![];
    if let Ok(entries) = fs::read_dir(&emu_dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                let has_files = fs::read_dir(entry.path())
                    .map(|mut rd| rd.next().is_some())
                    .unwrap_or(false);
                if has_files {
                    if let Some(name) = entry.file_name().to_str() {
                        installed.push(name.to_lowercase());
                    }
                }
            }
        }
    }
    // Check retroarch-* emulators by core presence in shared retroarch/
    let ra_dir = emu_dir.join("retroarch");
    if ra_dir.exists() {
        if let Some(ra_exe) = find_executable(&ra_dir, "retroarch.exe") {
            let cores_dir = ra_exe.parent().unwrap_or(&ra_dir).join("cores");
            let catalog = emulators::get_catalog();
            for emu in &catalog {
                if emu.id.starts_with("retroarch-") && !installed.contains(&emu.id) {
                    if let Some(core) = &emu.core_name {
                        if cores_dir.join(core).exists() {
                            installed.push(emu.id.clone());
                        }
                    }
                }
            }
        }
    }
    installed
}

#[tauri::command]
async fn install_emulator(emulator_id: String, app_handle: tauri::AppHandle) -> Result<String, String> {
    let catalog = emulators::get_catalog();
    let emu = catalog
        .iter()
        .find(|e| e.id == emulator_id)
        .ok_or_else(|| format!("Emulator '{}' not found in catalog", emulator_id))?
        .clone();
    push_log("INFO", &format!("Installation de {} ({})...", emu.name, emu.id));

    let config = get_config();
    let is_shared_retroarch = emu.id.starts_with("retroarch-");
    let install_dir = if is_shared_retroarch {
        PathBuf::from(&config.emulators_directory).join("retroarch")
    } else {
        PathBuf::from(&config.emulators_directory).join(&emu.id)
    };

    let _ = app_handle.emit("install-progress", serde_json::json!({
        "emulator_id": emulator_id,
        "status": "downloading",
        "progress": 5
    }));

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    // For shared RetroArch: only download base if not already installed
    let ra_already_installed = is_shared_retroarch && find_executable(&install_dir, "retroarch.exe").is_some();

    // If installing a retroarch-* core but RetroArch base is not installed, install RetroArch first
    if is_shared_retroarch && !ra_already_installed {
        push_log("INFO", "RetroArch non installé — installation automatique de la base RetroArch...");
        let _ = app_handle.emit("install-progress", serde_json::json!({
            "emulator_id": emulator_id,
            "status": "downloading",
            "progress": 10,
            "message": "Installing RetroArch base..."
        }));

        fs::create_dir_all(&install_dir).map_err(|e| format!("Failed to create directory: {}", e))?;

        // Download RetroArch base using streaming to avoid timeout on large files
        let ra_url = "https://buildbot.libretro.com/stable/1.20.0/windows/x86_64/RetroArch.7z";
        push_log("INFO", &format!("Téléchargement RetroArch depuis: {}", ra_url));

        let response = client
            .get(ra_url)
            .header("User-Agent", "EmuWorld/2.0 (Windows; Desktop)")
            .send()
            .await
            .map_err(|e| format!("RetroArch download failed: {}", e))?;

        if !response.status().is_success() {
            push_log("ERROR", &format!("RetroArch download HTTP {}", response.status()));
            return Err(format!("RetroArch download failed with HTTP status: {}", response.status()));
        }

        let total_size = response.content_length().unwrap_or(0);
        push_log("INFO", &format!("RetroArch taille: {} Mo", total_size / 1_048_576));

        // Stream to disk to avoid memory issues with large 7z files
        let archive_path = install_dir.join("archive.7z");
        {
            let mut file = fs::File::create(&archive_path)
                .map_err(|e| format!("Failed to create archive file: {}", e))?;
            let mut downloaded: u64 = 0;
            let mut last_emit = std::time::Instant::now();
            let mut stream = response;
            while let Some(chunk) = stream.chunk().await.map_err(|e| format!("Download stream error: {}", e))? {
                file.write_all(&chunk).map_err(|e| format!("Write error: {}", e))?;
                downloaded += chunk.len() as u64;
                if last_emit.elapsed().as_millis() >= 500 {
                    let progress = if total_size > 0 {
                        (downloaded as f64 / total_size as f64 * 50.0) as u32 + 10
                    } else { 30 };
                    let _ = app_handle.emit("install-progress", serde_json::json!({
                        "emulator_id": emulator_id,
                        "status": "downloading",
                        "progress": progress
                    }));
                    last_emit = std::time::Instant::now();
                }
            }
        }

        push_log("INFO", "RetroArch téléchargé, extraction en cours...");
        let _ = app_handle.emit("install-progress", serde_json::json!({
            "emulator_id": emulator_id,
            "status": "extracting",
            "progress": 60
        }));

        extract_7z(&archive_path, &install_dir).map_err(|e| {
            push_log("ERROR", &format!("Extraction RetroArch échouée: {}", e));
            format!("RetroArch 7z extraction failed: {}", e)
        })?;

        fs::remove_file(&archive_path).ok();

        if find_executable(&install_dir, "retroarch.exe").is_none() {
            push_log("ERROR", "retroarch.exe introuvable après extraction");
            return Err("RetroArch installation failed: retroarch.exe not found after extraction.".to_string());
        }
        push_log("INFO", "RetroArch base installé avec succès");
    }

    if !ra_already_installed && !is_shared_retroarch {
        // Standalone emulator install
        if install_dir.exists() {
            fs::remove_dir_all(&install_dir).ok();
        }
        fs::create_dir_all(&install_dir).map_err(|e| format!("Failed to create directory: {}", e))?;

        let _ = app_handle.emit("install-progress", serde_json::json!({
            "emulator_id": emulator_id,
            "status": "downloading",
            "progress": 10
        }));

        let response = client
            .get(&emu.download_url)
            .header("User-Agent", "EmuWorld/2.0 (Windows; Desktop)")
            .send()
            .await
            .map_err(|e| format!("Download failed: {}", e))?;

        if !response.status().is_success() {
            push_log("ERROR", &format!("Download échoué HTTP {}: {}", response.status(), emu.download_url));
            return Err(format!("Download failed with HTTP status: {}", response.status()));
        }

        let total_size = response.content_length().unwrap_or(0);
        const MAX_DOWNLOAD_SIZE: u64 = 1_500_000_000; // 1.5 GB max for emulators

        if total_size > MAX_DOWNLOAD_SIZE {
            return Err(format!("Download too large ({} MB) — max {} MB", total_size / 1_048_576, MAX_DOWNLOAD_SIZE / 1_048_576));
        }

        // Stream to disk for large files (>50 MB), buffer in memory for small ones
        let archive_ext = if emu.archive_type == "7z" { "7z" } else { "zip" };
        let archive_path = install_dir.join(format!("archive.{}", archive_ext));

        if total_size > 50_000_000 {
            push_log("INFO", &format!("Gros fichier ({} Mo), téléchargement en streaming...", total_size / 1_048_576));
            let mut file = fs::File::create(&archive_path)
                .map_err(|e| format!("Failed to create archive file: {}", e))?;
            let mut downloaded: u64 = 0;
            let mut last_emit = std::time::Instant::now();
            let mut stream = response;
            while let Some(chunk) = stream.chunk().await.map_err(|e| format!("Download stream error: {}", e))? {
                file.write_all(&chunk).map_err(|e| format!("Write error: {}", e))?;
                downloaded += chunk.len() as u64;
                if downloaded > MAX_DOWNLOAD_SIZE {
                    drop(file);
                    let _ = fs::remove_file(&archive_path);
                    return Err("Download exceeded maximum size limit".to_string());
                }
                if last_emit.elapsed().as_millis() >= 500 {
                    let progress = if total_size > 0 {
                        (downloaded as f64 / total_size as f64 * 50.0) as u32 + 10
                    } else { 30 };
                    let _ = app_handle.emit("install-progress", serde_json::json!({
                        "emulator_id": emulator_id,
                        "status": "downloading",
                        "progress": progress
                    }));
                    last_emit = std::time::Instant::now();
                }
            }
        } else {
            let bytes = response.bytes().await.map_err(|e| format!("Failed to read download data: {}", e))?;
            if bytes.is_empty() {
                return Err("Downloaded file is empty".to_string());
            }
            if bytes.len() as u64 > MAX_DOWNLOAD_SIZE {
                return Err("Downloaded file exceeds maximum size limit".to_string());
            }
            fs::write(&archive_path, &bytes).map_err(|e| format!("Failed to save archive: {}", e))?;
        }

        let _ = app_handle.emit("install-progress", serde_json::json!({
            "emulator_id": emulator_id,
            "status": "extracting",
            "progress": 60
        }));

        push_log("INFO", &format!("Extraction de l'archive ({})...", archive_ext));
        if emu.archive_type == "zip" {
            extract_zip(&archive_path, &install_dir).map_err(|e| {
                push_log("ERROR", &format!("Extraction zip échouée: {}", e));
                format!("Zip extraction failed: {}", e)
            })?;
        } else if emu.archive_type == "7z" {
            extract_7z(&archive_path, &install_dir).map_err(|e| {
                push_log("ERROR", &format!("Extraction 7z échouée: {}", e));
                format!("7z extraction failed: {}", e)
            })?;
        } else {
            return Err(format!("Unsupported archive type: {}", emu.archive_type));
        }

        fs::remove_file(&archive_path).ok();
    } else if ra_already_installed {
        // RetroArch already installed, skip to core download
        push_log("INFO", "RetroArch déjà installé, passage au téléchargement du core...");
        let _ = app_handle.emit("install-progress", serde_json::json!({
            "emulator_id": emulator_id,
            "status": "extracting",
            "progress": 70
        }));
    }

    // Verify executable exists
    if find_executable(&install_dir, &emu.executable_name).is_none() {
        return Err(format!("Installation failed: Executable '{}' not found in the extracted files.", emu.executable_name));
    }

    // Download setup files (keys, firmware, BIOS, cores)
    if !emu.setup_files.is_empty() {
        push_log("INFO", &format!("Téléchargement de {} fichier(s) de setup...", emu.setup_files.len()));
        let _ = app_handle.emit("install-progress", serde_json::json!({
            "emulator_id": emulator_id,
            "status": "setup",
            "progress": 85
        }));

        // Resolve setup file paths relative to the exe's parent dir (handles nested archive folders)
        let exe_base_dir = find_executable(&install_dir, &emu.executable_name)
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .unwrap_or_else(|| install_dir.clone());
        push_log("INFO", &format!("Setup base dir: {}", exe_base_dir.display()));

        for sf in &emu.setup_files {
            if sf.url.starts_with("PLACEHOLDER") { continue; }
            let dest_path = exe_base_dir.join(&sf.dest);
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent).ok();
            }
            push_log("INFO", &format!("Setup: {} → {} (extract: {})", sf.url, dest_path.display(), sf.extract));
            match client.get(&sf.url)
                .header("User-Agent", "EmuWorld/2.0 (Windows; Desktop)")
                .send().await
            {
                Ok(resp) => {
                    if resp.status().is_success() {
                        match resp.bytes().await {
                            Ok(data) => {
                                push_log("INFO", &format!("Setup: téléchargé {} octets pour {}", data.len(), sf.dest));
                                // Verify SHA256 integrity if expected hash is set
                                if let Some(ref expected) = sf.expected_sha256 {
                                    use sha2::Digest;
                                    let mut hasher = sha2::Sha256::new();
                                    hasher.update(&data);
                                    let actual = format!("{:x}", hasher.finalize());
                                    if actual != expected.to_lowercase() {
                                        push_log("ERROR", &format!("Setup: SHA256 mismatch for {} — expected {} got {}", sf.dest, expected, actual));
                                        continue;
                                    }
                                    push_log("INFO", &format!("Setup: SHA256 verified for {}", sf.dest));
                                }
                                if sf.extract {
                                    let tmp_zip = install_dir.join("_setup_tmp.zip");
                                    if fs::write(&tmp_zip, &data).is_ok() {
                                        fs::create_dir_all(&dest_path).ok();
                                        match extract_zip(&tmp_zip, &dest_path) {
                                            Ok(_) => push_log("INFO", &format!("Setup: extrait dans {}", dest_path.display())),
                                            Err(e) => push_log("ERROR", &format!("Setup: extraction échouée pour {}: {}", sf.dest, e)),
                                        }
                                        fs::remove_file(&tmp_zip).ok();
                                    }
                                } else {
                                    let _ = fs::write(&dest_path, &data);
                                    push_log("INFO", &format!("Setup: fichier écrit: {}", dest_path.display()));
                                }
                            }
                            Err(e) => push_log("ERROR", &format!("Setup: lecture échouée pour {}: {}", sf.dest, e)),
                        }
                    } else {
                        push_log("ERROR", &format!("Setup: HTTP {} pour {}", resp.status(), sf.url));
                    }
                }
                Err(e) => push_log("ERROR", &format!("Setup: téléchargement échoué pour {}: {}", sf.dest, e)),
            }
        }

        // RPCS3: auto-install firmware after download
        if emu.id == "rpcs3" {
            let fw_path = exe_base_dir.join("PS3UPDAT.PUP");
            if fw_path.exists() {
                if let Some(rpcs3_exe) = find_executable(&install_dir, "rpcs3.exe") {
                    println!("[Setup] Installing PS3 firmware via rpcs3 --installfw...");
                    let mut fw_cmd = Command::new(&rpcs3_exe);
                    fw_cmd.arg("--installfw").arg(&fw_path);
                    #[cfg(target_os = "windows")]
                    {
                        use std::os::windows::process::CommandExt;
                        fw_cmd.creation_flags(0x08000000);
                    }
                    match fw_cmd.output() {
                        Ok(out) => {
                            if out.status.success() {
                                println!("[Setup] PS3 firmware installed successfully");
                                fs::remove_file(&fw_path).ok();
                            } else {
                                let stderr = String::from_utf8_lossy(&out.stderr);
                                println!("[Setup] Firmware install failed: {}", stderr);
                            }
                        }
                        Err(e) => println!("[Setup] Failed to run rpcs3 --installfw: {}", e),
                    }
                }
            }
        }
    }

    let _ = app_handle.emit("install-progress", serde_json::json!({
        "emulator_id": emulator_id,
        "status": "done",
        "progress": 100
    }));

    push_log("INFO", &format!("{} installé avec succès", emu.name));
    Ok(format!("{} installed successfully!", emu.name))
}

fn extract_zip(archive_path: &PathBuf, install_dir: &PathBuf) -> Result<(), String> {
    let file = fs::File::open(archive_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let outpath = match entry.enclosed_name() {
            Some(name) => install_dir.join(name),
            None => continue,
        };
        if entry.is_dir() {
            fs::create_dir_all(&outpath).ok();
        } else {
            if let Some(p) = outpath.parent() { fs::create_dir_all(p).ok(); }
            let mut outfile = fs::File::create(&outpath).map_err(|e| e.to_string())?;
            std::io::copy(&mut entry, &mut outfile).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn extract_7z(archive_path: &PathBuf, install_dir: &PathBuf) -> Result<(), String> {
    // Look for bundled 7za.exe next to our executable first, then system 7-Zip
    let mut candidates: Vec<PathBuf> = Vec::new();

    // Bundled 7za.exe (shipped with EmuWorld)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            candidates.push(exe_dir.join("7za.exe"));
            candidates.push(exe_dir.join("binaries").join("7za.exe"));
        }
    }
    // System-installed 7-Zip
    candidates.push(PathBuf::from(r"C:\Program Files\7-Zip\7z.exe"));
    candidates.push(PathBuf::from(r"C:\Program Files (x86)\7-Zip\7z.exe"));

    for sz_path in &candidates {
        if sz_path.exists() {
            println!("[Extract] Using 7z: {}", sz_path.display());
            let mut sz_cmd = Command::new(sz_path);
            sz_cmd.args(&["x", "-y", &format!("-o{}", install_dir.display())])
                .arg(archive_path)
                .current_dir(sz_path.parent().unwrap_or(install_dir));
            #[cfg(target_os = "windows")]
            {
                use std::os::windows::process::CommandExt;
                sz_cmd.creation_flags(0x08000000);
            }
            let output = sz_cmd.output()
                .map_err(|e| format!("7z exec failed: {}", e))?;
            if output.status.success() {
                return Ok(());
            }
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!("[Extract] {} failed ({}), trying next...", sz_path.display(), stderr.trim());
        }
    }
    // Fallback to pure-Rust (slower but always works)
    println!("[Extract] No native 7z found, using sevenz-rust (this may be slow for large files)...");
    sevenz_rust::decompress_file(archive_path, install_dir).map_err(|e| e.to_string())
}

#[tauri::command]
fn uninstall_emulator(emulator_id: String) -> Result<String, String> {
    push_log("INFO", &format!("Désinstallation de l'émulateur: {}", emulator_id));
    let id_lower = emulator_id.to_lowercase();
    let config = get_config();
    let catalog = emulators::get_catalog();

    // For retroarch-* emulators: only remove the core DLL from the shared retroarch folder
    if id_lower.starts_with("retroarch-") && id_lower != "retroarch" {
        let ra_dir = PathBuf::from(&config.emulators_directory).join("retroarch");
        if let Some(emu) = catalog.iter().find(|e| e.id == id_lower) {
            if let Some(core) = &emu.core_name {
                if let Some(ra_exe) = find_executable(&ra_dir, "retroarch.exe") {
                    let cores_dir = ra_exe.parent().unwrap_or(&ra_dir).join("cores");
                    let core_path = cores_dir.join(core);
                    if core_path.exists() {
                        fs::remove_file(&core_path).ok();
                    }
                }
            }
        }
        // Also remove old standalone folder if it exists
        let old_dir = PathBuf::from(&config.emulators_directory).join(&id_lower);
        if old_dir.exists() {
            fs::remove_dir_all(&old_dir).ok();
        }
        return Ok(format!("Emulator '{}' uninstalled (core removed)", id_lower));
    }

    let install_dir = PathBuf::from(&config.emulators_directory).join(&id_lower);
    if install_dir.exists() {
        fs::remove_dir_all(&install_dir).map_err(|e| {
            if e.to_string().contains("Access is denied") {
                format!("Uninstallation failed: The emulator folder is locked. Please make sure the emulator is closed before uninstalling.")
            } else {
                format!("Failed to uninstall {}: {}", id_lower, e)
            }
        })?;
        Ok(format!("Emulator '{}' uninstalled", id_lower))
    } else {
        Err(format!("Emulator '{}' is not installed (checked {})", id_lower, install_dir.display()))
    }
}

#[tauri::command]
async fn launch_emulator(
    app_handle: tauri::AppHandle,
    emulator_id: String,
    rom_path: Option<String>,
    rom_name: Option<String>,
    rom_console: Option<String>,
) -> Result<String, String> {
    push_log("INFO", &format!("Lancement émulateur: {} (ROM: {:?}, jeu: {:?}, console: {:?})", emulator_id, rom_path, rom_name, rom_console));
    let catalog = emulators::get_catalog();
    let emu = catalog.iter().find(|e| e.id == emulator_id).ok_or_else(|| "Emulator not found".to_string())?.clone();
    let config = get_config();

    // Check if we should redirect to RetroArch for RA support
    let ra_config = retroachievements::load_config();
    let ra_active = !ra_config.token.is_empty() && !ra_config.username.is_empty();
    let ra_core = retroachievements::retroarch_core_for_emulator(&emu.id);
    let use_retroarch = ra_active && ra_core.is_some() && rom_path.is_some();

    let (install_dir, exe_path, effective_id) = if use_retroarch {
        let ra_dir = PathBuf::from(&config.emulators_directory).join("retroarch");
        let ra_exe = find_executable(&ra_dir, "retroarch.exe");
        if let Some(exe) = ra_exe {
            println!("[Launch] RA active — redirecting {} to RetroArch", emu.id);
            let effective_ra_dir = exe.parent().unwrap_or(&ra_dir).to_path_buf();
            // Auto-download missing RA core if needed
            if let Some(core_name) = ra_core {
                let cores_dir = effective_ra_dir.join("cores");
                fs::create_dir_all(&cores_dir).ok();
                if !cores_dir.join(core_name).exists() {
                    println!("[Launch] Core '{}' missing — downloading...", core_name);
                    let correct_url = format!("https://buildbot.libretro.com/nightly/windows/x86_64/latest/{}.zip", core_name);
                    if let Ok(dl_client) = reqwest::Client::builder().timeout(std::time::Duration::from_secs(30)).build() {
                        if let Ok(resp) = dl_client.get(&correct_url).send().await {
                            if resp.status().is_success() {
                                if let Ok(data) = resp.bytes().await {
                                    let tmp = effective_ra_dir.join("_core_tmp.zip");
                                    if fs::write(&tmp, &data).is_ok() {
                                        let _ = extract_zip(&tmp, &cores_dir);
                                        fs::remove_file(&tmp).ok();
                                        println!("[Launch] Core '{}' installed", core_name);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // Inject RA credentials into retroarch.cfg next to the exe
            let cfg_path = effective_ra_dir.join("retroarch.cfg");
            let cfg_content = fs::read_to_string(&cfg_path).unwrap_or_default();
            let new_cfg = retroachievements::inject_retroarch_cheevos_pub(
                &cfg_content, &ra_config.username, &ra_config.token
            );
            let _ = fs::write(&cfg_path, new_cfg);
            (effective_ra_dir, exe, "retroarch".to_string())
        } else {
            // RetroArch not installed, fall back to standalone
            println!("[Launch] RA active but RetroArch not installed — using standalone");
            let dir = PathBuf::from(&config.emulators_directory).join(&emu.id);
            let exe = find_executable(&dir, &emu.executable_name)
                .ok_or_else(|| format!("Executable '{}' not found.", emu.executable_name))?;
            (dir, exe, emu.id.clone())
        }
    } else {
        // For retroarch-* emulators, use the shared retroarch/ folder
        let dir = if emu.id.starts_with("retroarch-") {
            let shared_dir = PathBuf::from(&config.emulators_directory).join("retroarch");
            if find_executable(&shared_dir, &emu.executable_name).is_some() {
                // Auto-download missing core for retroarch-* emulators (even without RA linked)
                if let Some(core_name) = &emu.core_name {
                    let cores_dir = shared_dir.join("cores");
                    fs::create_dir_all(&cores_dir).ok();
                    if !cores_dir.join(core_name).exists() {
                        println!("[Launch] Core '{}' missing for {} — downloading...", core_name, emu.id);
                        push_log("INFO", &format!("Téléchargement du core {} en cours...", core_name));
                        let core_url = format!("https://buildbot.libretro.com/nightly/windows/x86_64/latest/{}.zip", core_name);
                        if let Ok(dl_client) = reqwest::Client::builder().timeout(std::time::Duration::from_secs(30)).build() {
                            if let Ok(resp) = dl_client.get(&core_url).send().await {
                                if resp.status().is_success() {
                                    if let Ok(data) = resp.bytes().await {
                                        let tmp = shared_dir.join("_core_tmp.zip");
                                        if fs::write(&tmp, &data).is_ok() {
                                            let _ = extract_zip(&tmp, &cores_dir);
                                            fs::remove_file(&tmp).ok();
                                            println!("[Launch] Core '{}' installed for {}", core_name, emu.id);
                                            push_log("INFO", &format!("Core {} installé", core_name));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                shared_dir
            } else {
                PathBuf::from(&config.emulators_directory).join(&emu.id)
            }
        } else {
            PathBuf::from(&config.emulators_directory).join(&emu.id)
        };
        let exe = find_executable(&dir, &emu.executable_name)
            .ok_or_else(|| format!("Executable '{}' not found.", emu.executable_name))?;
        (dir, exe, emu.id.clone())
    };

    let mut cmd = Command::new(&exe_path);
    cmd.current_dir(exe_path.parent().unwrap_or(&install_dir));
    if let Some(rom) = rom_path.clone() {
        let mut final_path = rom.replace(r"\\?\", "").replace("/", "\\");

        // CD-i: auto-convert .cue to .chd if needed (SAME CDi core only supports CHD)
        if emu.id == "retroarch-cdi" && final_path.to_lowercase().ends_with(".cue") {
            let chd_path = format!("{}.chd", &final_path[..final_path.len() - 4]);
            if !PathBuf::from(&chd_path).exists() {
                println!("[Launch] Converting CUE to CHD for CD-i: {}", final_path);
                push_log("INFO", "Conversion CUE → CHD en cours...");
                let chdman = std::env::current_exe().ok()
                    .and_then(|p| p.parent().map(|d| d.join("binaries").join("chdman.exe")));
                if let Some(chdman_path) = chdman {
                    if chdman_path.exists() {
                        let mut conv_cmd = Command::new(&chdman_path);
                        conv_cmd.args(&["createcd", "-i", &final_path, "-o", &chd_path]);
                        #[cfg(target_os = "windows")]
                        {
                            use std::os::windows::process::CommandExt;
                            conv_cmd.creation_flags(0x08000000);
                        }
                        match conv_cmd.output() {
                            Ok(out) if out.status.success() => {
                                println!("[Launch] CHD conversion successful");
                                final_path = chd_path;
                            }
                            Ok(out) => {
                                let stderr = String::from_utf8_lossy(&out.stderr);
                                println!("[Launch] CHD conversion failed: {}", stderr);
                            }
                            Err(e) => println!("[Launch] Failed to run chdman: {}", e),
                        }
                    }
                }
            } else {
                final_path = chd_path;
            }
        }
        println!("[Launch] Running: {:?} with Arg: {:?}", exe_path, final_path);
        push_log("INFO", &format!("Lancement: {} via {}", final_path.split('\\').last().unwrap_or(&final_path), effective_id));

        // Handle RetroArch cores — either from RA redirect or from retroarch-* emulators
        if use_retroarch {
            if let Some(core_name) = ra_core {
                let cores_dir = install_dir.join("cores");
                let core_path = cores_dir.join(core_name);
                if core_path.exists() {
                    println!("[Launch] Using RA core: {:?}", core_path);
                    cmd.arg("-L");
                    cmd.arg(&core_path);
                } else {
                    println!("[Launch] WARNING: Core '{}' not found in cores/, launching without -L", core_name);
                }
            }
        } else if effective_id.starts_with("retroarch") {
            if let Some(core) = &emu.core_name {
                let exe_dir = exe_path.parent().unwrap_or(&install_dir).to_path_buf();
                let core_search_path = if exe_dir.join("cores").exists() { exe_dir.clone() } else { install_dir.clone() };
                if let Some(core_path) = find_executable(&core_search_path, core) {
                    println!("[Launch] Detected RetroArch core: {:?}", core_path);
                    cmd.arg("-L");
                    cmd.arg(core_path);
                } else {
                    println!("[Launch] WARNING: Core '{}' not found in {}", core, core_search_path.display());
                }
            }
        }

        // Cemu requires -g flag to launch a game
        if emu.id == "cemu" {
            cmd.arg("-g");
        }

        cmd.arg(&final_path);
    }

    // Force borderless windowed fullscreen for RetroArch (no --fullscreen CLI flag,
    // as that triggers exclusive fullscreen which minimizes on focus loss)
    if effective_id == "retroarch" || effective_id.starts_with("retroarch-") {
        let ra_dir = exe_path.parent().unwrap_or(&install_dir);
        let cfg_path = ra_dir.join("retroarch.cfg");
        let mut cfg = fs::read_to_string(&cfg_path).unwrap_or_default();
        let settings = [
            ("video_fullscreen", "true"),
            ("video_windowed_fullscreen", "true"),
            ("pause_nonactive", "false"),
            ("menu_pause_libretro", "false"),
        ];
        for (key, val) in settings {
            let pattern = format!("{} = ", key);
            if let Some(pos) = cfg.find(&pattern) {
                let end = cfg[pos..].find('\n').map(|p| pos + p).unwrap_or(cfg.len());
                cfg.replace_range(pos..end, &format!("{} = \"{}\"", key, val));
            } else {
                cfg.push_str(&format!("\n{} = \"{}\"\n", key, val));
            }
        }
        let _ = fs::write(&cfg_path, &cfg);
        // No --fullscreen arg: config handles it via borderless windowed mode
    } else {
        match effective_id.as_str() {
            "dolphin" => { if rom_path.is_some() { cmd.arg("-b"); } },
            "cemu" => { if rom_path.is_some() { cmd.arg("-f"); } },
            "ppsspp" => { cmd.arg("--fullscreen"); },
            "duckstation" => { cmd.arg("-fullscreen"); },
            "pcsx2" => { cmd.arg("-fullscreen"); },
            "mgba" => { cmd.arg("-f"); },
            "rpcs3" => { if rom_path.is_some() { cmd.arg("--no-gui"); cmd.arg("--fullscreen"); } },
            "ryubing" => { cmd.arg("--fullscreen"); },
            "azahar" => { /* no fullscreen flag — azahar doesn't support CLI fullscreen */ },
            "melonds" => { cmd.arg("--fullscreen"); },
            "project64" => { cmd.arg("--fullscreen"); },
            "xemu" => { cmd.arg("-full-screen"); },
            "xenia" => { cmd.arg("--fullscreen"); },
            "flycast" => { cmd.arg("--config"); cmd.arg("window:fullscreen=yes"); },
            _ => {}
        }
    }

    // xemu: copy BIOS files to %APPDATA%/xemu/xemu/ and configure xemu.toml with dvd_path
    if effective_id == "xemu" {
        if let Some(appdata) = dirs::config_dir() {
            let xemu_dir = appdata.join("xemu").join("xemu");
            fs::create_dir_all(&xemu_dir).ok();
            let exe_dir = exe_path.parent().unwrap_or(&install_dir);
            for file in &["mcpx_1.0.bin", "Complex_4627v1.03.bin", "xbox_hdd.qcow2"] {
                let src = exe_dir.join(file);
                let dst = xemu_dir.join(file);
                if src.exists() && !dst.exists() {
                    fs::copy(&src, &dst).ok();
                }
            }
            let toml_path = xemu_dir.join("xemu.toml");
            let mcpx = xemu_dir.join("mcpx_1.0.bin").to_string_lossy().to_string();
            let flash = xemu_dir.join("Complex_4627v1.03.bin").to_string_lossy().to_string();
            let hdd = xemu_dir.join("xbox_hdd.qcow2").to_string_lossy().to_string();
            let eeprom = xemu_dir.join("eeprom.bin").to_string_lossy().to_string();
            let dvd = rom_path.clone().unwrap_or_default();
            let toml_content = format!(
                "[general]\nshow_welcome = false\n\n[general.ui]\nfit = 'stretch'\nanimation = 'fadeout'\n\n[sys]\nmem_limit = '128'\n\n[sys.files]\nbootrom_path = '{}'\nflashrom_path = '{}'\nhdd_path = '{}'\neeprom_path = '{}'\ndvd_path = '{}'\n",
                mcpx, flash, hdd, eeprom, dvd
            );
            let _ = fs::write(&toml_path, &toml_content);
        }
        // xemu reads the game from xemu.toml, remove the rom arg from cmd
        // Rebuild cmd without the rom path argument
        cmd = Command::new(&exe_path);
        cmd.current_dir(exe_path.parent().unwrap_or(&install_dir));
        cmd.arg("-full-screen");
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            println!("[Launch] ERROR spawning process: {}", e);
            push_log("ERROR", &format!("Échec lancement émulateur: {}", e));
            return Err(format!("Could not start emulator: {}", e));
        }
    };

    println!("[Launch] Success!");
    push_log("INFO", &format!("Émulateur {} démarré avec succès", emu.name));
    let launched_name = emu.name.clone();

    // Track playtime only when we have a ROM context (launching the bare emulator doesn't count as a game session).
    if let (Some(name), Some(console)) = (rom_name.clone(), rom_console.clone()) {
        let emulator_id_for_task = emu.id.clone();
        let exe_name = emu.executable_name.clone();

        // Save current session to disk for crash recovery
        let session_file = emuworld_base_dir().join("current_session.json");
        let start_epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let _ = fs::write(&session_file, serde_json::json!({
            "game": &name, "console": &console, "emulator": &emulator_id_for_task,
            "start_epoch": start_epoch
        }).to_string());

        // Wait for the child to exit on a blocking thread, then record the session.
        tauri::async_runtime::spawn_blocking(move || {
            use tauri::Emitter;
            let start = std::time::Instant::now();
            match child.wait() {
                Ok(status) => println!("[Launch] Child exited ({:?}) for {}", status, name),
                Err(e) => println!("[Launch] wait() failed: {}", e),
            }

            // If the process exited very quickly (<120s), check if a successor process
            // with the same exe name is still running (handles RPCS3 process re-launch)
            if start.elapsed().as_secs() < 120 {
                let poll_interval = std::time::Duration::from_secs(5);
                let max_wait = std::time::Duration::from_secs(28800); // 8h max
                let deadline = std::time::Instant::now() + max_wait;
                std::thread::sleep(std::time::Duration::from_secs(2)); // brief grace period
                while std::time::Instant::now() < deadline {
                    let output = std::process::Command::new("tasklist")
                        .args(["/FI", &format!("IMAGENAME eq {}", exe_name), "/FO", "CSV", "/NH"])
                        .output();
                    match output {
                        Ok(out) => {
                            let stdout = String::from_utf8_lossy(&out.stdout);
                            if !stdout.to_lowercase().contains(&exe_name.to_lowercase()) {
                                break; // process gone
                            }
                        }
                        Err(_) => break,
                    }
                    std::thread::sleep(poll_interval);
                }
            }

            let elapsed = start.elapsed().as_secs();
            // Ignore sessions < 3s (likely the emulator crashed or the user mis-clicked).
            if elapsed >= 3 {
                if let Err(e) = playtime::record_session(&console, &name, elapsed, &emulator_id_for_task) {
                    println!("[Playtime] record failed: {}", e);
                }
            }
            // Remove session file on normal exit
            let _ = fs::remove_file(emuworld_base_dir().join("current_session.json"));
            let _ = app_handle.emit("game-closed", serde_json::json!({
                "console": console,
                "name": name,
                "seconds": elapsed,
            }));
        });
    } else {
        // No ROM context: just drop the child into its own thread so we don't leak a zombie.
        tauri::async_runtime::spawn_blocking(move || {
            let _ = child.wait();
        });
    }

    Ok(format!("Launched {}", launched_name))
}

fn find_executable(dir: &PathBuf, name: &str) -> Option<PathBuf> {
    let target_name = name.to_lowercase();
    
    // 1. Direct match (try original and lowercase)
    let direct = dir.join(name);
    if direct.exists() { return Some(direct); }
    let direct_lower = dir.join(&target_name);
    if direct_lower.exists() { return Some(direct_lower); }

    // 2. Recursive search
    for entry in walkdir::WalkDir::new(dir).max_depth(5) {
        if let Ok(e) = entry {
            if e.file_type().is_file() {
                let file_name = e.file_name().to_string_lossy().to_lowercase();
                if file_name == target_name {
                    return Some(e.path().to_path_buf());
                }
            }
        }
    }

    // 3. Fallback: first .exe that isn't an uninstaller
    for entry in walkdir::WalkDir::new(dir).max_depth(5) {
        if let Ok(e) = entry {
            if e.file_type().is_file() {
                let file_name = e.file_name().to_string_lossy().to_lowercase();
                if file_name.ends_with(".exe") && !file_name.contains("uninstall") && !file_name.contains("setup") {
                    return Some(e.path().to_path_buf());
                }
            }
        }
    }
    None
}

static SCAN_CANCELLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[tauri::command]
fn cancel_scan() {
    push_log("INFO", "Scan ROMs annulé par l'utilisateur");
    SCAN_CANCELLED.store(true, std::sync::atomic::Ordering::Relaxed);
}

#[tauri::command]
fn scan_roms(directory: String) -> Vec<RomFile> {
    SCAN_CANCELLED.store(false, std::sync::atomic::Ordering::Relaxed);
    push_log("INFO", &format!("Scan ROMs démarré dans: {}", directory));
    let catalog = emulators::get_catalog();
    let mut roms = vec![];
    let dir = PathBuf::from(&directory);
    if !dir.exists() {
        push_log("WARN", &format!("Dossier ROMs inexistant: {}", directory));
        return roms;
    }

    // PS3 folder-based game detection: look for directories containing PS3_DISC.SFB
    let ps3_dir = dir.join("PlayStation 3");
    if ps3_dir.exists() {
        if let Ok(entries) = fs::read_dir(&ps3_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && path.join("PS3_DISC.SFB").exists() {
                    let name = path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                    let eboot = path.join("PS3_GAME").join("USRDIR").join("EBOOT.BIN");
                    let launch_path = if eboot.exists() { eboot } else { path.clone() };
                    roms.push(RomFile {
                        name,
                        path: launch_path.to_string_lossy().to_string(),
                        console: "PlayStation 3".to_string(),
                        emulator_id: "rpcs3".to_string(),
                        extension: "bin".to_string(),
                    });
                }
            }
        }
    }

    for entry in walkdir::WalkDir::new(&dir).max_depth(5) {
        if SCAN_CANCELLED.load(std::sync::atomic::Ordering::Relaxed) { break; }
        if let Ok(e) = entry {
            if e.file_type().is_file() {
                if let Some(ext) = e.path().extension() {
                    let ext_str = ext.to_string_lossy().to_lowercase();

                    // Skip EBOOT.BIN files inside PS3 game folders (handled above)
                    let path_str = e.path().to_string_lossy().to_lowercase();
                    if ext_str == "bin" && (path_str.contains("ps3_game") || path_str.contains("usrdir")) {
                        continue;
                    }

                    // Skip .bin/.raw track files when a .cue or .gdi exists in the same folder
                    // (CD-based games: Dreamcast, Saturn, Sega CD, PS1)
                    if ext_str == "bin" || ext_str == "raw" {
                        if let Some(parent) = e.path().parent() {
                            let has_cue_or_gdi = parent.read_dir().map(|rd| {
                                rd.flatten().any(|f| {
                                    let fe = f.path().extension().map(|x| x.to_string_lossy().to_lowercase()).unwrap_or_default();
                                    fe == "cue" || fe == "gdi"
                                })
                            }).unwrap_or(false);
                            if has_cue_or_gdi {
                                continue;
                            }
                        }
                    }

                    // Skip .cue when a .gdi exists in the same folder (prefer .gdi for Dreamcast)
                    if ext_str == "cue" {
                        if let Some(parent) = e.path().parent() {
                            let has_gdi = parent.read_dir().map(|rd| {
                                rd.flatten().any(|f| {
                                    f.path().extension().map(|x| x.to_string_lossy().to_lowercase() == "gdi").unwrap_or(false)
                                })
                            }).unwrap_or(false);
                            if has_gdi {
                                continue;
                            }
                        }
                    }

                    // Delete junk files found in ROM folders
                    let file_name_lower = e.path().file_name()
                        .map(|f| f.to_string_lossy().to_lowercase()).unwrap_or_default();
                    if file_name_lower == "vimm's lair.txt" || file_name_lower == "vimm.txt" {
                        let _ = fs::remove_file(e.path());
                        continue;
                    }

                    // Skip/delete .md files (markdown) — NOT Mega Drive ROMs
                    if ext_str == "md" {
                        let in_megadrive_folder = e.path().strip_prefix(&dir).ok()
                            .and_then(|rel| rel.components().next())
                            .map(|c| {
                                let f = c.as_os_str().to_string_lossy().to_lowercase();
                                f.contains("mega") || f.contains("genesis") || f == "md"
                            }).unwrap_or(false);
                        if !in_megadrive_folder {
                            // It's a markdown file, delete it
                            let _ = fs::remove_file(e.path());
                            continue;
                        }
                    }

                    // Try to infer console from parent folder name (matches target_console from Vimm or user-created folders)
                    let folder_console = e.path().strip_prefix(&dir).ok()
                        .and_then(|rel| rel.components().next())
                        .and_then(|c| {
                            let folder = c.as_os_str().to_string_lossy().to_string();
                            let normalized = normalize_console_folder(&folder);
                            catalog.iter().find(|emu| {
                                (emu.console.eq_ignore_ascii_case(&folder) || emu.console.eq_ignore_ascii_case(&normalized))
                                && emu.supported_extensions.contains(&ext_str)
                            }).map(|emu| (emu.console.clone(), emu.id.clone()))
                        });

                    // For .zip/.7z files, match by folder name alone (most emulators read zipped ROMs)
                    let folder_archive = if folder_console.is_none() && (ext_str == "zip" || ext_str == "7z") {
                        e.path().strip_prefix(&dir).ok()
                            .and_then(|rel| rel.components().next())
                            .and_then(|c| {
                                let folder = c.as_os_str().to_string_lossy().to_string();
                                let normalized = normalize_console_folder(&folder);
                                catalog.iter().find(|emu| {
                                    emu.console.eq_ignore_ascii_case(&folder) || emu.console.eq_ignore_ascii_case(&normalized)
                                }).map(|emu| (emu.console.clone(), emu.id.clone()))
                            })
                    } else { None };

                    let matched = folder_console.or(folder_archive).or_else(|| match_extension(&ext_str, &catalog));

                    if let Some((console, emu_id)) = matched {
                        let name = e.path().file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();

                        // Filter out updates and DLCs
                        if is_update_or_dlc(&name, &ext_str) {
                            continue;
                        }

                        roms.push(RomFile {
                            name,
                            path: e.path().to_string_lossy().to_string(),
                            console,
                            emulator_id: emu_id,
                            extension: ext_str,
                        });
                    }
                }
            }
        }
    }
    push_log("INFO", &format!("Scan ROMs terminé: {} jeux trouvés dans {}", roms.len(), directory));
    roms
}

fn normalize_console_folder(folder: &str) -> String {
    match folder.to_lowercase().as_str() {
        "psp" | "playstation portable" => "PlayStation Portable".to_string(),
        "ps1" | "psx" | "playstation" | "playstation 1" => "PlayStation 1".to_string(),
        "ps2" | "playstation 2" => "PlayStation 2".to_string(),
        "ps3" | "playstation 3" => "PlayStation 3".to_string(),
        "n64" | "nintendo 64" => "Nintendo 64".to_string(),
        "nds" | "ds" | "nintendo ds" => "Nintendo DS".to_string(),
        "3ds" | "nintendo 3ds" => "Nintendo 3DS".to_string(),
        "gc" | "gamecube" | "ngc" => "GameCube / Wii".to_string(),
        "wii" => "GameCube / Wii".to_string(),
        "wiiu" | "wii u" => "Wii U".to_string(),
        "nes" | "famicom" => "NES".to_string(),
        "snes" | "super nintendo" | "super famicom" => "SNES".to_string(),
        "gba" | "game boy advance" => "Game Boy Advance".to_string(),
        "gb" | "game boy" => "Game Boy".to_string(),
        "gbc" | "game boy color" => "Game Boy Color".to_string(),
        "switch" | "nintendo switch" => "Nintendo Switch".to_string(),
        "md" | "megadrive" | "mega drive" | "genesis" => "Mega Drive".to_string(),
        "dc" | "dreamcast" => "Dreamcast".to_string(),
        "vb" | "virtual boy" => "Virtual Boy".to_string(),
        "sega cd" | "mega cd" | "segacd" | "megacd" => "Sega CD".to_string(),
        "32x" | "sega 32x" => "Sega 32X".to_string(),
        "xbox" => "Xbox".to_string(),
        "xbox 360" | "xbox360" | "x360" => "Xbox 360".to_string(),
        "atari 5200" | "atari5200" => "Atari 5200".to_string(),
        "atari 7800" | "atari7800" => "Atari 7800".to_string(),
        "jaguar" | "atari jaguar" => "Jaguar".to_string(),
        "lynx" | "atari lynx" => "Lynx".to_string(),
        "turbografx-16" | "turbografx16" | "tg16" | "pc engine" | "pce" => "TurboGrafx-16".to_string(),
        "turbografx-cd" | "tgcd" | "pc engine cd" | "pcecd" => "TurboGrafx-CD".to_string(),
        "cd-i" | "cdi" | "philips cd-i" => "CD-i".to_string(),
        "saturn" | "sega saturn" => "Saturn".to_string(),
        "game gear" | "gg" | "gamegear" => "Game Gear".to_string(),
        "sms" | "master system" | "sega master system" => "Master System".to_string(),
        _ => folder.to_string(),
    }
}

/// Detect if a ROM file is a game update or DLC (should be hidden from the library)
fn is_update_or_dlc(name: &str, _ext: &str) -> bool {
    let lower = name.to_lowercase();

    // "Incl. DLC" / "+ DLC" / "with DLC" patterns mean base game bundled with DLC — keep it
    let has_bundled_dlc = lower.contains("incl") && lower.contains("dlc")
        || lower.contains("+ dlc")
        || lower.contains("+dlc")
        || lower.contains("with dlc")
        || lower.contains("all dlc")
        || lower.contains("dlcs");

    // Switch Title ID detection: Extract hex IDs from brackets
    // Base game IDs MUST end in 000.
    // Updates end in 800, DLCs end in 001-7FF.
    if let Some(id) = extract_title_id(name) {
        if id.starts_with("010") && !id.ends_with("000") {
            return true;
        }
        // Wii U check: Base=00050000, Update=0005000E, DLC=0005000C
        if id.starts_with("0005000E") || id.starts_with("0005000C") {
            return true;
        }
    }

    // Keyword-based detection — but skip if bundled DLC pattern detected
    if !has_bundled_dlc {
        if lower.contains("dlc") || lower.contains("update") || lower.contains("patch") {
            return true;
        }
    }

    // "upd" alone (not part of "update" already caught, but standalone like "[UPD]")
    if !has_bundled_dlc && lower.contains("[upd]") {
        return true;
    }

    // Additional Switch specific: Check for version strings in brackets like [v65536]
    // Base games are usually [v0].
    if lower.contains("[v0]") {
        // Base game, don't filter
    } else if lower.contains("[v") {
        // likely an update like [v65536]
        return true;
    }

    false
}

#[allow(dead_code)]
/// Strip version tags like "(v1.01)" or "(Rev 1)" from a name
fn regex_strip_version(name: &str) -> String {
    let mut result = name.to_string();
    // Remove (vX.XX) patterns
    while let Some(start) = result.find("(v") {
        if let Some(end) = result[start..].find(')') {
            result = format!("{}{}", &result[..start], &result[start + end + 1..]);
        } else {
            break;
        }
    }
    // Remove (Rev X) patterns
    while let Some(start) = result.find("(Rev ") {
        if let Some(end) = result[start..].find(')') {
            result = format!("{}{}", &result[..start], &result[start + end + 1..]);
        } else {
            break;
        }
    }
    result.trim().to_string()
}

/// Strip language/scene tags but keep region tags
fn regex_strip_tags(name: &str) -> String {
    let mut result = name.to_string();
    let regions = ["Europe", "USA", "World", "Japan", "France", "Germany", "Italy", "Spain", "Netherlands", "Sweden", "Australia", "Brazil", "Korea", "China"];
    
    let mut changed = true;
    while changed {
        changed = false;
        if let Some(start) = result.rfind('(') {
            if let Some(end_offset) = result[start..].find(')') {
                let end = start + end_offset;
                let content = &result[start + 1..end];
                
                // If it's NOT a known region, strip it
                let is_region = regions.iter().any(|&r| content.contains(r));
                if !is_region {
                    result = format!("{}{}", &result[..start], &result[end + 1..]);
                    changed = true;
                    continue;
                }
            }
        }
        if let Some(start) = result.rfind('[') {
            if let Some(end_offset) = result[start..].find(']') {
                let end = start + end_offset;
                result = format!("{}{}", &result[..start], &result[end + 1..]);
                changed = true;
            }
        }
    }
    result.replace("  ", " ").trim().to_string()
}

#[allow(dead_code)]
/// Strip ALL parenthetical content to get the base game name
fn regex_strip_all_parens(name: &str) -> String {
    let mut result = name.to_string();
    loop {
        if let Some(start) = result.find('(') {
            if let Some(end) = result[start..].find(')') {
                result = format!("{}{}", &result[..start], &result[start + end + 1..]);
            } else {
                break;
            }
        } else {
            break;
        }
    }
    // Also strip brackets
    loop {
        if let Some(start) = result.find('[') {
            if let Some(end) = result[start..].find(']') {
                result = format!("{}{}", &result[..start], &result[start + end + 1..]);
            } else {
                break;
            }
        } else {
            break;
        }
    }
    result.trim().to_string()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RomFile {
    pub name: String,
    pub path: String,
    pub console: String,
    pub emulator_id: String,
    pub extension: String,
}

#[tauri::command]
fn delete_rom(path: String) -> Result<String, String> {
    push_log("INFO", &format!("Suppression ROM demandée: {}", path));
    let p = PathBuf::from(&path);
    if !p.exists() {
        push_log("WARN", &format!("Suppression impossible — fichier introuvable: {}", path));
        return Err("File not found".to_string());
    }

    let config = get_config();
    let roms_root = PathBuf::from(&config.roms_directory);
    if let (Ok(canonical), Ok(canonical_root)) = (p.canonicalize(), roms_root.canonicalize()) {
        if !canonical.starts_with(&canonical_root) {
            return Err("Accès refusé : fichier hors du dossier ROMs".to_string());
        }
    }

    // Delete the ROM file
    fs::remove_file(&p).map_err(|e| format!("Failed to delete ROM: {}", e))?;
    
    // Attempt to delete cached cover if any
    let config = get_config();
    let name = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let console_dir = p.parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    
    for ext in &["webp", "png"] {
        let cover_path = PathBuf::from(&config.covers_directory)
            .join(&console_dir)
            .join(format!("{}.{}", name, ext));
        if cover_path.exists() {
            let _ = fs::remove_file(&cover_path);
        }
    }
    
    Ok(format!("Deleted {}", name))
}

fn match_extension(ext: &str, catalog: &[emulators::EmulatorInfo]) -> Option<(String, String)> {
    // 1. Prefer emulators that ONLY support this extension (more specific)
    for emu in catalog {
        if emu.supported_extensions.len() == 1 && emu.supported_extensions[0] == ext {
            return Some((emu.console.clone(), emu.id.clone()));
        }
    }
    
    // 2. Prefer non-generic emulators (not Arcade/Multi-System) for common extensions
    for emu in catalog {
        if emu.id != "retroarch-arcade" && emu.id != "retroarch" {
            for supported_ext in &emu.supported_extensions {
                if supported_ext == ext {
                    return Some((emu.console.clone(), emu.id.clone()));
                }
            }
        }
    }

    // 3. Fallback to any match (including generic ones)
    for emu in catalog {
        for supported_ext in &emu.supported_extensions {
            if supported_ext == ext {
                return Some((emu.console.clone(), emu.id.clone()));
            }
        }
    }
    None
}

fn save_cover_as_webp(png_bytes: &[u8], dest_dir: &PathBuf, safe_name: &str) -> Option<String> {
    let _ = fs::create_dir_all(dest_dir);
    let img = ImageReader::new(Cursor::new(png_bytes))
        .with_guessed_format().ok()?
        .decode().ok()?;
    let encoder = webp::Encoder::from_image(&img).ok()?;
    let webp_data = encoder.encode(85.0);
    let webp_path = dest_dir.join(format!("{}.webp", safe_name));
    fs::write(&webp_path, &*webp_data).ok()?;
    Some(format!("data:image/webp;base64,{}", base64::engine::general_purpose::STANDARD.encode(&*webp_data)))
}

#[tauri::command]
async fn fetch_boxart(app_handle: tauri::AppHandle, game_name: String, console: String, force_refresh: Option<bool>) -> Result<String, String> {
    let force_refresh = force_refresh.unwrap_or(false);
    let config = get_config();
    let covers_dir = PathBuf::from(&config.covers_directory);

    push_log("INFO", &format!("Fetch cover: '{}' ({}) [force={}]", game_name, console, force_refresh));

    // Helper to log to frontend via events
    let log_event = {
        let app_handle = app_handle.clone();
        let game_name = game_name.clone();
        move |url: &str, status: &str, err: Option<String>| {
            use tauri::Emitter;
            let _ = app_handle.emit("boxart-log", serde_json::json!({
                "game": game_name,
                "url": url,
                "status": status,
                "error": err
            }));
        }
    };

    let libretro_systems = match console.as_ref() {
        "NES" | "Famicom" => vec!["Nintendo - Nintendo Entertainment System"],
        "SNES" | "Super Famicom" | "Super Nintendo" => vec!["Nintendo - Super Nintendo Entertainment System"],
        "Nintendo 64" | "N64" => vec!["Nintendo - Nintendo 64"],
        "Game Boy" => vec!["Nintendo - Game Boy", "Nintendo - Game Boy Color"],
        "Game Boy Color" | "GBC" => vec!["Nintendo - Game Boy Color", "Nintendo - Game Boy"],
        "Game Boy Advance" | "GBA" => vec!["Nintendo - Game Boy Advance", "Nintendo - Game Boy Color", "Nintendo - Game Boy"],
        "Nintendo DS" => vec!["Nintendo - Nintendo DS"],
        "Nintendo 3DS" | "3DS" => vec!["Nintendo - Nintendo 3DS"],
        "GameCube" => vec!["Nintendo - GameCube", "Nintendo - Wii"],
        "GameCube / Wii" | "GameCube - Wii" => vec!["Nintendo - Wii", "Nintendo - GameCube"],
        "Wii" => vec!["Nintendo - Wii", "Nintendo - GameCube"],
        "Wii U" => vec!["Nintendo - Wii U", "Nintendo - Wii"],
        "Nintendo Switch" => vec!["Nintendo - Nintendo Switch"],
        "Virtual Boy" => vec!["Nintendo - Virtual Boy"],
        "PlayStation 1" | "PS1" => vec!["Sony - PlayStation"],
        "PlayStation 2" | "PS2" => vec!["Sony - PlayStation 2"],
        "PlayStation Portable" | "PSP" => vec!["Sony - PlayStation Portable"],
        "Mega Drive" | "Genesis" => vec!["Sega - Mega Drive - Genesis"],
        "Dreamcast" => vec!["Sega - Dreamcast"],
        "Master System" => vec!["Sega - Master System - Mark III"],
        "Game Gear" => vec!["Sega - Game Gear"],
        "Saturn" => vec!["Sega - Saturn"],
        "Sega CD" => vec!["Sega - Mega-CD - Sega CD"],
        "Sega 32X" => vec!["Sega - 32X"],
        "Xbox" => vec!["Microsoft - Xbox"],
        "Xbox 360" => vec!["Microsoft - Xbox 360"],
        "PlayStation 3" | "PS3" => vec!["Sony - PlayStation 3"],
        "Atari 2600" => vec!["Atari - 2600"],
        "Atari 5200" => vec!["Atari - 5200"],
        "Atari 7800" => vec!["Atari - 7800"],
        "Jaguar" => vec!["Atari - Jaguar"],
        "Lynx" => vec!["Atari - Lynx"],
        "TurboGrafx-16" => vec!["NEC - PC Engine - TurboGrafx 16"],
        "TurboGrafx-CD" => vec!["NEC - PC Engine CD - TurboGrafx-CD"],
        "CD-i" => vec!["Philips - CDi"],
        _ => vec![],
    };

    let candidates = generate_search_candidates(&game_name, &console);
    let safe_name: String = game_name.chars()
        .map(|c| if ['*', '/', '<', '>', '?', '\\', '|', '"', ':'].contains(&c) { '_' } else { c })
        .collect();
    
    let norm_target = game_name.to_lowercase().chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>();

    // Console-specific size thresholds to block junk/icons but keep small retro covers
    let min_size: usize = match console.as_str() {
        "Nintendo Switch" | "Wii U" => 10000,
        "NES" | "SNES" | "Super Nintendo" | "Game Boy" | "Game Boy Color" | "Game Boy Advance" => 3000,
        "Wii" | "GameCube" | "GameCube / Wii" | "PlayStation 2" => 5000,
        _ => 5000,
    };

    // Sanitize console name for filesystem ("GameCube / Wii" -> "GameCube - Wii")
    let safe_console: String = console.replace("/", "-");
    let console_covers_dir = covers_dir.join(&safe_console);
    let synonym_groups = vec![
        vec!["rouge", "red"], vec!["bleu", "blue"], vec!["jaune", "yellow"],
        vec!["argent", "silver"], vec!["or", "gold"], vec!["cristal", "crystal"],
        vec!["rubis", "ruby"], vec!["saphir", "sapphire"], vec!["emeraude", "emerald"],
        vec!["platine", "platinum"], vec!["perle", "pearl"], vec!["diamant", "diamond"],
        vec!["noir", "black"], vec!["blanc", "white"],
        vec!["soleil", "sun"], vec!["lune", "moon"],
        vec!["epee", "sword"], vec!["bouclier", "shield"],
        vec!["violet", "violet"], vec!["ecarlate", "scarlet"],
        vec!["vert", "green"]
    ];

    let check_mismatch = |target: &str, candidate: &str| -> Option<String> {
        let t_low = target.to_lowercase();
        let c_low = candidate.to_lowercase();
        
        for group in &synonym_groups {
            // Check for entire words using word boundaries to avoid "Bros" matching "Or"
            let target_has_group = group.iter().any(|syn| {
                let re = regex::Regex::new(&format!(r"(?i)\b{}\b", syn)).ok();
                re.map(|r| r.is_match(&t_low)).unwrap_or(false)
            });
            let candidate_has_group = group.iter().any(|syn| {
                let re = regex::Regex::new(&format!(r"(?i)\b{}\b", syn)).ok();
                re.map(|r| r.is_match(&c_low)).unwrap_or(false)
            });
            
            if target_has_group != candidate_has_group {
                return Some(format!("Mismatch on group {:?} (Target: {}, Candidate: {})", group, target_has_group, candidate_has_group));
            }
        }
        None
    };

    write_to_boxart_log(&format!("=== FETCH START: {} ({}) {} ===", game_name, console, if force_refresh { "[force]" } else { "" }));

    // If the user hit Retry, remove any stale cache entry so we can't fall back to it.
    if force_refresh {
        let target_png = console_covers_dir.join(format!("{}.png", &safe_name));
        let target_webp = console_covers_dir.join(format!("{}.webp", &safe_name));
        let _ = std::fs::remove_file(&target_png);
        let _ = std::fs::remove_file(&target_webp);
    }

    // 1. First check local covers directory (skipped when force_refresh is set)
    if !force_refresh {
    if let Ok(entries) = std::fs::read_dir(&console_covers_dir) {
        let mut best_local = None;
        for entry in entries.flatten() {
            if let Some(file_name) = entry.file_name().to_str() {
                let lower = file_name.to_lowercase();
                if lower.ends_with(".png") || lower.ends_with(".jpg") || lower.ends_with(".jpeg") || lower.ends_with(".webp") {
                    let name_no_ext = lower.rsplit_once('.').map(|(n,_)| n).unwrap_or(&lower);
                    let norm_file = name_no_ext.chars().filter(|c| c.is_alphanumeric()).collect::<String>();
                    
                    if norm_file == norm_target {
                        best_local = Some(entry.path());
                        break;
                    }
                    
                    if best_local.is_none() && (norm_target.contains(&norm_file) || norm_file.contains(&norm_target)) {
                        if let Some(reason) = check_mismatch(&game_name, &lower) {
                            write_to_boxart_log(&format!("Local Reject: {} - Reason: {}", lower, reason));
                            continue;
                        }
                        if norm_file.len() > 3 {
                            best_local = Some(entry.path());
                        }
                    }
                }
            }
        }
        
        if let Some(path) = best_local {
            if let Ok(data) = std::fs::read(&path) {
                if data.len() >= min_size {
                    log_event("Local Cache", "Match Found", None);
                    write_to_boxart_log("Result: Local Cache Success");
                    // Don't store URLs from cache — only real fetches (sections 2-4) store correct URLs
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                    let mime = if path.extension().and_then(|e| e.to_str()) == Some("webp") { "image/webp" } else { "image/png" };
                    return Ok(format!("data:{};base64,{}", mime, b64));
                }
            }
        }
    }
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    // min_size already computed above

    // === SWITCH/WIIU: Try tinfoil.media first (best source for Switch covers) ===
    if console == "Nintendo Switch" {
        if let Some(id) = extract_title_id(&game_name).or_else(|| resolve_title_id(&game_name)) {
            let url = format!("https://tinfoil.media/ti/{}/512/512", id);
            write_to_boxart_log(&format!("Trying Tinfoil.media: {}", url));
            if let Ok(resp) = client.get(&url).send().await {
                if resp.status().is_success() {
                    if let Ok(bytes) = resp.bytes().await {
                        if bytes.len() >= min_size {
                            write_to_boxart_log(&format!("Result: Tinfoil.media Success (ID: {})", id));
                            store_cover_url(&format!("{}::{}", console, game_name), &format!("https://tinfoil.media/ti/{}/800/800", id));
                            if let Some(data_url) = save_cover_as_webp(&bytes, &console_covers_dir, &safe_name) {
                                return Ok(data_url);
                            }
                            return Ok(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(&bytes)));
                        }
                    }
                }
            }
        }
    }

    // === LIBRETRO: Try raw filename first (exact match with region tags) ===
    let raw_libretro_name = game_name.chars().map(|c| if "&*/:<>?\\|".contains(c) { '_' } else { c }).collect::<String>();
    for folder in &libretro_systems {
        let url = format!("https://thumbnails.libretro.com/{}/Named_Boxarts/{}.png", urlencoding::encode(folder), urlencoding::encode(&raw_libretro_name));
        write_to_boxart_log(&format!("Trying Libretro (raw): {}", url));
        if let Ok(resp) = client.get(&url).send().await {
            if resp.status().is_success() {
                if let Ok(bytes) = resp.bytes().await {
                    if bytes.len() >= min_size {
                        write_to_boxart_log(&format!("Result: Libretro Raw Success ({})", raw_libretro_name));
                        store_cover_url(&format!("{}::{}", console, game_name), &url);
                        let _ = std::fs::create_dir_all(&console_covers_dir);
                        if let Some(data_url) = save_cover_as_webp(&bytes, &console_covers_dir, &safe_name) {
                            return Ok(data_url);
                        }
                        return Ok(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(&bytes)));
                    }
                }
            }
        }
    }

    // === LIBRETRO: Try stripped name (remove lang codes + version, keep region) ===
    {
        let stripped = regex_strip_version(&regex_strip_tags(&raw_libretro_name));
        if stripped != raw_libretro_name {
            for folder in &libretro_systems {
                let url = format!("https://thumbnails.libretro.com/{}/Named_Boxarts/{}.png", urlencoding::encode(folder), urlencoding::encode(&stripped));
                write_to_boxart_log(&format!("Trying Libretro (stripped): {}", url));
                if let Ok(resp) = client.get(&url).send().await {
                    if resp.status().is_success() {
                        if let Ok(bytes) = resp.bytes().await {
                            if bytes.len() >= min_size {
                                write_to_boxart_log(&format!("Result: Libretro Stripped Success ({})", stripped));
                                if let Some(data_url) = save_cover_as_webp(&bytes, &console_covers_dir, &safe_name) {
                                    return Ok(data_url);
                                }
                                return Ok(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(&bytes)));
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. Try Title Searching FIRST (Libretro / Wikipedia) as requested
    for search_name in &candidates {
        // --- 3.1: Libretro (with region suffix variants) ---
        let libretro_variants = libretro_name_variants(search_name);
        for variant in &libretro_variants {
            for folder in &libretro_systems {
                let url = format!("https://thumbnails.libretro.com/{}/Named_Boxarts/{}.png", urlencoding::encode(folder), urlencoding::encode(variant));
                if let Ok(resp) = client.get(&url).send().await {
                    if resp.status().is_success() {
                        if let Ok(bytes) = resp.bytes().await {
                            if bytes.len() >= min_size {
                                write_to_boxart_log(&format!("Result: Libretro Success ({})", variant));
                                store_cover_url(&format!("{}::{}", console, game_name), &url);
                                if let Some(data_url) = save_cover_as_webp(&bytes, &console_covers_dir, &safe_name) {
                                    return Ok(data_url);
                                }
                                return Ok(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(&bytes)));
                            }
                        }
                    }
                }
            }
        }

        // --- 3.2: Wikipedia ---
        for suffix in &[" video game", " (video game)", ""] {
            let wiki_query = format!("{}{}", search_name, suffix);
            let wiki_url = format!("https://en.wikipedia.org/w/api.php?action=query&list=search&srsearch={}&srlimit=1&format=json", urlencoding::encode(&wiki_query));
            if let Ok(resp) = client.get(&wiki_url).send().await {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if let Some(search_res) = json["query"]["search"].as_array().and_then(|a| a.get(0)) {
                        let title = search_res["title"].as_str().unwrap_or_default();
                        // Skip Wikipedia series/franchise pages
                        let title_low_w = title.to_lowercase();
                        if title_low_w.contains("series") || title_low_w.contains("franchise") || title_low_w.contains("list of") {
                            write_to_boxart_log(&format!("Wikipedia Skip Series Page: {}", title));
                            continue;
                        }
                        // Get the image
                        let img_query_url = format!("https://en.wikipedia.org/w/api.php?action=query&titles={}&prop=pageimages&format=json&pithumbsize=1000", urlencoding::encode(title));
                        if let Ok(img_resp) = client.get(&img_query_url).send().await {
                            if let Ok(img_json) = img_resp.json::<serde_json::Value>().await {
                                if let Some(pages) = img_json["query"]["pages"].as_object() {
                                    for (_, page) in pages {
                                        if let Some(thumbnail) = page["thumbnail"]["source"].as_str() {
                                            if let Ok(bytes_resp) = client.get(thumbnail).send().await {
                                                if let Ok(bytes) = bytes_resp.bytes().await {
                                                    if bytes.len() >= min_size {
                                                        write_to_boxart_log(&format!("Result: Wikipedia Success ({})", title));
                                                        if let Some(data_url) = save_cover_as_webp(&bytes, &console_covers_dir, &safe_name) {
                                                            return Ok(data_url);
                                                        }
                                                        return Ok(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(&bytes)));
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 3.5 Wikipedia fallback: search article images for "cover"/"box" keywords
    if libretro_systems.is_empty() || console == "Xbox 360" || console == "Xbox" || console == "PlayStation 3" {
        let wiki_search_name = candidates.first().cloned().unwrap_or_else(|| game_name.clone());
        for suffix in &[" (video game)", ""] {
            let wiki_title = format!("{}{}", wiki_search_name, suffix);
            let search_url = format!("https://en.wikipedia.org/w/api.php?action=query&list=search&srsearch={}&srlimit=1&format=json", urlencoding::encode(&wiki_title));
            if let Ok(resp) = client.get(&search_url).send().await {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if let Some(title) = json["query"]["search"].as_array().and_then(|a| a.get(0)).and_then(|r| r["title"].as_str()) {
                        let images_url = format!("https://en.wikipedia.org/w/api.php?action=query&titles={}&prop=images&format=json", urlencoding::encode(title));
                        if let Ok(img_resp) = client.get(&images_url).send().await {
                            if let Ok(img_json) = img_resp.json::<serde_json::Value>().await {
                                if let Some(pages) = img_json["query"]["pages"].as_object() {
                                    for (_, page) in pages {
                                        if let Some(images) = page["images"].as_array() {
                                            for img in images {
                                                let img_title = img["title"].as_str().unwrap_or_default().to_lowercase();
                                                if img_title.contains("cover") || img_title.contains("box") {
                                                    let file_title = img["title"].as_str().unwrap_or_default();
                                                    let info_url = format!("https://en.wikipedia.org/w/api.php?action=query&titles={}&prop=imageinfo&iiprop=url&format=json", urlencoding::encode(file_title));
                                                    if let Ok(info_resp) = client.get(&info_url).send().await {
                                                        if let Ok(info_json) = info_resp.json::<serde_json::Value>().await {
                                                            if let Some(info_pages) = info_json["query"]["pages"].as_object() {
                                                                for (_, info_page) in info_pages {
                                                                    if let Some(url) = info_page["imageinfo"].as_array().and_then(|a| a.get(0)).and_then(|i| i["url"].as_str()) {
                                                                        if let Ok(img_dl) = client.get(url).send().await {
                                                                            if let Ok(bytes) = img_dl.bytes().await {
                                                                                if bytes.len() >= min_size {
                                                                                    write_to_boxart_log(&format!("Result: Wikipedia Cover Image Success ({})", file_title));
                                                                                    if let Some(data_url) = save_cover_as_webp(&bytes, &console_covers_dir, &safe_name) {
                                                                                        return Ok(data_url);
                                                                                    }
                                                                                    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                                                                                    return Ok(format!("data:image/png;base64,{}", b64));
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        break;
                    }
                }
            }
        }
    }

    // 4. Try Title ID Fallback (GameTDB for Wii / Wii U / GameCube; Switch uses Tinfoil above)
    if let Some(id) = extract_title_id(&game_name).or_else(|| resolve_title_id(&game_name)) {
        // Pick the right GameTDB path + file extension per console:
        //   wii       => cover.png            (front face; coverfull is wrap-around, looks off)
        //   wiiu      => coverHQ.jpg          (front face HQ; coverfull is the full jacket)
        //   gamecube  => cover.png
        //   switch    => skipped (Tinfoil already tried above; GameTDB has no Switch)
        let (console_type, path, ext, mime) = if id.starts_with("0100") {
            ("", "", "", "")
        } else if id.len() == 6 || id.len() == 4 {
            // 4-char Wii U disc IDs (WDKE, WDKP) and 6-char Wii/WiiU/GameCube IDs (RSPP01, AMKP01, GLME01).
            match console.as_ref() {
                "GameCube" | "GameCube / Wii" | "GameCube - Wii" if id.starts_with('G') =>
                    ("gamecube", "cover", "png", "image/png"),
                _ if id.starts_with('A') || id.starts_with('B') || id.starts_with('W') =>
                    ("wiiu", "coverHQ", "jpg", "image/jpeg"),
                _ => ("wii", "cover", "png", "image/png"),
            }
        } else if id.starts_with("0005") {
            ("wiiu", "coverHQ", "jpg", "image/jpeg")
        } else {
            ("", "", "", "")
        };

        if !console_type.is_empty() {
            // Region char position: 3 for 6-char disc IDs (RSPP01 → P), 3 for 4-char (WDKE → E).
            let primary_region = match id.chars().nth(3) {
                Some('E') => "US",
                Some('P') => "EN",
                Some('J') => "JA",
                Some('K') => "KO",
                _ => "EN",
            };

            let mut regions = vec![primary_region];
            for r in &["EN", "US", "FR", "DE", "JA", "ES", "IT"] {
                if !regions.contains(r) { regions.push(r); }
            }

            for region in regions {
                let url = format!("https://art.gametdb.com/{}/{}/{}/{}.{}", console_type, path, region, id, ext);
                write_to_boxart_log(&format!("Trying GameTDB: {}", url));
                if let Ok(resp) = client.get(&url).send().await {
                    if resp.status().is_success() {
                        if let Ok(bytes) = resp.bytes().await {
                            if bytes.len() >= min_size {
                                write_to_boxart_log(&format!("Result: GameTDB Success ({}/{})", region, id));
                                store_cover_url(&format!("{}::{}", console, game_name), &url);
                                if let Some(data_url) = save_cover_as_webp(&bytes, &console_covers_dir, &safe_name) {
                                    return Ok(data_url);
                                }
                                return Ok(format!("data:{};base64,{}", mime, base64::engine::general_purpose::STANDARD.encode(&bytes)));
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. Libretro Fallback
    for system in &libretro_systems {
        for candidate in &candidates {
            let libretro_name = candidate.chars().map(|c| if "&*/:<>?\\|".contains(c) { '_' } else { c }).collect::<String>();
            if let Some(reason) = check_mismatch(&game_name, &libretro_name) {
                write_to_boxart_log(&format!("Libretro Reject: {} - Reason: {}", libretro_name, reason));
                continue;
            }

            let url = format!("https://thumbnails.libretro.com/{}/Named_Boxarts/{}.png", urlencoding::encode(system), urlencoding::encode(&libretro_name));
            write_to_boxart_log(&format!("Trying Libretro: {}", url));
            if let Ok(resp) = client.get(&url).send().await {
                if resp.status().is_success() {
                    if let Ok(bytes) = resp.bytes().await {
                        if bytes.len() >= min_size {
                            log_event(&url, "SUCCESS", None);
                            write_to_boxart_log("Result: Libretro Success");
                            if let Some(data_url) = save_cover_as_webp(&bytes, &console_covers_dir, &safe_name) {
                                return Ok(data_url);
                            }
                            return Ok(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(&bytes)));
                        } else {
                            write_to_boxart_log(&format!("Libretro Ignored (Too small: {} bytes)", bytes.len()));
                        }
                    }
                }
            }
        }
    }

    // 4. Archive.org Fallback
    let search_name = if !candidates.is_empty() { candidates[0].clone() } else { game_name.clone() };
    let arch_query = format!("title:(\"{}\") AND mediatype:image", search_name);
    let arch_url = format!("https://archive.org/advancedsearch.php?q={}&fl[]=identifier&rows=5&output=json", urlencoding::encode(&arch_query));
    write_to_boxart_log(&format!("Trying Archive.org Search: {}", arch_url));
    if let Ok(resp) = client.get(&arch_url).send().await {
        if let Ok(json) = resp.json::<serde_json::Value>().await {
            if let Some(docs) = json["response"]["docs"].as_array() {
                for doc in docs {
                    if let Some(ia_id) = doc["identifier"].as_str() {
                        let meta_url = format!("https://archive.org/metadata/{}", ia_id);
                        if let Ok(meta_resp) = client.get(&meta_url).send().await {
                            if let Ok(meta_json) = meta_resp.json::<serde_json::Value>().await {
                                if let Some(files) = meta_json["files"].as_array() {
                                    for file in files {
                                        if let Some(fname) = file["name"].as_str() {
                                            let low = fname.to_lowercase();
                                            if (low.ends_with(".png") || low.ends_with(".jpg")) && (low.contains("front") || low.contains("cover") || low.contains("box")) {
                                                if let Some(reason) = check_mismatch(&game_name, &low) {
                                                    write_to_boxart_log(&format!("Archive File Reject: {} - Reason: {}", low, reason));
                                                    continue;
                                                }
                                                if let Some(reason) = check_mismatch(&game_name, &ia_id.to_lowercase()) {
                                                    write_to_boxart_log(&format!("Archive ID Reject: {} - Reason: {}", ia_id, reason));
                                                    continue;
                                                }

                                                let img_url = format!("https://archive.org/download/{}/{}", ia_id, fname);
                                                if let Ok(img_resp) = client.get(&img_url).send().await {
                                                    if let Ok(bytes) = img_resp.bytes().await {
                                                        if bytes.len() >= min_size {
                                                            log_event(&img_url, "SUCCESS", None);
                                                            write_to_boxart_log("Result: Archive.org Success");
                                                            if let Some(data_url) = save_cover_as_webp(&bytes, &console_covers_dir, &safe_name) {
                                                                return Ok(data_url);
                                                            }
                                                            return Ok(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(&bytes)));
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 5. Wikipedia Fallback
    for suffix in &[" video game", " (video game)", ""] {
        let wiki_query = format!("{}{}", search_name, suffix);
        let wiki_url = format!("https://en.wikipedia.org/w/api.php?action=query&list=search&srsearch={}&srlimit=1&format=json", urlencoding::encode(&wiki_query));
        write_to_boxart_log(&format!("Trying Wikipedia Search: {}", wiki_url));
        if let Ok(resp) = client.get(&wiki_url).send().await {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(search_res) = json["query"]["search"].as_array().and_then(|a| a.get(0)) {
                    let title = search_res["title"].as_str().unwrap_or_default();
                    // Skip Wikipedia series/franchise pages
                    let title_low_w2 = title.to_lowercase();
                    if title_low_w2.contains("series") || title_low_w2.contains("franchise") || title_low_w2.contains("list of") {
                        write_to_boxart_log(&format!("Wikipedia Skip Series Page: {}", title));
                        continue;
                    }
                    if let Some(reason) = check_mismatch(&game_name, title) {
                        write_to_boxart_log(&format!("Wikipedia Title Reject: {} - Reason: {}", title, reason));
                        continue;
                    }

                    let page_url = format!("https://en.wikipedia.org/w/api.php?action=query&titles={}&prop=pageimages&format=json&pithumbsize=1000", urlencoding::encode(title));
                    if let Ok(page_resp) = client.get(&page_url).send().await {
                        if let Ok(page_json) = page_resp.json::<serde_json::Value>().await {
                            if let Some(pages) = page_json["query"]["pages"].as_object() {
                                for (_, page) in pages {
                                    if let Some(src) = page["thumbnail"]["source"].as_str() {
                                        let low_src = src.to_lowercase();
                                        if low_src.contains("logo") && !low_src.contains("box") {
                                            write_to_boxart_log(&format!("Wikipedia Skip Logo: {}", src));
                                            continue;
                                        }
                                        if let Ok(img_resp) = client.get(src).send().await {
                                            if let Ok(bytes) = img_resp.bytes().await {
                                                if bytes.len() >= min_size {
                                                    log_event(src, "SUCCESS (Wiki)", None);
                                                    write_to_boxart_log("Result: Wikipedia Success");
                                                    if let Some(data_url) = save_cover_as_webp(&bytes, &console_covers_dir, &safe_name) {
                                                        return Ok(data_url);
                                                    }
                                                    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                                                    return Ok(format!("data:image/png;base64,{}", b64));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    write_to_boxart_log("Result: FAILED - All sources exhausted");
    log_event("DONE", "NONE FOUND", None);
    push_log("WARN", &format!("Cover introuvable pour '{}' ({})", game_name, console));
    Err("No boxart found".to_string())
}

/// Clean a game name for searching (handles scene tags, IDs, etc.)
fn clean_game_name(name: &str) -> String {
    let mut cleaned = name.to_string();
    
    // Remove extensions first
    let extensions = vec![".xiso.iso", ".iso", ".chd", ".rvz", ".wbfs", ".nca", ".nsp", ".xci", ".zip", ".7z", ".gz", ".wud", ".wux", ".rpx", ".nes", ".sfc", ".smc", ".gba", ".gbc", ".gb", ".nds", ".n64", ".z64", ".3ds", ".cci", ".cxi", ".cia", ".3dsx", ".app", ".xiso", ".xbe", ".xex"];
    for ext in extensions {
        if cleaned.to_lowercase().ends_with(ext) {
            cleaned = cleaned[..cleaned.len()-ext.len()].to_string();
            break;
        }
    }

    // Use simple replacement to avoid unsupported look-around regex panics
    cleaned = cleaned.replace('.', " ").replace('_', " ");

    // Preserve alphanumeric + spaces, strip brackets/parens content (tags)
    let re_tags = Regex::new(r"\[.*?\]|\(.*?\)").unwrap();
    cleaned = re_tags.replace_all(&cleaned, "").to_string();

    // Strip catalog number prefix (e.g. "3DS0762 - ", "NDS1234 - ", "0123 - ")
    let re_catalog = Regex::new(r"^[A-Za-z0-9]{2,6}\d{2,5}\s*-\s*").unwrap();
    cleaned = re_catalog.replace(&cleaned, "").to_string();

    // Strip scene group after " - " at the end (e.g. "Pokemon Sword - kaze-nico")
    if let Some(pos) = cleaned.rfind(" - ") {
        let after = &cleaned[pos + 3..];
        if after.split_whitespace().count() <= 3 {
            cleaned = cleaned[..pos].to_string();
        }
    }

    // Strip version patterns like V1.3.2, v1 3 2, V2 0 etc.
    let re_version = Regex::new(r"(?i)\bV\d[\d .]*\d\b|\bV\d+\b").unwrap();
    cleaned = re_version.replace_all(&cleaned, "").to_string();

    // Strip "Incl. N DLCs" / "Incl DLC" / "+ 2 DLC" patterns as a whole before keyword strip
    let re_incl_dlc = Regex::new(r"(?i)\b(incl\.?\s*\d*\s*dlcs?|with\s+\d*\s*dlcs?|\+\s*\d*\s*dlcs?|all\s+dlcs?)\b").unwrap();
    cleaned = re_incl_dlc.replace_all(&cleaned, "").to_string();

    // Strip common scene keywords and region/format tags
    let scene_keywords = vec![
        "PROPER", "REPACK", "NSW", "MULTi", "READNFO", "INTERNAL", "D0WNLOAD",
        "BigBlueBox", "Kaze-Nico", "kaze-nico", "Update", "DLC", "DLCs", "Patch",
        "Collection", "nsw2u", "NKA", "NC", "NT",
        "decrypted", "encrypted", "trimmed", "undub",
        "Eur", "USA", "EUR", "JPN", "World",
        "SuperXCi", "SuperNSP", "XCI", "NSP", "NSZ",
    ];
    for kw in scene_keywords {
        let re = Regex::new(&format!(r"(?i)\b{}\b", regex::escape(kw))).unwrap();
        cleaned = re.replace_all(&cleaned, "").to_string();
    }


    // Final cleanup
    cleaned = cleaned
        .replace('é', "e").replace('è', "e").replace('ê', "e").replace('ë', "e")
        .replace('à', "a").replace('â', "a").replace('ô', "o")
        .replace('û', "u").replace('ï', "i").replace('î', "i").replace('ç', "c");
    
    while cleaned.contains("  ") {
        cleaned = cleaned.replace("  ", " ");
    }
    
    cleaned.trim().to_string()
}

/// Extract a Title ID from a string (e.g. "[01007EF...")
/// Resilient against truncation, spaces, and formatting junk.
fn extract_title_id(name: &str) -> Option<String> {
    // 1. "Dirty" Bracket extraction
    if let Some(start) = name.find('[') {
        let content = if let Some(end) = name[start..].find(']') {
            &name[start + 1..start + end]
        } else {
            &name[start + 1..] // Handle missing closing bracket
        };

        // Strip ALL non-hex characters
        let mut id = content.chars()
            .filter(|c| c.is_ascii_hexdigit())
            .collect::<String>()
            .to_uppercase();

        if id.len() >= 8 {
            // Pad to 16 if it looks like Switch (0100...) or WiiU (0005...)
            if id.starts_with("0100") || id.starts_with("0005") {
                while id.len() < 16 { id.push('0'); }
                if id.len() > 16 { id.truncate(16); }
            }
            return Some(id);
        }
    }

    // 2. Standalone hex search (Fallback)
    let re_id = regex::Regex::new(r"(?i)\b(0100[0-9A-F]{12})\b|\b(00050000[0-9A-F]{8})\b").ok()?;
    if let Some(caps) = re_id.captures(name) {
        if let Some(m) = caps.get(1) { return Some(m.as_str().to_uppercase()); }
        if let Some(m) = caps.get(2) { return Some(m.as_str().to_uppercase()); }
    }

    None
}

/// Resolve a title ID from game name using a hardcoded lookup table
/// for popular games that often lack IDs in their filenames.
fn resolve_title_id(name: &str) -> Option<String> {
    let cleaned = clean_game_name(name).to_lowercase();
    
    // Known Switch title IDs for games commonly missing IDs in filenames
    let known_ids: Vec<(&[&str], &str)> = vec![
        // Zelda
        (&["zelda", "echoes of wisdom"], "01008CF01BAAC000"),
        (&["zelda", "tears of the kingdom"], "0100F2C0115B6000"),
        (&["zelda", "breath of the wild"], "01007EF00011E000"),
        (&["zelda", "links awakening"], "01006BB00C6F0000"),
        // Mario & Luigi
        (&["mario", "luigi", "brothership"], "01006D0017F7A000"),
        // 1-2-Switch
        (&["1-2-switch"], "01000320000CC000"),
        (&["1 2 switch"], "01000320000CC000"),
        (&["12switch"], "01000320000CC000"),
        // Tomodachi Life
        (&["tomodachi", "life"], "010051F0207B2000"),
        // Other popular titles often without IDs
        (&["mario", "odyssey"], "0100000000010000"),
        (&["mario kart 8"], "0100152000022000"),
        (&["splatoon 3"], "0100C2500FC20000"),
        (&["animal crossing", "new horizons"], "01006F8002326000"),
        (&["pokemon", "sword"], "0100ABF008968000"),
        (&["pokemon", "shield"], "01008DB008C2C000"),
        (&["pokemon", "scarlet"], "0100A3D008C5C000"),
        (&["pokemon", "violet"], "01008F6008C5E000"),
        (&["pokemon", "brilliant diamond"], "0100000011D90000"),
        (&["pokemon", "shining pearl"], "010018E011D92000"),
        (&["pokemon", "lets go pikachu"], "010003F003A34000"),
        (&["pokemon", "lets go eevee"], "0100187003A36000"),
        (&["pokemon legends", "arceus"], "01001F5010DFA000"),
        (&["pokemon legends", "za"], "0100F43008C44000"),
        (&["pokemon mystery dungeon"], "01003D200BAA2000"),
        (&["pokemon snap"], "0100F4300BF2C000"),
        (&["super smash bros"], "01006A800016E000"),
        (&["nintendo switch sports"], "0100D2F00D5C0000"),
        (&["mario party jamboree"], "0100965017338000"),
        (&["princess peach", "showtime"], "01007A3009184000"),
        (&["sonic", "shadow generations"], "01005EA01C0FC000"),
        (&["super mario 3d world"], "010028600EBDA000"),
        (&["super mario bros", "wonder"], "010015100B514000"),
        (&["pikmin 4"], "0100B7C00933A000"),
        (&["mario strikers"], "010019401051C000"),
        (&["mario tennis aces"], "0100BDE00862A000"),
        (&["luigi mansion 2"], "010048701995E000"),
        (&["luigi mansion 3"], "0100DCA0064A6000"),
        (&["donkey kong country returns"], "01009D901BC56000"),
        (&["donkey kong country tropical"], "0100C1F0051B6000"),
        (&["lego horizon"], "010073C01AF34000"),
        (&["watermelon game"], "0100800015926000"),
        (&["suika game"], "0100800015926000"),
        (&["celeste"], "01002B30028F6000"),
        (&["forager"], "01001D200BCC4000"),
        (&["dragon quest builders 2"], "010042000A986000"),
        (&["dragon quest builders"], "010008900705C000"),
        (&["tomb raider", "remastered"], "010024601BB16000"),
        (&["super mario rpg"], "0100BC0018138000"),
        (&["mario vs donkey kong"], "0100B99019412000"),
        (&["pixark"], "0100CC700B2B4000"),
        // Switch — extra entries for common filenames
        (&["1.2.switch"], "01000320000CC000"),
        (&["tomodachi life living"], "010051F0207B2000"),
        // Wii U HD remasters — GameTDB disc IDs (4-char or 6-char, not 16-hex Title IDs)
        (&["zelda", "wind waker"], "WDKP"),           // Wind Waker HD (EUR; US = WDKE, JP = WDKJ)
        (&["zelda", "twilight princess"], "BCZP01"),  // Twilight Princess HD (EUR)
        (&["mario kart 8"], "AMKP01"),
        (&["super mario 3d world"], "ARDP01"),
        (&["new super mario bros u"], "ARPP01"),
        (&["super mario maker"], "AMAP01"),           // Wii U Super Mario Maker (was incorrectly mapped to Wind Waker)
        // Wii (Redump/nkit family often lacks bracketed IDs)
        (&["wii sports resort"], "RZTP01"),           // Wii Sports Resort (EUR)
        (&["wii sports"], "RSPP01"),                  // Wii Sports (EUR)
        (&["wii party"], "SUPP01"),
        (&["mario kart wii"], "RMCP01"),
        (&["new super mario bros wii"], "SMNP01"),
        (&["luigi mansion", "gamecube"], "GLME01"),
    ];
    
    for (keywords, id) in &known_ids {
        if keywords.iter().all(|kw| cleaned.contains(kw)) {
            write_to_boxart_log(&format!("Resolved title ID: {} -> {}", name, id));
            return Some(id.to_string());
        }
    }
    
    None
}

fn generate_search_candidates(name: &str, console: &str) -> Vec<String> {
    let cleaned = clean_game_name(name);
    if cleaned.is_empty() { return vec![]; }
    
    let mut candidates = vec![cleaned.clone()];

    // Aggressive cleaning for pure title
    let mut pure = cleaned.clone();
    if let Some(pos) = pure.find('(') { pure = pure[..pos].trim().to_string(); }
    if let Some(pos) = pure.find('[') { pure = pure[..pos].trim().to_string(); }
    if let Some(pos) = pure.find(" - ") { pure = pure[..pos].trim().to_string(); }
    
    if !pure.is_empty() && pure != cleaned {
        candidates.push(pure.clone());
    }

    // Language bridge (French to English mapping for common titles like Pokemon)
    let fr_to_en = vec![
        ("Platine", "Platinum"), ("Rouge", "Red"), ("Bleu", "Blue"), ("Jaune", "Yellow"),
        ("Or", "Gold"), ("Argent", "Silver"), ("Cristal", "Crystal"), ("Rubis", "Ruby"),
        ("Saphir", "Sapphire"), ("Emeraude", "Emerald"), ("Diamant", "Diamond"),
        ("Perle", "Pearl"), ("Noir", "Black"), ("Blanc", "White"), ("Soleil", "Sun"), ("Lune", "Moon"),
        ("Rouge Feu", "FireRed"), ("Vert Feuille", "LeafGreen")
    ];

    // Apply translation to both pure and full cleaned name
    let mut translated = pure.clone();
    let mut translated_full = cleaned.clone();
    let mut matched = false;
    for (fr, en) in &fr_to_en {
        if translated.contains(fr) {
            translated = translated.replace(fr, en);
            matched = true;
        }
        if translated_full.contains(fr) {
            translated_full = translated_full.replace(fr, en);
        }
    }

    // Fix "Pokemon - Version X" → "Pokemon - X Version" (libretro format)
    let version_re = regex::Regex::new(r"(?i)Version (\w+)").unwrap();
    if let Some(caps) = version_re.captures(&translated_full) {
        let color = caps.get(1).unwrap().as_str().to_string();
        let libretro_name = translated_full.replace(&format!("Version {}", color), &format!("{} Version", color));
        candidates.push(libretro_name.clone());
    }

    if matched {
        candidates.push(translated.clone());
        candidates.push(format!("{} (World)", translated));
        candidates.push(format!("{} (USA)", translated));
    }
    if translated_full != cleaned {
        candidates.push(translated_full.clone());
    }

    // Add console name to force the right Wikipedia result
    if !pure.is_empty() {
        candidates.push(format!("{} {}", pure, console));
        if matched {
            candidates.push(format!("{} {}", translated, console));
        }
    }

    // Libretro conversions
    let alt1 = pure.replace(':', " -").replace(" & ", " + ");
    if alt1 != pure { candidates.push(alt1); }

    // Re-add colon if it was missing but might be needed for Wikipedia/Libretro
    if !pure.contains(':') && (pure.contains("Mario") || pure.contains("Zelda") || pure.contains("Metroid")) {
        // Simple heuristic: add colon after the first word or known series names
        let series = ["Mario & Luigi", "The Legend of Zelda", "Super Mario", "Metroid"];
        for s in series {
            if pure.starts_with(s) && pure.len() > s.len() + 1 {
                candidates.push(format!("{}: {}", s, &pure[s.len()..].trim()));
                break;
            }
        }
    }

    // Specific fix for 1.2.Switch
    if pure.contains("1.2.Switch") || pure.contains("1 2 Switch") {
        candidates.push("1-2-Switch".to_string());
        candidates.push("1-2-Switch (video game)".to_string());
    }

    // Exact libretro filenames for games that are hard to match automatically
    let lower_pure = pure.to_lowercase();
    let exact_libretro: Vec<&str> = if lower_pure.contains("zelda") && lower_pure.contains("twilight") {
        vec!["Legend of Zelda, The - Twilight Princess HD (USA) (En,Fr,Es) (Rev 2)"]
    } else if lower_pure.contains("zelda") && lower_pure.contains("wind waker") {
        vec!["Legend of Zelda, The - The Wind Waker HD (USA, Asia) (En,Fr,Es)"]
    } else if lower_pure.contains("zelda") && lower_pure.contains("breath") {
        vec!["Legend of Zelda, The - Breath of the Wild (USA) (En,Fr,Es)"]
    } else if lower_pure.contains("mario kart 8") {
        vec!["Mario Kart 8 (USA) (En,Fr,Es)"]
    } else if lower_pure.contains("splatoon") && !lower_pure.contains("2") {
        vec!["Splatoon (USA) (En,Fr,Es)"]
    } else if lower_pure.contains("super mario 3d world") {
        vec!["Super Mario 3D World (USA) (En,Fr,Es)"]
    } else if lower_pure.contains("smash") && lower_pure.contains("wii u") {
        vec!["Super Smash Bros. for Wii U (USA) (En,Fr,Es)"]
    } else if lower_pure.contains("bayonetta 2") {
        vec!["Bayonetta 2 (USA) (En,Fr,Es)"]
    } else if lower_pure.contains("donkey kong") && lower_pure.contains("tropical") {
        vec!["Donkey Kong Country - Tropical Freeze (USA) (En,Fr,Es)"]
    } else if lower_pure.contains("pikmin 3") {
        vec!["Pikmin 3 (USA) (En,Fr,Es)"]
    } else if lower_pure.contains("xenoblade") && lower_pure.contains("x") {
        vec!["Xenoblade Chronicles X (USA) (En,Fr,Es)"]
    } else if lower_pure.contains("new super mario bros") && console == "Wii U" {
        vec!["New Super Mario Bros. U (USA) (En,Fr,Es)"]
    } else if lower_pure.contains("ducktales") {
        vec!["DuckTales - Remastered (USA)"]
    } else {
        vec![]
    };
    for name in exact_libretro {
        candidates.insert(0, name.to_string());
    }

    // Tomodachi Life (handles truncated "Living th..." filenames)
    if pure.to_lowercase().contains("tomodachi") {
        candidates.push("Tomodachi Life Living the Dream".to_string());
        candidates.push("Tomodachi Life: Living the Dream".to_string());
        candidates.push("Tomodachi Life".to_string());
    }

    // French-to-English full game name mapping (especially 3DS titles)
    let fr_game_map: Vec<(&str, Vec<&str>)> = vec![
        ("pokemon soleil", vec!["Pokemon Sun (Europe) (En,Ja,Fr,De,Es,It,Zh,Ko)", "Pokemon Sun"]),
        ("pokemon lune", vec!["Pokemon Moon (Europe) (En,Ja,Fr,De,Es,It,Zh,Ko)", "Pokemon Moon"]),
        ("pokemon ultra soleil", vec!["Pokemon Ultra Sun (Europe) (En,Ja,Fr,De,Es,It,Zh,Ko)", "Pokemon Ultra Sun"]),
        ("pokemon ultra lune", vec!["Pokemon Ultra Moon (Europe) (En,Ja,Fr,De,Es,It,Zh,Ko)", "Pokemon Ultra Moon"]),
        ("pokemon x", vec!["Pokemon X (Europe) (En,Ja,Fr,De,Es,It,Ko)", "Pokemon X"]),
        ("pokemon y", vec!["Pokemon Y (Europe) (En,Ja,Fr,De,Es,It,Ko)", "Pokemon Y"]),
        ("pokemon saphir alpha", vec!["Pokemon Alpha Sapphire (Europe) (En,Ja,Fr,De,Es,It,Ko)", "Pokemon Alpha Sapphire"]),
        ("pokemon rubis omega", vec!["Pokemon Omega Ruby (Europe) (En,Ja,Fr,De,Es,It,Ko)", "Pokemon Omega Ruby"]),
        ("pokemon donjon mystere", vec!["Pokemon Super Mystery Dungeon (Europe) (En,Fr,De,Es,It)", "Pokemon Super Mystery Dungeon"]),
        ("pokemon super donjon mystere", vec!["Pokemon Super Mystery Dungeon (Europe) (En,Fr,De,Es,It)", "Pokemon Super Mystery Dungeon"]),
        ("yo kai watch 1", vec!["Yo-Kai Watch (Europe) (En,Fr,De,Es,It)", "Yo-Kai Watch", "Yo-kai Watch"]),
        ("yo kai watch 2 bouffis", vec!["Yo-Kai Watch 2 - Fleshy Souls (Europe) (En,Fr,De,Es,It,Nl)", "Yo-Kai Watch 2 - Fleshy Souls"]),
        ("yo kai watch 2 espris", vec!["Yo-Kai Watch 2 - Bony Spirits (Europe) (En,Fr,De,Es,It,Nl)", "Yo-Kai Watch 2 - Bony Spirits"]),
        ("yo kai watch 2 spectres", vec!["Yo-Kai Watch 2 - Psychic Specters (Europe) (En,Fr,De,Es,It,Nl,Ru)", "Yo-Kai Watch 2 - Psychic Specters"]),
        ("zelda link beetween", vec!["Legend of Zelda, The - A Link Between Worlds (USA) (En,Fr,Es)", "The Legend of Zelda - A Link Between Worlds"]),
        ("zelda link between", vec!["Legend of Zelda, The - A Link Between Worlds (USA) (En,Fr,Es)", "The Legend of Zelda - A Link Between Worlds"]),
        ("zelda ocarina", vec!["Legend of Zelda, The - Ocarina of Time 3D (USA) (En,Fr,Es)", "The Legend of Zelda - Ocarina of Time 3D"]),
        ("zelda majora", vec!["Legend of Zelda, The - Majora's Mask 3D (USA) (En,Fr,Es)", "The Legend of Zelda - Majora's Mask 3D"]),
        ("pac man party 3d", vec!["Pac-Man Party 3D (Europe) (En,Fr,De,Es,It)", "Pac-Man Party 3D"]),
        ("pac man et l'aventure des fantomes", vec!["Pac-Man and the Ghostly Adventures (Europe) (En,Fr,De,Es,It)", "Pac-Man and the Ghostly Adventures"]),
        ("pac man et l'aventure de", vec!["Pac-Man and the Ghostly Adventures (Europe) (En,Fr,De,Es,It)", "Pac-Man and the Ghostly Adventures"]),
        ("sonic", vec!["Sonic Generations (Europe) (En,Fr,De,Es,It)", "Sonic Lost World (Europe) (En,Fr,De,Es,It)"]),
        ("mario kart 7", vec!["Mario Kart 7 (Europe) (En,Fr,De,Es,It,Nl,Pt,Ru)", "Mario Kart 7 (USA) (En,Fr,Es)", "Mario Kart 7"]),
    ];
    for (fr_name, en_names) in &fr_game_map {
        if lower_pure.contains(fr_name) || lower_pure == *fr_name {
            for en in en_names {
                candidates.insert(0, en.to_string());
            }
            break;
        }
    }

    // Split composite titles on " & " or " and " (e.g. "Wii Sports & Wii Sports Resort")
    let split_pattern = regex::Regex::new(r"(?i) & | and ").unwrap();
    for chunk in split_pattern.split(&pure) {
        let trimmed = chunk.trim();
        if !trimmed.is_empty() && trimmed.len() > 3 && trimmed != pure {
            candidates.push(trimmed.to_string());
        }
    }

    // Title case fallback for all-lowercase names ("zelda wind waker" -> "Zelda Wind Waker")
    if !pure.is_empty() && pure.chars().all(|c| !c.is_uppercase()) {
        let titled: String = pure.split_whitespace()
            .map(|w| {
                let mut chars = w.chars();
                match chars.next() {
                    Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
        if titled != pure {
            candidates.push(titled.clone());
            // Add franchise-prefixed variants that Wikipedia/Libretro expect
            if titled.contains("Zelda") && !titled.starts_with("The Legend") {
                candidates.push(format!("The Legend of Zelda - {}", titled.replace("Zelda ", "")));
                // Wii U titles have "HD" suffix in libretro
                if console == "Wii U" {
                    candidates.push(format!("The Legend of Zelda - {} HD", titled.replace("Zelda ", "")));
                    candidates.push(format!("The Legend of Zelda - The {} HD", titled.replace("Zelda ", "")));
                }
            }
        }
    }

    // Wii U: many games have "HD" in libretro name
    if console == "Wii U" {
        let base_for_hd = candidates.clone();
        for c in &base_for_hd {
            if !c.contains("HD") && !c.contains("(") {
                candidates.push(format!("{} HD", c));
            }
        }
    }

    // Deduplicate and return
    let mut seen = std::collections::HashSet::new();
    candidates.retain(|c| seen.insert(c.clone()));
    candidates
}

/// Expand a single candidate into variants with region suffixes for libretro
fn libretro_name_variants(name: &str) -> Vec<String> {
    let regions = ["(USA)", "(USA, Europe)", "(Europe)", "(World)"];
    let mut variants = vec![name.to_string()];

    let has_region = name.contains("(USA") || name.contains("(Europe") || name.contains("(World") || name.contains("(Japan");

    // Add region suffixes if not already present
    if !has_region {
        for region in &regions {
            variants.push(format!("{} {}", name, region));
        }
    }

    // "The X - Y" → "X, The - Y" inversion (libretro convention)
    if name.starts_with("The ") {
        let rest = &name[4..];
        let inverted = if let Some(dash_pos) = rest.find(" - ") {
            format!("{}, The - {}", &rest[..dash_pos], &rest[dash_pos + 3..])
        } else {
            format!("{}, The", rest)
        };
        variants.push(inverted.clone());
        if !has_region {
            for region in &regions {
                variants.push(format!("{} {}", inverted, region));
            }
        }
    }

    variants
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RomStoreEntry {
    pub id: String,
    pub name: String,
    pub console: String,
    pub region: String,
    pub size: String,
    pub file_name: String,
    pub download_url: String,
    pub ia_id: Option<String>,
    pub thumbnail_url: Option<String>,
}

/* ============================
   RetroGameSets.fr Integration
   ============================ */

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RgsConstructeurInfo {
    pub id: String,
    pub nom: String,
    pub icon: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RgsConsoleInfo {
    pub id: String,
    pub nom: String,
    pub image: String,
    pub constructeur_id: String,
    pub nb_liens: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RgsLien {
    pub id: String,
    pub url: String,
    pub nb_fichiers: String,
    pub taille: String,
    pub mot_de_passe: Option<String>,
    pub createur: String,
    pub informations: Option<String>,
    pub dossier: Option<String>,
    pub is_signaled: String,
    pub date_creation: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RgsFile {
    pub nom: String,
    pub taille: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RgsSearchResult {
    pub id: String,
    #[serde(rename = "titre")]
    pub nom: String,
    #[serde(rename = "type")]
    pub type_result: String,
    pub constructeur_nom: Option<String>,
    pub image: Option<String>,
    pub lien_id: Option<String>,
    pub url: Option<String>,
}

#[allow(dead_code)]
fn sanitize_filename(name: &str) -> String {
    let mut s = name.to_string();
    let invalid_chars = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
    for &c in &invalid_chars {
        s = s.replace(c, "");
    }
    s.trim().trim_matches(|c: char| c == '.').to_string()
}

fn get_ia_collection(console: &str) -> Option<&'static str> {
    match console {
        "NES" => Some("nintendo-nes-usa-redump"),
        "Super Nintendo" => Some("nintendo-super-nintendo-usa-redump"),
        "Nintendo 64" => Some("nintendo-n64-usa-redump"),
        "Game Boy Advance" => Some("nintendo-game-boy-advance-usa-redump"),
        "Nintendo DS" => Some("nintendo-ds-usa-redump"),
        "GameCube / Wii" => Some("nintendo-wii-usa-redump"),
        "Wii U" => Some("nintendo-wii-u-usa-redump"),
        "Nintendo Switch" => Some("nintendo-switch-roms-collection-v2"),
        "Virtual Boy" => Some("nintendo-virtual-boy-usa-redump"),
        "PlayStation 1" => Some("sony-playstation-usa-redump"),
        "PlayStation 2" => Some("sony-playstation-2-usa-redump"),
        "PlayStation 3" => Some("sony-playstation-3-psn-r-set-1"),
        "PlayStation Portable" => Some("sony-playstation-portable-usa-redump"),
        "Dreamcast" => Some("sega-dreamcast-usa-redump"),
        "Mega Drive" => Some("sega-mega-drive-genesis-usa-redump"),
        "Master System" => Some("sega-master-system-mark-iii-usa-redump"),
        "Saturn" => Some("sega-saturn-usa-redump"),
        "Game Gear" => Some("sega-game-gear-usa-redump"),
        "Xbox" => Some("microsoft-xbox-usa-redump"),
        "Neo-Geo" => Some("snk-neo-geo-aes-usa-redump"),
        "PC Engine" => Some("nec-pc-engine-turbografx-16-usa-redump"),
        "Atari 2600" => Some("atari-2600-usa-redump"),
        _ => None,
    }
}

#[allow(dead_code)]
fn console_to_folder(console: &str) -> &str {
    match console {
        "NES" => "NES",
        "Super Nintendo" => "SNES",
        "Nintendo 64" => "N64",
        "Game Boy Advance" => "GBA",
        "Nintendo DS" => "NDS",
        "GameCube / Wii" => "GameCube",
        "Wii U" => "WiiU",
        "Nintendo Switch" => "Switch",
        "Virtual Boy" => "VirtualBoy",
        "PlayStation 1" => "PS1",
        "PlayStation 2" => "PS2",
        "PlayStation 3" => "PS3",
        "PlayStation Portable" => "PSP",
        "Dreamcast" => "Dreamcast",
        "Mega Drive" => "MegaDrive",
        "Master System" => "MasterSystem",
        "Saturn" => "Saturn",
        "Game Gear" => "GameGear",
        "Xbox" => "Xbox",
        "Arcade" => "Arcade",
        "Neo-Geo" => "NeoGeo",
        "PC Engine" => "PCEngine",
        "Atari 2600" => "Atari2600",
        "WonderSwan" => "WonderSwan",
        "DOS / Win 3.x" => "DOS",
        _ => "Other",
    }
}

fn get_rom_catalog() -> Vec<RomStoreEntry> {
    vec![
        RomStoreEntry { 
            id: "smb-nes".to_string(), 
            name: "Super Mario Bros.".to_string(), 
            console: "NES".to_string(), 
            region: "USA".to_string(), 
            size: "40 KB".to_string(), 
            file_name: "Super Mario Bros. (USA).nes".to_string(), 
            download_url: "https://archive.org/download/nes-roms-collection/Super%20Mario%20Bros.%20%28USA%29.nes".to_string(), 
            ia_id: None,
            thumbnail_url: None 
        },
        RomStoreEntry { 
            id: "smw-snes".to_string(), 
            name: "Super Mario World".to_string(), 
            console: "Super Nintendo".to_string(), 
            region: "USA".to_string(), 
            size: "512 KB".to_string(), 
            file_name: "Super Mario World (USA).sfc".to_string(), 
            download_url: "https://archive.org/download/snes-roms-collection/Super%20Mario%20World%20%28USA%29.sfc".to_string(), 
            ia_id: None,
            thumbnail_url: None 
        },
    ]
}

#[allow(dead_code)]
fn detect_console_from_title(title: &str) -> Option<String> {
    let t = title.to_lowercase();
    if t.contains("wii u") || t.contains("(wiiu)") || t.contains("wii-u") { return Some("Wii U".to_string()); }
    if t.contains("nintendo switch") || t.contains("(switch)") || t.contains(" nx ") { return Some("Nintendo Switch".to_string()); }
    if t.contains("playstation 3") || t.contains("ps3") { return Some("PlayStation 3".to_string()); }
    if t.contains("playstation 2") || t.contains("ps2") { return Some("PlayStation 2".to_string()); }
    if t.contains("playstation 1") || t.contains(" ps1 ") || t.contains("psx") { return Some("PlayStation 1".to_string()); }
    if t.contains("nintendo 64") || t.contains("n64") || t.ends_with(" 64") || t.contains(" 64 ") { return Some("Nintendo 64".to_string()); }
    if t.contains("nes") || t.contains("nintendo entertainment system") { return Some("NES".to_string()); }
    if t.contains("snes") || t.contains("super nintendo") { return Some("Super Nintendo".to_string()); }
    if t.contains("game boy advance") || t.contains("gba") { return Some("Game Boy Advance".to_string()); }
    if t.contains("game boy color") || t.contains("gbc") { return Some("Game Boy Color".to_string()); }
    if t.contains("game boy") || t.contains(" gb ") && !t.contains("gba") { return Some("Game Boy".to_string()); }
    if t.contains("gamegear") || t.contains("game gear") { return Some("Game Gear".to_string()); }
    if t.contains("nintendo ds") || t.contains("nds") || t.contains(" ds") { return Some("Nintendo DS".to_string()); }
    if t.contains("psp") || t.contains("playstation portable") { return Some("PlayStation Portable".to_string()); }
    if t.contains("dreamcast") || t.contains("dc") { return Some("Dreamcast".to_string()); }
    if t.contains("mega drive") || t.contains("genesis") || t.contains("megadrive") { return Some("Sega Genesis".to_string()); }
    if t.contains("saturn") { return Some("Saturn".to_string()); }
    if t.contains("gamecube") || (t.contains("wii") && !t.contains("wii u")) { return Some("GameCube / Wii".to_string()); }
    None
}

#[tauri::command]
async fn search_rom_store(query: String, console_filter: Option<String>) -> Result<Vec<RomStoreEntry>, String> {
    push_log("INFO", &format!("Store: recherche '{}' (filtre: {:?})", query, console_filter));
    let mut results = Vec::new();
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    let (q, sort_param, final_console) = if let Some(console) = console_filter {
        if let Some(collection) = get_ia_collection(&console) {
            if query.is_empty() {
                (format!("collection:({})", collection), "&sort[]=downloads%20desc", console)
            } else {
                let clean_query = query.replace("&", " ").replace(":", " ").replace("-", " ").replace("+", " ");
                let words: Vec<&str> = clean_query.split_whitespace().collect();
                let title_clause = words.iter().map(|w| format!("title:({})", w)).collect::<Vec<_>>().join(" AND ");
                (format!("collection:({}) AND ({})", collection, title_clause), "", console)
            }
        } else {
            return Err(format!("Unknown console: {}", console));
        }
    } else {
        if query.is_empty() {
            ("mediatype:software AND (subject:rom OR subject:redump OR subject:no-intro) AND (subject:nintendo OR subject:sony OR subject:sega) AND downloads:[1000 TO *] AND NOT title:(part OR bios OR set OR merged OR pack OR collection OR bundle OR \"rom pack\" OR \"rom set\" OR roms OR \"iso set\" OR \"romset\")".to_string(), "&sort[]=downloads%20desc", "Multiple".to_string())
        } else {
            let clean_q = query.replace("&", " ").replace(":", " ").replace("+", " ");
            let words: Vec<&str> = clean_q.split_whitespace().collect();
            let title_clause = words.iter().map(|w| format!("title:({})", w)).collect::<Vec<_>>().join(" AND ");
            (format!("mediatype:software AND {} AND NOT title:(pack OR bundle OR collection OR romset OR roms)", title_clause), "&sort[]=downloads%20desc", "Mixed".to_string())
        }
    };
    
    let url = format!(
        "https://archive.org/advancedsearch.php?q={}&fl[]=identifier,title,description,collection,subject&rows=150{}&output=json",
        urlencoding::encode(&q),
        sort_param
    );
    
    let response = client.get(&url).send().await.map_err(|e| e.to_string())?;
    if !response.status().is_success() { return Err(format!("Archive.org API error: {}", response.status())); }
    
    let json: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
    
    if let Some(docs) = json["response"]["docs"].as_array() {
        for doc in docs {
            let title = doc["title"].as_str().unwrap_or("Unknown").to_string();
            let entry_console = if final_console == "Mixed" || final_console == "Multiple" {
                let collection_str = doc["collection"].as_array().and_then(|a| a.first()).and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
                let subject_str = doc["subject"].as_array()
                    .map(|a| a.iter().map(|v| v.as_str().unwrap_or("")).collect::<Vec<_>>().join(" "))
                    .unwrap_or_else(|| doc["subject"].as_str().unwrap_or("").to_string())
                    .to_lowercase();
                let combined_meta = format!("{} {}", collection_str, subject_str);
                match combined_meta.as_str() {
                    c if c.contains("nes") && !c.contains("snes") => "NES".to_string(),
                    c if c.contains("snes") || c.contains("super-nintendo") => "Super Nintendo".to_string(),
                    c if c.contains("n64") => "Nintendo 64".to_string(),
                    c if c.contains("game-boy-advance") || c.contains("gba") => "Game Boy Advance".to_string(),
                    c if c.contains("gbc") || c.contains("game-boy-color") => "Game Boy Color".to_string(),
                    c if c.contains("gb") || c.contains("game-boy") => "Game Boy".to_string(),
                    c if c.contains("playstation-2") || c.contains("ps2") => "PlayStation 2".to_string(),
                    c if c.contains("playstation-3") || c.contains("ps3") => "PlayStation 3".to_string(),
                    c if c.contains("portable") || c.contains("psp") => "PlayStation Portable".to_string(),
                    c if c.contains("playstation") && !c.contains("2") && !c.contains("portable") => "PlayStation 1".to_string(),
                    c if c.contains("nintendo-ds") || c.contains("-ds") => "Nintendo DS".to_string(),
                    c if c.contains("wii") && !c.contains("wii-u") => "GameCube / Wii".to_string(),
                    c if (c.contains("megadrive") || c.contains("genesis")) && !c.contains("dreamcast") => "Sega Genesis".to_string(),
                    _ => final_console.clone()
                }
            } else {
                final_console.clone()
            };

            let entry_id = doc["identifier"].as_str().unwrap_or("").to_string();
            let thumb = format!("https://archive.org/services/img/{}?&height=320", entry_id);
            
            results.push(RomStoreEntry {
                id: entry_id.clone(),
                name: title,
                console: entry_console,
                region: "World".to_string(), 
                size: "Varies".to_string(),
                file_name: "".to_string(), 
                download_url: "".to_string(), 
                ia_id: Some(entry_id.clone()),
                thumbnail_url: Some(thumb),
            });
        }
    }
    
    if results.is_empty() && query.is_empty() { results = get_rom_catalog(); }
    Ok(results)
}

#[tauri::command]
fn get_featured_games() -> Vec<RomStoreEntry> {
    vec![
        RomStoreEntry {
            id: "loz-oot".to_string(),
            name: "The Legend of Zelda: Ocarina of Time".to_string(),
            console: "Nintendo 64".to_string(),
            region: "World".to_string(),
            size: "32 MB".to_string(),
            file_name: "".to_string(),
            download_url: "".to_string(),
            ia_id: Some("legend-of-zelda-ocarina-of-time".to_string()),
            thumbnail_url: Some("https://thumbnails.libretro.com/Nintendo%20-%20Nintendo%2064/Named_Boxarts/Legend%20of%20Zelda%2C%20The%20-%20Ocarina%20of%20Time%20(USA).png".to_string()),
        },
        RomStoreEntry {
            id: "sm64".to_string(),
            name: "Super Mario 64".to_string(),
            console: "Nintendo 64".to_string(),
            region: "World".to_string(),
            size: "8 MB".to_string(),
            file_name: "".to_string(),
            download_url: "".to_string(),
            ia_id: Some("super-mario-64-cartridge-rom-z64-file".to_string()),
            thumbnail_url: Some("https://thumbnails.libretro.com/Nintendo%20-%20Nintendo%2064/Named_Boxarts/Super%20Mario%2064%20(USA).png".to_string()),
        },
        RomStoreEntry {
            id: "sonic-adv".to_string(),
            name: "Sonic Adventure".to_string(),
            console: "Dreamcast".to_string(),
            region: "World".to_string(),
            size: "1 GB".to_string(),
            file_name: "".to_string(),
            download_url: "".to_string(),
            ia_id: Some("chd_dc_smpl".to_string()),
            thumbnail_url: Some("https://thumbnails.libretro.com/Sega%20-%20Dreamcast/Named_Boxarts/Sonic%20Adventure%20(USA).png".to_string()),
        },
        RomStoreEntry {
            id: "m-kart-wii".to_string(),
            name: "Mario Kart Wii".to_string(),
            console: "Wii".to_string(),
            region: "World".to_string(),
            size: "4.3 GB".to_string(),
            file_name: "".to_string(),
            download_url: "".to_string(),
            ia_id: Some("mario-kart-wii_20200604".to_string()),
            thumbnail_url: Some("https://thumbnails.libretro.com/Nintendo%20-%20Wii/Named_Boxarts/Mario%20Kart%20Wii%20(USA).png".to_string()),
        },
        RomStoreEntry {
            id: "pkmn-em".to_string(),
            name: "Pokemon Emerald Version".to_string(),
            console: "Game Boy Advance".to_string(),
            region: "World".to_string(),
            size: "16 MB".to_string(),
            file_name: "".to_string(),
            download_url: "".to_string(),
            ia_id: Some("pokemon-emerald-version".to_string()),
            thumbnail_url: Some("https://thumbnails.libretro.com/Nintendo%20-%20Game%20Boy%20Advance/Named_Boxarts/Pokemon%20-%20Emerald%20Version%20(USA%2C%20Europe).png".to_string()),
        },
        RomStoreEntry {
            id: "halo-ce".to_string(),
            name: "Halo: Combat Evolved".to_string(),
            console: "Xbox".to_string(),
            region: "World".to_string(),
            size: "3.5 GB".to_string(),
            file_name: "".to_string(),
            download_url: "".to_string(),
            ia_id: Some("halo-combat-evolved".to_string()),
            thumbnail_url: Some("https://thumbnails.libretro.com/Microsoft%20-%20Xbox/Named_Boxarts/Halo%20-%20Combat%20Evolved%20(USA).png".to_string()),
        },
        RomStoreEntry {
            id: "gta-sa".to_string(),
            name: "Grand Theft Auto: San Andreas".to_string(),
            console: "PlayStation 2".to_string(),
            region: "World".to_string(),
            size: "4 GB".to_string(),
            file_name: "".to_string(),
            download_url: "".to_string(),
            ia_id: Some("grand-theft-auto-san-andreas".to_string()),
            thumbnail_url: Some("https://thumbnails.libretro.com/Sony%20-%20PlayStation%202/Named_Boxarts/Grand%20Theft%20Auto%20-%20San%20Andreas%20(USA)%20(v1.03).png".to_string()),
        },
        RomStoreEntry {
            id: "pkmn-plat".to_string(),
            name: "Pokémon Platinum Version".to_string(),
            console: "Nintendo DS".to_string(),
            region: "USA".to_string(),
            size: "128 MB".to_string(),
            file_name: "".to_string(),
            download_url: "".to_string(),
            ia_id: Some("pokemon-platinum-version".to_string()),
            thumbnail_url: Some("https://thumbnails.libretro.com/Nintendo%20-%20Nintendo%20DS/Named_Boxarts/Pokemon%20-%20Platinum%20Version%20(USA)%20(Rev%201).png".to_string()),
        },
        RomStoreEntry {
            id: "metroid-pr".to_string(),
            name: "Metroid Prime".to_string(),
            console: "GameCube".to_string(),
            region: "World".to_string(),
            size: "1.4 GB".to_string(),
            file_name: "".to_string(),
            download_url: "".to_string(),
            ia_id: Some("metroid-prime-usa".to_string()),
            thumbnail_url: Some("https://thumbnails.libretro.com/Nintendo%20-%20GameCube/Named_Boxarts/Metroid%20Prime%20(USA).png".to_string()),
        },
        RomStoreEntry {
            id: "crash-3".to_string(),
            name: "Crash Bandicoot 3: Warped".to_string(),
            console: "PlayStation 1".to_string(),
            region: "World".to_string(),
            size: "500 MB".to_string(),
            file_name: "".to_string(),
            download_url: "".to_string(),
            ia_id: Some("crash-bandicoot-3-warped".to_string()),
            thumbnail_url: Some("https://thumbnails.libretro.com/Sony%20-%20PlayStation/Named_Boxarts/Crash%20Bandicoot%20-%20Warped%20(USA).png".to_string()),
        },
        RomStoreEntry {
            id: "nsmbw".to_string(),
            name: "New Super Mario Bros. Wii".to_string(),
            console: "Wii".to_string(),
            region: "World".to_string(),
            size: "3.5 GB".to_string(),
            file_name: "".to_string(),
            download_url: "".to_string(),
            ia_id: Some("new-super-mario-bros-wii_202112".to_string()),
            thumbnail_url: Some("https://thumbnails.libretro.com/Nintendo%20-%20Wii/Named_Boxarts/New%20Super%20Mario%20Bros.%20Wii%20(USA).png".to_string()),
        }
    ]
}

#[tauri::command]
async fn download_rom(
    app_handle: tauri::AppHandle,
    download_url_arg: String,
    console: String,
    _rom_name: String,
    file_name_arg: String,
    ia_id: Option<String>,
    store_id: Option<String>,
) -> Result<String, String> {
    let config = get_config();
    let roms_dir = std::path::PathBuf::from(&config.roms_directory);
    let safe_console = console.replace("..", "").replace('/', "").replace('\\', "");
    let dest_dir = roms_dir.join(&safe_console);
    if !dest_dir.starts_with(&roms_dir) {
        return Err("Invalid console directory".to_string());
    }
    if !dest_dir.exists() {
        fs::create_dir_all(&dest_dir).map_err(|e| format!("Failed to create console directory: {}", e))?;
    }

    let mut final_url = download_url_arg;
    let mut final_file_name = if file_name_arg.is_empty() { "game.bin".to_string() } else { file_name_arg.replace("..", "").replace('/', "").replace('\\', "") };
    let final_store_id = store_id.clone();

    // ROM file extensions we care about
    let rom_extensions = [".zip", ".7z", ".iso", ".bin", ".nds", ".gba", ".rvz", ".wbfs", ".chd", ".cue", ".nes", ".sfc", ".smc", ".n64", ".z64", ".gcm", ".nsp", ".xci"];
    
    if let Some(ref id) = ia_id {
        let meta_url = format!("https://archive.org/metadata/{}", id);
        let client = reqwest::Client::builder().user_agent("Mozilla/5.0").build().map_err(|e| e.to_string())?;
        if let Ok(response) = client.get(&meta_url).send().await {
            if let Ok(json) = response.json::<serde_json::Value>().await {
                if let Some(files) = json["files"].as_array() {
                    let mut best_file = None;
                    let mut max_size = 0u64;
                    for file in files {
                        let name = file["name"].as_str().unwrap_or("");
                        let lower_name = name.to_lowercase();
                        let size = file["size"].as_str().unwrap_or("0").parse::<u64>().unwrap_or(0);
                        if rom_extensions.iter().any(|ext| lower_name.ends_with(ext)) {
                            if size >= max_size {
                                max_size = size;
                                best_file = Some(name.to_string());
                            }
                        }
                    }
                    if let Some(ref f) = best_file {
                        let encoded_f = urlencoding::encode(f);
                        final_url = format!("https://archive.org/download/{}/{}", id, encoded_f);
                        // Use the resolved filename if we didn't have one
                        if final_file_name == "game.bin" {
                            final_file_name = f.clone();
                        }
                    }
                }
            }
        }
    }
    
    // Bail out if we still have no URL to download from
    if final_url.is_empty() {
        return Err("Download failed: Could not resolve a download URL. The Archive.org item may be unavailable.".to_string());
    }
    
    println!("[Download] ======= Starting download =======");
    println!("[Download] ROM: {} ({})", _rom_name, console);
    println!("[Download] URL: {}", if final_url.len() > 80 { &final_url[..80] } else { &final_url });
    println!("[Download] File: {}", final_file_name);
    println!("[Download] IA ID: {:?}", ia_id);
    
    let dest = dest_dir.join(&final_file_name);
    let client_builder = reqwest::Client::builder().user_agent("Mozilla/5.0");
    let client = client_builder.build().map_err(|e| e.to_string())?;
    
    let mut response = client.get(&final_url).send().await.map_err(|e| {
        println!("[Download] HTTP request failed: {}", e);
        e.to_string()
    })?;
    
    let status = response.status();
    println!("[Download] HTTP Status: {}", status);
    
    // Check HTTP status — abort on any non-success response
    if !status.is_success() {
        println!("[Download] ABORTING: Server returned {}", status);
        return Err(format!("Download failed: Server returned HTTP {} for this ROM. The file may have been removed from Archive.org.", status));
    }
    
    let content_length = response.content_length();
    println!("[Download] Content-Length: {:?}", content_length);
    
    // Anti-Stub Guard: Only trigger when we KNOW the size and it's suspiciously small
    if let Some(cl) = content_length {
        if cl > 0 && cl < 1_000_000 && (console == "Wii" || console == "PlayStation 2" || console == "GameCube" || console == "GameCube / Wii") {
            println!("[Download] BLOCKED by Anti-Stub guard: {} bytes", cl);
            return Err(format!("Download failed: The file is too small ({}). This usually means the file is restricted or corrupted on Archive.org.", format_size(cl)));
        }
    }
    
    let mut total_size = content_length.unwrap_or(0);
    if total_size == 0 { total_size = 100 * 1024 * 1024; }

    let mut file = fs::File::create(&dest).map_err(|e| e.to_string())?;
    let mut downloaded_bytes = 0u64;
    let mut last_emit = std::time::Instant::now();
    let start_time = std::time::Instant::now();
    let mut throttle_bytes = 0u64;
    let mut throttle_start = std::time::Instant::now();

    use std::io::Write;
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        let chunk: bytes::Bytes = chunk;
        file.write_all(&chunk).map_err(|e| {
            println!("[Download] Write error: {}", e);
            e.to_string()
        })?;
        downloaded_bytes += chunk.len() as u64;

        // Bandwidth throttling
        let bw_limit = BANDWIDTH_LIMIT_KBPS.load(AtomicOrdering::Relaxed);
        if bw_limit > 0 {
            throttle_bytes += chunk.len() as u64;
            let limit_bps = bw_limit * 1024;
            let elapsed = throttle_start.elapsed().as_secs_f64();
            let expected_time = throttle_bytes as f64 / limit_bps as f64;
            if expected_time > elapsed {
                let sleep_ms = ((expected_time - elapsed) * 1000.0) as u64;
                tokio::time::sleep(std::time::Duration::from_millis(sleep_ms)).await;
            }
            if throttle_start.elapsed().as_secs() >= 2 {
                throttle_bytes = 0;
                throttle_start = std::time::Instant::now();
            }
        }

        if last_emit.elapsed().as_millis() >= 400 {
            let progress = (downloaded_bytes as f64 / total_size as f64 * 100.0) as u32;
            let elapsed_sec = start_time.elapsed().as_secs_f64();
            let speed_bps = if elapsed_sec > 0.0 { downloaded_bytes as f64 / elapsed_sec } else { 0.0 };
            let eta = if speed_bps > 0.0 && total_size > downloaded_bytes {
                ((total_size - downloaded_bytes) as f64 / speed_bps) as u64
            } else { 0 };

            let _ = app_handle.emit("rom-download-progress", serde_json::json!({
                "store_id": final_store_id,
                "status": "downloading",
                "progress": progress,
                "downloaded_bytes": downloaded_bytes,
                "total_bytes": total_size,
                "speed_bps": speed_bps as u64,
                "eta": eta
            }));
            last_emit = std::time::Instant::now();
        }
    }
    
    // Close the file handle before post-processing
    drop(file);
    println!("[Download] Download complete: {} bytes saved to {}", downloaded_bytes, dest.display());

    // === ZIP Auto-Extraction ===
    // Check if the downloaded file is actually a ZIP archive
    let is_zip = final_url.to_lowercase().ends_with(".zip")
        || final_file_name.to_lowercase().ends_with(".zip")
        || is_zip_file(&dest);

    let is_7z = final_file_name.to_lowercase().ends_with(".7z")
        || final_url.to_lowercase().ends_with(".7z");

    if is_zip || is_7z {
        let _ = app_handle.emit("rom-download-progress", serde_json::json!({
            "store_id": final_store_id,
            "status": "extracting",
            "progress": 99,
            "message": "Extracting archive..."
        }));
    }

    if is_zip {
        println!("[Download] Detected ZIP archive, extracting...");
        match extract_rom_zip(&dest, &dest_dir) {
            Ok(extracted_files) => {
                println!("[Download] Extracted {} files: {:?}", extracted_files.len(), extracted_files);
                let _ = fs::remove_file(&dest);
                println!("[Download] Deleted ZIP archive: {}", dest.display());
            }
            Err(e) => {
                println!("[Download] ZIP extraction failed: {} — keeping raw file", e);
            }
        }
    } else if is_7z {
        println!("[Download] Detected 7z archive, extracting...");
        let dest_clone = dest.clone();
        let dest_dir_clone = dest_dir.clone();
        let handle = app_handle.clone();
        let file_name_clone = final_file_name.clone();
        tokio::task::spawn_blocking(move || {
            use tauri::Emitter;
            match extract_7z(&dest_clone, &dest_dir_clone) {
                Ok(()) => {
                    println!("[Download] Extracted 7z successfully: {}", file_name_clone);
                    let _ = fs::remove_file(&dest_clone);
                    let _ = handle.emit("import-extract-done", serde_json::json!({
                        "file": file_name_clone,
                        "status": "success"
                    }));
                }
                Err(e) => {
                    println!("[Download] 7z extraction failed: {} — keeping raw file", e);
                    let _ = handle.emit("import-extract-done", serde_json::json!({
                        "file": file_name_clone,
                        "status": "error",
                        "message": e
                    }));
                }
            }
        });
    } else {
        println!("[Download] File is not an archive, keeping as-is");
    }
    
    let _ = app_handle.emit("rom-download-progress", serde_json::json!({
        "store_id": final_store_id,
        "status": "done",
        "progress": 100
    }));
    
    println!("[Download] ======= Download finished =======");
    Ok(format!("Downloaded to {}", dest_dir.display()))
}

#[tauri::command]
async fn finalize_rgs_import(
    app_handle: tauri::AppHandle,
    src_path: String,
    console: String,
) -> Result<String, String> {
    push_log("INFO", &format!("Import RGS: {} → console '{}'", src_path, console));
    let config = get_config();
    let roms_dir = std::path::PathBuf::from(&config.roms_directory);
    let roms_root = roms_dir.canonicalize().unwrap_or_else(|_| roms_dir.clone());
    let normalized = normalize_console_folder(&console);
    let dest_dir = roms_dir.join(&normalized);

    if !dest_dir.exists() {
        fs::create_dir_all(&dest_dir).map_err(|e| format!("Failed to create console directory: {}", e))?;
    }

    // Validate dest_dir is within ROMs root (prevent path traversal via console name)
    let dest_dir_canon = dest_dir.canonicalize().unwrap_or_else(|_| dest_dir.clone());
    if !dest_dir_canon.starts_with(&roms_root) {
        return Err("Invalid console path — directory traversal blocked".to_string());
    }

    let src = std::path::PathBuf::from(&src_path);
    if !src.exists() {
        return Err("Source file not found".to_string());
    }

    let file_name = src.file_name()
        .ok_or_else(|| "Invalid source filename".to_string())?
        .to_string_lossy()
        .to_string();

    // Auto-detect console from extension if console is "Mixed" or generic
    let mut final_console = console.clone();
    let lower_file = file_name.to_lowercase();
    
    if console == "Mixed" || console == "Unknown" {
        let catalog = emulators::get_catalog();
        for emu in catalog {
            if emu.supported_extensions.iter().any(|ext| lower_file.ends_with(&ext.to_lowercase())) {
                final_console = emu.console;
                break;
            }
        }
    }

    let normalized_final = normalize_console_folder(&final_console);
    let dest_dir = roms_dir.join(&normalized_final);
    if !dest_dir.exists() {
        fs::create_dir_all(&dest_dir).map_err(|e| format!("Failed to create console directory: {}", e))?;
    }

    let dest = dest_dir.join(&file_name);

    println!("[Import] Moving {} to {} (Console: {} -> {})", src.display(), dest.display(), final_console, normalized_final);
    
    // Try to move directly (instant if on same drive)
    if let Err(_) = fs::rename(&src, &dest) {
        // Fallback to copy then delete if move fails (cross-device)
        fs::copy(&src, &dest).map_err(|e| format!("Failed to copy file: {}", e))?;
        let _ = fs::remove_file(&src);
    }

    // Archive Auto-Extraction
    let is_zip = lower_file.ends_with(".zip") || is_zip_file(&dest);
    let is_7z = lower_file.ends_with(".7z");

    if is_zip {
        println!("[Import] Detected ZIP archive, extracting...");
        match extract_rom_zip(&dest, &dest_dir) {
            Ok(extracted_files) => {
                println!("[Import] Extracted {} files", extracted_files.len());
                let _ = fs::remove_file(&dest);
            }
            Err(e) => {
                println!("[Import] ZIP extraction failed: {} — keeping raw file", e);
            }
        }
    } else if is_7z {
        let size_mb = fs::metadata(&dest).map(|m| m.len() / 1_048_576).unwrap_or(0);
        println!("[Import] Detected 7z archive ({} MB), extracting in background...", size_mb);
        let dest_clone = dest.clone();
        let dest_dir_clone = dest_dir.clone();
        let handle = app_handle.clone();
        let file_name_clone = file_name.clone();
        // Fire-and-forget: extract in background, notify frontend when done
        tokio::task::spawn_blocking(move || {
            use tauri::Emitter;
            let result = extract_7z(&dest_clone, &dest_dir_clone);
            match result {
                Ok(()) => {
                    println!("[Import] Extracted 7z successfully: {}", file_name_clone);
                    let _ = fs::remove_file(&dest_clone);
                    let _ = handle.emit("import-extract-done", serde_json::json!({
                        "file": file_name_clone,
                        "status": "success"
                    }));
                }
                Err(e) => {
                    println!("[Import] 7z extraction failed: {} — keeping raw file", e);
                    let _ = handle.emit("import-extract-done", serde_json::json!({
                        "file": file_name_clone,
                        "status": "error",
                        "message": e
                    }));
                }
            }
        });
        return Ok(format!("Extracting {} ({} MB) in background — you'll be notified when done", file_name, size_mb));
    }

    push_log("INFO", &format!("Import RGS terminé: {} → dossier {}", file_name, final_console));
    Ok(format!("Imported successfully to {} folder", final_console))
}

/// Check the file's magic bytes to see if it's a ZIP
fn is_zip_file(path: &std::path::Path) -> bool {
    if let Ok(mut f) = fs::File::open(path) {
        let mut buf = [0u8; 4];
        use std::io::Read;
        if f.read_exact(&mut buf).is_ok() {
            return &buf == b"PK\x03\x04";
        }
    }
    false
}

/// Extract a ZIP archive, returning the list of extracted file names
fn extract_rom_zip(zip_path: &std::path::Path, dest_dir: &std::path::Path) -> Result<Vec<String>, String> {
    let file = fs::File::open(zip_path).map_err(|e| format!("Cannot open zip: {}", e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Invalid zip: {}", e))?;
    
    let mut extracted = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;

        // Use enclosed_name() to prevent path traversal (Zip Slip)
        let safe_path = match entry.enclosed_name() {
            Some(p) => p.to_path_buf(),
            None => continue,
        };
        let name = safe_path.to_string_lossy().to_string();

        // Skip directories and macOS resource forks
        if entry.is_dir() || name.starts_with("__MACOSX") || name.starts_with(".") {
            continue;
        }

        // Extract to the destination directory (flatten — no subdirectories)
        let file_name = safe_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| name.clone());
        
        let out_path = dest_dir.join(&file_name);
        println!("[Extract] {} -> {}", name, out_path.display());
        
        let mut out_file = fs::File::create(&out_path).map_err(|e| format!("Cannot create {}: {}", file_name, e))?;
        std::io::copy(&mut entry, &mut out_file).map_err(|e| format!("Extract failed for {}: {}", file_name, e))?;
        extracted.push(file_name);
    }
    
    Ok(extracted)
}

fn format_size(bytes: u64) -> String {
    if bytes == 0 { return "0 B".to_string(); }
    let k = 1024u64;
    let sizes = ["B", "KB", "MB", "GB", "TB"];
    let i = (bytes as f64).log(k as f64).floor() as usize;
    if i >= sizes.len() { return format!("{} bytes", bytes); }
    format!("{:.2} {}", bytes as f64 / k.pow(i as u32) as f64, sizes[i])
}

#[tauri::command]
fn get_store_consoles() -> Vec<String> {
    vec![
        "NES", "Super Nintendo", "Nintendo 64", "Game Boy Advance", "Nintendo DS",
        "GameCube / Wii", "Wii U", "Nintendo Switch", "PlayStation 1", "PlayStation 2",
        "PlayStation 3", "PlayStation Portable", "Dreamcast", "Mega Drive", "Master System",
        "Saturn", "Xbox", "Neo-Geo", "PC Engine", "Atari 2600"
    ].into_iter().map(|s| s.to_string()).collect()
}

// ============================
// RetroGameSets.fr Commands
// ============================

#[tauri::command]
fn get_rgs_constructeurs() -> Vec<RgsConstructeurInfo> {
    vec![
        RgsConstructeurInfo { id: "6".into(), nom: "Nintendo".into(), icon: "🍄".into() },
        RgsConstructeurInfo { id: "9".into(), nom: "Sony".into(), icon: "🎮".into() },
        RgsConstructeurInfo { id: "7".into(), nom: "Sega".into(), icon: "🔵".into() },
        RgsConstructeurInfo { id: "4".into(), nom: "Microsoft".into(), icon: "❎".into() },
        RgsConstructeurInfo { id: "8".into(), nom: "SNK".into(), icon: "🅰️".into() },
        RgsConstructeurInfo { id: "2".into(), nom: "Atari".into(), icon: "🟤".into() },
        RgsConstructeurInfo { id: "5".into(), nom: "NEC".into(), icon: "🔶".into() },
        RgsConstructeurInfo { id: "1".into(), nom: "Arcade".into(), icon: "🕹️".into() },
        RgsConstructeurInfo { id: "12".into(), nom: "Commodore".into(), icon: "💾".into() },
        RgsConstructeurInfo { id: "10".into(), nom: "Panasonic".into(), icon: "📀".into() },
    ]
}

#[tauri::command]
async fn get_rgs_consoles(constructeur_id: String) -> Result<Vec<RgsConsoleInfo>, String> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("https://www.retrogamesets.fr/get_constructeurs.php?id={}", constructeur_id);
    let response = client.get(&url).send().await.map_err(|e| e.to_string())?;
    
    if !response.status().is_success() {
        return Err(format!("RGS API error: {}", response.status()));
    }
    
    let json: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
    
    let mut consoles = Vec::new();
    if let Some(console_arr) = json["consoles"].as_array() {
        for c in console_arr {
            let nb = c["nb_liens"].as_u64()
                .or_else(|| c["nb_liens"].as_str().and_then(|s| s.parse::<u64>().ok()))
                .unwrap_or(0) as u32;
            
            // Only include consoles that have at least 1 link
            if nb == 0 { continue; }
            
            consoles.push(RgsConsoleInfo {
                id: c["id"].as_str().unwrap_or("").to_string(),
                nom: c["nom"].as_str().unwrap_or("").to_string(),
                image: c["image"].as_str().unwrap_or("").to_string(),
                constructeur_id: constructeur_id.clone(),
                nb_liens: nb,
            });
        }
    }
    
    Ok(consoles)
}

#[tauri::command]
async fn get_rgs_liens(console_id: String) -> Result<Vec<RgsLien>, String> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("https://www.retrogamesets.fr/get_liens.php?id={}", console_id);
    let response = client.get(&url).send().await.map_err(|e| e.to_string())?;
    
    if !response.status().is_success() {
        return Err(format!("RGS API error: {}", response.status()));
    }
    
    let json: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
    
    let mut liens = Vec::new();
    if let Some(arr) = json.as_array() {
        for l in arr {
            let is_signaled = l["is_signaled"].as_str().unwrap_or("0").to_string();
            // Skip reported/dead links
            if is_signaled == "1" { continue; }
            
            let url_str = l["url"].as_str().unwrap_or("").to_string();
            if url_str.is_empty() || !url_str.starts_with("http") { continue; }
            
            liens.push(RgsLien {
                id: l["id"].as_str().unwrap_or("").to_string(),
                url: url_str,
                nb_fichiers: l["nb_fichiers"].as_str().unwrap_or("0").to_string(),
                taille: l["taille"].as_str().unwrap_or("Inconnu").to_string(),
                mot_de_passe: l["mot_de_passe"].as_str().map(|s| s.to_string()),
                createur: l["createur"].as_str().unwrap_or("Anonyme").to_string(),
                informations: l["informations"].as_str().map(|s| s.to_string()),
                dossier: l["dossier"].as_str().map(|s| s.to_string()),
                is_signaled,
                date_creation: l["date_creation"].as_str().map(|s| s.to_string()),
            });
        }
    }
    
    Ok(liens)
}

#[tauri::command]
async fn search_rgs(query: String) -> Result<Vec<RgsSearchResult>, String> {
    push_log("INFO", &format!("Recherche RGS: '{}'", query));
    if query.len() < 2 {
        return Ok(vec![]);
    }
    
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("https://www.retrogamesets.fr/recherche_ajax.php?query={}", urlencoding::encode(&query));
    let response = client.get(&url).send().await.map_err(|e| e.to_string())?;
    
    if !response.status().is_success() {
        return Err(format!("RGS search error: {}", response.status()));
    }
    
    let json: Vec<RgsSearchResult> = response.json().await.map_err(|e| e.to_string())?;
    Ok(json)
}

#[tauri::command]
async fn scrape_1fichier_dir(url: String) -> Result<Vec<RgsFile>, String> {
    push_log("INFO", &format!("Scrape dossier 1fichier: {}", url));
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    // Ensure the AF cookie is set to avoid reloads/redirects
    let response = client.get(&url)
        .header("Cookie", "AF=3186111")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let html = response.text().await.map_err(|e| e.to_string())?;
    
    use scraper::{Html, Selector};
    let document = Html::parse_document(&html);
    
    // 1fichier folders list filenames in table rows. 
    // Filenames are in <td> with class alg file-obj
    // Size is the next <td>
    let row_selector = Selector::parse("tr").map_err(|_| "Invalid row selector")?;
    let file_cell_selector = Selector::parse("td.file-obj").map_err(|_| "Invalid cell selector")?;
    let size_cell_selector = Selector::parse("td:not(.file-obj)").map_err(|_| "Invalid size selector")?;

    let mut files = Vec::new();
    
    for row in document.select(&row_selector) {
        if let Some(file_cell) = row.select(&file_cell_selector).next() {
            if let Some(link) = file_cell.select(&Selector::parse("a").unwrap()).next() {
                let name = link.text().collect::<Vec<_>>().join("");
                let mut url = link.value().attr("href").unwrap_or("").to_string();
                if url.starts_with("/") {
                    url = format!("https://1fichier.com{}", url);
                }
                
                // Get size (it's the next td)
                let mut size = String::from("Unknown");
                let mut next_cells = row.select(&size_cell_selector);
                if let Some(size_cell) = next_cells.next() {
                    size = size_cell.text().collect::<Vec<_>>().join("").trim().to_string();
                }
                
                if !url.is_empty() && !name.is_empty() {
                    files.push(RgsFile { nom: name, taille: size, url });
                }
            }
        }
    }
    
    Ok(files)
}

#[tauri::command]
async fn resolve_1fichier_names(
    app_handle: tauri::AppHandle,
    urls: Vec<String>,
    collection_name: String,
) -> Result<Vec<(String, String)>, String> {
    push_log("INFO", &format!("Résolution des noms 1fichier: {} URLs (collection: {})", urls.len(), collection_name));
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;

    let mut results: Vec<(String, String)> = Vec::new();
    let total = urls.len();
    let batch_size = 5;

    for chunk_start in (0..total).step_by(batch_size) {
        let chunk_end = (chunk_start + batch_size).min(total);
        let chunk = &urls[chunk_start..chunk_end];

        let futures: Vec<_> = chunk.iter().map(|url| {
            let c = client.clone();
            let u = url.clone();
            async move {
                let name = match c.get(&u)
                    .header("Cookie", "AF=3186111")
                    .send()
                    .await
                {
                    Ok(resp) => {
                        if let Ok(html) = resp.text().await {
                            extract_1fichier_filename(&html)
                                .unwrap_or_else(|| u.split('?').last().unwrap_or("unknown").to_string())
                        } else {
                            u.split('?').last().unwrap_or("unknown").to_string()
                        }
                    }
                    Err(e) => {
                        push_log("WARN", &format!("Échec résolution nom pour {}: {}", u, e));
                        u.split('?').last().unwrap_or("unknown").to_string()
                    }
                };
                (u, name)
            }
        }).collect();

        let batch_results = futures::future::join_all(futures).await;
        results.extend(batch_results.clone());

        let _ = app_handle.emit("resolve-names-batch", serde_json::json!({
            "collection": collection_name,
            "batch": batch_results.iter().map(|(u, n)| serde_json::json!({"url": u, "name": n})).collect::<Vec<_>>(),
            "done": chunk_end,
            "total": total
        }));

        if chunk_end < total {
            tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        }
    }

    push_log("INFO", &format!("Résolution terminée: {}/{} noms récupérés", results.iter().filter(|(_, n)| n.contains('.')).count(), total));
    Ok(results)
}

fn extract_1fichier_filename(html: &str) -> Option<String> {
    use scraper::{Html, Selector};
    let doc = Html::parse_document(html);

    // Method 1: filename in the page title or header
    // 1fichier shows filename in various places
    if let Ok(sel) = Selector::parse("td.normal") {
        for el in doc.select(&sel) {
            let text = el.text().collect::<Vec<_>>().join("");
            let text = text.trim().to_string();
            if text.contains('.') && text.len() > 3 && !text.contains("http") {
                return Some(text);
            }
        }
    }

    // Method 2: the filename appears in a specific div
    if let Ok(sel) = Selector::parse(".dlname, .name, #filename") {
        for el in doc.select(&sel) {
            let text = el.text().collect::<Vec<_>>().join("").trim().to_string();
            if !text.is_empty() && text.contains('.') {
                return Some(text);
            }
        }
    }

    // Method 3: look in the page content for file-like patterns
    let re = regex::Regex::new(r"(?i)([A-Za-z0-9\[\]\(\)\s\-_.!&']+\.(zip|7z|rar|iso|nsp|xci|wux|wud|rpx|wbfs|rvz|chd|nds|gba|nes|sfc|n64|z64|bin|cue|pbp|cso))").unwrap();
    if let Some(m) = re.find(html) {
        let name = m.as_str().trim().to_string();
        if name.len() > 3 && name.len() < 200 {
            return Some(name);
        }
    }

    None
}

#[tauri::command]
async fn download_1fichier(
    app_handle: tauri::AppHandle,
    url: String,
    console: String,
    password: Option<String>,
    queue_id: String,
) -> Result<String, String> {
    use std::io::Write;

    push_log("INFO", &format!("Download 1fichier: {} (console: {}, pwd: {}, queue: {})", url, console, password.is_some(), queue_id));
    let config = get_config();
    let roms_dir = std::path::PathBuf::from(&config.roms_directory);
    let console_folder = normalize_console_folder(&console).replace("..", "").replace('/', "").replace('\\', "");
    let dest_dir = roms_dir.join(&console_folder);
    if !dest_dir.starts_with(&roms_dir) {
        return Err("Invalid console directory".to_string());
    }
    if !dest_dir.exists() {
        fs::create_dir_all(&dest_dir).map_err(|e| format!("Failed to create directory: {}", e))?;
    }

    let emit_progress = |status: &str, progress: u32, msg: &str| {
        let _ = app_handle.emit("1fichier-progress", serde_json::json!({
            "queue_id": queue_id,
            "status": status,
            "progress": progress,
            "message": msg
        }));
    };

    emit_progress("resolving", 0, "Fetching download page...");

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .redirect(reqwest::redirect::Policy::limited(10))
        .connect_timeout(std::time::Duration::from_secs(30))
        .read_timeout(std::time::Duration::from_secs(300))
        .cookie_store(true)
        .build()
        .map_err(|e| format!("Client build error: {}", e))?;

    // Step 1: GET the file page to extract tokens/form data
    let page_resp = client.get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to load page: {}", e))?;

    if !page_resp.status().is_success() {
        return Err(format!("1fichier returned HTTP {}", page_resp.status()));
    }

    let page_html = page_resp.text().await.map_err(|e| e.to_string())?;

    // Check for errors (file removed, etc.)
    if page_html.contains("not found") || page_html.contains("has been removed") || page_html.contains("fichier n'existe pas") {
        push_log("ERROR", &format!("1fichier: fichier supprimé — {}", url));
        return Err("File has been removed from 1fichier.".to_string());
    }

    // Check for rate-limit on GET page (no countdown = truly blocked)
    let has_countdown = page_html.contains("var ct");
    if !has_countdown {
        if page_html.contains("temporairement limit") || page_html.contains("forte affluence") {
            push_log("WARN", &format!("1fichier: rate limit on GET (no countdown) — browser fallback for {}", url));
            return Err("OPEN_BROWSER".to_string());
        }
    }

    push_log("INFO", &format!("1fichier: GET OK — has_countdown={}, page_len={}", has_countdown, page_html.len()));

    // Extract countdown timer
    let wait_seconds = {
        let re = regex::Regex::new(r"var\s+ct\s*=\s*(\d+)").unwrap();
        re.captures(&page_html)
            .and_then(|c| c[1].parse::<u64>().ok())
            .unwrap_or(0)
    };

    push_log("INFO", &format!("1fichier: countdown = {}s", wait_seconds));

    let form_params: Vec<(String, String)> = {
        let mut params = Vec::new();
        params.push(("dl_no_ssl".to_string(), "on".to_string()));
        if let Some(ref pw) = password {
            params.push(("pass".to_string(), pw.clone()));
        }
        params
    };

    // Quick probe: POST immediately to detect IP rate-limit without wasting 60s
    // - If rate-limited: returns "temporairement limité" → open browser right away
    // - If NOT rate-limited but too early: returns countdown page → wait then POST for real
    if wait_seconds > 0 {
        emit_progress("resolving", 0, "Checking...");
        let probe_resp = client.post(&url)
            .header("Referer", &url)
            .header("Origin", "https://1fichier.com")
            .form(&form_params)
            .send()
            .await
            .map_err(|e| format!("Probe POST failed: {}", e))?;
        let probe_html = probe_resp.text().await.map_err(|e| e.to_string())?;

        if probe_html.contains("temporairement limit") || probe_html.contains("forte affluence") {
            push_log("WARN", &format!("1fichier: IP rate-limited (detected via probe) — opening browser for {}", url));
            return Err("OPEN_BROWSER".to_string());
        }
        push_log("INFO", "1fichier: probe OK — not rate-limited, starting countdown");

        // Fresh GET to reset session timer (probe consumed it)
        let fresh = client.get(&url).send().await.map_err(|e| format!("Fresh GET failed: {}", e))?;
        let _ = fresh.text().await;
    }

    // Wait the countdown with progress bar
    if wait_seconds > 0 {
        emit_progress("resolving", 0, &format!("Waiting {}s...", wait_seconds));
        push_log("INFO", &format!("1fichier: starting {}s countdown", wait_seconds));
        for elapsed in 0..wait_seconds {
            if cancelled_downloads().lock().map(|s| s.contains(&queue_id)).unwrap_or(false) {
                cancelled_downloads().lock().map(|mut s| s.remove(&queue_id)).ok();
                push_log("INFO", &format!("Download cancelled during countdown (queue: {})", queue_id));
                emit_progress("cancelled", 0, "Download cancelled");
                return Err("Download cancelled by user".to_string());
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            let pct = ((elapsed + 1) as f64 / wait_seconds as f64 * 100.0) as u32;
            let remaining = wait_seconds - elapsed - 1;
            emit_progress("resolving", pct, &format!("Waiting {}s...", remaining));
        }
        push_log("INFO", "1fichier: countdown finished, sending POST");
    }

    emit_progress("resolving", 100, "Requesting download link...");

    // Step 2: POST to get download link after countdown
    let post_resp = client.post(&url)
        .header("Referer", &url)
        .header("Origin", "https://1fichier.com")
        .form(&form_params)
        .send()
        .await
        .map_err(|e| format!("POST failed: {}", e))?;

    let post_status = post_resp.status();
    let post_final_url = post_resp.url().to_string();
    push_log("INFO", &format!("1fichier: POST response status={}, final_url={}", post_status, post_final_url));

    // Check if we were redirected directly to a CDN URL
    let final_url = post_resp.url().to_string();
    let redirected_to_cdn = final_url.contains(".1fichier.com/") && final_url != url && !final_url.contains("img.1fichier.com");

    let download_link = if redirected_to_cdn {
        push_log("INFO", &format!("1fichier: redirected directly to CDN: {}", final_url));
        final_url
    } else {

    let post_html = post_resp.text().await.map_err(|e| e.to_string())?;
    push_log("INFO", &format!("1fichier: POST HTML len={}", post_html.len()));

    // Extract direct download link from response
    {
        use scraper::{Html, Selector};
        let doc = Html::parse_document(&post_html);

        // Try: <a class="ok btn-general btn-general-lg" href="...">
        let link_sel = Selector::parse("a.ok").unwrap_or_else(|_| Selector::parse("a").unwrap());
        let mut found_link = None;
        for el in doc.select(&link_sel) {
            if let Some(href) = el.value().attr("href") {
                if href.contains("1fichier.com") && !href.contains("/dir/") && href.starts_with("http") {
                    found_link = Some(href.to_string());
                    break;
                }
            }
        }

        // Fallback: regex for any CDN link
        if found_link.is_none() {
            let re = regex::Regex::new(r#"(https?://[a-z0-9\-]+\.1fichier\.com/[^\s"<>]+)"#).unwrap();
            for m in re.find_iter(&post_html) {
                let candidate = m.as_str();
                if !candidate.contains("img.1fichier.com")
                    && !candidate.ends_with(".css")
                    && !candidate.ends_with(".js")
                    && !candidate.ends_with(".ico")
                    && !candidate.ends_with(".png")
                {
                    found_link = Some(candidate.to_string());
                    break;
                }
            }
        }

        // Fallback 2: detect various failure modes
        if found_link.is_none() {
            // POST returned the countdown page again = server rejected (didn't wait long enough or session issue)
            if post_html.contains("var ct") {
                push_log("WARN", &format!("1fichier: POST returned countdown page again — opening browser for {}", url));
                return Err("OPEN_BROWSER".to_string());
            }
            if post_html.contains("must wait") || post_html.contains("Veuillez patienter") || post_html.contains("You must wait") || post_html.contains("attendre entre chaque") {
                push_log("WARN", "1fichier: must wait between downloads");
                return Err("OPEN_BROWSER".to_string());
            }
            if post_html.contains("temporairement limit") || post_html.contains("forte affluence") {
                push_log("WARN", &format!("1fichier: rate limit après POST — ouverture navigateur pour {}", url));
                return Err("OPEN_BROWSER".to_string());
            }
            // Log the full POST response for debugging
            push_log("ERROR", &format!("1fichier: impossible d'extraire le lien pour {} — HTML len={}, first 300: {}", url, post_html.len(), &post_html[..post_html.len().min(300)]));
            return Err("OPEN_BROWSER".to_string());
        }

        let link = found_link.unwrap();
        push_log("INFO", &format!("1fichier: download link extracted: {}", link));
        link
    }
    };

    emit_progress("downloading", 0, "Starting download...");
    push_log("INFO", &format!("1fichier: starting download from: {}", download_link));

    // Step 3: Download the file
    let dl_resp = client.get(&download_link)
        .send()
        .await
        .map_err(|e| format!("Download request failed: {}", e))?;

    if !dl_resp.status().is_success() {
        return Err(format!("Download failed: HTTP {}", dl_resp.status()));
    }

    // Extract filename from Content-Disposition or URL
    let file_name = dl_resp.headers()
        .get("content-disposition")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| {
            let re = regex::Regex::new(r##"filename\*?=(?:UTF-8''|"?)([^";\r\n]+)"??"##).unwrap();
            re.captures(s).map(|c| c[1].to_string())
        })
        .or_else(|| {
            download_link.split('/').last()
                .map(|s| urlencoding::decode(s).unwrap_or_else(|_| s.into()).to_string())
                .filter(|s| !s.is_empty() && s.contains('.'))
        })
        .unwrap_or_else(|| format!("1fichier_download_{}.bin", &queue_id[..8.min(queue_id.len())]));

    let total_size = dl_resp.content_length().unwrap_or(0);
    let dest_path = dest_dir.join(&file_name);

    let mut file = fs::File::create(&dest_path)
        .map_err(|e| format!("Failed to create file: {}", e))?;

    let mut downloaded: u64 = 0;
    let mut last_emit_time = std::time::Instant::now();
    let start_time = std::time::Instant::now();
    let mut stream = dl_resp;

    while let Some(chunk) = stream.chunk().await.map_err(|e| e.to_string())? {
        // Check for cancellation
        if cancelled_downloads().lock().map(|s| s.contains(&queue_id)).unwrap_or(false) {
            drop(file);
            let _ = fs::remove_file(&dest_path);
            cancelled_downloads().lock().map(|mut s| s.remove(&queue_id)).ok();
            push_log("INFO", &format!("Download annulé: {} (queue: {})", file_name, queue_id));
            emit_progress("cancelled", 0, "Download cancelled");
            return Err("Download cancelled by user".to_string());
        }

        file.write_all(&chunk).map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;

        // Bandwidth throttling
        let bw_limit = BANDWIDTH_LIMIT_KBPS.load(AtomicOrdering::Relaxed);
        if bw_limit > 0 {
            let elapsed = start_time.elapsed().as_secs_f64();
            let expected = downloaded as f64 / (bw_limit as f64 * 1024.0);
            if expected > elapsed {
                tokio::time::sleep(std::time::Duration::from_millis(((expected - elapsed) * 1000.0) as u64)).await;
            }
        }

        if last_emit_time.elapsed().as_millis() >= 400 {
            let progress = if total_size > 0 {
                (downloaded as f64 / total_size as f64 * 100.0) as u32
            } else {
                50
            };
            let speed = if start_time.elapsed().as_secs_f64() > 0.0 {
                downloaded as f64 / start_time.elapsed().as_secs_f64()
            } else { 0.0 };
            let eta = if speed > 0.0 && total_size > downloaded {
                ((total_size - downloaded) as f64 / speed) as u64
            } else { 0 };

            let _ = app_handle.emit("1fichier-progress", serde_json::json!({
                "queue_id": queue_id,
                "status": "downloading",
                "progress": progress,
                "downloaded_bytes": downloaded,
                "total_bytes": total_size,
                "speed_bps": speed as u64,
                "eta": eta,
                "file_name": file_name,
                "message": format!("{} / {}", format_size(downloaded), if total_size > 0 { format_size(total_size) } else { "?".to_string() })
            }));
            last_emit_time = std::time::Instant::now();
        }
    }
    drop(file);
    cancelled_downloads().lock().map(|mut s| s.remove(&queue_id)).ok();

    // Auto-extract if ZIP/7z
    let lower = file_name.to_lowercase();
    if lower.ends_with(".zip") || lower.ends_with(".7z") {
        emit_progress("extracting", 95, &format!("Extracting: {}", file_name));
    }

    if lower.ends_with(".zip") {
        let zip_path = dest_path.clone();
        if let Ok(zip_file) = fs::File::open(&zip_path) {
            if let Ok(mut archive) = zip::ZipArchive::new(zip_file) {
                for i in 0..archive.len() {
                    if let Ok(mut entry) = archive.by_index(i) {
                        if let Some(safe_name) = entry.enclosed_name() {
                            let out_path = dest_dir.join(safe_name);
                            if entry.is_dir() {
                                let _ = fs::create_dir_all(&out_path);
                            } else {
                                if let Some(p) = out_path.parent() {
                                    let _ = fs::create_dir_all(p);
                                }
                                if let Ok(mut outfile) = fs::File::create(&out_path) {
                                    let _ = std::io::copy(&mut entry, &mut outfile);
                                }
                            }
                        }
                    }
                }
                let _ = fs::remove_file(&zip_path);
            }
        }
    } else if lower.ends_with(".7z") {
        let sz_path = dest_path.clone();
        let out_dir = dest_dir.clone();
        if sevenz_rust::decompress_file(&sz_path, &out_dir).is_ok() {
            let _ = fs::remove_file(&sz_path);
        }
    }

    emit_progress("complete", 100, &format!("Done: {}", file_name));
    push_log("INFO", &format!("1fichier download terminé: {} → {} (queue: {})", file_name, dest_dir.display(), queue_id));

    Ok(file_name)
}

#[tauri::command]
fn cancel_download(download_id: String) -> Result<(), String> {
    push_log("INFO", &format!("Annulation demandée pour: {}", download_id));
    cancelled_downloads().lock()
        .map(|mut s| { s.insert(download_id); })
        .map_err(|e| e.to_string())
}

// ============================================================
// ============================================================
// MYRIENT — individual ROM downloads from myrient.erista.me
// Apache-indexed HTTP listing, one file per ROM, No-Intro / Redump.
// ============================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MyrientConsole {
    pub id: String,
    pub name: String,
    pub url: String,
    pub manufacturer: String,
    pub target_console: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MyrientFile {
    pub name: String,
    pub url: String,
    pub size: String,
    pub console: String,
}

fn myrient_consoles_catalog() -> Vec<MyrientConsole> {
    let entries: &[(&str, &str, &str, &str, &str)] = &[
        // (id, name, url, manufacturer, target_console)
        ("nes",        "NES",               "https://myrient.erista.me/files/No-Intro/Nintendo%20-%20Nintendo%20Entertainment%20System%20%28Headered%29/",         "Nintendo", "NES"),
        ("snes",       "Super Nintendo",    "https://myrient.erista.me/files/No-Intro/Nintendo%20-%20Super%20Nintendo%20Entertainment%20System/",                   "Nintendo", "Super Nintendo"),
        ("n64",        "Nintendo 64",       "https://myrient.erista.me/files/No-Intro/Nintendo%20-%20Nintendo%2064%20%28BigEndian%29/",                             "Nintendo", "Nintendo 64"),
        ("gb",         "Game Boy",          "https://myrient.erista.me/files/No-Intro/Nintendo%20-%20Game%20Boy/",                                                  "Nintendo", "Game Boy"),
        ("gbc",        "Game Boy Color",    "https://myrient.erista.me/files/No-Intro/Nintendo%20-%20Game%20Boy%20Color/",                                          "Nintendo", "Game Boy Color"),
        ("gba",        "Game Boy Advance",  "https://myrient.erista.me/files/No-Intro/Nintendo%20-%20Game%20Boy%20Advance/",                                        "Nintendo", "Game Boy Advance"),
        ("nds",        "Nintendo DS",       "https://myrient.erista.me/files/No-Intro/Nintendo%20-%20Nintendo%20DS%20%28Decrypted%29/",                             "Nintendo", "Nintendo DS"),
        ("gamecube",   "GameCube",          "https://myrient.erista.me/files/Redump/Nintendo%20-%20GameCube%20-%20NKit%20RVZ%20%5Bzstd-19-128k%5D/",               "Nintendo", "GameCube"),
        ("wii",        "Wii",               "https://myrient.erista.me/files/Redump/Nintendo%20-%20Wii%20-%20NKit%20RVZ%20%5Bzstd-19-128k%5D/",                    "Nintendo", "Wii"),
        ("ps1",        "PlayStation 1",     "https://myrient.erista.me/files/Redump/Sony%20-%20PlayStation/",                                                       "Sony",     "PlayStation 1"),
        ("ps2",        "PlayStation 2",     "https://myrient.erista.me/files/Redump/Sony%20-%20PlayStation%202/",                                                   "Sony",     "PlayStation 2"),
        ("psp",        "PSP",               "https://myrient.erista.me/files/Redump/Sony%20-%20PlayStation%20Portable/",                                            "Sony",     "PSP"),
        ("megadrive",  "Mega Drive",        "https://myrient.erista.me/files/No-Intro/Sega%20-%20Mega%20Drive%20-%20Genesis/",                                      "Sega",     "Mega Drive"),
        ("mastersys",  "Master System",     "https://myrient.erista.me/files/No-Intro/Sega%20-%20Master%20System%20-%20Mark%20III/",                                "Sega",     "Master System"),
        ("dreamcast",  "Dreamcast",         "https://myrient.erista.me/files/Redump/Sega%20-%20Dreamcast/",                                                         "Sega",     "Dreamcast"),
        ("saturn",     "Saturn",            "https://myrient.erista.me/files/Redump/Sega%20-%20Saturn/",                                                            "Sega",     "Saturn"),
    ];
    entries.iter().map(|(id, name, url, manuf, target)| MyrientConsole {
        id: id.to_string(),
        name: name.to_string(),
        url: url.to_string(),
        manufacturer: manuf.to_string(),
        target_console: target.to_string(),
    }).collect()
}

#[tauri::command]
fn get_myrient_consoles() -> Result<Vec<MyrientConsole>, String> {
    Ok(myrient_consoles_catalog())
}

#[tauri::command]
async fn browse_myrient(console_url: String, console_id: String) -> Result<Vec<MyrientFile>, String> {
    // Validate URL to prevent SSRF — only allow exact domain
    if !console_url.starts_with("https://myrient.erista.me/") {
        return Err("Invalid URL — only myrient.erista.me is allowed".to_string());
    }

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let response = client.get(&console_url).send().await.map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("Myrient returned HTTP {}", response.status()));
    }
    let html = response.text().await.map_err(|e| e.to_string())?;

    use scraper::{Html, Selector};
    let document = Html::parse_document(&html);

    // Apache index: table rows with <td><a href="...">name</a></td> <td>date</td> <td>size</td>
    let row_sel = Selector::parse("table tr").map_err(|_| "Invalid row selector")?;
    let link_sel = Selector::parse("td a").map_err(|_| "Invalid link selector")?;
    let td_sel  = Selector::parse("td").map_err(|_| "Invalid td selector")?;

    let catalog = myrient_consoles_catalog();
    let target_console = catalog.iter()
        .find(|c| c.id == console_id)
        .map(|c| c.target_console.clone())
        .unwrap_or_default();

    let mut files = Vec::new();
    for row in document.select(&row_sel) {
        let tds: Vec<_> = row.select(&td_sel).collect();
        if tds.len() < 2 { continue; }

        let link = match row.select(&link_sel).next() { Some(l) => l, None => continue };
        let href = link.value().attr("href").unwrap_or("");

        // Skip directories (end with "/") and parent link
        if href.ends_with('/') || href == "../" || href.starts_with("?") || href.is_empty() { continue; }

        let name = link.text().collect::<Vec<_>>().join("").trim().to_string();
        if name.is_empty() { continue; }

        let size = tds.get(2)
            .map(|td| td.text().collect::<Vec<_>>().join("").trim().to_string())
            .unwrap_or_default();

        // Build absolute URL (href is just the filename, already URL-encoded by Apache)
        let file_url = if href.starts_with("http") {
            href.to_string()
        } else {
            format!("{}{}", console_url, href)
        };

        files.push(MyrientFile {
            name,
            url: file_url,
            size,
            console: target_console.clone(),
        });
    }

    Ok(files)
}

#[tauri::command]
async fn download_myrient_rom(
    app_handle: tauri::AppHandle,
    url: String,
    console: String,
    file_name: String,
) -> Result<String, String> {
    use tauri::Emitter;
    if !url.starts_with("https://myrient.erista.me/") {
        return Err("Invalid URL — only myrient.erista.me is allowed".to_string());
    }
    let config = get_config();
    let roms_dir = std::path::PathBuf::from(&config.roms_directory);
    let safe_console = console.replace("..", "").replace('/', "").replace('\\', "");
    let safe_file = file_name.replace("..", "").replace('/', "").replace('\\', "");
    let dest_dir = roms_dir.join(&safe_console);
    if !dest_dir.starts_with(&roms_dir) {
        return Err("Invalid console directory".to_string());
    }
    if !dest_dir.exists() {
        fs::create_dir_all(&dest_dir).map_err(|e| format!("Failed to create console directory: {}", e))?;
    }

    let dest = dest_dir.join(&safe_file);

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .map_err(|e| e.to_string())?;

    let mut response = client.get(&url).send().await.map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("Download failed: HTTP {}", response.status()));
    }

    let total_size = response.content_length().unwrap_or(0);
    let mut downloaded_bytes = 0u64;
    let mut last_emit = std::time::Instant::now();
    let start_time = std::time::Instant::now();
    let mut throttle_bytes = 0u64;
    let mut throttle_start = std::time::Instant::now();

    let mut file = fs::File::create(&dest).map_err(|e| e.to_string())?;
    use std::io::Write;
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        file.write_all(&chunk).map_err(|e| e.to_string())?;
        downloaded_bytes += chunk.len() as u64;

        let bw_limit = BANDWIDTH_LIMIT_KBPS.load(AtomicOrdering::Relaxed);
        if bw_limit > 0 {
            throttle_bytes += chunk.len() as u64;
            let limit_bps = bw_limit * 1024;
            let elapsed = throttle_start.elapsed().as_secs_f64();
            let expected_time = throttle_bytes as f64 / limit_bps as f64;
            if expected_time > elapsed {
                let sleep_ms = ((expected_time - elapsed) * 1000.0) as u64;
                tokio::time::sleep(std::time::Duration::from_millis(sleep_ms)).await;
            }
            if throttle_start.elapsed().as_secs() >= 2 {
                throttle_bytes = 0;
                throttle_start = std::time::Instant::now();
            }
        }

        if last_emit.elapsed().as_millis() >= 400 {
            let progress = if total_size > 0 {
                (downloaded_bytes as f64 / total_size as f64 * 100.0) as u32
            } else { 0 };
            let elapsed = start_time.elapsed().as_secs_f64();
            let speed_bps = if elapsed > 0.0 { downloaded_bytes as f64 / elapsed } else { 0.0 };
            let eta = if speed_bps > 0.0 && total_size > downloaded_bytes {
                ((total_size - downloaded_bytes) as f64 / speed_bps) as u64
            } else { 0 };
            let _ = app_handle.emit("myrient-download-progress", serde_json::json!({
                "game": file_name,
                "status": "downloading",
                "progress": progress,
                "downloaded_bytes": downloaded_bytes,
                "total_bytes": total_size,
                "speed_bps": speed_bps as u64,
                "eta": eta
            }));
            last_emit = std::time::Instant::now();
        }
    }
    drop(file);

    // Auto-extract ZIP
    if file_name.to_lowercase().ends_with(".zip") || is_zip_file(&dest) {
        match extract_rom_zip(&dest, &dest_dir) {
            Ok(_) => { let _ = fs::remove_file(&dest); }
            Err(_) => {}
        }
    }

    let _ = app_handle.emit("myrient-download-progress", serde_json::json!({
        "game": file_name,
        "status": "done",
        "progress": 100
    }));

    push_log("INFO", &format!("Download terminé: {} → {}", file_name, dest_dir.display()));
    Ok(format!("Downloaded to {}", dest_dir.display()))
}

// VIMM'S LAIR — individual ROM downloads from vimm.net
// Classic vault, one ROM per game, box art for each entry.
// Downloads happen in the system browser (anti-bot on direct POST).
// ============================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VimmConsole {
    pub id: String,            // Vimm vault slug (e.g. "NES", "SNES", "GBA")
    pub name: String,          // display name
    pub image: String,         // box/icon URL (vimm's system icon)
    pub manufacturer: String,
    pub target_console: String, // console key used under ROMs/
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VimmGame {
    pub id: String,            // vault numeric id (e.g. "75294")
    pub name: String,          // game title
    pub region: String,        // "Europe", "USA", "Japan", "World", "-" or multi
    pub version: String,       // "1.0", "1.01"
    pub languages: String,     // language codes joined by space
    pub rating: String,        // "8.5" or "none"
    pub size: String,          // file size e.g. "1.2 GB"
    pub console_name: String,  // console display name from Vimm
    pub box_url: String,       // cover image URL
    pub page_url: String,      // page to open in system browser
}

fn vimm_consoles_catalog() -> Vec<VimmConsole> {
    // (vault_slug, display_name, manufacturer, target_console)
    let entries: &[(&str, &str, &str, &str)] = &[
        ("NES", "NES", "Nintendo", "NES"),
        ("SNES", "Super Nintendo", "Nintendo", "Super Nintendo"),
        ("N64", "Nintendo 64", "Nintendo", "Nintendo 64"),
        ("GB", "Game Boy", "Nintendo", "Game Boy"),
        ("GBC", "Game Boy Color", "Nintendo", "Game Boy Color"),
        ("GBA", "Game Boy Adv", "Nintendo", "Game Boy Advance"),
        ("VB", "Virtual Boy", "Nintendo", "Virtual Boy"),
        ("DS", "Nintendo DS", "Nintendo", "Nintendo DS"),
        ("3DS", "Nintendo 3DS", "Nintendo", "Nintendo 3DS"),
        ("GameCube", "GameCube", "Nintendo", "GameCube"),
        ("Wii", "Wii", "Nintendo", "Wii"),
        ("WiiWare", "WiiWare", "Nintendo", "Wii"),
        ("PS1", "PlayStation", "Sony", "PlayStation 1"),
        ("PS2", "PlayStation 2", "Sony", "PlayStation 2"),
        ("PS3", "PlayStation 3", "Sony", "PlayStation 3"),
        ("PSP", "PSP", "Sony", "PSP"),
        ("Genesis", "Genesis", "Sega", "Mega Drive"),
        ("SegaCD", "Sega CD", "Sega", "Sega CD"),
        ("32X", "Sega 32X", "Sega", "Sega 32X"),
        ("Saturn", "Saturn", "Sega", "Saturn"),
        ("Dreamcast", "Dreamcast", "Sega", "Dreamcast"),
        ("SMS", "Master System", "Sega", "Master System"),
        ("GG", "Game Gear", "Sega", "Game Gear"),
        ("Xbox", "Xbox", "Microsoft", "Xbox"),
        ("Xbox360", "Xbox 360", "Microsoft", "Xbox 360"),
        ("Atari2600", "Atari 2600", "Atari", "Atari 2600"),
        ("Atari5200", "Atari 5200", "Atari", "Atari 5200"),
        ("Atari7800", "Atari 7800", "Atari", "Atari 7800"),
        ("Jaguar", "Jaguar", "Atari", "Jaguar"),
        ("Lynx", "Lynx", "Atari", "Lynx"),
        ("TG16", "TurboGrafx-16", "NEC", "TurboGrafx-16"),
        ("TGCD", "TurboGrafx-CD", "NEC", "TurboGrafx-CD"),
        ("CDi", "CD-i", "Panasonic", "CD-i"),
    ];
    entries.iter().map(|(slug, name, manuf, target)| VimmConsole {
        id: slug.to_string(),
        name: name.to_string(),
        image: format!("https://vimm.net/images/{}.png", slug.to_lowercase()),
        manufacturer: manuf.to_string(),
        target_console: target.to_string(),
    }).collect()
}

#[tauri::command]
fn get_vimm_consoles() -> Result<Vec<VimmConsole>, String> {
    Ok(vimm_consoles_catalog())
}

#[tauri::command]
async fn browse_vimm(console_slug: String, letter: String) -> Result<Vec<VimmGame>, String> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())?;

    // Letter path. "#" means numeric section — vimm uses ?p=list&system=...&section=number
    let url = if letter == "#" {
        format!("https://vimm.net/vault/?p=list&system={}&section=number", console_slug)
    } else {
        format!("https://vimm.net/vault/{}/{}", console_slug, letter)
    };

    let response = client.get(&url).send().await.map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("Vimm returned HTTP {}", response.status()));
    }
    let html = response.text().await.map_err(|e| e.to_string())?;

    use scraper::{Html, Selector};
    let document = Html::parse_document(&html);

    // Rows live inside <table class="hoverable rounded"><tr>...</tr></table>. Each row has:
    // td[0] = link to /vault/{id} with game title
    // td[1] = region flags (img alt)
    // td[2] = version
    // td[3] = languages
    // td[4] = rating (inside <a>)
    let row_selector = Selector::parse("table.hovertable tr").map_err(|_| "Invalid row selector")?;
    let link_selector = Selector::parse("a").map_err(|_| "Invalid link selector")?;
    let td_selector = Selector::parse("td").map_err(|_| "Invalid td selector")?;
    let img_selector = Selector::parse("img.flag").map_err(|_| "Invalid img selector")?;

    let id_re = Regex::new(r"/vault/(\d+)").map_err(|e| e.to_string())?;

    let mut games = Vec::new();
    for row in document.select(&row_selector) {
        // Find an <a> whose href matches /vault/<numeric-id> that is NOT 999999 and has text
        let game_link = row.select(&link_selector).find(|a| {
            let href = a.value().attr("href").unwrap_or("");
            if let Some(caps) = id_re.captures(href) {
                let id = &caps[1];
                id != "999999" && !a.text().collect::<String>().trim().is_empty()
            } else {
                false
            }
        });
        let Some(link) = game_link else { continue };
        let href = link.value().attr("href").unwrap_or("");
        let Some(caps) = id_re.captures(href) else { continue };
        let id = caps[1].to_string();

        let name_raw = link.text().collect::<Vec<_>>().join("").trim().to_string();
        if name_raw.is_empty() { continue; }

        let tds: Vec<_> = row.select(&td_selector).collect();
        let region = tds.get(1)
            .and_then(|td| td.select(&img_selector).next())
            .and_then(|img| img.value().attr("title"))
            .unwrap_or("-")
            .to_string();
        let version = tds.get(2).map(|td| td.text().collect::<Vec<_>>().join("").trim().to_string()).unwrap_or_default();
        let languages = tds.get(3).map(|td| td.text().collect::<Vec<_>>().join("").trim().to_string()).unwrap_or_default();
        let rating = tds.get(4).map(|td| td.text().collect::<Vec<_>>().join("").trim().to_string()).unwrap_or_default();

        games.push(VimmGame {
            id: id.clone(),
            name: name_raw,
            region,
            version,
            languages,
            rating,
            size: String::new(),
            console_name: String::new(),
            box_url: format!("https://dl.vimm.net/image.php?type=box&id={}", id),
            page_url: format!("https://vimm.net/vault/{}", id),
        });
    }

    Ok(games)
}

#[tauri::command]
async fn search_vimm(query: String, console_slug: Option<String>) -> Result<Vec<VimmGame>, String> {
    if query.trim().len() < 2 { return Ok(vec![]); }
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())?;

    let url = if let Some(slug) = console_slug.as_ref().filter(|s| !s.is_empty()) {
        format!("https://vimm.net/vault/?p=list&system={}&q={}", slug, urlencoding::encode(query.trim()))
    } else {
        format!("https://vimm.net/vault/?p=list&q={}", urlencoding::encode(query.trim()))
    };
    let response = client.get(&url).send().await.map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("Vimm search HTTP {}", response.status()));
    }
    let html = response.text().await.map_err(|e| e.to_string())?;

    use scraper::{Html, Selector};
    let document = Html::parse_document(&html);
    let row_selector = Selector::parse("table.hovertable tr").map_err(|_| "Invalid row selector")?;
    let link_selector = Selector::parse("a").map_err(|_| "Invalid link selector")?;
    let td_selector = Selector::parse("td").map_err(|_| "Invalid td selector")?;
    let img_selector = Selector::parse("img").map_err(|_| "Invalid img selector")?;
    let id_re = Regex::new(r"/vault/(\d+)").map_err(|e| e.to_string())?;

    let mut games = Vec::new();
    for row in document.select(&row_selector) {
        let game_link = row.select(&link_selector).find(|a| {
            let href = a.value().attr("href").unwrap_or("");
            if let Some(caps) = id_re.captures(href) {
                let id = &caps[1];
                id != "999999" && !a.text().collect::<String>().trim().is_empty()
            } else {
                false
            }
        });
        let Some(link) = game_link else { continue };
        let href = link.value().attr("href").unwrap_or("");
        let Some(caps) = id_re.captures(href) else { continue };
        let id = caps[1].to_string();
        let name = link.text().collect::<Vec<_>>().join("").trim().to_string();
        if name.is_empty() { continue; }

        let tds: Vec<_> = row.select(&td_selector).collect();

        // Scan all columns (skip first = title) to find region (has <img>) and collect text values
        let mut region = String::new();
        let mut console_name = String::new();
        let mut version = String::new();
        let mut other_texts: Vec<String> = Vec::new();

        for (ci, td) in tds.iter().enumerate().skip(1) {
            // Check for flag image (region indicator)
            if let Some(img) = td.select(&img_selector).next() {
                let r = img.value().attr("title")
                    .or_else(|| img.value().attr("alt"))
                    .unwrap_or("")
                    .to_string();
                if !r.is_empty() && region.is_empty() {
                    region = r;
                    continue;
                }
            }
            let text = td.text().collect::<Vec<_>>().join("").trim().to_string();
            if text.is_empty() { continue; }
            // Heuristic: if it looks like a version number (digits and dots only)
            if text.chars().all(|c| c.is_ascii_digit() || c == '.') && version.is_empty() {
                version = text;
            } else if ci == 1 && console_slug.is_none() {
                // First non-title text col in global search is likely the console name
                console_name = text;
            } else {
                other_texts.push(text);
            }
        }
        // If no flag found but we have other_texts, first one might be region text
        if region.is_empty() {
            if let Some(first) = other_texts.first() {
                region = first.clone();
            }
        }

        games.push(VimmGame {
            id: id.clone(),
            name,
            region,
            version,
            languages: String::new(),
            rating: String::new(),
            size: String::new(),
            console_name,
            box_url: format!("https://dl.vimm.net/image.php?type=box&id={}", id),
            page_url: format!("https://vimm.net/vault/{}", id),
        });
    }
    Ok(games)
}

#[tauri::command]
async fn download_vimm_rom(
    app_handle: tauri::AppHandle,
    game_id: String,
    game_name: String,
    console: String,
) -> Result<String, String> {
    push_log("INFO", &format!("Téléchargement: {} ({})", game_name, console));
    let config = get_config();
    let roms_dir = std::path::PathBuf::from(&config.roms_directory);
    let dest_dir = roms_dir.join(&console);
    if !dest_dir.exists() {
        fs::create_dir_all(&dest_dir).map_err(|e| format!("Failed to create dir: {}", e))?;
    }

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .redirect(reqwest::redirect::Policy::limited(10))
        .connect_timeout(std::time::Duration::from_secs(30))
        .read_timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| format!("client build: {}", e))?;

    let _ = app_handle.emit("vimm-download-progress", serde_json::json!({
        "game_id": game_id,
        "status": "resolving",
        "progress": 0
    }));

    let page_url = format!("https://vimm.net/vault/{}", game_id);
    let page_resp = client.get(&page_url).send().await.map_err(|e| format!("page fetch: {}", e))?;
    let set_cookie = page_resp.headers().get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .map(|s| s.split(';').next().unwrap_or("").to_string())
        .collect::<Vec<_>>()
        .join("; ");
    let page_html = page_resp.text().await.map_err(|e| format!("page read: {}", e))?;

    let (media_id, dl_host) = {
        use scraper::{Html, Selector};
        let doc = Html::parse_document(&page_html);
        let media_sel = Selector::parse(r#"input[name="mediaId"]"#).map_err(|_| "selector error")?;
        let mid = doc.select(&media_sel).next()
            .and_then(|el| el.value().attr("value"))
            .ok_or_else(|| "Could not find mediaId on Vimm page".to_string())?
            .to_string();
        let form_sel = Selector::parse(r#"form#dl_form"#).map_err(|_| "selector error")?;
        let host = doc.select(&form_sel).next()
            .and_then(|el| el.value().attr("action"))
            .unwrap_or("//dl3.vimm.net/")
            .trim_start_matches("//")
            .trim_end_matches('/')
            .to_string();
        (mid, host)
    };

    println!("[Vimm] mediaId={} host={} for game {}", media_id, dl_host, game_name);

    let download_url = format!("https://{}/?mediaId={}", dl_host, media_id);
    let mut req = client.get(&download_url)
        .header("Referer", &page_url)
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8")
        .header("Accept-Language", "en-US,en;q=0.9")
        .header("Sec-Ch-Ua", r#""Not_A Brand";v="8", "Chromium";v="120", "Google Chrome";v="120""#)
        .header("Sec-Ch-Ua-Mobile", "?0")
        .header("Sec-Ch-Ua-Platform", "\"Windows\"")
        .header("Sec-Fetch-Dest", "document")
        .header("Sec-Fetch-Mode", "navigate")
        .header("Sec-Fetch-Site", "cross-site")
        .header("Sec-Fetch-User", "?1");
    if !set_cookie.is_empty() {
        req = req.header("Cookie", &set_cookie);
    }
    let response = req.send().await.map_err(|e| format!("download request: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Vimm download HTTP {}", response.status()));
    }

    let file_name = response.headers().get("content-disposition")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| {
            v.split("filename=").nth(1)
                .or_else(|| v.split("filename*=UTF-8''").nth(1))
                .map(|s| s.trim_matches('"').trim().to_string())
        })
        .unwrap_or_else(|| format!("{}.zip", game_name.replace(|c: char| !c.is_alphanumeric() && c != ' ', "").trim().replace(' ', "_")));

    let total_size = response.content_length().unwrap_or(0);
    let dest = dest_dir.join(&file_name);
    let mut file = fs::File::create(&dest).map_err(|e| e.to_string())?;
    let mut downloaded_bytes = 0u64;
    let mut last_emit = std::time::Instant::now();
    let start_time = std::time::Instant::now();
    let mut throttle_bytes = 0u64;
    let mut throttle_start = std::time::Instant::now();

    use std::io::Write;
    let mut stream = response;
    while let Some(chunk) = stream.chunk().await.map_err(|e| e.to_string())? {
        // Check for cancellation
        if cancelled_downloads().lock().map(|s| s.contains(&game_id)).unwrap_or(false) {
            drop(file);
            let _ = fs::remove_file(&dest);
            cancelled_downloads().lock().map(|mut s| s.remove(&game_id)).ok();
            push_log("INFO", &format!("Download Vimm annulé: {} ({})", game_name, game_id));
            let _ = app_handle.emit("vimm-download-progress", serde_json::json!({
                "game_id": game_id,
                "status": "cancelled",
                "progress": 0
            }));
            return Err("Download cancelled by user".to_string());
        }

        file.write_all(&chunk).map_err(|e| e.to_string())?;
        downloaded_bytes += chunk.len() as u64;

        let bw_limit = BANDWIDTH_LIMIT_KBPS.load(AtomicOrdering::Relaxed);
        if bw_limit > 0 {
            throttle_bytes += chunk.len() as u64;
            let limit_bps = bw_limit * 1024;
            let elapsed = throttle_start.elapsed().as_secs_f64();
            let expected_time = throttle_bytes as f64 / limit_bps as f64;
            if expected_time > elapsed {
                let sleep_ms = ((expected_time - elapsed) * 1000.0) as u64;
                tokio::time::sleep(std::time::Duration::from_millis(sleep_ms)).await;
            }
            if throttle_start.elapsed().as_secs() >= 2 {
                throttle_bytes = 0;
                throttle_start = std::time::Instant::now();
            }
        }

        if last_emit.elapsed().as_millis() >= 400 {
            let effective_total = if total_size > 0 { total_size } else { downloaded_bytes + 10_000_000 };
            let progress = (downloaded_bytes as f64 / effective_total as f64 * 100.0).min(99.0) as u32;
            let elapsed_sec = start_time.elapsed().as_secs_f64();
            let speed_bps = if elapsed_sec > 0.0 { downloaded_bytes as f64 / elapsed_sec } else { 0.0 };
            let eta = if speed_bps > 0.0 && total_size > downloaded_bytes {
                ((total_size - downloaded_bytes) as f64 / speed_bps) as u64
            } else { 0 };

            let _ = app_handle.emit("vimm-download-progress", serde_json::json!({
                "game_id": game_id,
                "status": "downloading",
                "progress": progress,
                "downloaded_bytes": downloaded_bytes,
                "total_bytes": total_size,
                "speed_bps": speed_bps as u64,
                "eta": eta
            }));
            last_emit = std::time::Instant::now();
        }
    }
    drop(file);

    let lower = file_name.to_lowercase();
    if lower.ends_with(".zip") || lower.ends_with(".7z") {
        let _ = app_handle.emit("vimm-download-progress", serde_json::json!({
            "game_id": game_id,
            "status": "extracting",
            "progress": 99
        }));
    }

    if lower.ends_with(".zip") {
        if let Ok(extracted) = extract_rom_zip(&dest, &dest_dir) {
            if !extracted.is_empty() { let _ = fs::remove_file(&dest); }
        }
    } else if lower.ends_with(".7z") {
        let dest_c = dest.clone();
        let dir_c = dest_dir.clone();
        match extract_7z(&dest_c, &dir_c) {
            Ok(()) => { let _ = fs::remove_file(&dest); }
            Err(_) => {}
        }
    }

    // Clean up Vimm's Lair.txt that gets bundled with downloads
    let vimm_txt = dest_dir.join("Vimm's Lair.txt");
    if vimm_txt.exists() { let _ = fs::remove_file(&vimm_txt); }

    let _ = app_handle.emit("vimm-download-progress", serde_json::json!({
        "game_id": game_id,
        "status": "done",
        "progress": 100
    }));

    push_log("INFO", &format!("Vimm download terminé: {} → {}", file_name, dest_dir.display()));
    Ok(format!("Downloaded {} to {}", file_name, dest_dir.display()))
}

// ============================================================
// RETROACHIEVEMENTS
// ============================================================

#[tauri::command]
fn save_ra_credentials(username: String, api_key: String) -> Result<(), String> {
    push_log("INFO", &format!("Sauvegarde credentials RA: user='{}'", username));
    let mut config = retroachievements::load_config();
    config.username = username.clone();
    config.api_key = api_key;
    retroachievements::save_config(&config)?;
    if !username.is_empty() {
        achievements::unlock_single("ra_connected");
    }
    Ok(())
}

#[tauri::command]
fn get_ra_credentials() -> retroachievements::RAConfig {
    retroachievements::load_config()
}

#[tauri::command]
async fn get_ra_game_progress(game_name: String, console: String) -> Result<retroachievements::RAGameInfo, String> {
    let config = retroachievements::load_config();
    if config.username.is_empty() || config.api_key.is_empty() {
        return Err("RetroAchievements credentials not configured".to_string());
    }

    let game_id = retroachievements::search_game(&game_name, &console, &config.api_key).await?
        .ok_or_else(|| format!("Game '{}' not found on RetroAchievements", game_name))?;

    retroachievements::get_game_progress(game_id, &config.username, &config.api_key).await
}

#[tauri::command]
async fn ra_login(username: String, password: String) -> Result<String, String> {
    push_log("INFO", &format!("Login RetroAchievements: user='{}'", username));
    let token = retroachievements::login_and_get_token(&username, &password).await?;
    // Save token to config
    let mut config = retroachievements::load_config();
    config.username = username;
    config.token = token.clone();
    retroachievements::save_config(&config)?;
    achievements::unlock_single("ra_connected");
    Ok(token)
}

#[tauri::command]
fn configure_ra_emulators() -> Result<Vec<String>, String> {
    push_log("INFO", "Configuration RA dans les émulateurs installés...");
    let config = retroachievements::load_config();
    if config.username.is_empty() || config.token.is_empty() {
        push_log("WARN", "RA: token non configuré, login requis");
        return Err("RetroAchievements token not configured. Use 'Login' first.".to_string());
    }
    let app_config = get_config();
    let configured = retroachievements::inject_ra_config_into_emulators(
        &app_config.emulators_directory,
        &config.username,
        &config.token,
    );
    if configured.is_empty() {
        push_log("WARN", "RA: aucun émulateur compatible trouvé");
        return Err("No compatible emulators found. Install RetroArch, DuckStation, PCSX2, Dolphin, or PPSSPP first.".to_string());
    }
    push_log("INFO", &format!("RA configuré dans: {:?}", configured));
    Ok(configured)
}

#[tauri::command]
async fn download_ra_cores(app_handle: tauri::AppHandle) -> Result<Vec<String>, String> {
    push_log("INFO", "Téléchargement des cores RetroArch...");
    let config = get_config();
    let ra_dir = PathBuf::from(&config.emulators_directory).join("retroarch");

    if !ra_dir.exists() {
        push_log("ERROR", "RetroArch non installé — impossible de télécharger les cores");
        return Err("RetroArch not installed. Please install RetroArch first.".to_string());
    }

    // Find the actual RetroArch exe to determine the real base dir (handles nested folders)
    let effective_ra_dir = find_executable(&ra_dir, "retroarch.exe")
        .and_then(|exe| exe.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| ra_dir.clone());

    let cores_dir = effective_ra_dir.join("cores");
    fs::create_dir_all(&cores_dir).map_err(|e| e.to_string())?;

    let cores = vec![
        ("mesen_libretro.dll", "NES"),
        ("mgba_libretro.dll", "GBA/GB/GBC"),
        ("snes9x_libretro.dll", "SNES"),
        ("mupen64plus_next_libretro.dll", "N64"),
        ("melonds_libretro.dll", "DS"),
        ("flycast_libretro.dll", "Dreamcast"),
    ];

    let client = reqwest::Client::builder()
        .user_agent("EmuWorld/0.2.0")
        .connect_timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    let mut downloaded = Vec::new();
    let base_url = "https://buildbot.libretro.com/nightly/windows/x86_64/latest/";

    for (core_file, label) in &cores {
        let dest = cores_dir.join(core_file);
        if dest.exists() {
            downloaded.push(format!("{} (already installed)", label));
            continue;
        }

        let zip_name = core_file.replace(".dll", ".dll.zip");
        let url = format!("{}{}", base_url, zip_name);
        println!("[RA Cores] Downloading {} from {}", core_file, url);

        use tauri::Emitter;
        let _ = app_handle.emit("ra-core-progress", serde_json::json!({
            "core": label,
            "status": "downloading"
        }));

        match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                match resp.bytes().await {
                    Ok(bytes) => {
                        let reader = std::io::Cursor::new(&bytes);
                        if let Ok(mut archive) = zip::ZipArchive::new(reader) {
                            for i in 0..archive.len() {
                                if let Ok(mut file) = archive.by_index(i) {
                                    let name = file.name().to_string();
                                    if name.ends_with(".dll") {
                                        let out_path = cores_dir.join(&name);
                                        if let Ok(mut out) = fs::File::create(&out_path) {
                                            let _ = std::io::copy(&mut file, &mut out);
                                            downloaded.push(format!("{} ✓", label));
                                            println!("[RA Cores] Installed {}", name);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => println!("[RA Cores] Failed to read {}: {}", core_file, e),
                }
            }
            Ok(resp) => println!("[RA Cores] HTTP {} for {}", resp.status(), core_file),
            Err(e) => println!("[RA Cores] Network error for {}: {}", core_file, e),
        }
    }

    if downloaded.is_empty() {
        return Err("Failed to download any cores.".to_string());
    }
    Ok(downloaded)
}

#[tauri::command]
async fn get_ra_completed_games() -> Result<Vec<retroachievements::RACompletedGame>, String> {
    let config = retroachievements::load_config();
    if config.username.is_empty() || config.api_key.is_empty() {
        return Err("RetroAchievements credentials not configured".to_string());
    }
    let games = retroachievements::get_completed_games(&config.username, &config.api_key).await?;
    let completed_count = games.iter()
        .filter(|g| g.num_awarded >= g.max_possible && g.max_possible > 0)
        .count();
    if completed_count >= 1 {
        achievements::unlock_single("ra_first_100");
    }
    if completed_count >= 5 {
        achievements::unlock_single("ra_five_100");
    }
    Ok(games)
}

// ============================================================
// GAME GUIDE — scrape Wikipedia summary + RA achievements
// ============================================================

#[derive(serde::Serialize)]
struct GameGuideData {
    summary: Option<String>,
    achievements: Vec<GuideAchievement>,
}

#[derive(serde::Serialize)]
struct GuideAchievement {
    title: String,
    description: String,
    points: u32,
    badge_url: String,
}

#[tauri::command]
async fn fetch_game_guide_data(game_name: String, console: String) -> Result<GameGuideData, String> {
    push_log("INFO", &format!("Fetch guide: '{}' ({})", game_name, console));
    let client = reqwest::Client::builder()
        .user_agent("EmuWorld/1.0.0")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    // 1. Wikipedia summary
    let clean = {
        let mut s = game_name.clone();
        // Remove file extension
        if let Some(idx) = s.rfind('.') {
            let ext = &s[idx..];
            if ext.len() <= 5 { s = s[..idx].to_string(); }
        }
        // Remove bracketed/parenthesized tags: (Europe), [v1.0], [!], (USA), etc.
        let re_brackets = regex::Regex::new(r"\s*[\[\(][^\]\)]*[\]\)]").unwrap();
        s = re_brackets.replace_all(&s, "").to_string();
        // Remove scene tags: .PROPER, .REPACK, v1.0
        let re_scene = regex::Regex::new(r"(?i)\.(proper|repack|v\d+[\.\d]*)").unwrap();
        s = re_scene.replace_all(&s, "").to_string();
        // Remove leading/trailing non-alphanumeric
        s.trim_matches(|c: char| !c.is_alphanumeric()).trim().to_string()
    };
    let wiki_url = format!(
        "https://en.wikipedia.org/api/rest_v1/page/summary/{}",
        urlencoding::encode(&clean.replace(' ', "_"))
    );
    // Also try with "(video_game)" suffix if first attempt fails
    let wiki_url_vg = format!(
        "https://en.wikipedia.org/api/rest_v1/page/summary/{}_(video_game)",
        urlencoding::encode(&clean.replace(' ', "_"))
    );
    let try_wiki = |url: &str| {
        let c = client.clone();
        let u = url.to_string();
        async move {
            match c.get(&u).send().await {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(json) = resp.json::<serde_json::Value>().await {
                        let extract = json["extract"].as_str().unwrap_or("").to_string();
                        let lower = extract.to_lowercase();
                        if extract.len() > 50 && (
                            lower.contains("video game") || lower.contains("game") ||
                            lower.contains("developed") || lower.contains("published") ||
                            lower.contains("console") || lower.contains("nintendo") ||
                            lower.contains("playstation") || lower.contains("sega") ||
                            lower.contains("capcom") || lower.contains("square") ||
                            lower.contains("player") || lower.contains("release")
                        ) {
                            Some(extract)
                        } else { None }
                    } else { None }
                }
                _ => None,
            }
        }
    };
    let summary = match try_wiki(&wiki_url).await {
        Some(s) => Some(s),
        None => try_wiki(&wiki_url_vg).await,
    };

    // 2. RetroAchievements
    let ra_config = retroachievements::load_config();
    let mut achievements = Vec::new();
    if !ra_config.api_key.is_empty() {
        if let Ok(Some(game_id)) = retroachievements::search_game(&game_name, &console, &ra_config.api_key).await {
            let url = format!(
                "https://retroachievements.org/API/API_GetGameInfoAndUserProgress.php?g={}&u={}&y={}",
                game_id, ra_config.username, ra_config.api_key
            );
            if let Ok(resp) = client.get(&url).send().await {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if let Some(achs) = json["Achievements"].as_object() {
                        for (_, ach) in achs {
                            let title = ach["Title"].as_str().unwrap_or("").to_string();
                            let description = ach["Description"].as_str().unwrap_or("").to_string();
                            let points = ach["Points"].as_u64().unwrap_or(0) as u32;
                            let badge_id = ach["BadgeName"].as_str().unwrap_or("");
                            let badge_url = if !badge_id.is_empty() {
                                format!("https://media.retroachievements.org/Badge/{}.png", badge_id)
                            } else { String::new() };
                            achievements.push(GuideAchievement { title, description, points, badge_url });
                        }
                    }
                }
            }
        }
    }

    Ok(GameGuideData { summary, achievements })
}

// ============================================================
// CLOUD BACKUP — Backblaze B2 save sync
// ============================================================

#[tauri::command]
fn save_b2_config(key_id: String, app_key: String, bucket_id: String, bucket_name: String) -> Result<(), String> {
    cloud_backup::save_config(&cloud_backup::B2Config { key_id, app_key, bucket_id, bucket_name })
}

#[tauri::command]
fn get_b2_config() -> cloud_backup::B2Config {
    cloud_backup::load_config()
}

#[tauri::command]
fn scan_local_saves() -> Vec<cloud_backup::SaveEntry> {
    let config = get_config();
    cloud_backup::scan_saves(&config.emulators_directory)
}

#[tauri::command]
async fn backup_saves_to_cloud() -> Result<String, String> {
    push_log("INFO", "Cloud backup: démarrage upload saves...");
    let app_config = get_config();
    let b2_config = cloud_backup::load_config();
    if b2_config.key_id.is_empty() || b2_config.app_key.is_empty() {
        push_log("WARN", "Cloud backup: B2 non configuré");
        return Err("Backblaze B2 not configured.".to_string());
    }

    // Create zip
    let zip_path = cloud_backup::create_backup_zip(&app_config.emulators_directory)?;
    let zip_data = fs::read(&zip_path).map_err(|e| e.to_string())?;
    let zip_name = zip_path.file_name().unwrap_or_default().to_string_lossy().to_string();

    // Auth + upload
    let (token, api_url) = cloud_backup::b2_authorize(&b2_config.key_id, &b2_config.app_key).await?;
    let b2_file_name = format!("emuworld/{}", zip_name);
    cloud_backup::b2_upload_file(&api_url, &token, &b2_config.bucket_id, &b2_file_name, zip_data).await?;

    // Clean up local zip
    let _ = fs::remove_file(&zip_path);

    Ok(format!("Backup uploaded: {}", b2_file_name))
}

#[tauri::command]
async fn list_cloud_backups() -> Result<Vec<cloud_backup::CloudFile>, String> {
    let b2_config = cloud_backup::load_config();
    if b2_config.key_id.is_empty() || b2_config.app_key.is_empty() {
        return Err("Backblaze B2 not configured.".to_string());
    }

    let (token, api_url) = cloud_backup::b2_authorize(&b2_config.key_id, &b2_config.app_key).await?;
    cloud_backup::b2_list_files(&api_url, &token, &b2_config.bucket_id, "emuworld/").await
}

#[tauri::command]
async fn restore_cloud_backup(file_id: String) -> Result<String, String> {
    push_log("INFO", &format!("Cloud backup: restauration fichier {}", file_id));
    let app_config = get_config();
    let b2_config = cloud_backup::load_config();
    if b2_config.key_id.is_empty() || b2_config.app_key.is_empty() {
        return Err("Backblaze B2 not configured.".to_string());
    }

    let (token, api_url) = cloud_backup::b2_authorize(&b2_config.key_id, &b2_config.app_key).await?;
    let data = cloud_backup::b2_download_file(&api_url, &token, &file_id).await?;

    // Write to temp file and restore
    let tmp_dir = emuworld_base_dir().join("backups");
    fs::create_dir_all(&tmp_dir).map_err(|e| e.to_string())?;
    let tmp_path = tmp_dir.join("restore_tmp.zip");
    fs::write(&tmp_path, &data).map_err(|e| e.to_string())?;

    let restored = cloud_backup::restore_backup_zip(&tmp_path.to_string_lossy(), &app_config.emulators_directory)?;
    let _ = fs::remove_file(&tmp_path);

    Ok(format!("{} files restored", restored))
}

#[tauri::command]
async fn delete_cloud_backup(file_id: String, file_name: String) -> Result<String, String> {
    let b2_config = cloud_backup::load_config();
    if b2_config.key_id.is_empty() || b2_config.app_key.is_empty() {
        return Err("Backblaze B2 not configured.".to_string());
    }

    let (token, api_url) = cloud_backup::b2_authorize(&b2_config.key_id, &b2_config.app_key).await?;
    cloud_backup::b2_delete_file(&api_url, &token, &file_id, &file_name).await?;

    Ok(format!("Deleted: {}", file_name))
}

// ============================================================
// PLAYTIME — per-game session tracking, favorites, aggregate stats.
// Data lives in %APPDATA%/Local/EmuWorld/playtime.json.
// ============================================================

#[tauri::command]
fn get_playtime() -> playtime::PlaytimeStore {
    playtime::load()
}

#[tauri::command]
fn toggle_favorite(console: String, name: String) -> Result<bool, String> {
    playtime::toggle_favorite(&console, &name)
}

#[tauri::command]
fn set_game_rating(console: String, name: String, rating: u8) -> Result<(), String> {
    playtime::set_rating(&console, &name, rating)
}

#[tauri::command]
fn set_game_notes(console: String, name: String, notes: String) -> Result<(), String> {
    playtime::set_notes(&console, &name, &notes)
}

#[tauri::command]
fn create_collection(name: String) -> Result<Vec<playtime::GameCollection>, String> {
    playtime::create_collection(&name)
}

#[tauri::command]
fn delete_collection(name: String) -> Result<Vec<playtime::GameCollection>, String> {
    playtime::delete_collection(&name)
}

#[tauri::command]
fn rename_collection(old_name: String, new_name: String) -> Result<Vec<playtime::GameCollection>, String> {
    playtime::rename_collection(&old_name, &new_name)
}

#[tauri::command]
fn add_to_collection(collection_name: String, game_key: String) -> Result<(), String> {
    playtime::add_to_collection(&collection_name, &game_key)
}

#[tauri::command]
fn remove_from_collection(collection_name: String, game_key: String) -> Result<(), String> {
    playtime::remove_from_collection(&collection_name, &game_key)
}

#[tauri::command]
fn get_profile_stats() -> playtime::ProfileStats {
    playtime::compute_stats()
}

/// Wipe the local playtime store. Called on sign-out so the next user
/// who signs in on this machine starts from a blank file instead of
/// inheriting the previous user's history.
#[tauri::command]
fn clear_playtime() -> Result<(), String> {
    playtime::clear()
}

/// Replace the local store with data coming from Supabase. Called right
/// after sign-in so the local file matches what the cloud has for this
/// user, before any further sync runs.
#[tauri::command]
fn overwrite_playtime(store: playtime::PlaytimeStore) -> Result<(), String> {
    playtime::overwrite(store)
}

#[tauri::command]
fn clear_cover_cache() -> Result<(), String> {
    push_log("INFO", "Nettoyage du cache des covers...");
    let config = get_config();
    let covers_dir = PathBuf::from(&config.covers_directory);
    if covers_dir.exists() {
        std::fs::remove_dir_all(&covers_dir).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(&covers_dir).map_err(|e| e.to_string())?;
    }
    push_log("INFO", "Cache covers nettoyé");
    Ok(())
}

// ============================================================
// ACHIEVEMENTS — in-app progression badges
// ============================================================

#[tauri::command]
fn get_achievements() -> Vec<achievements::Achievement> {
    achievements::get_all_with_status()
}

#[tauri::command]
fn get_achievement_rank() -> serde_json::Value {
    let count = achievements::unlocked_count();
    serde_json::json!({
        "count": count,
        "total": achievements::all_achievements().len(),
        "rank": achievements::rank_label(count),
        "icon": achievements::rank_icon(count),
    })
}

#[tauri::command]
fn check_achievements(
    library_count: u32,
    emulators_installed: u32,
    has_downloaded: bool,
) -> Vec<achievements::Achievement> {
    let stats = playtime::compute_stats();
    let consoles_played = {
        let store = playtime::load();
        let mut consoles: std::collections::HashSet<String> = std::collections::HashSet::new();
        for entry in store.games.values() {
            if entry.seconds > 0 {
                consoles.insert(entry.console.clone());
            }
        }
        consoles.len() as u32
    };
    achievements::check_and_unlock(
        library_count,
        stats.total_seconds,
        stats.total_launches,
        consoles_played,
        stats.favorite_count,
        emulators_installed,
        stats.streak_days,
        has_downloaded,
    )
}

#[tauri::command]
fn unlock_achievement(id: String) -> Option<achievements::Achievement> {
    achievements::unlock_single(&id)
}

#[tauri::command]
fn clear_achievements() -> Result<(), String> {
    achievements::clear()
}

#[tauri::command]
fn overwrite_achievements(unlocked: std::collections::HashMap<String, String>) -> Result<(), String> {
    achievements::overwrite(unlocked)
}

#[derive(Debug, Serialize, Deserialize)]
struct FullExport {
    config: AppConfig,
    playtime: playtime::PlaytimeStore,
}

#[tauri::command]
fn set_current_playing(state: tauri::State<'_, Mutex<CurrentPlayingState>>, game_name: Option<String>, console: Option<String>) {
    let mut s = state.lock().unwrap();
    s.game_name = game_name;
    s.console = console;
}

#[tauri::command]
fn get_current_playing(state: tauri::State<'_, Mutex<CurrentPlayingState>>) -> (Option<String>, Option<String>) {
    let s = state.lock().unwrap();
    (s.game_name.clone(), s.console.clone())
}

#[tauri::command]
fn migrate_covers_to_webp() -> Result<String, String> {
    let config = get_config();
    let covers_dir = PathBuf::from(&config.covers_directory);
    if !covers_dir.exists() { return Ok("No covers directory".to_string()); }

    let mut converted = 0u32;
    let mut saved_bytes: u64 = 0;

    for entry in walkdir::WalkDir::new(&covers_dir).max_depth(3) {
        if let Ok(e) = entry {
            if e.file_type().is_file() {
                let path = e.path().to_path_buf();
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                if ext == "png" || ext == "jpg" || ext == "jpeg" {
                    let original_size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                    if let Ok(data) = fs::read(&path) {
                        if let Ok(img) = ImageReader::new(Cursor::new(&data)).with_guessed_format().and_then(|r| Ok(r.decode())) {
                            if let Ok(img) = img {
                                if let Ok(encoder) = webp::Encoder::from_image(&img) {
                                    let webp_data = encoder.encode(85.0);
                                    let webp_path = path.with_extension("webp");
                                    if fs::write(&webp_path, &*webp_data).is_ok() {
                                        let new_size = webp_data.len() as u64;
                                        if new_size < original_size {
                                            saved_bytes += original_size - new_size;
                                            let _ = fs::remove_file(&path);
                                            converted += 1;
                                        } else {
                                            let _ = fs::remove_file(&webp_path);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let saved_mb = saved_bytes as f64 / 1024.0 / 1024.0;
    Ok(format!("{} covers converties, {:.1} MB économisés", converted, saved_mb))
}

#[tauri::command]
fn take_screenshot(game_name: String, console: String) -> Result<String, String> {
    push_log("INFO", &format!("Screenshot: '{}' ({})", game_name, console));
    use std::process::Command as Cmd;

    let mut base = emuworld_base_dir();
    base.push("screenshots");
    let safe_console = console.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|', '$', '(', ')', '`', '&', ';', '\''], "_");
    let safe_name = game_name.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|', '$', '(', ')', '`', '&', ';', '\''], "_");
    base.push(&safe_console);
    base.push(&safe_name);
    let _ = fs::create_dir_all(&base);

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let filename = format!("screenshot_{}.png", timestamp);
    let filepath = base.join(&filename);
    let path_str = filepath.to_string_lossy().to_string();

    let safe_path = path_str.replace('\\', "/").replace('\'', "_");
    let ps_script = format!(
        "Add-Type -AssemblyName System.Windows.Forms; Add-Type -AssemblyName System.Drawing; $b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds; $bmp = New-Object System.Drawing.Bitmap($b.Width, $b.Height); $g = [System.Drawing.Graphics]::FromImage($bmp); $g.CopyFromScreen($b.Location, [System.Drawing.Point]::Empty, $b.Size); $bmp.Save('{}'); $g.Dispose(); $bmp.Dispose()",
        safe_path
    );

    let mut ps_cmd = Cmd::new(r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe");
    ps_cmd.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &ps_script]);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        ps_cmd.creation_flags(0x08000000);
    }
    let output = ps_cmd.output()
        .map_err(|e| format!("Failed to run screenshot: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Screenshot failed: {}", stderr));
    }

    Ok(filepath.to_string_lossy().to_string())
}

#[derive(Serialize, Clone)]
struct AllScreenshotsGroup {
    game_name: String,
    console: String,
    screenshots: Vec<ScreenshotEntry>,
}

#[tauri::command]
fn get_all_screenshots() -> Vec<AllScreenshotsGroup> {
    let mut base = emuworld_base_dir();
    base.push("screenshots");
    let mut results: Vec<AllScreenshotsGroup> = Vec::new();
    if !base.exists() { return results; }
    if let Ok(consoles) = fs::read_dir(&base) {
        for console_entry in consoles.flatten() {
            if !console_entry.path().is_dir() { continue; }
            let console_name = console_entry.file_name().to_string_lossy().to_string();
            if let Ok(games) = fs::read_dir(console_entry.path()) {
                for game_entry in games.flatten() {
                    if !game_entry.path().is_dir() { continue; }
                    let game_name = game_entry.file_name().to_string_lossy().to_string();
                    let mut screenshots: Vec<ScreenshotEntry> = Vec::new();
                    if let Ok(files) = fs::read_dir(game_entry.path()) {
                        for file in files.flatten() {
                            if file.path().extension().map(|e| e == "png").unwrap_or(false) {
                                if let Ok(bytes) = fs::read(file.path()) {
                                    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                                    screenshots.push(ScreenshotEntry {
                                        path: file.path().to_string_lossy().to_string(),
                                        data_url: format!("data:image/png;base64,{}", b64),
                                    });
                                }
                            }
                        }
                    }
                    if !screenshots.is_empty() {
                        screenshots.sort_by(|a, b| a.path.cmp(&b.path));
                        results.push(AllScreenshotsGroup { game_name, console: console_name.clone(), screenshots });
                    }
                }
            }
        }
    }
    results
}

#[derive(Serialize, Clone)]
struct ScreenshotEntry {
    path: String,
    data_url: String,
}

#[tauri::command]
fn get_screenshots(game_name: String, console: String) -> Vec<ScreenshotEntry> {
    let mut base = emuworld_base_dir();
    base.push("screenshots");
    let safe_console = console.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
    let safe_name = game_name.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
    base.push(&safe_console);
    base.push(&safe_name);

    if !base.exists() { return vec![]; }

    let mut files: Vec<_> = fs::read_dir(&base)
        .map(|rd| rd.filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|x| x == "png").unwrap_or(false))
            .map(|e| e.path())
            .collect())
        .unwrap_or_default();
    files.sort();
    files.reverse();
    files.iter().filter_map(|p| {
        let bytes = fs::read(p).ok()?;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
        Some(ScreenshotEntry {
            path: p.to_string_lossy().to_string(),
            data_url: format!("data:image/png;base64,{}", b64),
        })
    }).collect()
}

#[tauri::command]
fn delete_screenshot(path: String) -> Result<(), String> {
    let screenshots_dir = emuworld_base_dir().join("screenshots");
    let canonical = fs::canonicalize(&path).map_err(|e| format!("Invalid path: {}", e))?;
    let canonical_base = fs::canonicalize(&screenshots_dir).unwrap_or(screenshots_dir);
    if !canonical.starts_with(&canonical_base) {
        return Err("Path must be within the screenshots directory".to_string());
    }
    fs::remove_file(&path).map_err(|e| format!("Cannot delete screenshot: {}", e))
}


#[tauri::command]
async fn watch_roms_directory(app: tauri::AppHandle) -> Result<(), String> {
    use notify::{Watcher, RecursiveMode, Event, EventKind};
    use std::sync::mpsc;

    let config = get_config();
    let dir = config.roms_directory.clone();
    if dir.is_empty() { return Err("No ROMs directory configured".into()); }
    let path = PathBuf::from(&dir);
    if !path.exists() { return Err("ROMs directory does not exist".into()); }

    let handle = app.clone();
    std::thread::spawn(move || {
        let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
        let mut watcher = match notify::recommended_watcher(tx) {
            Ok(w) => w,
            Err(_) => return,
        };
        if watcher.watch(&path, RecursiveMode::Recursive).is_err() { return; }

        let catalog = emulators::get_catalog();
        let supported_exts: std::collections::HashSet<String> = catalog.iter()
            .flat_map(|e| e.supported_extensions.iter().cloned())
            .collect();

        loop {
            match rx.recv() {
                Ok(Ok(event)) => {
                    if matches!(event.kind, EventKind::Create(_)) {
                        let new_roms: Vec<String> = event.paths.iter()
                            .filter(|p| {
                                p.extension()
                                    .map(|e| supported_exts.contains(&e.to_string_lossy().to_lowercase()))
                                    .unwrap_or(false)
                            })
                            .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()))
                            .collect();
                        if !new_roms.is_empty() {
                            let _ = handle.emit("roms-detected", &new_roms);
                        }
                    }
                }
                Ok(Err(_)) => {}
                Err(_) => break,
            }
        }
    });
    Ok(())
}


#[tauri::command]
async fn launch_netplay(emulator_id: String, rom_path: String, is_host: bool, lobby_id: String) -> Result<String, String> {
    let catalog = emulators::get_catalog();
    let emu = catalog.iter().find(|e| e.id == emulator_id).ok_or("Emulator not found")?;
    let config = get_config();
    let final_path = rom_path.replace(r"\\?\", "").replace("/", "\\");

    let mut cmd = if emu.id == "ppsspp" {
        // PPSSPP adhoc multiplayer — configure adhoc server then launch game
        let ppsspp_dir = PathBuf::from(&config.emulators_directory).join("ppsspp");
        let ppsspp_exe = find_executable(&ppsspp_dir, "PPSSPPWindows64.exe")
            .ok_or("PPSSPP not installed")?;
        let exe_dir = ppsspp_exe.parent().unwrap_or(&ppsspp_dir);
        let memstick = exe_dir.join("memstick");
        let psp_dir = memstick.join("PSP").join("SYSTEM");
        fs::create_dir_all(&psp_dir).ok();
        let ini_path = psp_dir.join("ppsspp.ini");
        let mut ini = fs::read_to_string(&ini_path).unwrap_or_default();
        // Enable networking + set adhoc server
        let net_settings = [
            ("Enable", "True"),
            ("EnableWlan", "True"),
            ("EnableAdhocServer", "False"),
            ("proAdhocServer", "myneighborsushicat.com"),
            ("PortOffset", "0"),
            ("MacAddress", if is_host { "01:02:03:04:05:06" } else { "01:02:03:04:05:07" }),
        ];
        for (key, val) in net_settings {
            let pattern = format!("{} = ", key);
            if let Some(pos) = ini.find(&pattern) {
                let end = ini[pos..].find('\n').map(|p| pos + p).unwrap_or(ini.len());
                ini.replace_range(pos..end, &format!("{} = {}", key, val));
            } else {
                if !ini.contains("[Network]") { ini.push_str("\n[Network]\n"); }
                ini.push_str(&format!("{} = {}\n", key, val));
            }
        }
        let _ = fs::write(&ini_path, &ini);

        let mut c = Command::new(&ppsspp_exe);
        c.current_dir(exe_dir);
        c.arg(&final_path);
        c.arg("--fullscreen");
        c
    } else if emu.id == "dolphin" {
        // Dolphin netplay via traversal server
        let dolphin_dir = PathBuf::from(&config.emulators_directory).join("dolphin");
        let dolphin_exe = find_executable(&dolphin_dir, "Dolphin.exe")
            .ok_or("Dolphin not installed")?;

        // Configure Dolphin to use traversal mode
        let dolphin_cfg_dir = dirs::config_dir().unwrap_or_default().join("Dolphin Emulator").join("Config");
        let dolphin_ini = dolphin_cfg_dir.join("Dolphin.ini");
        if dolphin_ini.exists() {
            let mut cfg = std::fs::read_to_string(&dolphin_ini).unwrap_or_default();
            if cfg.contains("[NetPlay]") {
                let re = regex::Regex::new(r"TraversalChoice = \w+").unwrap();
                cfg = re.replace(&cfg, "TraversalChoice = traversal").to_string();
            } else {
                cfg.push_str("\n[NetPlay]\nTraversalChoice = traversal\n");
            }
            let _ = std::fs::write(&dolphin_ini, &cfg);
        }

        let mut c = Command::new(&dolphin_exe);
        c.current_dir(dolphin_exe.parent().unwrap_or(&dolphin_dir));
        c.arg("-e").arg(&final_path);
        if is_host {
            c.arg("--netplay=host");
        } else {
            c.arg("--netplay=connect");
        }
        c
    } else {
        // RetroArch netplay via MITM relay
        let ra_dir = PathBuf::from(&config.emulators_directory).join("retroarch");
        let ra_exe = find_executable(&ra_dir, "retroarch.exe")
            .ok_or("RetroArch not installed — required for netplay")?;
        let effective_ra_dir = ra_exe.parent().unwrap_or(&ra_dir).to_path_buf();

        let core_name = emu.core_name.as_deref()
            .or_else(|| retroachievements::retroarch_core_for_emulator(&emu.id))
            .ok_or("No RetroArch core available for this emulator")?;
        let cores_dir = effective_ra_dir.join("cores");
        let core_path = cores_dir.join(core_name);
        if !core_path.exists() {
            return Err(format!("Core '{}' not found in RetroArch cores/", core_name));
        }

        let mut c = Command::new(&ra_exe);
        c.current_dir(&effective_ra_dir);
        c.arg("-L").arg(&core_path);
        if is_host {
            c.arg("--host");
            c.arg("--mitm=netplay.libretro.com");
            c.arg("--nick=EmuWorld-Host");
        } else {
            c.arg("--connect");
            c.arg("netplay.libretro.com");
            c.arg("--mitm=netplay.libretro.com");
            c.arg("--nick=EmuWorld-Client");
        }
        c.arg(&final_path);
        c
    };

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }

    println!("[Netplay] Launching {} as {} for lobby {}", emu.id, if is_host { "HOST" } else { "CLIENT" }, lobby_id);
    push_log("INFO", &format!("Netplay {} — {}", if is_host { "Host" } else { "Client" }, emu.name));

    cmd.spawn().map_err(|e| format!("Failed to launch netplay: {}", e))?;
    Ok(format!("Netplay {} started", if is_host { "host" } else { "client" }))
}

#[tauri::command]
fn get_logs() -> Vec<String> {
    app_logs().lock().map(|l| l.clone()).unwrap_or_default()
}

#[tauri::command]
fn clear_logs() {
    if let Ok(mut logs) = app_logs().lock() { logs.clear(); }
}

#[tauri::command]
fn get_log_file_path() -> String {
    log_file_path().to_string_lossy().to_string()
}

#[tauri::command]
fn get_logs_directory() -> String {
    emuworld_base_dir().join("logs").to_string_lossy().to_string()
}

#[tauri::command]
fn open_path(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let path_lower = path.to_lowercase();
        if path_lower.starts_with("shell:")
            || path_lower.starts_with("\\\\")
            || path_lower.starts_with("http")
            || path_lower.starts_with("ftp")
        {
            return Err("Type de chemin non autorisé".to_string());
        }
        let p = PathBuf::from(&path);
        if !p.is_dir() {
            return Err("Ce chemin n'est pas un dossier valide".to_string());
        }
        Command::new("explorer").arg(&path).spawn().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct RomHealthIssue {
    name: String,
    path: String,
    console: String,
    issue: String,
    size: u64,
}

#[tauri::command]
fn check_roms_health() -> Vec<RomHealthIssue> {
    push_log("INFO", "Vérification intégrité des ROMs...");
    let config = get_config();
    let roms = scan_roms(config.roms_directory.clone());
    let mut issues = Vec::new();

    for rom in &roms {
        let path = PathBuf::from(&rom.path);
        if !path.exists() {
            issues.push(RomHealthIssue {
                name: rom.name.clone(),
                path: rom.path.clone(),
                console: rom.console.clone(),
                issue: "Fichier introuvable".to_string(),
                size: 0,
            });
            continue;
        }
        let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        if size == 0 {
            issues.push(RomHealthIssue {
                name: rom.name.clone(),
                path: rom.path.clone(),
                console: rom.console.clone(),
                issue: "Fichier vide (0 octets)".to_string(),
                size,
            });
        } else if size < 100 && rom.extension != "a26" {
            issues.push(RomHealthIssue {
                name: rom.name.clone(),
                path: rom.path.clone(),
                console: rom.console.clone(),
                issue: format!("Fichier suspect ({} octets)", size),
                size,
            });
        } else if rom.extension == "zip" || rom.extension == "7z" {
            if rom.extension == "zip" {
                if let Ok(f) = std::fs::File::open(&path) {
                    if zip::ZipArchive::new(f).is_err() {
                        issues.push(RomHealthIssue {
                            name: rom.name.clone(),
                            path: rom.path.clone(),
                            console: rom.console.clone(),
                            issue: "Archive ZIP corrompue".to_string(),
                            size,
                        });
                    }
                }
            }
        }
    }
    push_log("INFO", &format!("ROM health check: {} problème(s) détecté(s) sur {} ROMs", issues.len(), roms.len()));
    issues
}

#[tauri::command]
fn delete_unhealthy_roms(paths: Vec<String>) -> Result<String, String> {
    let config = get_config();
    let roms_root = PathBuf::from(&config.roms_directory);
    let canonical_root = roms_root.canonicalize().unwrap_or_else(|_| roms_root.clone());
    let mut deleted = 0;
    for p in &paths {
        let path = PathBuf::from(p);
        if path.exists() {
            if let Ok(canonical) = path.canonicalize() {
                if !canonical.starts_with(&canonical_root) {
                    continue;
                }
            }
            fs::remove_file(&path).ok();
            deleted += 1;
        }
    }
    Ok(format!("{} fichier(s) supprimé(s)", deleted))
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct EmulatorUpdate {
    id: String,
    name: String,
    current_version: String,
    latest_version: String,
    download_url: String,
}

#[tauri::command]
async fn check_emulator_updates() -> Result<Vec<EmulatorUpdate>, String> {
    push_log("INFO", "Vérification des mises à jour émulateurs...");
    let installed = get_installed_emulators();
    let catalog = emulators::get_catalog();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let mut updates = Vec::new();

    for emu in &catalog {
        if !installed.contains(&emu.id) { continue; }
        let source = match emulators::update_info(&emu.id) {
            Some(s) => s,
            None => continue,
        };

        match source {
            emulators::UpdateSource::GitHub(repo, catalog_ver) => {
                let current = get_installed_version(&emu.id).unwrap_or_else(|| catalog_ver.to_string());
                let current = current.as_str();
                let api_url = format!("https://api.github.com/repos/{}/releases/latest", repo);
                if let Ok(resp) = client.get(&api_url).header("User-Agent", "EmuWorld/2.0").send().await {
                    if let Ok(json) = resp.json::<serde_json::Value>().await {
                        if let Some(tag) = json["tag_name"].as_str() {
                            let latest = tag.trim_start_matches('v').trim_start_matches("release-");
                            if latest != current && !latest.is_empty() {
                                let dl_url = json["assets"].as_array()
                                    .and_then(|assets| assets.iter().find(|a| {
                                        let name = a["name"].as_str().unwrap_or("").to_lowercase();
                                        name.contains("win") && (name.contains("x64") || name.contains("x86_64")) && !name.contains("pdb") && !name.contains("dbg")
                                    }))
                                    .and_then(|a| a["browser_download_url"].as_str())
                                    .unwrap_or("")
                                    .to_string();
                                updates.push(EmulatorUpdate { id: emu.id.clone(), name: emu.name.clone(), current_version: current.to_string(), latest_version: latest.to_string(), download_url: dl_url });
                            }
                        }
                    }
                }
            }
            emulators::UpdateSource::Forgejo(api_url, catalog_ver) => {
                let current = get_installed_version(&emu.id).unwrap_or_else(|| catalog_ver.to_string());
                if let Ok(resp) = client.get(api_url).header("User-Agent", "EmuWorld/2.0").send().await {
                    if let Ok(json) = resp.json::<serde_json::Value>().await {
                        let releases = if json.is_array() { json.as_array().cloned().unwrap_or_default() } else { vec![json] };
                        if let Some(release) = releases.first() {
                            if let Some(tag) = release["tag_name"].as_str() {
                                if tag != current {
                                    let dl_url = release["assets"].as_array()
                                        .and_then(|assets| assets.iter().find(|a| {
                                            let name = a["name"].as_str().unwrap_or("").to_lowercase();
                                            name.contains("win") && name.contains("x64") && !name.contains("pdb")
                                        }))
                                        .and_then(|a| a["browser_download_url"].as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    updates.push(EmulatorUpdate { id: emu.id.clone(), name: emu.name.clone(), current_version: current.to_string(), latest_version: tag.to_string(), download_url: dl_url });
                                }
                            }
                        }
                    }
                }
            }
            emulators::UpdateSource::DolphinEmu(catalog_ver) => {
                let current = get_installed_version(&emu.id).unwrap_or_else(|| catalog_ver.to_string());
                if let Ok(resp) = client.get("https://dolphin-emu.org/download/list/master/1/").header("User-Agent", "EmuWorld/2.0").send().await {
                    if let Ok(text) = resp.text().await {
                        if let Some(cap) = regex::Regex::new(r"Dolphin (\d+)").ok().and_then(|re| re.captures(&text)) {
                            let latest = cap.get(1).map(|m| m.as_str()).unwrap_or("");
                            if !latest.is_empty() && latest != current {
                                let dl_url = format!("https://dl.dolphin-emu.org/releases/{}/dolphin-{}-x64.7z", latest, latest);
                                updates.push(EmulatorUpdate { id: emu.id.clone(), name: emu.name.clone(), current_version: current.to_string(), latest_version: latest.to_string(), download_url: dl_url });
                            }
                        }
                    }
                }
            }
        }
    }
    push_log("INFO", &format!("Update check: {} mise(s) à jour disponible(s)", updates.len()));
    Ok(updates)
}

#[tauri::command]
fn save_emulator_version(emulator_id: String, version: String) -> Result<(), String> {
    let path = emuworld_base_dir().join("emulator_versions.json");
    let mut versions: std::collections::HashMap<String, String> = if let Ok(data) = fs::read_to_string(&path) {
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        std::collections::HashMap::new()
    };
    versions.insert(emulator_id, version);
    fs::write(&path, serde_json::to_string(&versions).unwrap_or_default()).map_err(|e| e.to_string())
}

fn get_installed_version(id: &str) -> Option<String> {
    let path = emuworld_base_dir().join("emulator_versions.json");
    let data = fs::read_to_string(&path).ok()?;
    let versions: std::collections::HashMap<String, String> = serde_json::from_str(&data).ok()?;
    versions.get(id).cloned()
}

#[tauri::command]
fn get_cover_url(game_name: String, console: String) -> Option<String> {
    let key = format!("{}::{}", console, game_name);
    let stored = cover_urls().lock().ok().and_then(|map| map.get(&key).cloned());
    // Only return URLs that Discord can display (libretro thumbnails or tinfoil)
    if let Some(ref url) = stored {
        if url.contains("thumbnails.libretro.com") || url.contains("tinfoil.media") {
            return stored;
        }
    }
    // Fallback: try to construct a libretro URL from the cover filename
    let config = get_config();
    let safe_console = console.replace("/", "-");
    let covers_dir = PathBuf::from(&config.covers_directory).join(&safe_console);
    let norm_target = game_name.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect::<String>();
    if let Ok(entries) = std::fs::read_dir(&covers_dir) {
        for entry in entries.flatten() {
            if let Some(fname) = entry.file_name().to_str() {
                let lower = fname.to_lowercase();
                if lower.ends_with(".webp") || lower.ends_with(".png") {
                    let name_no_ext = lower.rsplit_once('.').map(|(n,_)| n).unwrap_or(&lower);
                    let norm = name_no_ext.chars().filter(|c| c.is_alphanumeric()).collect::<String>();
                    if norm == norm_target || norm.contains(&norm_target) || norm_target.contains(&norm) {
                        let libretro_name = fname.rsplit_once('.').map(|(n,_)| n).unwrap_or(fname);
                        let systems: &[&str] = match console.as_str() {
                            "Wii U" => &["Nintendo - Wii U"],
                            "GameCube / Wii" | "Wii" => &["Nintendo - Wii"],
                            "Nintendo Switch" => &["Nintendo - Nintendo Switch"],
                            "Nintendo 3DS" => &["Nintendo - Nintendo 3DS"],
                            _ => &[],
                        };
                        if let Some(system) = systems.first() {
                            let url = format!("https://thumbnails.libretro.com/{}/Named_Boxarts/{}.png", urlencoding::encode(system), urlencoding::encode(libretro_name));
                            return Some(url);
                        }
                    }
                }
            }
        }
    }
    stored
}

static OAUTH_PORT: OnceLock<Mutex<u16>> = OnceLock::new();

fn oauth_port() -> &'static Mutex<u16> {
    OAUTH_PORT.get_or_init(|| Mutex::new(0))
}

#[tauri::command]
async fn start_oauth_server(app_handle: tauri::AppHandle) -> Result<u16, String> {
    use std::net::TcpListener;
    use tauri::Emitter;

    // Try to bind to a known fixed port first (so the callback page can reach us reliably)
    // Fall back to nearby ports if occupied
    let fixed_ports = [17643, 17644, 17645, 17646, 17647];
    let listener = fixed_ports.iter()
        .find_map(|p| TcpListener::bind(format!("127.0.0.1:{}", p)).ok())
        .or_else(|| TcpListener::bind("127.0.0.1:0").ok())
        .ok_or_else(|| "Failed to bind OAuth server".to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    *oauth_port().lock().unwrap() = port;
    push_log("INFO", &format!("OAuth HTTP server started on port {}", port));

    // Spawn a thread that accepts one connection, reads the request, extracts tokens, emits event
    let handle = app_handle.clone();
    std::thread::spawn(move || {
        // Accept connections for up to 120s
        listener.set_nonblocking(false).ok();
        let _ = listener.set_ttl(120);

        loop {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    use std::io::{Read, Write as IoWrite};
                    let mut buf = [0u8; 4096];
                    let n = stream.read(&mut buf).unwrap_or(0);
                    let request = String::from_utf8_lossy(&buf[..n]).to_string();

                    // Parse the GET request path
                    if let Some(path_line) = request.lines().next() {
                        let path = path_line.split_whitespace().nth(1).unwrap_or("/");

                        if path.starts_with("/callback") {
                            // Check if tokens are already in query string (PKCE flow)
                            let query_in_path = if let Some(q) = path.find('?') {
                                Some(path[q+1..].to_string())
                            } else {
                                None
                            };

                            if let Some(query) = query_in_path {
                                // Tokens came in query string — emit directly
                                let callback_url = format!("emuworld://auth-callback?{}", query);
                                let _ = handle.emit("oauth-callback", callback_url);
                                push_log("INFO", "OAuth tokens received via query string");
                                let html = r#"<!DOCTYPE html><html><head><meta charset="utf-8"><title>EmuWorld</title>
<style>body{background:#1a1a2e;display:grid;place-items:center;min-height:100vh;margin:0;font-family:-apple-system,sans-serif}
.card{background:#16213e;border:1px solid rgba(99,102,241,0.3);border-radius:16px;padding:40px;text-align:center;max-width:380px}
h2{color:#4ade80;margin:0 0 8px}p{color:#94a3b8;font-size:14px;margin:0}</style></head>
<body><div class="card"><h2>✅ Connecté !</h2><p>Tu peux fermer cet onglet.</p></div>
<script>setTimeout(()=>window.close(),1500)</script></body></html>"#;
                                let response = format!(
                                    "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                    html.len(), html
                                );
                                let _ = stream.write_all(response.as_bytes());
                                let _ = stream.flush();
                                break; // Done
                            }

                            // No query — tokens may be in hash fragment (implicit flow)
                            let html = r#"<!DOCTYPE html><html><head><meta charset="utf-8"><title>EmuWorld</title>
<style>body{background:#1a1a2e;display:grid;place-items:center;min-height:100vh;margin:0;font-family:-apple-system,sans-serif}
.card{background:#16213e;border:1px solid rgba(99,102,241,0.3);border-radius:16px;padding:40px;text-align:center;max-width:380px}
h2{color:#4ade80;margin:0 0 8px}p{color:#94a3b8;font-size:14px;margin:0}
.waiting h2{color:#a78bfa}.waiting p{color:#64748b}</style></head>
<body><div class="card waiting" id="card"><h2>⏳ Connexion en cours...</h2><p>Transfert vers EmuWorld...</p></div>
<script>
const hash = window.location.hash.substring(1);
const search = window.location.search.substring(1);
const params = hash || search;
if (params) {
    fetch('/token?' + params).then(() => {
        document.getElementById('card').innerHTML = '<h2 style="color:#4ade80">✅ Connecté !</h2><p>Tu peux fermer cet onglet.</p>';
        document.getElementById('card').className = 'card';
        setTimeout(() => window.close(), 1500);
    });
} else {
    document.getElementById('card').innerHTML = '<h2 style="color:#4ade80">✅ Connecté !</h2><p>Tu peux fermer cet onglet.</p>';
    document.getElementById('card').className = 'card';
}
</script></body></html>"#;
                            let response = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: keep-alive\r\n\r\n{}",
                                html.len(), html
                            );
                            let _ = stream.write_all(response.as_bytes());
                            let _ = stream.flush();
                        } else if path.starts_with("/token?") {
                            // Extract tokens from query string
                            let query = &path[7..]; // skip "/token?"
                            let callback_url = format!("emuworld://auth-callback#{}", query);
                            let _ = handle.emit("oauth-callback", callback_url);
                            push_log("INFO", "OAuth tokens received via HTTP server");

                            let response = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nAccess-Control-Allow-Origin: *\r\n\r\nOK";
                            let _ = stream.write_all(response.as_bytes());
                            let _ = stream.flush();
                            break; // Done, stop server
                        } else {
                            let response = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";
                            let _ = stream.write_all(response.as_bytes());
                        }
                    }
                }
                Err(_) => break,
            }
        }
    });

    Ok(port)
}

#[tauri::command]
fn create_overlay_window(app_handle: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    use tauri::WebviewWindowBuilder;

    if app_handle.get_webview_window("overlay").is_some() {
        return Ok(());
    }

    let url = tauri::WebviewUrl::App("index.html?overlay=1".into());
    let builder = WebviewWindowBuilder::new(&app_handle, "overlay", url)
        .title("EmuWorld Overlay")
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .fullscreen(true)
        .skip_taskbar(true)
        .focused(true);

    builder.build().map_err(|e| e.to_string())?;
    push_log("INFO", "Overlay window created");
    Ok(())
}

#[tauri::command]
fn close_overlay_window(app_handle: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    if let Some(win) = app_handle.get_webview_window("overlay") {
        win.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn export_config() -> Result<String, String> {
    push_log("INFO", "Export de la configuration complète");
    let export = FullExport {
        config: get_config(),
        playtime: playtime::load(),
    };
    serde_json::to_string_pretty(&export).map_err(|e| e.to_string())
}

#[tauri::command]
fn read_text_file(path: String) -> Result<String, String> {
    let p = PathBuf::from(&path);
    if p.extension().and_then(|e| e.to_str()) != Some("json") {
        return Err("Only .json files can be read".to_string());
    }
    if path.contains("..") {
        return Err("Path traversal not allowed".to_string());
    }
    std::fs::read_to_string(&path).map_err(|e| format!("Cannot read {}: {}", path, e))
}

#[tauri::command]
fn import_config(json: String) -> Result<(), String> {
    push_log("INFO", "Import de configuration depuis JSON");
    let imported: FullExport = serde_json::from_str(&json).map_err(|e| format!("JSON invalide: {}", e))?;
    save_config(imported.config)?;
    playtime::overwrite(imported.playtime)?;
    push_log("INFO", "Configuration importée avec succès");
    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .manage(discord_rpc::RpcState::new())
        .manage(Mutex::new(CurrentPlayingState::default()))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            for mut arg in argv {
                arg = arg.trim_matches('"').to_string();
                if arg.starts_with("emuworld://") {
                    let _ = app.emit("oauth-callback", arg);
                }
            }
        }))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            #[cfg(desktop)]
            {
                use tauri_plugin_deep_link::DeepLinkExt;

                // Register `emuworld://` with the OS at runtime. The scheme is
                // declared in tauri.conf.json but on Windows in dev mode the
                // installer step is skipped — without this the browser can't
                // hand the callback back to us after OAuth.
                if let Err(e) = app.deep_link().register_all() {
                    eprintln!("[deep-link] register_all failed: {}", e);
                }

                let handle = app.handle().clone();
                app.deep_link().on_open_url(move |event| {
                    if let Some(url) = event.urls().first() {
                        let _ = handle.emit("oauth-callback", url.to_string());
                    }
                });

                // Start gamepad polling thread
                let gamepad_handle = app.handle().clone();
                gamepad::start_gamepad_thread(gamepad_handle);

                // Register global Ctrl+F12 for screenshots (works even when game is focused)
                use tauri_plugin_global_shortcut::GlobalShortcutExt;
                let shortcut_handle = app.handle().clone();
                app.global_shortcut().on_shortcut("ctrl+f12", move |_app, _shortcut, event| {
                    if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        use tauri::Manager;
                        let (game_name, console) = {
                            let state = shortcut_handle.state::<Mutex<CurrentPlayingState>>();
                            let s = state.lock().unwrap();
                            (s.game_name.clone(), s.console.clone())
                        };
                        if let (Some(name), Some(cons)) = (game_name, console) {
                            match take_screenshot(name, cons) {
                                Ok(_) => { let _ = shortcut_handle.emit("screenshot-taken", "ok"); }
                                Err(e) => { let _ = shortcut_handle.emit("screenshot-error", e); }
                            }
                        } else {
                            let _ = shortcut_handle.emit("screenshot-error", "no_game_running");
                        }
                    }
                }).map_err(|e| eprintln!("[global-shortcut] failed: {}", e)).ok();

                // Low-level keyboard hook for Shift+Tab overlay (works even in fullscreen)
                let overlay_handle = app.handle().clone();
                std::thread::spawn(move || {
                    start_keyboard_hook(overlay_handle);
                });

                // Recover orphaned game session (EmuWorld was killed while a game was running)
                let session_file = emuworld_base_dir().join("current_session.json");
                if session_file.exists() {
                    if let Ok(content) = fs::read_to_string(&session_file) {
                        if let Ok(session) = serde_json::from_str::<serde_json::Value>(&content) {
                            let game = session["game"].as_str().unwrap_or_default().to_string();
                            let console = session["console"].as_str().unwrap_or_default().to_string();
                            let emulator = session["emulator"].as_str().unwrap_or_default().to_string();
                            let start_epoch = session["start_epoch"].as_u64().unwrap_or(0);
                            if start_epoch > 0 && !game.is_empty() {
                                let now_epoch = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_secs();
                                let elapsed = now_epoch.saturating_sub(start_epoch);
                                if elapsed >= 3 && elapsed < 86400 {
                                    let _ = playtime::record_session(&console, &game, elapsed, &emulator);
                                    push_log("INFO", &format!("Session orpheline récupérée : {} — {}s", game, elapsed));
                                }
                            }
                        }
                    }
                    let _ = fs::remove_file(&session_file);
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            is_portable_mode,
            save_config,
            get_emulator_catalog,
            get_installed_emulators,
            install_emulator,
            uninstall_emulator,
            launch_emulator,
            scan_roms,
            cancel_scan,
            fetch_boxart,
            search_rom_store,
            get_store_consoles,
            get_featured_games,
            download_rom,
            delete_rom,
            get_rgs_constructeurs,
            get_rgs_consoles,
            get_rgs_liens,
            search_rgs,
            scrape_1fichier_dir,
            resolve_1fichier_names,
            download_1fichier,
            cancel_download,
            finalize_rgs_import,
            get_myrient_consoles,
            browse_myrient,
            download_myrient_rom,
            get_vimm_consoles,
            browse_vimm,
            search_vimm,
            download_vimm_rom,
            get_playtime,
            toggle_favorite,
            set_game_rating,
            set_game_notes,
            create_collection,
            delete_collection,
            rename_collection,
            add_to_collection,
            remove_from_collection,
            get_profile_stats,
            clear_playtime,
            overwrite_playtime,
            clear_cover_cache,
            get_achievements,
            get_achievement_rank,
            check_achievements,
            unlock_achievement,
            save_ra_credentials,
            get_ra_credentials,
            get_ra_game_progress,
            get_ra_completed_games,
            ra_login,
            configure_ra_emulators,
            download_ra_cores,
            save_b2_config,
            get_b2_config,
            scan_local_saves,
            backup_saves_to_cloud,
            list_cloud_backups,
            restore_cloud_backup,
            delete_cloud_backup,
            discord_rpc::discord_set_idle,
            discord_rpc::discord_set_playing,
            discord_rpc::discord_clear,
            get_logs,
            clear_logs,
            get_cover_url,
            check_roms_health,
            delete_unhealthy_roms,
            check_emulator_updates,
            get_log_file_path,
            get_logs_directory,
            save_emulator_version,
            open_path,
            launch_netplay,
            start_oauth_server,
            create_overlay_window,
            close_overlay_window,
            export_config,
            import_config,
            read_text_file,
            watch_roms_directory,
            set_current_playing,
            get_current_playing,
            take_screenshot,
            migrate_covers_to_webp,
            get_screenshots,
            get_all_screenshots,
            delete_screenshot,
            clear_achievements,
            overwrite_achievements,
            fetch_game_guide_data,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                use tauri::Emitter;
                let _ = window.emit("app-closing", ());
                std::thread::sleep(std::time::Duration::from_millis(300));
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running EmuWorld");
}
