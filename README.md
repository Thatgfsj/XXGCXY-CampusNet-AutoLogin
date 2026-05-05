# XXGCXY-CampusNet-AutoLogin

新乡工程学院校园网自动登录助手 —— 基于 Tauri 2.x 的 Windows 桌面应用，自动检测 / 重连校园网 WiFi 并完成 Portal 认证登录。

> 校园网认证机制详解讲解（感兴趣的话推荐查看）：[AUTH_MECHANISM.md](./AUTH_MECHANISM.md)

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

| 版本 | 说明 | 内置 PS7 | 需要系统 PS7 |
|------|------|:---------:|:-------------:|
| **内置 PS7 版** | Windows 便携版，开箱即用 | ✅ | ❌ |
| **系统 PS7 版** | Windows 版，需系统已安装 PS7 | ❌ | ✅ |
| **Linux 版** | 纯 Shell 脚本，需系统安装 pwsh | 👻 | 👻 |

## 功能

- **自动检测网络状态** — 实时监测 WiFi 连接和互联网访问
- **自动重连 WiFi** — 断网时自动连接预设的 WiFi
- **自动登录校园网** — 连接后自动执行 PowerShell 认证脚本
- **系统托盘运行** — 最小化到托盘，后台静默运行
- **开机自启** — 可选的开机自动启动
- **凭证加密存储** — 密码通过 Windows DPAPI 加密，不存明文
- **中文界面** — 安装包和 UI 均支持简体中文

## 安装

### Windows（推荐）

1. 下载最新的 `xxgcxy-wifi_x.x.x_x64-setup.exe`
2. 双击运行，按提示完成安装
3. 首次运行需配置 WiFi 和账号信息

> **内置 PS7 版** 已自带 PowerShell 7，无需额外安装。
> **系统 PS7 版** 需系统已安装 [PowerShell 7](https://github.com/PowerShell/PowerShell)。

### Linux

```bash
chmod +x xywdl.sh
sudo apt install pwsh        # Debian/Ubuntu
sudo dnf install powershell   # Fedora
./xywdl.sh
```

## 项目结构

```
├── index.html              # 前端界面 (HTML/CSS/JS)
├── package.json            # Node.js 依赖
├── xywdl.ps1               # 校园网认证脚本 (PowerShell 类实现)
├── xywdl.bat               # Windows 启动器
├── xywdl.sh                # Linux 启动脚本
├── AUTH_MECHANISM.md       # 认证机制详解文档
├── src-tauri/              # Tauri 后端 (Rust)
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── bin/_pw7_/          # 内置 PowerShell 7 便携版
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
- Git LFS（用于存储 PS7 便携版文件）

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

## 许可证

[MIT License](LICENSE) © Thatgfsj
