# XXGCXY-CampusNet-AutoLogin — 完整技术文档

## 1. 项目概述

| 属性 | 值 |
|------|-----|
| **项目名称** | 校园网自动登录助手 (CampusNet Auto Login) |
| **用途** | 新乡工程学院校园网 Portal 认证自动登录 + WiFi 自动重连 |
| **仓库地址** | https://github.com/Thatgfsj/XXGCXY-CampusNet-AutoLogin |
| **作者** | Thatgfsj |
| **许可证** | MIT |
| **当前版本** | 1.8.3 |
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
| ping | 0.5 | ICMP Ping（嵌入式依赖） |
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
│       └── build-linux.yml        # Linux .deb 构建工作流（仅 tag 触发）
│
├── index.html                     # 前端单页应用（~910行），含完整 CSS + JS
├── package.json                   # Node.js 项目配置 (campus-wifi, 1.8.3)
├── package-lock.json              # 依赖锁定文件
│
├── xywdl.ps1                      # ★ 核心认证脚本（~604行，PowerShell 类实现）
├── xywdl.bat                      # Windows 启动器（~87行，自动查找 PS7/回退 PS5）
├── xywdl.sh                       # Linux 启动脚本（~305行，纯 Bash）
│
├── README.md                      # 项目说明（功能、安装、构建）
├── SPEC.md                        # 功能规范文档（13条验收标准）
├── AUTH_MECHANISM.md              # ★ 认证机制详解（Portal 协议、DPAPI 加密、204检测）
├── TECHNICAL_DOC.md               # ★ 本文档
│
├── create_icon.ps1                # 图标生成脚本
├── linux_logs.zip                 # Linux 日志归档
│
└── src-tauri/                     # Tauri 后端（Rust）
    ├── Cargo.toml                 # Rust 包配置 (app, 1.8.3)
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
    │   ├── _pw7_/                 # 内置 PowerShell 7 便携版（Git LFS 管理）
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
- **结构体**：`{ primary_ssid, backup_ssid, check_interval, test_hosts }`
- **加载时机**：程序启动时、setup 阶段（如果已有配置则隐藏窗口）
- **保存时机**：用户在配置面板点击"保存配置"
- **首次运行判断**：`primary_ssid` 为空 → 首次运行 → 显示主窗口

#### 5.1.10 Tauri Commands 清单

| Command | 参数 | 返回 | 说明 |
|---------|------|------|------|
| `load_config` | — | `Result<Config>` | 从磁盘加载配置 |
| `save_config` | `config: Config` | `Result<()>` | 保存配置到磁盘 |
| `scan_wifi` | — | `Result<Vec<WifiNetwork>>` | 扫描可用 WiFi |
| `connect_wifi` | `ssid: String` | `Result<()>` | 连接指定 WiFi |
| `get_wifi_signal` | `ssid: String` | `Result<u8>` | 获取指定 SSID 信号强度 |
| `check_network` | — | `Result<NetworkStatus>` | 综合网络状态检测 |
| `run_login_script` | — | `Result<String>` | 执行登录脚本 |
| `get_check_enabled` | — | `bool` | 获取自动检测开关状态 |
| `toggle_check_enabled` | — | `bool` | 切换自动检测开关 |
| `get_autostart_enabled` | — | `bool` | 获取开机自启状态 |
| `set_autostart_enabled` | `enabled: bool` | `Result<()>` | 设置开机自启 |
| `open_github` | — | `Result<()>` | 打开 GitHub 仓库 |
| `load_campus_net_info` | — | `Result<CampusNetInfo>` | 读取校园网配置(学号/运营商) |
| `clear_campus_net_info` | — | `Result<()>` | 删除校园网配置文件 |

---

### 5.2 前端界面 (`index.html`)

**文件行数**：910 行（单文件，CSS + HTML + JS 内联）

#### 5.2.1 界面结构

两个屏幕，通过 `.hidden` 类切换显示：

