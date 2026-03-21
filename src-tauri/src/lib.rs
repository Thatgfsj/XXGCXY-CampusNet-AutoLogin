use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Emitter, Manager};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::AppHandle;

// 单例检查 - 防止重复启动
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
        
        let result = CreateMutexW(
            None,
            false,
            windows::core::PCWSTR(mutex_name.as_ptr()),
        );
        
        if result.is_err() {
            return false;
        }
        
        // 如果 GetLastError 返回 ERROR_ALREADY_EXISTS，说明已有实例运行
        let err = windows::Win32::Foundation::GetLastError();
        err != ERROR_ALREADY_EXISTS
    }
}

#[cfg(not(windows))]
fn check_single_instance() -> bool {
    true
}

// Windows 上隐藏命令行窗口
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

// 配置结构体
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

// WiFi 网络信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WifiNetwork {
    pub ssid: String,
    pub signal: u8,
    pub secured: bool,
}

// 网络状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStatus {
    pub wifi_connected: Option<String>,
    pub internet_ok: bool,
    pub needs_reconnect: bool,
    pub needs_login: bool,
}

// 全局配置状态
pub struct AppState {
    pub config: Mutex<Config>,
    pub first_run: Mutex<bool>,
    pub check_enabled: Mutex<bool>,
}

// 获取检测开关状态
#[tauri::command]
fn get_check_enabled(state: tauri::State<'_, AppState>) -> bool {
    *state.check_enabled.lock().unwrap()
}

// 切换检测开关
#[tauri::command]
fn toggle_check_enabled(state: tauri::State<'_, AppState>) -> bool {
    let mut enabled = state.check_enabled.lock().unwrap();
    *enabled = !*enabled;
    *enabled
}

// 开机自启动相关
#[cfg(windows)]
const AUTOSTART_REGISTRY_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
#[cfg(windows)]
const AUTOSTART_REGISTRY_VALUE: &str = "CampusWifiHelper";

// 检查是否已设置开机自启动
#[tauri::command]
fn get_autostart_enabled() -> bool {
    #[cfg(windows)]
    {
        use winreg::RegKey;
        use winreg::enums::*;
        
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(key) = hkcu.open_subkey_with_flags(AUTOSTART_REGISTRY_KEY, KEY_READ) {
            if key.get_value::<String, _>(AUTOSTART_REGISTRY_VALUE).is_ok() {
                return true;
            }
        }
        false
    }
    #[cfg(not(windows))]
    {
        false
    }
}

// 设置开机自启动
#[tauri::command]
fn set_autostart_enabled(enabled: bool) -> Result<(), String> {
    #[cfg(windows)]
    {
        use winreg::RegKey;
        use winreg::enums::*;
        
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let key = hkcu
            .create_subkey_with_flags(AUTOSTART_REGISTRY_KEY, KEY_WRITE)
            .map_err(|e| format!("打开注册表失败: {}", e))?
            .0;
        
        if enabled {
            // 获取当前 exe 路径
            let exe_path = std::env::current_exe()
                .map_err(|e| format!("获取程序路径失败: {}", e))?;
            let exe_path_str = exe_path.to_string_lossy().to_string();
            
            // 写入注册表
            key.set_value(AUTOSTART_REGISTRY_VALUE, &exe_path_str)
                .map_err(|e| format!("写入注册表失败: {}", e))?;
        } else {
            // 删除注册表项
            let _ = key.delete_value(AUTOSTART_REGISTRY_VALUE);
        }
        
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = enabled;
        Err("仅支持 Windows 系统".to_string())
    }
}

// 获取配置文件路径
fn get_config_path() -> PathBuf {
    let exe_dir = std::env::current_exe()
        .map(|p| p.parent().unwrap_or(std::path::Path::new(".")).to_path_buf())
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    exe_dir.join("config.json")
}

// 加载配置
#[tauri::command]
fn load_config(state: tauri::State<'_, AppState>) -> Result<Config, String> {
    let config_path = get_config_path();
    
    if config_path.exists() {
        let content = fs::read_to_string(&config_path)
            .map_err(|e| format!("读取配置文件失败: {}", e))?;
        let config: Config = serde_json::from_str(&content)
            .map_err(|e| format!("解析配置文件失败: {}", e))?;
        
        let mut current_config = state.config.lock().unwrap();
        *current_config = config.clone();
        
        // 如果有配置，标记为非首次运行
        if !config.primary_ssid.is_empty() {
            let mut first_run = state.first_run.lock().unwrap();
            *first_run = false;
        }
        
        Ok(config)
    } else {
        Ok(Config::default())
    }
}

// 保存配置
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

