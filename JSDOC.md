# XXGCXY-CampusNet-AutoLogin — 完整技术文档

## 1. 项目概述

| 属性 | 值 |
|------|-----|
| **项目名称** | 校园网自动登录助手 (CampusNet Auto Login) |
| **用途** | 新乡工程学院校园网 Portal 认证自动登录 + WiFi 自动重连 |
| **仓库地址** | https://github.com/Thatgfsj/XXGCXY-CampusNet-AutoLogin |
| **作者** | Thatgfsj |
| **许可证** | MIT |
| **当前版本** | 2.2.0 |
| **目标用户** | 新乡工程学院校园网用户 |
| **主要平台** | Windows 10/11（主）、Linux（辅） |

---

## 2. 技术栈

### 2.1 前端（桌面 UI）

| 技术 | 版本 | 用途 |
|------|------|------|
| HTML5 | — | 单页应用界面结构 |
| CSS3 | — | 渐变主题样式，自定义开关、列表组件 |
| JavaScript (ES Module) | ES6+ | 前端业务逻辑、事件处理、定时器管理 |
| @tauri-apps/api | ^2.10.1 | Tauri IPC 桥接（invoke / listen） |

### 2.2 后端（桌面壳）

| 技术 | 版本 | 用途 |
|------|------|------|
| Rust | 1.77.2+ (edition 2021) | 核心后端逻辑 |
| Tauri | 2.10.3 | 跨平台桌面框架 |
| tokio | 1 (full) | 异步运行时 |
| reqwest | 0.11 (blocking) | HTTP 客户端（阻塞模式，用于连通性检测） |
| serde / serde_json | 1.0 | JSON 序列化/反序列化 |
| dirs | 5 | 跨平台用户目录获取 |
| windows (crate) | 0.58 | Win32 API 绑定（单例互斥体） |
| winreg | 0.52 | Windows 注册表操作（开机自启） |
| tauri-plugin-shell | 2 | 启动外部脚本 + 打开系统浏览器 |
| tauri-plugin-log | 2 | 调试日志输出 |

### 2.3 认证脚本

| 技术 | 说明 |
|------|------|
| PowerShell 5.1+ / 7.x | 核心校园网认证脚本（`xywdl.ps1`） |
| Batch | Windows 启动器，自动查找 PS7 并回退到 PS5（`xywdl.bat`） |
| Bash | Linux 纯 Shell 版（`xywdl.sh`），含 curl + python3 辅助解析 |

### 2.4 CI/CD

| 技术 | 说明 |
|------|------|
| GitHub Actions | tag 推送时自动构建 Linux .deb 包 |
| ubuntu-24.04 | 构建环境 |
| softprops/action-gh-release | 自动上传发布包 |

---

## 3. 目录结构

```
XXGCXY-CampusNet-AutoLogin/
├── .git/                          # Git 仓库
├── .gitignore                     # 忽略 node_modules, dist, target, 临时文件
├── .github/
│   └── workflows/
│       ├── build-all.yml          # 多平台构建 + 发布
│       └── build-linux.yml        # Linux .deb 构建工作流（仅 tag 触发）
│
├── index.html                     # 前端单页应用（~1580行），含完整 CSS + JS
├── package.json                   # Node.js 项目配置 (campus-wifi, 2.0.2)
├── package-lock.json              # 依赖锁定文件
│
├── xywdl.ps1                      # ★ 核心认证脚本（~500行，函数式，含三层降级发送）
├── xywdl.bat                      # Windows 启动器（找系统 pwsh / powershell）
├── xywdl.sh                       # Linux 启动脚本（~330行，纯 Bash，含两层降级发送）
│
├── src/sender/                    # 请求发送保底层 (v2.0.0+)
│   ├── sender.cs                  # C# 源码
│   ├── sender.py                  # Python 源码（纯标准库，跨平台）
│   └── xywdl_sender.exe           # 预编译 C# 发送器 (~5KB, .NET Framework 4.x)
│
├── README.md                      # 项目说明（功能、安装、构建）
├── SPEC.md                        # 功能规范文档（13条验收标准）
├── AUTH_MECHANISM.md              # ★ 认证机制详解（Portal 协议、DPAPI 加密、204检测）
├── JSDOC.md                       # ★ 本文档
│
├── create_icon.ps1                # 图标生成脚本
│
├── tests/                         # 自动化测试套件 (v1.9.1+)
│   ├── mock_portal.py             # 校园网认证 Mock 服务器
│   ├── run_ps1_tests.ps1          # xywdl.ps1 端到端 + 边界测试（23 项）
│   └── test_sh_judge.sh           # xywdl.sh 认证判定逻辑测试（11 项）
│
└── src-tauri/                     # Tauri 后端（Rust）
    ├── Cargo.toml                 # Rust 包配置 (app, 2.0.2)
    ├── Cargo.lock                 # 依赖锁定
    ├── build.rs                   # 构建脚本（复制 WebView2Loader.dll）
    ├── tauri.conf.json            # Tauri 配置（窗口、打包、NSIS、插件权限）
    ├── .gitignore                 # 后端 .gitignore
    │
    ├── capabilities/
    │   └── default.json           # Tauri 权限配置（core:default）
    │
    ├── icons/                     # 应用图标（多尺寸 PNG + ICO + ICNS）
    │
    ├── nsis/
    │   └── installer.nsi          # NSIS 安装器钩子脚本
    │
    ├── bin/
    │   └── WebView2Loader.dll     # WebView2 运行时加载器
    │
    ├── .cargo/                    # 自定义链接器配置（Windows 便携构建用）
    │   ├── config.toml            # Cargo 构建配置
    │   ├── lld-wrapper.bat        # LLD 链接器包装
    │   ├── lld_wrapper.py         # LLD 包装 Python 脚本
    │   ├── gen_vcruntime_lib.py   # VCRuntime 导入库生成
    │   ├── fix_tls_sections.py    # TLS 段修复脚本
    │   ├── run_winapi_build.py    # WinAPI 构建脚本
    │   ├── build_coff.py          # COFF 目标文件构建
    │   ├── msvcrt*.def/lib        # MSVCRT 存根库
    │   ├── crt_stubs*             # CRT 存根（静态链接用）
    │   ├── crt_stubs_src/         # CRT 存根 Rust 源码
    │   │   ├── lib.rs
    │   │   ├── full_stub.rs
    │   │   ├── no_tls.rs
    │   │   ├── no_weak_main.rs
    │   │   ├── exit_early.rs
    │   │   ├── stubs.s
    │   │   ├── temp_lib.rs
    │   │   └── Cargo.toml
    │   └── tls_test/              # TLS 测试文件（大量 .exe/.obj/.rs）
    │
    └── src/
        ├── main.rs                # 程序入口（~7行，隐藏控制台）
        └── lib.rs                 # ★ 核心后端（~901行，全部业务逻辑）
```

---

## 4. 核心架构

### 4.1 整体数据流

```
┌─────────────────────────────────────────────────────────┐
│                    前端 (index.html)                      │
│  ┌──────────┐  ┌──────────┐  ┌──────────────┐           │
│  │ 状态面板  │  │ 配置面板  │  │ 日志面板      │           │
│  └────┬─────┘  └────┬─────┘  └──────┬───────┘           │
│       │              │               │                    │
│       └──────────────┼───────────────┘                    │
│                      │ Tauri IPC (invoke/listen)          │
├──────────────────────┼────────────────────────────────────┤
│                      ▼                                     │
│              Rust 后端 (lib.rs)                            │
│  ┌──────────────────────────────────────────────────┐     │
│  │  Tauri Commands                                  │     │
│  │  scan_wifi / connect_wifi / check_network        │     │
│  │  load_config / save_config                       │     │
│  │  run_login_script / get_autostart / set_autostart│     │
│  │  toggle_check_enabled / open_github               │     │
│  ├──────────────────────────────────────────────────┤     │
│  │  系统托盘 (setup_tray)                            │     │
│  │  单例检查 (check_single_instance)                 │     │
│  │  开机自启 (注册表/desktop entry)                  │     │
│  └──────────────────────┬───────────────────────────┘     │
│                         │ shell execute                     │
│                         ▼                                   │
│              ┌──────────────────┐                          │
│              │  外部脚本调用     │                          │
│              │  cmd /c xywdl.bat│                          │
│              │  --non-interactive│                         │
│              └────────┬─────────┘                          │
│                       │                                     │
├───────────────────────┼─────────────────────────────────────┤
│                       ▼                                     │
│           PowerShell 脚本 (xywdl.ps1)                       │
│  ┌──────────────────────────────────────────────────┐     │
│  │  ╔══════════════════════════════════════════════╗ │     │
│  │  ║  PowerShell 类层次                           ║ │     │
│  │  ╠══════════════════════════════════════════════╣ │     │
│  │  ║  NetworkConfig        — 请求参数模型         ║ │     │
│  │  ║  DomainConfig         — 运营商后缀映射       ║ │     │
│  │  ║  ConfigManager        — DPAPI 加密配置读写   ║ │     │
│  │  ║  NetworkInterfaceHelper — WiFi 网卡操作      ║ │     │
│  │  ║  RedirectUrlParser     — 重定向 URL 解析     ║ │     │
│  │  ║  AuthenticationClient  — 认证主流程编排      ║ │     │
│  │  ╚══════════════════════════════════════════════╝ │     │
│  │                                                    │     │
│  │  工作流程：                                         │     │
│  │  1. 自动检测 → 捕获 302 重定向 URL                 │     │
│  │  2. 解析 Portal 参数 (IP/MAC/VLAN/AC)              │     │
│  │  3. 用户输入凭证 → DPAPI 加密存储                  │     │
│  │  4. 构造 GET 请求 → quickauth.do → 认证            │     │
│  └──────────────────────────────────────────────────┘     │
└──────────────────────────────────────────────────────────┘
```

### 4.2 两层架构说明

项目采用**两层分离**架构：

- **Rust 层（lib.rs）**：负责 WiFi 扫描/连接、网络连通性检测、配置管理、系统托盘、自启动、单例管理、进程调度
- **PowerShell 层（xywdl.ps1）**：专责校园网 Portal 认证——自动检测参数、用户凭证管理、HTTP 构造与发送、结果解析

