@echo off
REM 获取bat文件所在目录
set "SCRIPT_DIR=%~dp0"

REM 查找PowerShell 7便携版（按优先级查找）
set "PW7_PATH="

REM 1. 尝试 bat 同目录下的 _pw7_
if exist "%SCRIPT_DIR%_pw7_\pwsh.exe" (
    set "PW7_PATH=%SCRIPT_DIR%_pw7_\pwsh.exe"
)

REM 2. 尝试父目录的 bin\_pw7_（安装后的实际位置）
if not defined PW7_PATH if exist "%SCRIPT_DIR%..\bin\_pw7_\pwsh.exe" (
    set "PW7_PATH=%SCRIPT_DIR%..\bin\_pw7_\pwsh.exe"
)

REM 3. 尝试父目录的 _pw7_
if not defined PW7_PATH if exist "%SCRIPT_DIR%..\_pw7_\pwsh.exe" (
    set "PW7_PATH=%SCRIPT_DIR%..\_pw7_\pwsh.exe"
)

REM 4. 尝试 bin\_pw7_
if not defined PW7_PATH if exist "%SCRIPT_DIR%bin\_pw7_\pwsh.exe" (
    set "PW7_PATH=%SCRIPT_DIR%bin\_pw7_\pwsh.exe"
)

REM 执行脚本
if defined PW7_PATH (
    REM 使用便携版PowerShell 7
    "%PW7_PATH%" -ExecutionPolicy Bypass -File "%SCRIPT_DIR%xywdl.ps1"
    REM 成功时关闭窗口，失败时暂停让用户看到错误
    if %errorlevel%==0 exit 0
    pause
    exit %errorlevel%
) else (
    REM 尝试系统pwsh
    where pwsh >nul 2>&1
    if %errorlevel%==0 (
        pwsh -ExecutionPolicy Bypass -File "%SCRIPT_DIR%xywdl.ps1"
        if %errorlevel%==0 exit 0
        pause
        exit %errorlevel%
    ) else (
        REM 回退到Windows PowerShell 5
        powershell -ExecutionPolicy Bypass -File "%SCRIPT_DIR%xywdl.ps1"
        if %errorlevel%==0 exit 0
        pause
        exit %errorlevel%
    )
)