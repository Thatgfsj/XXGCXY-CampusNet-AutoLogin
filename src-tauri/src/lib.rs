use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::AppHandle;

// ============= 单例检查 =============

#[cfg(windows)]
fn check_single_instance() -> bool {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Foundation::ERROR_ALREADY_EXISTS;
    use windows::Win32::System::Threading::CreateMutexW;

    unsafe {
        let mutex_name: Vec<u16> = OsStr::new("Global\\CampusWifiHelper_SingleInstance")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let result = CreateMutexW(None, false, windows::core::PCWSTR(mutex_name.as_ptr()));
        if result.is_err() {
            return false;
        }
        let err = windows::Win32::Foundation::GetLastError();
        err != ERROR_ALREADY_EXISTS
    }
}

#[cfg(not(windows))]
fn check_single_instance() -> bool {
    let lock_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("xxgcxy-wifi");
    let _ = fs::create_dir_all(&lock_dir);
    let lock_path = lock_dir.join("single_instance.lock");

    use std::io::Write;
    match fs::OpenOptions::new().create_new(true).write(true).open(&lock_path) {
        Ok(mut file) => {
            let pid = std::process::id().to_string();
            let _ = file.write_all(pid.as_bytes());
            true
        }
        Err(_) => false,
    }
}

// ============= 兼容 UTF-8 BOM 清洗 =============

#[inline]
fn strip_bom(s: &str) -> &str {
    s.strip_prefix("\u{feff}").unwrap_or(s)
}

// ============= 隐藏命令行窗口 =============

#[cfg(windows)]
fn hidden_command(program: &str) -> std::process::Command {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let mut cmd = std::process::Command::new(program);
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

#[allow(dead_code)]
#[cfg(not(windows))]
fn hidden_command(program: &str) -> std::process::Command {
    std::process::Command::new(program)
}

// ============= 配置结构体 =============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub primary_ssid: String,
    pub backup_ssid: String,
    pub check_interval: u64,
    #[serde(default)]
    pub hotspot_keepalive: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            primary_ssid: String::new(),
            backup_ssid: String::new(),
            check_interval: 15,
            hotspot_keepalive: false,
        }
    }
}

// ============= WiFi 网络信息 =============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WifiNetwork {
    pub ssid: String,
    pub signal: u8,
    pub secured: bool,
}

// ============= 网络状态 =============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStatus {
    pub wifi_connected: Option<String>,
    pub internet_ok: bool,
    pub needs_reconnect: bool,
    pub needs_login: bool,
}

// ============= 校园网信息(由 xywdl.ps1 / xywdl.sh 写入) =============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampusNetInfo {
    pub configured: bool,
    pub student_id: String,
    pub operator: String,
    pub ssid: String,
}

// ============= 登录模块 (v1.9.0+) =============
//
// 登录配置拆为两个文件,位于 %APPDATA%/xxgcxy-wifi/ 下:
//   - login_profile.json   : 非敏感元数据(学号/运营商/SSID/portal URL/AC/VLAN 等)
//   - login_credential.bin : DPAPI 加密的密码(PS 端 ConvertTo-SecureString 可直接读取)
//
// 设计要点:
// 1. JSON 模板驱动:UI 改的是 JSON,PS 脚本只负责读 JSON + 发请求,不再硬编码
// 2. 旧 `xxgc_campus_net_config.txt` 不再读写,新用户和老用户都能干净开始
// 3. 密码用 Windows DPAPI 加密(同用户同机器可由 PS 解密),Linux 用明文(暂存,后续可换 libsecret)

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginProfile {
    pub user_id: String,        // 形如 "2021110101@xxgcyd"
    pub operator: String,       // "yd" | "lt" | "dx"
    pub ssid: String,
    pub base_url: String,       // 完整 portal.do URL, 如 http://172.16.x.x:6060/portal.do
    pub wlan_ac_name: String,
    pub wlan_ac_ip: String,
    pub vlan: String,
    pub wlan_user_ip: String,   // 留空时 PS 端运行时用 Get-WifiIpAddress 拿
    pub mac_address: String,
    pub portal_page_id: String, // 默认 "3"
    pub portal_type: String,    // 默认 "0"
    pub version: String,        // 默认 "0"
    pub bind_ctrl_id: String,   // 默认 ""
    pub hostname: String,       // 留空时 PS 用 $env:COMPUTERNAME
    pub updated_at: String,     // ISO8601
}

impl Default for LoginProfile {
    fn default() -> Self {
        LoginProfile {
            user_id: String::new(),
            operator: String::new(),
            ssid: String::new(),
            base_url: String::new(),
            wlan_ac_name: String::new(),
            wlan_ac_ip: String::new(),
            vlan: String::new(),
            wlan_user_ip: String::new(),
            mac_address: String::new(),
            portal_page_id: "3".to_string(),
            portal_type: "0".to_string(),
            version: "0".to_string(),
            bind_ctrl_id: String::new(),
            hostname: String::new(),
            updated_at: String::new(),
        }
    }
}

/// portal.do 重定向 URL 解析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedPortal {
    pub base_url: String,       // 提取的 BaseURL(到 /portal.do)
    pub wlan_ac_name: String,
    pub wlan_ac_ip: String,
    pub wlan_user_ip: String,
    pub vlan: String,
    pub mac_address: String,
    pub ssid: String,
    pub hostname: String,
    pub rand: String,
}

/// 登录模块文件路径 (v1.9.0+)
/// - Windows: `%APPDATA%\xxgcxy-wifi\login_profile.json` + `login_credential.bin`
/// - Linux:   `~/.config/xxgcxy-wifi/login_profile.json` + `login_credential.bin`
fn get_login_dir() -> PathBuf {
    let base = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("xxgcxy-wifi");
    let _ = fs::create_dir_all(&base);
    base
}

fn get_login_profile_path() -> PathBuf {
    get_login_dir().join("login_profile.json")
}

fn get_login_credential_path() -> PathBuf {
    get_login_dir().join("login_credential.bin")
}

/// 旧式 (v1.8.x 及之前) 配置文件路径 —— 仅用于检测残留,不再读写
fn get_legacy_campus_config_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    #[cfg(windows)]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            paths.push(PathBuf::from(appdata).join("xxgc_campus_net_config.txt"));
        }
    }
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".config").join("xxgcxy-wifi").join("login_config.json"));
    }
    paths
}

/// 向后兼容:之前 `load_campus_net_info` 用的辅助函数,现在统一返回新路径
fn get_campus_config_path() -> PathBuf {
    get_login_profile_path()
}

fn operator_from_suffix(suffix: &str) -> &'static str {
    match suffix {
        "@xxgcyd" => "移动",
        "@xxgclt" => "联通",
        "@xxgcdx" => "电信",
        _ => "未知",
    }
}

#[tauri::command]
fn load_campus_net_info() -> Result<CampusNetInfo, String> {
    let path = get_campus_config_path();
    if !path.exists() {
        return Ok(CampusNetInfo {
            configured: false,
            student_id: String::new(),
            operator: String::new(),
            ssid: String::new(),
        });
    }
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("读取校园网配置失败: {}", e))?;
    // v1.9.0+ 新格式直接是 LoginProfile JSON (snake_case)
    // 兼容旧 v1.8.x PascalCase 格式 (UserId/Ssid)
    let json: serde_json::Value = serde_json::from_str(strip_bom(&content))
        .map_err(|e| format!("解析校园网配置失败: {}", e))?;

    let user_id = json
        .get("user_id")
        .or_else(|| json.get("UserId"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let ssid = json
        .get("ssid")
        .or_else(|| json.get("Ssid"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let operator_code = json
        .get("operator")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // 拆分 学号@运营商后缀
    let (student_id, operator) = match user_id.find('@') {
        Some(idx) => {
            let id = user_id[..idx].to_string();
            // 优先用 profile.operator 字段,其次用后缀反推
            let op = if !operator_code.is_empty() {
                operator_short_to_name(&operator_code).to_string()
            } else {
                operator_from_suffix(&user_id[idx..]).to_string()
            };
            (id, op)
        }
        None => (String::new(), String::new()),
    };

    Ok(CampusNetInfo {
        configured: !student_id.is_empty(),
        student_id,
        operator,
        ssid,
    })
}

#[tauri::command]
fn clear_campus_net_info() -> Result<(), String> {
    // v1.9.0+ 一次清掉 profile + credential
    let _ = fs::remove_file(get_login_profile_path());
    let _ = fs::remove_file(get_login_credential_path());
    // 同时清理旧版残留文件
    for legacy in get_legacy_campus_config_candidates() {
        let _ = fs::remove_file(legacy);
    }
    Ok(())
}

// ============= 登录模块命令 (v1.9.0+) =============

/// 启动时检查:是否已配置过校园网账号?
/// 用于决定首次启动是否弹出登录配置屏。
#[tauri::command]
fn is_login_configured() -> bool {
    let profile_path = get_login_profile_path();
    let cred_path = get_login_credential_path();
    if !profile_path.exists() || !cred_path.exists() {
        return false;
    }
    match fs::read_to_string(&profile_path) {
        Ok(content) => {
            match serde_json::from_str::<LoginProfile>(strip_bom(&content)) {
                Ok(p) => !p.user_id.is_empty() && !p.base_url.is_empty(),
                Err(_) => false,
            }
        }
        Err(_) => false,
    }
}

/// 读取当前登录配置 (不含密码,密码由 PS 端从 .bin 文件读)
#[tauri::command]
fn get_login_profile() -> Result<LoginProfile, String> {
    let path = get_login_profile_path();
    if !path.exists() {
        return Ok(LoginProfile::default());
    }
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("读取登录配置失败: {}", e))?;
    serde_json::from_str(strip_bom(&content))
        .map_err(|e| format!("解析登录配置失败: {}", e))
}

/// 保存登录配置 + 加密密码
#[tauri::command]
fn save_login_profile(mut profile: LoginProfile, password: String) -> Result<(), String> {
    if profile.user_id.is_empty() {
        return Err("学号/账号不能为空".to_string());
    }
    if profile.base_url.is_empty() {
        return Err("Portal URL 不能为空".to_string());
    }

    // 净化 base_url: 去除首尾空格，去除 ? 和 # 后的所有 query string
    let mut clean_base_url = profile.base_url.trim().to_string();
    if let Some(idx) = clean_base_url.find('?') {
        clean_base_url.truncate(idx);
    }
    if let Some(idx) = clean_base_url.find('#') {
        clean_base_url.truncate(idx);
    }
    profile.base_url = clean_base_url;

    // 硬校验: Portal URL 必须含 http(s):// 和 .do 路径
    // 避免用户填了 `172.18.x.x:6060/portal.do` (无 scheme) 或纯 IP 进 profile
    // 这些 URL 在 PS 脚本 Invoke-WebRequest 里依赖 WinHTTP 隐式推断, 不可靠
    if !profile.base_url.contains("://") {
        return Err("Portal URL 必须以 http:// 或 https:// 开头".to_string());
    }
    if !profile.base_url.to_lowercase().contains(".do") {
        return Err("Portal URL 必须包含 .do 路径 (如 /portal.do 或 /quickauth.do)".to_string());
    }
    if password.is_empty() {
        return Err("密码不能为空".to_string());
    }

    // 写 JSON
    let json = serde_json::to_string_pretty(&profile)
        .map_err(|e| format!("序列化登录配置失败: {}", e))?;
    fs::write(get_login_profile_path(), json)
        .map_err(|e| format!("写入登录配置失败: {}", e))?;

    // 加密密码
    let encrypted = encrypt_password(&password)
        .map_err(|e| format!("加密密码失败: {}", e))?;
    fs::write(get_login_credential_path(), encrypted)
        .map_err(|e| format!("写入加密密码失败: {}", e))?;

    Ok(())
}

fn save_login_profile_json(profile: &LoginProfile) -> Result<(), String> {
    let json = serde_json::to_string_pretty(profile)
        .map_err(|e| format!("序列化登录配置失败: {}", e))?;
    fs::write(get_login_profile_path(), json)
        .map_err(|e| format!("写入登录配置失败: {}", e))
}

/// 解析 portal.do 重定向 URL (替代旧 PS 端的 TryAutoDetectParams)
/// 用户从浏览器复制粘贴的 URL 进来,我们用跟 PS 端 RedirectUrlParser 一样的正则解析。
#[tauri::command]
fn parse_portal_url(url: String) -> Result<ParsedPortal, String> {
    if url.trim().is_empty() {
        return Err("URL 不能为空".to_string());
    }

    // 简化版 URL 解码:用 percent-decoding 的核心规则
    // 修复: 用 Vec<u8> 收集字节后 UTF-8 整体解码, 而不是按 char 拼 (避免 UTF-8 多字节字符被当成单字节)
    // (注: + 不被转空格,因为 portal.do URL 是 redirect URL,不是 form-data)
    fn url_decode(s: &str) -> String {
        let bytes = s.as_bytes();
        let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%' && i + 2 < bytes.len() {
                if let Ok(b) = u8::from_str_radix(
                    std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("00"),
                    16,
                ) {
                    out.push(b);
                    i += 3;
                    continue;
                }
            }
            out.push(bytes[i]);
            i += 1;
        }
        // 整体 UTF-8 解码, 失败回退 Latin-1 (每个字节 1 char)
        String::from_utf8(out.clone()).unwrap_or_else(|_| {
            out.iter().map(|&b| b as char).collect()
        })
    }

    // form-encoded 形式: + 也转空格 (application/x-www-form-urlencoded)
    // 同样修复 UTF-8 多字节字符处理
    fn url_decode_form(s: &str) -> String {
        let bytes = s.as_bytes();
        let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'+' {
                out.push(b' ');
                i += 1;
                continue;
            }
            if bytes[i] == b'%' && i + 2 < bytes.len() {
                if let Ok(b) = u8::from_str_radix(
                    std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("00"),
                    16,
                ) {
                    out.push(b);
                    i += 3;
                    continue;
                }
            }
            out.push(bytes[i]);
            i += 1;
        }
        String::from_utf8(out.clone()).unwrap_or_else(|_| {
            out.iter().map(|&b| b as char).collect()
        })
    }

    let decoded = url_decode(&url);

    // 提取 BaseURL: 形如 "http://host:port/portal.do"
    let base_url = {
        // 简单正则: ^http://[^/]+/\w+\.do
        // 注意: 这里要保留 scheme (http:// 或 https://), 不然前端会拿到无 scheme 的 host:port/portal.do
        let lower = decoded.to_lowercase();
        if let Some(idx) = lower.find("://") {
            // idx 指向 ':' 位置, :// 是 3 字符, scheme 总长度 = idx + 3
            let scheme_end = idx + 3;
            let after_scheme = &decoded[scheme_end..];
            // 找到第一个 /,然后到 .do 结尾
            if let Some(slash_idx) = after_scheme.find('/') {
                let host_and_path = &after_scheme[slash_idx..];
                // 找 ".do" 结尾
                if let Some(do_idx) = host_and_path.to_lowercase().find(".do") {
                    let end = do_idx + 3;
                    // 用 decoded[..scheme_end + slash_idx + end] 而不是 after_scheme[..] 来保留 scheme
                    decoded[..scheme_end + slash_idx + end].to_string()
                } else {
                    return Err("URL 中找不到 portal.do 路径".to_string());
                }
            } else {
                return Err("URL 格式不正确(缺少路径)".to_string());
            }
        } else {
            return Err("URL 必须以 http:// 开头".to_string());
        }
    };

    // 解析 query string
    let mut parsed = ParsedPortal {
        base_url: base_url.clone(),
        wlan_ac_name: String::new(),
        wlan_ac_ip: String::new(),
        wlan_user_ip: String::new(),
        vlan: String::new(),
        mac_address: String::new(),
        ssid: String::new(),
        hostname: String::new(),
        rand: String::new(),
    };

    if let Some(q_idx) = decoded.find('?') {
        let qs = &decoded[q_idx + 1..];
        for kv in qs.split('&') {
            let mut parts = kv.splitn(2, '=');
            let k = parts.next().unwrap_or("").to_lowercase();
            let v_raw = parts.next().unwrap_or("");
            // portal.do redirect URL 的 query 实际是 form-encoded,
            // + 应该转空格, 用 url_decode_form
            let v = url_decode_form(v_raw);
            match k.as_str() {
                "wlanuserip" => parsed.wlan_user_ip = v,
                "wlanacname" => parsed.wlan_ac_name = v,
                "wlanacip" => parsed.wlan_ac_ip = v,
                "mac" => parsed.mac_address = v.to_lowercase(),
                "vlan" => parsed.vlan = v,
                "hostname" => parsed.hostname = v,
                "rand" => parsed.rand = v,
                "ssid" => parsed.ssid = v,
                _ => {}
            }
        }
    }

    Ok(parsed)
}