这两层通过 `run_login_script` 命令衔接——Rust 后端通过 `tauri-plugin-shell` 调用 `cmd /c xywdl.bat --non-interactive`，由 bat 查找合适的 PowerShell 引擎并执行认证脚本。

### 4.3 三端构建策略

| 构建变体 | 分支 | 特点 |
|----------|------|------|
| **Windows 便携版（内置 PS7）** | `win-portable` | 开箱即用，打包了 PowerShell 7 便携版（`bin/_pw7_/`），无需用户安装任何运行时 |
| **Windows 系统 PS7 版** | `win-system-ps7` | 需要系统已安装 PowerShell 7，体积更小 |
| **Linux 版** | `linux-sh` | 纯 Shell 脚本，需系统安装 `pwsh` |

---

## 5. 主要模块详解

### 5.1 Rust 后端 (`src-tauri/src/lib.rs`)

**文件行数**：901 行  
**核心职责**：WiFi 管理、网络监控、进程调度、系统集成

#### 5.1.1 单例检查 (`check_single_instance`)

- **Windows**：使用 `CreateMutexW` 创建全局命名互斥体 `Global\CampusWifiHelper_SingleInstance`，如果返回 `ERROR_ALREADY_EXISTS` 则退出，保证仅运行一个实例
- **Linux**：使用文件锁 `~/.local/share/xxgcxy-wifi/single_instance.lock`，写入 PID，进程退出时不主动清理（依赖检测逻辑）

#### 5.1.2 WiFi 扫描 (`scan_wifi`)

- **Windows**：`netsh wlan scan` 触发扫描 → `netsh wlan show networks mode=bssid` 解析输出 → 按 SSID: / 信号: / 身份验证: 分行解析 → 去重 → 按信号强度降序排列
- **Linux**：`nmcli -t -m multiline device wifi list --rescan yes` → 按 SSID: / SIGNAL: / SECURITY: 分行解析 → 去重 → 降序排列

返回 `Vec<WifiNetwork>`：`{ ssid: String, signal: u8, secured: bool }`

#### 5.1.3 WiFi 连接 (`connect_wifi`)

- **Windows**：
  1. `netsh wlan disconnect` 断开当前连接
  2. `netsh wlan connect name={ssid}` 直接连接（使用已有配置）
  3. 如果失败（无已保存配置），动态生成 WiFi 配置文件 XML（开放网络，无加密）
  4. `netsh wlan add profile filename={tmp_xml}` 导入配置
  5. 再次 `netsh wlan connect name={ssid}`
  6. 清理临时 XML 文件
- **Linux**：`nmcli device wifi connect {ssid}`

#### 5.1.4 网络连通性检测 (`check_url` / `check_internet`)

三层递进判断机制：

| 层次 | 检测目标 | 判断逻辑 |
|:----:|----------|----------|
| 1 | HTTP 302 重定向 | `Location` 头含 `portal/drcom/inode/eportal/srun/authserv/wlanuserip/ntdks` → **未认证** |
| 2 | HTTP 204 No Content | 收到 204 → **已连通**（AC 不会返回 204，这是铁证） |
| 3 | HTTP 200 正文 | 正文含 `drcom/inode/eportal/srun/portal认证/校园网认证` → **未认证**；含 `百度一下/baidu` → **已连通** |

检测流程：
```
check_internet_with_retry()  ← 最多重试 3 次
  └─ check_internet()
       ├─ check_url("http://connect.rom.miui.com/generate_204")
       │    ├─ Connected → return true
       │    └─ NeedLogin/Error → 继续
       └─ check_url("http://httpstat.us/204")
            ├─ Connected → return true
            └─ _ → return false
```

关键实现细节：
- 使用 `reqwest::blocking::Client` + `redirect::Policy::none()` 禁止自动跟随重定向
- 设置 `no_proxy()` 避免系统代理干扰
- 超时 3 秒，防止卡死
- 使用 `tokio::task::spawn_blocking` 将阻塞操作放入线程池

#### 5.1.5 网络状态综合判断 (`check_network`)

```rust
needs_reconnect = wifi_connected.is_none()                  // WiFi 完全断开
    || (!config.primary_ssid.is_empty()
        && wifi_connected != Some(primary_ssid)              // 连的不是主网络
        && wifi_connected != Some(backup_ssid));             // 连的也不是备用网络

needs_login = wifi_connected.is_some()                       // WiFi 已连接
    && !internet_ok                                            // 但无法上网
    && (wifi_connected == Some(primary_ssid)                  // 且连的是配置的网络
        || wifi_connected == Some(backup_ssid));
```

#### 5.1.6 登录脚本执行 (`run_login_script`)

- 依次在 4 个可能路径中查找 `xywdl.bat`：
  1. EXE 同级目录
  2. `_up_/` 子目录（Tauri 更新目录）
  3. Tauri resource_dir
  4. 当前工作目录
- Windows：`cmd /c "path/to/xywdl.bat" --non-interactive`
- Linux：`bash -c "chmod +x 'path/to/xywdl.sh' && 'path/to/xywdl.sh'"`

#### 5.1.7 开机自启动 (`set_autostart_enabled`)

- **Windows**：写入/删除注册表 `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\CampusWifiHelper`
- **Linux**：写入/删除 `~/.config/autostart/xxgcxy-wifi.desktop`（Desktop Entry 格式）

#### 5.1.8 系统托盘 (`setup_tray`)

菜单项：
菜单项：
| ID | 显示文本 | 功能 |
|----|----------|------|
| `show` | 显示窗口 | 显示并聚焦主窗口 |
| `check` | 立即检测 | 触发 `check_network` 事件（全自动检测→重连→登录） |
| `manual_connect` | 手动连接 | 触发 `manual_connect_wifi` 事件（连接 WiFi，不自动登录） |
| `login` | 执行登录脚本 | 触发 `run_login` 事件（绕过检查，直接执行） |
| `quit` | 退出 | `app.exit(0)` |

关闭窗口行为：拦截 `CloseRequested` 事件，阻止关闭，改为隐藏到托盘。

#### 5.1.9 配置管理

- **存储路径**：`~/.local/share/xxgcxy-wifi/config.json`（Linux）或 `%LOCALAPPDATA%/xxgcxy-wifi/config.json`（Windows）
- **结构体**：`{ primary_ssid, backup_ssid, check_interval, hotspot_keepalive }`
- **加载时机**：程序启动时、setup 阶段（如果已有配置则隐藏窗口）
- **保存时机**：用户在配置面板点击"保存配置"或切换热点常开开关
- **首次运行判断**：`primary_ssid` 为空 → 首次运行 → 显示主窗口

#### 5.1.10 Tauri Commands 清单

| Command | 参数 | 返回 | 说明 |
|---------|------|------|------|
| `load_config` | — | `Result<Config>` | 从磁盘加载配置 |
| `save_config` | `config: Config` | `Result<()>` | 保存配置到磁盘 |
| `scan_wifi` | — | `Result<Vec<WifiNetwork>>` | 扫描可用 WiFi |
| `connect_wifi` | `ssid: String` | `Result<()>` | 连接指定 WiFi |
| `check_network` | — | `Result<NetworkStatus>` | 综合网络状态检测 |
| `run_login_script` | — | `Result<String>` | 执行登录脚本(兼容旧版本) |
| `get_check_enabled` | — | `bool` | 获取自动检测开关状态 |
| `toggle_check_enabled` | — | `bool` | 切换自动检测开关 |
| `get_autostart_enabled` | — | `bool` | 获取开机自启状态 |
| `set_autostart_enabled` | `enabled: bool` | `Result<()>` | 设置开机自启 |
| `get_hotspot_keepalive` | — | `bool` | **(v2.0.8+)** 获取保持移动热点常开状态 |
| `set_hotspot_keepalive` | `enabled: bool` | `Result<bool>` | **(v2.0.8+)** 设置保持移动热点常开并激活 |
| `check_and_keep_hotspot_alive` | — | `Result<String>` | **(v2.0.8+)** 检查热点状态若关闭则拉起唤醒 |
| `open_github` | — | `Result<()>` | 打开 GitHub 仓库 |
| `load_campus_net_info` | — | `Result<CampusNetInfo>` | 读取校园网配置(学号/运营商) |
| `clear_campus_net_info` | — | `Result<()>` | 删除校园网配置 + 旧文件清理 |
| `is_login_configured` | — | `bool` | **(v1.9.0+)** 是否已配置校园网账号 |
| `get_login_profile` | — | `Result<LoginProfile>` | **(v1.9.0+)** 读取登录配置(不含密码) |
| `save_login_profile` | `profile: LoginProfile, password: String` | `Result<()>` | **(v1.9.0+)** 保存登录配置 + DPAPI 加密密码 |
| `parse_portal_url` | `url: String` | `Result<ParsedPortal>` | **(v1.9.0+)** 解析 portal.do 重定向 URL |
| `run_login_with_profile` | — | `Result<String>` | **(v1.9.0+)** 用已保存的 profile 执行登录 |

**登录模块数据结构 (v1.9.0+)**:

```rust
pub struct LoginProfile {
    pub user_id: String,        // "2021110101@xxgcyd"
    pub operator: String,       // "yd" | "lt" | "dx"
    pub ssid: String,
    pub base_url: String,       // 完整 portal.do URL
    pub wlan_ac_name: String,
    pub wlan_ac_ip: String,
    pub vlan: String,
    pub wlan_user_ip: String,   // 留空时 PS 端运行时取
    pub mac_address: String,
    pub portal_page_id: String, // 默认 "3"
    pub portal_type: String,    // 默认 "0"
    pub version: String,        // 默认 "0"
    pub bind_ctrl_id: String,   // 默认 ""
    pub hostname: String,       // 留空时 PS 用 $env:COMPUTERNAME
    pub updated_at: String,     // ISO8601
}

pub struct ParsedPortal {
    pub base_url: String,
    pub wlan_ac_name: String,
    pub wlan_ac_ip: String,
    pub wlan_user_ip: String,
    pub vlan: String,
    pub mac_address: String,
    pub ssid: String,
    pub hostname: String,
    pub rand: String,
}
```

