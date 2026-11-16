@echo off
echo Building Arch3D for WebAssembly (wasm32-unknown-unknown)...
cargo build --target wasm32-unknown-unknown --release -p plinth-app --lib
if errorlevel 1 goto error

echo Generating wasm-bindgen web package in docs\wasm...
tools\wasm-bindgen-0.2.130-x86_64-pc-windows-msvc\wasm-bindgen.exe target\wasm32-unknown-unknown\release\plinth_app.wasm --out-dir docs\wasm --target web --no-typescript
if errorlevel 1 goto error

echo ========================================================
echo Arch3D WebAssembly Studio Build Successful!
echo Open docs\app.html in browser to run the EXACT desktop app.
echo ========================================================
goto end

:error
echo [ERROR] WebAssembly build failed.
exit /b 1

:end
