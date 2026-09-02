###############################################################################
#  新乡工程学院校园网登录脚本 (v1.9.0+)
#
#  与旧版的区别 (v1.8.x 兼容层已移除):
#    1. 不再交互式读 Read-Host 取账号/密码
#    2. 不再自动检测 portal 重定向
#    3. 配置全部从 JSON 模板读取: %APPDATA%/xxgcxy-wifi/login_profile.json
#    4. 密码从 DPAPI 加密的 .bin 读取,本脚本自动解密
#    5. 运营商由 profile.operator 字段决定 (yd/lt/dx)
#    6. (v2.0.0+) 请求发送三层降级:
#       ① PowerShell Invoke-WebRequest (默认主力)
#       ② src/sender/xywdl_sender.exe (C#, .NET Framework 4.x, Win7+ 自带)
#       ③ src/sender/sender.py (Python 3, 纯标准库, 跨平台最强保底)
#       PS 发送失败自动降级到下一层, 三层全失败才报错。
#
#  兼容:
#    - pwsh 7.x  (推荐)
#    - powershell 5.1  (Windows PS 5,UTF-8 BOM 读取时已确认可用)
###############################################################################

[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
# 注意: 用不带 BOM 的 UTF8 作为 $OutputEncoding。
# 若用 [System.Text.Encoding]::UTF8 (带 BOM), 管道传给原生进程
# (C#/Python sender) 时会叠加 [Console]::OutputEncoding 的 BOM,
# 导致 URL 开头出现两个 BOM 字符, 被 sender 判为"无效 URI"。
$OutputEncoding = New-Object System.Text.UTF8Encoding($false)
chcp 65001 | Out-Null

# ============= PS 版本检查 =============
# 我们依赖 PowerShell 5.1+ (Windows 10/11 自带; Win 7 需装 WMF 5.1)
# 关键 API: ProtectedData (.NET 4.0+), Get-WmiObject (Win NT 4.0+),
#          Invoke-WebRequest (PS 3.0+), ConvertFrom-Json (PS 3.0+)

if ($PSVersionTable.PSVersion.Major -lt 5) {
    Write-Host "[!] 需要 PowerShell 5.1 或更高版本, 当前是 $($PSVersionTable.PSVersion)" -ForegroundColor Red
    Write-Host "    Windows 7/8 用户需要手动安装 WMF 5.1:" -ForegroundColor Yellow
    Write-Host "      https://www.microsoft.com/en-us/download/details.aspx?id=54616" -ForegroundColor Yellow
    exit 1
}
if ($PSVersionTable.PSVersion.Major -eq 5 -and $PSVersionTable.PSVersion.Minor -lt 1) {
    Write-Host "[!] 需要 PowerShell 5.1 或更高版本, 当前是 $($PSVersionTable.PSVersion)" -ForegroundColor Red
    Write-Host "    Windows 7/8 用户需要手动安装 WMF 5.1:" -ForegroundColor Yellow
    Write-Host "      https://www.microsoft.com/en-us/download/details.aspx?id=54616" -ForegroundColor Yellow
    exit 1
}

# ============= 路径 =============

$AppDataDir = Join-Path $env:APPDATA "xxgcxy-wifi"
$ProfilePath = Join-Path $AppDataDir "login_profile.json"
$CredPath = Join-Path $AppDataDir "login_credential.bin"

# ============= 日志文件 (今天写, 昨天留, 前天删) =============
# 路径: %APPDATA%\xxgcxy-wifi\logs\xywdl-YYYY-MM-DD.log
# 启动时清理 > 1 天的日志 (即只保留今天 + 昨天)
$LogsDir = Join-Path $AppDataDir "logs"
if (-not (Test-Path $LogsDir)) {
    New-Item -ItemType Directory -Path $LogsDir -Force | Out-Null
}
$today = Get-Date -Format "yyyy-MM-dd"
$yesterday = (Get-Date).AddDays(-1).ToString("yyyy-MM-dd")
Get-ChildItem -Path $LogsDir -Filter "xywdl-*.log" -ErrorAction SilentlyContinue | ForEach-Object {
    $dateStr = $_.BaseName -replace '^xywdl-', ''
    if ($dateStr -ne $today -and $dateStr -ne $yesterday) {
        Remove-Item $_.FullName -Force -ErrorAction SilentlyContinue
    }
}
# 用 Start-Transcript 把所有 Write-Host 输出同时写到当天日志
$LogFile = Join-Path $LogsDir "xywdl-$today.log"
Start-Transcript -Path $LogFile -Append -NoClobber -ErrorAction SilentlyContinue | Out-Null

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
        if ([string]::IsNullOrWhiteSpace($content)) {
            Write-Host "[!] 登录配置文件为空: $ProfilePath" -ForegroundColor Red
            Write-Host "    请在 UI 中重新保存配置。" -ForegroundColor Yellow
            return $null
        }
        $json = $content | ConvertFrom-Json -ErrorAction Stop

        # 烟囱 29.1 修复: PSObject.Properties 在 $json 不是 hashtable/object (如纯数字 12345) 时返回 $null
        if ($null -eq $json) {
            Write-Host "[!] 登录配置不是有效的对象: $ProfilePath" -ForegroundColor Red
            Write-Host "    请在 UI 中重新保存配置。" -ForegroundColor Yellow
            return $null
        }
        if (-not ($json -is [hashtable] -or $json -is [PSCustomObject] -or $json.PSObject -ne $null)) {
            Write-Host "[!] 登录配置不是有效的对象: $ProfilePath (类型: $($json.GetType().Name))" -ForegroundColor Red
            Write-Host "    请在 UI 中重新保存配置。" -ForegroundColor Yellow
            return $null
        }
        # 拿到字段名 (兼容 hashtable 和 PSCustomObject)
        $fieldNames = if ($json -is [hashtable]) { $json.Keys } else { $json.PSObject.Properties.Name }

        $required = @("user_id", "operator", "base_url", "vlan")
        # wlan_user_ip / mac_address / ssid 是可选的: UI 允许留空,
        # 运行时由 Get-WifiIpAddress() / Get-WirelessMacAddress() / Get-CurrentSsid() 自动取本地值兜底
        foreach ($f in $required) {
            if (-not ($fieldNames -contains $f) -or [string]::IsNullOrWhiteSpace($json.$f)) {
                Write-Host "[!] 登录配置缺少字段: $f" -ForegroundColor Red
                return $null
            }
        }
        # 软必填: wlan_ac_name / wlan_ac_ip 缺失时, 尝试从 base_url 触发 portal.do 重定向,
        # 从 Location header 的 ?wlanacname=...&wlanacIp=... 提取并回填。
        # 提取不到时:
        #   - wlan_ac_ip: 用 base_url 的 host 兜底 (校园网 AC 通常就是 portal host)
        #   - wlan_ac_name: 留空 (服务器可能拒, 但流程能跑下去)
        $softRequired = @("wlan_ac_name", "wlan_ac_ip")
        $missingSoft = @()
        foreach ($f in $softRequired) {
            if (-not ($fieldNames -contains $f) -or [string]::IsNullOrWhiteSpace($json.$f)) {
                $missingSoft += $f
            }
        }
        if ($missingSoft.Count -gt 0) {
            Write-Host "[*] 检测到可选字段缺失: $($missingSoft -join ', ') (将尝试自动探测)" -ForegroundColor Yellow
            $autoFilled = Get-AutoPortalParams -baseUrl $json.base_url
            if ($autoFilled) {
                foreach ($f in $missingSoft) {
                    if ($autoFilled.ContainsKey($f) -and -not [string]::IsNullOrWhiteSpace($autoFilled[$f])) {
                        $json.$f = $autoFilled[$f]
                        Write-Host "    [✓] 自动填入 $f = $($autoFilled[$f])" -ForegroundColor Green
                    }
                }
            }
            # 兜底: wlan_ac_ip 如果还是空, 用 base_url 的 host 代替
            if ($missingSoft -contains "wlan_ac_ip") {
                if (-not ($fieldNames -contains "wlan_ac_ip") -or [string]::IsNullOrWhiteSpace($json.wlan_ac_ip)) {
                    $hostFallback = $null
                    try {
                        $tmpUri = [System.Uri]::new($json.base_url)
                        $hostFallback = $tmpUri.Host
                    } catch {}
                    if ($hostFallback) {
                        $json.wlan_ac_ip = $hostFallback
                        Write-Host "    [~] 兜底: wlan_ac_ip = base_url host ($hostFallback)" -ForegroundColor Cyan
                    } else {
                        if (-not ($json.PSObject.Properties.Name -contains "wlan_ac_ip")) {
                            $json | Add-Member -NotePropertyName "wlan_ac_ip" -NotePropertyValue "" -Force
                        }
                        Write-Host "    [!] wlan_ac_ip 留空 (可能登录会被服务器拒绝)" -ForegroundColor DarkYellow
                    }
                }
            }
            # wlan_ac_name 兜底: 留空
            if ($missingSoft -contains "wlan_ac_name") {
                if (-not ($json.PSObject.Properties.Name -contains "wlan_ac_name") -or [string]::IsNullOrWhiteSpace($json.wlan_ac_name)) {
                    $json | Add-Member -NotePropertyName "wlan_ac_name" -NotePropertyValue "" -Force
                    Write-Host "    [!] wlan_ac_name 留空 (可能登录会被服务器拒绝)" -ForegroundColor DarkYellow
                }
            }
        }
        return $json
    } catch {
        Write-Host "[!] 读取登录配置失败: $($_.Exception.Message)" -ForegroundColor Red
        return $null
    }
}

