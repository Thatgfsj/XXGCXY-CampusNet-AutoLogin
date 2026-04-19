@echo off
setlocal DisableDelayedExpansion

REM ============================================
REM 校园网自动登录脚本启动器 (系统PS7版)
REM ============================================

set "SCRIPT_DIR=%~dp0"
set "PS_SCRIPT=%SCRIPT_DIR%xywdl.ps1"
set "PW7_PATH="
set "PS_VERSION="

REM 去除路径末尾的空格和反斜杠（如果有）
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

REM ============================================
REM 注意：本版本不包含内置 PowerShell 7
REM 只会使用系统安装的 PowerShell
REM ============================================

REM 尝试查找系统 PowerShell 7 (pwsh)
where pwsh >nul 2>&1
if %errorlevel%==0 (
    echo [信息] 找到系统 PowerShell 7 (pwsh)
    set "PW7_PATH=pwsh"
) else (
    echo [警告] 未找到系统 PowerShell 7 (pwsh)
    echo [信息] 尝试使用 Windows PowerShell 5.x...
)

REM 执行 PowerShell 脚本
if defined PW7_PATH (
    echo [执行] pwsh -ExecutionPolicy Bypass -File "%PS_SCRIPT%"
    pwsh -ExecutionPolicy Bypass -File "%PS_SCRIPT%"
) else (
    REM 回退到 Windows PowerShell 5.x
    echo [执行] powershell -ExecutionPolicy Bypass -File "%PS_SCRIPT%"
    powershell -ExecutionPolicy Bypass -File "%PS_SCRIPT%"
)

set "EXIT_CODE=%errorlevel%"

if %EXIT_CODE% neq 0 (
    echo.
    echo [错误] 脚本执行失败，错误码: %EXIT_CODE%
    echo.
    echo 常见问题:
    echo   1. 确保已连接校园网 WiFi
    echo   2. 检查 PowerShell 是否正常工作
    echo   3. 建议安装 PowerShell 7 以获得更好体验
    echo   4. 尝试以管理员身份运行
    pause
    exit /b %EXIT_CODE%
)

exit /b 0
