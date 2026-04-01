use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tauri::Emitter;
use reqwest;
use urlencoding;

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
        let final_path = rom.replace(r"\\?\", "").replace("/", "\\");
        println!("[Launch] Running: {:?} with Arg: {:?}", exe_path, final_path);
        cmd.arg(&final_path);
    }
    
    // Use CREATE_NO_WINDOW (0x08000000) to separate the emulator but keep console handles
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW (0x08000000) provides a console but no window,
        // preventing "Invalid Handle" crashes in apps like Ryujinx.
        cmd.creation_flags(0x08000000);
    }

    match cmd.spawn() {
        Ok(_) => {
            println!("[Launch] Success!");
            Ok(format!("Launched {}", emu.name))
        },
        Err(e) => {
            println!("[Launch] ERROR spawning process: {}", e);
            Err(format!("Could not start emulator: {}", e))
        }
    }
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
        "Mixed" | "Multiple" => vec![
            "Nintendo - Nintendo Entertainment System",
            "Nintendo - Super Nintendo Entertainment System",
            "Sega - Mega Drive - Genesis",
            "Sony - PlayStation",
            "Nintendo - Nintendo 64",
            "Nintendo - Game Boy Advance",
            "Nintendo - Nintendo DS",
            "Sega - Dreamcast",
            "Nintendo - Game Boy",
        ],
        _ => return Err(format!("No cover source for console: {}", console)),
    };
    
    // Normalize name for better matching: replace underscores with spaces, collapse spaces
    let normalized_name = game_name.replace('_', " ").replace("  ", " ").trim().to_string();
    
    // Libretro naming: ONLY replace chars truly forbidden in URLs/filenames
    // Keep apostrophes, parentheses, commas — libretro uses them!
    let forbidden = ['&', '*', '/', ':', '<', '>', '?', '\\', '|', '"'];
    let safe_name: String = normalized_name.chars()
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
// ═══════════════════════════════════════════════════════════════
//  ROM STORE — Search and Download ROMs
// ═══════════════════════════════════════════════════════════════

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

