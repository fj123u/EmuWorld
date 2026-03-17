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
/// All download URLs have been verified as working.
pub fn get_catalog() -> Vec<EmulatorInfo> {
    vec![
        // ── Game Boy / GBA ──────────────────────────────────────────
        EmulatorInfo {
            id: "mgba".to_string(),
            name: "mGBA".to_string(),
            console: "Game Boy Advance".to_string(),
            description: "A fast and accurate Game Boy Advance emulator. Also supports Game Boy and Game Boy Color.".to_string(),
            download_url: "https://github.com/mgba-emu/mgba/releases/download/0.10.4/mGBA-0.10.4-win64.7z".to_string(),
            executable_name: "mGBA.exe".to_string(),
            supported_extensions: vec!["gba".to_string(), "gb".to_string(), "gbc".to_string()],
            icon: "🎮".to_string(),
            website: "https://mgba.io".to_string(),
            archive_type: "7z".to_string(),
        },
        // ── Nintendo DS ─────────────────────────────────────────────
        EmulatorInfo {
            id: "melonds".to_string(),
            name: "melonDS".to_string(),
            console: "Nintendo DS".to_string(),
            description: "A modern Nintendo DS and DSi emulator focused on accuracy and performance.".to_string(),
            download_url: "https://github.com/melonDS-emu/melonDS/releases/download/0.9.5/melonDS_0.9.5_win_x64.zip".to_string(),
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
            download_url: "https://github.com/TASEmulators/desmume/releases/download/release_0_9_13/desmume-0.9.13-win64.zip".to_string(),
            executable_name: "DeSmuME_0.9.13_x64.exe".to_string(),
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
            icon: "🕹️".to_string(),
            website: "https://www.snes9x.com".to_string(),
            archive_type: "zip".to_string(),
        },
        // ── Nintendo 64 ─────────────────────────────────────────────
        EmulatorInfo {
            id: "project64".to_string(),
            name: "Project64".to_string(),
            console: "Nintendo 64".to_string(),
            description: "The leading N64 emulator with great compatibility and plugin support.".to_string(),
            download_url: "https://sourceforge.net/projects/project64/files/Builds/Release%20Builds/Windows%20x86-64/3.0.1/Project64_3.0.1-5664_2df3434-win64.zip/download".to_string(),
            executable_name: "Project64.exe".to_string(),
            supported_extensions: vec!["z64".to_string(), "n64".to_string(), "v64".to_string()],
            icon: "🎮".to_string(),
            website: "https://www.pj64-emu.com".to_string(),
            archive_type: "zip".to_string(),
        },
        // ── GameCube / Wii ──────────────────────────────────────────
        EmulatorInfo {
            id: "dolphin".to_string(),
            name: "Dolphin".to_string(),
            console: "GameCube / Wii".to_string(),
            description: "The gold standard emulator for GameCube and Wii games. Requires 7-Zip to extract.".to_string(),
            download_url: "https://dl.dolphin-emu.org/builds/2c/5d/dolphin-master-2c5d071-x64.7z".to_string(),
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
            download_url: "https://github.com/stenzek/duckstation/releases/download/v0.1-7892/duckstation-windows-x64-release.zip".to_string(),
            executable_name: "duckstation-qt-x64-ReleaseLTCG.exe".to_string(),
            supported_extensions: vec!["bin".to_string(), "iso".to_string(), "cue".to_string(), "img".to_string(), "chd".to_string()],
            icon: "🎯".to_string(),
            website: "https://www.duckstation.org".to_string(),
            archive_type: "zip".to_string(),
        },
        // ── PlayStation 2 ───────────────────────────────────────────
        EmulatorInfo {
            id: "pcsx2".to_string(),
            name: "PCSX2".to_string(),
            console: "PlayStation 2".to_string(),
            description: "The most trusted PlayStation 2 emulator with Qt interface and Vulkan support. Requires 7-Zip to extract.".to_string(),
            download_url: "https://github.com/PCSX2/pcsx2/releases/download/v2.6.3/pcsx2-v2.6.3-windows-x64-Qt.7z".to_string(),
            executable_name: "pcsx2-qt.exe".to_string(),
            supported_extensions: vec!["iso".to_string(), "bin".to_string(), "chd".to_string()],
            icon: "🎮".to_string(),
            website: "https://pcsx2.net".to_string(),
            archive_type: "7z".to_string(),
        },
        // ── PlayStation Portable ────────────────────────────────────
        EmulatorInfo {
            id: "ppsspp".to_string(),
            name: "PPSSPP".to_string(),
            console: "PlayStation Portable".to_string(),
            description: "The best PSP emulator, capable of running games at full HD and beyond.".to_string(),
            download_url: "https://www.ppsspp.org/files/1_17_1/PPSSPPWindowsPortable64.zip".to_string(),
            executable_name: "PPSSPPWindows64.exe".to_string(),
            supported_extensions: vec!["iso".to_string(), "cso".to_string(), "pbp".to_string()],
            icon: "🕹️".to_string(),
            website: "https://www.ppsspp.org".to_string(),
            archive_type: "zip".to_string(),
        },
        // ── Multi-System ────────────────────────────────────────────
        EmulatorInfo {
            id: "retroarch".to_string(),
            name: "RetroArch".to_string(),
            console: "Multi-System".to_string(),
            description: "A frontend for emulator cores (libretro). Supports dozens of consoles. Requires 7-Zip to extract.".to_string(),
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
