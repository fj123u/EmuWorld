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
        // ═══════════════════════════════════════════════════════════
        //  NINTENDO
        // ═══════════════════════════════════════════════════════════

        // ── NES ─────────────────────────────────────────────────────
        EmulatorInfo {
            id: "fceux".to_string(),
            name: "FCEUX".to_string(),
            console: "NES".to_string(),
            description: "The most complete NES/Famicom emulator with debugging tools, TAS support, and excellent compatibility.".to_string(),
            download_url: "https://github.com/TASEmulators/fceux/releases/download/v2.6.6/fceux-2.6.6-win64.zip".to_string(),
            executable_name: "fceux.exe".to_string(),
            supported_extensions: vec!["nes".to_string(), "fds".to_string(), "nsf".to_string()],
            icon: "🔴".to_string(),
            website: "https://fceux.com".to_string(),
            archive_type: "zip".to_string(),
        },
        // ── Game Boy / GBA ──────────────────────────────────────────
        EmulatorInfo {
            id: "mgba".to_string(),
            name: "mGBA".to_string(),
            console: "Game Boy Advance".to_string(),
            description: "A fast and accurate GBA emulator. Also supports Game Boy and Game Boy Color.".to_string(),
            download_url: "https://github.com/mgba-emu/mgba/releases/download/0.10.5/mGBA-0.10.5-win64.7z".to_string(),
            executable_name: "mGBA.exe".to_string(),
            supported_extensions: vec!["gba".to_string(), "gb".to_string(), "gbc".to_string()],
            icon: "🟢".to_string(),
            website: "https://mgba.io".to_string(),
            archive_type: "7z".to_string(),
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
            description: "The leading N64 emulator with great compatibility and plugin support (Legacy portable edition).".to_string(),
            download_url: "https://github.com/pj64team/Project64-Legacy/releases/download/Project64-1.6.4/Project64.Legacy.v1.6.4.The.End.-.2024.06.22.zip".to_string(),
            executable_name: "Project64.exe".to_string(),
            supported_extensions: vec!["z64".to_string(), "n64".to_string(), "v64".to_string()],
            icon: "🟡".to_string(),
            website: "https://www.pj64-emu.com".to_string(),
            archive_type: "zip".to_string(),
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

        // ═══════════════════════════════════════════════════════════
        //  SONY
        // ═══════════════════════════════════════════════════════════

        // ── PlayStation 1 ───────────────────────────────────────────
        EmulatorInfo {
            id: "duckstation".to_string(),
            name: "DuckStation".to_string(),
            console: "PlayStation 1".to_string(),
            description: "A PS1 emulator focused on playability, speed, and long-term maintainability.".to_string(),
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

        // ═══════════════════════════════════════════════════════════
        //  SEGA
        // ═══════════════════════════════════════════════════════════

        // ── Dreamcast / Naomi / Atomiswave ─────────────────────────
        EmulatorInfo {
            id: "flycast".to_string(),
            name: "Flycast".to_string(),
            console: "Dreamcast".to_string(),
            description: "Multi-platform Dreamcast, Naomi, and Atomiswave emulator. Open-source fork of Reicast.".to_string(),
            download_url: "https://github.com/flyinghead/flycast/releases/download/v2.6/flycast-win64-2.6.zip".to_string(),
            executable_name: "flycast.exe".to_string(),
            supported_extensions: vec!["gdi".to_string(), "cdi".to_string(), "chd".to_string(), "cue".to_string()],
            icon: "🌀".to_string(),
            website: "https://github.com/flyinghead/flycast".to_string(),
            archive_type: "zip".to_string(),
        },

        // ═══════════════════════════════════════════════════════════
        //  MICROSOFT
        // ═══════════════════════════════════════════════════════════

        // ── DOS ────────────────────────────────────────────────────
        EmulatorInfo {
            id: "dosbox-x".to_string(),
            name: "DOSBox-X".to_string(),
            console: "DOS / Windows 3.x".to_string(),
            description: "A complete DOS emulation package. Runs DOS games, Windows 3.x, and Win9x. Enhanced fork of DOSBox.".to_string(),
            download_url: "https://github.com/joncampbell123/dosbox-x/releases/download/v2026.01.02/dosbox-x-vsbuild-win64-2026.01.02.zip".to_string(),
            executable_name: "dosbox-x.exe".to_string(),
            supported_extensions: vec!["exe".to_string(), "com".to_string(), "bat".to_string(), "iso".to_string(), "img".to_string()],
            icon: "💾".to_string(),
            website: "https://dosbox-x.com".to_string(),
            archive_type: "zip".to_string(),
        },

        // ═══════════════════════════════════════════════════════════
        //  MULTI-SYSTEM
        // ═══════════════════════════════════════════════════════════

        EmulatorInfo {
            id: "retroarch".to_string(),
            name: "RetroArch".to_string(),
            console: "Multi-System".to_string(),
            description: "A frontend for emulator cores (libretro). Supports dozens of consoles: NES, SNES, Genesis, Saturn, Arcade, Neo-Geo, PC Engine, 3DO, Atari, and many more.".to_string(),
            download_url: "https://buildbot.libretro.com/stable/1.20.0/windows/x86_64/RetroArch.7z".to_string(),
            executable_name: "retroarch.exe".to_string(),
            supported_extensions: vec![
                "nes".to_string(), "sfc".to_string(), "smc".to_string(), "gba".to_string(),
                "gb".to_string(), "gbc".to_string(), "nds".to_string(), "n64".to_string(),
                "z64".to_string(), "v64".to_string(), "iso".to_string(), "bin".to_string(),
                "cue".to_string(), "gen".to_string(), "md".to_string(), "sms".to_string(),
                "gg".to_string(), "pce".to_string(), "ngp".to_string(), "ngc".to_string(),
                "3do".to_string(), "a26".to_string(), "col".to_string(), "sg".to_string(),
            ],
            icon: "🔄".to_string(),
            website: "https://www.retroarch.com".to_string(),
            archive_type: "7z".to_string(),
        },

        // ═══════════════════════════════════════════════════════════
        //  RetroArch-powered consoles (use RetroArch cores)
        //  These are listed separately for organization,
        //  they all point to RetroArch as the emulator.
        // ═══════════════════════════════════════════════════════════

        // ── Master System ──────────────────────────────────────────
        EmulatorInfo {
            id: "retroarch-sms".to_string(),
            name: "RetroArch (Genesis Plus GX)".to_string(),
            console: "Master System".to_string(),
            description: "Sega Master System via RetroArch's Genesis Plus GX core. Download RetroArch, then install the genesis_plus_gx core.".to_string(),
            download_url: "https://buildbot.libretro.com/stable/1.20.0/windows/x86_64/RetroArch.7z".to_string(),
            executable_name: "retroarch.exe".to_string(),
            supported_extensions: vec!["sms".to_string(), "sg".to_string()],
            icon: "🎮".to_string(),
            website: "https://www.retroarch.com".to_string(),
            archive_type: "7z".to_string(),
        },
        // ── Mega Drive / Genesis ────────────────────────────────────
        EmulatorInfo {
            id: "retroarch-md".to_string(),
            name: "RetroArch (Genesis Plus GX)".to_string(),
            console: "Mega Drive".to_string(),
            description: "Sega Mega Drive / Genesis via RetroArch's Genesis Plus GX core. Supports Mega-CD and 32X with additional cores.".to_string(),
            download_url: "https://buildbot.libretro.com/stable/1.20.0/windows/x86_64/RetroArch.7z".to_string(),
            executable_name: "retroarch.exe".to_string(),
            supported_extensions: vec!["gen".to_string(), "md".to_string(), "bin".to_string(), "smd".to_string()],
            icon: "🎮".to_string(),
            website: "https://www.retroarch.com".to_string(),
            archive_type: "7z".to_string(),
        },
        // ── Game Gear ──────────────────────────────────────────────
        EmulatorInfo {
            id: "retroarch-gg".to_string(),
            name: "RetroArch (Genesis Plus GX)".to_string(),
            console: "Game Gear".to_string(),
            description: "Sega Game Gear via RetroArch's Genesis Plus GX core.".to_string(),
            download_url: "https://buildbot.libretro.com/stable/1.20.0/windows/x86_64/RetroArch.7z".to_string(),
            executable_name: "retroarch.exe".to_string(),
            supported_extensions: vec!["gg".to_string()],
            icon: "🎮".to_string(),
            website: "https://www.retroarch.com".to_string(),
            archive_type: "7z".to_string(),
        },
        // ── Mega-CD / Sega CD ──────────────────────────────────────
        EmulatorInfo {
            id: "retroarch-segacd".to_string(),
            name: "RetroArch (Genesis Plus GX)".to_string(),
            console: "Mega-CD".to_string(),
            description: "Sega Mega-CD / Sega CD via RetroArch. Requires BIOS files (bios_CD_U.bin, etc.).".to_string(),
            download_url: "https://buildbot.libretro.com/stable/1.20.0/windows/x86_64/RetroArch.7z".to_string(),
            executable_name: "retroarch.exe".to_string(),
            supported_extensions: vec!["cue".to_string(), "iso".to_string(), "chd".to_string()],
            icon: "💿".to_string(),
            website: "https://www.retroarch.com".to_string(),
            archive_type: "7z".to_string(),
        },
        // ── Sega 32X ───────────────────────────────────────────────
        EmulatorInfo {
            id: "retroarch-32x".to_string(),
            name: "RetroArch (PicoDrive)".to_string(),
            console: "Sega 32X".to_string(),
            description: "Sega 32X via RetroArch's PicoDrive core.".to_string(),
            download_url: "https://buildbot.libretro.com/stable/1.20.0/windows/x86_64/RetroArch.7z".to_string(),
            executable_name: "retroarch.exe".to_string(),
            supported_extensions: vec!["32x".to_string(), "bin".to_string()],
            icon: "🎮".to_string(),
            website: "https://www.retroarch.com".to_string(),
            archive_type: "7z".to_string(),
        },
        // ── Saturn ─────────────────────────────────────────────────
        EmulatorInfo {
            id: "retroarch-saturn".to_string(),
            name: "RetroArch (Beetle Saturn)".to_string(),
            console: "Saturn".to_string(),
            description: "Sega Saturn via RetroArch's Beetle Saturn / Kronos core. Requires BIOS.".to_string(),
            download_url: "https://buildbot.libretro.com/stable/1.20.0/windows/x86_64/RetroArch.7z".to_string(),
            executable_name: "retroarch.exe".to_string(),
            supported_extensions: vec!["cue".to_string(), "iso".to_string(), "chd".to_string(), "bin".to_string()],
            icon: "🪐".to_string(),
            website: "https://www.retroarch.com".to_string(),
            archive_type: "7z".to_string(),
        },
        // ── PC Engine / TurboGrafx-16 ──────────────────────────────
        EmulatorInfo {
            id: "retroarch-pce".to_string(),
            name: "RetroArch (Beetle PCE)".to_string(),
            console: "PC Engine".to_string(),
            description: "PC Engine / TurboGrafx-16 via RetroArch's Beetle PCE FAST core. Also supports PC Engine CD.".to_string(),
            download_url: "https://buildbot.libretro.com/stable/1.20.0/windows/x86_64/RetroArch.7z".to_string(),
            executable_name: "retroarch.exe".to_string(),
            supported_extensions: vec!["pce".to_string(), "cue".to_string(), "chd".to_string()],
            icon: "🔶".to_string(),
            website: "https://www.retroarch.com".to_string(),
            archive_type: "7z".to_string(),
        },
        // ── Neo-Geo ────────────────────────────────────────────────
        EmulatorInfo {
            id: "retroarch-neogeo".to_string(),
            name: "RetroArch (FinalBurn Neo)".to_string(),
            console: "Neo-Geo".to_string(),
            description: "SNK Neo-Geo (AES/MVS) via RetroArch's FinalBurn Neo core. Also plays Neo-Geo CD.".to_string(),
            download_url: "https://buildbot.libretro.com/stable/1.20.0/windows/x86_64/RetroArch.7z".to_string(),
            executable_name: "retroarch.exe".to_string(),
            supported_extensions: vec!["zip".to_string(), "7z".to_string()],
            icon: "🅰️".to_string(),
            website: "https://www.retroarch.com".to_string(),
            archive_type: "7z".to_string(),
        },
        // ── 3DO ────────────────────────────────────────────────────
        EmulatorInfo {
            id: "retroarch-3do".to_string(),
            name: "RetroArch (Opera)".to_string(),
            console: "3DO".to_string(),
            description: "3DO Interactive Multiplayer via RetroArch's Opera core. Requires BIOS (panafz10.bin).".to_string(),
            download_url: "https://buildbot.libretro.com/stable/1.20.0/windows/x86_64/RetroArch.7z".to_string(),
            executable_name: "retroarch.exe".to_string(),
            supported_extensions: vec!["iso".to_string(), "cue".to_string(), "chd".to_string()],
            icon: "🎲".to_string(),
            website: "https://www.retroarch.com".to_string(),
            archive_type: "7z".to_string(),
        },
        // ── Arcade (MAME / FBNeo) ──────────────────────────────────
        EmulatorInfo {
            id: "retroarch-arcade".to_string(),
            name: "RetroArch (FinalBurn Neo)".to_string(),
            console: "Arcade".to_string(),
            description: "Arcade machines via RetroArch's FinalBurn Neo or MAME core. Supports thousands of arcade ROMs.".to_string(),
            download_url: "https://buildbot.libretro.com/stable/1.20.0/windows/x86_64/RetroArch.7z".to_string(),
            executable_name: "retroarch.exe".to_string(),
            supported_extensions: vec!["zip".to_string(), "7z".to_string()],
            icon: "🕹️".to_string(),
            website: "https://www.retroarch.com".to_string(),
            archive_type: "7z".to_string(),
        },
        // ── Atari 2600 ─────────────────────────────────────────────
        EmulatorInfo {
            id: "retroarch-atari2600".to_string(),
            name: "RetroArch (Stella)".to_string(),
            console: "Atari 2600".to_string(),
            description: "Atari 2600 via RetroArch's Stella core.".to_string(),
            download_url: "https://buildbot.libretro.com/stable/1.20.0/windows/x86_64/RetroArch.7z".to_string(),
            executable_name: "retroarch.exe".to_string(),
            supported_extensions: vec!["a26".to_string(), "bin".to_string()],
            icon: "🟤".to_string(),
            website: "https://www.retroarch.com".to_string(),
            archive_type: "7z".to_string(),
        },
        // ── Atari 7800 ─────────────────────────────────────────────
        EmulatorInfo {
            id: "retroarch-atari7800".to_string(),
            name: "RetroArch (ProSystem)".to_string(),
            console: "Atari 7800".to_string(),
            description: "Atari 7800 ProSystem via RetroArch.".to_string(),
            download_url: "https://buildbot.libretro.com/stable/1.20.0/windows/x86_64/RetroArch.7z".to_string(),
            executable_name: "retroarch.exe".to_string(),
            supported_extensions: vec!["a78".to_string(), "bin".to_string()],
            icon: "🟤".to_string(),
            website: "https://www.retroarch.com".to_string(),
            archive_type: "7z".to_string(),
        },
        // ── Atari Lynx ─────────────────────────────────────────────
        EmulatorInfo {
            id: "retroarch-lynx".to_string(),
            name: "RetroArch (Handy)".to_string(),
            console: "Atari Lynx".to_string(),
            description: "Atari Lynx via RetroArch's Handy core.".to_string(),
            download_url: "https://buildbot.libretro.com/stable/1.20.0/windows/x86_64/RetroArch.7z".to_string(),
            executable_name: "retroarch.exe".to_string(),
            supported_extensions: vec!["lnx".to_string()],
            icon: "🟤".to_string(),
            website: "https://www.retroarch.com".to_string(),
            archive_type: "7z".to_string(),
        },
        // ── WonderSwan ─────────────────────────────────────────────
        EmulatorInfo {
            id: "retroarch-wswan".to_string(),
            name: "RetroArch (Beetle Cygne)".to_string(),
            console: "WonderSwan".to_string(),
            description: "Bandai WonderSwan / WonderSwan Color via RetroArch's Beetle Cygne core.".to_string(),
            download_url: "https://buildbot.libretro.com/stable/1.20.0/windows/x86_64/RetroArch.7z".to_string(),
            executable_name: "retroarch.exe".to_string(),
            supported_extensions: vec!["ws".to_string(), "wsc".to_string()],
            icon: "🔲".to_string(),
            website: "https://www.retroarch.com".to_string(),
            archive_type: "7z".to_string(),
        },
        // ── Neo Geo Pocket ─────────────────────────────────────────
        EmulatorInfo {
            id: "retroarch-ngp".to_string(),
            name: "RetroArch (Beetle NeoPop)".to_string(),
            console: "Neo Geo Pocket".to_string(),
            description: "SNK Neo Geo Pocket / Color via RetroArch's Beetle NeoPop core.".to_string(),
            download_url: "https://buildbot.libretro.com/stable/1.20.0/windows/x86_64/RetroArch.7z".to_string(),
            executable_name: "retroarch.exe".to_string(),
            supported_extensions: vec!["ngp".to_string(), "ngc".to_string()],
            icon: "🔲".to_string(),
            website: "https://www.retroarch.com".to_string(),
            archive_type: "7z".to_string(),
        },
        // ── Virtual Boy ────────────────────────────────────────────
        EmulatorInfo {
            id: "retroarch-vb".to_string(),
            name: "RetroArch (Beetle VB)".to_string(),
            console: "Virtual Boy".to_string(),
            description: "Nintendo Virtual Boy via RetroArch's Beetle VB core.".to_string(),
            download_url: "https://buildbot.libretro.com/stable/1.20.0/windows/x86_64/RetroArch.7z".to_string(),
            executable_name: "retroarch.exe".to_string(),
            supported_extensions: vec!["vb".to_string(), "vboy".to_string()],
            icon: "🔴".to_string(),
            website: "https://www.retroarch.com".to_string(),
            archive_type: "7z".to_string(),
        },
        // ── PlayStation 3 ───────────────────────────────────────────
        EmulatorInfo {
            id: "rpcs3".to_string(),
            name: "RPCS3".to_string(),
            console: "PlayStation 3".to_string(),
            description: "The world's first open-source PlayStation 3 emulator and debugger.".to_string(),
            download_url: "https://github.com/RPCS3/rpcs3-binaries-win/releases/download/build-d13c9e6d4206584c311c970172e2530a273e86c1/rpcs3-v0.0.35-17482-d13c9e6d_win64.7z".to_string(),
            executable_name: "rpcs3.exe".to_string(),
            supported_extensions: vec!["pbp".to_string(), "iso".to_string(), "pkg".to_string()],
            icon: "⚪".to_string(),
            website: "https://rpcs3.net".to_string(),
            archive_type: "7z".to_string(),
        },
        // ── Xbox ────────────────────────────────────────────────────
        EmulatorInfo {
            id: "xemu".to_string(),
            name: "xemu".to_string(),
            console: "Xbox".to_string(),
            description: "A free and open-source emulator of the original Xbox console.".to_string(),
            download_url: "https://github.com/xemu-project/xemu/releases/download/v0.8.134/xemu-win-x86_64.zip".to_string(),
            executable_name: "xemu.exe".to_string(),
            supported_extensions: vec!["iso".to_string(), "xbe".to_string()],
            icon: "❎".to_string(),
            website: "https://xemu.app".to_string(),
            archive_type: "zip".to_string(),
        },
        // ── Wii U ───────────────────────────────────────────────────
        EmulatorInfo {
            id: "cemu".to_string(),
            name: "Cemu".to_string(),
            console: "Wii U".to_string(),
            description: "Highly polished Wii U emulator capable of running major titles at 4K and beyond.".to_string(),
            download_url: "https://github.com/cemu-project/Cemu/releases/download/v2.6/cemu-2.6-windows-x64.zip".to_string(),
            executable_name: "Cemu.exe".to_string(),
            supported_extensions: vec!["wud".to_string(), "wux".to_string(), "rpx".to_string()],
            icon: "📽️".to_string(),
            website: "https://cemu.info".to_string(),
            archive_type: "zip".to_string(),
        },
    ]
}
