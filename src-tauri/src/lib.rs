use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tauri::Emitter;

mod emulators;

/// App configuration stored as JSON
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub roms_directory: String,
    pub emulators_directory: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        let base = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("EmuWorld");
        Self {
            roms_directory: base.join("ROMs").to_string_lossy().to_string(),
            emulators_directory: base.join("Emulators").to_string_lossy().to_string(),
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

    // Auto-create ROMs and Emulators directories
    fs::create_dir_all(&config.roms_directory).ok();
    fs::create_dir_all(&config.emulators_directory).ok();

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

    // Clean up any previous failed install
    if install_dir.exists() {
        fs::remove_dir_all(&install_dir).ok();
    }
    fs::create_dir_all(&install_dir).map_err(|e| format!("Failed to create directory: {}", e))?;

    let _ = app_handle.emit("install-progress", serde_json::json!({
        "emulator_id": emulator_id,
        "status": "downloading",
        "progress": 10
    }));

    // Download with full redirect support
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

    let status = response.status();
    if !status.is_success() {
        return Err(format!("Download failed with HTTP status: {}", status));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read download data: {}", e))?;

    if bytes.is_empty() {
        return Err("Downloaded file is empty".to_string());
    }

    let _ = app_handle.emit("install-progress", serde_json::json!({
        "emulator_id": emulator_id,
        "status": "extracting",
        "progress": 60
    }));

    // Determine archive type and extract
    let archive_ext = if emu.archive_type == "7z" { "7z" } else { "zip" };
    let archive_filename = format!("archive.{}", archive_ext);
    let archive_path = install_dir.join(&archive_filename);
    fs::write(&archive_path, &bytes).map_err(|e| format!("Failed to save archive: {}", e))?;

    if emu.archive_type == "zip" {
        extract_zip(&archive_path, &install_dir)?;
    } else if emu.archive_type == "7z" {
        extract_7z(&archive_path, &install_dir)?;
    }

    // Clean up archive file
    fs::remove_file(&archive_path).ok();

    let exe_found = find_executable(&install_dir, &emu.executable_name).is_some();

    // Auto-create a ROM subfolder for this console
    let roms_dir = PathBuf::from(&config.roms_directory).join(&emu.console);
    fs::create_dir_all(&roms_dir).ok();

    let _ = app_handle.emit("install-progress", serde_json::json!({
        "emulator_id": emulator_id,
        "status": "done",
        "progress": 100
    }));

    if exe_found {
        Ok(format!("{} installed successfully!", emu.name))
    } else {
        Ok(format!("{} files extracted. The executable '{}' was not found — you may need to configure it manually.", emu.name, emu.executable_name))
    }
}

fn extract_zip(archive_path: &PathBuf, install_dir: &PathBuf) -> Result<(), String> {
    let file = fs::File::open(archive_path)
        .map_err(|e| format!("Failed to open archive: {}", e))?;

    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("Invalid ZIP archive: {}", e))?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)
            .map_err(|e| format!("Failed to read entry {}: {}", i, e))?;

        let outpath = match entry.enclosed_name() {
            Some(name) => install_dir.join(name),
            None => continue,
        };

        if entry.is_dir() {
            fs::create_dir_all(&outpath).ok();
        } else {
            if let Some(p) = outpath.parent() {
                fs::create_dir_all(p).ok();
            }
            let mut outfile = fs::File::create(&outpath)
                .map_err(|e| format!("Failed to create file {:?}: {}", outpath, e))?;
            std::io::copy(&mut entry, &mut outfile)
                .map_err(|e| format!("Failed to write file {:?}: {}", outpath, e))?;
        }
    }

    Ok(())
}

fn extract_7z(archive_path: &PathBuf, install_dir: &PathBuf) -> Result<(), String> {
    sevenz_rust::decompress_file(archive_path, install_dir)
        .map_err(|e| format!("7z extraction failed: {}", e))?;
    Ok(())
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
    let emu = catalog
        .iter()
        .find(|e| e.id == emulator_id)
        .ok_or_else(|| "Emulator not found".to_string())?;

    let config = get_config();
    let install_dir = PathBuf::from(&config.emulators_directory).join(&emu.id);

    let exe_path = find_executable(&install_dir, &emu.executable_name)
        .ok_or_else(|| format!(
            "Executable '{}' not found. Try reinstalling the emulator.",
            emu.executable_name
        ))?;

    let mut cmd = Command::new(&exe_path);
    cmd.current_dir(exe_path.parent().unwrap_or(&install_dir));

    if let Some(rom) = rom_path {
        cmd.arg(&rom);
    }

    cmd.spawn().map_err(|e| format!("Failed to launch: {}", e))?;

    Ok(format!("Launched {}", emu.name))
}

fn find_executable(dir: &PathBuf, name: &str) -> Option<PathBuf> {
    let direct = dir.join(name);
    if direct.exists() {
        return Some(direct);
    }

    // Search recursively up to depth 5
    for entry in walkdir::WalkDir::new(dir).max_depth(5) {
        if let Ok(e) = entry {
            if e.file_type().is_file() {
                let file_name = e.file_name().to_string_lossy();
                if file_name == name {
                    return Some(e.path().to_path_buf());
                }
            }
        }
    }

    // Fallback: look for any .exe file
    for entry in walkdir::WalkDir::new(dir).max_depth(5) {
        if let Ok(e) = entry {
            if e.file_type().is_file() {
                let file_name = e.file_name().to_string_lossy().to_lowercase();
                if file_name.ends_with(".exe") && !file_name.contains("uninstall") {
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
    if !dir.exists() {
        return roms;
    }

    for entry in walkdir::WalkDir::new(&dir).max_depth(5) {
        if let Ok(e) = entry {
            if e.file_type().is_file() {
                if let Some(ext) = e.path().extension() {
                    let ext_str = ext.to_string_lossy().to_lowercase();
                    if let Some((console, emu_id)) = match_extension(&ext_str, &catalog) {
                        let name = e
                            .path()
                            .file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_default();
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
            if supported_ext == ext {
                return Some((emu.console.clone(), emu.id.clone()));
            }
        }
    }
    None
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running EmuWorld");
}
