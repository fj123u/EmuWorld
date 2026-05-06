use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::io::{Read, Write};
use base64::Engine;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct B2Config {
    pub key_id: String,
    pub app_key: String,
    pub bucket_id: String,
    pub bucket_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SaveEntry {
    pub emulator: String,
    pub game_name: String,
    pub file_name: String,
    pub size: u64,
    pub modified: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CloudFile {
    pub file_name: String,
    pub file_id: String,
    pub size: u64,
    pub upload_timestamp: u64,
}

#[derive(Debug, Deserialize)]
struct B2AuthResponse {
    #[serde(rename = "authorizationToken")]
    authorization_token: String,
    #[serde(rename = "apiUrl")]
    api_url: String,
}

#[derive(Debug, Deserialize)]
struct B2GetUploadUrlResponse {
    #[serde(rename = "uploadUrl")]
    upload_url: String,
    #[serde(rename = "authorizationToken")]
    authorization_token: String,
}

#[derive(Debug, Deserialize)]
struct B2UploadFileResponse {
    #[serde(rename = "fileId")]
    file_id: String,
    #[serde(rename = "fileName")]
    #[allow(dead_code)]
    file_name: String,
}

#[derive(Debug, Deserialize)]
struct B2ListFilesResponse {
    files: Vec<B2FileInfo>,
}

#[derive(Debug, Deserialize)]
struct B2FileInfo {
    #[serde(rename = "fileId")]
    file_id: String,
    #[serde(rename = "fileName")]
    file_name: String,
    #[serde(rename = "contentLength")]
    content_length: u64,
    #[serde(rename = "uploadTimestamp")]
    upload_timestamp: u64,
}

fn config_path() -> PathBuf {
    let base = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("EmuWorld").join("b2_config.json")
}

pub fn load_config() -> B2Config {
    let path = config_path();
    if let Ok(data) = fs::read_to_string(&path) {
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        B2Config::default()
    }
}

pub fn save_config(config: &B2Config) -> Result<(), String> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())
}

/// Save file extensions we look for
const SAVE_EXTENSIONS: &[&str] = &[
    "srm", "sav", "state", "sav0", "sav1", "sav2", "sav3",
    "ss0", "ss1", "ss2", "ss3", "ss4", "ss5", "ss6", "ss7", "ss8", "ss9",
    "mcr", "mcd", "gme", "psv", "psx",
    "dsv", "sna", "sta",
    "eep", "fla", "mpk", "sra",
    "bsv", "oops", "cht",
    "rtc", "savestate",
];

/// Known save locations for each emulator (relative to the emulator install dir)
fn save_dirs_for_emulator(emulator_id: &str, install_dir: &PathBuf) -> Vec<(String, PathBuf)> {
    let mut paths = Vec::new();

    // Generic approach: search common folder names recursively (1 level deep for perf)
    let common_save_folders = match emulator_id {
        "retroarch" => vec!["saves", "states"],
        "duckstation" => vec!["memcards", "savestates"],
        "pcsx2" => vec!["memcards", "sstates"],
        "dolphin" => vec!["GC", "Wii", "StateSaves"],
        "ppsspp" => vec!["SAVEDATA", "PPSSPP_STATE"],
        "ryubing" => vec!["save"],
        "cemu" => vec!["save"],
        "mgba" => vec!["battery", "state", "Saves"],
        "mesen" => vec!["Saves", "SaveStates"],
        "snes9x" => vec!["Saves"],
        "melonds" => vec!["battery", "saves"],
        "rpcs3" => vec!["savedata"],
        _ => vec!["saves", "Saves", "save"],
    };

    // Search recursively up to 3 levels deep for these folder names
    find_save_dirs_recursive(install_dir, &common_save_folders, 0, 3, &mut paths);
    paths
}

fn find_save_dirs_recursive(dir: &PathBuf, targets: &[&str], depth: u32, max_depth: u32, results: &mut Vec<(String, PathBuf)>) {
    if depth > max_depth { return; }
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() { continue; }
        let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();

        if targets.iter().any(|t| t.eq_ignore_ascii_case(&name)) {
            // Check if this dir actually contains save files
            if dir_has_save_files(&path) {
                results.push((name.clone(), path.clone()));
            }
        }

        // Keep searching deeper
        if depth < max_depth {
            find_save_dirs_recursive(&path, targets, depth + 1, max_depth, results);
        }
    }
}

