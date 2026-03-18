use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EmulatorInfo {
    pub id: String,
    pub name: String,
    pub console: String,
    pub description: String,
    pub download_url: String,
    pub executable_name: String,
    pub supported_extensions: Vec<String>,
    pub icon: String,
    pub website: String,
    /// "zip" or "7z" — determines extraction method
    pub archive_type: String,
}

/// Returns the built-in catalog of supported emulators.
/// All download URLs verified via GitHub API (March 2026).
pub fn get_catalog() -> Vec<EmulatorInfo> {
    vec![
        // ── Game Boy / GBA ──────────────────────────────────────────
        EmulatorInfo {
            id: "mgba".to_string(),
            name: "mGBA".to_string(),
            console: "Game Boy Advance".to_string(),
            description: "A fast and accurate Game Boy Advance emulator. Also supports GB and GBC.".to_string(),
            download_url: "https://github.com/mgba-emu/mgba/releases/download/0.10.5/mGBA-0.10.5-win64.7z".to_string(),
            executable_name: "mGBA.exe".to_string(),
            supported_extensions: vec!["gba".to_string(), "gb".to_string(), "gbc".to_string()],
            icon: "🟢".to_string(),
            website: "https://mgba.io".to_string(),
            archive_type: "7z".to_string(),
        },
        // ── Nintendo DS ─────────────────────────────────────────────
        EmulatorInfo {
            id: "melonds".to_string(),
            name: "melonDS".to_string(),
            console: "Nintendo DS".to_string(),
            description: "A modern Nintendo DS and DSi emulator focused on accuracy and performance.".to_string(),
            download_url: "https://github.com/melonDS-emu/melonDS/releases/download/1.1/melonDS-1.1-windows-x86_64.zip".to_string(),
            executable_name: "melonDS.exe".to_string(),
            supported_extensions: vec!["nds".to_string(), "ds".to_string()],
            icon: "📱".to_string(),
            website: "https://melonds.kuribo64.net".to_string(),
            archive_type: "zip".to_string(),
        },
        EmulatorInfo {
            id: "desmume".to_string(),
            name: "DeSmuME".to_string(),
            console: "Nintendo DS".to_string(),
            description: "The most established Nintendo DS emulator with extensive compatibility.".to_string(),
            download_url: "https://nightly.link/TASEmulators/desmume/workflows/build_win/master/desmume-win-x64.zip".to_string(),
            executable_name: "DeSmuME_x64.exe".to_string(),
            supported_extensions: vec!["nds".to_string(), "ds".to_string()],
            icon: "📱".to_string(),
            website: "http://desmume.org".to_string(),
            archive_type: "zip".to_string(),
        },
        // ── Super Nintendo ──────────────────────────────────────────
        EmulatorInfo {
            id: "snes9x".to_string(),
            name: "Snes9x".to_string(),
            console: "Super Nintendo".to_string(),
            description: "A portable, freeware Super Nintendo Entertainment System emulator.".to_string(),
            download_url: "https://github.com/snes9xgit/snes9x/releases/download/1.63/snes9x-1.63-win32-x64.zip".to_string(),
            executable_name: "snes9x-x64.exe".to_string(),
            supported_extensions: vec!["sfc".to_string(), "smc".to_string()],
            icon: "🟣".to_string(),
            website: "https://www.snes9x.com".to_string(),
            archive_type: "zip".to_string(),
        },
        // ── Nintendo 64 ─────────────────────────────────────────────
        EmulatorInfo {
            id: "project64".to_string(),
            name: "Project64".to_string(),
            console: "Nintendo 64".to_string(),
            description: "The leading N64 emulator with great compatibility and plugin support.".to_string(),
            download_url: "https://github.com/pj64team/Project64-Legacy/releases/download/release_1.6.4/Project64.Legacy.v1.6.4.The.End.-.2024.06.22.zip".to_string(),
            executable_name: "Project64.exe".to_string(),
            supported_extensions: vec!["z64".to_string(), "n64".to_string(), "v64".to_string()],
            icon: "🟡".to_string(),
            website: "https://www.pj64-emu.com".to_string(),
            archive_type: "zip".to_string(),
        },
        // ── GameCube / Wii ──────────────────────────────────────────
        EmulatorInfo {
            id: "dolphin".to_string(),
            name: "Dolphin".to_string(),
            console: "GameCube / Wii".to_string(),
            description: "The gold standard emulator for GameCube and Wii games.".to_string(),
            download_url: "https://dl.dolphin-emu.org/releases/2412/dolphin-2412-x64.7z".to_string(),
            executable_name: "Dolphin.exe".to_string(),
            supported_extensions: vec!["iso".to_string(), "gcm".to_string(), "wbfs".to_string(), "rvz".to_string()],
            icon: "🐬".to_string(),
            website: "https://dolphin-emu.org".to_string(),
            archive_type: "7z".to_string(),
        },
        // ── PlayStation 1 ───────────────────────────────────────────
        EmulatorInfo {
            id: "duckstation".to_string(),
            name: "DuckStation".to_string(),
            console: "PlayStation 1".to_string(),
            description: "A PlayStation 1 emulator focused on playability, speed, and long-term maintainability.".to_string(),
            download_url: "https://github.com/stenzek/duckstation/releases/download/latest/duckstation-windows-x64-release.zip".to_string(),
            executable_name: "duckstation-qt-x64-ReleaseLTCG.exe".to_string(),
            supported_extensions: vec!["bin".to_string(), "iso".to_string(), "cue".to_string(), "img".to_string(), "chd".to_string()],
            icon: "⚪".to_string(),
            website: "https://www.duckstation.org".to_string(),
            archive_type: "zip".to_string(),
        },
        // ── PlayStation 2 ───────────────────────────────────────────
        EmulatorInfo {
            id: "pcsx2".to_string(),
            name: "PCSX2".to_string(),
            console: "PlayStation 2".to_string(),
            description: "The most trusted PlayStation 2 emulator with Qt interface and Vulkan support.".to_string(),
            download_url: "https://github.com/PCSX2/pcsx2/releases/download/v2.6.3/pcsx2-v2.6.3-windows-x64-Qt.7z".to_string(),
            executable_name: "pcsx2-qt.exe".to_string(),
            supported_extensions: vec!["iso".to_string(), "bin".to_string(), "chd".to_string()],
            icon: "🔵".to_string(),
            website: "https://pcsx2.net".to_string(),
            archive_type: "7z".to_string(),
        },
        // ── PlayStation Portable ────────────────────────────────────
        EmulatorInfo {
            id: "ppsspp".to_string(),
            name: "PPSSPP".to_string(),
            console: "PlayStation Portable".to_string(),
            description: "The best PSP emulator, capable of running games at full HD and beyond.".to_string(),
            download_url: "https://github.com/hrydgard/ppsspp/releases/download/v1.20.3/PPSSPP-v1.20.3-Windows-x64.zip".to_string(),
            executable_name: "PPSSPPWindows64.exe".to_string(),
            supported_extensions: vec!["iso".to_string(), "cso".to_string(), "pbp".to_string()],
            icon: "⬛".to_string(),
            website: "https://www.ppsspp.org".to_string(),
            archive_type: "zip".to_string(),
        },
        // ── Multi-System ────────────────────────────────────────────
        EmulatorInfo {
            id: "retroarch".to_string(),
            name: "RetroArch".to_string(),
            console: "Multi-System".to_string(),
            description: "A frontend for emulator cores (libretro). Supports dozens of consoles.".to_string(),
            download_url: "https://buildbot.libretro.com/stable/1.20.0/windows/x86_64/RetroArch.7z".to_string(),
            executable_name: "retroarch.exe".to_string(),
            supported_extensions: vec![
                "nes".to_string(), "sfc".to_string(), "smc".to_string(), "gba".to_string(),
                "gb".to_string(), "gbc".to_string(), "nds".to_string(), "n64".to_string(),
                "z64".to_string(), "v64".to_string(), "iso".to_string(), "bin".to_string(),
                "cue".to_string(),
            ],
            icon: "🔄".to_string(),
            website: "https://www.retroarch.com".to_string(),
            archive_type: "7z".to_string(),
        },
    ]
}
