//! RetroArch save states manager: lists the states of one game with their
//! thumbnail, gives them a name, duplicates them to a free slot and sends
//! them to the recycle bin.
//!
//! RetroArch names a state after the content file: `<rom stem>.state` (slot
//! 0), `<rom stem>.state<N>` (slot N) and `<rom stem>.state.auto` (auto
//! save), plus an optional `<state file>.png` thumbnail. Depending on its
//! "sort by core / by content" options they sit in `states/`,
//! `states/<core>/`, `states/<content dir>/` or `states/<core>/<content dir>/`.
//!
//! Names given in EmuWorld are kept in `savestate_labels.json` instead of
//! renaming the files, so RetroArch still finds every slot.

use base64::Engine;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::push_log;

/// Highest slot RetroArch's menu can reach.
const MAX_SLOT: u32 = 999;
/// Thumbnails larger than this are not inlined (a real one is ~50–300 KB).
const MAX_THUMBNAIL_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SaveState {
    pub path: String,
    pub file_name: String,
    /// Slot number; `None` for the auto save.
    pub slot: Option<u32>,
    pub auto: bool,
    /// Sub-folder of `states/` the file is in (core name when sorted by core).
    pub folder: Option<String>,
    pub modified: Option<String>,
    pub size: u64,
    /// `data:image/png;base64,…` when RetroArch saved a thumbnail.
    pub thumbnail: Option<String>,
    pub label: Option<String>,
}

/// RetroArch's own folder (the one holding retroarch.exe) under the emulators directory.
pub fn retroarch_dir(emulators_directory: &Path) -> Option<PathBuf> {
    let base = emulators_directory.join("retroarch");
    if base.join("retroarch.exe").exists() {
        return Some(base);
    }
    fs::read_dir(&base)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| p.is_dir() && p.join("retroarch.exe").exists())
}

/// Resolves `savestate_directory` from retroarch.cfg (`:\states` is relative
/// to RetroArch's folder; `default` or missing means `states/`).
pub fn states_dir(ra_dir: &Path, cfg: &str) -> PathBuf {
    let value = cfg.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;
        (key.trim() == "savestate_directory").then(|| value.trim().trim_matches('"').to_string())
    });
    match value.as_deref() {
        None | Some("") | Some("default") => ra_dir.join("states"),
        Some(v) if v.starts_with(":\\") || v.starts_with(":/") => ra_dir.join(&v[2..]),
        Some(v) => PathBuf::from(v),
    }
}

/// Parses a state file name for a given ROM stem: `Some((slot, auto))`.
pub fn parse_state_name(file_name: &str, rom_stem: &str) -> Option<(Option<u32>, bool)> {
    let lower = file_name.to_lowercase();
    let prefix = format!("{}.state", rom_stem.to_lowercase());
    let rest = lower.strip_prefix(&prefix)?;
    match rest {
        "" => Some((Some(0), false)),
        ".auto" => Some((None, true)),
        digits if !digits.is_empty() && digits.len() <= 3 && digits.chars().all(|c| c.is_ascii_digit()) => {
            Some((Some(digits.parse().ok()?), false))
        }
        _ => None,
    }
}

fn labels_path() -> PathBuf {
    crate::emuworld_base_dir().join("savestate_labels.json")
}

fn label_key(path: &Path) -> String {
    path.to_string_lossy().replace('/', "\\").to_lowercase()
}