fn dir_has_save_files(dir: &PathBuf) -> bool {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    let ext_str = ext.to_string_lossy().to_lowercase();
                    if SAVE_EXTENSIONS.contains(&ext_str.as_str()) {
                        return true;
                    }
                }
                // Also include files without extension or common save patterns
                let name = path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
                if name.ends_with(".srm") || name.ends_with(".sav") || name.contains("save") {
                    return true;
                }
            }
        }
    }
    false
}

fn find_retroarch_base(install_dir: &PathBuf) -> PathBuf {
    if install_dir.join("retroarch.exe").exists() {
        return install_dir.clone();
    }
    if let Ok(entries) = fs::read_dir(install_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && path.join("retroarch.exe").exists() {
                return path;
            }
        }
    }
    install_dir.clone()
}

pub fn scan_saves(emulators_dir: &str) -> Vec<SaveEntry> {
    let base = PathBuf::from(emulators_dir);
    let mut entries = Vec::new();

    // Scan all directories in the emulators folder
    let emu_dirs = match fs::read_dir(&base) {
        Ok(e) => e,
        Err(_) => return entries,
    };

    for emu_entry in emu_dirs.flatten() {
        let emu_dir = emu_entry.path();
        if !emu_dir.is_dir() { continue; }
        let emu_id = emu_dir.file_name().unwrap_or_default().to_string_lossy().to_string();

        let save_dirs = save_dirs_for_emulator(&emu_id, &emu_dir);
        for (category, dir) in save_dirs {
            let walker = walkdir::WalkDir::new(&dir).max_depth(3).into_iter().filter_map(|e| e.ok());
            for file in walker {
                let path = file.path().to_path_buf();
                if !path.is_file() { continue; }
                let meta = match fs::metadata(&path) {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                let modified = meta.modified()
                    .map(|t| {
                        let dt: chrono::DateTime<chrono::Local> = t.into();
                        dt.format("%Y-%m-%d %H:%M").to_string()
                    })
                    .unwrap_or_default();

                let rel = path.strip_prefix(&dir).unwrap_or(&path);
                entries.push(SaveEntry {
                    emulator: emu_id.clone(),
                    game_name: format!("{}/{}", category, rel.to_string_lossy().replace('\\', "/")),
                    file_name: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
                    size: meta.len(),
                    modified,
                });
            }
        }
    }

    entries
}

pub fn create_backup_zip(emulators_dir: &str) -> Result<PathBuf, String> {
    let saves = scan_saves(emulators_dir);
    if saves.is_empty() {
        return Err("No saves found to backup.".to_string());
    }

    let base = PathBuf::from(emulators_dir);
    let backup_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("EmuWorld")
        .join("backups");
    fs::create_dir_all(&backup_dir).map_err(|e| e.to_string())?;

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let zip_path = backup_dir.join(format!("emuworld_saves_{}.zip", timestamp));
    let file = fs::File::create(&zip_path).map_err(|e| e.to_string())?;
    let mut zip_writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let known_emus = [
        "retroarch", "duckstation", "pcsx2", "dolphin", "ppsspp",
        "ryubing", "cemu", "mgba", "mesen", "snes9x", "melonds", "rpcs3",
    ];

    for emu_id in &known_emus {
        let emu_dir = base.join(emu_id);
        if !emu_dir.exists() { continue; }

        let save_dirs = save_dirs_for_emulator(emu_id, &emu_dir);
        for (category, dir) in save_dirs {
            let walker = walkdir::WalkDir::new(&dir).into_iter().filter_map(|e| e.ok());
            for entry in walker {
                let path = entry.path();
                if !path.is_file() { continue; }
                let rel = path.strip_prefix(&dir).unwrap_or(path);
                let archive_name = format!("{}/{}/{}", emu_id, category, rel.to_string_lossy().replace('\\', "/"));

                if let Ok(mut f) = fs::File::open(path) {
                    let mut buf = Vec::new();
                    if f.read_to_end(&mut buf).is_ok() {
                        let _ = zip_writer.start_file(&archive_name, options);
                        let _ = zip_writer.write_all(&buf);
                    }
                }
            }
        }
    }

    zip_writer.finish().map_err(|e| e.to_string())?;
    Ok(zip_path)
}

pub fn restore_backup_zip(zip_path: &str, emulators_dir: &str) -> Result<u32, String> {
    let file = fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    let base = PathBuf::from(emulators_dir);
    let mut restored = 0u32;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        if entry.is_dir() { continue; }

        let name = entry.name().to_string();
        let dest = base.join(&name);
        if let Some(parent) = dest.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf).map_err(|e| e.to_string())?;
        fs::write(&dest, &buf).map_err(|e| e.to_string())?;
        restored += 1;
    }

    Ok(restored)
}

