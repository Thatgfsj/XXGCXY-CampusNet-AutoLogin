if ($PSVersionTable.PSVersion.Major -lt 5 -or ($PSVersionTable.PSVersion.Major -eq 5 -and $PSVersionTable.PSVersion.Minor -lt 1)) {
    Write-Host "错误：本脚本需要PowerShell 5.1或更高版本。" -ForegroundColor Red
    Read-Host "按Enter退出"
    exit 1
}

class NetworkConfig {
    [string] $BaseURL
    [string] $WlanAcName
    [string] $WlanAcIp
    [string] $Ssid
    [string] $Version
    [string] $PortalPageId
    [string] $PortalType
    [string] $Hostname
    [string] $BindCtrlId
    [int]    $TimeoutSec
    [string] $UserId
    [SecureString] $Passwd
    [string] $Vlan
    [string] $WlanUserIp
    [string] $MacAddress
    [string] $Rand

    [int] $SuccessCode = 0
    [int] $UserNotFoundCode = 1
    [int] $IllegalAccessCode = 44

    NetworkConfig() {
        $this.BaseURL = ""
        $this.WlanAcName = ""
        $this.WlanAcIp = ""
        $this.Ssid = ""
        $this.Version = "0"
        $this.PortalPageId = "3"
        $this.PortalType = "0"
        $this.Hostname = $env:COMPUTERNAME
        $this.BindCtrlId = ""
        $this.TimeoutSec = 30
        $this.UserId = ""
        $this.Passwd = $null
        $this.Vlan = ""
        $this.WlanUserIp = ""
        $this.MacAddress = ""
        $this.Rand = ""
    }
}

class DomainConfig {
    static [hashtable] GetDomainOptions() {
        return @{
            "1" = "@xxgcyd"
            "2" = "@xxgclt"
            "3" = "@xxgcdx"
        }
    }

    static [string] GetDomainName([string]$suffix) {
        if ($suffix -eq "@xxgcyd") { return "移动" }
        if ($suffix -eq "@xxgclt") { return "联通" }
        if ($suffix -eq "@xxgcdx") { return "电信" }
        return "未知"
    }
}

class ConfigManager {
    [string] $ConfigFilePath

    ConfigManager([string]$path) {
        $this.ConfigFilePath = $path
    }

    [hashtable] LoadConfig() {
        if (-not (Test-Path $this.ConfigFilePath)) { return $null }
        try {
            $content = Get-Content -Path $this.ConfigFilePath -Raw -ErrorAction Stop
            if ([string]::IsNullOrWhiteSpace($content)) { return $null }
            $config = ConvertFrom-Json $content -ErrorAction Stop

            $requiredFields = @("BaseURL", "WlanAcName", "WlanUserIp", "MacAddress", "Vlan")
            foreach ($field in $requiredFields) {
                if (-not $config.PSObject.Properties.Name.Contains($field)) {
                    return $null
                }
            }

            $password = ""
            if ($config.PSObject.Properties.Name.Contains('EncryptedPassword')) {
                $secureString = ConvertTo-SecureString -String $config.EncryptedPassword -ErrorAction Stop
                $bstr = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($secureString)
                $password = [Runtime.InteropServices.Marshal]::PtrToStringBSTR($bstr)
                [Runtime.InteropServices.Marshal]::FreeBSTR($bstr)
            }

            return @{
                BaseURL = $config.BaseURL
                WlanAcName = $config.WlanAcName
                WlanAcIp = if ($config.PSObject.Properties.Name.Contains('WlanAcIp')) { $config.WlanAcIp } else { "" }
                Ssid = if ($config.PSObject.Properties.Name.Contains('Ssid')) { $config.Ssid } else { "" }
                Version = if ($config.PSObject.Properties.Name.Contains('Version')) { $config.Version } else { "0" }
                PortalPageId = if ($config.PSObject.Properties.Name.Contains('PortalPageId')) { $config.PortalPageId } else { "3" }
                PortalType = if ($config.PSObject.Properties.Name.Contains('PortalType')) { $config.PortalType } else { "0" }
                Hostname = if ($config.PSObject.Properties.Name.Contains('Hostname')) { $config.Hostname } else { $env:COMPUTERNAME }
                BindCtrlId = if ($config.PSObject.Properties.Name.Contains('BindCtrlId')) { $config.BindCtrlId } else { "" }
                UserId = if ($config.PSObject.Properties.Name.Contains('UserId')) { $config.UserId } else { "" }
                Password = $password
                Vlan = $config.Vlan
                WlanUserIp = $config.WlanUserIp
                MacAddress = $config.MacAddress
                Rand = if ($config.PSObject.Properties.Name.Contains('Rand')) { $config.Rand } else { "" }
            }
        }
        catch {
            Write-Host "读取配置文件失败: $($_.Exception.Message)" -ForegroundColor Yellow
            return $null
        }
    }

