###############################################################################
#  新乡工程学院校园网登录脚本 (v1.9.0+)
#
#  与旧版的区别 (v1.8.x 兼容层已移除):
#    1. 不再交互式读 Read-Host 取账号/密码
#    2. 不再自动检测 portal 重定向
#    3. 配置全部从 JSON 模板读取: %APPDATA%/xxgcxy-wifi/login_profile.json
#    4. 密码从 DPAPI 加密的 .bin 读取,本脚本自动解密
#    5. 运营商由 profile.operator 字段决定 (yd/lt/dx)
#
#  兼容:
#    - pwsh 7.x  (推荐)
#    - powershell 5.1  (Windows PS 5,UTF-8 BOM 读取时已确认可用)
###############################################################################

[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$OutputEncoding = [System.Text.Encoding]::UTF8
chcp 65001 | Out-Null

# ============= 路径 =============

$AppDataDir = Join-Path $env:APPDATA "xxgcxy-wifi"
$ProfilePath = Join-Path $AppDataDir "login_profile.json"
$CredPath = Join-Path $AppDataDir "login_credential.bin"

# ============= 工具函数 =============

function Get-LoginDir {
    if (-not (Test-Path $AppDataDir)) {
        New-Item -ItemType Directory -Path $AppDataDir -Force | Out-Null
    }
    return $AppDataDir
}

function Get-OperatorName {
    param([string]$code)
    switch ($code) {
        "yd" { return "移动" }
        "lt" { return "联通" }
        "dx" { return "电信" }
        default { return "未知" }
    }
}

function Get-OperatorSuffix {
    param([string]$code)
    switch ($code) {
        "yd" { return "@xxgcyd" }
        "lt" { return "@xxgclt" }
        "dx" { return "@xxgcdx" }
        default { return "" }
    }
}

# ============= LoginProfile 读取 =============

function Load-LoginProfile {
    $dir = Get-LoginDir
    if (-not (Test-Path $ProfilePath)) {
        Write-Host "[!] 未找到登录配置: $ProfilePath" -ForegroundColor Red
        Write-Host "    请先在 UI 主页或网络配置页填写校园网账号信息。" -ForegroundColor Yellow
        return $null
    }
    if (-not (Test-Path $CredPath)) {
        Write-Host "[!] 未找到加密密码文件: $CredPath" -ForegroundColor Red
        Write-Host "    请重新在 UI 中保存配置。" -ForegroundColor Yellow
        return $null
    }
    try {
        $content = Get-Content -Path $ProfilePath -Raw -Encoding UTF8 -ErrorAction Stop
        if ([string]::IsNullOrWhiteSpace($content)) { return $null }
        $json = $content | ConvertFrom-Json -ErrorAction Stop

        $required = @("user_id", "operator", "base_url", "vlan", "mac_address")
        # wlan_user_ip 是可选的,运行时由 Get-WifiIpAddress() 自动取本地 IP 兜底
        foreach ($f in $required) {
            if (-not $json.PSObject.Properties.Name.Contains($f) -or [string]::IsNullOrWhiteSpace($json.$f)) {
                Write-Host "[!] 登录配置缺少字段: $f" -ForegroundColor Red
                return $null
            }
        }
        return $json
    } catch {
        Write-Host "[!] 读取登录配置失败: $($_.Exception.Message)" -ForegroundColor Red
        return $null
    }
}

# ============= 密码解密 =============

function Get-LoginPassword {
    try {
        # 加载 .NET ProtectedData (Windows 专属,System.Security.Cryptography.ProtectedData 在 System.Security.dll)
        Add-Type -AssemblyName System.Security -ErrorAction Stop
    } catch {
        Write-Host "[!] 加载 System.Security 程序集失败: $($_.Exception.Message)" -ForegroundColor Red
        return $null
    }
    try {
        # v1.9.0+ 简化: 文件就是裸 DPAPI blob (ProtectedData::Protect 输出字节流)
        # 与 PS 端 ConvertFrom-SecureString / Rust 端 CryptProtectData 输出一致
        $bytes = [System.IO.File]::ReadAllBytes($CredPath)
        if ($bytes.Length -lt 16) {
            throw "credential 文件过短 ($($bytes.Length) 字节)"
        }

        # DPAPI 解密 (无 entropy, 走 CurrentUser scope)
        $plain = [System.Security.Cryptography.ProtectedData]::Unprotect(
            $bytes,
            $null,
            [System.Security.Cryptography.DataProtectionScope]::CurrentUser
        )
        # 解出的是 UTF-16 LE 字节,还原成字符串
        $text = [System.Text.Encoding]::Unicode.GetString($plain)
        return $text.TrimEnd("`0")
    } catch {
        Write-Host "[!] 解密密码失败: $($_.Exception.Message)" -ForegroundColor Red
        return $null
    }
}

# ============= 网络接口 =============

function Get-WirelessMacAddress {
    try {
        $ad = Get-NetAdapter | Where-Object {
            ($_.InterfaceDescription -match 'Wi-Fi|Wireless|WLAN') -and
            $_.Status -eq 'Up' -and
            $_.Name -notmatch 'Virtual|VMware|Hyper-V|VirtualBox'
        } | Select-Object -First 1
        if ($ad) {
            $mac = ($ad.MacAddress -replace '[-:]', ':').ToLower()
            if ($mac -notmatch '^([0-9a-f]{2}:){5}[0-9a-f]{2}$') {
                $mac = ($ad.MacAddress -replace '[-.]', ':').ToLower()
            }
            return $mac
        }
    } catch {}
    return $null
}

function Get-WifiIpAddress {
    try {
        $ad = Get-NetAdapter | Where-Object {
            ($_.InterfaceDescription -match 'Wi-Fi|Wireless|WLAN') -and
            $_.Status -eq 'Up' -and
            $_.Name -notmatch 'Virtual|VMware|Hyper-V|VirtualBox'
        } | Select-Object -First 1
        if ($ad) {
            $ip = Get-NetIPAddress -InterfaceIndex $ad.IfIndex -AddressFamily IPv4 -ErrorAction Stop
            return $ip.IPAddress
        }
    } catch {}
    return $null
}

function Get-CurrentSsid {
    try {
        $wifi = netsh wlan show interfaces | Out-String
        if ($wifi -match 'SSID\s*:\s*(.+)') {
            return $matches[1].Trim()
        }
    } catch {}
    return $null
}

function Is-SsidConnected {
    param([string]$targetSsid)
    if ([string]::IsNullOrEmpty($targetSsid)) { return $true }
    $current = Get-CurrentSsid
    if ($current -eq $targetSsid) {
        $adapter = Get-NetAdapter | Where-Object {
            ($_.InterfaceDescription -match 'Wi-Fi|Wireless|WLAN') -and
            $_.Status -eq 'Up' -and
            $_.Name -notmatch 'Virtual|VMware|Hyper-V|VirtualBox'
        } | Select-Object -First 1
        return ($null -ne $adapter)
    }
    return $false
}

function Reconnect-ToSsid {
    param([string]$targetSsid)
    if ([string]::IsNullOrEmpty($targetSsid)) { return }
    Write-Host "[*] 当前已断开 $targetSsid,尝试重连..." -ForegroundColor Yellow
    try {
        $profiles = netsh wlan show profiles | Out-String
        if ($profiles -notmatch $targetSsid) {
            Write-Host "[!] 未找到已保存的 WiFi 配置: $targetSsid" -ForegroundColor Red
            return
        }
        netsh wlan disconnect | Out-Null
        Start-Sleep -Milliseconds 500
        netsh wlan connect name="$targetSsid" | Out-Null
        Write-Host "[*] 正在连接 $targetSsid ..." -ForegroundColor Cyan
        for ($i = 0; $i -lt 15; $i++) {
            Start-Sleep -Seconds 1
            if (Is-SsidConnected $targetSsid) {
                Write-Host "[+] WiFi 重连成功" -ForegroundColor Green
                return
            }
        }
        Write-Host "[!] WiFi 重连超时" -ForegroundColor Red
    } catch {
        Write-Host "[!] WiFi 重连失败: $($_.Exception.Message)" -ForegroundColor Red
    }
}

# ============= 认证 =============

function Invoke-CampusLogin {
    param($profile, [string]$password)

    Write-Host ""
    Write-Host "===== 新乡工程学院校园网登录 (v1.9.0+) =====" -ForegroundColor Cyan
    Write-Host "    账号: $($profile.user_id)" -ForegroundColor White
    Write-Host "    运营商: $(Get-OperatorName $profile.operator)" -ForegroundColor White
    Write-Host "    认证地址: $($profile.base_url)" -ForegroundColor White
    Write-Host "    SSID: $($profile.ssid)" -ForegroundColor White
    Write-Host ""

    # 1. 拿运行时网络信息
    $localIp = Get-WifiIpAddress
    $localMac = Get-WirelessMacAddress
    if ($localIp)  { $wlanUserIp = $localIp }  else { $wlanUserIp = $profile.wlan_user_ip }
    if ($localMac) { $macAddress = $localMac } else { $macAddress = $profile.mac_address }

    # 2. WiFi 状态检查
    if (-not (Is-SsidConnected $profile.ssid)) {
        Reconnect-ToSsid $profile.ssid
    }

    # 3. 构造 quickauth.do URL
    $hostname = if ($profile.hostname) { $profile.hostname } else { $env:COMPUTERNAME }
    $portalPageId = if ($profile.portal_page_id) { $profile.portal_page_id } else { "3" }
    $portalType   = if ($profile.portal_type)    { $profile.portal_type }    else { "0" }
    $version      = if ($profile.version)        { $profile.version }        else { "0" }
    $bindCtrlId   = if ($profile.bind_ctrl_id)   { $profile.bind_ctrl_id }   else { "" }

    $authUrl = $profile.base_url -replace '/\w+\.do', '/quickauth.do'

    $queryParams = @(
        "userid=$([Uri]::EscapeDataString($profile.user_id))",
        "passwd=$([Uri]::EscapeDataString($password))",
        "wlanuserip=$wlanUserIp",
        "wlanacname=$([Uri]::EscapeDataString($profile.wlan_ac_name))",
        "wlanacIp=$($profile.wlan_ac_ip)",
        "ssid=$($profile.ssid)",
        "vlan=$($profile.vlan)",
        "mac=$macAddress",
        "version=$version",
        "portalpageid=$portalPageId",
        "timestamp=$([int](Get-Date -UFormat %s) * 1000)",
        "uuid=$([guid]::NewGuid().ToString())",
        "portaltype=$portalType",
        "hostname=$([Uri]::EscapeDataString($hostname))",
        "bindCtrlId=$bindCtrlId"
    ) -join "&"
    $requestUrl = $authUrl + "?" + $queryParams
    Write-Host "[*] 请求: $requestUrl" -ForegroundColor Gray

    # 4. 发送
    try {
        $response = Invoke-WebRequest -Uri $requestUrl -Method Get -UseBasicParsing -TimeoutSec 15 -ErrorAction Stop -Proxy $null
        $body = $response.Content
        Write-Host "[*] HTTP $($response.StatusCode): $body" -ForegroundColor White

        if ($body -match '"code"\s*:\s*0' -or $body -match "success" -or $body -match "认证成功") {
            Write-Host "[+] 认证成功,已连接到互联网" -ForegroundColor Green
            return 0
        } elseif ($body -match '"code"\s*:\s*1' -or $body -match "账号不存在") {
            Write-Host "[!] 认证失败:账号不存在,请检查学号和运营商" -ForegroundColor Red
            return 1
        } elseif ($body -match '"code"\s*:\s*44' -or $body -match "非法接入") {
            Write-Host "[!] 认证失败:非法接入,请检查 VLAN / MAC" -ForegroundColor Red
            return 44
        } else {
            Write-Host "[!] 认证结果未知,请检查账号密码" -ForegroundColor Yellow
            return 99
        }
    } catch {
        Write-Host "[!] 认证请求失败: $($_.Exception.Message)" -ForegroundColor Red
        return 99
    }
}

# ============= 主流程 =============

try {
    $profile = Load-LoginProfile
    if ($null -eq $profile) {
        exit 2
    }
    $password = Get-LoginPassword
    if ($null -eq $password) {
        exit 3
    }
    $code = Invoke-CampusLogin -profile $profile -password $password
    if ($args -contains '--non-interactive') {
        # 自动调用模式:直接退出,带返回码
        exit $code
    } else {
        # 交互模式:暂停
        Read-Host "`n按 Enter 键退出" | Out-Null
        exit $code
    }
} catch {
    Write-Host "[!] 脚本执行出错: $($_.Exception.Message)" -ForegroundColor Red
    Write-Host "    $($_.ScriptStackTrace)" -ForegroundColor DarkGray
    if ($args -notcontains '--non-interactive') {
        Read-Host "`n按 Enter 键退出" | Out-Null
    }
    exit 1
}