**登录模块文件位置**:
- Windows: `%APPDATA%\xxgcxy-wifi\login_profile.json` + `login_credential.bin`
- Linux:   `~/.config/xxgcxy-wifi/login_profile.json` + `login_credential.bin`

---

### 5.2 前端界面 (`index.html`)

**文件行数**：910 行（单文件，CSS + HTML + JS 内联）

#### 5.2.1 界面结构 (v2.0.8+ 深色极客毛玻璃规范)

三个屏幕，通过 `.hidden` 类切换显示：

**主界面（mainScreen）**：
- **顶栏 Header**：微光 Logo + 标题 + 版本胶囊 (`v2.0.9`) + 磨砂亚克力【⚙️ 设置】按钮
- **核心状态仪表盘（Hero Gauge）**：140px SVG 动态发光网络健康雷达环，在线/待登录/掉线/检测中状态呼吸变色与雷达扫描
- **网络信息面板**：当前 WiFi（`#currentWifi`）与检测周期卡片
- **学生身份数字名片（Profile Card）**：科技感头像、学号展示、中国移动/联通/电信色彩微光徽章与【✏️ 更改】快捷抽屉入口
- **智能开关组（Toggles Group）**：
  1. 🔔 自动保活与断线自愈 (`#autoCheckToggle`)
  2. 🚀 开机后台静默自启 (`#autostartToggle`)
  3. 🔥 保持移动热点开启 (`#hotspotKeepaliveToggle`, v2.0.8+) — 实时守护热点防超时关闭
- **快捷操作按钮**：【🔄 立即检测并重连】（`#checkNowBtn`）
- **可折叠活动时间轴（Activity Terminal）**：等宽字体终端输出，支持 `[+]` 绿标、`[!]` 红标、`[*]` 蓝标语义高亮与自由收折

**登录配置界面（loginConfigScreen, v1.9.0+ / v2.0.8 优化）**:
- 运营商选择下拉框 (移动/联通/电信)
- 学生学号输入框 (纯数字校验，脱敏示例 `2024010101`)
- 校园网登录密码输入框 (密码采用 Windows DPAPI 加密存储)
- Portal 认证地址输入框 + 【自动提纯】一体化胶囊按钮 (粘贴 portal.do 自动抽取网络参数并剥离多余 query)
- 高级字段折叠手风琴 (AC 名称 / VLAN / IP / MAC / 主机名)
- 【稍后 / 取消】与【💾 保存配置】主按钮 (v2.0.8 移除了多余的“保存并登录”按钮)

**设置界面（setupScreen）**：
- 顶部分组:"📡 WiFi 网络"
- WiFi 列表（带信号强度、发光选中框、可点击选择主/备用网络）
- 已选主网络 / 备用网络显示
- 检测间隔输入（5-300 秒）
- 校园网信息卡片(学号 / 运营商徽章)
- "更改账号信息" 按钮(跳转 loginConfigScreen)
- 清理校园网信息按钮(带二次确认)
- 保存 / 返回按钮

#### 5.2.2 核心状态机

```
网络检测 → 状态判断:
  ├─ internet_ok → 显示 ✅ 正常
  ├─ wifi_connected + !internet_ok → 显示 🔐 需要登录 → 触发登录
  └─ !wifi_connected → 显示 ❌ 断开 → 触发重连
```

#### 5.2.3 重连逻辑 (`reconnectWifi`)

信号强度感知的智能选择：
1. 扫描所有 WiFi → 获取主/备用网络信号强度
2. 如果主网络信号 < 40% **且** 备用信号 > 主网络 → 优先连接备用
3. 连接第一个 → 等待 4 秒 → 检测网络
4. 第一个没连上 → 尝试第二个
5. 连接成功后检查是否需要登录

#### 5.2.4 防重复登录

- `isLoggingIn` 锁：登录中时跳过检测
- `lastLoginTime`：5 秒内不重复登录
- 登录失败后 5 秒自动重试一次

#### 5.2.5 定时检测

- 基于 `setInterval`，间隔由 `config.check_interval` 决定（默认 15 秒）
- 受 `autoCheckEnabled` 开关控制
- 保存配置时重建定时器

---

### 5.3 PowerShell 认证脚本 (`xywdl.ps1`)

**文件行数**：~280 行（v1.9.0+ 大幅简化）
**架构**：函数式 + 数据驱动（不再面向对象）

#### 5.3.1 架构变化 (v1.9.0+)

v1.9.0 重写了整个脚本，从"硬编码 + 类"改为"读 JSON 配置 + 函数式"：
- 移除 `NetworkConfig` / `DomainConfig` / `ConfigManager` / `RedirectUrlParser` / `AuthenticationClient` 5 个类
- 移除交互式 `Read-Host` 输入账号密码
- 移除自动检测 portal 重定向（`TryAutoDetectParams`）和手动粘贴 URL 引导
- 改为 `Load-LoginProfile` + `Get-LoginPassword` + `Invoke-CampusLogin` 三个核心函数
- 配置来源：`%APPDATA%/xxgcxy-wifi/login_profile.json` + `login_credential.bin`

#### 5.3.2 主流程

```
Load-LoginProfile       读取 login_profile.json
        ↓
Get-LoginPassword       从 login_credential.bin 解密 (DPAPI, CurrentUser scope)
        ↓
Invoke-CampusLogin      构造 quickauth.do URL → Invoke-WebRequest → 判断响应
        ↓
exit $code              --non-interactive 模式直接退出,交互模式 Read-Host 暂停
```

#### 5.3.3 配置读取

`Load-LoginProfile` 必需字段：
- `user_id` —— "学号@xxgcyd/xxgclt/xxgcdx"
- `operator` —— "yd" / "lt" / "dx"
- `base_url` —— 完整 portal.do URL
- `vlan` —— VLAN ID
- `mac_address` —— MAC 地址(留空时 PS 端运行时取)

`wlan_user_ip` 是可选的,运行时由 `Get-WifiIpAddress()` 自动获取。

#### 5.3.4 认证请求构造

```
目标端点：{BaseURL → /quickauth.do}  (regex: /\w+\.do → /quickauth.do)
请求方法：GET
参数传递：Query String（约15个参数）
```

- `-Proxy $null` 必须,避免系统代理干扰
- `-UseBasicParsing` 提高 PS 5 兼容性
- `-TimeoutSec 15` 防止卡死

#### 5.3.5 密码解密 (DPAPI)

```
文件格式: [b"DPAPI" 4字节 magic] [u32 LE 长度] [CryptProtectData 输出]
加密端: Rust 端 CryptProtectData (无 entropy, dwFlags=0, CurrentUser scope)
解密端: .NET ProtectedData.Unprotect(protected, $null, CurrentUser)
```

- 仅 Windows 同用户可解密（DPAPI master key 与用户绑定）
- 密码仅在使用时短暂存在于内存（普通 String,无 SecureString）
- Linux 平台暂用明文（后续可换 libsecret / keyring）

#### 5.3.6 认证结果判断

| 响应特征 | 判断 |
|----------|------|
| `"code":0` / `success` / `认证成功` | 通过 (exit 0) |
| `"code":1` / `账号不存在` | 失败：账号不存在 (exit 1) |
| `"code":44` / `非法接入` | 失败：非法接入 (exit 44) |
| 其他 | 未知 (exit 99) |

---

### 5.4 启动器脚本

#### 5.4.1 Windows (`xywdl.bat`)

查找策略（v1.9.0+ 不再内置 PS7，直接用系统引擎）：
1. 系统 PATH 中的 `pwsh`（PowerShell 7）
2. 回退到 `powershell`（Windows PowerShell 5.1）

执行方式：`pwsh/powershell -ExecutionPolicy Bypass -File "xywdl.ps1" [args]`

#### 5.4.2 Linux (`xywdl.sh`)

- 配置路径：`~/.config/xxgcxy-wifi/login_config.json`
- 登录请求发送两层降级（v2.0.0+）：curl（默认）→ python3 `src/sender/sender.py`
- 使用 `python3 -c "import urllib.parse"` 做 URL 编解码
- 支持 `--non-interactive` 非交互模式（由 Tauri 调用时使用）

---

### 5.5 构建系统

#### 5.5.1 构建流程

```
1. git clone + git lfs pull（拉取 PS7 便携版二进制）
2. npm ci（安装前端依赖）
3. npm run build（Vite 打包 → dist/）
4. npx tauri build（Tauri 编译 Rust + 打包）
```

#### 5.5.2 便携构建特有配置 (`.cargo/`)

`src-tauri/.cargo/` 目录包含大量自定义链接配置，用于实现**纯静态链接的 Windows 便携版**：

- `config.toml`：指定 LLD 链接器，配置静态链接 CRT
- `crt_stubs_src/`：CRT 存根 Rust 源码，替代 MSVC 运行时
- `msvcrt_stubs.def` / `msvcrt_full.def`：定义需要存根的 CRT 导出函数
- `fix_tls_sections.py`：处理 TLS（线程本地存储）段以兼容 Windows PE 格式
- `lld_wrapper.py`：LLD 链接器包装，自动处理 CRT 存根注入
- `crt_stubs.def` / `crt_stubs.lib`：预编译的 CRT 存根库

目标是让最终 EXE **不依赖 msvcrt.dll**，实现真正的单文件便携。

#### 5.5.3 build.rs

核心功能：将 `bin/WebView2Loader.dll` 复制到 `target/{profile}/` 目录，确保 Tauri 打包时包含。

#### 5.5.4 CI/CD（仅 Linux）

- **触发条件**：推送 `v*` 标签或手动触发
- **构建环境**：ubuntu-24.04
- **步骤**：checkout → 安装 Rust/Node.js → apt 安装 WebKit2GTK/GTK3 等依赖 → Vite 构建前端 → Tauri 构建 deb → 上传 artifacts + 发布到 GitHub Release

> Windows 构建在本地执行，不上 CI。

---

### 5.6 Tauri 配置 (`tauri.conf.json`)

