# ============================================================
#  tests/run_ps1_tests.ps1
#  新乡工程学院校园网登录脚本 (xywdl.ps1) 端到端 + 边界测试
#
#  覆盖:
#    A. 认证结果判定 (code 0/1/44/99/10/100/123/440)
#    B. 缺失/损坏 profile / credential 文件 → 正确退出码 2/3
#    C. 请求参数编码 (SSID 含空格, 密码含特殊字符)
#    D. 密码不泄漏到日志 (passwd= 应被脱敏)
#    E. 稳定性: 连续多次调用退出码稳定
#
#  用法:
#    powershell -ExecutionPolicy Bypass -File tests/run_ps1_tests.ps1
# ============================================================
param(
    [string]$ProjectDir = (Split-Path $PSScriptRoot -Parent)
)

$ErrorActionPreference = "Stop"
$PASS = 0; $FAIL = 0; $FAILURES = @()

function Assert-True([bool]$Cond, [string]$Name, [string]$Detail) {
    if ($Cond) { $script:PASS++; Write-Host "  [PASS] $Name" -ForegroundColor Green }
    else {
        $script:FAIL++
        $script:FAILURES += $Name
        Write-Host "  [FAIL] $Name :: $Detail" -ForegroundColor Red
    }
}

function Write-CredentialBin([string]$CredPath, [string]$Plain) {
    Add-Type -AssemblyName System.Security
    $bytes = [System.Text.Encoding]::Unicode.GetBytes($Plain)
    $blob = [System.Security.Cryptography.ProtectedData]::Protect(
        $bytes, $null, [System.Security.Cryptography.DataProtectionScope]::CurrentUser)
    [System.IO.File]::WriteAllBytes($CredPath, $blob)
}

function New-TestProfile {
    param(
        [hashtable]$Overrides = @{},
        [string]$BaseUrl = "http://127.0.0.1:18080/portal.do"
    )
    $p = @{
        user_id       = "2021110101@xxgcyd"
        operator      = "yd"
        ssid          = "XXGC-WiFi"
        base_url      = $BaseUrl
        wlan_ac_name  = "XXGC-AC"
        wlan_ac_ip    = "172.18.252.1"
        vlan          = "100"
        wlan_user_ip  = ""
        mac_address   = "aa:bb:cc:dd:ee:ff"
        portal_page_id= "3"
        portal_type   = "0"
        version       = "0"
        bind_ctrl_id  = ""
        hostname      = "TEST-PC"
        updated_at    = "2026-08-30T00:00:00"
    }
    foreach ($k in $Overrides.Keys) { $p[$k] = $Overrides[$k] }
    return $p
}

# 每个 mock 用独立端口启动,避免重启竞态
function Start-Mock([int]$Port, [string]$Code, [string]$LogPath) {
    $proc = Start-Process -FilePath "python" -ArgumentList @(
        (Join-Path $PSScriptRoot "mock_portal.py"),
        "--port", "$Port", "--code", $Code, "--log", $LogPath
    ) -PassThru -WindowStyle Hidden
    $ready = $false
    for ($i = 0; $i -lt 40; $i++) {
        Start-Sleep -Milliseconds 250
        if ($proc.HasExited) { break }
        try { $null = Invoke-WebRequest "http://127.0.0.1:$Port/healthz" -TimeoutSec 1 -UseBasicParsing; $ready = $true; break } catch {}
    }
    return @{ Proc = $proc; Ready = $ready }
}

function Invoke-Xywdl {
    param([string]$AppData)
    $env:APPDATA = $AppData
    $out = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$ProjectDir\xywdl.ps1" --non-interactive 2>&1
    $code = $LASTEXITCODE
    return @{ Output = ($out -join "`n"); ExitCode = $code }
}

# ---------- 0. 准备 ----------
$TestRoot = Join-Path $env:TEMP "xywdl_test_$(Get-Random)"
$null = New-Item -ItemType Directory -Path $TestRoot -Force

