# XXGCXY-CampusNet-AutoLogin

新乡工程学院校园网自动登录助手 —— 基于 Tauri 2.x 的 Windows / Linux 桌面应用，自动检测 / 重连校园网 WiFi 并完成 Portal 认证登录。

> 校园网认证机制详解讲解（感兴趣的话推荐查看）：[AUTH_MECHANISM.md](./AUTH_MECHANISM.md)
>
> 开发者文档（API、架构、问题修复史）：[JSDOC.md](./JSDOC.md)

## 技术栈

<p align="left">
  <img src="https://img.shields.io/badge/PowerShell-5.1%2B-5391FE?style=flat-square&logo=powershell&logoColor=white" alt="PowerShell">
  <img src="https://img.shields.io/badge/Rust-1.70%2B-000000?style=flat-square&logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/Tauri-2.x-FFC131?style=flat-square&logo=tauri&logoColor=black" alt="Tauri">
  <img src="https://img.shields.io/badge/JavaScript-ES6-F7DF1E?style=flat-square&logo=javascript&logoColor=black" alt="JavaScript">
  <img src="https://img.shields.io/badge/HTML-CSS-E34F26?style=flat-square&logo=html5&logoColor=white" alt="HTML/CSS">
  <img src="https://img.shields.io/badge/Node.js-18%2B-339933?style=flat-square&logo=nodedotjs&logoColor=white" alt="Node.js">
  <img src="https://img.shields.io/badge/Batch-4D4D4D?style=flat-square&logo=windowsterminal&logoColor=white" alt="Batch">
  <img src="https://img.shields.io/badge/Shell-121011?style=flat-square&logo=gnubash&logoColor=white" alt="Shell">
  <img src="https://img.shields.io/badge/Git_LFS-F05032?style=flat-square&logo=git&logoColor=white" alt="Git LFS">
</p>

## 下载