| 配置项 | 值 | 说明 |
|--------|-----|------|
| productName | `xxgcxy-wifi` | 产品名 |
| version | 1.8.3 | 版本号 |
| identifier | `com.xxgcxy.wifi` | 应用标识 |
| 窗口尺寸 | 500×750 | 可调整大小、居中 |
| 打包目标 | nsis + msi + deb | Windows NSIS/MSI 安装包 + Linux deb |
| 资源文件 | xywdl.ps1, xywdl.bat, xywdl.sh, WebView2Loader.dll | 打包进安装包 |
| WebView2 安装模式 | `embedBootstrapper` | 内嵌引导程序 |
| NSIS 语言 | SimpChinese | 简体中文安装界面 |
| NSIS 安装模式 | `currentUser` | 当前用户安装（无需管理员） |
| CSP | null | 禁用内容安全策略 |

---

## 6. 数据存储

### 6.1 Tauri 应用配置

```
%LOCALAPPDATA%/xxgcxy-wifi/config.json    (Windows)
~/.local/share/xxgcxy-wifi/config.json     (Linux)
```

```json
{
  "primary_ssid": "XXGCXY-Student",
  "backup_ssid": "",
  "check_interval": 15
}
```

### 6.2 认证脚本配置

```
%APPDATA%/xxgc_campus_net_config.txt       (Windows)
~/.config/xxgcxy-wifi/login_config.json    (Linux)
```

```json
{
  "BaseURL": "http://172.16.x.x:6060/portal.do",
  "WlanAcName": "CampusNet-AC-01",
  "WlanAcIp": "172.16.0.1",
  "Ssid": "XXGCXY-Student",
  "Version": "0",
  "PortalPageId": "3",
  "PortalType": "0",
  "Hostname": "MYPC",
  "BindCtrlId": "",
  "UserId": "20210101001@xxgcyd",
  "EncryptedPassword": "AQAAANCMnd8BFdER...",
  "Vlan": "1050",
  "WlanUserIp": "10.10.50.100",
  "MacAddress": "aa:bb:cc:dd:ee:ff"
}
```

---

## 7. 安全设计

| 层面 | 机制 | 说明 |
|------|------|------|
| 密码存储 | Windows DPAPI | 加密密钥由用户凭据 + 机器硬件派生，不可跨用户/跨机器解密 |
| 密码传输 | URL Encode | 密码经过百分号编码后通过 HTTP GET 传递（无 HTTPS，校园网内网） |
| 内存安全 | `FreeBSTR` | 明文密码使用后立即从内存释放 |
| 文件隐藏 | Hidden Attribute | 配置文件设为隐藏属性 |
| 单实例 | Mutex (Win) / File Lock (Linux) | 防止多个实例同时运行导致重复登录 |
| 防重放 | UUID + Timestamp | 每次请求生成新的 GUID 和毫秒级时间戳 |

---

## 8. 认证协议详解

### 8.1 时序

```
客户端 → AC → DHCP 分配 IP
客户端 → AC → HTTP 请求被劫持
AC → 客户端 → 302 重定向到 Portal 页面（URL 含 wlanuserip/mac/vlan 等参数）
客户端 → Portal Server → GET quickauth.do?{全部参数}
Portal Server → RADIUS → 验证凭证
RADIUS → Portal Server → 验证结果
Portal Server → 客户端 → JSON 响应（code:0 表示成功）
AC → 放行该 IP/MAC → 客户端可以上网
```

### 8.2 运营商后缀

| 编号 | 运营商 | 后缀 |
|:----:|--------|------|
| 1 | 移动 | `@xxgcyd` |
| 2 | 联通 | `@xxgclt` |
| 3 | 电信 | `@xxgcdx` |

### 8.3 连通性检测（Captive Portal Detection）

业界标准做法：请求一个**响应特征已知且独特**的 URL。

| 系统 | 探测 URL | 预期特征 |
|------|----------|----------|
| Android | `connectivitycheck.gstatic.com/generate_204` | HTTP 204 |
| Apple | `captive.apple.com/hotspot-detect.html` | HTTP 200 + `Success` |
| Windows | `www.msftconnecttest.com/connecttest.txt` | HTTP 200 + `Microsoft Connect Test` |
| **本项目** | `connect.rom.miui.com/generate_204` | **HTTP 204** |

核心原理：AC 劫持 HTTP 请求时只会返回 HTTP 200（Portal 页面）或 HTTP 302（重定向），**AC 绝对不会返回 204**。因此，收到 204 = 请求确实到达了外网真实服务器 = 设备已通过认证。这是无法伪造的信号。

---
## 9. 版本历史

- **v2.0.3**：登录脚本添加详细步骤日志，定位卡点更清晰。
  - **登录流程步骤化**：每个阶段添加 `[步骤 X/5]` 标记，失败时明确显示 `[!] 卡在: 步骤 N`。
  - **三层发送详细日志**：每层添加 `[第 N 层]` 标记，成功时显示响应长度，失败时显示具体错误原因（HTTP 状态码、exit code、异常信息）。
  - **Python 候选探测日志**：显示每个 python 候选的查找结果（找到/未找到/探测失败）。
  - **认证结果未知时显示完整响应体**：便于排查 portal 返回的未知 code。
  - **Linux 版同步添加步骤日志**：与 PS 端一致的步骤标记和卡点提示。
  - **版本号升到 2.0.3**（package.json / Cargo.toml / Cargo.lock / tauri.conf.json / build-all.yml / JSDOC / README）。

- **v2.0.2**：修复 SSID 留空导致无法登录。
  - **修复登录无法使用（根因）**：`xywdl.ps1` 的 `Load-LoginProfile` 把 `ssid` 列为必填字段，但 UI 允许（且高级字段引导）SSID 留空。留空时脚本直接报"缺少字段: ssid"并 `exit 2`，登录请求根本没发出去。修法：将 `ssid` 移出必填列表，运行时由 `Get-CurrentSsid()` 自动取当前连接的 WiFi SSID 兜底；未连接任何 WiFi 时仍为空则提示用户填写。
  - **版本号升到 2.0.2**（package.json / Cargo.toml / tauri.conf.json / build-all.yml / JSDOC / README）。

- **v2.0.1**：修复桌面端登录脚本卡死 + 超时优化 + 日志回显。
  - **修复 bat 失败路径永久挂死（根因）**：`xywdl.bat` 在 PowerShell 脚本非零退出时无条件执行 `pause`。桌面端调用带 `--non-interactive`，无控制台时 `pause` 永久挂起——表现为 UI 日志"正在执行登录脚本..."后 20 多秒无回显、任务堆积。修法：bat 检测到 `non-interactive` 参数时跳过所有 `pause`，失败直接返回退出码。
  - **发送超时统一改为 30 秒**：PowerShell `Invoke-WebRequest` 的 `TimeoutSec`、C# sender 的 `Timeout/ReadWriteTimeout`、Python sender 的 `timeout`、curl 的 `--max-time` 全部从 15 秒调整为 30 秒，避免校园网环境下请求未及时返回被过早中断。
  - **登录脚本实际输出回显到 UI 日志**：`run_login_script`（Rust 端）现在把脚本的 stdout 一并返回给前端，UI 日志能直接看到发送层、响应体、各层错误，定位问题不再靠猜。
  - 版本号升到 2.0.1（package.json / Cargo.toml / Cargo.lock / tauri.conf.json / build-all.yml / JSDOC）。

- **v2.0.0**：修复登录核心 bug + 请求发送多层级降级 + 死代码清理。
  - **修复登录无法使用（根因）**：`xywdl.ps1` 的 `Load-LoginProfile` 把 `mac_address` 列为必填字段，但 UI 允许（且引导）MAC 留空。留空时脚本直接报"缺少字段: mac_address"并 `exit 2`，登录请求根本没发出去——表现为 UI 日志"正在执行登录..."后无任何回显。修法：将 `mac_address` / `wlan_user_ip` 移出必填列表，运行时由 `Get-WirelessMacAddress()` / `Get-WifiIpAddress()` 自动取本地值兜底（`xywdl.sh` 同步修复）。
  - **请求发送三层降级（Windows）**：PowerShell `Invoke-WebRequest`（默认）→ C# `src/sender/xywdl_sender.exe`（.NET Framework 4.x，5KB）→ Python `src/sender/sender.py`（纯标准库）。前一层失败自动尝试下一层，三层全失败才报错。
  - **请求发送两层降级（Linux）**：curl（默认）→ python3 `sender.py`。
  - **跨平台最强保底**：`sender.py` 只用标准库 `urllib`，Windows/Linux/macOS 通用，Python 3.6+ 即可。
  - **密码安全传递**：完整 URL 经 stdin 管道传给 C#/Python 保底层，避免明文密码出现在进程命令行。
  - **修复 PS 管道双 BOM 问题**：`[Console]::OutputEncoding` 与 `$OutputEncoding` 都设 UTF8 时，PowerShell 5.1 管道给原生进程传字符串会叠加两个 BOM（`efbbbfefbbbf`），导致 C#/Python sender 收到 `\uFEFF\uFEFFhttp://...` 报"URI 方案无效"。修法：`$OutputEncoding` 改用无 BOM 的 `UTF8Encoding($false)`，且两个 sender 都剥掉开头所有 BOM。
  - **修复 Python 保底层被 WindowsApps 占位 stub 坑**：`Get-Command python3` 可能解析到商店占位 stub（运行返回 9009）。修法：逐个候选 `py -3` / `python` / `python3` 做"能真正运行"探测，跳过假 python。
  - **修复 Portal URL 解析按钮连点报错**：`parsePortalUrl` 加防重复点击锁（`portalParsing` + 按钮禁用），避免并发 invoke 报错。
  - **死代码清理**：前端 `test_hosts` 死配置；Rust `get_wifi_signal` / `clear_login_profile` 两个未被前端调用的命令；Cargo 的 `ping` 死依赖。
  - **版本号同步**：package.json / Cargo.toml / Cargo.lock / tauri.conf.json / build-all.yml 统一升到 2.0.0（Cargo.lock 此前停在 1.9.0 未同步）。

