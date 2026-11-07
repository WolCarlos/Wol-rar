@echo off
setlocal
cd /d "%~dp0"
if not exist "rust\target\release\wol-rar.exe" (
    cargo build --release --manifest-path rust\Cargo.toml
    if errorlevel 1 goto :error
)
start "" "rust\target\release\wol-rar.exe"
exit /b 0
:error
pause
