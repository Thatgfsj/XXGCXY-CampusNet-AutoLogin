@echo off
REM ??bat??????
set "SCRIPT_DIR=%~dp0"

REM ??PowerShell 7???
set "PW7_PATH=%SCRIPT_DIR%_pw7_\pwsh.exe"
if not exist "%PW7_PATH%" set "PW7_PATH=%SCRIPT_DIR%bin\_pw7_\pwsh.exe"

REM ?????????pwsh
if exist "%PW7_PATH%" (
    REM ?????PowerShell 7????
    "%PW7_PATH%" -ExecutionPolicy Bypass -File "%SCRIPT_DIR%xywdl.ps1"
) else (
    REM ?????????pwsh
    where pwsh >nul 2>&1
    if %errorlevel%==0 (
        pwsh -ExecutionPolicy Bypass -File "%SCRIPT_DIR%xywdl.ps1"
    ) else (
        REM ??????Windows PowerShell 5
        powershell -ExecutionPolicy Bypass -File "%SCRIPT_DIR%xywdl.ps1"
    )
)
