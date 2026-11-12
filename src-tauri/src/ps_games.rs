//! PlayStation 4 and 5 games are dumped as folders, not single files:
//!
//! ```text
//! PS4/CUSA00900/eboot.bin            PS5/PPSA01234/eboot.bin
//! PS4/CUSA00900/sce_sys/param.sfo    PS5/PPSA01234/sce_sys/param.json
//! ```
//!
//! The scan lists each of those folders as one game. Its name comes from the
//! dump's own metadata (the folder is usually just the title ID), and what is
//! handed to the emulator differs: shadPS4 takes the `eboot.bin`, KytyPS5 the
//! folder.

use std::fs;
use std::path::{Path, PathBuf};

pub const PS4: &str = "PlayStation 4";
pub const PS5: &str = "PlayStation 5";

/// A game found as a folder.
#[derive(Debug, Clone, PartialEq)]
pub struct FolderGame {
    pub name: String,
    /// What the emulator is launched with.
    pub launch_path: PathBuf,
}

/// Emulator that runs the folder games of `console`, if it is one of ours.
pub fn emulator_for(console: &str) -> Option<&'static str> {
    match console {
        PS4 => Some("shadps4"),
        PS5 => Some("kytyps5"),
        _ => None,
    }
}

/// Whether a library entry is a game folder (PS5) or the executable inside one
/// (PS3, PS4). "Delete" must not remove that single file and leave gigabytes
/// of game data behind, nor wipe a whole folder on one click.
pub fn is_folder_game_path(path: &Path) -> bool {
    path.is_dir()
        || path.file_name().map(|n| n.to_string_lossy().eq_ignore_ascii_case("eboot.bin")).unwrap_or(false)
}

/// `TITLE` of a PS4 `param.sfo`.
///
/// Layout (little endian): magic `\0PSF`, version, key table offset, data
/// table offset, entry count, then 16-byte entries (key offset u16, format
/// u16, length u32, max length u32, data offset u32).
pub fn sfo_title(bytes: &[u8]) -> Option<String> {
    let u16_at = |o: usize| bytes.get(o..o + 2).map(|b| u16::from_le_bytes([b[0], b[1]]) as usize);
    let u32_at = |o: usize| bytes.get(o..o + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize);
    if bytes.get(0..4)? != b"\0PSF" {
        return None;
    }
    let key_table = u32_at(8)?;
    let data_table = u32_at(12)?;
    let count = u32_at(16)?;
    // A real file has a few dozen entries; refuse a corrupt header instead of looping on it.
    if count > 4096 {
        return None;
    }
    for i in 0..count {
        let entry = 20 + i * 16;
        let key_start = key_table.checked_add(u16_at(entry)?)?;
        let key = bytes.get(key_start..)?;
        let key_len = key.iter().position(|&b| b == 0)?;
        if &key[..key_len] != b"TITLE" {
            continue;
        }
        let len = u32_at(entry + 4)?;
        let start = data_table.checked_add(u32_at(entry + 12)?)?;
        let raw = bytes.get(start..start.checked_add(len)?)?;
        let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
        return clean_title(&String::from_utf8_lossy(&raw[..end]));
    }
    None
}

/// Title of a PS5 `param.json`: `localizedParameters.<defaultLanguage>.titleName`.
pub fn ps5_title(json: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    let localized = value.get("localizedParameters")?;
    let default_language = localized.get("defaultLanguage").and_then(|l| l.as_str()).unwrap_or("en-US");
    let title = localized
        .get(default_language)
        .and_then(|l| l.get("titleName"))
        .and_then(|t| t.as_str())
        .or_else(|| {
            localized.as_object()?.values().find_map(|l| l.get("titleName").and_then(|t| t.as_str()))
        })?;
    clean_title(title)
}

/// Drops trademark signs and line breaks: the name is shown on a card, used
/// to look the cover up and stored as the playtime key.
fn clean_title(raw: &str) -> Option<String> {
    let cleaned: String = raw
        .chars()
        .filter(|c| !matches!(c, '™' | '®' | '©'))
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    if cleaned.is_empty() { None } else { Some(cleaned) }
}

