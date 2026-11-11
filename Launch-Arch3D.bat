@echo off
title Arch3D
cd /d "%~dp0"
echo Launching Arch3D Architectural ^& Structural Modeler...
taskkill /f /im arch3d.exe >nul 2>&1
cargo run --bin arch3d
pause
