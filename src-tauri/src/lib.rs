use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tauri::Emitter;
use reqwest;

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
                        installed.push(name.to_string());
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
    let config = get_config();
    let install_dir = PathBuf::from(&config.emulators_directory).join(&emulator_id);
    if install_dir.exists() {
        fs::remove_dir_all(&install_dir).map_err(|e| e.to_string())?;
        Ok(format!("Emulator '{}' uninstalled", emulator_id))
    } else {
        Err("Emulator not installed".to_string())
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
        // Normalize slashes to backslashes for Windows emulators 
        // while also handling potential forward slashes from JS
        let final_path = clean_rom.replace("/", "\\");
        cmd.arg(&final_path);
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
    
    // Libretro naming: replace characters forbidden in filenames with _
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
    
    // Build candidate names to try
    let mut candidates = vec![safe_name.clone()];
    
    // Try without region tags: "Super Mario Bros. 3 (USA)" -> "Super Mario Bros. 3"
    if let Some(paren_start) = safe_name.find('(') {
        let trimmed = safe_name[..paren_start].trim().to_string();
        if !trimmed.is_empty() {
            candidates.push(trimmed);
        }
    }
    if let Some(bracket_start) = safe_name.find('[') {
        let trimmed = safe_name[..bracket_start].trim().to_string();
        if !trimmed.is_empty() {
            candidates.push(trimmed);
        }
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;
    
    // Try each system and each candidate name
    for system in &libretro_systems {
        let encoded_system = urlencoding::encode(system);
        
        for candidate in &candidates {
            let encoded_name = urlencoding::encode(candidate);
            
            // For Nintendo Switch, we try shawnshyguy/Boxart with .png then .jpg
            let extensions = if console == "Nintendo Switch" {
                vec![".png", ".jpg"]
            } else {
                vec![".png"]
            };

            for ext in extensions {
                let url = if console == "Nintendo Switch" {
                    format!(
                        "https://raw.githubusercontent.com/shawnshyguy/Boxart/main/Nintendo%20-%20Switch/Boxart/Front-Boxart/{}{}",
                        encoded_name, ext
                    )
                } else {
                    format!(
                        "https://thumbnails.libretro.com/{}/Named_Boxarts/{}.png",
                        encoded_system, encoded_name
                    )
                };
                
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
    }
    
    Err("Boxart not found".to_string())
}


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
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
