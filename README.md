# 🎮 EmuWorld

<p align="center">
  <strong>All-in-one emulator launcher for Windows</strong>
</p>

<p align="center">
  Download, install, and launch your favorite emulators from a single premium interface.
</p>

---

## ✨ Features

- **Emulator Catalog** — Browse 10+ emulators (mGBA, DeSmuME, melonDS, DuckStation, PCSX2, PPSSPP, Project64, Snes9x, Dolphin, RetroArch)
- **One-Click Install** — Download and extract emulators automatically from official sources
- **Game Library** — Scan your ROM folders and launch games with the right emulator
- **Emulator Manager** — Install, update, uninstall emulators from one place
- **Modern UI** — Dark theme with glassmorphism, smooth animations, and a custom titlebar
- **Open Source** — MIT license, contribute and extend freely

## 🖥️ Supported Consoles

| Console | Emulator(s) |
|---|---|
| Game Boy / GBA | mGBA |
| Nintendo DS | DeSmuME, melonDS |
| Nintendo 64 | Project64 |
| Super Nintendo | Snes9x |
| GameCube / Wii | Dolphin |
| PlayStation 1 | DuckStation |
| PlayStation 2 | PCSX2 |
| PSP | PPSSPP |
| Multi-System | RetroArch |

## 🚀 Getting Started

### Prerequisites

- [Node.js](https://nodejs.org/) (v18+)
- [Rust](https://www.rust-lang.org/) (latest stable)
- [Tauri Prerequisites](https://tauri.app/start/prerequisites/)

### Install & Run

```bash
# Clone the repository
git clone https://github.com/your-username/EmuWorld.git
cd EmuWorld

# Install dependencies
npm install

# Run in development mode
npm run tauri dev
```

### Build for Production

```bash
npm run tauri build
```

The installer will be generated in `src-tauri/target/release/bundle/`.

## 📁 Project Structure

```
EmuWorld/
├── src/                    # React frontend
│   ├── App.tsx             # Main application component
│   ├── App.css             # Design system & styles
│   └── main.tsx            # Entry point
├── src-tauri/              # Tauri backend (Rust)
│   ├── src/
│   │   ├── lib.rs          # Core logic (commands, config, ROM scanning)
│   │   ├── emulators.rs    # Emulator catalog & metadata
│   │   └── main.rs         # Application entry
│   ├── Cargo.toml          # Rust dependencies
│   └── tauri.conf.json     # Tauri configuration
├── index.html              # HTML shell
├── package.json            # Node dependencies
└── README.md
```

## 🧩 Adding an Emulator

To add a new emulator, edit `src-tauri/src/emulators.rs` and add a new `EmulatorInfo` entry to the `get_catalog()` function:

```rust
EmulatorInfo {
    id: "my-emulator".to_string(),
    name: "My Emulator".to_string(),
    console: "Console Name".to_string(),
    description: "Description of the emulator.".to_string(),
    download_url: "https://example.com/download.zip".to_string(),
    executable_name: "emulator.exe".to_string(),
    supported_extensions: vec!["rom".to_string()],
    icon: "🎮".to_string(),
    website: "https://example.com".to_string(),
},
```

## 🛠️ Tech Stack

- **Frontend**: React 19 + TypeScript + Vite
- **Backend**: Tauri 2 (Rust)
- **UI**: Vanilla CSS + Framer Motion + Lucide Icons
- **HTTP**: reqwest (Rust)
- **Archive**: zip-rs

## 📄 License

MIT License — see [LICENSE](LICENSE) for details.