/// 用已保存的 profile 直接执行登录 (接入统一互斥认证流水线)
#[tauri::command]
async fn run_login_with_profile(app: AppHandle) -> Result<String, String> {
    if !is_login_configured() {
        return Err("尚未配置校园网账号,请先在主页或网络配置页填写".to_string());
    }
    let state = app.state::<AppState>();
    let _guard = state.is_logging_in.try_lock()
        .map_err(|_| "当前已有认证流程正在执行，请勿重复操作".to_string())?;
    execute_login_flow(&app).await
}

// ============= DPAPI 密码加密 =============
//
// 链路:plaintext -> UTF-16 LE 字节 -> CryptProtectData (DPAPI, 无 entropy) -> 裸字节
// PS 端读: [IO.File]::ReadAllBytes -> [Security.Cryptography.ProtectedData]::Unprotect($null, CurrentUser) -> UTF-16 LE 解码
//
// 注意: Rust 和 PS 必须都走"无 entropy"模式,否则解密失败
// (旧版 v1.8.x 加过 8 字节 magic 头, v1.9.0+ 简化掉了, PS 端直接 ProtectedData::Unprotect)

#[cfg(windows)]
fn encrypt_password(plain: &str) -> Result<Vec<u8>, String> {
    use windows::Win32::Security::Cryptography::{CryptProtectData, CRYPT_INTEGER_BLOB};
    use windows::Win32::Foundation::{HLOCAL, LocalFree};

    // 链路:plaintext -> UTF-16 LE 字节 -> CryptProtectData (DPAPI, 无 entropy)
    // PS 端读 bytes,剥掉 8 字节头部 ("DPAPI" magic + 4 字节 LE 长度) -> ProtectedData.Unprotect
    //  -> UTF-16 LE 字节 -> 还原成字符串
    //
    // 注意:Rust 和 PS 必须都走"无 entropy"模式,否则解密失败

    let utf16: Vec<u16> = plain.encode_utf16().collect();
    let utf16_bytes: Vec<u8> = utf16.iter().flat_map(|c| c.to_le_bytes()).collect();

    unsafe {
        let input = CRYPT_INTEGER_BLOB {
            cbData: utf16_bytes.len() as u32,
            pbData: utf16_bytes.as_ptr() as *mut u8,
        };
        let mut output = std::mem::zeroed();

        let result = CryptProtectData(
            &input,
            None, // szDataDescr: 不需要描述
            None, // pOptionalEntropy: 必须 None,与 PS 端 Unprotect 的 $null 配对
            None, // pvReserved
            None, // pPromptStruct
            0,    // dwFlags: 0 表示默认 (CurrentUser 范围)
            &mut output,
        );

        if result.is_err() {
            return Err(format!("CryptProtectData 失败: {:?}", result));
        }

        // 拷贝输出数据
        let protected_bytes = std::slice::from_raw_parts(
            output.pbData,
            output.cbData as usize,
        )
        .to_vec();

        // 释放 LocalAlloc 分配的内存
        if !output.pbData.is_null() {
            let _ = LocalFree(HLOCAL(output.pbData as *mut _));
        }

        // 直接写裸 DPAPI blob (v1.9.0+ 简化: 不再加 magic 头, 与 PS 端 ProtectedData::Protect 字节布局一致)
        Ok(protected_bytes)
    }
}

#[cfg(not(windows))]
fn encrypt_password(plain: &str) -> Result<Vec<u8>, String> {
    // Linux 临时实现:明文存(后续可换 libsecret / keyring)
    // 注意:写文件时用 OpenOptionsExt 设权限 0600 在 save_login_profile 的 fs::write 处不可控,
    // 这里仅返回字节,具体落盘策略由调用方负责。
    Ok(plain.as_bytes().to_vec())
}

#[cfg(windows)]
fn decrypt_password(protected_bytes: &[u8]) -> Result<String, String> {
    use windows::Win32::Security::Cryptography::{CryptUnprotectData, CRYPT_INTEGER_BLOB};
    use windows::Win32::Foundation::{HLOCAL, LocalFree};

    if protected_bytes.is_empty() {
        return Err("凭据文件为空".to_string());
    }

    unsafe {
        let input = CRYPT_INTEGER_BLOB {
            cbData: protected_bytes.len() as u32,
            pbData: protected_bytes.as_ptr() as *mut u8,
        };
        let mut output = std::mem::zeroed();

        let result = CryptUnprotectData(
            &input,
            None,
            None,
            None,
            None,
            0,
            &mut output,
        );

        if result.is_err() {
            return Err(format!("CryptUnprotectData 解密失败: {:?}", result));
        }

        let plain_bytes = std::slice::from_raw_parts(
            output.pbData,
            output.cbData as usize,
        );

        let u16_slice: Vec<u16> = plain_bytes
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();

        if !output.pbData.is_null() {
            let _ = LocalFree(HLOCAL(output.pbData as *mut _));
        }

        let s = String::from_utf16_lossy(&u16_slice);
        Ok(s.trim_end_matches('\0').to_string())
    }
}

#[cfg(not(windows))]
fn decrypt_password(protected_bytes: &[u8]) -> Result<String, String> {
    String::from_utf8(protected_bytes.to_vec()).map_err(|e| format!("UTF-8 解码失败: {}", e))
}

/// operator 短码 -> 中文名 (用于 load_campus_net_info 渲染)
fn operator_short_to_name(code: &str) -> &'static str {
    match code {
        "yd" => "移动",
        "lt" => "联通",
        "dx" => "电信",
        _ => "未知",
    }
}

