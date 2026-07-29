@echo off
chcp 65001 >nul
setlocal DisableDelayedExpansion

REM ============================================
REM 校园网自动登录脚本启动器 (v1.9.0+ 简化版)
REM 不再内置 PowerShell 7, 依赖系统 PowerShell 5.1+ (Windows 10/11 自带)
REM Windows 7 用户需要装 WMF 5.1:
REM   https://www.microsoft.com/en-us/download/details.aspx?id=54616
REM ============================================

set "SCRIPT_DIR=%~dp0"
set "PS_SCRIPT=%SCRIPT_DIR%xywdl.ps1"
set "PS_EXE="

REM 去除路径末尾的反斜杠
for %%a in ("%SCRIPT_DIR%") do set "SCRIPT_DIR=%%~fa"
set "SCRIPT_DIR=%SCRIPT_DIR:~0,-1%"

echo [信息] 脚本目录: %SCRIPT_DIR%
echo.

REM 检查 PowerShell 脚本是否存在
if not exist "%PS_SCRIPT%" (
    echo [错误] 找不到 PowerShell 脚本: %PS_SCRIPT%
    echo.
    echo 请确保 xywdl.ps1 与 xywdl.bat 在同一目录下
    pause
    exit /b 1
)

REM 优先用 PowerShell 7 (如果用户装过, 提供更好体验)
where pwsh >nul 2>&1
if %errorlevel%==0 (
    set "PS_EXE=pwsh"
    echo [信息] 使用 PowerShell 7 (pwsh)
) else (
    REM 回退到 Windows PowerShell 5.1 (Win10/11 自带)
    set "PS_EXE=powershell"
    echo [信息] 使用 Windows PowerShell (5.1, 系统自带)
)

REM 检测 PS 版本 (兼容 Win 7 默认 PS 2.0)
echo [信息] 检测 PowerShell 版本...
for /f "delims=" %%v in ('%PS_EXE% -NoProfile -Command "$PSVersionTable.PSVersion.ToString()" 2^>nul') do set "PS_VER=%%v"
if "%PS_VER%"=="" (
    echo [错误] 无法检测 PowerShell 版本, 请检查 PowerShell 是否正常工作
    echo   提示: 打开 cmd 跑 "powershell -Command \"`$PSVersionTable.PSVersion\""
    pause
    exit /b 1
)
echo [信息] PowerShell 版本: %PS_VER%

REM 检查最低版本 5.1
set "PS_MAJOR="
set "PS_MINOR="
for /f "tokens=1,2 delims=." %%a in ("%PS_VER%") do (
    set "PS_MAJOR=%%a"
    set "PS_MINOR=%%b"
)
if %PS_MAJOR% LSS 5 (
    echo [错误] 需要 PowerShell 5.1 或更高版本, 当前是 %PS_VER%
    echo.
    echo Windows 7 用户需要手动安装 WMF 5.1:
    echo   https://www.microsoft.com/en-us/download/details.aspx?id=54616
    echo Windows 10/11 用户已自带 PowerShell 5.1, 如果检测不到请检查 PATH
    pause
    exit /b 1
)
if %PS_MAJOR% EQU 5 if %PS_MINOR% LSS 1 (
    echo [错误] 需要 PowerShell 5.1 或更高版本, 当前是 %PS_VER%
    echo.
    echo Windows 7/8 用户需要手动安装 WMF 5.1:
    echo   https://www.microsoft.com/en-us/download/details.aspx?id=54616
    pause
    exit /b 1
)

REM 执行
echo [执行] %PS_EXE% -ExecutionPolicy Bypass -File "%PS_SCRIPT%" %*
%PS_EXE% -ExecutionPolicy Bypass -File "%PS_SCRIPT%" %*
set "EXIT_CODE=%errorlevel%"

if %EXIT_CODE% neq 0 (
    echo.
    echo [错误] 脚本执行失败，错误码: %EXIT_CODE%
    echo.
    echo 常见问题:
    echo   1. 确保已连接校园网 WiFi
    echo   2. 检查 PowerShell 是否正常工作 (在 cmd 跑 powershell -Command "$PSVersionTable.PSVersion")
    echo   3. 尝试以管理员身份运行
    pause
    exit /b %EXIT_CODE%
)

exit /b 0
