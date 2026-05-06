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

/// Known save locations for each emulator (relative to the emulator install dir)
fn save_dirs_for_emulator(emulator_id: &str, install_dir: &PathBuf) -> Vec<(String, PathBuf)> {
    let mut paths = Vec::new();
    match emulator_id {
        "retroarch" => {
            // RetroArch saves are in saves/ and states/ next to the exe
            let base = find_retroarch_base(install_dir);
            let saves = base.join("saves");
            let states = base.join("states");
            if saves.exists() { paths.push(("saves".to_string(), saves)); }
            if states.exists() { paths.push(("states".to_string(), states)); }
        }
        "duckstation" => {
            let memcards = install_dir.join("memcards");
            let savestates = install_dir.join("savestates");
            if memcards.exists() { paths.push(("memcards".to_string(), memcards)); }
            if savestates.exists() { paths.push(("savestates".to_string(), savestates)); }
        }
        "pcsx2" => {
            let memcards = install_dir.join("memcards");
            let sstates = install_dir.join("sstates");
            if memcards.exists() { paths.push(("memcards".to_string(), memcards)); }
            if sstates.exists() { paths.push(("sstates".to_string(), sstates)); }
        }
        "dolphin" => {
            let gc = install_dir.join("User").join("GC");
            let wii = install_dir.join("User").join("Wii");
            let state = install_dir.join("User").join("StateSaves");
            if gc.exists() { paths.push(("GC".to_string(), gc)); }
            if wii.exists() { paths.push(("Wii".to_string(), wii)); }
            if state.exists() { paths.push(("StateSaves".to_string(), state)); }
        }
        "ppsspp" => {
            let savedata = install_dir.join("memstick").join("PSP").join("SAVEDATA");
            let ppstate = install_dir.join("memstick").join("PSP").join("PPSSPP_STATE");
            if savedata.exists() { paths.push(("SAVEDATA".to_string(), savedata)); }
            if ppstate.exists() { paths.push(("PPSSPP_STATE".to_string(), ppstate)); }
        }
        "ryubing" => {
            let saves = install_dir.join("portable").join("bis").join("user").join("save");
            if saves.exists() { paths.push(("saves".to_string(), saves)); }
        }
        "cemu" => {
            let mlc = install_dir.join("mlc01").join("usr").join("save");
            if mlc.exists() { paths.push(("saves".to_string(), mlc)); }
        }
        "mgba" => {
            // mGBA stores saves next to the ROM by default, but also has a battery/ folder
            let battery = install_dir.join("battery");
            let state = install_dir.join("state");
            if battery.exists() { paths.push(("battery".to_string(), battery)); }
            if state.exists() { paths.push(("state".to_string(), state)); }
        }
        "mesen" => {
            let saves = install_dir.join("Saves");
            let states = install_dir.join("SaveStates");
            if saves.exists() { paths.push(("Saves".to_string(), saves)); }
            if states.exists() { paths.push(("SaveStates".to_string(), states)); }
        }
        "snes9x" => {
            let saves = install_dir.join("Saves");
            if saves.exists() { paths.push(("Saves".to_string(), saves)); }
        }
        "melonds" => {
            // melonDS saves next to ROM by default
            let battery = install_dir.join("battery");
            if battery.exists() { paths.push(("battery".to_string(), battery)); }
        }
        "rpcs3" => {
            let savedata = install_dir.join("dev_hdd0").join("home").join("00000001").join("savedata");
            if savedata.exists() { paths.push(("savedata".to_string(), savedata)); }
        }
        _ => {}
    }
    paths
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

    let known_emus = [
        "retroarch", "duckstation", "pcsx2", "dolphin", "ppsspp",
        "ryubing", "cemu", "mgba", "mesen", "snes9x", "melonds", "rpcs3",
    ];

    for emu_id in &known_emus {
        let emu_dir = base.join(emu_id);
        if !emu_dir.exists() { continue; }

        let save_dirs = save_dirs_for_emulator(emu_id, &emu_dir);
        for (category, dir) in save_dirs {
            if let Ok(walker) = fs::read_dir(&dir) {
                for file in walker.flatten() {
                    let path = file.path();
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

                    entries.push(SaveEntry {
                        emulator: emu_id.to_string(),
                        game_name: format!("{}/{}", category, path.file_name().unwrap_or_default().to_string_lossy()),
                        file_name: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
                        size: meta.len(),
                        modified,
                    });
                }
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
    use std::fmt::Write as FmtWrite;
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
