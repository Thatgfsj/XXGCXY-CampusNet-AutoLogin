use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
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
}

impl Default for Config {
    fn default() -> Self {
        Config {
            primary_ssid: String::new(),
            backup_ssid: String::new(),
            check_interval: 15,
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
    pub base_url: String,       // 完整 portal.do URL,如 http://172.18.252.12:6060/portal.do
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
    let json: serde_json::Value = serde_json::from_str(&content)
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
            match serde_json::from_str::<LoginProfile>(&content) {
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
    serde_json::from_str(&content)
        .map_err(|e| format!("解析登录配置失败: {}", e))
}

/// 保存登录配置 + 加密密码
#[tauri::command]
fn save_login_profile(profile: LoginProfile, password: String) -> Result<(), String> {
    if profile.user_id.is_empty() {
        return Err("学号/账号不能为空".to_string());
    }
    if profile.base_url.is_empty() {
        return Err("Portal URL 不能为空".to_string());
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
        let lower = decoded.to_lowercase();
        if let Some(idx) = lower.find("://") {
            let after_scheme = &decoded[idx + 3..];
            // 找到第一个 /,然后到 .do 结尾
            if let Some(slash_idx) = after_scheme.find('/') {
                let host_and_path = &after_scheme[slash_idx..];
                // 找 ".do" 结尾
                if let Some(do_idx) = host_and_path.to_lowercase().find(".do") {
                    let end = do_idx + 3;
                    after_scheme[..slash_idx + end].to_string()
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

/// 用已保存的 profile 直接执行登录 (与 run_login_script 等价)
#[tauri::command]
async fn run_login_with_profile(app: AppHandle) -> Result<String, String> {
    if !is_login_configured() {
        return Err("尚未配置校园网账号,请先在主页或网络配置页填写".to_string());
    }
    // 直接复用现有的 run_login_script 内部逻辑
    run_login_script(app).await
}

// ============= DPAPI 密码加密 =============
//
// PS 端 ConvertFrom-SecureString 默认产出 UTF-16 LE 编码的 DPAPI 字节流,
// 用 `ConvertTo-SecureString -String <blob> | ...` 可还原为 SecureString。
//
// 为了让 PS 端能直接读我们写入的 .bin,我们用同样的字节布局:
//   [0..4]   = "DPAPI" magic (4 bytes ASCII)
//   [4..]    = CryptProtectData 输出
//
// PS 端读取后:
//   $blob = [System.IO.File]::ReadAllBytes($credPath)
//   $b64  = [Convert]::ToBase64String($blob)
//   $sec  = ConvertTo-SecureString -String $b64 -Key $([Byte[]](1..16))
//
// (因为我们走的是 DPAPI 而不是 AES,PS 端不应该用 -Key;改成:
//   $sec  = ConvertTo-SecureString -String $b64
// )
//
// 我们的实现:CryptProtectData → 直接写裸字节 → PS 端用 ConvertTo-SecureString 读。

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

/// operator 短码 -> 中文名 (用于 load_campus_net_info 渲染)
fn operator_short_to_name(code: &str) -> &'static str {
    match code {
        "yd" => "移动",
        "lt" => "联通",
        "dx" => "电信",
        _ => "未知",
    }
}

// ============= 全局状态 =============

pub struct AppState {
    pub config: Mutex<Config>,
    pub first_run: Mutex<bool>,
    pub check_enabled: Mutex<bool>,
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
            return key.get_value::<String, _>("CampusWifiHelper").is_ok();
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
        if enabled {
            let exe_path = std::env::current_exe()
                .map_err(|e| format!("获取程序路径失败: {}", e))?
                .to_string_lossy()
                .to_string();
            key.set_value("CampusWifiHelper", &exe_path)
                .map_err(|e| format!("写入注册表失败: {}", e))?;
        } else {
            let _ = key.delete_value("CampusWifiHelper");
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
                "[Desktop Entry]\nType=Application\nName=xxgcxy-wifi\nExec={}\nHidden=false\nX-GNOME-Autostart-enabled=true\n",
                exe_path
            );
            fs::write(&desktop_path, desktop_content)
                .map_err(|e| format!("写入启动文件失败: {}", e))?;
        } else {
            let _ = fs::remove_file(&desktop_path);
        }
        Ok(())
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
            serde_json::from_str(&content).map_err(|e| format!("解析配置文件失败: {}", e))?;
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
        let stdout = String::from_utf8_lossy(&output.stdout);

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
        let guard = TempFileGuard(profile_path.clone());
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
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let line = line.trim();
            if line.starts_with("SSID") && line.contains(':') {
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

// ============= 检测互联网连接 =============

async fn check_internet() -> bool {
    // 优先使用 HTTP 检测（避免 HTTPS 绕过 captive portal）
    match check_url("http://connect.rom.miui.com/generate_204").await {
        CheckResult::Connected => return true,
        CheckResult::NeedLogin => return false,
        CheckResult::Error => {}
    }
    match check_url("http://httpstat.us/204").await {
        CheckResult::Connected => return true,
        CheckResult::NeedLogin => return false,
        CheckResult::Error => {}
    }
    false
}

// ============= 带重试的互联网检测 =============

async fn check_internet_with_retry() -> bool {
    for i in 0..3 {
        if check_internet().await {
            return true;
        }
        if i < 2 {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
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
            .timeout(std::time::Duration::from_secs(3))
            .redirect(reqwest::redirect::Policy::none())
            .build()
        {
            Ok(c) => c,
            Err(_) => return CheckResult::Error,
        };
        let response = match client.get(&url).send() {
            Ok(r) => r,
            Err(_) => return CheckResult::Error,
        };
        let status = response.status();
        if status.is_redirection() {
            if let Some(location) = response.headers().get("location") {
                if let Ok(loc_str) = location.to_str() {
                    let loc_lower = loc_str.to_lowercase();
                    if loc_lower.contains("portal")
                        || loc_lower.contains("drcom")
                        || loc_lower.contains("inode")
                        || loc_lower.contains("eportal")
                        || loc_lower.contains("srun")
                        || loc_lower.contains("authserv")
                        || loc_lower.contains("1x")
                        || loc_lower.contains("wlanuserip")
                        || loc_lower.contains("ntdks")
                        || (loc_lower.contains("edu") && loc_lower.contains("login"))
                        || (loc_lower.contains("login") && (loc_lower.contains("auth") || loc_lower.contains("portal") || loc_lower.contains("redirect")))
                    {
                        return CheckResult::NeedLogin;
                    }
                }
            }
            return CheckResult::Connected;
        }
        if status.as_u16() == 204 {
            return CheckResult::Connected;
        }
        if status.is_success() {
            use std::io::Read;
            let mut limited = response.take(8192);
            let mut body = String::new();
            if limited.read_to_string(&mut body).is_ok() {
                let content_lower = body.to_lowercase();
                if content_lower.contains("drcom")
                    || content_lower.contains("inode")
                    || content_lower.contains("eportal")
                    || content_lower.contains("srun")
                    || content_lower.contains("wlanuserip")
                    || content_lower.contains("portal认证")
                    || content_lower.contains("校园网认证")
                    || content_lower.contains("校园网登录")
                {
                    return CheckResult::NeedLogin;
                }
                if content_lower.contains("百度一下")
                    || content_lower.contains("baidu")
                    || content_lower.contains("百度")
                {
                    return CheckResult::Connected;
                }
            }
            return CheckResult::Connected;
        }
        CheckResult::Error
    })
    .await;

    match result {
        Ok(r) => r,
        Err(_) => CheckResult::Error,
    }
}

// ============= 检测网络状态 =============

#[tauri::command]
async fn check_network(state: tauri::State<'_, AppState>) -> Result<NetworkStatus, String> {
    let config = state.config.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let wifi_connected = get_connected_wifi();
    let internet_ok = check_internet_with_retry().await;
    let needs_reconnect = wifi_connected.is_none()
        || (!config.primary_ssid.is_empty()
            && wifi_connected.as_ref() != Some(&config.primary_ssid)
            && wifi_connected.as_ref() != Some(&config.backup_ssid));
    let needs_login = wifi_connected.is_some()
        && !internet_ok
        && !config.primary_ssid.is_empty()
        && (wifi_connected.as_ref() == Some(&config.primary_ssid)
            || wifi_connected.as_ref() == Some(&config.backup_ssid));
    Ok(NetworkStatus {
        wifi_connected,
        internet_ok,
        needs_reconnect,
        needs_login,
    })
}

// ============= 运行登录脚本 =============

#[tauri::command]
async fn run_login_script(app: AppHandle) -> Result<String, String> {
    use tauri_plugin_shell::ShellExt;

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
        let shell = app.shell();
        let output = shell
            .command(script_path.to_string_lossy().as_ref())
            .arg("--non-interactive")
            .output()
            .await
            .map_err(|e| format!("执行登录脚本失败: {}", e))?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        if output.status.success() {
            // 把脚本 stdout 一并返回, 方便 UI 日志看到发送层/响应细节
            let detail = if stdout.trim().is_empty() {
                "登录脚本执行成功".to_string()
            } else {
                format!("登录脚本执行成功: {}", stdout.trim())
            };
            Ok(detail)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let combined = format!("{}{}", stderr, stdout);
            Err(format!("登录脚本执行失败: {}", combined.trim()))
        }
    }

    #[cfg(not(windows))]
    {
        let shell = app.shell();
        let script_str = script_path.to_string_lossy().to_string();
        let output = shell
            .command("bash")
            .args(["-c", &format!("chmod +x '{}' && '{}' --non-interactive", script_str, script_str)])
            .output()
            .await
            .map_err(|e| format!("执行登录脚本失败: {}", e))?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        if output.status.success() {
            let detail = if stdout.trim().is_empty() {
                "登录脚本执行成功".to_string()
            } else {
                format!("登录脚本执行成功: {}", stdout.trim())
            };
            Ok(detail)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let combined = format!("{}{}", stderr, stdout);
            Err(format!("登录脚本执行失败: {}", combined.trim()))
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
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.emit("check_network", ());
                }
            }
            "login" => {
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
    tray_builder.build(app)?;
    Ok(())
}

// ============= 主入口 =============

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if !check_single_instance() {
        return;
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

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

            // 启动窗口行为 (用户优先级要求): 有登录信息 → 隐藏到托盘; 没登录信息 → 显示
            // 用 is_login_configured 检查 (login_profile.json + login_credential.bin 同时存在)
            // 不再用 config.primary_ssid — v1.9.0 拆分 WiFi 配置和登录账号, WiFi 配了不等于登录账号配了
            let login_configured = is_login_configured();
            let should_hide = login_configured;
            if let Some(window) = app.get_webview_window("main") {
                if should_hide {
                    let _ = window.hide();
                }
                // 不 hide: 窗口默认显示, 前端 checkFirstRun 会检测未配置状态
                // 然后 setTimeout 500ms 后弹登录配置屏提示用户填写
            }

            Ok(())
        })
        .manage(AppState {
            config: Mutex::new(Config::default()),
            first_run: Mutex::new(true),
            check_enabled: Mutex::new(true),
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
            get_autostart_enabled,
            set_autostart_enabled,
            open_github,
            load_campus_net_info,
            clear_campus_net_info,
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
            "http://a.b/portal.do?x=100%25".to_string(),
            "http://a.b/portal.do?".to_owned() + &"y=".repeat(5000), // 超长 query
            "file:///etc/passwd".to_string(),
            "http://x/portal.do?a=b&c".to_string(),
        ];
        for s in &samples {
            let _ = parse_portal_url(s.clone());
        }
    }
}
