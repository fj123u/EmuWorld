use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tauri::Emitter;
use reqwest;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

mod emulators;

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
fn launch_emulator(emulator_id: String, rom_path: Option<String>) -> Result<String, String> {
    let catalog = emulators::get_catalog();
    let emu = catalog.iter().find(|e| e.id == emulator_id).ok_or_else(|| "Emulator not found".to_string())?;
    let config = get_config();
    let install_dir = PathBuf::from(&config.emulators_directory).join(&emu.id);
    let exe_path = find_executable(&install_dir, &emu.executable_name)
        .ok_or_else(|| format!("Executable '{}' not found.", emu.executable_name))?;
    let mut cmd = Command::new(&exe_path);
    cmd.current_dir(exe_path.parent().unwrap_or(&install_dir));
    if let Some(rom) = rom_path {
        let mut clean_rom = rom.replace(r"\\?\", "");
        if clean_rom.starts_with(r"\\?\") {
             clean_rom = clean_rom.trim_start_matches(r"\\?\").to_string();
        }
        let final_path = clean_rom.replace("/", "\\");
        cmd.arg(&final_path);
    }
    // Open in a new console window if requested (best for Ryujinx as per user request)
    #[cfg(target_os = "windows")]
    {
        const CREATE_NEW_CONSOLE: u32 = 0x00000010;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
        // Use CREATE_NEW_CONSOLE to ensure a separate window opens
        cmd.creation_flags(CREATE_NEW_CONSOLE | CREATE_NEW_PROCESS_GROUP);
    }
    cmd.spawn().map_err(|e| e.to_string())?;
    Ok(format!("Launched {}", emu.name))
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

/// Strip language code parentheses like "(En,Fr,De,Es,It)" — keeps region like "(Europe)"
fn regex_strip_languages(name: &str) -> String {
    let mut result = name.to_string();
    let mut changed = true;
    while changed {
        changed = false;
        if let Some(start) = result.rfind('(') {
            if let Some(end) = result[start..].find(')') {
                let content = &result[start + 1..start + end];
                // Language codes are short comma-separated items like "En,Fr,De"
                let is_lang = content.contains(',') && content.split(',').all(|s| {
                    let trimmed = s.trim();
                    trimmed.len() <= 4 && trimmed.chars().all(|c| c.is_alphabetic())
                });
                if is_lang {
                    result = format!("{}{}", &result[..start], &result[start + end + 1..]);
                    changed = true;
                }
            }
        }
    }
    result.trim().to_string()
}

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

fn match_extension(ext: &str, catalog: &[emulators::EmulatorInfo]) -> Option<(String, String)> {
    for emu in catalog {
        for supported_ext in &emu.supported_extensions {
            if supported_ext == ext { return Some((emu.console.clone(), emu.id.clone())); }
        }
    }
    None
}

#[tauri::command]
async fn fetch_boxart(game_name: String, console: String) -> Result<String, String> {
    let config = get_config();
    let covers_dir = PathBuf::from(&config.covers_directory);
    
    // Map EmuWorld console names to libretro system directory names
    // Some consoles have multiple possible system names for better matching
    let libretro_systems: Vec<&str> = match console.as_str() {
        "NES" => vec!["Nintendo - Nintendo Entertainment System"],
        "Super Nintendo" => vec!["Nintendo - Super Nintendo Entertainment System"],
        "Nintendo 64" => vec!["Nintendo - Nintendo 64"],
        "Game Boy Advance" => vec!["Nintendo - Game Boy Advance", "Nintendo - Game Boy Color", "Nintendo - Game Boy"],
        "Nintendo DS" => vec!["Nintendo - Nintendo DS"],
        "GameCube / Wii" => vec!["Nintendo - GameCube", "Nintendo - Wii"],
        "Wii U" => vec!["Nintendo - Wii U"],
        "Nintendo Switch" => vec!["Nintendo - Nintendo Switch"],
        "Virtual Boy" => vec!["Nintendo - Virtual Boy"],
        "PlayStation 1" => vec!["Sony - PlayStation"],
        "PlayStation 2" => vec!["Sony - PlayStation 2"],
        "PlayStation 3" => vec!["Sony - PlayStation 3"],
        "PlayStation Portable" => vec!["Sony - PlayStation Portable"],
        "Dreamcast" => vec!["Sega - Dreamcast"],
        "Mega Drive" => vec!["Sega - Mega Drive - Genesis"],
        "Master System" => vec!["Sega - Master System - Mark III"],
        "Saturn" => vec!["Sega - Saturn"],
        "Game Gear" => vec!["Sega - Game Gear"],
        "Xbox" => vec!["Microsoft - Xbox"],
        "Neo-Geo" => vec!["SNK - Neo Geo"],
        "Arcade" => vec!["FBNeo - Arcade Games", "MAME"],
        "PC Engine" => vec!["NEC - PC Engine - TurboGrafx 16"],
        "Atari 2600" => vec!["Atari - 2600"],
        _ => return Err(format!("No cover source for console: {}", console)),
    };
    
    // Libretro naming: ONLY replace chars truly forbidden in URLs/filenames
    // Keep apostrophes, parentheses, commas — libretro uses them!
    let forbidden = ['&', '*', '/', ':', '<', '>', '?', '\\', '|', '"'];
    let safe_name: String = game_name.chars()
        .map(|c| if forbidden.contains(&c) { '_' } else { c })
        .collect();
    
    // Cache in per-console subdirectories
    let console_covers_dir = covers_dir.join(&console);
    let file_path = console_covers_dir.join(format!("{}.png", &safe_name));
    
    // Return cached file as base64 data URL if it exists
    if file_path.exists() {
        if let Ok(data) = fs::read(&file_path) {
            use base64::Engine;
            let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
            return Ok(format!("data:image/png;base64,{}", b64));
        }
    }
    
    // Build candidate names to try (order matters — most specific first)
    let mut candidates = vec![safe_name.clone()];
    
    // Wii U Specific mapping for common short names
    // Note: libretro Wii U only has Japanese-named Zelda entries!
    if console == "Wii U" {
        let lower_name = safe_name.to_lowercase();
        if lower_name.contains("zelda") && lower_name.contains("twilight") && lower_name.contains("princess") {
            candidates.push("Zelda no Densetsu - Twilight Princess HD (Japan) (Rev 1)".to_string());
            candidates.push("Zelda no Densetsu - Twilight Princess HD (Japan)".to_string());
        }
        if lower_name.contains("zelda") && lower_name.contains("wind") && lower_name.contains("waker") {
            candidates.push("Zelda no Densetsu - Kaze no Takuto HD (Japan)".to_string());
        }
    }
    
    // GameCube / Wii: handle combo discs
    if console == "GameCube / Wii" {
        let lower_name = safe_name.to_lowercase();
        if lower_name.contains("wii sports") && lower_name.contains("resort") {
            candidates.push("Wii Sports + Wii Sports Resort (Europe) (En,Fr,De,Es,It,Nl,Pt)".to_string());
            candidates.push("Wii Sports + Wii Sports Resort (USA) (En,Fr,Es)".to_string());
        }
    }
    
    // Strip version tags like (v1.01) but keep region/language
    let no_version = regex_strip_version(&safe_name);
    if no_version != safe_name {
        candidates.push(no_version.clone());
    }
    
    // Strip only the language suffix: "(Europe) (En,Fr,De,Es,It)" -> "(Europe)"
    let no_lang = regex_strip_languages(&safe_name);
    if no_lang != safe_name {
        candidates.push(no_lang.clone());
    }
    
    // Strip ALL parenthetical content to get the base name
    let base_name = regex_strip_all_parens(&safe_name);
    if !base_name.is_empty() && base_name != safe_name {
        // Try with common region tags
        let regions = ["(USA)", "(Europe)", "(Japan)", "(World)"];
        for reg in regions {
            candidates.push(format!("{} {}", &base_name, reg));
        }
        candidates.push(base_name);
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()
        .map_err(|e| e.to_string())?;

    // ===== SWITCH SPECIFIC: Try Title ID-based cover sources =====
    if console == "Nintendo Switch" {
        // Extract Title ID from filename (16 hex chars in brackets like [0100000000010000])
        if let Some(title_id) = extract_title_id(&game_name) {
            // Normalize: base game ID ends in 000
            let base_id = format!("{}000", &title_id[..13]);
            
            // Primary: nlib.cc API (backed by TitleDB, returns JPEG icons)
            let mut switch_urls = vec![
                format!("https://api.nlib.cc/nx/{}/icon/256/256", &base_id),
                format!("https://api.nlib.cc/nx/{}/icon/256/256", &title_id),
            ];
            
            // Fallback: GameTDB (returns cover art images)
            switch_urls.push(format!("https://art.gametdb.com/switch/coverM/{}.jpg", &base_id));
            switch_urls.push(format!("https://art.gametdb.com/switch/coverM/{}.jpg", &title_id));

            for url in &switch_urls {
                match client.get(url).send().await {
                    Ok(response) if response.status().is_success() => {
                        if let Ok(bytes) = response.bytes().await {
                            if bytes.len() > 500 {
                                fs::create_dir_all(&console_covers_dir).ok();
                                if let Ok(mut file) = fs::File::create(&file_path) {
                                    use std::io::Write;
                                    let _ = file.write_all(&bytes);
                                }
                                use base64::Engine;
                                let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                                // Detect JPEG vs PNG
                                let mime = if bytes.starts_with(&[0xFF, 0xD8]) { "image/jpeg" } else { "image/png" };
                                return Ok(format!("data:{};base64,{}", mime, b64));
                            }
                        }
                    }
                    _ => continue,
                }
            }
        } else {
            // No Title ID in filename — try a known games lookup table
            let clean_name = game_name.split('[').next().unwrap_or(&game_name).trim().to_lowercase();
            let known_id = match clean_name.as_str() {
                // ===== Zelda =====
                s if s.contains("zelda") && s.contains("echoes") && s.contains("wisdom") => Some("01008CF01BAAC000"),
                s if s.contains("zelda") && s.contains("tears") && s.contains("kingdom") => Some("0100F2C0115B6000"),
                s if s.contains("zelda") && s.contains("breath") && s.contains("wild") => Some("01007EF00011E000"),
                s if s.contains("zelda") && s.contains("skyward") => Some("01002DA013484000"),
                s if s.contains("zelda") && s.contains("link") && s.contains("awaken") => Some("01006BB00C6F0000"),
                // ===== Mario =====
                s if s.contains("mario") && s.contains("luigi") && s.contains("brother") => Some("01006D0017F7A000"),
                s if s.contains("mario") && s.contains("wonder") => Some("010015100B514000"),
                s if s.contains("mario") && s.contains("odyssey") => Some("0100000000010000"),
                s if s.contains("mario kart") && s.contains("8") => Some("0100152000022000"),
                s if s.contains("mario") && s.contains("3d") && s.contains("world") => Some("010028600EBDA000"),
                s if s.contains("mario") && s.contains("3d") && s.contains("all") => Some("0100A3900C3E2000"),
                s if s.contains("mario") && s.contains("party") && s.contains("jamboree") => Some("0100965017338000"),
                s if s.contains("mario") && s.contains("party") && s.contains("super") => Some("010036B0034E4000"),
                s if s.contains("mario") && s.contains("rpg") => Some("0100BC0018138000"),
                s if s.contains("mario") && s.contains("strikers") => Some("010019401051C000"),
                s if s.contains("mario") && s.contains("tennis") => Some("0100BDE00862A000"),
                s if s.contains("mario") && s.contains("golf") => Some("0100C9C00E25C000"),
                s if s.contains("mario") && s.contains("maker") && s.contains("2") => Some("01009B90006DC000"),
                s if s.contains("mario") && s.contains("vs") && s.contains("donkey") => Some("0100B99019412000"),
                s if s.contains("paper mario") && s.contains("origami") => Some("0100A3900C3E2000"),
                s if s.contains("paper mario") && s.contains("thousand") => Some("0100ECD018EBE000"),
                s if s.contains("new super mario") && s.contains("deluxe") => Some("0100EA80032EA000"),
                // ===== Pokémon =====
                s if s.contains("pokemon") && s.contains("legends") && s.contains("z-a") => Some("0100F43008C44000"),
                s if s.contains("pokemon") && s.contains("legends") && s.contains("za") => Some("0100F43008C44000"),
                s if s.contains("pokemon") && s.contains("legends") && s.contains("arceus") => Some("01001F5010DFA000"),
                s if s.contains("pokemon") && s.contains("scarlet") => Some("0100A3D008C5C000"),
                s if s.contains("pokemon") && s.contains("violet") => Some("01008F6008C5E000"),
                s if s.contains("pokemon") && s.contains("sword") => Some("0100ABF008968000"),
                s if s.contains("pokemon") && s.contains("shield") => Some("01008DB008C2C000"),
                s if s.contains("pokemon") && s.contains("brilliant") => Some("0100000011D90000"),
                s if s.contains("pokemon") && s.contains("shining") => Some("010018E011D92000"),
                s if s.contains("pokemon") && s.contains("let's go") && s.contains("pikachu") => Some("010003F003A34000"),
                s if s.contains("pokemon") && s.contains("let's go") && s.contains("eevee") => Some("0100187003A36000"),
                s if s.contains("pokemon") && s.contains("snap") => Some("0100F4300BF2C000"),
                s if s.contains("pokemon") && s.contains("mystery") => Some("01003D200BAA2000"),
                // ===== Smash / Fighting =====
                s if s.contains("smash") && s.contains("bros") => Some("01006A800016E000"),
                // ===== Splatoon =====
                s if s.contains("splatoon") && s.contains("3") => Some("0100C2500FC20000"),
                s if s.contains("splatoon") && s.contains("2") => Some("01003BC0000A0000"),
                // ===== Animal Crossing =====
                s if s.contains("animal") && s.contains("crossing") => Some("01006F8002326000"),
                // ===== Donkey Kong =====
                s if s.contains("donkey kong") && s.contains("returns") => Some("01009D901BC56000"),
                s if s.contains("donkey kong") && s.contains("tropical") => Some("0100C1F0051B6000"),
                // ===== Kirby =====
                s if s.contains("kirby") && s.contains("forgotten") => Some("01004D300C5AE000"),
                s if s.contains("kirby") && s.contains("star") && s.contains("allies") => Some("01007E3006DDA000"),
                s if s.contains("kirby") && s.contains("return") && s.contains("dream") => Some("01006B601380E000"),
                // ===== Metroid =====
                s if s.contains("metroid") && s.contains("dread") => Some("010093801237C000"),
                s if s.contains("metroid") && s.contains("prime") && s.contains("remaster") => Some("010012101468C000"),
                // ===== Fire Emblem =====
                s if s.contains("fire emblem") && s.contains("engage") => Some("0100A6301214E000"),
                s if s.contains("fire emblem") && s.contains("three") && s.contains("houses") => Some("010055D009F78000"),
                s if s.contains("fire emblem") && s.contains("three") && s.contains("hopes") => Some("010071F0143EA000"),
                // ===== Xenoblade =====
                s if s.contains("xenoblade") && s.contains("3") => Some("010074F013262000"),
                s if s.contains("xenoblade") && s.contains("2") => Some("0100E95004038000"),
                s if s.contains("xenoblade") && s.contains("definitive") => Some("0100FF500E34A000"),
                // ===== Bayonetta =====
                s if s.contains("bayonetta") && s.contains("3") => Some("01004A4010FEA000"),
                s if s.contains("bayonetta") && s.contains("2") => Some("01007960049A0000"),
                // ===== Pikmin =====
                s if s.contains("pikmin") && s.contains("4") => Some("0100B7C00933A000"),
                s if s.contains("pikmin") && s.contains("3") => Some("0100F4C009322000"),
                // ===== Luigi =====
                s if s.contains("luigi") && s.contains("mansion") && s.contains("3") => Some("0100DCA0064A6000"),
                s if s.contains("luigi") && s.contains("mansion") && s.contains("2") => Some("010048701995E000"),
                // ===== Other Nintendo =====
                s if s.contains("princess peach") && s.contains("showtime") => Some("01007A3009184000"),
                s if s.contains("nintendo switch sports") => Some("0100D2F00D5C0000"),
                s if s.contains("ring fit") => Some("01002DA00AFFE000"),
                s if s.contains("arms") => Some("01009B500007C000"),
                s if s.contains("astral chain") => Some("01007300020FA000"),
                s if s.contains("advance wars") => Some("0100300012F2A000"),
                // ===== Third Party =====
                s if s.contains("sonic") && s.contains("shadow") && s.contains("generation") => Some("01005EA01C0FC000"),
                s if s.contains("sonic") && s.contains("frontiers") => Some("01004AD014BF0000"),
                s if s.contains("celeste") => Some("01002B30028F6000"),
                s if s.contains("hollow knight") => Some("0100633007D48000"),
                s if s.contains("hades") && !s.contains("2") => Some("0100535007A26000"),
                s if s.contains("stardew") && s.contains("valley") => Some("0100E65002BB8000"),
                s if s.contains("cuphead") => Some("0100A5C00D162000"),
                s if s.contains("ori") && s.contains("blind") && s.contains("forest") => Some("010061D00DB74000"),
                s if s.contains("ori") && s.contains("will") && s.contains("wisps") => Some("01008B900DC8A000"),
                s if s.contains("undertale") => Some("010080B00AD74000"),
                s if s.contains("dragon quest") && s.contains("builders") && s.contains("2") => Some("010042000A986000"),
                s if s.contains("dragon quest") && s.contains("builders") => Some("010008900705C000"),
                s if s.contains("dragon quest") && s.contains("xi") => Some("01006C300E9F0000"),
                s if s.contains("persona") && s.contains("5") && s.contains("royal") => Some("01005CA01580E000"),
                s if s.contains("persona") && s.contains("4") => Some("010062B01525C000"),
                s if s.contains("persona") && s.contains("3") => Some("010087701B092000"),
                s if s.contains("nier") && s.contains("automata") => Some("0100B8E016F76000"),
                s if s.contains("octopath") && s.contains("traveler") && s.contains("2") => Some("0100A3501946E000"),
                s if s.contains("octopath") && s.contains("traveler") => Some("010057D006492000"),
                s if s.contains("monster hunter") && s.contains("rise") => Some("0100559011D90000"),
                s if s.contains("forager") => Some("01001D200BCC4000"),
                s if s.contains("lego") && s.contains("horizon") => Some("010073C01AF34000"),
                s if s.contains("tomb raider") && s.contains("remaster") => Some("010024601BB16000"),
                s if s.contains("minecraft") => Some("0100D71004694000"),
                s if s.contains("terraria") => Some("0100E46006708000"),
                s if s.contains("among us") => Some("0100B0C013912000"),
                s if s.contains("overcooked") && s.contains("2") => Some("01006FD0080B2000"),
                s if s.contains("it takes two") => Some("010092A0172E4000"),
                s if s.contains("portal") && s.contains("companion") => Some("01007BC00E55A000"),
                _ => None,
            };
            if let Some(tid) = known_id {
                let icon_url = format!("https://api.nlib.cc/nx/{}/icon/256/256", tid);
                if let Ok(icon_resp) = client.get(&icon_url).send().await {
                    if icon_resp.status().is_success() {
                        if let Ok(bytes) = icon_resp.bytes().await {
                            if bytes.len() > 500 {
                                fs::create_dir_all(&console_covers_dir).ok();
                                if let Ok(mut file) = fs::File::create(&file_path) {
                                    use std::io::Write;
                                    let _ = file.write_all(&bytes);
                                }
                                use base64::Engine;
                                let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                                let mime = if bytes.starts_with(&[0xFF, 0xD8]) { "image/jpeg" } else { "image/png" };
                                return Ok(format!("data:{};base64,{}", mime, b64));
                            }
                        }
                    }
                }
            }
        }
    }

    // ===== LIBRETRO CDN: Try each system and each candidate name =====
    for system in &libretro_systems {
        let encoded_system = urlencoding::encode(system);
        
        for candidate in &candidates {
            let encoded_name = urlencoding::encode(candidate);
            let url = format!(
                "https://thumbnails.libretro.com/{}/Named_Boxarts/{}.png",
                encoded_system, encoded_name
            );
                
            match client.get(&url).send().await {
                Ok(response) if response.status().is_success() => {
                    if let Ok(bytes) = response.bytes().await {
                        if bytes.len() > 500 {
                            fs::create_dir_all(&console_covers_dir).ok();
                            if let Ok(mut file) = fs::File::create(&file_path) {
                                use std::io::Write;
                                let _ = file.write_all(&bytes);
                            }
                            use base64::Engine;
                            let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                            return Ok(format!("data:image/png;base64,{}", b64));
                        }
                    }
                }
                _ => continue,
            }
        }
    }
    
    Err("Boxart not found".to_string())
}

/// Extract a 16-character hex Title ID from a filename (e.g. "[0100000000010000]")
fn extract_title_id(name: &str) -> Option<String> {
    // Split by brackets and look for hex patterns
    for part in name.split('[') {
        if let Some(bracket_end) = part.find(']') {
            let content = part[..bracket_end].trim().to_uppercase();
            // Character-safe way to find 16 hex digits
            if content.len() >= 16 {
                let chars: Vec<char> = content.chars().collect();
                if chars.len() >= 16 {
                    for i in 0..=(chars.len() - 16) {
                        let potential: String = chars[i..i+16].iter().collect();
                        if potential.chars().all(|c| c.is_ascii_hexdigit()) {
                            return Some(potential);
                        }
                    }
                }
            }
        }
    }
    None
}


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            println!("[EmuWorld] Signal reçu d'une autre instance: {:?}", argv);
            for mut arg in argv {
                // Nettoyage rapide au cas où Windows envoie des guillemets
                arg = arg.trim_matches('"').to_string();
                
                if arg.starts_with("emuworld://") {
                    println!("[EmuWorld] Redirection du lien vers la fenêtre principale: {}", arg);
                    let _ = app.emit("oauth-callback", arg);
                }
            }
        }))
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            #[cfg(desktop)]
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                // Listen for deep-link URLs (emuworld://auth-callback?code=...)
                let handle = app.handle().clone();
                app.deep_link().on_open_url(move |event| {
                    if let Some(url) = event.urls().first() {
                        let url_str = url.to_string();
                        println!("[EmuWorld] Deep link received: {}", url_str);
                        // Emit to frontend
                        let _ = handle.emit("oauth-callback", url_str);
                    }
                });
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running EmuWorld");
}
