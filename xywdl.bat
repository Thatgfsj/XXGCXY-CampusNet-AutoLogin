@echo off
setlocal enabledelayedexpansion
set "SCRIPT_DIR=%~dp0"
set "PW7_PATH="

if exist "%SCRIPT_DIR%_pw7_\pwsh.exe" set "PW7_PATH=%SCRIPT_DIR%_pw7_\pwsh.exe"
if not defined PW7_PATH if exist "%SCRIPT_DIR%..\bin\_pw7_\pwsh.exe" set "PW7_PATH=%SCRIPT_DIR%..\bin\_pw7_\pwsh.exe"
if not defined PW7_PATH if exist "%SCRIPT_DIR%..\_pw7_\pwsh.exe" set "PW7_PATH=%SCRIPT_DIR%..\_pw7_\pwsh.exe"
if not defined PW7_PATH if exist "%SCRIPT_DIR%bin\_pw7_\pwsh.exe" set "PW7_PATH=%SCRIPT_DIR%bin\_pw7_\pwsh.exe"

if defined PW7_PATH (
    "%PW7_PATH%" -ExecutionPolicy Bypass -File "%SCRIPT_DIR%xywdl.ps1"
    if !errorlevel!==0 exit 0
    pause
    exit !errorlevel!
)

where pwsh >nul 2>&1
if %errorlevel%==0 (
    pwsh -ExecutionPolicy Bypass -File "%SCRIPT_DIR%xywdl.ps1"
    if !errorlevel!==0 exit 0
    pause
    exit !errorlevel!
)

powershell -ExecutionPolicy Bypass -File "%SCRIPT_DIR%xywdl.ps1"
if !errorlevel!==0 exit 0
pause
exit !errorlevel!
