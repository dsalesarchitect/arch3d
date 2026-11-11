@echo off
title Plinth BIM
cd /d "%~dp0"
echo Launching Plinth BIM...
taskkill /f /im plinth.exe >nul 2>&1
cargo run --bin plinth
pause