fn load_labels(file: &Path) -> HashMap<String, String> {
    fs::read_to_string(file)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_labels(file: &Path, labels: &HashMap<String, String>) -> Result<(), String> {
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let data = serde_json::to_vec_pretty(labels).map_err(|e| e.to_string())?;
    crate::atomic_file::write(file, &data).map_err(|e| e.to_string())
}

fn thumbnail_of(state: &Path) -> Option<String> {
    let png = PathBuf::from(format!("{}.png", state.display()));
    let meta = fs::metadata(&png).ok()?;
    if meta.len() == 0 || meta.len() > MAX_THUMBNAIL_BYTES {
        return None;
    }
    let bytes = fs::read(&png).ok()?;
    Some(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
}

/// Every state of the ROM `rom_stem` under `states_root`, newest first.
pub fn list_states(states_root: &Path, rom_stem: &str, labels: &HashMap<String, String>) -> Vec<SaveState> {
    let mut states: Vec<SaveState> = walkdir::WalkDir::new(states_root)
        .max_depth(3)
        .into_iter()
        .flatten()
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| {
            let file_name = e.file_name().to_string_lossy().to_string();
            let (slot, auto) = parse_state_name(&file_name, rom_stem)?;
            let meta = e.metadata().ok()?;
            let folder = e
                .path()
                .parent()
                .filter(|p| *p != states_root)
                .and_then(|p| p.strip_prefix(states_root).ok())
                .map(|p| p.to_string_lossy().to_string());
            Some(SaveState {
                path: e.path().to_string_lossy().to_string(),
                file_name,
                slot,
                auto,
                folder,
                modified: meta.modified().ok().map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339()),
                size: meta.len(),
                thumbnail: thumbnail_of(e.path()),
                label: labels.get(&label_key(e.path())).cloned(),
            })
        })
        .collect();
    states.sort_by(|a, b| b.modified.cmp(&a.modified).then(a.slot.cmp(&b.slot)));
    states
}

/// Rejects any path that is not a state file inside `states_root`, so the
/// commands below can never copy or delete another file.
pub fn ensure_state_file(states_root: &Path, path: &Path) -> Result<PathBuf, String> {
    let canonical = path.canonicalize().map_err(|_| "Save state introuvable".to_string())?;
    let root = states_root.canonicalize().map_err(|_| "Dossier states RetroArch introuvable".to_string())?;
    let name = canonical.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
    let is_state = name.contains(".state") && !name.ends_with(".png");
    if !canonical.starts_with(&root) || !is_state || !canonical.is_file() {
        return Err("Accès refusé : ce fichier n'est pas un save state RetroArch".to_string());
    }
    Ok(canonical)
}

/// Copies a state (and its thumbnail) to the first free slot ≥ 1 of the same game.
pub fn duplicate_state(path: &Path) -> Result<PathBuf, String> {
    let dir = path.parent().ok_or("Dossier invalide")?;
    let file_name = path.file_name().map(|n| n.to_string_lossy().to_string()).ok_or("Nom invalide")?;
    let idx = file_name.to_lowercase().rfind(".state").ok_or("Nom de save state invalide")?;
    let base = &file_name[..idx];
    let target = (1..=MAX_SLOT)
        .map(|n| dir.join(format!("{}.state{}", base, n)))
        .find(|p| !p.exists())
        .ok_or("Tous les slots sont occupés")?;
    fs::copy(path, &target).map_err(|e| format!("Copie impossible: {}", e))?;
    let png = PathBuf::from(format!("{}.png", path.display()));
    if png.exists() {
        let _ = fs::copy(&png, format!("{}.png", target.display()));
    }
    Ok(target)
}

#[cfg(windows)]
fn move_to_recycle_bin(path: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use winapi::um::shellapi::{SHFileOperationW, FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_NOERRORUI, FOF_SILENT, FO_DELETE, SHFILEOPSTRUCTW};
    // pFrom is a list of paths ended by a double NUL.
    let mut from: Vec<u16> = path.as_os_str().encode_wide().collect();
    from.extend([0, 0]);
    let mut op = SHFILEOPSTRUCTW {
        hwnd: std::ptr::null_mut(),
        wFunc: FO_DELETE as u32,
        pFrom: from.as_ptr(),
        pTo: std::ptr::null(),
        fFlags: (FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_NOERRORUI | FOF_SILENT) as u16,
        fAnyOperationsAborted: 0,
        hNameMappings: std::ptr::null_mut(),
        lpszProgressTitle: std::ptr::null(),
    };
    let code = unsafe { SHFileOperationW(&mut op) };
    if code != 0 || op.fAnyOperationsAborted != 0 {
        return Err(format!("Corbeille indisponible (code {})", code));
    }
    Ok(())
}

#[cfg(not(windows))]
fn move_to_recycle_bin(path: &Path) -> Result<(), String> {
    fs::remove_file(path).map_err(|e| e.to_string())
}