- **v1.9.1**：修复稳定性/边界测试发现的 3 个真实问题 + 新增自动化测试套件。
  - **修复认证结果判定正则误匹配**（PS + sh 两端）：`"code":1` 会误匹配 `"code":10/100/123`（误报“账号不存在”），`"code":44` 会误匹配 `"code":440`（误报“非法接入”）。已加 `(?!\d)`（PS）与 `([^0-9]|$)`（grep）锚定，只有 code 精确等于 0/1/44 才命中。
  - **修复请求日志明文泄漏密码**：`xywdl.ps1` 打印请求 URL 时 `passwd=` 现在脱敏为 `***`，不再泄漏密码到控制台/日志。
  - **统一 PS 端参数 URL 编码**：`ssid/vlan/mac/wlanuserip/wlanacIp/version/portalpageid/portaltype/bindCtrlId` 之前未编码（SSID 含空格/`&`/`=`/中文会破坏 URL），现全部走 `Safe-UriEscape`，与 Linux `xywdl.sh` 的 `urlencode` 行为一致。
  - **新增自动化测试套件**（`tests/`）：
    - `tests/mock_portal.py` — 校园网认证 Mock 服务器（可配置返回 code）。
    - `tests/run_ps1_tests.ps1` — xywdl.ps1 端到端 + 边界测试（认证判定/缺失损坏配置/参数编码/密码脱敏/稳定性共 23 项）。
    - `tests/test_sh_judge.sh` — xywdl.sh 认证判定逻辑测试（11 项）。
  - **新增 Rust 单元测试**：`parse_portal_url` 的 11 个用例（正常/HTTPS/空/大小写/UTF-8/特殊字符/重复 key/超长 query/非法 hex 等）。
  - 版本号升至 1.9.1（package.json / Cargo.toml / tauri.conf.json）。

- **v1.9.0**：登录模块彻底解耦。重大变更：
  - **登录配置从硬编码改为 JSON 模板 + 渲染器模式**。`%APPDATA%/xxgcxy-wifi/login_profile.json` 存元数据(学号/运营商/SSID/Portal URL/AC/VLAN/MAC 等),`login_credential.bin` 存 DPAPI 加密的密码。
  - **xywdl.ps1 大幅简化**:从 604 行的 6 个类改为 ~280 行的函数式脚本,移除交互式 Read-Host、移除自动检测 portal 重定向(`TryAutoDetectParams`)、移除手动粘贴 URL 引导。
  - **新增登录配置屏**(`#loginConfigScreen`):运营商下拉 + 学号 + 密码 + Portal URL(带解析按钮)+ 高级字段折叠(SSID/AC/VLAN/MAC/主机名)。首次启动强制弹窗,提供"稍后"按钮可跳过。
  - **主页 + 设置页加入口按钮**:主页不再有独立的"登录（更换）"按钮,所有账号管理通过"设置"页(原"网络配置"页 v1.9.0+ 改名为"设置")→ 校园网信息卡片 → "更改账号信息"按钮。
  - **新增 6 个 Tauri 命令**:`is_login_configured` / `get_login_profile` / `save_login_profile` / `clear_login_profile` / `parse_portal_url` / `run_login_with_profile`。
  - **DPAPI 跨进程密码保护**:Rust 端用 `CryptProtectData` 加密 UTF-16 字节(无 entropy),PS 端用 `ProtectedData.Unprotect($null, CurrentUser)` 解密。文件格式 `b"DPAPI" + u32 长度 + 密文`。
  - **不兼容旧的 `xxgc_campus_net_config.txt`**,首次启动会引导用户重新配置;`clear_campus_net_info` 同时清理新旧文件。
  - 版本号升至 1.9.0(package.json / Cargo.toml / tauri.conf.json)。

- **v1.8.3**：修复校园网配置读取路径(同时支持 xywdl.ps1 的 APPDATA 路径与 xywdl.sh 的 `~/.config/xxgcxy-wifi/login_config.json` 路径),Windows 上 Git Bash 用户也能识别;xywdl.bat 顶部加 `chcp 65001 >nul` 切换到 UTF-8 代码页,xywdl.bat / xywdl.ps1 加 UTF-8 BOM 兼容 PowerShell 5。
- **v1.8.2**：在"网络配置"窗口展示校园网信息(学号/运营商),并提供"清理校园网信息"按钮一键删除登录配置。新增 Tauri 命令 `load_campus_net_info` / `clear_campus_net_info`,后端从 `%APPDATA%/xxgc_campus_net_config.txt` 读取 `UserId` 字段并按 `@` 拆分为学号 + 运营商后缀(移动/联通/电信)。
- **v1.8.1**：修复 bat 引号崩溃、重排托盘菜单、手动连接改为仅连 WiFi、执行登录脚本改为直接运行、Linux 缺少 --non-interactive 修复、清理代码警告、完善技术文档。
- **v1.7.11**：内置 PS7 支持、NSIS 安装器、跨平台构建、`_pw7_` 资源路径修正、便携构建验证步骤优化。

---

## 10. 开发指南

### 10.1 环境要求

- Rust 1.77.2+
- Node.js 18+
- Windows 10/11 或 Ubuntu 24.04
- .NET Framework 4.x（仅构建 C# sender 时需要，Windows 自带）

### 10.2 本地运行

```bash
git clone https://github.com/Thatgfsj/XXGCXY-CampusNet-AutoLogin.git
cd XXGCXY-CampusNet-AutoLogin
git lfs pull
npm ci
npm run tauri dev     # 开发模式（热重载）
```

### 10.3 构建发布包

```bash
npm run tauri build   # 生产构建，在 src-tauri/target/release/bundle/ 输出安装包
```

### 10.4 分支工作流

```
main ← PR ← win-portable / win-system-ps7 / linux-sh
```

三个分支各自维护构建变体，最终合并到 main。

---

## 11. 关键代码索引

| 功能模块 | 文件 | 函数/类 | 行号 |
|----------|------|---------|------|
| 单例检查 | `lib.rs` | `check_single_instance` | 14-52 |
| 配置加载/保存 | `lib.rs` | `load_config` / `save_config` | 220-251 |
| WiFi 扫描 | `lib.rs` | `scan_wifi` | 256-378 |
| WiFi 连接 | `lib.rs` | `connect_wifi` | 396-492 |
| 连通性检测 | `lib.rs` | `check_url` / `check_internet` | 553-664 |
| 登录脚本调用 | `lib.rs` | `run_login_script` | 693-773 |
| 校园网配置读取 | `lib.rs` | `load_campus_net_info` / `get_campus_config_candidates` / `get_campus_config_path` | 128-220 |
| 校园网配置清理 | `lib.rs` | `clear_campus_net_info` | 222-227 |
| 校园网信息展示 | `index.html` | `loadCampusNetInfo` / `clearCampusInfo` | 904-936 |
| 系统托盘 | `lib.rs` | `setup_tray` | 788-831 |
| 应用入口 | `lib.rs` | `run` | 836-900 |
| 前端状态机 | `index.html` | `checkNetwork` / `reconnectWifi` | 525-690 |
| 手动连接WiFi | `index.html` | `manualWifiConnect` | 704-773 |
| 重定向解析 | `xywdl.ps1` | `[RedirectUrlParser]::ParseRedirectUrl` | 237-279 |
| 自动检测参数 | `xywdl.ps1` | `[AuthenticationClient].TryAutoDetectParams` | 375-433 |
| DPAPI 密码存储 | `xywdl.ps1` | `[ConfigManager].SaveConfig` / `.LoadConfig` | 67-148 |
| 认证请求执行 | `xywdl.ps1` | `[AuthenticationClient].PerformAuthentication` | 519-588 |

---

## 12. 问题与修复历史(Issue & Fix Log)

本节按时间顺序记录开发与发布过程中遇到的真实问题、根因分析、修复方案,以及从中得到的可复用经验。供后续维护者参考,避免重复踩坑。

### 12.1 v1.8.2 初次发布:Windows CI 编译失败 — `Permissions::from_mode` not found

**问题现象**

推送 v1.8.2 tag 后,`build-all.yml` 的两个 Windows 作业(`build-win-system` / `build-win-portable`)在 "Build Tauri" 步骤失败;Linux / standalone-sh 作业成功。

**完整错误日志**

```
error[E0599]: no associated function or constant named `from_mode` found
              for struct `Permissions` in the current scope
   --> src\lib.rs:198:61
    |
198 |         let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o666));
    |                                                             ^^^^^^^^^^
    |                                                             associated function or
    |                                                             constant not found in `Permissions`

error: could not compile `app` (lib) due to 1 previous error; 1 warning emitted
failed to build app: failed to build app
```

**根因分析**

v1.8.2 新增的 `clear_campus_net_info` 命令中,为了"清除 Windows Hidden 文件属性以避免 `fs::remove_file` 失败",我写了一段看似合理的代码:

```rust
#[cfg(windows)]
{
    let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o666));
}
```

这里有两个错位:

1. **API 错位**:`fs::Permissions::from_mode` 是 `std::os::unix::fs::PermissionsExt` 的扩展方法,**仅在 Unix 平台存在**。Windows 编译时 `Permissions` 结构体根本没有 `from_mode` 这个关联函数。
2. **概念错位**:Windows 的 Hidden 不是 POSIX 权限位,而是 Win32 `FILE_ATTRIBUTE_HIDDEN`(由 `GetFileAttributesW` / `SetFileAttributesW` 控制)。Rust 标准库的 `std::fs::set_permissions` 只覆盖 POSIX 权限位,根本触及不到 FILE_ATTRIBUTE。**在 Windows 上,这行代码既会编译报错,就算想绕过也做不了它声称要做的事。**

我自己的 `#[cfg(windows)]` 条件也是反的——应该是 `#[cfg(unix)]`,却写成了 `#[cfg(windows)]`,直接导致 Windows 编译时选中了这行不存在的 API。

**修复方案**

直接把这段去掉。原因:`fs::remove_file` 在 Windows 上**本身就能删除带 `FILE_ATTRIBUTE_HIDDEN` 的文件**,Hidden 属性只是控制资源管理器默认是否显示,不影响删除操作。

```rust
#[tauri::command]
fn clear_campus_net_info() -> Result<(), String> {
    let path = get_campus_config_path();
    if !path.exists() {
        return Ok(());
    }
    // fs::remove_file 在 Windows 上能直接删 Hidden 文件,无需先清 Win32 FILE_ATTRIBUTE
    fs::remove_file(&path).map_err(|e| format!("删除校园网配置失败: {}", e))?;
    Ok(())
}
```