**主界面（mainScreen）**：
- 状态面板（图标 + 状态文字 + 详情）
- 网络信息面板（当前 WiFi / 主网络 / 备用网络 / 检测间隔）
- "立即检测网络" 按钮
- 自动检测开关（toggle）
- 开机自启动开关（toggle）
- "网络配置" 按钮 → 切换到配置界面
- 日志面板（黑色终端风格，保留最近 50 条）
- GitHub 链接

**配置界面（setupScreen）**：
- WiFi 列表（带信号强度、可点击选择主/备用网络）
- 已选主网络 / 备用网络显示
- 检测间隔输入（5-300 秒）
- 校园网信息卡片(学号 / 运营商,来源:xywdl.ps1 写入的 `%APPDATA%/xxgc_campus_net_config.txt`)
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

**文件行数**：604 行  
**架构**：6 个 PowerShell 类，面向对象设计

#### 5.3.1 类继承关系

```
NetworkConfig                ← 数据类：请求参数模型
DomainConfig                 ← 工具类（静态）：运营商后缀映射
ConfigManager                ← 配置读写：DPAPI 加密存储
NetworkInterfaceHelper       ← 网卡操作：获取 IP/MAC/SSID，WiFi 重连
RedirectUrlParser            ← 工具类（静态）：解析 AC 重定向 URL
AuthenticationClient         ← 编排类：整合以上所有类，执行完整登录流程
```

#### 5.3.2 参数自动检测（两级回退）

```
方法 ① GET http://www.qq.com (MaximumRedirection=0)
         └─ 捕获 302 → 解析 Location 头 URL → 提取全部参数
         └─ 失败 → 方法 ②

方法 ② GET http://172.18.252.12:6060 (MaximumRedirection=0)
         └─ 捕获 302 → 解析 URL → 用系统查询的 IP/MAC 补全缺失字段
         └─ 失败 → 方法 ③

方法 ③ 提示用户手动在浏览器复制 Portal URL 后粘贴
```

核心技术细节：
- `-MaximumRedirection 0` 阻止 PowerShell 自动跟随重定向，这样才能拿到 302 的 Location 头
- `-Proxy $null` 避免系统代理干扰
- 虚拟机网卡过滤：`InterfaceDescription` 匹配 `Wi-Fi|Wireless|WLAN`，排除 `Virtual/VMware/Hyper-V/VirtualBox`

#### 5.3.3 认证请求构造

```
目标端点：{BaseURL}/quickauth.do  （注意是 quickauth.do，不是 portal.do）
请求方法：GET
参数传递：Query String（约15个参数，全部 URL 编码）

参数分类：
  - 用户凭证：userid, passwd
  - 设备信息：wlanuserip, mac, vlan, hostname
  - AC 固定参数：wlanacname, wlanacIp, portalpageid, portaltype, version, bindCtrlId
  - 唯一性参数：uuid (GUID v4), timestamp (毫秒级)
```

#### 5.3.4 密码加密存储

```
写入：Read-Host -AsSecureString → ConvertFrom-SecureString (DPAPI) → Base64 → JSON
读取：JSON Base64 → ConvertTo-SecureString (DPAPI) → SecureStringToBSTR → 使用后 FreeBSTR
```

- 配置文件路径：`$env:APPDATA/xxgc_campus_net_config.txt`
- 文件属性设为 Hidden
- 密码仅在使用时短暂存在于内存，用完立即 `FreeBSTR` 释放

#### 5.3.5 认证结果判断

| 响应特征 | 判断 |
|----------|------|
| `"code":0` / `success` / `认证成功` | 通过 |
| `"code":1` / `账号不存在` | 失败：账号不存在 |
| `"code":44` / `非法接入` | 失败：非法接入（VLAN/MAC 不匹配） |
| 其他 | 未知 |

---

### 5.4 启动器脚本

#### 5.4.1 Windows (`xywdl.bat`)

