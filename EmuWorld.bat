@echo off
title EmuWorld Launcher
cd /d "%~dp0"

echo.
echo  ============================
echo   EmuWorld - Starting...
echo  ============================
echo.

:: Check if Node.js is available
where node >nul 2>&1
if %errorlevel% neq 0 (
    echo [ERROR] Node.js is not installed or not in PATH.
    echo Please install Node.js from https://nodejs.org/
    pause
    exit /b 1
)

:: Check if Rust/Cargo is available
where cargo >nul 2>&1
if %errorlevel% neq 0 (
    echo [ERROR] Rust is not installed or not in PATH.
    echo Please install Rust from https://www.rust-lang.org/
    pause
    exit /b 1
)

:: Install dependencies if needed
if not exist "node_modules" (
    echo [INFO] Installing dependencies...
    npm install
    if %errorlevel% neq 0 (
        echo [ERROR] Failed to install dependencies.
        pause
        exit /b 1
    )
)

echo [INFO] Launching EmuWorld in development mode...
echo [INFO] The app window will appear shortly.
echo [INFO] Press Ctrl+C in this window to stop the app.
echo.

npm run tauri dev
