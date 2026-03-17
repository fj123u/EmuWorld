# 🎮 EmuWorld

All-in-one emulator launcher for Windows. Download, install, and launch emulators from a single modern interface.

## ✨ Features

- **Emulator Catalog** — 10 emulators (mGBA, melonDS, DeSmuME, Snes9x, Project64, Dolphin, DuckStation, PCSX2, PPSSPP, RetroArch)
- **One-Click Install** — Automatic download & extraction from official sources
- **Game Library** — Scan ROM folders and launch games with the right emulator
- **Modern UI** — Dark theme, glassmorphism, Framer Motion animations, custom titlebar

## 🚀 Quick Start

**Double-click `EmuWorld.bat`** or run:

```bash
npm install
npm run tauri dev
```

### Prerequisites

- [Node.js](https://nodejs.org/) (v18+)
- [Rust](https://www.rust-lang.org/) (latest stable)
- [7-Zip](https://www.7-zip.org/) (optional, for .7z emulators)

## 🛠️ Tech Stack

- **Frontend**: React 19 + TypeScript + Vite
- **Backend**: Tauri 2 (Rust)
- **UI**: Vanilla CSS + Framer Motion + Lucide Icons

## 📁 Structure

```
EmuWorld/
├── src/                    # React frontend
│   ├── App.tsx             # Main app component
│   ├── App.css             # Design system
│   └── main.tsx            # Entry point
├── src-tauri/              # Rust backend
│   ├── src/
│   │   ├── lib.rs          # Core logic
│   │   ├── emulators.rs    # Emulator catalog
│   │   └── main.rs         # Entry point
│   ├── Cargo.toml
│   └── tauri.conf.json
├── EmuWorld.bat            # Double-click to launch
└── package.json
```

## 📄 License

MIT
