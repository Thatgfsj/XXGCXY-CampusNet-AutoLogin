use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
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

    use std::io::{Read, Write};
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
    pub test_hosts: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            primary_ssid: String::new(),
            backup_ssid: String::new(),
            check_interval: 15,
            test_hosts: vec!["baidu.com".to_string(), "qq.com".to_string()],
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

// ============= 全局状态 =============

pub struct AppState {
    pub config: Mutex<Config>,
    pub first_run: Mutex<bool>,
    pub check_enabled: Mutex<bool>,
}

// ============= Tauri 命令 =============

#[tauri::command]
fn get_check_enabled(state: tauri::State<'_, AppState>) -> bool {
    *state.check_enabled.lock().unwrap()
}

#[tauri::command]
fn toggle_check_enabled(state: tauri::State<'_, AppState>) -> bool {
    let mut enabled = state.check_enabled.lock().unwrap();
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
        let mut current_config = state.config.lock().unwrap();
        *current_config = config.clone();
        if !config.primary_ssid.is_empty() {
            let mut first_run = state.first_run.lock().unwrap();
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
    let mut current_config = state.config.lock().unwrap();
    *current_config = config;
    let mut first_run = state.first_run.lock().unwrap();
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
        std::thread::sleep(std::time::Duration::from_secs(2));
        let output = hidden_command("netsh")
            .args(["wlan", "show", "networks", "mode=bssid"])
            .output()
            .map_err(|e| format!("执行扫描命令失败: {}", e))?;
        let stdout = String::from_utf8_lossy(&output.stdout);
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
        let output = Command::new("nmcli")
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
        let name_arg = format!("name={}", ssid);
        let output = hidden_command("netsh")
            .args(["wlan", "connect", &name_arg])
            .output()
            .map_err(|e| format!("执行连接命令失败: {}", e))?;
        if output.status.success() {
            std::thread::sleep(std::time::Duration::from_secs(3));
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Err(format!("连接 WiFi 失败: {}", stderr))
        }
    }

    #[cfg(not(windows))]
    {
        let output = Command::new("nmcli")
            .args(["device", "wifi", "connect", &ssid])
            .output()
            .map_err(|e| format!("执行连接命令失败: {}", e))?;
        if output.status.success() {
            std::thread::sleep(std::time::Duration::from_secs(3));
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
        let output = Command::new("nmcli")
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
    // 使用 Cloudflare 的 generate_204 端点
    // 返回 204 表示已连接互联网，返回 200/重定向表示需要登录
    match check_url("http://cp.cloudflare.com/generate_204").await {
        CheckResult::Connected => return true,
        CheckResult::NeedLogin => return false,
        CheckResult::Error => {}
    }
    // 备用检测
    match check_url("https://www.baidu.com/").await {
        CheckResult::Connected => return true,
        CheckResult::NeedLogin => return false,
        CheckResult::Error => {}
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
                        || (loc_lower.contains("edu") && loc_lower.contains("login"))
                        || (loc_lower.contains("login") && !loc_lower.contains("baidu"))
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
            if let Ok(content) = response.text() {
                let content_lower = content.to_lowercase();
                if content_lower.contains("drcom")
                    || content_lower.contains("inode")
                    || content_lower.contains("eportal")
                    || content_lower.contains("srun")
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
    let config = state.config.lock().unwrap().clone();
    let wifi_connected = get_connected_wifi();
    let internet_ok = check_internet().await;
    let needs_reconnect =
        wifi_connected.is_none()
        || (!config.primary_ssid.is_empty()
            && wifi_connected.as_ref() != Some(&config.primary_ssid));
    let needs_login = wifi_connected.is_some() && !internet_ok;
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
            .command("cmd")
            .args(["/c", "start", "/wait", &script_path.to_string_lossy()])
            .output()
            .await
            .map_err(|e| format!("执行登录脚本失败: {}", e))?;
        if output.status.success() {
            Ok("登录脚本执行成功".to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Err(format!("登录脚本执行失败: {}", stderr))
        }
    }

    #[cfg(not(windows))]
    {
        let shell = app.shell();
        let script_str = script_path.to_string_lossy().to_string();
        let output = shell
            .command("bash")
            .args(["-c", &format!("chmod +x '{}' && '{}'", script_str, script_str)])
            .output()
            .await
            .map_err(|e| format!("执行登录脚本失败: {}", e))?;
        if output.status.success() {
            Ok("登录脚本执行成功".to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            Err(format!("登录脚本执行失败: {} {}", stderr, stdout))
        }
    }
}

// ============= 打开 GitHub =============

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
    let check_item = MenuItem::with_id(app, "check", "立即检测", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_item, &check_item, &quit_item])?;
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
            "check" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.emit("check_network", ());
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

            let config_path = get_config_path();
            if config_path.exists() {
                if let Ok(content) = fs::read_to_string(&config_path) {
                    if let Ok(config) = serde_json::from_str::<Config>(&content) {
                        if !config.primary_ssid.is_empty() {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.hide();
                            }
                        }
                    }
                }
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