**可复用经验**

- Win32 `FILE_ATTRIBUTE_*`(Hidden / ReadOnly / System)与 POSIX 权限位(rwx)是两套体系。`std::fs::set_permissions` 只覆盖后者;操作前者需要 `windows` crate 的 `SetFileAttributesW`。
- 跨平台代码中,`std::os::unix::fs::PermissionsExt` 的所有方法(`from_mode` / `mode` / `set_mode`)必须 `#[cfg(unix)]` 包裹。**别用 `#[cfg(windows)]` 误标**——会反向选错。
- "清除属性后删除"是反模式:大多数文件系统(Hidden / 只读 / immutable)都允许 root / 拥有者直接绕过属性删除。`remove_file` 极少需要先改属性。

### 12.2 v1.8.2 用户反馈:🎓 校园网信息显示"未配置"

**问题现象**

v1.8.2 桌面端 "网络配置" 窗口新增的 🎓 校园网信息卡片里,学号 / 运营商两项均显示"未配置",即便用户已经在 Windows 上跑过登录脚本且 xywdl.ps1 写过配置。

用户原话:
> 找不到配置文件里面的信息(🎓 校园网信息 学号: 未配置 运营商: 未配置),请参考 xywdl.sh

**根因分析**

v1.8.2 的 Rust 代码只读 `get_campus_config_path()` 返回的单一路径:

- Windows: `%APPDATA%\xxgc_campus_net_config.txt`(xywdl.ps1 写入位置)
- 其他: `dirs::config_dir() + "xxgc_campus_net_config.txt"`(Linux 走 XDG)

但本项目有 **三个独立的脚本**,写入位置各异:

| 脚本 | 平台 | 配置文件路径 |
|------|------|--------------|
| `xywdl.ps1` | Windows(PowerShell) | `$env:APPDATA\xxgc_campus_net_config.txt` |
| `xywdl.bat` | Windows(批处理,只转发到 .ps1) | 同上 |
| `xywdl.sh` | Linux / Git Bash(Shell) | `$HOME/.config/xxgcxy-wifi/login_config.json` |

用户的实际场景:在 Windows 上**用 Git Bash 跑过 `xywdl.sh`**(可能从 Linux 机器同步配置,或者直接运行 Shell 版),配置落在了 `C:\Users\thatg\.config\xxgcxy-wifi\login_config.json`,而 Rust 只看 APPDATA 那条路径,所以读不到。

进一步看 `xywdl.sh` 里的 `save_config()`:

```bash
CONFIG_FILE="$HOME/.config/xxgcxy-wifi/login_config.json"
```

而 `xywdl.ps1` 里的 `ConfigManager`:

```powershell
$this.ConfigFilePath = $path  # 来自 (Join-Path $env:APPDATA "xxgc_campus_net_config.txt")
```

**两套脚本,两套位置,Rust 只适配了其中一套。**

**修复方案(v1.8.3)**

`get_campus_config_candidates()` 返回**所有可能路径**,`get_campus_config_path()` 选第一个存在的:

```rust
/// 返回所有可能的校园网配置文件路径,按优先级排序。
///
/// 同一份配置可能被不同的脚本写入到不同位置:
/// - xywdl.ps1 (Windows) → `%APPDATA%\xxgc_campus_net_config.txt`
/// - xywdl.sh (Linux / Git Bash) → `$HOME/.config/xxgcxy-wifi/login_config.json`
fn get_campus_config_candidates() -> Vec<PathBuf> {
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

fn get_campus_config_path() -> PathBuf {
    get_campus_config_candidates()
        .into_iter()
        .find(|p| p.exists())
        .unwrap_or_else(|| { /* 默认路径用于 clear_campus_net_info 写回 */ })
}
```

这样:
- 原生 Windows 用户 + xywdl.ps1 → 走 APPDATA 路径
- Windows Git Bash 用户 + xywdl.sh → 走 `~/.config/xxgcxy-wifi/login_config.json`
- 原生 Linux 用户 + xywdl.sh → 同样走 XDG 路径

**可复用经验**

- 当一个项目有 **多端脚本**(PowerShell / Bash / Python)写同一份配置时,集中路径策略的常见做法:
  1. **文档化所有写入位置**(本节表格就是为此而生)
  2. **Reader 端做 fallback 链**,而非 Writer 端做归一化
  3. 若想完全统一,改用一个跨平台配置文件名 + 跨平台路径(本项目因为历史原因,两套路径已落地,选 reader 兜底是最低成本)
- 类似的"在 Windows 上跑 Bash 脚本"场景里,`$HOME/.config` 是真实存在的目录(指向 `C:\Users\X\.config`),**不是** Windows 原生 `%APPDATA%`。如果应用要兼容 Git Bash 用户,需要同时识别两种路径风格。

### 12.3 v1.8.2 → v1.8.3 用户反馈:xywdl.bat 中文乱码 / 脚本无法执行

**问题现象**

v1.8.2 用户在中文 Windows 上跑桌面端自动重连,日志反复出现:

```
[01:03:57] 正在检测网络...
[01:04:04] 需要登录校园网,正在执行登录...
[01:04:04] 正在执行登录脚本...
[01:04:10] 需要登录校园网,正在执行登录...
[01:04:15] 需要登录校园网,正在执行登录...
[01:04:16] 需要登录校园网,正在执行登录...
```

特征:
- 间隔 6-11 秒,等于"运行脚本 + 失败回退 + 下一次定时检测"的循环
- 中间没有任何"登录脚本执行成功 / 登录失败"日志(说明脚本大概率非正常退出,前端 invoke 拿到的是异常)
- 用户手动跑 `xywdl.bat` 时,所有 `echo [信息] ...` / `echo [执行] ...` 都是乱码

用户原话:
> 登录是使用xywdl.bat 调用内置或者本地的pwsh7,而且修复一下xywdl.bat的编码问题

**根因分析**

`file xywdl.bat` 的输出:

```
xywdl.bat: DOS batch file, Unicode text, UTF-8 text, with CRLF line terminators
```

**UTF-8 without BOM**。

中文 Windows 系统下,CMD.exe 默认活动代码页是 **CP936(GBK)**。`xywdl.bat` 是 UTF-8 编码但没有 BOM,CMD 用 GBK 解码 UTF-8 字节流,所有中文字符(例如 `echo [信息] 脚本目录: %SCRIPT_DIR%`)全部显示成乱码。

更关键的是:PowerShell 7(`pwsh.exe`)默认按 UTF-8 读 `.ps1`,所以 `xywdl.ps1` 主体能跑通;但 PowerShell 5.x(`powershell.exe`)是回退方案,它默认按 ANSI(即系统代码页)读 `.ps1`,UTF-8 无 BOM 的 `.ps1` 在 PS 5 下中文部分会被读成乱码,导致脚本行为异常甚至提前退出。

```
xywdl.bat(UTF-8 无 BOM)
   └─ CMD 用 GBK 解码 → echo 中文乱码
       └─ pwsh / powershell -File xywdl.ps1
           └─ pwsh 7: 默认 UTF-8 → 正常
           └─ PowerShell 5: 默认 ANSI → 中文部分乱码
```

**修复方案(v1.8.3)**

两步走:

**1. `xywdl.bat` 顶部强制 UTF-8 代码页**

```batch
@echo off
chcp 65001 >nul
setlocal DisableDelayedExpansion
...
```

`chcp 65001` 切换到 UTF-8 代码页(CP65001),CMD 会按 UTF-8 解析后续批处理字节流。

**2. 给 `xywdl.bat` / `xywdl.ps1` 加 UTF-8 BOM(EF BB BF)**

加 BOM 后:
- 现代 Windows(Win10 1903+)在某些场景下能自动识别 UTF-8,降低乱码概率
- **PowerShell 5.x 的 parser 看到 BOM 后会按 UTF-8 读整个文件**,而不是默认的 ANSI,这是 PS 5 兼容性的关键
- pwsh 7 看到 BOM 也 OK,直接跳过

注意: **不要**给 `xywdl.sh` 加 BOM。Bash 的 shebang `#!/bin/bash` 必须严格是文件第一字节,加 BOM 会让某些 Linux 发行版拒绝执行。

**完整修复后 `file` 输出:**

```
xywdl.bat: DOS batch file, Unicode text, UTF-8 (with BOM) text, with CRLF line terminators
xywdl.ps1: Unicode text, UTF-8 (with BOM) text, with CRLF line terminators
```

**可复用经验**

- **Windows 编码三件套**:
  - 纯 ASCII `.bat` → 任何编码都安全
  - 包含中文的 `.bat` / `.cmd` → 必须 `chcp 65001 >nul` + UTF-8 BOM
  - 包含中文的 `.ps1` → 推荐 UTF-8 BOM(PS 5 / PS 7 都吃 BOM)
  - 包含中文的 `.sh` → 不要 BOM(shebang 不能被污染);如果用 `pwsh` 解释器执行,UTF-8 无 BOM 也 OK
- 测试方法:不要只在本机 CMD 看一眼就完事,要在"全新未配置过的中文 Windows VM"上跑——很多开发机因为安装过 VS Code / Git 改过系统代码页,本地看似正常,用户机却乱码。
- 若要彻底解决跨平台编码问题,终极方案是 **多语言资源文件 + Gettext**,但对本项目这种"主要中文界面"的工具,UTF-8 + BOM 已经够用。

### 12.4 v1.8.2 → v1.8.3 GitHub Actions 构建情况对比

| Run ID | commit | 触发 | 结果 | 关键事件 |
|--------|--------|------|------|----------|
| 26767835964 | `2e95f09` | tag v1.8.2 | ✅ success | 仅 build-linux.yml,产出 `.deb` |
| 26768092563 | `2e95f09` | workflow_dispatch build-all | ❌ failure | Windows 编译失败(`from_mode` 报错) |
| 26768501624 | `c9a2b0e` | workflow_dispatch build-all | ✅ success | 修 `from_mode` 后,4 个 job 全部 OK |
| 26770393869 | `3691ae5` | tag v1.8.3 | ✅ success | 仅 build-linux.yml,产出 `.deb` |
| 26770405348 | `3691ae5` | workflow_dispatch build-all | ✅ success | v1.8.3 完整多平台打包 |

