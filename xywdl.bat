@echo off
setlocal DisableDelayedExpansion
set "SCRIPT_DIR=%~dp0"
set "PS_SCRIPT=%SCRIPT_DIR%xywdl.ps1"
set "PW7_PATH="

REM 检查 PowerShell 7 便携版
if exist "%SCRIPT_DIR%_pw7_\pwsh.exe" set "PW7_PATH=%SCRIPT_DIR%_pw7_\pwsh.exe"
if not defined PW7_PATH if exist "%SCRIPT_DIR%..\bin\_pw7_\pwsh.exe" set "PW7_PATH=%SCRIPT_DIR%..\bin\_pw7_\pwsh.exe"
if not defined PW7_PATH if exist "%SCRIPT_DIR%..\_pw7_\pwsh.exe" set "PW7_PATH=%SCRIPT_DIR%..\_pw7_\pwsh.exe"
if not defined PW7_PATH if exist "%SCRIPT_DIR%bin\_pw7_\pwsh.exe" set "PW7_PATH=%SCRIPT_DIR%bin\_pw7_\pwsh.exe"

REM 检查 PowerShell 脚本是否存在
if not exist "%PS_SCRIPT%" (
    echo [错误] 找不到脚本文件: %PS_SCRIPT%
    pause
    exit /b 1
)

REM 输出诊断信息
echo [信息] 脚本路径: %PS_SCRIPT%
echo.

if defined PW7_PATH (
    echo [信息] 使用 PowerShell 7: %PW7_PATH%
    "%PW7_PATH%" -ExecutionPolicy Bypass -File "%PS_SCRIPT%"
    if errorlevel 1 (
        echo [错误] PowerShell 7 执行失败，错误码: %errorlevel%
        pause
        exit /b %errorlevel%
    )
    exit /b 0
)

where pwsh >nul 2>&1
if %errorlevel%==0 (
    echo [信息] 使用系统 PowerShell 7 (pwsh)
    pwsh -ExecutionPolicy Bypass -File "%PS_SCRIPT%"
    if errorlevel 1 (
        echo [错误] pwsh 执行失败，错误码: %errorlevel%
        pause
        exit /b %errorlevel%
    )
    exit /b 0
)

echo [信息] 使用系统 PowerShell (powershell)
powershell -ExecutionPolicy Bypass -File "%PS_SCRIPT%"
if errorlevel 1 (
    echo [错误] powershell 执行失败，错误码: %errorlevel%
    pause
    exit /b %errorlevel%
)
exit /b 0