// ============= 全局状态与服务状态机 =============

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServiceState {
    Idle,
    Checking,
    Connected,
    NeedsLogin,
    LoggingIn,
    Disconnected,
    Backoff { next_retry_secs: u64, reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatus {
    pub state: ServiceState,
    pub wifi_connected: Option<String>,
    pub internet_ok: bool,
    pub last_check_time: u64,
    pub consecutive_business_errors: u32,
    pub backoff_remaining_secs: u64,
}

impl Default for ServiceStatus {
    fn default() -> Self {
        Self {
            state: ServiceState::Idle,
            wifi_connected: None,
            internet_ok: false,
            last_check_time: 0,
            consecutive_business_errors: 0,
            backoff_remaining_secs: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerAction {
    Check,
    Login,
    ResetBackoff,
}

pub struct AppState {
    pub config: Mutex<Config>,
    pub first_run: Mutex<bool>,
    pub check_enabled: Mutex<bool>,
    pub service_status: Mutex<ServiceStatus>,
    pub trigger_tx: tokio::sync::mpsc::Sender<TriggerAction>,
    pub is_logging_in: Arc<tokio::sync::Mutex<()>>,
    pub last_manual_login: Mutex<std::time::Instant>,
}

// ============= Tauri 命令 =============

#[tauri::command]
fn get_check_enabled(state: tauri::State<'_, AppState>) -> bool {
    *state.check_enabled.lock().unwrap_or_else(|e| e.into_inner())
}

#[tauri::command]
fn toggle_check_enabled(state: tauri::State<'_, AppState>) -> bool {
    let mut enabled = state.check_enabled.lock().unwrap_or_else(|e| e.into_inner());
    *enabled = !*enabled;
    *enabled
}

#[tauri::command]
fn get_service_status(state: tauri::State<'_, AppState>) -> ServiceStatus {
    state.service_status.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

#[tauri::command]
fn trigger_manual_check(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.trigger_tx.try_send(TriggerAction::Check).map_err(|e| format!("触发检测失败: {}", e))
}

#[tauri::command]
fn trigger_manual_login(state: tauri::State<'_, AppState>) -> Result<(), String> {
    // 频控节流：限制 3 秒内最多触发一次手动认证，防止连点排队轰炸网关
    let mut last = state.last_manual_login.lock().unwrap_or_else(|e| e.into_inner());
    let now = std::time::Instant::now();
    if now.duration_since(*last) < std::time::Duration::from_secs(3) {
        return Err("操作过于频繁，请稍候再试 (最少间隔 3 秒)".to_string());
    }
    *last = now;
    state.trigger_tx.try_send(TriggerAction::Login).map_err(|e| format!("触发登录失败: {}", e))
}

// ============= 开机自启动 =============

#[tauri::command]
fn get_autostart_enabled() -> bool {
    #[cfg(windows)]
    {
        use winreg::RegKey;
        use winreg::enums::*;
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(key) =
            hkcu.open_subkey_with_flags("Software\\Microsoft\\Windows\\CurrentVersion\\Run", KEY_READ)
        {
            return key.get_value::<String, _>("新乡工程校园网保活").is_ok()
                || key.get_value::<String, _>("CampusWifiHelper").is_ok()
                || key.get_value::<String, _>("XXGCXY_WiFi").is_ok();
        }
        false
    }
    #[cfg(not(windows))]
    {
        let desktop_path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("autostart")
            .join("xxgcxy-wifi.desktop");
        desktop_path.exists()
    }
}

#[tauri::command]
fn set_autostart_enabled(enabled: bool) -> Result<(), String> {
    #[cfg(windows)]
    {
        use winreg::RegKey;
        use winreg::enums::*;
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let key = hkcu
            .create_subkey_with_flags("Software\\Microsoft\\Windows\\CurrentVersion\\Run", KEY_WRITE)
            .map_err(|e| format!("打开注册表失败: {}", e))?
            .0;
        let _ = key.delete_value("CampusWifiHelper");
        let _ = key.delete_value("XXGCXY_WiFi");
        let _ = key.delete_value("新乡工程校园网保活");
        if enabled {
            let exe_path = std::env::current_exe()
                .map_err(|e| format!("获取程序路径失败: {}", e))?
                .to_string_lossy()
                .to_string();
            // 路径含空格时必须加引号, 并且追加 --autostart 参数以便开机启动时静默保持托盘运行
            let reg_value = format!("\"{}\" --autostart", exe_path);
            key.set_value("新乡工程校园网保活", &reg_value)
                .map_err(|e| format!("写入注册表失败: {}", e))?;
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let desktop_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("autostart");
        let _ = fs::create_dir_all(&desktop_dir);
        let desktop_path = desktop_dir.join("xxgcxy-wifi.desktop");

        if enabled {
            let exe_path = std::env::current_exe()
                .map_err(|e| format!("获取程序路径失败: {}", e))?
                .to_string_lossy()
                .to_string();
            let desktop_content = format!(
                "[Desktop Entry]\nType=Application\nName=xxgcxy-wifi\nExec={} --autostart\nHidden=false\nX-GNOME-Autostart-enabled=true\n",
                exe_path
            );
            fs::write(&desktop_path, desktop_content)
                .map_err(|e| format!("写入启动文件失败: {}", e))?;
            // 文件包含 Exec 路径 (虽然不包含密码), 设 0600 防止其他用户读取安装路径
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(&desktop_path, fs::Permissions::from_mode(0o600));
            }
        } else {
            let _ = fs::remove_file(&desktop_path);
        }
        Ok(())
    }
}

// ============= 保持移动热点常开 (Hotspot Keep-Alive) =============

#[tauri::command]
fn get_hotspot_keepalive(state: tauri::State<'_, AppState>) -> bool {
    let config = state.config.lock().unwrap_or_else(|e| e.into_inner());
    config.hotspot_keepalive
}

#[tauri::command]
async fn set_hotspot_keepalive(enabled: bool, state: tauri::State<'_, AppState>) -> Result<bool, String> {
    {
        let mut config = state.config.lock().unwrap_or_else(|e| e.into_inner());
        config.hotspot_keepalive = enabled;
        let config_path = get_config_path();
        if let Ok(content) = serde_json::to_string_pretty(&*config) {
            let _ = fs::write(&config_path, content);
        }
    }
    if enabled {
        let _ = check_and_keep_hotspot_alive().await;
    }
    Ok(enabled)
}

#[tauri::command]
async fn check_and_keep_hotspot_alive() -> Result<String, String> {
    #[cfg(windows)]
    {
        static HOTSPOT_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
        let _guard = match HOTSPOT_LOCK.try_lock() {
            Ok(g) => g,
            Err(_) => return Ok("BUSY:CheckInProgress".to_string()),
        };

        let script = r#"
Add-Type -AssemblyName System.Runtime.WindowsRuntime
$asTaskGeneric = [System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' }
function AwaitAction($WinRtTask, $ResultType) {
    $asTask = $asTaskGeneric.MakeGenericMethod($ResultType)
    $netTask = $asTask.Invoke($null, @($WinRtTask))
    $netTask.Wait(12000) | Out-Null
    return $netTask.Result
}
try {
    [Windows.Networking.Connectivity.NetworkInformation, Windows.Networking.Connectivity, ContentType=WindowsRuntime] | Out-Null
    [Windows.Networking.NetworkOperators.NetworkOperatorTetheringManager, Windows.Networking.NetworkOperators, ContentType=WindowsRuntime] | Out-Null
    $profile = [Windows.Networking.Connectivity.NetworkInformation]::GetInternetConnectionProfile()
    $canTether = $false
    if ($null -ne $profile) {
        try {
            $cap = [Windows.Networking.NetworkOperators.NetworkOperatorTetheringManager]::GetTetheringCapabilityFromConnectionProfile($profile)
            if ($cap.ToString() -eq 'Enabled') { $canTether = $true }
        } catch {}
    }
    if (-not $canTether) {
        $profiles = [Windows.Networking.Connectivity.NetworkInformation]::GetConnectionProfiles()
        foreach ($p in $profiles) {
            try {
                $cap = [Windows.Networking.NetworkOperators.NetworkOperatorTetheringManager]::GetTetheringCapabilityFromConnectionProfile($p)
                if ($cap.ToString() -eq 'Enabled') {
                    $profile = $p
                    $canTether = $true
                    break
                }
            } catch {}
        }
    }
    if ($null -ne $profile) {
        $manager = [Windows.Networking.NetworkOperators.NetworkOperatorTetheringManager]::CreateFromConnectionProfile($profile)
        $state = $manager.TetheringOperationalState.ToString()
        $waitCount = 0
        while ($state -eq 'InTransition' -and $waitCount -lt 4) {
            Start-Sleep -Milliseconds 1000
            $waitCount++
            $state = $manager.TetheringOperationalState.ToString()
        }
        if ($state -eq 'Off') {
            $res = AwaitAction ($manager.StartTetheringAsync()) ([Windows.Networking.NetworkOperators.NetworkOperatorTetheringOperationResult])
            $status = $res.Status.ToString()
            if ($status -eq 'Success' -or $status -eq 'AlreadyInProgress' -or $status -eq 'OperationInProgress') {
                Write-Output "STARTED:Success"
            } else {
                Write-Output "STARTED:$status"
            }
        } else {
            Write-Output "ACTIVE:$state"
        }
    } else {
        Write-Output "NO_PROFILE"
    }
} catch {
    Write-Output "ERR:$($_.Exception.Message)"
}
"#;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let mut cmd = tokio::process::Command::new("powershell");
        cmd.creation_flags(CREATE_NO_WINDOW);
        cmd.kill_on_drop(true);
        cmd.args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script]);

        let output = match tokio::time::timeout(std::time::Duration::from_secs(12), cmd.output()).await {
            Ok(Ok(out)) => out,
            Ok(Err(e)) => return Err(format!("调用移动热点保活失败: {}", e)),
            Err(_) => return Err("移动热点保活执行超时 (12 秒)，已中止".to_string()),
        };
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if !stdout.is_empty() {
            Ok(stdout)
        } else if !stderr.is_empty() {
            Err(format!("PowerShell stderr: {}", stderr))
        } else {
            Ok("EMPTY_OUTPUT".to_string())
        }
    }
    #[cfg(not(windows))]
    {
        Ok("UNSUPPORTED_PLATFORM".to_string())
    }
}

// ============= 配置文件路径 =============

fn get_config_path() -> PathBuf {
    let base = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("xxgcxy-wifi");
    let _ = fs::create_dir_all(&base);
    base.join("config.json")
}

// ============= 加载/保存配置 =============

#[tauri::command]
fn load_config(state: tauri::State<'_, AppState>) -> Result<Config, String> {
    let config_path = get_config_path();
    if config_path.exists() {
        let content =
            fs::read_to_string(&config_path).map_err(|e| format!("读取配置文件失败: {}", e))?;
        let config: Config =
            serde_json::from_str(strip_bom(&content)).map_err(|e| format!("解析配置文件失败: {}", e))?;
        let mut current_config = state.config.lock().unwrap_or_else(|e| e.into_inner());
        *current_config = config.clone();
        if !config.primary_ssid.is_empty() {
            let mut first_run = state.first_run.lock().unwrap_or_else(|e| e.into_inner());
            *first_run = false;
        }
        Ok(config)
    } else {
        Ok(Config::default())
    }
}

#[tauri::command]
fn save_config(config: Config, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let config_path = get_config_path();
    let content = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("序列化配置失败: {}", e))?;
    fs::write(&config_path, content)
        .map_err(|e| format!("写入配置文件失败: {}", e))?;
    let mut current_config = state.config.lock().unwrap_or_else(|e| e.into_inner());
    *current_config = config;
    let mut first_run = state.first_run.lock().unwrap_or_else(|e| e.into_inner());
    *first_run = false;
    Ok(())
}

// ============= WiFi 扫描（跨平台） =============

#[tauri::command]
async fn scan_wifi() -> Result<Vec<WifiNetwork>, String> {
    #[cfg(windows)]
    {
        let _ = hidden_command("netsh")
            .args(["wlan", "scan"])
            .output();
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let output = hidden_command("netsh")
            .args(["wlan", "show", "networks", "mode=bssid"])
            .output()
            .map_err(|e| format!("执行扫描命令失败: {}", e))?;
        let stdout = decode_console_output(&output.stdout);

        // 诊断: netsh 命令本身失败(典型: "系统上没有无线接口")在 stdout 不显示但在 stderr 显示
        // 把 stderr 信息带上,方便定位"没扫到 WiFi"的根因
        let stderr_text = String::from_utf8_lossy(&output.stderr).to_string();
        if !output.status.success() || stdout.trim().is_empty() {
            return Err(format!(
                "netsh 扫描失败: exit={:?}, stdout_len={}, stderr={}",
                output.status.code(),
                stdout.len(),
                if stderr_text.is_empty() { "<empty>".to_string() } else { stderr_text.trim().to_string() }
            ));
        }

        let mut networks = Vec::new();
        let mut current_ssid = String::new();
        let mut current_signal: u8 = 0;
        let mut current_secured = false;
        for line in stdout.lines() {
            let line = line.trim();
            if line.starts_with("SSID") && line.contains(':') {
                if !current_ssid.is_empty() {
                    networks.push(WifiNetwork {
                        ssid: current_ssid.clone(),
                        signal: current_signal,
                        secured: current_secured,
                    });
                }
                let parts: Vec<&str> = line.splitn(2, ':').collect();
                if parts.len() > 1 {
                    current_ssid = parts[1].trim().to_string();
                }
                current_signal = 0;
                current_secured = false;
            } else if line.starts_with("信号") || line.starts_with("Signal") {
                if line.contains(':') {
                    let parts: Vec<&str> = line.splitn(2, ':').collect();
                    if parts.len() > 1 {
                        let signal_str = parts[1].trim();
                        if let Ok(pct) = signal_str.replace('%', "").trim().parse::<u8>() {
                            current_signal = pct;
                        }
                    }
                }
            } else if line.starts_with("身份验证") || line.starts_with("Authentication") {
                current_secured = !(line.contains("开放") || line.contains("Open"));
            }
        }
        if !current_ssid.is_empty() {
            networks.push(WifiNetwork {
                ssid: current_ssid,
                signal: current_signal,
                secured: current_secured,
            });
        }
        let mut seen = std::collections::HashSet::new();
        networks.retain(|n| {
            if seen.contains(&n.ssid) {
                false
            } else {
                seen.insert(n.ssid.clone());
                true
            }
        });
        networks.sort_by(|a, b| b.signal.cmp(&a.signal));
        Ok(networks)
    }

    #[cfg(not(windows))]
    {
        let output = std::process::Command::new("nmcli")
            .args(["-t", "-m", "multiline", "device", "wifi", "list", "--rescan", "yes"])
            .output()
            .map_err(|e| format!("执行扫描命令失败: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut networks: Vec<WifiNetwork> = Vec::new();
        let mut current_ssid = String::new();
        let mut current_signal: u8 = 0;
        let mut current_secured = false;

        for line in stdout.lines() {
            let line = line.trim();
            if line.starts_with("SSID:") {
                if !current_ssid.is_empty() {
                    networks.push(WifiNetwork {
                        ssid: current_ssid.clone(),
                        signal: current_signal,
                        secured: current_secured,
                    });
                }
                current_ssid = line.trim_start_matches("SSID:").trim().to_string();
                current_signal = 0;
                current_secured = false;
            } else if line.starts_with("SIGNAL:") {
                let sig_str = line.trim_start_matches("SIGNAL:").trim();
                if let Ok(s) = sig_str.parse::<u8>() {
                    current_signal = s;
                }
            } else if line.starts_with("SECURITY:") {
                let sec_str = line.trim_start_matches("SECURITY:").trim();
                current_secured = !sec_str.is_empty() && sec_str != "--";
            }
        }
        if !current_ssid.is_empty() {
            networks.push(WifiNetwork {
                ssid: current_ssid,
                signal: current_signal,
                secured: current_secured,
            });
        }

        let mut seen = std::collections::HashSet::new();
        networks.retain(|n| {
            if seen.contains(&n.ssid) {
                false
            } else {
                seen.insert(n.ssid.clone());
                true
            }
        });
        networks.sort_by(|a, b| b.signal.cmp(&a.signal));
        Ok(networks)
    }
}

// ============= 连接 WiFi（跨平台） =============

#[tauri::command]
async fn connect_wifi(ssid: String) -> Result<(), String> {
    #[cfg(windows)]
    {
        // 先断开当前连接
        let _ = hidden_command("netsh")
            .args(["wlan", "disconnect"])
            .output();
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        // netsh wlan connect 需要 name= 参数（配置文件名），ssid= 不能单独使用
        let output = hidden_command("netsh")
            .args(["wlan", "connect", &format!("name={}", ssid)])
            .output()
            .map_err(|e| format!("执行连接命令失败: {}", e))?;

        if output.status.success() {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            return Ok(());
        }

        // 如果配置文件不存在，创建开放网络配置文件后导入
        let escaped_ssid = ssid.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
            .replace('"', "&quot;").replace('\'', "&apos;");
        let profile_xml = format!(
            r#"<?xml version="1.0"?>
<WLANProfile xmlns="http://www.microsoft.com/networking/WLAN/profile/v1">
    <name>{ssid}</name>
    <SSIDConfig>
        <SSID>
            <name>{ssid}</name>
        </SSID>
    </SSIDConfig>
    <connectionType>ESS</connectionType>
    <connectionMode>auto</connectionMode>
    <MSM>
        <security>
            <authEncryption>
                <authentication>open</authentication>
                <encryption>none</encryption>
                <useOneX>false</useOneX>
            </authEncryption>
        </security>
    </MSM>
</WLANProfile>"#,
            ssid = escaped_ssid
        );

        // 用唯一临时文件名, 在所有路径(包括错误)上确保清理
        let tmp_dir = std::env::temp_dir();
        let profile_path = tmp_dir.join(format!(
            "xxgcxy_wifi_{}_{}.xml",
            ssid.replace(|c: char| !c.is_alphanumeric(), ""),
            std::process::id()
        ));

        // RAII 守卫: 离开作用域时自动删除临时文件 (即使中途 panic)
        struct TempFileGuard(PathBuf);
        impl Drop for TempFileGuard {
            fn drop(&mut self) {
                let _ = fs::remove_file(&self.0);
            }
        }
        let _guard = TempFileGuard(profile_path.clone());
        let profile_path_str = profile_path.to_string_lossy().to_string();

        // 写文件
        // guard 在 fs::write 失败时通过 ? 早返回, 自动 drop 删除临时文件
        fs::write(&profile_path, &profile_xml)
            .map_err(|e| format!("创建配置文件失败: {}", e))?;

        // 导入配置文件 (不传播错误, 即使失败也继续重试 connect)
        let add_result = hidden_command("netsh")
            .args(["wlan", "add", "profile", &format!("filename={}", profile_path_str)])
            .output();
        if let Err(e) = &add_result {
            log::warn!("导入WLAN配置文件失败: {e}");
        }

        // 再次尝试连接（使用 name=）
        // guard 不显式 drop: 函数返回时 Rust 自动 drop 所有 locals, 触发清理
        let result = hidden_command("netsh")
            .args(["wlan", "connect", &format!("name={}", ssid)])
            .output();

        match result {
            Ok(output) => {
                if output.status.success() {
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                    Ok(())
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    Err(format!("连接 WiFi 失败: {} {}", stderr, stdout))
                }
            }
            Err(e) => Err(format!("执行连接命令失败: {}", e)),
        }
        // 函数末尾 guard 自动 drop, 临时文件清理
    }

    #[cfg(not(windows))]
    {
        let output = std::process::Command::new("nmcli")
            .args(["device", "wifi", "connect", &ssid])
            .output()
            .map_err(|e| format!("执行连接命令失败: {}", e))?;
        if output.status.success() {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Err(format!("连接 WiFi 失败: {}", stderr))
        }
    }
}

// ============= 获取当前连接的 WiFi（跨平台） =============

fn get_connected_wifi() -> Option<String> {
    #[cfg(windows)]
    {
        let output = hidden_command("netsh")
            .args(["wlan", "show", "interfaces"])
            .output()
            .ok()?;
        let stdout = decode_console_output(&output.stdout);
        for line in stdout.lines() {
            let line = line.trim();
            if line.starts_with("SSID") && !line.starts_with("BSSID") && line.contains(':') {
                let parts: Vec<&str> = line.splitn(2, ':').collect();
                if parts.len() > 1 {
                    let ssid = parts[1].trim().to_string();
                    if !ssid.is_empty() {
                        return Some(ssid);
                    }
                }
            }
        }
        None
    }

    #[cfg(not(windows))]
    {
        let output = std::process::Command::new("nmcli")
            .args(["-t", "-m", "multiline", "device", "show"])
            .output()
            .ok()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut in_wifi_device = false;
        for line in stdout.lines() {
            let line = line.trim();
            if line.starts_with("GENERAL.DEVICE:") {
                let dev = line.trim_start_matches("GENERAL.DEVICE:").trim();
                if dev.contains("wlo")
                    || dev.contains("wlan")
                    || dev.contains("wlp")
                    || dev.contains("wifi")
                {
                    in_wifi_device = true;
                } else {
                    in_wifi_device = false;
                }
            } else if line.starts_with("GENERAL.CONNECTION:") && in_wifi_device {
                let conn = line.trim_start_matches("GENERAL.CONNECTION:").trim();
                if !conn.is_empty() && conn != "--" {
                    return Some(conn.to_string());
                }
            }
        }
        None
    }
}

async fn get_connected_wifi_async() -> Option<String> {
    tokio::task::spawn_blocking(get_connected_wifi).await.unwrap_or(None)
}

// ============= 检测互联网连接 =============

async fn check_internet() -> bool {
    // 优先使用国内高可用 HTTP 204 探针检测（避免 HTTPS 绕过 captive portal）
    match check_url("http://connect.rom.miui.com/generate_204").await {
        CheckResult::Connected => return true,
        CheckResult::NeedLogin => return false,
        CheckResult::Error => {}
    }
    match check_url("http://connectivitycheck.platform.hicloud.com/generate_204").await {
        CheckResult::Connected => return true,
        CheckResult::NeedLogin => return false,
        CheckResult::Error => {}
    }
    false
}

// ============= 带重试的互联网检测 =============

async fn check_internet_with_retry() -> bool {
    for i in 0..2 {
        if check_internet().await {
            return true;
        }
        if i < 1 {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }
    false
}

enum CheckResult {
    Connected,
    NeedLogin,
    Error,
}

async fn check_url(url: &str) -> CheckResult {
    let url = url.to_string();
    let result = tokio::task::spawn_blocking(move || {
        let client = match reqwest::blocking::Client::builder()
            .no_proxy()
            .local_address(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED))
            .timeout(std::time::Duration::from_secs(3))
            .redirect(reqwest::redirect::Policy::none())
            .build()
        {
            Ok(c) => c,
            Err(_) => return CheckResult::Error,
        };
        let mut response = match client.get(&url).send() {
            Ok(r) => r,
            Err(_) => return CheckResult::Error,
        };
        let status = response.status();

        // 1. 对于 generate_204 探测端点，正常联网绝不发生 3xx 重定向
        // 发生重定向即 100% 代表被内网网关或 Captive Portal 劫持
        if status.is_redirection() {
            return CheckResult::NeedLogin;
        }

        // 2. 正常 204 无内容响应，表示直通外网
        if status.as_u16() == 204 {
            return CheckResult::Connected;
        }

        // 3. 返回 200 OK 说明 204 端点被伪造/篡改返回了登录网页
        if status.is_success() {
            use std::io::Read;
            let mut buf = [0u8; 8192];
            let n = response.read(&mut buf).unwrap_or(0);
            let slice = &buf[..n];
            // 使用 lossy 解码兼容 UTF-8 和 GBK/GB2312 编码的登录页
            let content_lower = String::from_utf8_lossy(slice).to_lowercase();
            if content_lower.contains("portal")
                || content_lower.contains("drcom")
                || content_lower.contains("inode")
                || content_lower.contains("eportal")
                || content_lower.contains("srun")
                || content_lower.contains("wlanuserip")
                || content_lower.contains("认证")
                || content_lower.contains("登录")
            {
                return CheckResult::NeedLogin;
            }
            // generate_204 返回了非 204 且非预期的 200 页面，视为被 Portal 劫持
            return CheckResult::NeedLogin;
        }
        CheckResult::Error
    })
    .await;

    match result {
        Ok(r) => r,
        Err(_) => CheckResult::Error,
    }
}

// ============= 302 动态参数嗅探与网关特征提取 =============

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SniffedParams {
    pub base_url: Option<String>,
    pub wlan_user_ip: Option<String>,
    pub mac_address: Option<String>,
    pub vlan: Option<String>,
    pub wlan_ac_name: Option<String>,
    pub wlan_ac_ip: Option<String>,
    pub hostname: Option<String>,
}

/// 发送禁止重定向的 HTTP 探测包，截获 302 Location 中的真实参数
async fn sniff_portal_params() -> Result<Option<SniffedParams>, String> {
    let endpoints = [
        "http://connect.rom.miui.com/generate_204",
        "http://connectivitycheck.platform.hicloud.com/generate_204",
        "http://1.1.1.1",
    ];

    for endpoint in endpoints {
        let client = match reqwest::Client::builder()
            .no_proxy()
            .local_address(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED))
            .timeout(std::time::Duration::from_millis(2500))
            .redirect(reqwest::redirect::Policy::none())
            .build()
        {
            Ok(c) => c,
            Err(_) => continue,
        };

        if let Ok(resp) = client.get(endpoint).send().await {
            if resp.status().is_redirection() {
                if let Some(loc) = resp.headers().get("location") {
                    if let Ok(loc_str) = loc.to_str() {
                        if let Ok(parsed) = url::Url::parse(loc_str) {
                            let mut sniffed = SniffedParams::default();
                            let mut base = parsed.clone();
                            base.set_query(None);
                            base.set_fragment(None);
                            sniffed.base_url = Some(base.to_string());

                            for (k, v) in parsed.query_pairs() {
                                match k.to_ascii_lowercase().as_str() {
                                    "wlanuserip" => sniffed.wlan_user_ip = Some(v.to_string()),
                                    "mac" => sniffed.mac_address = Some(v.to_string()),
                                    "vlan" => sniffed.vlan = Some(v.to_string()),
                                    "wlanacname" => sniffed.wlan_ac_name = Some(v.to_string()),
                                    "wlanacip" => sniffed.wlan_ac_ip = Some(v.to_string()),
                                    "hostname" => sniffed.hostname = Some(v.to_string()),
                                    _ => {}
                                }
                            }
                            return Ok(Some(sniffed));
                        }
                    }
                }
            }
        }
    }
    Ok(None)
}

/// 校验 Portal Base URL 是否合法有效（防广告劫持与污染配置）
pub fn is_valid_portal_base_url(url_str: &str) -> bool {
    let t = url_str.trim();
    if t.is_empty() {
        return false;
    }
    let parsed = match url::Url::parse(t) {
        Ok(u) => u,
        Err(_) => return false,
    };
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return false;
    }
    let host = match parsed.host_str() {
        Some(h) => h.to_lowercase(),
        None => return false,
    };
    // 排除探测端点自身与常见外部域或运营商劫持广告域
    if host.contains("miui.com")
        || host.contains("hicloud.com")
        || host.contains("1.1.1.1")
        || host.contains("qq.com")
        || host.contains("baidu.com")
        || host.contains("bing.com")
        || host.contains("msftconnecttest.com")
        || host.contains("apple.com")
    {
        return false;
    }
    // 路径检查：必须是合法的 portal 路径 (包含 portal, quickauth 或以 .do 结尾)
    let path = parsed.path().to_lowercase();
    path.contains("portal") || path.contains("quickauth") || path.ends_with(".do") || path.contains("eportal")
}

/// 将嗅探到的参数安全同步回登录配置
fn apply_sniffed_params_to_profile(profile: &mut LoginProfile, sniffed: &SniffedParams) -> bool {
    let mut changed = false;
    if let Some(ip) = &sniffed.wlan_user_ip {
        if !is_dummy_ip(ip) && profile.wlan_user_ip != *ip {
            profile.wlan_user_ip = ip.clone();
            changed = true;
        }
    }
    if let Some(mac) = &sniffed.mac_address {
        if !is_dummy_mac(mac) && profile.mac_address != *mac {
            profile.mac_address = mac.clone();
            changed = true;
        }
    }
    if let Some(vlan) = &sniffed.vlan {
        if !vlan.trim().is_empty() && profile.vlan != *vlan {
            profile.vlan = vlan.clone();
            changed = true;
        }
    }
    if let Some(ac_name) = &sniffed.wlan_ac_name {
        if !ac_name.trim().is_empty() && profile.wlan_ac_name != *ac_name {
            profile.wlan_ac_name = ac_name.clone();
            changed = true;
        }
    }
    if let Some(ac_ip) = &sniffed.wlan_ac_ip {
        if !is_dummy_ip(ac_ip) && profile.wlan_ac_ip != *ac_ip {
            profile.wlan_ac_ip = ac_ip.clone();
            changed = true;
        }
    }
    if let Some(base) = &sniffed.base_url {
        if is_valid_portal_base_url(base) && profile.base_url != *base {
            profile.base_url = base.clone();
            changed = true;
        }
    }
    changed
}

/// 假 IP 防御检测
fn is_dummy_ip(ip: &str) -> bool {
    let t = ip.trim();
    t.is_empty() || t == "0.0.0.0" || t == "127.0.0.1" || t == "10.0.0.1"
}

/// 假 MAC 防御检测
fn is_dummy_mac(mac: &str) -> bool {
    let t = mac.trim().to_lowercase();
    t.is_empty()
        || t == "00:00:00:00:00:00"
        || t == "aa:bb:cc:dd:ee:ff"
        || t.chars().filter(|c| *c == ':').count() != 5
}

/// 严谨的 quickauth.do 认证地址解析，彻底杜绝尾斜杠/无斜杠 URL 拼接错误
fn resolve_quickauth_url(raw_base: &str) -> Result<String, String> {
    let clean = raw_base
        .split('?')
        .next()
        .unwrap_or(raw_base)
        .split('#')
        .next()
        .unwrap_or(raw_base)
        .trim();
    if clean.is_empty() {
        return Err("Base URL 为空".to_string());
    }
    if clean.ends_with("/quickauth.do") {
        return Ok(clean.to_string());
    }
    let with_scheme = if !clean.starts_with("http://") && !clean.starts_with("https://") {
        format!("http://{}", clean)
    } else {
        clean.to_string()
    };
    let parsed = url::Url::parse(&with_scheme)
        .map_err(|e| format!("解析 Base URL 失败: {}", e))?;
    let path = parsed.path();
    let new_path = if path.is_empty() || path == "/" {
        "/quickauth.do".to_string()
    } else if path.ends_with("/portal.do") {
        path.replace("/portal.do", "/quickauth.do")
    } else if let Some(idx) = path.rfind('/') {
        if idx == 0 {
            "/quickauth.do".to_string()
        } else {
            format!("{}/quickauth.do", &path[..idx])
        }
    } else {
        format!("{}/quickauth.do", path.trim_end_matches('/'))
    };
    let mut out = parsed;
    out.set_path(&new_path);
    out.set_query(None);
    out.set_fragment(None);
    Ok(out.to_string())
}

fn emit_backend_log(app: &AppHandle, msg: &str) {
    log::info!("{}", msg);
    let _ = app.emit("backend-log", msg);
}

fn emit_service_status(app: &AppHandle, status: &ServiceStatus) {
    let state = app.state::<AppState>();
    if let Ok(mut lock) = state.service_status.lock() {
        *lock = status.clone();
    }
    let _ = app.emit("service-status-changed", status);
}

// ============= 检测网络状态 =============

#[tauri::command]
async fn check_network(state: tauri::State<'_, AppState>) -> Result<NetworkStatus, String> {
    let config = state.config.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let wifi_connected = get_connected_wifi();
    let internet_ok = check_internet_with_retry().await;
    let is_matched_ssid = |cur: &str, target: &str| -> bool {
        !target.is_empty() && cur.eq_ignore_ascii_case(target)
    };
    let has_configured_ssid = !config.primary_ssid.is_empty() || !config.backup_ssid.is_empty();
    let connected_matches = match &wifi_connected {
        Some(ssid) => {
            if !has_configured_ssid {
                true
            } else {
                is_matched_ssid(ssid, &config.primary_ssid) || is_matched_ssid(ssid, &config.backup_ssid)
            }
        }
        None => false,
    };
    let needs_reconnect = wifi_connected.is_none()
        || (has_configured_ssid && !connected_matches);
    let needs_login = wifi_connected.is_some()
        && !internet_ok
        && connected_matches;
    Ok(NetworkStatus {
        wifi_connected,
        internet_ok,
        needs_reconnect,
        needs_login,
    })
}

// ============= 运行登录脚本 =============

fn clean_script_output(s: &str) -> String {
    let mut cleaned_lines = Vec::new();
    let mut prev_empty = false;
    for line in s.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !prev_empty && !cleaned_lines.is_empty() {
                cleaned_lines.push("");
                prev_empty = true;
            }
        } else {
            cleaned_lines.push(trimmed);
            prev_empty = false;
        }
    }
    cleaned_lines.join("\n").trim().to_string()
}

// ============= Rust 进程内原生极速直发 (Sub-100ms Instant Login) =============

fn url_encode_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                out.push_str(&format!("%{:02X}", b));
            }
        }
    }
    out
}