**GitHub Releases**: [最新版本](https://github.com/Thatgfsj/XXGCXY-CampusNet-AutoLogin/releases)

| 版本 | 说明 | 内置 PS7 | 需要系统 PS 5.1+ |
|------|------|:---------:|:-------------:|
| **Windows NSIS / MSI** | Windows 安装器，体积小 | ❌ | ✅ |
| **Linux** | .deb / .rpm / tar.gz | 👻 | 👻 |

> v1.9.0+ 起不再内置 PowerShell 7 移植版。Windows 10/11 自带 PowerShell 5.1，已支持 DPAPI 加密。

## 功能

- **可视化登录配置 (v1.9.0+)** — 首次启动弹窗引导,运营商/学号/密码/Portal URL 表单填写,小白也能自助配置;"设置"页(原"网络配置"页)有"更改账号信息"按钮可随时再改
- **请求发送多层级保底 (v2.0.0+)** — 登录请求发送失败自动降级: Windows 依次尝试 PowerShell → C# → Python;Linux 依次尝试 curl → Python。任一层成功即可完成认证
- **自动检测网络状态** — 实时监测 WiFi 连接和互联网访问
- **自动重连 WiFi** — 断网时自动连接预设的 WiFi
- **自动登录校园网** — 连接后自动执行 PowerShell 认证脚本
- **系统托盘运行** — 最小化到托盘,后台静默运行
- **开机自启** — 可选的开机自动启动
- **凭证加密存储** — 密码通过 Windows DPAPI 加密, 不存明文
- **中文界面** — 安装包和 UI 均支持简体中文

## 安装

### Windows（推荐）

1. 下载最新的 `xxgcxy-wifi_x.x.x_x64-setup.exe`
2. 双击运行，按提示完成安装
3. 首次运行需配置 WiFi 和账号信息

**系统要求**:
- **Windows 10 / 11**: 自带 PowerShell 5.1 + .NET Framework 4.x，**无需任何额外操作**（开箱即用）
- **Windows 8 / 8.1 / Server 2012-2012R2**: 自带 PowerShell 4.0，需升级到 PowerShell 5.1
  - 下载 WMF 5.1: <https://www.microsoft.com/en-us/download/details.aspx?id=54616>
- **Windows 7 SP1 / Server 2008 R2 SP1**: 自带 PowerShell 2.0，**必须**手动安装 WMF 5.1
  - 下载 WMF 5.1: <https://www.microsoft.com/en-us/download/details.aspx?id=54616>
  - 安装后重启
- **Windows XP / Vista / Server 2003/2008**: **不支持**（太老）

> 自 v1.9.0 起不再内置 PowerShell 7 移植版。Windows 自带的 PowerShell 5.1+ 完整支持 DPAPI `ProtectedData`（.NET Framework 4.x）。v2.0.0 起登录请求发送带多层级降级（PowerShell → C# → Python），任一层可用即可认证，脚本已做 Win 7/8 兼容（WMI fallback 处理 Get-NetAdapter 不存在的问题）。

### Linux

```bash
chmod +x xywdl.sh
./xywdl.sh     # 需要 curl 或 python3 (登录请求发送会自动选择可用的一层)
```

## 项目结构

```
├── index.html              # 前端界面 (HTML/CSS/JS)
├── package.json            # Node.js 依赖
├── xywdl.ps1               # 校园网认证脚本 (PowerShell)
├── xywdl.bat               # Windows 启动器（找系统 pwsh / powershell）
├── xywdl.sh                # Linux 启动脚本
├── src/sender/             # 请求发送保底层 (v2.0.0+)
│   ├── sender.cs           # C# 源码 (预编译为 xywdl_sender.exe)
│   ├── sender.py           # Python 源码 (纯标准库, 跨平台)
│   └── xywdl_sender.exe    # 预编译 C# 发送器 (~5KB, .NET Framework 4.x)
├── AUTH_MECHANISM.md       # 校园网认证机制详解
├── JSDOC.md                # 项目技术文档（含 API、架构、问题修复史）
├── src-tauri/              # Tauri 后端 (Rust)
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── src/
│       ├── lib.rs          # 核心业务逻辑
│       └── main.rs         # 程序入口
└── .github/                # CI/CD 配置
```

## 开发

### 环境要求

- Rust 1.70+
- Node.js 18+
- Windows 10/11 或 Linux
- .NET Framework 4.x（仅 Windows 上构建 C# sender 用，本机 Windows 自带）

### 构建

```bash
git clone https://github.com/Thatgfsj/XXGCXY-CampusNet-AutoLogin.git
cd XXGCXY-CampusNet-AutoLogin
git lfs pull
npm ci
npx @tauri-apps/cli build
```

### 分支说明

| 分支 | 用途 |
|------|------|
| `win-portable` | Windows 便携版（含内置 PS7） |
| `win-system-ps7` | Windows 版（需系统 PS7） |
| `linux-sh` | Linux 纯脚本版 |

## 更新日志

- **v2.0.1**：修复桌面端调用登录脚本时 bat 失败路径的 `pause` 永久挂死；发送超时统一改为 30 秒；登录脚本实际输出回显到 UI 日志。
- **v2.0.0**：修复登录无法使用（MAC 留空被误判为缺少字段）、请求发送多层级降级（Windows: PowerShell→C#→Python；Linux: curl→Python）、跨平台 Python 保底层、Portal URL 解析按钮防连点、清理死代码。详见 [JSDOC.md §9](JSDOC.md)。
- **v1.9.1**：修复认证结果判定正则误匹配（`code:10/100/123` 被误判为“账号不存在”、`code:440` 被误判为“非法接入”）、请求日志脱敏密码（`passwd=***`）、统一 PS 端参数 URL 编码；新增自动化测试套件（`tests/`）与 Rust 单元测试。详见 [JSDOC.md §9](JSDOC.md)。
- **v1.9.0**：登录模块解耦为 JSON 模板 + 渲染器；可视化登录配置屏；DPAPI 加密密码；取消内置 PS7。

## 许可证

[MIT License](LICENSE) © Thatgfsj