查找策略（优先级从高到低）：
1. `%~dp0\_pw7_\pwsh.exe`（内置 PS7 便携版）
2. `%~dp0\..\bin\_pw7_\pwsh.exe`
3. `%~dp0\..\_pw7_\pwsh.exe`
4. `%~dp0\bin\_pw7_\pwsh.exe`
5. 系统 PATH 中的 `pwsh`
6. 回退到 `powershell`（PS 5.1）

执行方式：`pwsh/powershell -ExecutionPolicy Bypass -File "xywdl.ps1" [args]`

#### 5.4.2 Linux (`xywdl.sh`)

- 配置路径：`~/.config/xxgcxy-wifi/login_config.json`
- 使用 `curl --max-redirs 0` 实现重定向捕获
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
  "check_interval": 15,
  "test_hosts": [
    "http://connect.rom.miui.com/generate_204",
    "http://httpstat.us/204"
  ]
}
```

### 6.2 认证脚本配置

```
%APPDATA%/xxgc_campus_net_config.txt       (Windows)
~/.config/xxgcxy-wifi/login_config.json    (Linux)
```

```json
{
  "BaseURL": "http://172.18.252.12:6060/portal.do",
  "WlanAcName": "XXGC-AC-01",
  "WlanAcIp": "172.18.252.1",
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

- **v1.8.3**：当前版本。修复校园网配置读取路径(同时支持 xywdl.ps1 的 APPDATA 路径与 xywdl.sh 的 `~/.config/xxgcxy-wifi/login_config.json` 路径),Windows 上 Git Bash 用户也能识别;xywdl.bat 顶部加 `chcp 65001 >nul` 切换到 UTF-8 代码页,xywdl.bat / xywdl.ps1 加 UTF-8 BOM 兼容 PowerShell 5。
- **v1.8.2**：在"网络配置"窗口展示校园网信息(学号/运营商),并提供"清理校园网信息"按钮一键删除登录配置。新增 Tauri 命令 `load_campus_net_info` / `clear_campus_net_info`,后端从 `%APPDATA%/xxgc_campus_net_config.txt` 读取 `UserId` 字段并按 `@` 拆分为学号 + 运营商后缀(移动/联通/电信)。
- **v1.8.1**：修复 bat 引号崩溃、重排托盘菜单、手动连接改为仅连 WiFi、执行登录脚本改为直接运行、Linux 缺少 --non-interactive 修复、清理代码警告、完善技术文档。
- **v1.7.11**：内置 PS7 支持、NSIS 安装器、跨平台构建、`_pw7_` 资源路径修正、便携构建验证步骤优化。

---

## 10. 开发指南

### 10.1 环境要求

- Rust 1.77.2+
- Node.js 18+
- Windows 10/11 或 Ubuntu 24.04
- Git LFS（存储 PS7 便携版二进制）

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
| 校园网配置读取 | `lib.rs` | `load_campus_net_info` / `get_campus_config_path` | 121-187 |
| 校园网配置清理 | `lib.rs` | `clear_campus_net_info` | 190-201 |
| 校园网信息展示 | `index.html` | `loadCampusNetInfo` / `clearCampusInfo` | 904-936 |
| 系统托盘 | `lib.rs` | `setup_tray` | 788-831 |
| 应用入口 | `lib.rs` | `run` | 836-900 |
| 前端状态机 | `index.html` | `checkNetwork` / `reconnectWifi` | 525-690 |
| 手动连接WiFi | `index.html` | `manualWifiConnect` | 704-773 |
| 重定向解析 | `xywdl.ps1` | `[RedirectUrlParser]::ParseRedirectUrl` | 237-279 |
| 自动检测参数 | `xywdl.ps1` | `[AuthenticationClient].TryAutoDetectParams` | 375-433 |
| DPAPI 密码存储 | `xywdl.ps1` | `[ConfigManager].SaveConfig` / `.LoadConfig` | 67-148 |
| 认证请求执行 | `xywdl.ps1` | `[AuthenticationClient].PerformAuthentication` | 519-588 |