fn generate_uuid() -> String {
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let pid = std::process::id() as u128;
    let mut val = nanos ^ (pid << 64) ^ 0x9e3779b97f4a7c15_u128;
    val = val.wrapping_mul(0xbf58476d1ce4e5b9_u128) ^ (val >> 30);
    let mut bytes = val.to_be_bytes();
    bytes[6] = (bytes[6] & 0x0f) | 0x40; // Version 4 (RFC 4122)
    bytes[8] = (bytes[8] & 0x3f) | 0x80; // Variant 1 (RFC 4122)
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5],
        bytes[6], bytes[7],
        bytes[8], bytes[9],
        bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
    )
}

/// 解码控制台程序输出: 中文 Windows 的 netsh 输出为 GBK 编码,
/// String::from_utf8_lossy 会把 "物理地址"/"IPv4 地址" 等标签变成乱码导致 MAC/IP 永远解析失败
#[allow(dead_code)]
fn decode_console_output(bytes: &[u8]) -> String {
    // 若字节本就是合法 UTF-8 (如系统开启 "Beta: 使用 Unicode UTF-8 提供全球语言支持") 则直接使用
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_string();
    }
    #[cfg(windows)]
    let decoded = {
        let (decoded, _, _) = encoding_rs::GBK.decode(bytes);
        decoded.into_owned()
    };
    #[cfg(not(windows))]
    let decoded = String::from_utf8_lossy(bytes).into_owned();
    decoded
}