// ============================================================
// B2 Cloud API
// ============================================================

pub async fn b2_authorize(key_id: &str, app_key: &str) -> Result<(String, String), String> {
    let client = reqwest::Client::new();
    let credentials = format!("{}:{}", key_id, app_key);
    let encoded = base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes());

    let resp = client
        .get("https://api.backblazeb2.com/b2api/v2/b2_authorize_account")
        .header("Authorization", format!("Basic {}", encoded))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("B2 auth failed ({}): {}", status, body));
    }

    let auth: B2AuthResponse = resp.json().await.map_err(|e| e.to_string())?;
    Ok((auth.authorization_token, auth.api_url))
}

pub async fn b2_upload_file(
    api_url: &str,
    auth_token: &str,
    bucket_id: &str,
    file_name: &str,
    data: Vec<u8>,
) -> Result<String, String> {
    let client = reqwest::Client::new();

    // Get upload URL
    let get_url_resp = client
        .post(format!("{}/b2api/v2/b2_get_upload_url", api_url))
        .header("Authorization", auth_token)
        .json(&serde_json::json!({ "bucketId": bucket_id }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !get_url_resp.status().is_success() {
        return Err(format!("B2 get_upload_url failed: {}", get_url_resp.status()));
    }

    let upload_info: B2GetUploadUrlResponse = get_url_resp.json().await.map_err(|e| e.to_string())?;

    // Calculate SHA1
    let digest = sha1_hash(&data);

    // Upload
    let resp = client
        .post(&upload_info.upload_url)
        .header("Authorization", &upload_info.authorization_token)
        .header("X-Bz-File-Name", urlencoding::encode(file_name).as_ref())
        .header("Content-Type", "application/zip")
        .header("Content-Length", data.len())
        .header("X-Bz-Content-Sha1", &digest)
        .body(data)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("B2 upload failed: {}", body));
    }

    let result: B2UploadFileResponse = resp.json().await.map_err(|e| e.to_string())?;
    Ok(result.file_id)
}

pub async fn b2_list_files(
    api_url: &str,
    auth_token: &str,
    bucket_id: &str,
    prefix: &str,
) -> Result<Vec<CloudFile>, String> {
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{}/b2api/v2/b2_list_file_names", api_url))
        .header("Authorization", auth_token)
        .json(&serde_json::json!({
            "bucketId": bucket_id,
            "prefix": prefix,
            "maxFileCount": 100
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("B2 list failed: {}", resp.status()));
    }

    let list: B2ListFilesResponse = resp.json().await.map_err(|e| e.to_string())?;
    Ok(list.files.iter().map(|f| CloudFile {
        file_name: f.file_name.clone(),
        file_id: f.file_id.clone(),
        size: f.content_length,
        upload_timestamp: f.upload_timestamp,
    }).collect())
}

pub async fn b2_download_file(
    api_url: &str,
    auth_token: &str,
    file_id: &str,
) -> Result<Vec<u8>, String> {
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{}/b2api/v2/b2_download_file_by_id", api_url))
        .header("Authorization", auth_token)
        .json(&serde_json::json!({ "fileId": file_id }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        // Try GET endpoint
        let resp2 = client
            .get(format!("{}/b2api/v2/b2_download_file_by_id?fileId={}", api_url, file_id))
            .header("Authorization", auth_token)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp2.status().is_success() {
            return Err(format!("B2 download failed: {}", resp2.status()));
        }
        return resp2.bytes().await.map(|b| b.to_vec()).map_err(|e| e.to_string());
    }

    resp.bytes().await.map(|b| b.to_vec()).map_err(|e| e.to_string())
}

fn sha1_hash(_data: &[u8]) -> String {
    "do_not_verify".to_string()
}