fn ra_states_root() -> Result<PathBuf, String> {
    let config = crate::get_config();
    let ra_dir = retroarch_dir(Path::new(&config.emulators_directory)).ok_or("RetroArch n'est pas installé")?;
    let cfg = fs::read_to_string(ra_dir.join("retroarch.cfg")).unwrap_or_default();
    Ok(states_dir(&ra_dir, &cfg))
}

#[tauri::command]
pub fn list_save_states(rom_path: String) -> Result<Vec<SaveState>, String> {
    push_log("INFO", &format!("Save states: liste pour {}", rom_path));
    let stem = Path::new(&rom_path)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .ok_or("Chemin de ROM invalide")?;
    let root = match ra_states_root() {
        Ok(root) => root,
        // No RetroArch: no states, not an error the UI has to show.
        Err(e) => {
            push_log("INFO", &format!("Save states: {}", e));
            return Ok(Vec::new());
        }
    };
    if !root.exists() {
        return Ok(Vec::new());
    }
    let states = list_states(&root, &stem, &load_labels(&labels_path()));
    push_log("INFO", &format!("Save states: {} trouvé(s) pour '{}' dans {}", states.len(), stem, root.display()));
    Ok(states)
}

#[tauri::command]
pub fn rename_save_state(path: String, label: String) -> Result<(), String> {
    let root = ra_states_root()?;
    let file = ensure_state_file(&root, Path::new(&path)).map_err(|e| {
        push_log("WARN", &format!("Save states: renommage refusé pour {}: {}", path, e));
        e
    })?;
    let labels_file = labels_path();
    let mut labels = load_labels(&labels_file);
    let label = label.trim();
    if label.is_empty() {
        labels.remove(&label_key(&file));
    } else {
        labels.insert(label_key(&file), label.chars().take(80).collect());
    }
    save_labels(&labels_file, &labels)?;
    push_log("INFO", &format!("Save states: '{}' nommé '{}'", file.display(), label));
    Ok(())
}

#[tauri::command]
pub fn duplicate_save_state(path: String) -> Result<String, String> {
    let root = ra_states_root()?;
    let file = ensure_state_file(&root, Path::new(&path))?;
    let copy = duplicate_state(&file).map_err(|e| {
        push_log("ERROR", &format!("Save states: duplication de {} impossible: {}", file.display(), e));
        e
    })?;
    push_log("INFO", &format!("Save states: {} dupliqué vers {}", file.display(), copy.display()));
    Ok(copy.to_string_lossy().to_string())
}