fn get_wlan_network_info() -> (Option<String>, Option<String>, Option<String>) {
    #[cfg(windows)]
    {
        let mut ssid = None;
        let mut mac = None;
        let mut iface_name = "WLAN".to_string();

        if let Ok(output) = hidden_command("netsh").args(["wlan", "show", "interfaces"]).output() {
            let stdout = decode_console_output(&output.stdout);
            for line in stdout.lines() {
                let line = line.trim();
                if (line.starts_with("Name") || line.starts_with("名称")) && line.contains(':') {
                    if let Some(val) = line.splitn(2, ':').nth(1) {
                        let n = val.trim();
                        if !n.is_empty() { iface_name = n.to_string(); }
                    }
                } else if (line.starts_with("Physical address") || line.starts_with("物理地址")) && line.contains(':') {
                    if let Some(val) = line.splitn(2, ':').nth(1) {
                        let m = val.trim().replace('-', ":").to_lowercase();
                        if !m.is_empty() { mac = Some(m); }
                    }
                } else if line.starts_with("SSID") && !line.starts_with("BSSID") && line.contains(':') {
                    if let Some(val) = line.splitn(2, ':').nth(1) {
                        let s = val.trim().to_string();
                        if !s.is_empty() { ssid = Some(s); }
                    }
                }
            }
        }

        let mut ip = None;
        if let Ok(output) = hidden_command("netsh").args(["interface", "ipv4", "show", "addresses", &iface_name]).output() {
            let stdout = decode_console_output(&output.stdout);
            for line in stdout.lines() {
                let line = line.trim();
                if (line.starts_with("IP Address") || line.starts_with("IP 地址") || line.starts_with("IPv4 地址")) && line.contains(':') {
                    if let Some(val) = line.splitn(2, ':').nth(1) {
                        let addr = val.trim().to_string();
                        if !addr.is_empty() && !addr.starts_with("169.254") && addr != "127.0.0.1" {
                            ip = Some(addr);
                            break;
                        }
                    }
                }
            }
        }

        // 兜底：若按网卡名未查询到 IP，遍历所有 IPv4 地址寻找非回环、非保留的首个活动 IP
        if ip.is_none() {
            if let Ok(output) = hidden_command("netsh").args(["interface", "ipv4", "show", "addresses"]).output() {
                let stdout = decode_console_output(&output.stdout);
                for line in stdout.lines() {
                    let line = line.trim();
                    if (line.starts_with("IP Address") || line.starts_with("IP 地址") || line.starts_with("IPv4 地址")) && line.contains(':') {
                        if let Some(val) = line.splitn(2, ':').nth(1) {
                            let addr = val.trim().to_string();
                            if !addr.is_empty() && !addr.starts_with("169.254") && addr != "127.0.0.1" {
                                ip = Some(addr);
                                break;
                            }
                        }
                    }
                }
            }
        }

        (ssid, mac, ip)
    }
    #[cfg(not(windows))]
    {
        (None, None, None)
    }
}

async fn get_wlan_network_info_async() -> (Option<String>, Option<String>, Option<String>) {
    tokio::task::spawn_blocking(get_wlan_network_info).await.unwrap_or((None, None, None))
}

