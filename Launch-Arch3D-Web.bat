@echo off
title Arch3D Web Studio (Zero Install - Local-First)
cd /d "%~dp0"

echo ========================================================
echo       Starting Arch3D Web Studio (Local-First)
echo ========================================================
echo.
echo Launching local server at http://localhost:8080/app.html
echo Zero Cloud Storage -- 100%% Local Processing
echo.

node serve.js
if errorlevel 1 (
    echo [NOTICE] Starting fallback browser view...
    start "" "docs\app.html"
)
pause