    [void] SaveConfig([NetworkConfig]$config, [string]$password) {
        try {
            $securePassword = ConvertTo-SecureString -String $password -AsPlainText -Force
            $encryptedPassword = ConvertFrom-SecureString -SecureString $securePassword

            $configObject = [PSCustomObject]@{
                BaseURL = $config.BaseURL
                WlanAcName = $config.WlanAcName
                WlanAcIp = $config.WlanAcIp
                Ssid = $config.Ssid
                Version = $config.Version
                PortalPageId = $config.PortalPageId
                PortalType = $config.PortalType
                Hostname = $config.Hostname
                BindCtrlId = $config.BindCtrlId
                UserId = $config.UserId
                EncryptedPassword = $encryptedPassword
                Vlan = $config.Vlan
                WlanUserIp = $config.WlanUserIp
                MacAddress = $config.MacAddress
                Rand = $config.Rand
            }

            $dir = Split-Path $this.ConfigFilePath -Parent
            if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Path $dir -Force | Out-Null }

            $json = $configObject | ConvertTo-Json -Compress
            Set-Content -Path $this.ConfigFilePath -Value $json -Encoding UTF8 -NoNewline
            (Get-Item $this.ConfigFilePath).Attributes = [System.IO.FileAttributes]::Hidden

            Write-Host "配置已保存到: $this.ConfigFilePath" -ForegroundColor Green
        }
        catch {
            Write-Host "保存配置失败: $($_.Exception.Message)" -ForegroundColor Red
        }
    }
}

