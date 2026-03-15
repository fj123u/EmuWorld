@echo off
title Universal Emulator Hub (C++/Qt)
cd /d "%~dp0"

echo.
echo  ============================================
echo   Universal Emulator Hub - C++ / Qt Launcher
echo  ============================================
echo.

:: Vérifie CMake
where cmake >nul 2>&1
if %errorlevel% neq 0 (
    echo [ERROR] CMake n'est pas installe ou pas dans le PATH.
    echo         Installe-le depuis https://cmake.org/
    pause
    exit /b 1
)

:: Crée le dossier de build s'il n'existe pas
if not exist "build" (
    mkdir build
)

cd build

:: Configure le projet (une seule fois)
if not exist "CMakeCache.txt" (
    echo [INFO] Configuration du projet CMake...
    cmake ..
    if %errorlevel% neq 0 (
        echo [ERROR] Echec de la configuration CMake.
        pause
        exit /b 1
    )
)

:: Compile en Debug
echo [INFO] Compilation du projet...
cmake --build . --config Debug
if %errorlevel% neq 0 (
    echo [ERROR] Echec de la compilation.
    pause
    exit /b 1
)

echo [INFO] Lancement de l'application...
echo.

UniversalEmulatorHubCpp.exe

echo.
echo [INFO] L'application s'est fermee.
pause