# ============= 自动探测 portal 参数 (wlan_ac_name / wlan_ac_ip) =============
# 当用户保存配置时漏填了 wlan_ac_name / wlan_ac_ip, 尝试向 portal.do 发一次 HTTP GET,
# 校园网 AC 通常会 302 重定向到 portal.do?wlanacname=...&wlanacIp=...&vlan=...&mac=...
# 我们从 Location header 抓出 wlanacname / wlanacIp 回填到 profile。
# 失败返回 $null (调用方会降级为空串)。

function Get-AutoPortalParams {
    param([string]$baseUrl)
    if ([string]::IsNullOrWhiteSpace($baseUrl)) {
        return $null
    }
    # 归一化: 如果 baseUrl 没有 http(s):// 前缀, 补上
    $probeUrl = $baseUrl.Trim()
    if ($probeUrl -notmatch '^https?://') {
        $probeUrl = "http://$probeUrl"
    }
    Write-Host "    [*] 自动探测中: GET $probeUrl" -ForegroundColor DarkCyan
    try {
        # 不跟随重定向, 我们只读 Location header
        $req = [System.Net.HttpWebRequest]::Create($probeUrl)
        $req.Method = "GET"
        $req.Timeout = 8000
        $req.ReadWriteTimeout = 8000
        $req.AllowAutoRedirect = $false
        $req.UserAgent = "xxgcxy-wifi/2.0.3"
        $req.KeepAlive = $false
        $resp = $null
        try {
            $resp = $req.GetResponse()
        } catch [System.Net.WebException] {
            # 3xx 重定向被 .NET 视作 WebException
            $resp = $_.Exception.Response
        }
        if ($null -eq $resp) {
            Write-Host "    [!] 探测无响应" -ForegroundColor DarkYellow
            return $null
        }
        $location = $resp.Headers["Location"]
        $resp.Close()
        if ([string]::IsNullOrWhiteSpace($location)) {
            Write-Host "    [!] 响应无 Location header" -ForegroundColor DarkYellow
            return $null
        }
        Write-Host "    [*] 收到重定向: $location" -ForegroundColor DarkCyan
        # 解析 query string
        $uri = [System.Uri]::new($location)
        $query = $uri.Query.TrimStart('?')
        if ([string]::IsNullOrWhiteSpace($query)) {
            return $null
        }
        $result = @{}
        foreach ($kv in $query.Split('&')) {
            $parts = $kv.Split('=', 2)
            if ($parts.Count -ne 2) { continue }
            $k = [System.Uri]::UnescapeDataString($parts[0]).ToLower()
            $v = [System.Uri]::UnescapeDataString($parts[1])
            switch ($k) {
                "wlanacname" { $result["wlan_ac_name"] = $v }
                "wlanacip"   { $result["wlan_ac_ip"]   = $v }
            }
        }
        if ($result.Count -eq 0) {
            Write-Host "    [!] Location 中无 wlanacname / wlanacIp 参数" -ForegroundColor DarkYellow
            return $null
        }
        return $result
    } catch {
        Write-Host "    [!] 自动探测异常: $($_.Exception.Message)" -ForegroundColor DarkYellow
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
        Write-Host "    可能原因: credential 文件损坏 / 密码是另一台电脑/账号加密的" -ForegroundColor Yellow
        Write-Host "    解决: 在 UI 中重新保存配置" -ForegroundColor Yellow
        return $null
    }
}