class NetworkInterfaceHelper {
    [string] GetWirelessMacAddress() {
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

    [string] GetWifiIpAddress() {
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
}

class RedirectUrlParser {
    static [hashtable] ParseRedirectUrl([string]$url) {
        $result = @{}

        try {
            $decodedUrl = [Uri]::UnescapeDataString($url)

            if ($decodedUrl -match '^(http://[^/]+(/\w+\.do))') {
                $result['BaseURL'] = $matches[1]
            }

            if ($decodedUrl -match '\?(.*)$') {
                $queryString = $matches[1]
                $params = $queryString -split '&'

                foreach ($param in $params) {
                    $kv = $param -split '=', 2
                    if ($kv.Length -eq 2) {
                        $key = $kv[0]
                        $value = [Uri]::UnescapeDataString($kv[1])

                        switch ($key) {
                            "wlanuserip" { $result['WlanUserIp'] = $value }
                            "wlanacname" { $result['WlanAcName'] = $value }
                            "wlanacIp" { $result['WlanAcIp'] = $value }
                            "mac" { $result['MacAddress'] = $value.ToLower() }
                            "vlan" { $result['Vlan'] = $value }
                            "hostname" { $result['Hostname'] = $value }
                            "rand" { $result['Rand'] = $value }
                            "url" { $result['Url'] = $value }
                        }
                    }
                }
            }

            return $result
        }
        catch {
            Write-Host "解析URL失败: $($_.Exception.Message)" -ForegroundColor Red
            return $null
        }
    }
}

class AuthenticationClient {
    [NetworkConfig] $Config
    [ConfigManager] $ConfigMgr
    [NetworkInterfaceHelper] $NetworkHelper
    [bool] $IsAutoDetected

    AuthenticationClient([NetworkConfig]$c, [ConfigManager]$cm, [NetworkInterfaceHelper]$nh) {
        $this.Config = $c
        $this.ConfigMgr = $cm
        $this.NetworkHelper = $nh
        $this.IsAutoDetected = $false
    }

    [void] Run() {
        Write-Host "`n===== 新乡工程学院校园网登录脚本 =====" -ForegroundColor Cyan
        $savedConfig = $this.ConfigMgr.LoadConfig()
        if ($savedConfig) {
            Write-Host "`n已找到保存的配置，自动登录中..." -ForegroundColor Cyan
            $this.ApplyConfig($savedConfig)
            if (-not [string]::IsNullOrEmpty($savedConfig['Password'])) {
                $this.Config.Passwd = ConvertTo-SecureString $savedConfig['Password'] -AsPlainText -Force
            }
            $this.PerformAuthentication()
            return
        }

        Write-Host "`n=== 步骤1：自动获取登录参数 ===" -ForegroundColor Yellow
        Write-Host "正在尝试通过重定向获取登录信息..." -ForegroundColor White

        $autoParams = $this.TryAutoDetectParams()
        if ($autoParams) {
            $this.ApplyConfig($autoParams)
            $this.IsAutoDetected = $true
            Write-Host "自动获取成功！" -ForegroundColor Green
            $this.DisplayNetworkInfo()
        } else {
            Write-Host "`n请按以下步骤操作：" -ForegroundColor White
            Write-Host "1. 断开当前Wi-Fi重新连接" -ForegroundColor Gray
            Write-Host "2. 打开浏览器访问任意网站（如 www.qq.com）" -ForegroundColor Gray
            Write-Host "3. 浏览器会自动重定向到登录页面" -ForegroundColor Gray
            Write-Host "4. 复制浏览器地址栏中的完整URL地址" -ForegroundColor Gray
            Write-Host "5. 将URL粘贴到下面" -ForegroundColor Gray

            $manualUrl = Read-Host "`n请粘贴重定向URL"
            while ([string]::IsNullOrWhiteSpace($manualUrl) -or $manualUrl -notmatch '^http://' -or $manualUrl -notmatch '/portal\.do') {
                Write-Host "URL格式不正确，请输入包含 /portal.do 的重定向URL" -ForegroundColor Red
                $manualUrl = Read-Host "请重新粘贴重定向URL"
            }

            $manualParams = [RedirectUrlParser]::ParseRedirectUrl($manualUrl)
            if ($manualParams -and $manualParams['WlanUserIp']) {
                $this.ApplyConfig($manualParams)
                Write-Host "URL解析成功！" -ForegroundColor Green
                $this.DisplayNetworkInfo()
            } else {
                Write-Host "URL解析失败，请检查URL是否完整" -ForegroundColor Red
                Read-Host "按Enter退出"
                exit 1
            }
        }

        $this.PromptForCredentials()

        Write-Host "`n=== 是否保存配置？ ===" -ForegroundColor Yellow
        $save = Read-Host "保存后下次无需重新输入 (y/N)"
        if ($save -eq 'y' -or $save -eq 'Y') {
            $b = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($this.Config.Passwd)
            $plainPwd = [Runtime.InteropServices.Marshal]::PtrToStringBSTR($b)
            [Runtime.InteropServices.Marshal]::FreeBSTR($b)

            $this.ConfigMgr.SaveConfig($this.Config, $plainPwd)
        }

        $this.PerformAuthentication()
    }

    [hashtable] TryAutoDetectParams() {
        $response = $null

        try {
            Write-Host "方法1: 尝试访问 www.qq.com 捕获重定向..." -ForegroundColor Gray

            try {
                $response = Invoke-WebRequest -Uri "http://www.qq.com" -MaximumRedirection 0 -ErrorAction SilentlyContinue
            }
            catch {
                $response = $null
            }

            if ($response -and $response.Headers["Location"]) {
                $redirectUrl = $response.Headers["Location"]
                Write-Host "捕获到重定向: $redirectUrl" -ForegroundColor Gray
                return [RedirectUrlParser]::ParseRedirectUrl($redirectUrl)
            }
        }
        catch {
            Write-Host "方法1失败: $($_.Exception.Message)" -ForegroundColor Gray
        }

        try {
            Write-Host "方法2: 尝试直接访问校园网portal..." -ForegroundColor Gray

            $localIp = $this.NetworkHelper.GetWifiIpAddress()
            $localMac = $this.NetworkHelper.GetWirelessMacAddress()

            if (-not $localIp) {
                Write-Host "无法获取本地IP，请确保已连接Wi-Fi" -ForegroundColor Red
                return $null
            }

            $response = $null
            try {
                $response = Invoke-WebRequest -Uri "http://172.18.252.12:6060" -MaximumRedirection 0 -TimeoutSec 10 -ErrorAction SilentlyContinue
            }
            catch {
                $response = $null
            }

            if ($response -and $response.Headers["Location"]) {
                $redirectUrl = $response.Headers["Location"]
                Write-Host "捕获到重定向: $redirectUrl" -ForegroundColor Gray

                $params = [RedirectUrlParser]::ParseRedirectUrl($redirectUrl)
                if ($params) {
                    if (-not $params['WlanUserIp']) { $params['WlanUserIp'] = $localIp }
                    if (-not $params['MacAddress']) { $params['MacAddress'] = $localMac }
                    return $params
                }
            }
        }
        catch {
            Write-Host "方法2失败: $($_.Exception.Message)" -ForegroundColor Gray
        }

        return $null
    }

    [void] ApplyConfig([hashtable]$params) {
        if ($params['BaseURL']) { $this.Config.BaseURL = $params['BaseURL'] }
        if ($params['WlanAcName']) { $this.Config.WlanAcName = $params['WlanAcName'] }
        if ($params['WlanAcIp']) { $this.Config.WlanAcIp = $params['WlanAcIp'] }
        if ($params['Ssid']) { $this.Config.Ssid = $params['Ssid'] }
        if ($params['Version']) { $this.Config.Version = $params['Version'] }
        if ($params['PortalPageId']) { $this.Config.PortalPageId = $params['PortalPageId'] }
        if ($params['PortalType']) { $this.Config.PortalType = $params['PortalType'] }
        if ($params['Hostname']) { $this.Config.Hostname = $params['Hostname'] }
        if ($params['BindCtrlId']) { $this.Config.BindCtrlId = $params['BindCtrlId'] }
        if ($params['UserId']) { $this.Config.UserId = $params['UserId'] }
        if ($params['Vlan']) { $this.Config.Vlan = $params['Vlan'] }
        if ($params['WlanUserIp']) { $this.Config.WlanUserIp = $params['WlanUserIp'] }
        if ($params['MacAddress']) { $this.Config.MacAddress = $params['MacAddress'] }
        if ($params['Rand']) { $this.Config.Rand = $params['Rand'] }

        $localIp = $this.NetworkHelper.GetWifiIpAddress()
        $localMac = $this.NetworkHelper.GetWirelessMacAddress()
        if ($localIp) { $this.Config.WlanUserIp = $localIp }
        if ($localMac) { $this.Config.MacAddress = $localMac }
    }

    [void] DisplayNetworkInfo() {
        Write-Host "`n--- 当前网络信息 ---" -ForegroundColor Cyan
        Write-Host "  认证地址: $($this.Config.BaseURL)" -ForegroundColor White
        Write-Host "  AC名称:   $($this.Config.WlanAcName)" -ForegroundColor White
        Write-Host "  用户IP:  $($this.Config.WlanUserIp)" -ForegroundColor White
        Write-Host "  MAC地址: $($this.Config.MacAddress)" -ForegroundColor White
        Write-Host "  VLAN:    $($this.Config.Vlan)" -ForegroundColor White
        Write-Host "  主机名:  $($this.Config.Hostname)" -ForegroundColor White
    }

    [void] PromptForCredentials() {
        Write-Host "`n=== 步骤2：输入账号信息 ===" -ForegroundColor Yellow

        Write-Host "请选择运营商:" -ForegroundColor White
        $opt = [DomainConfig]::GetDomainOptions()
        foreach ($k in $opt.Keys | Sort-Object) {
            $name = [DomainConfig]::GetDomainName($opt[$k])
            Write-Host "  $k. $name ($($opt[$k]))"
        }

        $v = ""
        do {
            $v = Read-Host "请输入对应数字 (1/2/3)"
        } while ($opt.Keys -notcontains $v)
        $choice = $v

        $suffix = $opt[$choice]
        $operatorName = [DomainConfig]::GetDomainName($suffix)

        $id = Read-Host "请输入学号（纯数字）"
        while ($id -notmatch '^\d+$' -or [string]::IsNullOrWhiteSpace($id)) {
            Write-Host "学号必须是纯数字！" -ForegroundColor Red
            $id = Read-Host "请重新输入学号"
        }

        $this.Config.UserId = $id + $suffix
        Write-Host "完整账号: $($this.Config.UserId) ($operatorName)" -ForegroundColor Cyan

        do {
            $p1 = Read-Host "请输入校园网密码" -AsSecureString
            $p2 = Read-Host "请再次输入密码确认" -AsSecureString

            $b1 = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($p1)
            $s1 = [Runtime.InteropServices.Marshal]::PtrToStringBSTR($b1)
            [Runtime.InteropServices.Marshal]::FreeBSTR($b1)

            $b2 = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($p2)
            $s2 = [Runtime.InteropServices.Marshal]::PtrToStringBSTR($b2)
            [Runtime.InteropServices.Marshal]::FreeBSTR($b2)

            if ($s1 -eq $s2 -and -not [string]::IsNullOrWhiteSpace($s1)) {
                $this.Config.Passwd = $p1
                break
            } elseif ([string]::IsNullOrWhiteSpace($s1)) {
                Write-Host "密码不能为空！" -ForegroundColor Red
            } else {
                Write-Host "两次输入的密码不一致！" -ForegroundColor Red
            }
        } while ($true)
    }

    [void] PerformAuthentication() {
        Write-Host "`n=== 步骤3：开始认证 ===" -ForegroundColor Yellow
        Write-Host "MAC地址: $($this.Config.MacAddress)" -ForegroundColor Gray

        $b = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($this.Config.Passwd)
        $plainPwd = [Runtime.InteropServices.Marshal]::PtrToStringBSTR($b)

        try {
            $uuid = [guid]::NewGuid().ToString()
            $timestamp = [int](Get-Date -UFormat %s) * 1000

            $authUrl = $this.Config.BaseURL -replace '/\w+\.do', '/quickauth.do'

            $queryParams = @(
                "userid=$([Uri]::EscapeDataString($this.Config.UserId))",
                "passwd=$([Uri]::EscapeDataString($plainPwd))",
                "wlanuserip=$($this.Config.WlanUserIp)",
                "wlanacname=$([Uri]::EscapeDataString($this.Config.WlanAcName))",
                "wlanacIp=$($this.Config.WlanAcIp)",
                "ssid=$($this.Config.Ssid)",
                "vlan=$($this.Config.Vlan)",
                "mac=$($this.Config.MacAddress)",
                "version=$($this.Config.Version)",
                "portalpageid=$($this.Config.PortalPageId)",
                "timestamp=$timestamp",
                "uuid=$uuid",
                "portaltype=$($this.Config.PortalType)",
                "hostname=$([Uri]::EscapeDataString($this.Config.Hostname))",
                "bindCtrlId=$($this.Config.BindCtrlId)"
            ) -join "&"

            $requestUrl = $authUrl + "?" + $queryParams
            Write-Host "请求地址: $requestUrl" -ForegroundColor Gray

            $response = Invoke-WebRequest -Uri $requestUrl -Method Get -UseBasicParsing -TimeoutSec 15 -ErrorAction Stop

            Write-Host "`n=== 认证响应 ===" -ForegroundColor Cyan
            Write-Host "HTTP状态码: $($response.StatusCode)" -ForegroundColor Green
            Write-Host "响应内容: $($response.Content)" -ForegroundColor White

            if ($response.Content -match '"code"\s*:\s*0' -or $response.Content -match "success" -or $response.Content -match "认证成功") {
                Write-Host "`n认证成功！您已连接到互联网。" -ForegroundColor Green
                Write-Host "账号: $($this.Config.UserId)" -ForegroundColor Cyan
                exit 0
            } elseif ($response.Content -match '"code"\s*:\s*1' -or $response.Content -match "账号不存在") {
                Write-Host "`n认证失败：账号不存在，请检查学号和运营商是否正确" -ForegroundColor Red
            } elseif ($response.Content -match '"code"\s*:\s*44' -or $response.Content -match "非法接入") {
                Write-Host "`n认证失败：非法接入，请检查VLAN ID或MAC地址是否正确" -ForegroundColor Red
            } else {
                Write-Host "`n认证结果未知，请检查账号密码是否正确" -ForegroundColor Yellow
                Write-Host "响应: $($response.Content)" -ForegroundColor Gray
            }
        }
        catch {
            Write-Host "`n认证请求发送失败！" -ForegroundColor Red
            Write-Host "错误信息: $($_.Exception.Message)" -ForegroundColor DarkGray
        }
        finally {
            if ($b) { [Runtime.InteropServices.Marshal]::FreeBSTR($b) }
        }

        Read-Host "`n按 Enter 键退出脚本" | Out-Null
    }
}

try {
    $config = [NetworkConfig]::new()
    $configMgr = [ConfigManager]::new((Join-Path $env:APPDATA "xxgc_campus_net_config.txt"))
    $netHelper = [NetworkInterfaceHelper]::new()
    $authClient = [AuthenticationClient]::new($config, $configMgr, $netHelper)
    $authClient.Run()
}
catch {
    Write-Host "`n脚本执行出错：$($_.Exception.Message)" -ForegroundColor Red
    Write-Host "详细错误：$($_.ScriptStackTrace)" -ForegroundColor DarkGray
    Read-Host "按 Enter 键退出"
    exit 1
}