/// Update and add-on dumps sit next to the game (`CUSA00900-patch`,
/// `CUSA00900-UPDATE`…): the emulator merges them itself, they are not games.
fn is_patch_folder(name: &str) -> bool {
    let lower = name.to_lowercase();
    ["-patch", "-update", "-dlc", "_patch", "_update"].iter().any(|suffix| lower.ends_with(suffix))
}

fn game_in(dir: &Path, console: &str) -> Option<FolderGame> {
    let eboot = dir.join("eboot.bin");
    if !eboot.is_file() {
        return None;
    }
    let folder = dir.file_name()?.to_string_lossy().to_string();
    let title = match console {
        PS4 => fs::read(dir.join("sce_sys").join("param.sfo")).ok().and_then(|b| sfo_title(&b)),
        PS5 => fs::read_to_string(dir.join("sce_sys").join("param.json")).ok().and_then(|j| ps5_title(&j)),
        _ => None,
    };
    Some(FolderGame {
        name: title.unwrap_or(folder),
        // shadPS4 boots the executable, KytyPS5 the game folder.
        launch_path: if console == PS5 { dir.to_path_buf() } else { eboot },
    })
}

/// Games of a console folder (`<library>/PS4`, `<library>/PlayStation 5`…).
/// A game may sit one level deeper (`PS4/Bloodborne/CUSA00900/eboot.bin`).
pub fn scan_console_dir(console_dir: &Path, console: &str) -> Vec<FolderGame> {
    let subdirs = |dir: &Path| -> Vec<PathBuf> {
        let mut dirs: Vec<PathBuf> = fs::read_dir(dir)
            .map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect())
            .unwrap_or_default();
        dirs.sort();
        dirs
    };
    let is_patch = |p: &Path| p.file_name().map(|n| is_patch_folder(&n.to_string_lossy())).unwrap_or(false);

    let mut games = vec![];
    for dir in subdirs(console_dir) {
        if is_patch(&dir) {
            continue;
        }
        if let Some(game) = game_in(&dir, console) {
            games.push(game);
            continue;
        }
        for inner in subdirs(&dir) {
            if is_patch(&inner) {
                continue;
            }
            if let Some(game) = game_in(&inner, console) {
                games.push(game);
            }
        }
    }
    games
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal param.sfo with the given string entries.
    fn sfo(entries: &[(&str, &str)]) -> Vec<u8> {
        let mut keys = vec![];
        let mut data = vec![];
        let mut index = vec![];
        for (key, value) in entries {
            let len = value.len() as u32 + 1;
            index.push((keys.len() as u16, len, data.len() as u32));
            keys.extend_from_slice(key.as_bytes());
            keys.push(0);
            data.extend_from_slice(value.as_bytes());
            data.push(0);
        }
        let key_table = 20 + entries.len() as u32 * 16;
        let data_table = key_table + keys.len() as u32;
        let mut out = b"\0PSF".to_vec();
        out.extend_from_slice(&0x0101u32.to_le_bytes());
        out.extend_from_slice(&key_table.to_le_bytes());
        out.extend_from_slice(&data_table.to_le_bytes());
        out.extend_from_slice(&(entries.len() as u32).to_le_bytes());
        for (key_offset, len, data_offset) in index {
            out.extend_from_slice(&key_offset.to_le_bytes());
            out.extend_from_slice(&0x0204u16.to_le_bytes());
            out.extend_from_slice(&len.to_le_bytes());
            out.extend_from_slice(&len.to_le_bytes());
            out.extend_from_slice(&data_offset.to_le_bytes());
        }
        out.extend_from_slice(&keys);
        out.extend_from_slice(&data);
        out
    }

    fn library(tag: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("emuworld_psgames_{}_{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn write(path: &Path, bytes: &[u8]) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }

    #[test]
    fn reads_the_title_of_a_param_sfo() {
        let bytes = sfo(&[("CATEGORY", "gd"), ("TITLE", "Bloodborne™"), ("TITLE_ID", "CUSA00900")]);
        assert_eq!(sfo_title(&bytes).as_deref(), Some("Bloodborne"));
        assert_eq!(sfo_title(&sfo(&[("TITLE_ID", "CUSA00900")])), None, "no TITLE entry");
    }

    #[test]
    fn a_broken_param_sfo_gives_no_title_instead_of_a_panic() {
        assert_eq!(sfo_title(b""), None);
        assert_eq!(sfo_title(b"not an sfo file at all"), None);
        let mut truncated = sfo(&[("TITLE", "Bloodborne")]);
        truncated.truncate(30);
        assert_eq!(sfo_title(&truncated), None);
        // Header claiming billions of entries.
        let mut huge = b"\0PSF".to_vec();
        huge.extend_from_slice(&[1, 1, 0, 0, 20, 0, 0, 0, 20, 0, 0, 0, 0xff, 0xff, 0xff, 0xff]);
        assert_eq!(sfo_title(&huge), None);
    }

    #[test]
    fn reads_the_title_of_a_ps5_param_json() {
        let json = r#"{"titleId":"PPSA01234","localizedParameters":{"defaultLanguage":"fr-FR",
            "en-US":{"titleName":"Astro's Playroom"},"fr-FR":{"titleName":"ASTRO's PLAYROOM®"}}}"#;
        assert_eq!(ps5_title(json).as_deref(), Some("ASTRO's PLAYROOM"));
        let no_default = r#"{"localizedParameters":{"ja-JP":{"titleName":"Saros"}}}"#;
        assert_eq!(ps5_title(no_default).as_deref(), Some("Saros"));
        assert_eq!(ps5_title("{}"), None);
        assert_eq!(ps5_title("not json"), None);
    }

    #[test]
    fn lists_ps4_folders_by_title_and_launches_their_eboot() {
        let root = library("ps4");
        let ps4 = root.join("PS4");
        write(&ps4.join("CUSA00900").join("eboot.bin"), b"elf");
        write(&ps4.join("CUSA00900").join("sce_sys").join("param.sfo"), &sfo(&[("TITLE", "Bloodborne™")]));
        // No metadata: the folder name is the game's name.
        write(&ps4.join("Some Homebrew").join("eboot.bin"), b"elf");
        // One level deeper.
        write(&ps4.join("Journey").join("CUSA00470").join("eboot.bin"), b"elf");
        // Not games: an update dump and a folder without executable.
        write(&ps4.join("CUSA00900-patch").join("eboot.bin"), b"elf");
        write(&ps4.join("saves").join("readme.txt"), b"x");

        let games = scan_console_dir(&ps4, PS4);
        let names: Vec<&str> = games.iter().map(|g| g.name.as_str()).collect();
        assert_eq!(names, ["Bloodborne", "CUSA00470", "Some Homebrew"]);
        assert_eq!(games[0].launch_path, ps4.join("CUSA00900").join("eboot.bin"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn lists_ps5_folders_and_launches_the_folder() {
        let root = library("ps5");
        let ps5 = root.join("PS5");
        write(&ps5.join("PPSA01234").join("eboot.bin"), b"elf");
        write(
            &ps5.join("PPSA01234").join("sce_sys").join("param.json"),
            br#"{"localizedParameters":{"defaultLanguage":"en-US","en-US":{"titleName":"Astro Bot"}}}"#,
        );
        let games = scan_console_dir(&ps5, PS5);
        assert_eq!(games, [FolderGame { name: "Astro Bot".to_string(), launch_path: ps5.join("PPSA01234") }]);
        assert!(scan_console_dir(&root.join("missing"), PS5).is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn folder_games_are_recognised_for_the_delete_guard() {
        let root = library("delete");
        write(&root.join("PS4").join("CUSA00900").join("eboot.bin"), b"elf");
        write(&root.join("NES").join("Mario.nes"), b"rom");
        assert!(is_folder_game_path(&root.join("PS4").join("CUSA00900").join("eboot.bin")));
        assert!(is_folder_game_path(&root.join("PS3").join("Game").join("PS3_GAME").join("USRDIR").join("EBOOT.BIN")));
        assert!(is_folder_game_path(&root.join("PS4").join("CUSA00900")), "a PS5-style entry is the folder itself");
        assert!(!is_folder_game_path(&root.join("NES").join("Mario.nes")));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn each_folder_console_has_its_emulator() {
        assert_eq!(emulator_for(PS4), Some("shadps4"));
        assert_eq!(emulator_for(PS5), Some("kytyps5"));
        assert_eq!(emulator_for("PlayStation 3"), None);
    }
}
