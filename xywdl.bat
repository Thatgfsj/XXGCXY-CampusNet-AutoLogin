@echo off
setlocal DisableDelayedExpansion

REM ============================================
REM 校园网自动登录脚本启动器
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

REM 查找 PowerShell 7 便携版
if exist "%SCRIPT_DIR%\_pw7_\pwsh.exe" set "PW7_PATH=%SCRIPT_DIR%\_pw7_\pwsh.exe"
if not defined PW7_PATH if exist "%SCRIPT_DIR%\..\bin\_pw7_\pwsh.exe" set "PW7_PATH=%SCRIPT_DIR%\..\bin\_pw7_\pwsh.exe"
if not defined PW7_PATH if exist "%SCRIPT_DIR%\..\_pw7_\pwsh.exe" set "PW7_PATH=%SCRIPT_DIR%\..\_pw7_\pwsh.exe"
if not defined PW7_PATH if exist "%SCRIPT_DIR%\bin\_pw7_\pwsh.exe" set "PW7_PATH=%SCRIPT_DIR%\bin\_pw7_\pwsh.exe"

REM 如果找到 PowerShell 7，验证版本
if defined PW7_PATH (
    echo [信息] 找到 PowerShell 7: %PW7_PATH%
    "%PW7_PATH%" -Command "$PSVersionTable.PSVersion.ToString()" >nul 2>&1
    if errorlevel 1 (
        echo [警告] PowerShell 7 无法执行，将尝试其他版本
        set "PW7_PATH="
    ) else (
        echo [信息] 使用 PowerShell 7
    )
)

REM 如果没有找到 PowerShell 7，尝试系统 pwsh
if not defined PW7_PATH (
    where pwsh >nul 2>&1
    if %errorlevel%==0 (
        echo [信息] 使用系统 PowerShell 7 (pwsh)
        set "PW7_PATH=pwsh"
    )
)

REM 执行 PowerShell 脚本
if defined PW7_PATH (
    if "%PW7_PATH%"=="pwsh" (
        echo [执行] pwsh -ExecutionPolicy Bypass -File "%PS_SCRIPT%"
        pwsh -ExecutionPolicy Bypass -File "%PS_SCRIPT%"
    ) else (
        echo [执行] "%PW7_PATH%" -ExecutionPolicy Bypass -File "%PS_SCRIPT%"
        "%PW7_PATH%" -ExecutionPolicy Bypass -File "%PS_SCRIPT%"
    )
) else (
    REM 回退到 Windows PowerShell 5.x
    echo [信息] 未找到 PowerShell 7，使用系统 PowerShell
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
    echo   3. 尝试以管理员身份运行
    pause
    exit /b %EXIT_CODE%
)

exit /b 0