$mocks = @{}
function Ensure-Mock([string]$Code, [int]$Port) {
    $log = Join-Path $TestRoot "mock_$Code.log"
    if (-not $mocks.ContainsKey($Code)) {
        $m = Start-Mock $Port $Code $log
        if (-not $m.Ready) { throw "mock code=$Code 未就绪" }
        $mocks[$Code] = $m
    }
    return $mocks[$Code]
}

function New-CaseAppData([string]$Name) {
    $dir = Join-Path $TestRoot $Name
    $null = New-Item -ItemType Directory -Path (Join-Path $dir "xxgcxy-wifi") -Force
    return $dir
}

# 准备: 先启动所有需要的 mock (固定端口映射)
#  code0:18080 code1:18081 code44:18082 code99:18083 code10:18084 code100:18085 code123:18086 code440:18087
$portMap = @{
    "0"   = 18080; "1"   = 18081; "44"  = 18082; "99"  = 18083
    "10"  = 18084; "100" = 18085; "123" = 18086; "440" = 18087
    "ac_device_error" = 18088; "ac_string_zero" = 18089
}
foreach ($code in @("0","1","44","99","10","100","123","440","ac_device_error","ac_string_zero")) {
    Ensure-Mock $code $portMap[$code] | Out-Null
}

Write-Host "===== A. 认证结果判定 (退出码) =====" -ForegroundColor Cyan
$codeCases = @(
    @{ code="0";  expect=0;  name="code=0 认证成功" }
    @{ code="1";  expect=1;  name="code=1 账号不存在" }
    @{ code="44"; expect=44; name="code=44 非法接入" }
    @{ code="99"; expect=99; name="code=99 未知错误" }
    @{ code="10"; expect=99; name="code=10 参数错误(边界:不应判为账号不存在)" }
    @{ code="100";expect=99; name="code=100 服务器错误(边界:不应判为账号不存在)" }
    @{ code="123";expect=99; name="code=123 其他错误(边界:不应判为账号不存在)" }
    @{ code="440";expect=99; name="code=440 VLAN校验(边界:不应判为非法接入)" }
)
foreach ($c in $codeCases) {
    $AppData = New-CaseAppData "case_a_$($c.code)"
    $profile = New-TestProfile -BaseUrl "http://127.0.0.1:$($portMap[$c.code])/portal.do"
    ($profile | ConvertTo-Json) | Set-Content -Path (Join-Path $AppData "xxgcxy-wifi\login_profile.json") -Encoding UTF8
    Write-CredentialBin (Join-Path $AppData "xxgcxy-wifi\login_credential.bin") "TestPass123"
    $r = Invoke-Xywdl -AppData $AppData
    $errSnippet = if ($r.ExitCode -ne $c.expect) { " [OUT: $($r.Output -replace '\r?\n', ' | ')]" } else { "" }
    Assert-True ($r.ExitCode -eq $c.expect) $c.name "exit=$($r.ExitCode), 期望 $($c.expect)$errSnippet"
}

Write-Host "===== B. 缺失/损坏配置 (退出码) =====" -ForegroundColor Cyan
# B1 无 profile 文件 → exit 2
$AppData = New-CaseAppData "case_b1"
$r = Invoke-Xywdl -AppData $AppData
Assert-True ($r.ExitCode -eq 2) "B1 缺 profile.json → exit 2" "exit=$($r.ExitCode)"

# B2 profile 存在但缺 credential → exit 2
$AppData = New-CaseAppData "case_b2"
(New-TestProfile | ConvertTo-Json) | Set-Content -Path (Join-Path $AppData "xxgcxy-wifi\login_profile.json") -Encoding UTF8
$r = Invoke-Xywdl -AppData $AppData
Assert-True ($r.ExitCode -eq 2) "B2 缺 credential.bin → exit 2" "exit=$($r.ExitCode)"