# ============= 网络接口 (Win 8+ / Win 7 兼容) =============
# Get-NetAdapter / Get-NetIPAddress 是 Win 8+ 才有, Win 7 必须 fallback
# Win 7 用 WMI Win32_NetworkAdapter (Win 95+ 都有, 最稳)
# 注意: 我们的代码优先用 Get-NetAdapter (Win 8+), 不行才 fallback

# 检测 NetAdapter module 是否可用
$script:HasNetAdapterModule = $null
function Test-NetAdapterModule {
    if ($null -eq $script:HasNetAdapterModule) {
        try {
            $null = Get-Command Get-NetAdapter -ErrorAction Stop
            $script:HasNetAdapterModule = $true
        } catch {
            $script:HasNetAdapterModule = $false
        }
    }
    return $script:HasNetAdapterModule
}

# 检测 NetTCPIP module 是否可用 (Win 8+)
$script:HasNetTCPIPModule = $null
function Test-NetTCPIPModule {
    if ($null -eq $script:HasNetTCPIPModule) {
        try {
            $null = Get-Command Get-NetIPAddress -ErrorAction Stop
            $script:HasNetTCPIPModule = $true
        } catch {
            $script:HasNetTCPIPModule = $false
        }
    }
    return $script:HasNetTCPIPModule
}

