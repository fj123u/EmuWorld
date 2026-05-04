use serde::{Deserialize, Serialize};
use regex::Regex;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tauri::Emitter;
use reqwest;
use urlencoding;
use base64::Engine;
use std::io::Write;

mod emulators;
mod playtime;
mod discord_rpc;
mod achievements;
mod gamepad;

fn write_to_boxart_log(message: &str) {
    let mut path = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("EmuWorld");
    let _ = std::fs::create_dir_all(&path);
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
}

impl Default for AppConfig {
    fn default() -> Self {
        let base = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("EmuWorld");
        Self {
            roms_directory: base.join("ROMs").to_string_lossy().to_string(),
            emulators_directory: base.join("Emulators").to_string_lossy().to_string(),
            covers_directory: base.join("Covers").to_string_lossy().to_string(),
        }
    }
}

fn config_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("EmuWorld")
        .join("config.json")
}

#[tauri::command]
fn get_config() -> AppConfig {
    let path = config_path();
    let config = if path.exists() {
        let data = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        AppConfig::default()
    };

    fs::create_dir_all(&config.roms_directory).ok();
    fs::create_dir_all(&config.emulators_directory).ok();
    fs::create_dir_all(&config.covers_directory).ok();

    config
}

#[tauri::command]
fn save_config(config: AppConfig) -> Result<(), String> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let data = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    fs::write(&path, data).map_err(|e| e.to_string())?;
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
                        // Always return lowercase to match catalog IDs
                        installed.push(name.to_lowercase());
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

    let config = get_config();
    let install_dir = PathBuf::from(&config.emulators_directory).join(&emu.id);

    if install_dir.exists() {
        fs::remove_dir_all(&install_dir).ok();
    }
    fs::create_dir_all(&install_dir).map_err(|e| format!("Failed to create directory: {}", e))?;

    let _ = app_handle.emit("install-progress", serde_json::json!({
        "emulator_id": emulator_id,
        "status": "downloading",
        "progress": 10
    }));

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(&emu.download_url)
        .header("User-Agent", "EmuWorld/0.1.0 (Windows; Desktop)")
        .send()
        .await
        .map_err(|e| format!("Download failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Download failed with HTTP status: {}", response.status()));
    }

    let bytes = response.bytes().await.map_err(|e| format!("Failed to read download data: {}", e))?;
    if bytes.is_empty() {
        return Err("Downloaded file is empty".to_string());
    }

    let _ = app_handle.emit("install-progress", serde_json::json!({
        "emulator_id": emulator_id,
        "status": "extracting",
        "progress": 60
    }));

    let archive_ext = if emu.archive_type == "7z" { "7z" } else { "zip" };
    let archive_path = install_dir.join(format!("archive.{}", archive_ext));
    fs::write(&archive_path, &bytes).map_err(|e| format!("Failed to save archive: {}", e))?;

    if emu.archive_type == "zip" {
        extract_zip(&archive_path, &install_dir).map_err(|e| format!("Zip extraction failed: {}", e))?;
    } else if emu.archive_type == "7z" {
        extract_7z(&archive_path, &install_dir).map_err(|e| format!("7z extraction failed: {}", e))?;
    } else {
        return Err(format!("Unsupported archive type: {}", emu.archive_type));
    }

    // Clean up archive
    fs::remove_file(&archive_path).ok();

    // Verify executable exists
    if find_executable(&install_dir, &emu.executable_name).is_none() {
        return Err(format!("Installation failed: Executable '{}' not found in the extracted files.", emu.executable_name));
    }

    let _ = app_handle.emit("install-progress", serde_json::json!({
        "emulator_id": emulator_id,
        "status": "done",
        "progress": 100
    }));

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
    sevenz_rust::decompress_file(archive_path, install_dir).map_err(|e| e.to_string())
}

