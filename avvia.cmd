@echo off
setlocal
cd /d "%~dp0"
set "PYTHONPATH=%~dp0src"
where py >nul 2>nul
if %errorlevel%==0 (
    py -3 -m wolrar
) else (
    python -m wolrar
)
if errorlevel 1 pause