// 扫描 WiFi 网络（使用 netsh 命令）
#[tauri::command]
async fn scan_wifi() -> Result<Vec<WifiNetwork>, String> {
    // 先刷新网络列表
    let _ = hidden_command("netsh")
        .args(["wlan", "scan"])
        .output();
    
    // 等待扫描完成
    std::thread::sleep(std::time::Duration::from_secs(2));
    
    // 获取网络列表
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
        
        // 解析 SSID
        if line.starts_with("SSID") && line.contains(':') {
            // 保存上一个网络
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
        }
        // 解析信号强度
        else if line.starts_with("信号") || line.starts_with("Signal") {
            if line.contains(':') {
                let parts: Vec<&str> = line.splitn(2, ':').collect();
                if parts.len() > 1 {
                    let signal_str = parts[1].trim();
                    // 解析百分比，如 "80%"
                    if let Ok(pct) = signal_str.replace('%', "").trim().parse::<u8>() {
                        current_signal = pct;
                    }
                }
            }
        }
        // 解析加密状态
        else if line.starts_with("身份验证") || line.starts_with("Authentication") {
            if line.contains("开放") || line.contains("Open") {
                current_secured = false;
            } else {
                current_secured = true;
            }
        }
    }
    
    // 保存最后一个网络
    if !current_ssid.is_empty() {
        networks.push(WifiNetwork {
            ssid: current_ssid,
            signal: current_signal,
            secured: current_secured,
        });
    }
    
    // 去重
    let mut seen = std::collections::HashSet::new();
    networks.retain(|n| {
        if seen.contains(&n.ssid) {
            false
        } else {
            seen.insert(n.ssid.clone());
            true
        }
    });
    
    // 按信号强度排序
    networks.sort_by(|a, b| b.signal.cmp(&a.signal));
    
    Ok(networks)
}

// 连接 WiFi（使用 netsh 命令）
#[tauri::command]
async fn connect_wifi(ssid: String) -> Result<(), String> {
    // 构建 name 参数
    let name_arg = format!("name={}", ssid);
    
    // 使用 netsh wlan connect 命令连接
    let output = hidden_command("netsh")
        .args(["wlan", "connect", &name_arg])
        .output()
        .map_err(|e| format!("执行连接命令失败: {}", e))?;
    
    if output.status.success() {
        // 等待连接完成
        std::thread::sleep(std::time::Duration::from_secs(3));
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(format!("连接 WiFi 失败: {}", stderr))
    }
}

// 检测网络状态
#[tauri::command]
async fn check_network(state: tauri::State<'_, AppState>) -> Result<NetworkStatus, String> {
    let config = state.config.lock().unwrap().clone();
    
    // 获取当前连接的 WiFi
    let wifi_connected = get_connected_wifi();
    
    // 检测互联网连接
    let internet_ok = check_internet().await;
    
    // 判断是否需要重连
    let needs_reconnect = wifi_connected.is_none() || 
        (!config.primary_ssid.is_empty() && wifi_connected.as_ref() != Some(&config.primary_ssid));
    
    // 判断是否需要登录（WiFi 已连接但无法上网）
    let needs_login = wifi_connected.is_some() && !internet_ok;
    
    Ok(NetworkStatus {
        wifi_connected,
        internet_ok,
        needs_reconnect,
        needs_login,
    })
}