/// 进程内异步直发认证请求 (零外部脚本依赖，毫秒级响应)
async fn native_direct_login() -> Result<String, String> {
    let mut profile = get_login_profile()?;
    if profile.user_id.is_empty() || profile.base_url.is_empty() {
        return Err("未配置校园网账号或 Portal URL".to_string());
    }

    let cred_path = get_login_credential_path();
    if !cred_path.exists() {
        return Err("凭据文件不存在，请先在主页或网络配置页保存配置".to_string());
    }
    let cred_bytes = fs::read(&cred_path).map_err(|e| format!("读取凭据文件失败: {}", e))?;
    let password = decrypt_password(&cred_bytes)?;
    if password.is_empty() {
        return Err("解密密码为空，请在设置中重新保存账号密码".to_string());
    }

    // 1. 登录前置强制取参：先发一次禁重定向的 HTTP 探测，302 就解析 Location 动态刷新参数
    if let Ok(Some(sniffed)) = sniff_portal_params().await {
        if apply_sniffed_params_to_profile(&mut profile, &sniffed) {
            let _ = save_login_profile_json(&profile);
        }
    }

    // 2. 提取或自动获取网络硬件信息 (IP, MAC, SSID)
    let (detected_ssid, detected_mac, detected_ip) = get_wlan_network_info_async().await;

    // 彻底删除硬编码 SSID，完全采用用户配置或当前连接的真实 SSID
    let ssid = if !profile.ssid.trim().is_empty() {
        profile.ssid.trim().to_string()
    } else if let Some(s) = detected_ssid {
        s
    } else {
        String::new()
    };

    let mac = if !profile.mac_address.trim().is_empty() {
        profile.mac_address.trim().to_lowercase()
    } else if let Some(m) = detected_mac {
        m
    } else {
        String::new()
    };

    let user_ip = if !profile.wlan_user_ip.trim().is_empty() {
        profile.wlan_user_ip.trim().to_string()
    } else if let Some(ip) = detected_ip {
        ip
    } else {
        String::new()
    };

    // 3. 兜底值安全拦截：假 IP / 假 MAC / 全零 一律禁止向网关发包，以防计费异常
    if is_dummy_ip(&user_ip) || is_dummy_mac(&mac) {
        return Err(format!(
            "安全拦截: 未获取到有效的真实网卡参数 (IP='{}', MAC='{}')，拒绝发送伪造参数数据包",
            user_ip, mac
        ));
    }

    let hostname = if !profile.hostname.trim().is_empty() {
        profile.hostname.trim().to_string()
    } else {
        std::env::var("COMPUTERNAME").unwrap_or_else(|_| "DESKTOP-PC".to_string())
    };

    let portal_page_id = if !profile.portal_page_id.is_empty() { profile.portal_page_id } else { "3".to_string() };
    let portal_type = if !profile.portal_type.is_empty() { profile.portal_type } else { "0".to_string() };
    let version = if !profile.version.is_empty() { profile.version } else { "0".to_string() };
    let bind_ctrl_id = profile.bind_ctrl_id;

    // 4. 严谨净化与解析 BaseURL，杜绝尾斜杠/无斜杠 URL 拼接错误
    let auth_url = resolve_quickauth_url(&profile.base_url)?;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let uuid = generate_uuid();

    // 构造 Query String
    let query_params = vec![
        format!("userid={}", url_encode_component(&profile.user_id)),
        format!("passwd={}", url_encode_component(&password)),
        format!("wlanuserip={}", url_encode_component(&user_ip)),
        format!("wlanacname={}", url_encode_component(&profile.wlan_ac_name)),
        format!("wlanacIp={}", url_encode_component(&profile.wlan_ac_ip)),
        format!("ssid={}", url_encode_component(&ssid)),
        format!("vlan={}", url_encode_component(&profile.vlan)),
        format!("mac={}", url_encode_component(&mac)),
        format!("version={}", url_encode_component(&version)),
        format!("portalpageid={}", url_encode_component(&portal_page_id)),
        format!("timestamp={}", timestamp),
        format!("uuid={}", uuid),
        format!("portaltype={}", url_encode_component(&portal_type)),
        format!("hostname={}", url_encode_component(&hostname)),
        format!("bindCtrlId={}", url_encode_component(&bind_ctrl_id)),
    ].join("&");

    let request_url = format!("{}?{}", auth_url, query_params);
    let portal_referer = format!("{}/portal.do", auth_url.trim_end_matches("/quickauth.do"));

    let client = reqwest::Client::builder()
        .no_proxy()
        .local_address(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED))
        .connect_timeout(std::time::Duration::from_millis(2000))
        .timeout(std::time::Duration::from_secs(6))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| format!("构建 HTTP 客户端失败: {}", e))?;

    let start_instant = std::time::Instant::now();
    let resp = client
        .get(&request_url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36")
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8")
        .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
        .header("Referer", portal_referer)
        .header("X-Requested-With", "XMLHttpRequest")
        .send()
        .await
        .map_err(|e| format!("网络请求失败: {}", e))?;

    let elapsed = start_instant.elapsed().as_millis();

    // 成功判定彻底收口：删掉 302 算成功和模糊字符串匹配
    if resp.status().is_redirection() {
        return Err(format!("认证未通过: 网关返回重定向状态码 {}", resp.status()));
    }

    let body = resp.text().await.map_err(|e| format!("读取响应失败: {}", e))?;

    // 解析 JSON 响应
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
        let code_str = match v.get("code") {
            Some(serde_json::Value::String(s)) => s.clone(),
            Some(serde_json::Value::Number(n)) => n.to_string(),
            _ => "unknown".to_string(),
        };
        let msg = v.get("message")
            .or_else(|| v.get("msg"))
            .and_then(|m| m.as_str())
            .unwrap_or("");

        if code_str == "0" {
            // 发送成功 != 认证成功，必须进行 204 复验探针
            tokio::time::sleep(std::time::Duration::from_millis(400)).await;
            if check_internet_with_retry().await {
                Ok(format!("[+] 认证成功并复验通过 (耗时 {} ms): {}", elapsed, if msg.is_empty() { "success" } else { msg }))
            } else {
                Err(format!("[!] 网关返回 code=0 成功，但 204 外网复验未通过，网络尚未放行 (耗时 {} ms)", elapsed))
            }
        } else if code_str == "44" {
            Err(format!("CODE_44_RETRY: 认证被拒绝 (非法接入/VLAN会话失效, code: 44, 耗时 {} ms): {}", elapsed, msg))
        } else if code_str == "1" {
            if msg.contains("设备不在正常状态") {
                Err(format!("BUSINESS_REJECTED: 设备不在正常状态, 无法认证上网, 请稍候 (耗时 {} ms)", elapsed))
            } else {
                Err(format!("BUSINESS_REJECTED: 账号或密码错误 (code: 1, 耗时 {} ms): {}", elapsed, if msg.is_empty() { "请检查用户名密码" } else { msg }))
            }
        } else {
            Err(format!("[!] 认证异常 (code: {}, 耗时 {} ms): {}", code_str, elapsed, msg))
        }
    } else {
        // 非 JSON 响应，永远以 204 外网复验为准
        if check_internet_with_retry().await {
            Ok(format!("[+] 认证成功 (外网复验通过, 耗时 {} ms)", elapsed))
        } else {
            Err(format!("[!] 服务器返回非 JSON 响应且 204 复验未通过 (耗时 {} ms):\n{}", elapsed, clean_script_output(&body)))
        }
    }
}