# 找"正在 Up 状态的 WiFi/无线网卡" - 优先用 Get-NetAdapter, fallback 到 WMI
function Get-WirelessAdapter {
    # 路径 1: Win 8+ Get-NetAdapter
    if (Test-NetAdapterModule) {
        try {
            $ad = Get-NetAdapter | Where-Object {
                ($_.InterfaceDescription -match 'Wi-Fi|Wireless|WLAN|802\.11') -and
                $_.Status -eq 'Up' -and
                $_.Name -notmatch 'Virtual|VMware|Hyper-V|VirtualBox'
            } | Select-Object -First 1
            if ($ad) { return $ad }
        } catch {}
    }
    # 路径 2: Win 7+ WMI Win32_NetworkAdapter (Win 95+ 都有, 最稳)
    try {
        # WMI 在 PS 5.1 都能用, CIM 在 PS 6+ 才有
        $wmi = Get-WmiObject Win32_NetworkAdapter |
            Where-Object {
                $_.NetEnabled -eq $true -and
                ($_.Name -match 'Wi-Fi|Wireless|WLAN|802\.11') -and
                $_.Name -notmatch 'Virtual|VMware|Hyper-V|VirtualBox'
            } | Select-Object -First 1
        if ($wmi) { return $wmi }
    } catch {}
    return $null
}

function Get-WirelessMacAddress {
    $ad = Get-WirelessAdapter
    if ($null -eq $ad) { return $null }
    # Get-NetAdapter 用 .MacAddress, WMI 用 .MACAddress (大写)
    $raw = $ad.MACAddress
    if ([string]::IsNullOrEmpty($raw)) { $raw = $ad.MacAddress }
    if ([string]::IsNullOrEmpty($raw)) { return $null }
    # 标准化为小写冒号分隔 (aa:bb:cc:dd:ee:ff)
    $mac = ($raw -replace '[-]', ':').ToLower()
    if ($mac -notmatch '^([0-9a-f]{2}:){5}[0-9a-f]{2}$') {
        $mac = ($raw -replace '[-.]', ':').ToLower()
    }
    return $mac
}