// 获取当前连接的 WiFi（使用 netsh 命令）
fn get_connected_wifi() -> Option<String> {
    let output = hidden_command("netsh")
        .args(["wlan", "show", "interfaces"])
        .output()
        .ok()?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    for line in stdout.lines() {
        let line = line.trim();
        // 解析 SSID 行（中文和英文系统）
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

// 检测互联网连接（通过 HTTP 检测是否被重定向到登录页）
async fn check_internet() -> bool {
    // 优先检测 example.com（简单可靠）
    match check_url("https://example.com/").await {
        CheckResult::Connected => return true,
        CheckResult::NeedLogin => return false,
        CheckResult::Error => {}
    }
    
    // 再检测小米的 204 响应
    match check_url("http://connect.rom.miui.com/generate_204").await {
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
    // 使用阻塞 reqwest 在单独线程中执行
    let url = url.to_string();
    let result = tokio::task::spawn_blocking(move || {
        let client = match reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(3))
            .redirect(reqwest::redirect::Policy::none()) // 不自动跟随重定向
            .build() {
                Ok(c) => c,
                Err(_) => return CheckResult::Error,
            };
        
        let response = match client.get(&url).send() {
            Ok(r) => r,
            Err(_) => return CheckResult::Error,
        };
        
        // 检查状态码
        let status = response.status();
        
        // 如果是 302/301 重定向 - 这是校园网未登录的典型特征
        if status.is_redirection() {
            if let Some(location) = response.headers().get("location") {
                if let Ok(loc_str) = location.to_str() {
                    // 检查重定向地址是否包含校园网认证相关关键词
                    let loc_lower = loc_str.to_lowercase();
                    // 校园网认证系统常见特征
                    if loc_lower.contains("portal")
                        || loc_lower.contains("drcom")
                        || loc_lower.contains("inode")
                        || loc_lower.contains("eportal")
                        || loc_lower.contains("srun")
                        || loc_lower.contains("authserv")
                        || loc_lower.contains("1x")
                        || (loc_lower.contains("edu") && loc_lower.contains("login"))
                        || (loc_lower.contains("login") && !loc_lower.contains("baidu")) {
                        return CheckResult::NeedLogin;
                    }
                }
            }
            // 其他重定向（如百度跳转）视为正常
            return CheckResult::Connected;
        }
        
        // 状态码 204（小米检测）- 正常联网
        if status.as_u16() == 204 {
            return CheckResult::Connected;
        }
        
        // 状态码 200
        if status.is_success() {
            // 检查内容是否包含校园网登录页面特征（排除正常网站的登录按钮）
            if let Ok(content) = response.text() {
                let content_lower = content.to_lowercase();
                // 只检测校园网认证系统特有的关键词，不检测通用的"登录"
                if content_lower.contains("drcom")
                    || content_lower.contains("inode")
                    || content_lower.contains("eportal")
                    || content_lower.contains("srun")
                    || content_lower.contains("portal认证")
                    || content_lower.contains("校园网认证")
                    || content_lower.contains("校园网登录") {
                    return CheckResult::NeedLogin;
                }
                // 检查是否是正常的百度首页（包含百度特有内容）
                if content_lower.contains("百度一下")
                    || content_lower.contains("baidu")
                    || content_lower.contains("百度") {
                    return CheckResult::Connected;
                }
            }
            return CheckResult::Connected;
        }
        
        CheckResult::Error
    }).await;
    
    match result {
        Ok(r) => r,
        Err(_) => CheckResult::Error,
    }
}

// 运行登录脚本
#[tauri::command]
async fn run_login_script(app: AppHandle) -> Result<String, String> {
    use tauri_plugin_shell::ShellExt;
    
    // 获取 exe 目录
    let exe_dir = std::env::current_exe()
        .map(|p| p.parent().unwrap_or(std::path::Path::new(".")).to_path_buf())
        .unwrap_or_else(|_| PathBuf::from("."));
    
    // 尝试多个可能的脚本路径
    let possible_paths: Vec<PathBuf> = vec![
        // 1. exe 同目录
        exe_dir.join("xywdl.ps1"),
        // 2. exe 目录下的 _up_ 子目录（MSI 安装后的位置）
        exe_dir.join("_up_").join("xywdl.ps1"),
        // 3. 资源目录
        app.path().resource_dir().map(|p| p.join("xywdl.ps1")).unwrap_or_default(),
        // 4. 当前工作目录
        std::env::current_dir().map(|p| p.join("xywdl.ps1")).unwrap_or_default(),
    ];
    
    // 查找存在的脚本路径
    let script_path = possible_paths
        .into_iter()
        .find(|p| p.exists())
        .ok_or_else(|| format!("登录脚本不存在 (exe目录: {})", exe_dir.display()))?;
    
    let script_path_str = script_path.to_string_lossy().to_string();
    
    // 使用可见的 PowerShell 窗口执行脚本（用户可能需要输入账号密码）
    // start 命令会打开新窗口，/wait 等待脚本完成
    let shell = app.shell();
    let output = shell
        .command("cmd")
        .args([
            "/c",
            "start",
            "/wait",
            "powershell",
            "-NoExit",           // 脚本执行完后不关闭窗口，让用户看到输出
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            &script_path_str
        ])
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

// 打开 GitHub 页面
#[tauri::command]
async fn open_github(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_shell::ShellExt;
    
    let shell = app.shell();
    shell
        .open("https://github.com/Thatgfsj/XXGCXY-CampusNet-AutoLogin", None)
        .map_err(|e| format!("打开链接失败: {}", e))?;
    
    Ok(())
}

// 创建托盘菜单
fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let show_item = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
    let check_item = MenuItem::with_id(app, "check", "立即检测", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    
    let menu = Menu::with_items(app, &[&show_item, &check_item, &quit_item])?;
    
    // 获取窗口图标
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 单例检查
    if !check_single_instance() {
        // 已有实例运行，直接退出
        return;
    }
    
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // 初始化日志（仅在调试模式）
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            
            // 设置托盘
            setup_tray(app.handle())?;
            
            // 窗口关闭时隐藏而不是退出
            if let Some(window) = app.get_webview_window("main") {
                let window_clone = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        // 阻止默认关闭行为
                        api.prevent_close();
                        // 隐藏窗口
                        let _ = window_clone.hide();
                    }
                });
            }
            
            // 检查是否已有配置
            let config_path = get_config_path();
            if config_path.exists() {
                // 尝试加载配置，检查是否已配置主网络
                if let Ok(content) = fs::read_to_string(&config_path) {
                    if let Ok(config) = serde_json::from_str::<Config>(&content) {
                        if !config.primary_ssid.is_empty() {
                            // 已有有效配置，隐藏主窗口，后台运行
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