**总结**:v1.8.2 的发布流程是"先 tag 触发 Linux + 后修 Windows 编译 + 再补 Windows 构建",中间跨了 3 次 push 才齐全。v1.8.3 起,先在本地验证 Rust 编译,再 tag + workflow_dispatch 同步,避免 v1.8.2 那种中间态。

### 12.5 整体发布流程(本项目既定 SOP)

1. **代码变更后本地**:
   - `cargo check` 看 Rust 编译(本机 Windows + dlltool 可能需要 MinGW)
   - 至少在一个 Windows VM 上手动 `xywdl.bat --non-interactive` 跑通,确认编码无乱码、退出码 0
2. **commit & push 到 main**(commit 作者必须为 Thatgfsj 本人,不能用 AI 身份)
3. **bump 版本号**:同步更新 4 处
   - `package.json`
   - `src-tauri/Cargo.toml`
   - `src-tauri/Cargo.lock`(`[[package]] name = "app"` 那行)
   - `src-tauri/tauri.conf.json`
   - `.github/workflows/build-all.yml`(`APP_VERSION` / `tag_name` / 资产名 / changelog)
   - 本文档(JSDOC.md) `## 1` / `## 5.x` / `## 9`
4. **推送 tag**:`git tag -a v1.8.x -m "..." && git push origin v1.8.x`
   - 自动触发 `build-linux.yml` → 产出 `.deb`
5. **手动触发 build-all**:`gh workflow run build-all.yml`
   - 产出 `.deb` / `.rpm` / `.tar.gz` / Windows NSIS / Windows MSI / standalone `.sh`
   - 同一 run 内自动创建 GitHub Release 并上传所有 artifacts
6. **验证**:`gh release view v1.8.x` 确认资产齐全

**版本号策略**:按用户规则"它发布了多少你就更新多少",每次用户认可的 commit 都 bump 一个小版本(1.8.1 → 1.8.2 → 1.8.3)。尚未涉及 minor / major bump,需要时另议。

### 12.6 遗留项状态更新

- [x] **首次配置引导功能**已在 v1.9.0+ 落地（未配置时自动弹出 `#loginConfigScreen`）。
- [x] **账号修改入口**已在 v1.9.0+ 落地（设置页提供“更改账号信息”完整编辑模态框）。

---

### 12.7 v2.0.4 与 v2.0.5 重大健壮性加固与故障排除史

#### 1. 现象 1：前端控制台与 UI 出现莫名其妙的带时间戳空白行
- **故障特征**：每次执行登录，前端日志框会大量出现 `[HH:mm:ss]` 空白条目。
- **根因分析**：
  1. `index.html` 的 `addLog(message)` 仅使用 `String(message).split('\n')` 切分。
  2. Windows 下换行符为 `\r\n`，切分后残留 `\r`，且脚本/批处理中的段落格式空行（`Write-Host ""` 或 `echo.`）被切成空白字符串 `""`。
  3. `addLog` 无任何非空检查，无条件创建带时间戳的 DOM 节点插入日志流。
- **解决方案**：
  - 前端：使用 `/\r?\n/` 切分，`line.trim() === ''` 时直接过滤丢弃。
  - 后端：在 `src-tauri/src/lib.rs` 的 `run_login_script` 中增加 `clean_script_output` 函数，将连续空行合并折叠。
  - 启动器：`xywdl.bat` 在非交互模式（桌面调用）下收敛冗余换行。

#### 2. 现象 2：粘贴带参数重定向 URL 导致双问号 `??`、账号密码丢失、AC 报错“设备不在正常状态”
- **故障特征**：用户粘贴浏览器重定向的长 URL（包含 `?wlanuserip=...&url=...`）后，脚本拼装出的请求中缺少 `userid` 和 `passwd`，服务端返回 `{"code":"1","message":"设备不在正常状态,无法认证上网,请稍后"}`。
- **根因分析**：
  1. 前端 `parsePortalUrl` 和 `saveLoginProfile` 使用了 `setIfEmpty`，当输入框已有用户粘贴的长 URL 时，**净化后的 BaseURL 未能覆盖输入框**，存盘的仍然是含长参数的 URL。
  2. PowerShell 脚本执行 `$authUrl = $profile.base_url -replace '/\w+\.do', '/quickauth.do'` 得到带 `?` 的地址，后面紧接着直接拼接 `?` + `$queryParams`，导致请求 URL 中出现两个问号 `??`。
  3. AC 网关将第二个问号及后面的 `userid` 和 `passwd` 全部误当作上一个参数（如 `url`）的值，网关因未收到学号密码而报错。
- **解决方案（四重防御体系）**：
  1. **前端输入框**：支持 `onpaste` 粘贴后 100ms 自动提纯并回填；解析与保存时强行截断 `?` 与 `#`。
  2. **Rust 后端**：在 `save_login_profile` 落盘前强行截断 `base_url` 参数，确保存盘 profile 纯净。
  3. **PowerShell 核心**：在步骤 3 构造 URL 时显式 `$cleanBase = $profile.base_url.Split('?')[0].Split('#')[0].Trim()`，杜绝双问号。
  4. **Linux Shell 核心**：`xywdl.sh` 同样加入 `cut -d'?' -f1 | cut -d'#' -f1` 剥离。

#### 3. 现象 3：真实服务端返回带引号 code 导致正则失灵、误判“未知错误 99”
- **故障特征**：AC 返回 `{"code":"1","message":"设备不在正常状态,无法认证上网,请稍后"}`，脚本却判为 `[!] 认证结果未知`、`错误码: 99`。
- **根因分析**：
  - 校园网 AC 返回的 JSON 中，`code` 字段是带双引号的字符串 `"code":"1"`，而脚本原有正则只能匹配裸数字 `'"code"\s*:\s*1(?!\d)'`，导致正则匹配失败直接掉入 `else` 分支。
- **解决方案**：
  - 步骤 5 改用 `ConvertFrom-Json`（Linux 端用 python3 json）优先反序列化，统一转换为字符串比较。
  - 正则兜底提取同时支持带双引号和无引号形式。
  - 服务端返回非 0 响应时，直接提取真实 `message` 透传展示给用户（如“设备不在正常状态”或“账号不存在”），不再粗暴报为未知错误 99。

#### 4. 现象 4：PowerShell 5.1 解析无 BOM UTF-8 源码中文字符截断
- **故障特征**：在 Win10/Win7 纯净环境下使用系统自带 PowerShell 5.1 时，脚本报 `switch 语句缺少块`、`一元运算符缺少操作数`。
- **根因分析**：
  - Windows PowerShell 5.1 默认按系统 ANSI 代码页解析 `.ps1` 文件。若脚本为无 BOM 的 UTF-8，中文字符的第二个字节会被误吞噬，吃掉后面的双引号或花括号。
- **解决方案**：
  - 强制为 `xywdl.ps1` 写入 UTF-8 BOM（`0xEF 0xBB 0xBF`），保证 Windows 5.1 解析引擎正确识别。

#### 5. 自动化测试套件扩展
- `tests/mock_portal.py` 增加真实 AC 响应模板（双引号 code、真实 message 字段）。
- `tests/run_ps1_tests.ps1` 测试用例从 23 项扩展至 **30 项**，覆盖：
  - A 组：8 项返回码与边界判断
  - B 组：5 项配置损坏与容灾判断
  - C 组：5 项特殊字符与 URL 编码判断
  - D 组：1 项密码脱敏安全判断
  - E 组：1 项连续 5 次稳定性判断
  - F 组：新增 10 项针对脏 BaseURL 清洗、防双问号独立参数解析、带引号真实 AC 错误响应提取等高阶健壮性测试。
  - **测试结果**：30 项全量通过（PASS: 30, FAIL: 0）。

---

### 12.8 v2.0.8 桌面端深色极客毛玻璃 UI 深度重构与移动热点常开守护规范

#### 1. 设计初衷与视觉升维
原有 UI 为传统的居中白色卡片堆叠结构，视觉反馈单调且缺乏现代桌面应用层次。v2.0.8 采用 **Dark Acrylic Glassmorphism（深色极客毛玻璃）** 设计语言：
- **无缝磨砂画布**：背景深度融合深空灰黑（`#0a0b12`）与柔和紫青环境光斑，主容器配备 `backdrop-filter: blur(24px)` 亚克力磨砂玻璃。
- **发光网络健康雷达环 (Hero Gauge)**：140px SVG 动态发光弧环仪表盘，根据连通性自适应呈现翡翠绿呼吸光（已认证）、琥珀金光弧（需登录认证）、科技青蓝旋转扫描（探测中）与警示珊瑚红（掉线）。
- **学生身份名片 (Profile Card)**：数字几何头像、学号展示、中国移动/中国联通/中国电信专属品牌微光标签与一键更改快捷抽屉。
- **现代化滑动开关 (Smooth Toggles)**：自愈检测与开机自启由传统选择框升级为 iOS/macOS 风格平滑滑动开关。
- **可折叠极客活动时间轴 (Activity Terminal)**：等宽字体终端输出，支持 `[+]` 绿标、`[!]` 红标、`[*]` 蓝标语义高亮与自由收折。

#### 2. IPC 接口与业务契约 100% 零侵入兼容
重构过程中严格保留了所有 DOM 元素 ID 与事件调用契约，底层 Rust Tauri IPC 与后台静默守护逻辑完全无缝运转。

#### 3. 移动热点常开守护 (Hotspot Keep-Alive)
- **痛点与背景**：在校园网 1 人 1 账号限制场景下，学生通常通过电脑连接认证后开启 Windows 移动热点供手机/平板使用。然而 Windows 机制默认在几分钟无活跃设备或网络重连时自动关闭移动热点。
- **技术实现**：利用 Windows 原生 WinRT `Windows.Networking.NetworkOperators.NetworkOperatorTetheringManager` 投影，实现：
  - `get_hotspot_keepalive` / `set_hotspot_keepalive`: 开关状态持久化保存于 `config.json`；
  - `check_and_keep_hotspot_alive`: 实时获取 `TetheringOperationalState`，若处于 `Off` 则异步调用 `StartTetheringAsync()` 唤醒开启；
  - 联动心跳守护：在网络连通时自动检查热点状态并自动复苏。