/// 综合登录流水线：优先进程内极速直发，支持 code 44 自动重新嗅探重试，遇未知错误降级外部脚本
async fn execute_login_flow(app: &AppHandle) -> Result<String, String> {
    emit_backend_log(app, "[*] 开始执行校园网认证流程 (Rust 原生直发优先)...");

    let mut attempt = 0;
    let mut native_res;
    loop {
        attempt += 1;
        native_res = native_direct_login().await;
        if let Err(ref e) = native_res {
            if e.starts_with("CODE_44_RETRY") && attempt == 1 {
                emit_backend_log(app, "[!] 网关返回 code 44 (非法接入/会话失效)，正在强制 302 重新取参并重试一次...");
                if let Ok(Some(sniffed)) = sniff_portal_params().await {
                    if let Ok(mut profile) = get_login_profile() {
                        if apply_sniffed_params_to_profile(&mut profile, &sniffed) {
                            emit_backend_log(app, &format!("[*] 已根据 302 刷新权威参数: IP={:?}, MAC={:?}, VLAN={:?}",
                                sniffed.wlan_user_ip, sniffed.mac_address, sniffed.vlan));
                            let _ = save_login_profile_json(&profile);
                        }
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                continue;
            }
        }
        break;
    }

    match native_res {
        Ok(msg) => {
            emit_backend_log(app, &format!("[+] 原生直发成功: {}", msg));
            Ok(msg)
        }
        Err(e) => {
            if e.starts_with("BUSINESS_REJECTED") {
                emit_backend_log(app, &format!("[!] 认证被计费系统拒绝: {}", e));
                return Err(e);
            }
            emit_backend_log(app, &format!("[!] 原生直发未就绪: {}，正在调用外部脚本兜底...", e));
            run_login_script(app.clone()).await
        }
    }
}

#[tauri::command]
async fn run_login_script(app: AppHandle) -> Result<String, String> {
    let exe_dir = std::env::current_exe()
        .map(|p| p.parent().unwrap_or(std::path::Path::new(".")).to_path_buf())
        .unwrap_or_else(|_| PathBuf::from("."));

    #[cfg(windows)]
    let possible_bat_paths: Vec<PathBuf> = vec![
        exe_dir.join("xywdl.bat"),
        exe_dir.join("_up_").join("xywdl.bat"),
        app.path()
            .resource_dir()
            .map(|p| p.join("xywdl.bat"))
            .unwrap_or_default(),
        std::env::current_dir()
            .map(|p| p.join("xywdl.bat"))
            .unwrap_or_default(),
    ];

    #[cfg(not(windows))]
    let possible_sh_paths: Vec<PathBuf> = vec![
        exe_dir.join("xywdl.sh"),
        exe_dir.join("_up_").join("xywdl.sh"),
        app.path()
            .resource_dir()
            .map(|p| p.join("xywdl.sh"))
            .unwrap_or_default(),
        std::env::current_dir()
            .map(|p| p.join("xywdl.sh"))
            .unwrap_or_default(),
    ];

    #[cfg(windows)]
    let script_path = possible_bat_paths
        .into_iter()
        .find(|p| p.exists())
        .ok_or_else(|| format!("登录脚本不存在 (exe目录: {})", exe_dir.display()))?;

    #[cfg(not(windows))]
    let script_path = possible_sh_paths
        .into_iter()
        .find(|p| p.exists())
        .ok_or_else(|| format!("登录脚本不存在 (exe目录: {})", exe_dir.display()))?;

    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let mut cmd = tokio::process::Command::new(&script_path);
        cmd.creation_flags(CREATE_NO_WINDOW);
        cmd.kill_on_drop(true);
        cmd.arg("--non-interactive");

        let output = match tokio::time::timeout(std::time::Duration::from_secs(15), cmd.output()).await {
            Ok(Ok(out)) => out,
            Ok(Err(e)) => return Err(format!("执行登录脚本失败: {}", e)),
            Err(_) => return Err("执行登录脚本超时 (15 秒)，已中止以防挂起".to_string()),
        };

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        if output.status.success() {
            let trimmed = clean_script_output(&stdout);
            let detail = if trimmed.is_empty() {
                "登录脚本执行成功".to_string()
            } else {
                format!("登录脚本执行成功:\n{}", trimmed)
            };
            Ok(detail)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let combined = format!("{}{}", stderr, stdout);
            let trimmed = clean_script_output(&combined);
            Err(format!("登录脚本执行失败:\n{}", trimmed))
        }
    }

    #[cfg(not(windows))]
    {
        use tauri_plugin_shell::ShellExt;
        let shell = app.shell();
        let script_str = script_path.to_string_lossy().to_string();
        let cmd_future = shell
            .command("bash")
            .args(["-c", &format!("chmod +x '{}' && '{}' --non-interactive", script_str, script_str)])
            .output();

        let output = match tokio::time::timeout(std::time::Duration::from_secs(15), cmd_future).await {
            Ok(Ok(out)) => out,
            Ok(Err(e)) => return Err(format!("执行登录脚本失败: {}", e)),
            Err(_) => return Err("执行登录脚本超时 (15 秒)，已中止以防挂起".to_string()),
        };

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        if output.status.success() {
            let trimmed = clean_script_output(&stdout);
            let detail = if trimmed.is_empty() {
                "登录脚本执行成功".to_string()
            } else {
                format!("登录脚本执行成功:\n{}", trimmed)
            };
            Ok(detail)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let combined = format!("{}{}", stderr, stdout);
            let trimmed = clean_script_output(&combined);
            Err(format!("登录脚本执行失败:\n{}", trimmed))
        }
    }
}

// ============= 打开 GitHub =============
#[allow(deprecated)]

#[tauri::command]
async fn open_github(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_shell::ShellExt;
    app.shell()
        .open("https://github.com/Thatgfsj/XXGCXY-CampusNet-AutoLogin", None)
        .map_err(|e| format!("打开链接失败: {}", e))?;
    Ok(())
}

// ============= Rust 后端核心保活状态机 (tokio::spawn + interval) =============

async fn start_background_service(app: AppHandle, mut rx: tokio::sync::mpsc::Receiver<TriggerAction>) {
    emit_backend_log(&app, "[*] 启动 Rust 后端核心保活状态机 (自主探测-登录-复验状态机)...");

    let mut consecutive_business_errors: u32 = 0;
    let mut next_retry_instant: Option<std::time::Instant> = None;
    let mut transport_cooldown_until: Option<std::time::Instant> = None;
    let mut last_error_reason: String = String::new();

    loop {
        // 读取配置检测间隔 (默认 15s) 与开关状态
        let (interval_secs, check_enabled) = {
            let state = app.state::<AppState>();
            let cfg = state.config.lock().unwrap_or_else(|e| e.into_inner());
            let enabled = *state.check_enabled.lock().unwrap_or_else(|e| e.into_inner());
            let secs = if cfg.check_interval > 0 { cfg.check_interval } else { 15 };
            (secs, enabled)
        };

        let now = std::time::Instant::now();
        let in_backoff = if let Some(retry_at) = next_retry_instant {
            if now < retry_at {
                true
            } else {
                next_retry_instant = None;
                false
            }
        } else {
            false
        };

        // 传输层失败 (超时/连接重置等) 60 秒冷却: 未到期则跳过本轮自动登录, 防止高频轰炸认证服务器触发限流
        let transport_cooldown_active = if let Some(until) = transport_cooldown_until {
            now < until
        } else {
            false
        };

        let backoff_remaining_secs = if let Some(retry_at) = next_retry_instant {
            retry_at.saturating_duration_since(now).as_secs()
        } else {
            0
        };

        let timestamp_now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if check_enabled {
            // 1. 探测 WiFi 与互联网状态
            let wifi = get_connected_wifi_async().await;
            let has_internet = check_internet_with_retry().await;

            let (has_configured_ssid, connected_matches) = {
                let state = app.state::<AppState>();
                let cfg = state.config.lock().unwrap_or_else(|e| e.into_inner());
                let has_cfg = !cfg.primary_ssid.is_empty() || !cfg.backup_ssid.is_empty();
                let matches = match &wifi {
                    Some(s) => {
                        if !has_cfg {
                            true
                        } else {
                            (!cfg.primary_ssid.is_empty() && s.eq_ignore_ascii_case(&cfg.primary_ssid))
                                || (!cfg.backup_ssid.is_empty() && s.eq_ignore_ascii_case(&cfg.backup_ssid))
                        }
                    }
                    None => false,
                };
                (has_cfg, matches)
            };

            if wifi.is_none() || (has_configured_ssid && !connected_matches) {
                // WiFi 未连接或 SSID 不匹配
                emit_service_status(&app, &ServiceStatus {
                    state: ServiceState::Disconnected,
                    wifi_connected: wifi.clone(),
                    internet_ok: has_internet,
                    last_check_time: timestamp_now,
                    consecutive_business_errors,
                    backoff_remaining_secs,
                });
            } else if has_internet {
                // 互联网畅通，重置业务退避计数器
                if consecutive_business_errors > 0 || next_retry_instant.is_some() {
                    consecutive_business_errors = 0;
                    next_retry_instant = None;
                    last_error_reason.clear();
                }
                emit_service_status(&app, &ServiceStatus {
                    state: ServiceState::Connected,
                    wifi_connected: wifi.clone(),
                    internet_ok: true,
                    last_check_time: timestamp_now,
                    consecutive_business_errors: 0,
                    backoff_remaining_secs: 0,
                });
            } else {
                // WiFi 已就绪但外网不通 -> 需认证
                if in_backoff {
                    emit_service_status(&app, &ServiceStatus {
                        state: ServiceState::Backoff {
                            next_retry_secs: backoff_remaining_secs,
                            reason: last_error_reason.clone(),
                        },
                        wifi_connected: wifi.clone(),
                        internet_ok: false,
                        last_check_time: timestamp_now,
                        consecutive_business_errors,
                        backoff_remaining_secs,
                    });
                } else if transport_cooldown_active {
                    // 传输层冷却未到期: 保持 NeedsLogin 状态但不发起登录请求
                    emit_service_status(&app, &ServiceStatus {
                        state: ServiceState::NeedsLogin,
                        wifi_connected: wifi.clone(),
                        internet_ok: false,
                        last_check_time: timestamp_now,
                        consecutive_business_errors,
                        backoff_remaining_secs: 0,
                    });
                } else {
                    emit_service_status(&app, &ServiceStatus {
                        state: ServiceState::LoggingIn,
                        wifi_connected: wifi.clone(),
                        internet_ok: false,
                        last_check_time: timestamp_now,
                        consecutive_business_errors,
                        backoff_remaining_secs: 0,
                    });

                    let login_res = {
                        let state = app.state::<AppState>();
                        let _guard = state.is_logging_in.lock().await;
                        execute_login_flow(&app).await
                    };

                    match login_res {
                        Ok(_) => {
                            consecutive_business_errors = 0;
                            next_retry_instant = None;
                            transport_cooldown_until = None;
                            last_error_reason.clear();
                            emit_service_status(&app, &ServiceStatus {
                                state: ServiceState::Connected,
                                wifi_connected: wifi.clone(),
                                internet_ok: true,
                                last_check_time: timestamp_now,
                                consecutive_business_errors: 0,
                                backoff_remaining_secs: 0,
                            });
                        }
                        Err(err) => {
                            if err.starts_with("BUSINESS_REJECTED") {
                                consecutive_business_errors += 1;
                                // 指数退避: 15s -> 30s -> 60s -> 120s -> 300s
                                let backoff_secs = match consecutive_business_errors {
                                    1 => 15,
                                    2 => 30,
                                    3 => 60,
                                    4 => 120,
                                    _ => 300,
                                };
                                next_retry_instant = Some(std::time::Instant::now() + std::time::Duration::from_secs(backoff_secs));
                                last_error_reason = err.clone();
                                let alert_msg = format!("认证被拒绝: {}。已进入保护性退避 (将在 {} 秒后重试)，防止高频轰炸导致锁号。", err, backoff_secs);
                                emit_backend_log(&app, &alert_msg);
                                let _ = app.emit("backend-alert", &alert_msg);

                                emit_service_status(&app, &ServiceStatus {
                                    state: ServiceState::Backoff {
                                        next_retry_secs: backoff_secs,
                                        reason: last_error_reason.clone(),
                                    },
                                    wifi_connected: wifi.clone(),
                                    internet_ok: false,
                                    last_check_time: timestamp_now,
                                    consecutive_business_errors,
                                    backoff_remaining_secs: backoff_secs,
                                });
                            } else {
                                // 非业务拒绝的登录失败 (超时/连接重置等传输层错误): 60 秒后再自动重试
                                transport_cooldown_until = Some(std::time::Instant::now() + std::time::Duration::from_secs(60));
                                emit_service_status(&app, &ServiceStatus {
                                    state: ServiceState::NeedsLogin,
                                    wifi_connected: wifi.clone(),
                                    internet_ok: false,
                                    last_check_time: timestamp_now,
                                    consecutive_business_errors,
                                    backoff_remaining_secs: 0,
                                });
                            }
                        }
                    }
                }
            }
        }

        // 等待下一个周期或被外部信号打断 (例如托盘点击立即检测/立即登录)
        let sleep_duration = if in_backoff {
            std::time::Duration::from_secs(1.max(backoff_remaining_secs.min(5)))
        } else {
            std::time::Duration::from_secs(interval_secs)
        };

        tokio::select! {
            _ = tokio::time::sleep(sleep_duration) => {}
            action = rx.recv() => {
                match action {
                    Some(TriggerAction::Check) => {
                        emit_backend_log(&app, "[*] 收到立即检测信号");
                    }
                    Some(TriggerAction::Login) => {
                        emit_backend_log(&app, "[*] 收到手动立即登录信号，重置退避计时器");
                        consecutive_business_errors = 0;
                        next_retry_instant = None;
                        transport_cooldown_until = None;
                        last_error_reason.clear();
                        // 消费积压的重复登录信号，防止排队并发
                        while let Ok(action) = rx.try_recv() {
                            if action != TriggerAction::Login {
                                // 暂存或其它信号
                            }
                        }
                        let state = app.state::<AppState>();
                        let _guard = state.is_logging_in.lock().await;
                        let _ = execute_login_flow(&app).await;
                    }
                    Some(TriggerAction::ResetBackoff) => {
                        consecutive_business_errors = 0;
                        next_retry_instant = None;
                        last_error_reason.clear();
                    }
                    None => break,
                }
            }
        }
    }
}

// ============= 托盘菜单 =============

fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let show_item = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
    let manual_item = MenuItem::with_id(app, "manual_connect", "手动连接", true, None::<&str>)?;
    let check_item = MenuItem::with_id(app, "check", "立即检测", true, None::<&str>)?;
    let login_item = MenuItem::with_id(app, "login", "执行登录脚本", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_item, &check_item, &manual_item, &login_item, &quit_item])?;
    let icon = app.default_window_icon().cloned();
    let mut tray_builder = TrayIconBuilder::new()
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "manual_connect" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.emit("manual_connect_wifi", ());
                }
            }
            "check" => {
                let state = app.state::<AppState>();
                let _ = state.trigger_tx.try_send(TriggerAction::Check);
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.emit("check_network", ());
                }
            }
            "login" => {
                let state = app.state::<AppState>();
                let _ = state.trigger_tx.try_send(TriggerAction::Login);
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.emit("run_login", ());
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        });
    if let Some(icon) = icon {
        tray_builder = tray_builder.icon(icon);
    }
    tray_builder = tray_builder.tooltip("新乡工程校园网保活");
    tray_builder.build(app)?;
    Ok(())
}