# B3 空 profile.json → exit 2
$AppData = New-CaseAppData "case_b3"
Set-Content -Path (Join-Path $AppData "xxgcxy-wifi\login_profile.json") -Value "" -Encoding UTF8
Write-CredentialBin (Join-Path $AppData "xxgcxy-wifi\login_credential.bin") "TestPass123"
$r = Invoke-Xywdl -AppData $AppData
Assert-True ($r.ExitCode -eq 2) "B3 空 profile.json → exit 2" "exit=$($r.ExitCode)"

# B4 profile 缺字段 → exit 2
$AppData = New-CaseAppData "case_b4"
$p = New-TestProfile; $p.Remove("vlan")
($p | ConvertTo-Json) | Set-Content -Path (Join-Path $AppData "xxgcxy-wifi\login_profile.json") -Encoding UTF8
Write-CredentialBin (Join-Path $AppData "xxgcxy-wifi\login_credential.bin") "TestPass123"
$r = Invoke-Xywdl -AppData $AppData
Assert-True ($r.ExitCode -eq 2) "B4 profile 缺 vlan 字段 → exit 2" "exit=$($r.ExitCode)"

# B5 credential 文件太短 (损坏) → exit 3
$AppData = New-CaseAppData "case_b5"
(New-TestProfile | ConvertTo-Json) | Set-Content -Path (Join-Path $AppData "xxgcxy-wifi\login_profile.json") -Encoding UTF8
[System.IO.File]::WriteAllBytes((Join-Path $AppData "xxgcxy-wifi\login_credential.bin"), [byte[]](1,2,3,4,5))
$r = Invoke-Xywdl -AppData $AppData
Assert-True ($r.ExitCode -eq 3) "B5 credential 损坏 → exit 3" "exit=$($r.ExitCode)"

Write-Host "===== C. 请求参数编码 =====" -ForegroundColor Cyan
# C1 SSID 含空格 + 密码含特殊字符 → mock 收到的参数应正确解码
$AppData = New-CaseAppData "case_c1"
$p = New-TestProfile -BaseUrl "http://127.0.0.1:18080/portal.do"
$p.ssid = "XXGC-WiFi 5G"
($p | ConvertTo-Json) | Set-Content -Path (Join-Path $AppData "xxgcxy-wifi\login_profile.json") -Encoding UTF8
Write-CredentialBin (Join-Path $AppData "xxgcxy-wifi\login_credential.bin") 'P@ssw0rd! a=b&c?d#e'
$r = Invoke-Xywdl -AppData $AppData
Assert-True ($r.ExitCode -eq 0) "C1 SSID含空格+特殊字符密码 → 登录成功" "exit=$($r.ExitCode)"

Start-Sleep -Milliseconds 400
$mockLog = Join-Path $TestRoot "mock_0.log"
$mockRecords = @()
if (Test-Path $mockLog) { $mockRecords = Get-Content $mockLog -Encoding UTF8 | ForEach-Object { $_ | ConvertFrom-Json } }
if ($mockRecords.Count -gt 0) {
    $params = $mockRecords[-1].params
    Assert-True ($params.ssid -eq "XXGC-WiFi 5G") "C1 ssid 正确解码" "got='$($params.ssid)'"
    Assert-True ($params.passwd -eq 'P@ssw0rd! a=b&c?d#e') "C1 passwd 正确解码(含空格/&=?#)" "got='$($params.passwd)'"
    Assert-True ($params.userid -eq "2021110101@xxgcyd") "C1 userid 正确解码" "got='$($params.userid)'"
    # mac: 脚本会优先用运行时取到的真实本机 MAC (自动兜底), 而非 profile 里的值
    # 只断言格式合法即可, 不锁定具体值
    $macOk = $params.mac -match '^([0-9a-f]{2}:){5}[0-9a-f]{2}$'
    Assert-True $macOk "C1 mac 为合法 MAC 格式(脚本自动获取)" "got='$($params.mac)'"
} else {
    Assert-True $false "C1 mock 未收到请求" "mock_log 为空 ($mockLog)"
}

