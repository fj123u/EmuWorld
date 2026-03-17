@echo off
title EmuWorld Launcher
cd /d "%~dp0"

echo.
echo  ============================
echo   EmuWorld - Starting...
echo  ============================
echo.

:: Check Node.js
where node >nul 2>&1
if %errorlevel% neq 0 (
    echo [ERROR] Node.js not found. Install from https://nodejs.org/
    pause
    exit /b 1
)

:: Check Rust
where cargo >nul 2>&1
if %errorlevel% neq 0 (
    echo [ERROR] Rust not found. Install from https://www.rust-lang.org/
    pause
    exit /b 1
)

:: Install deps if needed
if not exist "node_modules" (
    echo [INFO] Installing dependencies...
    call npm install
    if %errorlevel% neq 0 (
        echo [ERROR] npm install failed.
        pause
        exit /b 1
    )
)

echo [INFO] Launching EmuWorld...
echo [INFO] The app window will appear shortly.
echo [INFO] Press Ctrl+C to stop.
echo.

call npm run tauri dev