function Get-WifiIpAddress {
    $ad = Get-WirelessAdapter
    if ($null -eq $ad) { return $null }
    # 路径 1: Win 8+ Get-NetIPAddress (如果有 IfIndex 字段)
    if (Test-NetTCPIPModule -and $ad.PSObject.Properties.Name -contains 'IfIndex') {
        try {
            $ip = Get-NetIPAddress -InterfaceIndex $ad.IfIndex -AddressFamily IPv4 -ErrorAction Stop
            if ($ip) { return $ip.IPAddress }
        } catch {}
    }
    # 路径 2: WMI Win32_NetworkAdapterConfiguration (Win NT 4.0+ 都有)
    try {
        $idx = $null
        if ($ad.PSObject.Properties.Name -contains 'Index') { $idx = $ad.Index }
        elseif ($ad.PSObject.Properties.Name -contains 'InterfaceIndex') { $idx = $ad.InterfaceIndex }
        if ($null -ne $idx) {
            $cfg = Get-WmiObject Win32_NetworkAdapterConfiguration |
                Where-Object { $_.Index -eq $idx }
            if ($cfg -and $cfg.IPAddress) {
                # WMI IPAddress 是字符串数组, 取第一个 IPv4
                foreach ($ip in $cfg.IPAddress) {
                    if ($ip -match '^\d+\.\d+\.\d+\.\d+$') { return $ip }
                }
            }
        }
    } catch {}
    # 路径 3: 兜底 ipconfig
    try {
        $ipconfig = ipconfig
        $line = $ipconfig | Select-String -Pattern 'IPv4'
        if ($line) {
            $match = [regex]::Match($line.ToString(), '(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})')
            if ($match.Success) { return $match.Groups[1].Value }
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
        # 复用 Get-WirelessAdapter (Win 8+ / Win 7 WMI fallback)
        $adapter = Get-WirelessAdapter
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
    Write-Host ""
    Write-Host "[*] 开始执行登录流程..." -ForegroundColor Yellow
    Write-Host ""

    # 1. 拿运行时网络信息
    Write-Host "[步骤 1/5] 拿网络信息..." -ForegroundColor Cyan
    $localIp = Get-WifiIpAddress
    $localMac = Get-WirelessMacAddress
    if ($localIp)  { $wlanUserIp = $localIp }  else { $wlanUserIp = $profile.wlan_user_ip }
    if ($localMac) { $macAddress = $localMac } else { $macAddress = $profile.mac_address }
    # ssid 可选: 留空时运行时自动检测当前连接的 WiFi
    $ssid = if (-not [string]::IsNullOrWhiteSpace($profile.ssid)) { $profile.ssid } else { Get-CurrentSsid }
    Write-Host "    IP=$wlanUserIp MAC=$macAddress SSID=$ssid" -ForegroundColor Gray
    Write-Host "[步骤 1/5] 完成" -ForegroundColor Green
    Write-Host ""

    # 2. WiFi 状态检查
    Write-Host "[步骤 2/5] 检查 WiFi..." -ForegroundColor Cyan
    if (-not (Is-SsidConnected $profile.ssid)) {
        Write-Host "    不匹配,重连中..." -ForegroundColor Yellow
        Reconnect-ToSsid $profile.ssid
    } else {
        Write-Host "    已连" -ForegroundColor Gray
    }
    Write-Host "[步骤 2/5] 完成" -ForegroundColor Green
    Write-Host ""

    # 3. 构造 quickauth.do URL
    Write-Host "[步骤 3/5] 构造 URL..." -ForegroundColor Cyan
    $hostname = if ($profile.hostname) { $profile.hostname } else { $env:COMPUTERNAME }
    $portalPageId = if ($profile.portal_page_id) { $profile.portal_page_id } else { "3" }
    $portalType   = if ($profile.portal_type)    { $profile.portal_type}    else { "0" }
    $version      = if ($profile.version)        { $profile.version }        else { "0" }
    $bindCtrlId   = if ($profile.bind_ctrl_id)   { $profile.bind_ctrl_id}   else { "" }

    $authUrl = $profile.base_url -replace '/\w+\.do', '/quickauth.do'
    Write-Host "    $authUrl" -ForegroundColor Gray

    # 烟囱 29.2 修复: [Uri]::EscapeDataString 对超长字符串会抛 UriFormatException
    # 用 try/catch 包裹, 失败时退化为 PowerShell 自带 [uri]::EscapeDataString / [Web.HttpUtility]::UrlEncode
    # 如果都失败, 返回空字符串, 但不中断整个登录流程
    function Safe-UriEscape {
        param([string]$s)
        try {
            return [Uri]::EscapeDataString($s)
        } catch {
            # PS 7+ 有 [uri]::EscapeDataString 也可用
            try { return [uri]::EscapeDataString($s) } catch { return "" }
        }
    }
    function Safe-UriUnescape {
        param([string]$s)
        try {
            return [Uri]::UnescapeDataString($s)
        } catch { return "" }
    }

    $queryParams = @(
        "userid=$(Safe-UriEscape $profile.user_id)",
        "passwd=$(Safe-UriEscape $password)",
        "wlanuserip=$(Safe-UriEscape $wlanUserIp)",
        "wlanacname=$(Safe-UriEscape $profile.wlan_ac_name)",
        "wlanacIp=$(Safe-UriEscape $profile.wlan_ac_ip)",
        "ssid=$(Safe-UriEscape $ssid)",
        "vlan=$(Safe-UriEscape $profile.vlan)",
        "mac=$(Safe-UriEscape $macAddress)",
        "version=$(Safe-UriEscape $version)",
        "portalpageid=$(Safe-UriEscape $portalPageId)",
        "timestamp=$([int](Get-Date -UFormat %s) * 1000)",
        "uuid=$([guid]::NewGuid().ToString())",
        "portaltype=$(Safe-UriEscape $portalType)",
        "hostname=$(Safe-UriEscape $hostname)",
        "bindCtrlId=$(Safe-UriEscape $bindCtrlId)"
    ) -join "&"
    $requestUrl = $authUrl + "?" + $queryParams
    # 安全: 不要在日志/控制台打印明文密码, 只显示脱敏后的 passwd=***
    $maskedUrl = $requestUrl -replace '(?i)(passwd=)[^&]*', '$1***'
    Write-Host "[步骤 3/5] 完成" -ForegroundColor Green
    Write-Host ""

    # 4. 发送 (三层降级: PowerShell → C# sender → Python sender)
    # 每层失败都会记录原因, 自动尝试下一层; 只有三层全失败才报错。
    Write-Host "[步骤 4/5] 发送..." -ForegroundColor Cyan
    $body = $null
    $sendSource = ""
    $sendErrors = @()

    # 4.1 第 1 层: PowerShell Invoke-WebRequest (默认主力)
    Write-Host "    [L1] PowerShell..." -ForegroundColor Gray
    try {
        $response = Invoke-WebRequest -Uri $requestUrl -Method Get -UseBasicParsing -TimeoutSec 30 -ErrorAction Stop -Proxy $null
        $body = $response.Content
        $sendSource = "PowerShell (Invoke-WebRequest)"
        Write-Host "    [L1] 成功 ($($body.Length) 字符)" -ForegroundColor Green
    } catch [System.Net.WebException] {
        $errMsg = $_.Exception.Message
        $statusCode = ""
        if ($_.Exception.Response -ne $null -and $_.Exception.Response.StatusCode -ne $null) {
            $statusCode = " (HTTP $($_.Exception.Response.StatusCode))"
        }
        $sendErrors += "PowerShell: $errMsg$statusCode"
        Write-Host "    [L1] 失败: $errMsg$statusCode" -ForegroundColor Red
    } catch {
        $sendErrors += "PowerShell: $($_.Exception.Message)"
        Write-Host "    [L1] 失败: $($_.Exception.Message)" -ForegroundColor Red
    }

    # 4.2 第 2 层: C# sender (xywdl_sender.exe, .NET Framework 4.x, Win7+ 自带运行时)
    if ($null -eq $body) {
        Write-Host "    [L2] C# sender..." -ForegroundColor Gray
        $senderExe = $null
        foreach ($cand in @(
                (Join-Path $PSScriptRoot "src\sender\xywdl_sender.exe"),
                (Join-Path $PSScriptRoot "xywdl_sender.exe")
            )) {
            if (Test-Path -LiteralPath $cand) { $senderExe = $cand; break }
        }
        if ($senderExe) {
            Write-Host "      找到发送器: $senderExe" -ForegroundColor Gray
            try {
                # 完整 URL 通过 stdin 传给 sender, 避免明文密码出现在进程命令行
                $rawOut = $requestUrl | & $senderExe 2>&1
                $exitCode = $LASTEXITCODE
                if ($exitCode -eq 0) {
                    $body = ($rawOut | Out-String).Trim()
                    $sendSource = "C# ($([System.IO.Path]::GetFileName($senderExe)))"
                    Write-Host "    [L2] 成功 ($($body.Length) 字符)" -ForegroundColor Green
                } else {
                    $errDetail = ($rawOut | Out-String).Trim()
                    $sendErrors += "C#: exit=$exitCode $errDetail"
                    Write-Host "    [L2] 失败: $errDetail" -ForegroundColor Red
                }
            } catch {
                $sendErrors += "C#: $($_.Exception.Message)"
                Write-Host "    [L2] 异常: $($_.Exception.Message)" -ForegroundColor Red
            }
        } else {
            $sendErrors += "C#: 未找到 xywdl_sender.exe (检查 src/sender/ 目录)"
            Write-Host "    [L2] 未找到 xywdl_sender.exe" -ForegroundColor Yellow
        }
    }

    # 4.3 第 3 层: Python sender (sender.py, 纯标准库, 跨平台最强保底)
    if ($null -eq $body) {
        Write-Host "    [L3] Python..." -ForegroundColor Gray
        $pyScript = $null
        foreach ($cand in @(
                (Join-Path $PSScriptRoot "src\sender\sender.py"),
                (Join-Path $PSScriptRoot "sender.py")
            )) {
            if (Test-Path -LiteralPath $cand) { $pyScript = $cand; break }
        }
        # 找可用的 python: 逐个候选做"能真正运行"的验证,
        # 跳过 WindowsApps 商店占位 stub (运行返回 9009 的假 python)
        $py = $null
        $pyArgs = @()
        $pyCandidates = @(
            @{ Name = "py";    Args = @("-3") },
            @{ Name = "python"; Args = @() },
            @{ Name = "python3"; Args = @() }
        )
        foreach ($c in $pyCandidates) {
            $cmd = Get-Command $c.Name -ErrorAction SilentlyContinue
            if (-not $cmd) {
                Write-Host "    Python 候选 '$($c.Name)': 未找到" -ForegroundColor DarkGray
                continue
            }
            try {
                $probe = & $cmd.Source @($c.Args) -c "import sys; print('PYOK')" 2>$null
                if ($LASTEXITCODE -eq 0 -and ($probe -join '') -match 'PYOK') {
                    $py = $cmd.Source
                    $pyArgs = @($c.Args)
                    Write-Host "    使用 Python: $($py.Name) (参数: $($pyArgs -join ' '))" -ForegroundColor Gray
                    break
                }
                Write-Host "    Python 候选 '$($c.Name)': 探测失败 (exit=$LASTEXITCODE)" -ForegroundColor DarkGray
                $sendErrors += "Python: $($c.Name) 不可用 (exit=$LASTEXITCODE)"
            } catch {
                Write-Host "    Python 候选 '$($c.Name)': 启动失败" -ForegroundColor DarkGray
                $sendErrors += "Python: $($c.Name) 启动失败: $($_.Exception.Message)"
            }
        }
        if ($pyScript -and $py) {
            try {
                Write-Host "    发送请求: $maskedUrl" -ForegroundColor DarkGray
                $rawOut = $requestUrl | & $py @pyArgs $pyScript 2>&1
                $exitCode = $LASTEXITCODE
                if ($exitCode -eq 0) {
                    $body = ($rawOut | Out-String).Trim()
                    $sendSource = "Python (sender.py)"
                    Write-Host "    [L3] 成功 ($($body.Length) 字符)" -ForegroundColor Green
                } else {
                    $errDetail = ($rawOut | Out-String).Trim()
                    $sendErrors += "Python: exit=$exitCode $errDetail"
                    Write-Host "    [L3] 失败: $errDetail" -ForegroundColor Red
                }
            } catch {
                $sendErrors += "Python: $($_.Exception.Message)"
                Write-Host "    [L3] 异常: $($_.Exception.Message)" -ForegroundColor Red
            }
        } else {
            $sendErrors += "Python: 未找到可用的 python 解释器或 sender.py"
            Write-Host "    [L3] 未找到可用的 python 解释器或 sender.py" -ForegroundColor Yellow
        }
    }
    Write-Host "[步骤 4/5] 完成" -ForegroundColor Green
    Write-Host ""

    # 5. 判定结果 (三层共用同一套判定逻辑)
    Write-Host "[步骤 5/5] 判定..." -ForegroundColor Cyan
    if ($null -eq $body) {
        Write-Host "[!] 三层全部失败:" -ForegroundColor Red
        foreach ($e in $sendErrors) { Write-Host "    - $e" -ForegroundColor Red }
        return 99
    }

    # 提取响应摘要 (不打印整个 JSON, 太长且重复)
    # 找 "code":"X" 和 "message":"..."
    $respCode = ""
    $respMsg = ""
    if ($body -match '"code"\s*:\s*"([^"]*)"') { $respCode = $Matches[1] }
    if ($body -match '"message"\s*:\s*"([^"]*)"') { $respMsg = $Matches[1] }
    Write-Host "    发送层: $sendSource  code=$respCode  msg=$respMsg" -ForegroundColor Cyan
    Write-Host "[步骤 5/5] 完成" -ForegroundColor Green
    Write-Host ""

    # 注意: code 匹配必须锚定 "后面不能紧跟数字", 否则 "code":10/100/123 会被误判成
    # "code":1 (账号不存在), "code":440 会被误判成 "code":44 (非法接入)。
    if ($body -match '"code"\s*:\s*0(?!\d)' -or $body -match "success" -or $body -match "认证成功") {
        Write-Host "[+] 认证成功" -ForegroundColor Green
        return 0
    } elseif ($body -match '"code"\s*:\s*1(?!\d)' -or $body -match "账号不存在") {
        Write-Host "[!] 认证失败:账号不存在" -ForegroundColor Red
        return 1
    } elseif ($body -match '"code"\s*:\s*44(?!\d)' -or $body -match "非法接入") {
        Write-Host "[!] 认证失败:非法接入" -ForegroundColor Red
        return 44
    } else {
        Write-Host "[!] 认证结果未知" -ForegroundColor Yellow
        return 99
    }
}

# ============= 主流程 =============

try {
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host "  校园网自动登录脚本 (v1.9.0+)" -ForegroundColor Cyan
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "[*] 步骤 0: 加载登录配置..." -ForegroundColor Yellow
    $profile = Load-LoginProfile
    if ($null -eq $profile) {
        Write-Host "[!] 卡在: 步骤 0 - 登录配置加载失败" -ForegroundColor Red
        Write-Host "    可能原因: 配置缺失、密码文件不存在、JSON 格式错误" -ForegroundColor Yellow
        exit 2
    }
    # 步骤 0 内部已经打印过配置, 这里只打一行 summary
    Write-Host "[*] 配置 OK: 学号=$($profile.user_id) VLAN=$($profile.vlan) AC=$($profile.wlan_ac_name) IP=$($profile.wlan_ac_ip)" -ForegroundColor Green
    Write-Host ""

    Write-Host "[*] 步骤 0.5: 解密密码..." -ForegroundColor Yellow
    $password = Get-LoginPassword
    if ($null -eq $password) {
        Write-Host "[!] 卡在: 步骤 0.5 - 密码解密失败" -ForegroundColor Red
        Write-Host "    可能原因: credential 文件损坏、DPAPI 加密钥匙不匹配" -ForegroundColor Yellow
        exit 3
    }
    Write-Host "[*] 密码解密 OK" -ForegroundColor Green
    Write-Host ""

    $code = Invoke-CampusLogin -profile $profile -password $password
    Write-Host ""
    if ($code -eq 0) {
        Write-Host "[+] 登录成功!" -ForegroundColor Green
    } elseif ($code -eq 1) {
        Write-Host "[!] 登录失败: 账号不存在" -ForegroundColor Red
    } elseif ($code -eq 44) {
        Write-Host "[!] 登录失败: 非法接入 (VLAN/MAC 不匹配)" -ForegroundColor Red
    } elseif ($code -eq 99) {
        Write-Host "[!] 登录失败: 未知错误" -ForegroundColor Red
    } else {
        Write-Host "[!] 登录失败: 未知返回码 $code" -ForegroundColor Red
    }

    if ($args -contains '--non-interactive') {
        # 自动调用模式:直接退出,带返回码
        exit $code
    } else {
        # 交互模式:暂停
        Read-Host "`n按 Enter 键退出" | Out-Null
        exit $code
    }
} catch {
    Write-Host ""
    Write-Host "[!] 脚本执行出错: $($_.Exception.Message)" -ForegroundColor Red
    Write-Host "    $($_.ScriptStackTrace)" -ForegroundColor DarkGray
    Write-Host "[!] 卡在: 脚本异常中断" -ForegroundColor Red
    if ($args -notcontains '--non-interactive') {
        Read-Host "`n按 Enter 键退出" | Out-Null
    }
    exit 1
}