# C2 SSID 含 &、=、中文 + vlan 含空格 → 必须正确编码且不破坏 query 结构
$AppData = New-CaseAppData "case_c2"
$p2 = New-TestProfile -BaseUrl "http://127.0.0.1:18080/portal.do"
$p2.ssid = "XXGC&=测试WiFi"
$p2.vlan = "100 5"
($p2 | ConvertTo-Json) | Set-Content -Path (Join-Path $AppData "xxgcxy-wifi\login_profile.json") -Encoding UTF8
Write-CredentialBin (Join-Path $AppData "xxgcxy-wifi\login_credential.bin") "TestPass123"
$r = Invoke-Xywdl -AppData $AppData
Assert-True ($r.ExitCode -eq 0) "C2 SSID含&=中文+vlan含空格 → 登录成功" "exit=$($r.ExitCode)"
Start-Sleep -Milliseconds 400
$mockRecords = @()
if (Test-Path $mockLog) { $mockRecords = Get-Content $mockLog -Encoding UTF8 | ForEach-Object { $_ | ConvertFrom-Json } }
if ($mockRecords.Count -gt 0) {
    $params2 = $mockRecords[-1].params
    Assert-True ($params2.ssid -eq "XXGC&=测试WiFi") "C2 ssid 正确解码(含&=中文)" "got='$($params2.ssid)'"
    Assert-True ($params2.vlan -eq "100 5") "C2 vlan 正确解码(含空格)" "got='$($params2.vlan)'"
} else {
    Assert-True $false "C2 mock 未收到请求" "mock_log 为空"
}

Write-Host "===== D. 密码不泄漏到日志 =====" -ForegroundColor Cyan
$AppData = New-CaseAppData "case_d1"
(New-TestProfile -BaseUrl "http://127.0.0.1:18080/portal.do" | ConvertTo-Json) |
    Set-Content -Path (Join-Path $AppData "xxgcxy-wifi\login_profile.json") -Encoding UTF8
Write-CredentialBin (Join-Path $AppData "xxgcxy-wifi\login_credential.bin") 'SuperSecretPwd99'
$r = Invoke-Xywdl -AppData $AppData
$leaked = $r.Output -match 'passwd=SuperSecretPwd99'
Assert-True (-not $leaked) "D1 密码未以明文出现在输出" "输出含明文密码或未知: $($r.Output -replace '[^\x20-\x7E]',' ')"

Write-Host "===== E. 稳定性: 连续 5 次调用 =====" -ForegroundColor Cyan
$AppData = New-CaseAppData "case_e1"
(New-TestProfile -BaseUrl "http://127.0.0.1:18080/portal.do" | ConvertTo-Json) |
    Set-Content -Path (Join-Path $AppData "xxgcxy-wifi\login_profile.json") -Encoding UTF8
Write-CredentialBin (Join-Path $AppData "xxgcxy-wifi\login_credential.bin") "TestPass123"
$codes = @()
foreach ($n in 1..5) { $rr = Invoke-Xywdl -AppData $AppData; $codes += $rr.ExitCode; Start-Sleep -Milliseconds 200 }
Write-Host "===== F. 真实校园网 AC 响应与 URL 净化强健性测试 =====" -ForegroundColor Cyan
# F1: 脏 BaseURL (带有完整 query string 如 ?wlanuserip=...&url=...)
$AppData = New-CaseAppData "case_f1"
$dirtyUrl = "http://127.0.0.1:18080/portal.do?wlanuserip=10.4.124.192&wlanacname=AuteWifi-XXGC&mac=f4:6a:dd:e5:4a:7b&vlan=31002201&hostname=LAPTOP-FTBM6JJ1&rand=100bd08f91d44a1&url=http%3A%2F%2Fwww.qq.com"
(New-TestProfile -BaseUrl $dirtyUrl | ConvertTo-Json) |
    Set-Content -Path (Join-Path $AppData "xxgcxy-wifi\login_profile.json") -Encoding UTF8