#[tauri::command]
fn uninstall_emulator(emulator_id: String) -> Result<String, String> {
    let id_lower = emulator_id.to_lowercase();
    let config = get_config();
    let install_dir = PathBuf::from(&config.emulators_directory).join(&id_lower);
    
    if install_dir.exists() {
        // Try to remove. On Windows, this fails if an EXE is running.
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
    let catalog = emulators::get_catalog();
    let emu = catalog.iter().find(|e| e.id == emulator_id).ok_or_else(|| "Emulator not found".to_string())?.clone();
    let config = get_config();
    let install_dir = PathBuf::from(&config.emulators_directory).join(&emu.id);
    let exe_path = find_executable(&install_dir, &emu.executable_name)
        .ok_or_else(|| format!("Executable '{}' not found.", emu.executable_name))?;
    let mut cmd = Command::new(&exe_path);
    cmd.current_dir(exe_path.parent().unwrap_or(&install_dir));
    if let Some(rom) = rom_path.clone() {
        let final_path = rom.replace(r"\\?\", "").replace("/", "\\");
        println!("[Launch] Running: {:?} with Arg: {:?}", exe_path, final_path);

        // Handle RetroArch cores if applicable
        if emu.id.starts_with("retroarch") {
            if let Some(core) = &emu.core_name {
                if let Some(core_path) = find_executable(&install_dir, core) {
                    println!("[Launch] Detected RetroArch core: {:?}", core_path);
                    cmd.arg("-L");
                    cmd.arg(core_path);
                } else {
                    println!("[Launch] WARNING: Core '{}' not found in {}", core, install_dir.display());
                }
            }
        }

        // Cemu requires -g flag to launch a game
        if emu.id == "cemu" {
            cmd.arg("-g");
        }

        cmd.arg(&final_path);
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
            return Err(format!("Could not start emulator: {}", e));
        }
    };

    println!("[Launch] Success!");
    let launched_name = emu.name.clone();

    // Track playtime only when we have a ROM context (launching the bare emulator doesn't count as a game session).
    if let (Some(name), Some(console)) = (rom_name.clone(), rom_console.clone()) {
        let emulator_id_for_task = emu.id.clone();
        // Wait for the child to exit on a blocking thread, then record the session.
        tauri::async_runtime::spawn_blocking(move || {
            use tauri::Emitter;
            let start = std::time::Instant::now();
            match child.wait() {
                Ok(status) => println!("[Launch] Child exited ({:?}) for {}", status, name),
                Err(e) => println!("[Launch] wait() failed: {}", e),
            }
            let elapsed = start.elapsed().as_secs();
            // Ignore sessions < 3s (likely the emulator crashed or the user mis-clicked).
            if elapsed >= 3 {
                if let Err(e) = playtime::record_session(&console, &name, elapsed, &emulator_id_for_task) {
                    println!("[Playtime] record failed: {}", e);
                }
            }
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

#[tauri::command]
fn scan_roms(directory: String) -> Vec<RomFile> {
    let catalog = emulators::get_catalog();
    let mut roms = vec![];
    let dir = PathBuf::from(&directory);
    if !dir.exists() { return roms; }
    for entry in walkdir::WalkDir::new(&dir).max_depth(5) {
        if let Ok(e) = entry {
            if e.file_type().is_file() {
                if let Some(ext) = e.path().extension() {
                    let ext_str = ext.to_string_lossy().to_lowercase();
                    if let Some((console, emu_id)) = match_extension(&ext_str, &catalog) {
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
    roms
}

/// Detect if a ROM file is a game update or DLC (should be hidden from the library)
fn is_update_or_dlc(name: &str, _ext: &str) -> bool {
    let lower = name.to_lowercase();
    
    // Keyword-based detection (DLC, UPD, etc.)
    if lower.contains("upd") || lower.contains("dlc") || lower.contains("patch") || lower.contains("update") {
        return true;
    }
    
    // Switch Title ID detection: Extract hex IDs from brackets
    // Base game IDs MUST end in 000. 
    // Updates end in 800, DLCs end in 001-7FF.
    if let Some(id) = extract_title_id(name) {
        // Switch check
        if id.starts_with("010") && !id.ends_with("000") {
            return true; 
        }
        // Wii U check: Base=00050000, Update=0005000E, DLC=0005000C
        if id.starts_with("0005000E") || id.starts_with("0005000C") {
            return true;
        }
    }
    
    // Additional Switch specific: Check for version strings in brackets like [v65536]
    // Base games are usually [v0].
    if lower.contains("[v0]") {
        // Base game, don't filter
    } else if lower.contains("[v") {
        // likely an update like [v65536]
        return true;
    }
    
    // Folder-based: if path contains "update" or "dlc" folder (common in multi-folder dumps)
    if (lower.contains("update") || lower.contains("dlc")) || (lower.contains("patch") && !lower.contains("game")) {
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
    let p = PathBuf::from(&path);
    if !p.exists() {
        return Err("File not found".to_string());
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
    
    let cover_path = PathBuf::from(&config.covers_directory)
        .join(&console_dir)
        .join(format!("{}.png", name));
    if cover_path.exists() {
        let _ = fs::remove_file(cover_path);
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

#[tauri::command]
async fn fetch_boxart(app_handle: tauri::AppHandle, game_name: String, console: String, force_refresh: Option<bool>) -> Result<String, String> {
    let force_refresh = force_refresh.unwrap_or(false);
    let config = get_config();
    let covers_dir = PathBuf::from(&config.covers_directory);

    println!("[Boxart] Request for: '{}' ({})", game_name, console);

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
        "Game Boy" => vec!["Nintendo - Game Boy"],
        "Game Boy Color" | "GBC" => vec!["Nintendo - Game Boy Color"],
        "Game Boy Advance" | "GBA" => vec!["Nintendo - Game Boy Advance"],
        "Nintendo DS" => vec!["Nintendo - Nintendo DS"],
        "GameCube" => vec!["Nintendo - GameCube"],
        "GameCube / Wii" | "GameCube - Wii" => vec!["Nintendo - Wii", "Nintendo - GameCube"],
        "Wii" => vec!["Nintendo - Wii"],
        "Wii U" => vec!["Nintendo - Wii U"],
        "Nintendo Switch" => vec!["Nintendo - Nintendo Switch"],
        "Virtual Boy" => vec!["Nintendo - Virtual Boy"],
        "PlayStation 1" | "PS1" => vec!["Sony - PlayStation"],
        "PlayStation 2" | "PS2" => vec!["Sony - PlayStation 2"],
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
        let _ = std::fs::remove_file(&target_png);
    }

    // 1. First check local covers directory (skipped when force_refresh is set)
    if !force_refresh {
    if let Ok(entries) = std::fs::read_dir(&console_covers_dir) {
        let mut best_local = None;
        for entry in entries.flatten() {
            if let Some(file_name) = entry.file_name().to_str() {
                let lower = file_name.to_lowercase();
                if lower.ends_with(".png") || lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
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
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                    return Ok(format!("data:image/png;base64,{}", b64));
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
                            let _ = std::fs::create_dir_all(&console_covers_dir);
                            let _ = std::fs::write(console_covers_dir.join(format!("{}.png", &safe_name)), &bytes);
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
                        let _ = std::fs::create_dir_all(&console_covers_dir);
                        let _ = std::fs::write(console_covers_dir.join(format!("{}.png", &safe_name)), &bytes);
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
                                let _ = std::fs::create_dir_all(&console_covers_dir);
                                let _ = std::fs::write(console_covers_dir.join(format!("{}.png", &safe_name)), &bytes);
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
        // --- 3.1: Libretro ---
        for folder in &libretro_systems {
            let url = format!("https://thumbnails.libretro.com/{}/Named_Boxarts/{}.png", urlencoding::encode(folder), urlencoding::encode(search_name));
            if let Ok(resp) = client.get(&url).send().await {
                if resp.status().is_success() {
                    if let Ok(bytes) = resp.bytes().await {
                        if bytes.len() >= min_size {
                            write_to_boxart_log(&format!("Result: Libretro Success ({})", search_name));
                            let _ = std::fs::create_dir_all(&console_covers_dir);
                            let _ = std::fs::write(console_covers_dir.join(format!("{}.png", &safe_name)), &bytes);
                            return Ok(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(&bytes)));
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
                                                        let _ = std::fs::create_dir_all(&console_covers_dir);
                                                        let _ = std::fs::write(console_covers_dir.join(format!("{}.png", &safe_name)), &bytes);
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
                                let _ = std::fs::create_dir_all(&console_covers_dir);
                                let _ = std::fs::write(console_covers_dir.join(format!("{}.png", &safe_name)), &bytes);
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
                            let _ = std::fs::create_dir_all(&console_covers_dir);
                            let _ = std::fs::write(console_covers_dir.join(format!("{}.png", &safe_name)), &bytes);
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
                                                            let _ = std::fs::create_dir_all(&console_covers_dir);
                                                            let _ = std::fs::write(console_covers_dir.join(format!("{}.png", &safe_name)), &bytes);
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
                                                    let _ = std::fs::create_dir_all(&console_covers_dir);
                                                    let _ = std::fs::write(console_covers_dir.join(format!("{}.png", &safe_name)), &bytes);
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
    Err("No boxart found".to_string())
}

/// Clean a game name for searching (handles scene tags, IDs, etc.)
fn clean_game_name(name: &str) -> String {
    let mut cleaned = name.to_string();
    
    // Remove extensions first
    let extensions = vec![".iso", ".chd", ".rvz", ".wbfs", ".nca", ".nsp", ".xci", ".zip", ".7z", ".gz", ".wud", ".wux", ".rpx", ".nes", ".sfc", ".smc", ".gba", ".gbc", ".gb", ".nds", ".n64", ".z64"];
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

    // Strip common scene keywords
    let scene_keywords = vec![
        "PROPER", "REPACK", "NSW", "MULTi", "READNFO", "INTERNAL", "D0WNLOAD",
        "BigBlueBox", "Kaze-Nico", "kaze-nico", "v1.1", "v1.0", "Update", "DLC", "Patch",
        "Collection", "v0", "v65536", "nsw2u", "NKA", "NC", "NT"
    ];
    for kw in scene_keywords {
        let re = Regex::new(&format!(r"(?i)\b{}\b", kw)).unwrap();
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
        (&["pokemon", "scarlet"], "0100A3D008C5C000"),
        (&["pokemon", "violet"], "01008F6008C5E000"),
        (&["pokemon legends", "arceus"], "01001F5010DFA000"),
        (&["pokemon legends", "za"], "0100F43008C44000"),
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
        (&["zelda", "wind waker"], "WDKE"),           // Wind Waker HD (US; EUR = WDKP, JP = WDKJ)
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
        ("Perle", "Pearl"), ("Noir", "Black"), ("Blanc", "White"), ("Soleil", "Sun"), ("Lune", "Moon")
    ];

    let mut translated = pure.clone();
    let mut matched = false;
    for (fr, en) in fr_to_en {
        if translated.contains(fr) {
            translated = translated.replace(fr, en);
            matched = true;
        }
    }
    if matched {
        candidates.push(translated.clone());
        candidates.push(format!("{} (World)", translated));
        candidates.push(format!("{} (USA)", translated));
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

    // Tomodachi Life (handles truncated "Living th..." filenames)
    if pure.to_lowercase().contains("tomodachi") {
        candidates.push("Tomodachi Life Living the Dream".to_string());
        candidates.push("Tomodachi Life: Living the Dream".to_string());
        candidates.push("Tomodachi Life".to_string());
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
                candidates.push(format!("The Legend of {}", titled));
            }
        }
    }

    // Deduplicate and return
    candidates.dedup();
    candidates
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
                // Clean up query: remove & and other special IA-reserved characters
                let clean_query = query.replace("&", "*").replace(":", " ").replace("-", " ").replace("+", " ");
                (format!("collection:({}) AND (title:({})^10 OR {})", collection, clean_query, clean_query), "", console)
            }
        } else {
            return Err(format!("Unknown console: {}", console));
        }
    } else {
        if query.is_empty() {
            ("mediatype:software AND (subject:rom OR subject:redump OR subject:no-intro) AND (subject:nintendo OR subject:sony OR subject:sega) AND downloads:[1000 TO *] AND NOT title:(part OR bios OR set OR merged OR pack OR collection OR bundle OR \"rom pack\" OR \"rom set\" OR roms OR \"iso set\" OR \"romset\")".to_string(), "&sort[]=downloads%20desc", "Multiple".to_string())
        } else {
            (format!("(rom OR emulator OR game) AND mediatype:software AND title:(\"{}\") AND NOT title:(pack OR bundle OR collection OR romset OR roms)", query), "&sort[]=downloads%20desc", "Mixed".to_string())
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
    let dest_dir = roms_dir.join(&console);
    if !dest_dir.exists() {
        fs::create_dir_all(&dest_dir).map_err(|e| format!("Failed to create console directory: {}", e))?;
    }
    
    let mut final_url = download_url_arg;
    let mut final_file_name = if file_name_arg.is_empty() { "game.bin".to_string() } else { file_name_arg };
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

    use std::io::Write;
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        let chunk: bytes::Bytes = chunk;
        file.write_all(&chunk).map_err(|e| {
            println!("[Download] Write error: {}", e);
            e.to_string()
        })?;
        downloaded_bytes += chunk.len() as u64;

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
    
    if is_zip {
        println!("[Download] Detected ZIP archive, extracting...");
        match extract_rom_zip(&dest, &dest_dir) {
            Ok(extracted_files) => {
                println!("[Download] Extracted {} files: {:?}", extracted_files.len(), extracted_files);
                // Delete the zip after successful extraction
                let _ = fs::remove_file(&dest);
                println!("[Download] Deleted ZIP archive: {}", dest.display());
            }
            Err(e) => {
                println!("[Download] ZIP extraction failed: {} — keeping raw file", e);
            }
        }
    } else {
        println!("[Download] File is not a ZIP, keeping as-is");
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
    src_path: String,
    console: String,
) -> Result<String, String> {
    let config = get_config();
    let roms_dir = std::path::PathBuf::from(&config.roms_directory);
    let dest_dir = roms_dir.join(&console);
    
    if !dest_dir.exists() {
        fs::create_dir_all(&dest_dir).map_err(|e| format!("Failed to create console directory: {}", e))?;
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

    let dest_dir = roms_dir.join(&final_console);
    if !dest_dir.exists() {
        fs::create_dir_all(&dest_dir).map_err(|e| format!("Failed to create console directory: {}", e))?;
    }

    let dest = dest_dir.join(&file_name);
    
    println!("[Import] Moving {} to {} (Console: {})", src.display(), dest.display(), final_console);
    
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
        println!("[Import] Detected 7z archive, extracting...");
        match extract_7z(&dest, &dest_dir) {
            Ok(()) => {
                println!("[Import] Extracted 7z successfully");
                let _ = fs::remove_file(&dest);
            }
            Err(e) => {
                println!("[Import] 7z extraction failed: {} — keeping raw file", e);
            }
        }
    }

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
        let name = entry.name().to_string();
        
        // Skip directories and macOS resource forks
        if entry.is_dir() || name.starts_with("__MACOSX") || name.starts_with(".") {
            continue;
        }
        
        // Extract to the destination directory (flatten — no subdirectories)
        let file_name = std::path::Path::new(&name)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or(name.clone());
        
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
    let config = get_config();
    let roms_dir = std::path::PathBuf::from(&config.roms_directory);
    let dest_dir = roms_dir.join(&console);
    if !dest_dir.exists() {
        fs::create_dir_all(&dest_dir).map_err(|e| format!("Failed to create console directory: {}", e))?;
    }

    let dest = dest_dir.join(&file_name);

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

    let mut file = fs::File::create(&dest).map_err(|e| e.to_string())?;
    use std::io::Write;
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        file.write_all(&chunk).map_err(|e| e.to_string())?;
        downloaded_bytes += chunk.len() as u64;

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
        // Find the first <a> whose href matches /vault/<numeric-id>
        let game_link = row.select(&link_selector).find(|a| {
            let href = a.value().attr("href").unwrap_or("");
            id_re.is_match(href)
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
    let id_re = Regex::new(r"/vault/(\d+)").map_err(|e| e.to_string())?;

    let mut games = Vec::new();
    for row in document.select(&row_selector) {
        let game_link = row.select(&link_selector).find(|a| {
            let href = a.value().attr("href").unwrap_or("");
            id_re.is_match(href)
        });
        let Some(link) = game_link else { continue };
        let href = link.value().attr("href").unwrap_or("");
        let Some(caps) = id_re.captures(href) else { continue };
        let id = caps[1].to_string();
        let name = link.text().collect::<Vec<_>>().join("").trim().to_string();
        if name.is_empty() { continue; }

        let tds: Vec<_> = row.select(&td_selector).collect();
        let region = tds.get(1).map(|td| td.text().collect::<Vec<_>>().join("").trim().to_string()).unwrap_or_default();

        games.push(VimmGame {
            id: id.clone(),
            name,
            region,
            version: String::new(),
            languages: String::new(),
            rating: String::new(),
            box_url: format!("https://dl.vimm.net/image.php?type=box&id={}", id),
            page_url: format!("https://vimm.net/vault/{}", id),
        });
    }
    Ok(games)
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
    let config = get_config();
    let covers_dir = PathBuf::from(&config.covers_directory);
    if covers_dir.exists() {
        std::fs::remove_dir_all(&covers_dir).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(&covers_dir).map_err(|e| e.to_string())?;
    }
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

pub fn run() {
    tauri::Builder::default()
        .manage(discord_rpc::RpcState::new())
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
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            get_emulator_catalog,
            get_installed_emulators,
            install_emulator,
            uninstall_emulator,
            launch_emulator,
            scan_roms,
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
            finalize_rgs_import,
            get_myrient_consoles,
            browse_myrient,
            download_myrient_rom,
            get_vimm_consoles,
            browse_vimm,
            search_vimm,
            get_playtime,
            toggle_favorite,
            get_profile_stats,
            clear_playtime,
            overwrite_playtime,
            clear_cover_cache,
            get_achievements,
            get_achievement_rank,
            check_achievements,
            unlock_achievement,
            discord_rpc::discord_set_idle,
            discord_rpc::discord_set_playing,
            discord_rpc::discord_clear,
        ])
        .run(tauri::generate_context!())
        .expect("error while running EmuWorld");
}
