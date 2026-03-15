## Universal Emulator Hub (C++ / Qt)

Projet universitaire : lanceur / gestionnaire d'émulateurs écrit en **C++17** avec **Qt 6** et **CMake**.

### Prérequis

- [CMake](https://cmake.org/)
- Qt 6 (modules **Widgets** et **Network**)
- Un compilateur C++ compatible C++17 (MSVC, clang, etc.)

### Compilation et lancement (Windows)

1. **Double-cliquer** sur `EmuWorld.bat`
   - Configure CMake dans le dossier `build/`
   - Compile le projet en Debug
   - Lance l'exécutable `UniversalEmulatorHubCpp.exe`

2. Ou en ligne de commande :

```bash
cmake -S . -B build
cmake --build build --config Debug
build/UniversalEmulatorHubCpp.exe
```

### Structure du projet

```text
UniversalEmulatorHubCpp/
├── CMakeLists.txt
├── src/
│   ├── CMakeLists.txt
│   ├── main.cpp
│   ├── MainWindow.h / .cpp        # Fenêtre principale (UI)
│   ├── EmulatorManager.h / .cpp   # Gestion du catalogue et des téléchargements
│   └── models/
│       ├── Emulator.h
│       └── Rom.h
└── EmuWorld.bat                   # Script de build + lancement
```