#[tauri::command]
pub fn delete_save_state(path: String) -> Result<(), String> {
    let root = ra_states_root()?;
    let file = ensure_state_file(&root, Path::new(&path))?;
    move_to_recycle_bin(&file).map_err(|e| {
        push_log("ERROR", &format!("Save states: suppression de {} impossible: {}", file.display(), e));
        e
    })?;
    let png = PathBuf::from(format!("{}.png", file.display()));
    if png.exists() {
        let _ = move_to_recycle_bin(&png);
    }
    let labels_file = labels_path();
    let mut labels = load_labels(&labels_file);
    if labels.remove(&label_key(&file)).is_some() {
        let _ = save_labels(&labels_file, &labels);
    }
    push_log("INFO", &format!("Save states: {} mis à la corbeille", file.display()));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("emuworld_states_{}_{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn parses_retroarch_state_names() {
        let stem = "Super Mario World (USA)";
        assert_eq!(parse_state_name("Super Mario World (USA).state", stem), Some((Some(0), false)));
        assert_eq!(parse_state_name("Super Mario World (USA).state3", stem), Some((Some(3), false)));
        assert_eq!(parse_state_name("super mario world (usa).STATE12", stem), Some((Some(12), false)));
        assert_eq!(parse_state_name("Super Mario World (USA).state.auto", stem), Some((None, true)));
        assert_eq!(parse_state_name("Super Mario World (USA).state3.png", stem), None);
        assert_eq!(parse_state_name("Super Mario World (USA) 2.state", stem), None);
        assert_eq!(parse_state_name("Super Mario World (USA).srm", stem), None);
    }

    #[test]
    fn resolves_the_states_directory_from_retroarch_cfg() {
        let ra = Path::new("C:\\Emu\\retroarch");
        assert_eq!(states_dir(ra, "savestate_directory = \":\\states\""), ra.join("states"));
        assert_eq!(states_dir(ra, "savestate_directory = \"default\""), ra.join("states"));
        assert_eq!(states_dir(ra, ""), ra.join("states"));
        assert_eq!(states_dir(ra, "savestate_directory = \"D:\\Saves\\RA\""), PathBuf::from("D:\\Saves\\RA"));
    }

    #[test]
    fn lists_states_of_one_game_in_every_layout() {
        let root = temp_dir("list");
        fs::create_dir_all(root.join("Snes9x")).unwrap();
        fs::write(root.join("Snes9x").join("Zelda.state"), b"s0").unwrap();
        fs::write(root.join("Snes9x").join("Zelda.state2"), b"s2").unwrap();
        fs::write(root.join("Snes9x").join("Zelda.state2.png"), b"\x89PNG").unwrap();
        fs::write(root.join("Zelda.state.auto"), b"auto").unwrap();
        fs::write(root.join("Snes9x").join("Mario.state"), b"other game").unwrap();

        let key = label_key(&root.join("Snes9x").join("Zelda.state2"));
        let labels = HashMap::from([(key, "Avant le boss".to_string())]);
        let mut states = list_states(&root, "Zelda", &labels);
        states.sort_by_key(|s| s.file_name.clone());

        let names: Vec<_> = states.iter().map(|s| s.file_name.as_str()).collect();
        assert_eq!(names, ["Zelda.state", "Zelda.state.auto", "Zelda.state2"]);
        let slot2 = &states[2];
        assert_eq!(slot2.slot, Some(2));
        assert_eq!(slot2.folder.as_deref(), Some("Snes9x"));
        assert_eq!(slot2.label.as_deref(), Some("Avant le boss"));
        assert!(slot2.thumbnail.as_deref().unwrap().starts_with("data:image/png;base64,"));
        assert!(states[1].auto && states[1].folder.is_none());
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn refuses_files_outside_the_states_folder() {
        let root = temp_dir("guard");
        let outside = temp_dir("guard_outside");
        fs::write(root.join("Zelda.state1"), b"ok").unwrap();
        fs::write(root.join("Zelda.state1.png"), b"png").unwrap();
        fs::write(outside.join("Zelda.state1"), b"not managed").unwrap();
        fs::write(root.join("notes.txt"), b"x").unwrap();

        assert!(ensure_state_file(&root, &root.join("Zelda.state1")).is_ok());
        assert!(ensure_state_file(&root, &outside.join("Zelda.state1")).is_err());
        assert!(ensure_state_file(&root, &root.join("notes.txt")).is_err());
        assert!(ensure_state_file(&root, &root.join("Zelda.state1.png")).is_err());
        assert!(ensure_state_file(&root, &root.join("..").join(outside.file_name().unwrap()).join("Zelda.state1")).is_err());
        fs::remove_dir_all(&root).ok();
        fs::remove_dir_all(&outside).ok();
    }

    #[test]
    fn duplicates_to_the_first_free_slot_with_its_thumbnail() {
        let root = temp_dir("dup");
        fs::write(root.join("Zelda.state"), b"slot0").unwrap();
        fs::write(root.join("Zelda.state.png"), b"thumb").unwrap();
        fs::write(root.join("Zelda.state1"), b"slot1").unwrap();

        let copy = duplicate_state(&root.join("Zelda.state")).unwrap();
        assert_eq!(copy, root.join("Zelda.state2"));
        assert_eq!(fs::read(&copy).unwrap(), b"slot0");
        assert_eq!(fs::read(root.join("Zelda.state2.png")).unwrap(), b"thumb");
        // The original slots are untouched.
        assert_eq!(fs::read(root.join("Zelda.state1")).unwrap(), b"slot1");

        let again = duplicate_state(&root.join("Zelda.state2")).unwrap();
        assert_eq!(again, root.join("Zelda.state3"));
        fs::remove_dir_all(&root).ok();
    }
}
