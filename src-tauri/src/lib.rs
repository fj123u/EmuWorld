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
        
        // Handle RetroArch cores if applicable
        if emu.id.starts_with("retroarch") {
            if let Some(core) = &emu.core_name {
                // Find the core file in the emulator directory
                if let Some(core_path) = find_executable(&install_dir, core) {
                    println!("[Launch] Detected RetroArch core: {:?}", core_path);
                    cmd.arg("-L");
                    cmd.arg(core_path);
                } else {
                    println!("[Launch] WARNING: Core '{}' not found in {}", core, install_dir.display());
                }
            }
        }
        
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

#[allow(dead_code)]
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
async fn fetch_boxart(game_name: String, console: String) -> Result<String, String> {
    let config = get_config();
    let covers_dir = PathBuf::from(&config.covers_directory);
    
    let libretro_systems: Vec<&str> = match console.as_str() {
        "NES" => vec!["Nintendo - Nintendo Entertainment System"],
        "Super Nintendo" => vec!["Nintendo - Super Nintendo Entertainment System"],
        "Nintendo 64" => vec!["Nintendo - Nintendo 64"],
        "Game Boy Advance" => vec!["Nintendo - Game Boy Advance"],
        "Nintendo DS" => vec!["Nintendo - Nintendo DS"],
        "GameCube" => vec!["Nintendo - GameCube"],
        "Wii" => vec!["Nintendo - Wii"],
        "GameCube / Wii" => vec!["Nintendo - Wii", "Nintendo - GameCube"],
        "Wii U" => vec!["Nintendo - Wii U"],
        "Nintendo Switch" => vec!["Nintendo - Nintendo Switch"],
        "Virtual Boy" => vec!["Nintendo - Virtual Boy"],
        "PlayStation 1" | "PS1" | "PSX" => vec!["Sony - PlayStation"],
        "PlayStation 2" | "PS2" => vec!["Sony - PlayStation 2"],
        "PlayStation 3" | "PS3" => vec!["Sony - PlayStation 3"],
        "PlayStation Portable" | "PSP" => vec!["Sony - PlayStation Portable"],
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
    
    let mut normalized_name = game_name.replace('_', " ").replace("  ", " ").trim().to_string();
    // Strip accents (é -> e, etc)
    normalized_name = normalized_name
        .replace('é', "e").replace('è', "e").replace('ê', "e").replace('ë', "e")
        .replace('à', "a").replace('â', "a")
        .replace('ô', "o").replace('û', "u").replace('ï', "i").replace('î', "i")
        .replace('ç', "c")
        .to_string();
    
    let lower_name = normalized_name.to_lowercase();
    if lower_name.ends_with(".chd") || lower_name.ends_with(".iso") || lower_name.ends_with(".rvz") || lower_name.ends_with(".wbfs") {
        normalized_name = normalized_name[..normalized_name.len()-4].to_string();
    }
    
    // Libretro naming conventions replacements
    let cleaned_for_search = normalized_name
        .replace(':', " -")
        .replace(" & ", " + ")
        .to_string();

    let forbidden = ['*', '/', '<', '>', '?', '\\', '|', '"']; // Removed & and : from forbidden to handle them in search variants
    let safe_name: String = normalized_name.chars()
        .map(|c| if forbidden.contains(&c) { '_' } else { c })
        .collect();
    
    let mut cleaned = cleaned_for_search; // Use the cleaned variant as base for candidates
    let tags = vec![" (USA)", " (Europe)", " (World)", " (Japan)", " (En,Fr,De)", " (En,Fr,Es)", " (Canada)", " (Italy)", " (Proto)"];
    for tag in &tags {
        cleaned = cleaned.replace(tag, "");
    }
    
    if let Some(bracket_pos) = cleaned.find('[') {
        cleaned = cleaned[..bracket_pos].trim().to_string();
    }
    if let Some(paren_pos) = cleaned.find('(') {
        cleaned = cleaned[..paren_pos].trim().to_string();
    }

    let mut candidates = vec![
        cleaned.clone(),
        format!("{} (USA)", cleaned),
        format!("{} (World)", cleaned),
        format!("{} (Europe)", cleaned),
        format!("{} (USA, Europe)", cleaned),
    ];

    // Try variants for titles with ampersands
    if cleaned.contains(" and ") {
        let plus = cleaned.replace(" and ", " + ");
        let amp = cleaned.replace(" and ", " & ");
        candidates.push(plus.clone());
        candidates.push(format!("{} (USA)", plus));
        candidates.push(format!("{} (Europe)", plus));
        candidates.push(amp.clone());
        candidates.push(format!("{} (USA)", amp));
        candidates.push(format!("{} (Europe)", amp));
    }
    
    if cleaned.to_lowercase().starts_with("the ") {
        let suffix = &cleaned[4..];
        let swapped = format!("{}, The", suffix);
        candidates.push(swapped.clone());
        candidates.push(format!("{} (USA)", swapped));
        candidates.push(format!("{} (World)", swapped));
        candidates.push(format!("{} (Europe)", swapped));
    }

    // 1. First check local covers directory with fuzzy matching
    let console_covers_dir = covers_dir.join(&console);
    if let Ok(entries) = std::fs::read_dir(&console_covers_dir) {
        for entry in entries.flatten() {
            if let Some(file_name) = entry.file_name().to_str() {
                let lower = file_name.to_lowercase();
                if lower.ends_with(".png") || lower.ends_with(".jpg") {
                    // Strip extension, strip bracket content like [01008F...][v0], then normalize
                    let name_no_ext = lower.rsplit_once('.').map(|(n,_)| n).unwrap_or(&lower);
                    // Remove everything in brackets: [titleid], [v0], (USA), etc.
                    let mut stripped = String::new();
                    let mut depth = 0;
                    for ch in name_no_ext.chars() {
                        match ch {
                            '[' | '(' => depth += 1,
                            ']' | ')' => { depth -= 1; },
                            _ if depth == 0 => stripped.push(ch),
                            _ => {}
                        }
                    }
                    // Also strip accents from the file name
                    let clean_file = stripped
                        .replace('é', "e").replace('è', "e").replace('ê', "e").replace('ë', "e")
                        .replace('à', "a").replace('â', "a").replace('ô', "o")
                        .replace('û', "u").replace('ï', "i").replace('î', "i").replace('ç', "c")
                        .replace(' ', "").replace('_', "").replace('+', "").replace('&', "").replace(':', "").replace('-', "").replace('.', "");
                    
                    // Also strip brackets/parens from the game name
                    let mut game_stripped = String::new();
                    let mut gdepth = 0;
                    for ch in normalized_name.to_lowercase().chars() {
                        match ch {
                            '[' | '(' => gdepth += 1,
                            ']' | ')' => { gdepth -= 1; },
                            _ if gdepth == 0 => game_stripped.push(ch),
                            _ => {}
                        }
                    }
                    let clean_game = game_stripped
                        .replace('é', "e").replace('è', "e").replace('ê', "e").replace('ë', "e")
                        .replace('à', "a").replace('â', "a").replace('ô', "o")
                        .replace('û', "u").replace('ï', "i").replace('î', "i").replace('ç', "c")
                        .replace('\u{FFFD}', "e").replace("__", "e") // Handle corrupted encoding
                        .replace(' ', "").replace('_', "").replace('+', "").replace('&', "").replace(':', "").replace('-', "").replace('.', "")
                        .to_lowercase();
                    
                    if !clean_file.is_empty() && clean_file == clean_game {
                        if let Ok(data) = std::fs::read(&entry.path()) {
                            use base64::Engine;
                            let ext = if lower.ends_with(".jpg") { "jpeg" } else { "png" };
                            let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                            return Ok(format!("data:image/{};base64,{}", ext, b64));
                        }
                    }
                }
            }
        }
    }
    
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("Mozilla/5.0")
        .build()
        .map_err(|e| e.to_string())?;

    for system in &libretro_systems {
        let encoded_system = urlencoding::encode(system);
        for candidate in &candidates {
            let encoded_name = urlencoding::encode(candidate);
            let url = format!("https://thumbnails.libretro.com/{}/Named_Boxarts/{}.png", encoded_system, encoded_name);
            if let Ok(response) = client.get(&url).send().await {
                if response.status().is_success() {
                    if let Ok(bytes) = response.bytes().await {
                        if bytes.len() > 500 {
                            let _ = std::fs::create_dir_all(&console_covers_dir);
                            let file_path = console_covers_dir.join(format!("{}.png", &safe_name));
                            if let Ok(mut file) = std::fs::File::create(&file_path) {
                                use std::io::Write;
                                let _ = file.write_all(&bytes);
                            }
                            use base64::Engine;
                            let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                            return Ok(format!("data:image/png;base64,{}", b64));
                        }
                    }
                }
            }
        }
    }
    
    Err("Boxart not found".to_string())
}

/// Extract a 16-character hex Title ID from a filename (e.g. "[0100000000010000]")
fn extract_title_id(name: &str) -> Option<String> {
    for part in name.split('[') {
        if let Some(bracket_end) = part.find(']') {
            let content = part[..bracket_end].trim().to_uppercase();
            if content.len() >= 16 {
                let chars: Vec<char> = content.chars().collect();
                for i in 0..=(chars.len() - 16) {
                    let potential: String = chars[i..i+16].iter().collect();
                    if potential.chars().all(|c| c.is_ascii_hexdigit()) {
                        return Some(potential);
                    }
                }
            }
        }
    }
    None
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
                let url = link.value().attr("href").unwrap_or("").to_string();
                
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

pub fn run() {
    tauri::Builder::default()
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
                let handle = app.handle().clone();
                app.deep_link().on_open_url(move |event| {
                    if let Some(url) = event.urls().first() {
                        let _ = handle.emit("oauth-callback", url.to_string());
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
            delete_rom,
            get_rgs_constructeurs,
            get_rgs_consoles,
            get_rgs_liens,
            search_rgs,
            scrape_1fichier_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error while running EmuWorld");
}