// ============= 主入口 =============

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(windows)]
    {
        // 若以旧名/打包名 xxgcxy-wifi.exe 启动，且同目录下已生成中文程序「新乡工程校园网保活.exe」，则自动无缝交接至中文程序
        if let Ok(current_path) = std::env::current_exe() {
            if let Some(file_name) = current_path.file_name().and_then(|s| s.to_str()) {
                if file_name.eq_ignore_ascii_case("xxgcxy-wifi.exe") {
                    if let Some(parent) = current_path.parent() {
                        let target_exe = parent.join("新乡工程校园网保活.exe");
                        if target_exe.exists() {
                            let _ = std::process::Command::new(target_exe)
                                .args(std::env::args().skip(1))
                                .spawn();
                            return;
                        }
                    }
                }
            }
        }
    }

    if !check_single_instance() {
        return;
    }

    let (trigger_tx, trigger_rx) = tokio::sync::mpsc::channel::<TriggerAction>(32);

    let initial_config = {
        let path = get_config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                serde_json::from_str::<Config>(strip_bom(&content)).unwrap_or_default()
            } else {
                Config::default()
            }
        } else {
            Config::default()
        }
    };
    let is_first_run = initial_config.primary_ssid.is_empty();

    let app_state = AppState {
        config: Mutex::new(initial_config),
        first_run: Mutex::new(is_first_run),
        check_enabled: Mutex::new(true),
        service_status: Mutex::new(ServiceStatus::default()),
        trigger_tx: trigger_tx.clone(),
        is_logging_in: Arc::new(tokio::sync::Mutex::new(())),
        last_manual_login: Mutex::new(std::time::Instant::now() - std::time::Duration::from_secs(10)),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .setup(move |app| {
            // 后端日志落盘: 与 xywdl-*.log 脚本日志同目录 (%APPDATA%\xxgcxy-wifi\logs), 文件名前缀 backend
            // 注意: 生产构建同样注册 (此前 release 完全无后端日志, 故障时无法回溯)
            let log_dir = get_login_dir().join("logs");
            app.handle().plugin(
                tauri_plugin_log::Builder::default()
                    .level(log::LevelFilter::Info)
                    .targets([
                        tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
                        tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Webview),
                        tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Folder {
                            path: log_dir,
                            file_name: Some(String::from("backend")),
                        }),
                    ])
                    .max_file_size(1024 * 1024)
                    .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepAll)
                    .build(),
            )?;

            setup_tray(app.handle())?;

            if let Some(window) = app.get_webview_window("main") {
                let window_clone = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = window_clone.hide();
                    }
                });
            }

            // 启动独立 Rust 后端保活状态机 (tokio::spawn)
            let handle_clone = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                start_background_service(handle_clone, trigger_rx).await;
            });

            // 启动窗口行为:
            // 1. 开机自启动 (--autostart / --minimized / --silent): 绝对静默隐藏于托盘, 绝不弹窗
            // 2. 用户双击手动启动: 显示窗口并聚焦, 方便查看网络状态或配置账号
            let args: Vec<String> = std::env::args().collect();
            let is_autostart = args.iter().any(|arg| {
                arg == "--autostart" || arg == "--minimized" || arg == "--silent" || arg == "-s"
            });

            if let Some(window) = app.get_webview_window("main") {
                if is_autostart {
                    let _ = window.hide();
                } else {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            scan_wifi,
            connect_wifi,
            check_network,
            run_login_script,
            get_check_enabled,
            toggle_check_enabled,
            get_service_status,
            trigger_manual_check,
            trigger_manual_login,
            get_autostart_enabled,
            set_autostart_enabled,
            open_github,
            load_campus_net_info,
            clear_campus_net_info,
            // ===== 移动热点保活 =====
            get_hotspot_keepalive,
            set_hotspot_keepalive,
            check_and_keep_hotspot_alive,
            // ===== 登录模块 (v1.9.0+) =====
            is_login_configured,
            get_login_profile,
            save_login_profile,
            parse_portal_url,
            run_login_with_profile,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// ============= 单元测试 (cargo test --lib) =============

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_portal(
        url: &str,
        expect_ok: bool,
        expected_base: Option<&str>,
        expected_ssid: Option<&str>,
        expected_vlan: Option<&str>,
        expected_mac: Option<&str>,
    ) {
        match parse_portal_url(url.to_string()) {
            Ok(p) => {
                assert!(expect_ok, "URL 应报错但成功解析: {}", url);
                if let Some(b) = expected_base {
                    assert_eq!(p.base_url, b, "base_url 不匹配: {}", url);
                }
                if let Some(s) = expected_ssid {
                    assert_eq!(p.ssid, s, "ssid 不匹配: {}", url);
                }
                if let Some(v) = expected_vlan {
                    assert_eq!(p.vlan, v, "vlan 不匹配: {}", url);
                }
                if let Some(m) = expected_mac {
                    assert_eq!(p.mac_address, m, "mac 不匹配: {}", url);
                }
            }
            Err(e) => {
                assert!(!expect_ok, "URL 应成功解析但报错: {} -> {}", url, e);
            }
        }
    }

    // ---------- 正常/常规 ----------
    #[test]
    fn test_parse_normal() {
        assert_portal(
            "http://172.18.252.12:6060/portal.do?wlanuserip=10.0.0.5&wlanacname=XXGC-AC&wlanacIp=172.18.252.1&vlan=100&mac=aa-bb-cc-dd-ee-ff&ssid=XXGC-WiFi&hostname=PC1&rand=0.123",
            true,
            Some("http://172.18.252.12:6060/portal.do"),
            Some("XXGC-WiFi"),
            Some("100"),
            Some("aa:bb:cc:dd:ee:ff"),
        );
    }

    #[test]
    fn test_parse_https() {
        assert_portal(
            "https://portal.xxgc.edu.cn:8443/portal.do?ssid=ABC&vlan=200",
            true,
            Some("https://portal.xxgc.edu.cn:8443/portal.do"),
            Some("ABC"),
            Some("200"),
            None,
        );
    }

    // ---------- 边界: 空/无端口/大小写 ----------
    #[test]
    fn test_parse_empty_url() {
        assert_portal("", false, None, None, None, None);
        assert_portal("   ", false, None, None, None, None);
    }

    #[test]
    fn test_parse_no_scheme() {
        assert_portal("172.18.252.12/portal.do", false, None, None, None, None);
    }

    #[test]
    fn test_parse_no_path() {
        assert_portal("http://172.18.252.12", false, None, None, None, None);
    }

    #[test]
    fn test_parse_no_do_suffix() {
        assert_portal("http://172.18.252.12/login", false, None, None, None, None);
    }

    #[test]
    fn test_parse_uppercase_do() {
        assert_portal(
            "http://172.18.252.12/PORTAL.DO?ssid=X",
            true,
            Some("http://172.18.252.12/PORTAL.DO"),
            Some("X"),
            None,
            None,
        );
    }

    // ---------- 边界: 编码 ----------
    #[test]
    fn test_parse_utf8_ssid() {
        // %E6%B5%8B%E8%AF%95 = 测试 (UTF-8 多字节)
        assert_portal(
            "http://172.18.252.12/portal.do?ssid=%E6%B5%8B%E8%AF%95WiFi",
            true,
            Some("http://172.18.252.12/portal.do"),
            Some("测试WiFi"),
            None,
            None,
        );
    }

    #[test]
    fn test_parse_plus_as_space() {
        // form-encoded: + 应转空格
        assert_portal(
            "http://172.18.252.12/portal.do?ssid=XXGC+WiFi+5G",
            true,
            Some("http://172.18.252.12/portal.do"),
            Some("XXGC WiFi 5G"),
            None,
            None,
        );
    }

    #[test]
    fn test_parse_special_chars() {
        // 值含 & = 等已被编码的字符
        assert_portal(
            "http://172.18.252.12/portal.do?ssid=A%26B%3DC&vlan=100",
            true,
            Some("http://172.18.252.12/portal.do"),
            Some("A&B=C"),
            Some("100"),
            None,
        );
    }

    // ---------- 边界: 超长/空值/重复 key ----------
    #[test]
    fn test_parse_empty_value() {
        assert_portal(
            "http://172.18.252.12/portal.do?ssid=&vlan=",
            true,
            Some("http://172.18.252.12/portal.do"),
            Some(""),
            Some(""),
            None,
        );
    }

    #[test]
    fn test_parse_duplicate_key_last_wins() {
        assert_portal(
            "http://172.18.252.12/portal.do?ssid=first&ssid=second",
            true,
            Some("http://172.18.252.12/portal.do"),
            Some("second"),
            None,
            None,
        );
    }

    #[test]
    fn test_parse_query_before_do_in_ssid() {
        // SSID 值里含 ? 不应截断 query 解析 (取第一个 ? 之前为 base)
        // 这里 SSID 中编码了 %3F (即 ?), 不应影响 base_url 提取
        assert_portal(
            "http://172.18.252.12/portal.do?ssid=AB%3FCD",
            true,
            Some("http://172.18.252.12/portal.do"),
            Some("AB?CD"),
            None,
            None,
        );
    }

    // ---------- 稳定性: 大量 URL 不 panic ----------
    #[test]
    fn test_parse_stress_no_panic() {
        let samples = [
            "http://a.b/portal.do?a=1".to_string(),
            "http://a.b/portal.do?ssid=%E6%B5%8B".to_string(), // 截断的 UTF-8
            "http://a.b/portal.do?ssid=%ZZ".to_string(),        // 非法 hex
            "file:///etc/passwd".to_string(),
            "http://x/portal.do?a=b&c".to_string(),
        ];
        for s in &samples {
            let _ = parse_portal_url(s.clone());
        }
    }

    #[test]
    fn test_resolve_quickauth_url() {
        assert_eq!(
            resolve_quickauth_url("http://172.18.252.12:6060/portal.do").unwrap(),
            "http://172.18.252.12:6060/quickauth.do"
        );
        assert_eq!(
            resolve_quickauth_url("http://172.18.252.12:6060/quickauth.do").unwrap(),
            "http://172.18.252.12:6060/quickauth.do"
        );
        assert_eq!(
            resolve_quickauth_url("http://172.18.252.12:6060").unwrap(),
            "http://172.18.252.12:6060/quickauth.do"
        );
        assert_eq!(
            resolve_quickauth_url("http://172.18.252.12:6060/").unwrap(),
            "http://172.18.252.12:6060/quickauth.do"
        );
        assert_eq!(
            resolve_quickauth_url("http://172.18.252.12:6060/custom/path/portal.do").unwrap(),
            "http://172.18.252.12:6060/custom/path/quickauth.do"
        );
        assert_eq!(
            resolve_quickauth_url("http://172.18.252.12:6060/portal.do?wlanuserip=1.1.1.1").unwrap(),
            "http://172.18.252.12:6060/quickauth.do"
        );
    }

    #[test]
    fn test_dummy_ip_mac_defense() {
        assert!(is_dummy_ip(""));
        assert!(is_dummy_ip("127.0.0.1"));
        assert!(is_dummy_ip("0.0.0.0"));
        assert!(is_dummy_ip("10.0.0.1"));
        assert!(!is_dummy_ip("10.12.34.56"));
        assert!(!is_dummy_ip("172.18.252.12"));

        assert!(is_dummy_mac(""));
        assert!(is_dummy_mac("00:00:00:00:00:00"));
        assert!(is_dummy_mac("aa:bb:cc:dd:ee:ff"));
        assert!(is_dummy_mac("invalid_mac"));
        assert!(!is_dummy_mac("18:c0:4d:82:11:22"));
    }

    #[test]
    fn test_valid_portal_base_url() {
        // 合法 Portal Base URL
        assert!(is_valid_portal_base_url("http://172.18.252.12:6060/portal.do"));
        assert!(is_valid_portal_base_url("http://10.1.1.1/eportal/index.do"));
        assert!(is_valid_portal_base_url("http://172.18.252.12/quickauth.do"));
        assert!(is_valid_portal_base_url("https://portal.xxgc.edu.cn/portal.do"));

        // 广告/公网劫持/外网探测端点（严防覆盖污染用户配置）
        assert!(!is_valid_portal_base_url("http://connect.rom.miui.com/generate_204"));
        assert!(!is_valid_portal_base_url("http://connectivitycheck.platform.hicloud.com/generate_204"));
        assert!(!is_valid_portal_base_url("http://1.1.1.1"));
        assert!(!is_valid_portal_base_url("http://www.baidu.com"));
        assert!(!is_valid_portal_base_url("http://ad.carrier.com/ads.html"));
        assert!(!is_valid_portal_base_url(""));
        assert!(!is_valid_portal_base_url("ftp://172.18.252.12/portal.do"));
    }

    #[test]
    fn test_decode_console_output_gbk() {
        // 中文 Windows 下 netsh 的 stdout 为 GBK 编码, 必须正确解码 "物理地址" 等中文标签
        let (gbk_bytes, _, _) = encoding_rs::GBK.encode("物理地址       : AA-BB-CC-DD-EE-FF");
        let decoded = decode_console_output(&gbk_bytes);
        assert!(decoded.contains("物理地址"), "GBK 解码后应包含 '物理地址': {}", decoded);
        assert!(decoded.contains("AA-BB-CC-DD-EE-FF"), "GBK 解码后应包含 MAC: {}", decoded);
    }

    #[test]
    fn test_decode_console_output_utf8_passthrough() {
        // 合法 UTF-8 字节 (如系统开启 UTF-8 beta 选项) 应原样直通不被二次转码
        let raw = "IPv4 地址          : 10.12.34.56";
        assert_eq!(decode_console_output(raw.as_bytes()), raw);
    }
}