fn sanitize_filename(name: &str) -> String {
    let mut s = name.to_string();
    let invalid_chars = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
    for &c in &invalid_chars {
        s = s.replace(c, "");
    }
    // Windows filenames cannot end in a space or a dot
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

fn detect_console_from_title(title: &str) -> Option<String> {
    let t = title.to_lowercase();
    
    // 1. Precise strings
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

    // 2. Franchise Fallback (Cult names that are usually on specific systems)
    if t.contains("super mario bros") && !t.contains("wii") && !t.contains("switch") { 
        if t.contains(" 3") || t.contains(" 2") || t.contains("lost levels") { return Some("NES".to_string()); }
        if t.contains("world") { return Some("Super Nintendo".to_string()); }
        return Some("NES".to_string()); 
    }
    if t.contains("sonic the hedgehog") && !t.contains("adv") && !t.contains("2006") { return Some("Sega Genesis".to_string()); }
    if t.contains("legend of zelda") {
        if t.contains("ocarina") || t.contains("majora") { return Some("Nintendo 64".to_string()); }
        if t.contains("link to the past") { return Some("Super Nintendo".to_string()); }
        return Some("NES".to_string());
    }
    if t.contains("pokemon") || t.contains("pokémon") {
        if t.contains("ruby") || t.contains("sapphire") || t.contains("emerald") || t.contains("firered") { return Some("Game Boy Advance".to_string()); }
        if t.contains("red") || t.contains("blue") || t.contains("yellow") || t.contains("gold") || t.contains("silver") { return Some("Game Boy".to_string()); }
    }

    None
}

#[tauri::command]
async fn search_rom_store(query: String, console_filter: Option<String>) -> Result<Vec<RomStoreEntry>, String> {
    let mut results = Vec::new();
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    println!("[Store] Search triggered - Query: '{}', Console: {:?}", query, console_filter);

    let (q, sort_param, final_console) = if let Some(console) = console_filter {
        if let Some(collection) = get_ia_collection(&console) {
            if query.is_empty() {
                (format!("collection:({})", collection), "&sort[]=downloads%20desc", console)
            } else {
                (format!("collection:({}) AND title:(\"{}\")", collection, query), "", console)
            }
        } else {
            return Err(format!("Unknown console: {}", console));
        }
    } else {
        // GLOBAL SEARCH - empty filter
        if query.is_empty() {
            // New robust query excluding packs and collections to get individual games
            let q_str = "mediatype:software AND (subject:rom OR subject:redump OR subject:no-intro) AND (subject:nintendo OR subject:sony OR subject:sega) AND downloads:[1000 TO *] AND NOT title:(part OR bios OR set OR merged OR pack OR collection OR bundle OR \"rom pack\" OR \"rom set\" OR roms OR \"iso set\" OR \"romset\")";
            (q_str.to_string(), "&sort[]=downloads%20desc", "Multiple".to_string())
        } else {
            // General ROM search across IA excluding bundles
            (format!("(rom OR emulator OR game) AND mediatype:software AND title:(\"{}\") AND NOT title:(pack OR bundle OR collection OR romset OR roms)", query), "&sort[]=downloads%20desc", "Mixed".to_string())
        }
    };
    
    let url = format!(
        "https://archive.org/advancedsearch.php?q={}&fl[]=identifier,title,description,collection,subject&rows=150{}&output=json",
        urlencoding::encode(&q),
        sort_param
    );
    
    println!("[Store] IA Request: {}", url);
    
    let response = client.get(&url).send().await.map_err(|e| e.to_string())?;
    
    if !response.status().is_success() {
        println!("[Store] IA Error: {}", response.status());
        return Err(format!("Archive.org API error: {}", response.status()));
    }
    
    let json: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
    
    if let Some(docs) = json["response"]["docs"].as_array() {
        println!("[Store] Found {} items", docs.len());
        for doc in docs {
            let title = doc["title"].as_str().unwrap_or("Unknown").to_string();
            // Determine console from collection or subject
            let mut entry_console = if final_console == "Mixed" || final_console == "Multiple" {
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

            // Fallback: Detect from title if still "Mixed" or "Multiple"
            if entry_console == "Mixed" || entry_console == "Multiple" {
                if let Some(detected) = detect_console_from_title(&title) {
                    entry_console = detected;
                }
            }

            let entry_id = doc["identifier"].as_str().unwrap_or("").to_string();
            // Try to find a better thumbnail url (preview image)
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
    } else {
        println!("[Store] No docs found in response");
    }
    
    // Final fallback if IA fails us entirely
    if results.is_empty() && query.is_empty() {
        results = get_rom_catalog();
    }
    
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
            file_name: "Legend_of_Zelda_The_Ocarina_of_Time_USA_En_Fr_De.zip".to_string(),
            download_url: "https://archive.org/download/Legend_of_Zelda_The_Ocarina_of_Time_USA_En_Fr_De/Legend_of_Zelda_The_Ocarina_of_Time_USA_En_Fr_De.zip".to_string(),
            ia_id: Some("Legend_of_Zelda_The_Ocarina_of_Time_USA_En_Fr_De".to_string()), 
            thumbnail_url: Some("https://archive.org/services/img/Legend_of_Zelda_The_Ocarina_of_Time_USA_En_Fr_De?&height=320".to_string()),
        },
        RomStoreEntry {
            id: "sm64".to_string(),
            name: "Super Mario 64".to_string(),
            console: "Nintendo 64".to_string(),
            region: "World".to_string(),
            size: "8 MB".to_string(),
            file_name: "super-mario-64_n64.zip".to_string(),
            download_url: "https://archive.org/download/super-mario-64_n64/super-mario-64_n64.zip".to_string(),
            ia_id: Some("super-mario-64_n64".to_string()),
            thumbnail_url: Some("https://archive.org/services/img/super-mario-64_n64?&height=320".to_string()),
        },
        RomStoreEntry {
            id: "sonic-adv".to_string(),
            name: "Sonic Adventure".to_string(),
            console: "Dreamcast".to_string(),
            region: "World".to_string(),
            size: "1 GB".to_string(),
            file_name: "Sonic_Adventure_1999_U.zip".to_string(),
            download_url: "https://archive.org/download/Sonic_Adventure_1999_U/Sonic_Adventure_1999_U.zip".to_string(),
            ia_id: Some("Sonic_Adventure_1999_U".to_string()),
            thumbnail_url: Some("https://archive.org/services/img/Sonic_Adventure_1999_U?&height=320".to_string()),
        },
        RomStoreEntry {
            id: "m-kart-wii".to_string(),
            name: "Mario Kart Wii".to_string(),
            console: "GameCube / Wii".to_string(),
            region: "World".to_string(),
            size: "4.3 GB".to_string(),
            file_name: "mario-kart_202107.zip".to_string(),
            download_url: "https://archive.org/download/mario-kart_202107/mario-kart_202107.zip".to_string(),
            ia_id: Some("mario-kart_202107".to_string()),
            thumbnail_url: Some("https://archive.org/services/img/mario-kart_202107?&height=320".to_string()),
        },
        RomStoreEntry {
            id: "pkmn-em".to_string(),
            name: "Pokémon Emerald Version".to_string(),
            console: "Game Boy Advance".to_string(),
            region: "World".to_string(),
            size: "16 MB".to_string(),
            file_name: "pokemon-emerald-version_202308.zip".to_string(),
            download_url: "https://archive.org/download/pokemon-emerald-version_202308/pokemon-emerald-version_202308.zip".to_string(),
            ia_id: Some("pokemon-emerald-version_202308".to_string()),
            thumbnail_url: Some("https://archive.org/services/img/pokemon-emerald-version_202308?&height=320".to_string()),
        },
        RomStoreEntry {
            id: "halo-ce".to_string(),
            name: "Halo: Combat Evolved".to_string(),
            console: "Xbox".to_string(),
            region: "World".to_string(),
            size: "3.5 GB".to_string(),
            file_name: "halo-combat-evolved_202101.zip".to_string(),
            download_url: "https://archive.org/download/halo-combat-evolved_202101/halo-combat-evolved_202101.zip".to_string(),
            ia_id: Some("halo-combat-evolved_202101".to_string()),
            thumbnail_url: Some("https://archive.org/services/img/halo-combat-evolved_202101?&height=320".to_string()),
        },
        RomStoreEntry {
            id: "gta-sa".to_string(),
            name: "Grand Theft Auto: San Andreas".to_string(),
            console: "PlayStation 2".to_string(),
            region: "World".to_string(),
            size: "4 GB".to_string(),
            file_name: "grand-theft-auto-san-andreas-utilities.zip".to_string(),
            download_url: "https://archive.org/download/grand-theft-auto-san-andreas-utilities/grand-theft-auto-san-andreas-utilities.zip".to_string(),
            ia_id: Some("grand-theft-auto-san-andreas-utilities".to_string()),
            thumbnail_url: Some("https://archive.org/services/img/grand-theft-auto-san-andreas-utilities?&height=320".to_string()),
        },
        RomStoreEntry {
            id: "pkmn-plt".to_string(),
            name: "Pokémon Platinum Version".to_string(),
            console: "Nintendo DS".to_string(),
            region: "World".to_string(),
            size: "128 MB".to_string(),
            file_name: "pokemon-platinum-version-nintendods-hiresscans.zip".to_string(),
            download_url: "https://archive.org/download/pokemon-platinum-version-nintendods-hiresscans/pokemon-platinum-version-nintendods-hiresscans.zip".to_string(),
            ia_id: Some("pokemon-platinum-version-nintendods-hiresscans".to_string()),
            thumbnail_url: Some("https://archive.org/services/img/pokemon-platinum-version-nintendods-hiresscans?&height=320".to_string()),
        },
        RomStoreEntry {
            id: "metroid-pr".to_string(),
            name: "Metroid Prime".to_string(),
            console: "GameCube / Wii".to_string(),
            region: "World".to_string(),
            size: "1.4 GB".to_string(),
            file_name: "metroid-prime-remastered.zip".to_string(),
            download_url: "https://archive.org/download/metroid-prime-remastered/metroid-prime-remastered.zip".to_string(),
            ia_id: Some("metroid-prime-remastered".to_string()),
            thumbnail_url: Some("https://archive.org/services/img/metroid-prime-remastered?&height=320".to_string()),
        },
        RomStoreEntry {
            id: "crash-3".to_string(),
            name: "Crash Bandicoot 3: Warped".to_string(),
            console: "PlayStation 1".to_string(),
            region: "World".to_string(),
            size: "500 MB".to_string(),
            file_name: "psx_crash3.zip".to_string(),
            download_url: "https://archive.org/download/psx_crash3/psx_crash3.zip".to_string(),
            ia_id: Some("psx_crash3".to_string()),
            thumbnail_url: Some("https://archive.org/services/img/psx_crash3?&height=320".to_string()),
        },
        RomStoreEntry {
            id: "nsmbw".to_string(),
            name: "New Super Mario Bros. Wii".to_string(),
            console: "GameCube / Wii".to_string(),
            region: "World".to_string(),
            size: "4 GB".to_string(),
            file_name: "new-super-mario-bros.-wii.zip".to_string(),
            download_url: "https://archive.org/download/new-super-mario-bros.-wii/new-super-mario-bros.-wii.zip".to_string(),
            ia_id: Some("new-super-mario-bros.-wii".to_string()),
            thumbnail_url: Some("https://archive.org/services/img/new-super-mario-bros.-wii?&height=320".to_string()),
        }
    ]
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

#[tauri::command]
async fn download_rom(
    download_url_arg: String, // Might be empty for IA
    console: String,
    rom_name: String,        // NEW: Used for human-readable filenames
    file_name_arg: String,   // Might be empty for IA
    ia_id: Option<String>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    println!("[Download] Starting - Console: {}, IA ID: {:?}", console, ia_id);
    let config = get_config();
    let folder = if console == "Mixed" || console == "Multiple" || console.is_empty() {
        "Downloads".to_string()
    } else {
        console_to_folder(&console).to_string()
    };
    let rom_dir = PathBuf::from(&config.roms_directory).join(folder);
    
    fs::create_dir_all(&rom_dir).map_err(|e| format!("Failed to create ROM directory: {}", e))?;
    
    let mut final_url = if download_url_arg.is_empty() { "".to_string() } else { download_url_arg };
    let mut final_file_name = if file_name_arg.is_empty() { "game.bin".to_string() } else { file_name_arg };
    
    // If we have an ia_id, we need to resolve the best file first
    if let Some(ref id) = ia_id {
        println!("[Download] Resolving IA files for item: {}", id);
        let _ = app_handle.emit("rom-download-progress", serde_json::json!({
            "file_id": id,
            "status": "resolving",
            "progress": 5
        }));
        
        let meta_url = format!("https://archive.org/metadata/{}", id);
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .build()
            .map_err(|e| e.to_string())?;
        
        let response = client.get(&meta_url).send().await.map_err(|e| e.to_string())?;
        let json: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
        
        if let Some(files) = json["files"].as_array() {
            // Find best file based on extensions
            let extensions = match console.as_str() {
                "NES" => vec![".nes"],
                "Super Nintendo" => vec![".sfc", ".smc"],
                "Nintendo 64" => vec![".z64", ".v64", ".n64"],
                "Game Boy Advance" => vec![".gba"],
                "Nintendo DS" => vec![".nds"],
                "Game Boy" => vec![".gb", ".gbc"],
                "GameCube / Wii" => vec![".rvz", ".wbfs", ".iso"],
                "Wii U" => vec![".wua", ".wud", ".wux"],
                "Nintendo Switch" => vec![".nsp", ".xci"],
                "PlayStation 1" => vec![".chd", ".pbp", ".bin", ".iso"],
                "PlayStation 2" => vec![".chd", ".iso"],
                "PlayStation 3" => vec![".iso", ".pkg"],
                "PlayStation Portable" => vec![".iso", ".cso"],
                "Dreamcast" => vec![".chd", ".gdi", ".cdi"],
                "Sega Genesis" => vec![".md", ".gen", ".bin"],
                _ => vec![".zip", ".7z", ".rom", ".bin"],
            };
            
            let mut best_file = None;
            for file in files {
                let name = file["name"].as_str().unwrap_or("");
                let size = file["size"].as_str().unwrap_or("0").parse::<u64>().unwrap_or(0);
                
                // Skip small files (nfo, txt, xml) - ROMs are usually > 10KB
                if size < 10000 && !name.ends_with(".nes") && !name.ends_with(".gba") {
                    continue;
                }

                if extensions.iter().any(|ext| name.to_lowercase().ends_with(ext)) {
                    best_file = Some(name.to_string());
                    println!("[Download] Selected file: {} (Size: {} bytes)", name, size);
                    break;
                }
            }
            
            if let Some(f) = best_file {
                // Determine original extension
                let ext = std::path::Path::new(&f)
                    .extension()
                    .and_then(|ex| ex.to_str())
                    .unwrap_or("bin");
                
                // Smart Renaming: "New Super Mario Bros. Wii.wbfs" instead of "SMNE01.wbfs"
                let safe_base = sanitize_filename(&rom_name);
                final_file_name = format!("{}.{}", safe_base, ext);
                
                // CRITICAL FIX: URL Encode the filename part to handle spaces and brackets
                let encoded_f = urlencoding::encode(&f);
                final_url = format!("https://archive.org/download/{}/{}", id, encoded_f);
                println!("[Download] Resolved to human-readable: {} (Encoded source: {})", final_file_name, final_url);
            } else {
                return Err("Could not find a compatible ROM file in this collection. Try another result.".to_string());
            }
        } else {
            return Err("No files found in IA metadata.".to_string());
        }
    }
    
    let dest = rom_dir.join(&final_file_name);
    println!("[Download] Full target path: {:?}", dest);
    
    // Check if already downloaded
    if dest.exists() {
        return Ok(format!("{} is already downloaded!", final_file_name));
    }
    
    let _ = app_handle.emit("rom-download-progress", serde_json::json!({
        "file_name": final_file_name,
        "status": "downloading",
        "progress": 10
    }));
    
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(std::time::Duration::from_secs(3600)) // 1 hour for big games
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;
    
    let mut response = client
        .get(&final_url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .send()
        .await
        .map_err(|e| format!("Download failed: {}", e))?;
    
    if !response.status().is_success() {
        return Err(format!("Download failed with HTTP {}", response.status()));
    }
    
    let total_size = response.content_length().unwrap_or(0);
    let mut downloaded_size: u64 = 0;
    let mut last_emit = std::time::Instant::now();
    
    let mut file = fs::File::create(&dest).map_err(|e| format!("Failed to create file: {}", e))?;
    use std::io::Write;
    
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        file.write_all(&chunk).map_err(|e| e.to_string())?;
        downloaded_size += chunk.len() as u64;
        
        if last_emit.elapsed().as_millis() > 500 {
            let progress = if total_size > 0 {
                (downloaded_size as f64 / total_size as f64 * 100.0) as u32
            } else {
                50
            };
            
            let _ = app_handle.emit("rom-download-progress", serde_json::json!({
                "file_name": final_file_name,
                "status": "downloading",
                "progress": progress,
                "downloaded": downloaded_size,
                "total": total_size
            }));
            last_emit = std::time::Instant::now();
        }
    }
    
    let _ = app_handle.emit("rom-download-progress", serde_json::json!({
        "file_name": final_file_name,
        "status": "done",
        "progress": 100
    }));
    
    Ok(format!("{} downloaded successfully to {}", final_file_name, dest.display()))
}

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
            search_rom_store,
            get_store_consoles,
            get_featured_games,
            download_rom,
        ])
        .run(tauri::generate_context!())
        .expect("error while running EmuWorld");
}
