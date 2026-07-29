@echo off
chcp 65001 >nul
echo =======================================================
echo   XXGCXY-WiFi 诊断脚本 (v1.9.0+)
echo   目的: 排查"找不到 WiFi"问题的根因
echo =======================================================
echo.

echo [1/6] 检查 netsh 是否可用
where netsh >nul 2>&1
if %ERRORLEVEL% NEQ 0 (
    echo   [FAIL] netsh 不在 PATH, 系统环境异常
) else (
    echo   [OK] netsh 已找到
)
echo.

echo [2/6] 检查 WLAN AutoConfig 服务 (WiFi 必需)
sc query "WlanSvc" | findstr /C:"STATE" 
echo.

echo [3/6] 列出所有网络适配器
powershell -NoProfile -Command "Get-NetAdapter | Format-Table Name, InterfaceDescription, Status -AutoSize"
echo.

echo [4/6] 检查 WiFi 接口
netsh wlan show interfaces
echo.

echo [5/6] 触发一次扫描
netsh wlan scan
timeout /t 3 >nul
echo.

echo [6/6] 查看扫描到的网络
netsh wlan show networks mode=bssid
echo.

echo =======================================================
echo   诊断完毕
echo   - 如果 [4/6] 提示"系统上没有无线接口" → 电脑没 WiFi
echo   - 如果 [4/6] 列出接口但 status 非 Up → 驱动/硬件问题
echo   - 如果 [2/6] WlanSvc STOPPED → netsh 服务被关,跑: sc config WlanSvc start= auto ^& sc start WlanSvc
echo   - 如果 [6/6] 列表为空但前 5 步都正常 → 检查 WiFi 飞行模式
echo =======================================================
pause