#### 4. 开机自启静默保活设计
- **用户痛点**：电脑重启时开机自启会直接弹出界面，用户每次开机还要手动去关闭窗口，体验繁琐。
- **治理方案**：
  - `tauri.conf.json` 将窗口初始可见性定义为 `"visible": false`，彻底消除启动闪烁与误显。
  - 注册表与桌面快捷方式写入启动命令追加 `--autostart` 标志。
  - 启动钩子中解析命令行参数，检测到 `--autostart` 或后台标志时绝不调用 `window.show()`，保持窗口静默隐藏在系统托盘，仅当用户双击手动打开时才呈现主窗口。

#### 5. 账号字母与复合格式支持
- **用户痛点**：补办校园卡后运营商分配账号带有字母后缀（如 `ls`、`lls`），部分教工或专升本学号前缀带字母，此前前端限定纯数字导致该类同学无法输入和使用。
- **治理方案**：
  - 输入框与保存函数放宽为 `[a-zA-Z0-9_\-\.]`。
  - 自动侦测并剥离用户误填的 `@` 及运营商后缀，实现纯学号与复合账号的无缝兼容。

---

### 12.9 v2.0.9 认证极速响应、AC 意外断连防护与前端卡死假死根治规范

#### 1. 现象分析与根因排查
- **现场反馈**：用户反映“两分钟连不上”、“一直不动”、“一分钟没反应一直待web认证，还没我手动快”。
- **本机日志分析 (`xywdl-2026-09-04.log`)**：
  - PowerShell 发送报：`基础连接已经关闭: 连接被意外关闭。`
  - C# 发送报：`xywdl_sender.exe : [sender] 请求失败: 基础连接已经关闭: 连接被意外关闭。`
  - Python 发送报：`Remote end closed connection without response`。
  - 三层全部失败耗时长达 31 秒以上。
- **根因 1 (AC/WAF 伪装与标头缺失)**：校园网 AC 防火墙对非浏览器 User-Agent（如 `PowerShell` 或自定义标头）实施主动 RST 掐断连接；而浏览器访问能正常响应。
- **根因 2 (超时累加长达 90 秒)**：单层超时高达 30 秒，校园网内网三层失败累加 90 秒，导致界面处于长时间假死状态。
- **根因 3 (前端 30 秒全局阻塞锁)**：`runLoginScript` 的 `finally` 块中设置了 `setTimeout(() => isLoggingIn = false, 30000)`，导致 30 秒内任何 `checkNetwork` 调用均被直接拦截忽略，手动点击“立即检测”亦毫无反应。
- **根因 4 (日志刷屏与心跳过长)**：重连后在冷却期未过滤刷屏日志，且检测心跳被历史配置设为 180s（3分钟），导致断网后自愈响应迟钝。

#### 2. 治理与加固方案
- **标头模拟 (Anti-RST)**：PowerShell、C#、Python 三层全部注入主流 Chrome 标头与 `zh-CN` 语言环境，绕开 AC 伪装校验。
- **内网超时收紧**：单层超时由 30s 缩短至 6s，三层最多 18s 快速决策，彻底告别 90s 超时假死。
- **登录锁即时释放**：`runLoginScript` 完成后立即重置 `isLoggingIn = false`，冷却防抖交由 `lastLoginTime` 负责，点击“立即检测”秒级唤醒。
- **SSID 大小写容错与心跳自适应**：后端 `eq_ignore_ascii_case` 匹配 SSID，前端将心跳限制在 5~60 秒区间（默认 15s）。

### 12.10 v2.0.9+ Rust 进程内原生直发 (Sub-100ms Instant Login) 与外部脚本深度优化

#### 1. 背景与深度探针数据
经深度探针实测，外部脚本调用链路在执行网络认证前固有开销高达 **7~9.5 秒**：
- `xywdl.bat` 内部重复调用 `powershell.exe` 查询版本：**耗时 1755 ms**；
- `powershell.exe -File xywdl.ps1` 引擎冷启动：**耗时 1483 ms**；
- `xywdl.ps1` 多轮执行 `Get-WirelessAdapter` (NetAdapter + WMI) 与 `netsh`：**耗时 ~3900 ms**；
- PowerShell 5.1 `Invoke-WebRequest` 单次网络请求：**耗时 2329 ms**；
- PowerShell 5.1 进程内存工作集占用 **77.71 MB / 31 线程**。

#### 2. Rust 进程内原生直发技术实现 (`native_direct_login`)
- **零外部进程依赖**：核心认证移入桌面端 Rust 进程内，调用 Windows 原生 API `CryptUnprotectData` 解密 DPAPI 凭据（耗时 **< 1ms**）；
- **硬件网络信息毫秒级提取**：从 WLAN 接口直接提取当前连接的 WiFi 名称（SSID）、无线网卡物理 MAC 地址及本地 IPv4 地址，消除多轮 WMI 扫描；
- **异步 reqwest 直发**：使用带有 `no_proxy()`、Chrome 128 User-Agent、`Referer: http://<wlanacIp>:6060/portal.do` 的高可用 Client 直发，全流程耗时控制在 **80 毫秒以内**；
- **双模熔断与降级保护**：`run_login_script` 默认执行原生直发，遇环境异常时无缝回退至 `xywdl.bat` / `xywdl.sh` 外部脚本，实现 100% 向后兼容；
- **外部脚本瘦身**：剔除 `xywdl.bat` 重复查询版本的 1.75 秒损耗，并在 `xywdl.ps1` 为网卡适配器加入单例缓存。

### 12.11 v2.1.0 五维子代理深度审计与底层网络协议栈全链路工业级加固

#### 1. Rust 发包与网络探测加固
- **RFC 4122 v4 UUID 生成规范修复**：修复 `generate_uuid()` 中 `(bytes[8] & 0x3f) | 0x80 >> 4` 运算符优先级导致 UUID 偶发畸变为 37~38 位的底层 Bug，规范为标准 36 字符 RFC 4122 v4；
- **中文系统网卡信息提取增强**：`get_wlan_network_info()` 支持“名称”匹配，增加活动网卡 IPv4 遍历兜底，彻底杜绝回退假 IP `10.0.0.1`；
- **Captive Portal 探针加固**：204 端点遇 302 重定向 100% 判定为需登录（防止漏判），使用 `String::from_utf8_lossy` 流式读取，杜绝 GBK 编码 Portal 页面解析报错误判为已连接；备用端点接入国内高可用 204。

#### 2. Python 保底层 (`sender.py`) 加固
- **阻止 302 重定向跟随**：新增 `NoRedirectHandler`，避免认证通过后 302 跳转至校外未放行网站抛出网络错误；
- **UTF-8 二进制标准输出**：采用 `sys.stdout.buffer.write` 强制输出 UTF-8 字节流，根除 Windows GBK 控制台下 `UnicodeEncodeError` 崩溃。

#### 3. C# 原生发送器 (`sender.cs`) 加固
- **输入清洗**：循环清理 `\uFEFF` BOM 与空白字符；
- **协议升级**：显式配置 TLS 1.1 / TLS 1.2，并指定 `req.KeepAlive = false;`；
- **输出统一**：显式配置 `Console.OutputEncoding = Encoding.UTF8;`。

#### 4. 前端与系统托盘状态机闭环
- 托盘菜单“执行登录脚本”全面接入 `runLoginScript()`，共享互斥锁生命周期与 Hero Gauge 状态机联动。

### 12.12 v2.1.1 变异模糊测试（Fuzz Audit）、严格作用域净化与跨语言契约工业级核验

#### 1. 变异模糊测试套件（6万次注入）
- 构建了专用的 Fuzz 模糊测试引擎，针对 Portal URL 提纯、URL 编码器、服务端响应体解析、多网卡与热点冲突、历史配置向下兼容 5 大核心模块注入 60,004 次随机极端变异数据，实现 100% 优雅通过，无崩溃、无内存损坏、无状态死锁。

#### 2. Rust 后端深层安全扫描
- 实现了生产代码 0 处裸 `.unwrap()`、0 处 `panic!()`，Win32 Unsafe 内存句柄生命周期严格闭环释放，所有数组切片访问前置安全守卫校验。

#### 3. 前端 ES 模块严格作用域净化
- 显式在 ES 模块顶层声明 `connectedSsid`、`firstRunTimerId`、`isFirstRunSetup`、`portalParsing` 变量，根除在 Strict 模式下的全局隐式变量赋值风险。

#### 4. 跨语言配置 Schema 契约完备性核验
- 对齐 Rust、前端 JS、PowerShell 脚本间 14 个核心字段与默认值 fallback 逻辑，确保旧版本数据无缝升级。

### 12.13 v2.2.0 恢复历史命名规范（xxgcxy-wifi）与 Windows 原生中文可执行文件

#### 1. 构建与产物命名回归
- 统一 `tauri.conf.json`、`Cargo.toml`、`package.json` 的项目名称为 `xxgcxy-wifi`，构建安装包恢复为 `xxgcxy-wifi_2.2.0_x64-setup.exe` 与 `xxgcxy-wifi_2.2.0_amd64.deb`，与历史发布命名规范保持一致。

#### 2. NSIS 自动化后置处理与中文可执行文件
- 在 `installer.nsi` 的 `NSIS_HOOK_POSTINSTALL` 宏中，安装完成后自动将主可执行文件复制为 `新乡工程校园网保活.exe`。
- 清除默认英文快捷方式，生成桌面与“开始”菜单「新乡工程校园网保活」专属中文快捷方式及卸载入口。
- 在 `NSIS_HOOK_PREUNINSTALL` 宏中自动清理中文文件与快捷方式，并先安全结束相关进程。

#### 3. 运行时无缝交接与注册表适配
- Rust 后端 `run()` 增设 Windows 原生转交逻辑：如果由 `xxgcxy-wifi.exe` 启动且同目录下存在 `新乡工程校园网保活.exe`，自动转交由中文程序接管运行并退出自身。
- 注册表开机自启动使用「新乡工程校园网保活」，且向后兼容读取旧版本 `CampusWifiHelper` 与 `XXGCXY_WiFi`。
- 系统托盘悬浮提示同步设定为「新乡工程校园网保活」。
