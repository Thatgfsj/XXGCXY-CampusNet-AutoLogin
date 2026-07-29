## 登录模块解耦 + UI 入口改造实施计划（v1.9.0）

### 背景
当前校园网登录逻辑硬编码在 `xywdl.ps1` (603 行) 中：自动检测 portal 重定向 / 手动粘贴 URL / 交互式选运营商+学号+密码。要把它拆成"JSON 模板 + 渲染器"模式：UI 表单驱动配置，PS 脚本只读配置执行，Rust 端提供 DPAPI 加密。

### 决策
- **入口**：首次启动自动弹登录配置屏；之后在"网络配置"页加"更改账号信息"按钮
- **架构**：保留 xywdl.ps1 作为执行壳（跟之前一样）
- **多 Profile**：仅支持一个账号
- **遗留脚本**：xywdl.sh 保留（服务 Linux 脚本用户），xywdl.bat 保留
- **兼容性**：不兼容旧 `xxgc_campus_net_config.txt`，启动时检测到就提示重新配置
- **版本号**：v1.9.0

### 新增文件位置
```
%APPDATA%/xxgcxy-wifi/
├── config.json              # 现有（WiFi 主备/间隔）— 不动
├── login_profile.json       # 新：元数据（学号/运营商/SSID/portal URL/AC/VLAN/MAC 等）
└── login_credential.bin     # 新：DPAPI 加密的 SecureString 密码（PS 端 ConvertFrom-SecureString 可读）
```

### 文件改动清单

| 文件 | 改动 |
|---|---|
| `xywdl.ps1` | 砍掉 TryAutoDetectParams + 交互输入；新增 LoginProfile 类读 JSON；密码读 .bin |
| `xywdl.sh` | 仅加注释建议用 Tauri 应用，逻辑不动 |
| `xywdl.bat` | 不动 |
| `src-tauri/src/lib.rs` | 新增 6 个 Tauri 命令 + 2 结构体 + DPAPI 加密 + 改写旧命令数据源 |
| `src-tauri/Cargo.toml` | windows crate 启用 `Win32_Security_Cryptography` |
| `src-tauri/tauri.conf.json` | 不动 |
| `index.html` | 新增 #loginConfigScreen 完整屏 + 启动跳转 + 表单事件 |
| `JSDOC.md` | §5.1.10 加 6 命令 + §5.2.1 描述登录屏 + §9 v1.9.0 changelog |
| `SPEC.md` | §3.2.x 扩写 |
| `README.md` | 微调特性描述 |
| `package.json` / `Cargo.toml` / `tauri.conf.json` | 升 v1.9.0 |

### 新增 Rust 命令

| 命令 | 入参 | 返回 |
|---|---|---|
| `is_login_configured` | — | `bool` |
| `get_login_profile` | — | `Result<LoginProfile, String>` |
| `save_login_profile` | `LoginProfile, password: String` | `Result<(), String>` |
| `clear_login_profile` | — | `Result<(), String>` |
| `parse_portal_url` | `url: String` | `Result<ParsedPortal, String>` |
| `run_login_with_profile` | — | `Result<String, String>` |

### 实施阶段

**阶段 1: Rust 基础设施**
- 加 LoginProfile/ParsedPortal 结构体
- 实现 5 个新命令 + dpapi_protect
- 改写 load_campus_net_info 读新数据源
- 保留 run_login_script 兼容层

**阶段 2: PS 脚本改造**
- 删 TryAutoDetectParams + RedirectUrlParser + PromptForCredentials
- 新增 LoadLoginProfile(profilePath, credPath)
- PerformAuthentication 改读 profile
- 手工测试：构造 test profile 跑 pwsh -File xywdl.ps1 --non-interactive

**阶段 3: 前端改造**
- 加 #loginConfigScreen 完整 HTML（运营商/学号/密码/portal URL/解析按钮/高级字段折叠/保存&保存并登录）
- 启动时 is_login_configured → 自动跳转
- 主页 + 网络配置页加入口按钮
- 改 loadCampusNetInfo 渲染新数据源

**阶段 4: 文档 + 收尾**
- 更新 JSDOC.md / SPEC.md / README.md
- 三处版本号升 1.9.0
- git commit + tag v1.9.0 + 同步 win-portable / win-system-ps7 / linux-sh 三个构建分支
- 手动 e2e 验证

### 风险与缓解

| 风险 | 缓解 |
|---|---|
| Rust DPAPI 加密 PS 解不开 | 字节序测试 + 保留 run_login_script 兜底 |
| 旧 config 文件怎么办 | 启动检测到 → 提示重新配置 |
| 首次弹窗打扰用户 | 弹窗含"稍后"按钮，下次启动再弹 |
| parse_portal_url 正则跟 PS 不一致 | 复用 PS 端 RedirectUrlParser 同一套正则 |
| 构建分支没同步 | 阶段 4 同步（沿用上轮经验） |