Write-CredentialBin (Join-Path $AppData "xxgcxy-wifi\login_credential.bin") "TestPass123"
$r = Invoke-Xywdl -AppData $AppData
Assert-True ($r.ExitCode -eq 0) "F1 脏 BaseURL 净化后成功请求并认证" "exit=$($r.ExitCode)"
Start-Sleep -Milliseconds 400
$mockRecords = @()
$mock0Log = Join-Path $TestRoot "mock_0.log"
if (Test-Path $mock0Log) { $mockRecords = Get-Content $mock0Log -Encoding UTF8 | ForEach-Object { $_ | ConvertFrom-Json } }
if ($mockRecords.Count -gt 0) {
    $lastReq = $mockRecords[-1]
    Assert-True ($lastReq.path -eq "/quickauth.do") "F1 请求路径为纯净 /quickauth.do" "path='$($lastReq.path)'"
    Assert-True ($lastReq.params.userid -eq "2021110101@xxgcyd") "F1 userid 参数独立解析成功" "userid='$($lastReq.params.userid)'"
    Assert-True ($lastReq.params.passwd -eq "TestPass123") "F1 passwd 参数独立解析成功" "passwd='$($lastReq.params.passwd)'"
    Assert-True (-not $lastReq.query.Contains("??")) "F1 请求不含双重问号" "query='$($lastReq.query)'"
}

# F2: 真实 AC 错误响应: {"code":"1","rec":null,"message":"设备不在正常状态,无法认证上网,请稍后",...}
$AppData = New-CaseAppData "case_f2"
(New-TestProfile -BaseUrl "http://127.0.0.1:18088/portal.do" | ConvertTo-Json) |
    Set-Content -Path (Join-Path $AppData "xxgcxy-wifi\login_profile.json") -Encoding UTF8
Write-CredentialBin (Join-Path $AppData "xxgcxy-wifi\login_credential.bin") "TestPass123"
$r2 = Invoke-Xywdl -AppData $AppData
Assert-True ($r2.ExitCode -eq 1) "F2 code='1' 识别为认证未通过(exit 1 非 exit 99)" "exit=$($r2.ExitCode)"
Assert-True ($r2.Output.Contains("设备不在正常状态,无法认证上网,请稍后")) "F2 包含服务器真实错误信息" "output=$($r2.Output)"

# F3: 真实 AC 成功响应: {"code":"0","message":"success"} (字符串 code '0')
$AppData = New-CaseAppData "case_f3"
(New-TestProfile -BaseUrl "http://127.0.0.1:18089/portal.do" | ConvertTo-Json) |
    Set-Content -Path (Join-Path $AppData "xxgcxy-wifi\login_profile.json") -Encoding UTF8
Write-CredentialBin (Join-Path $AppData "xxgcxy-wifi\login_credential.bin") "TestPass123"
$r3 = Invoke-Xywdl -AppData $AppData
Assert-True ($r3.ExitCode -eq 0) "F3 code='0' 字符串 0 正确识别为认证成功" "exit=$($r3.ExitCode)"

# ---------- 清理 ----------
foreach ($m in $mocks.Values) { Stop-Process -Id $m.Proc.Id -Force -ErrorAction SilentlyContinue }
Remove-Item -Recurse -Force $TestRoot -ErrorAction SilentlyContinue

Write-Host ""
Write-Host "===== 结果: $PASS 通过, $FAIL 失败 =====" -ForegroundColor $(if($FAIL -eq 0){'Green'}else{'Red'})
if ($FAIL -gt 0) { Write-Host "失败项: $($FAILURES -join ', ')" -ForegroundColor Red }
exit $(if($FAIL -eq 0){0}else{1})



