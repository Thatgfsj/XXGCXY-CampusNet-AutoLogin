# XXGCXY-CampusNet-AutoLogin

新乡工程校园网自动登录助手 - 基于 Tauri 2.x 的 Windows 桌面应用，用于自动检测和重连校园网 WiFi，并执行自动登录脚本。

## 下载地址

**GitHub Releases**: https://github.com/Thatgfsj/XXGCXY-CampusNet-AutoLogin/releases

## 版本说明

本项目提供三个版本的安装包：

| 版本 | 说明 | 内置PS7 | 需要系统PS7 |
|------|------|---------|-------------|
| **内置PS7版** | Windows 便携版，开箱即用 | ✅ | ❌ |
| **系统PS7版** | Windows 版，需系统已安装 PS7 | ❌ | ✅ |
| **Linux版** | 纯 Shell 脚本，需系统安装 pwsh | N/A | ✅ |

### 各版本选择建议

- **Windows 用户想要开箱即用** → 下载 `xxgcxy-wifi_1.4.0_x64-setup.exe` 或 `xxgcxy-wifi_1.4.0_x64_en-US.msi`
- **Windows 用户系统已安装 PS7** → 下载 `xxgcxy-wifi_1.4.0_x64-setup_no-ps7.exe`
- **Linux 用户** → 下载 `xywdl.sh` 脚本

## 功能特性

- **自动检测网络状态** - 实时监测 WiFi 连接和互联网访问状态
- **自动重连 WiFi** - 检测到断网时自动连接预设的 WiFi 网络
- **自动登录校园网** - WiFi 连接后自动执行登录脚本
- **系统托盘运行** - 关闭窗口后最小化到托盘，后台运行
- **开机自启动** - 可选的开机自动启动功能
- **中文安装界面** - 安装包支持简体中文界面
- **内置 PowerShell 7** - 便携版已内置 PS7，无需手动安装

## 安装说明

### Windows 安装包（推荐）

1. 下载最新的 `xxgcxy-wifi_x.x.x_x64-setup.exe` 安装包
2. 双击运行安装程序，按照提示完成安装
3. 安装完成后，桌面会生成快捷方式

> **注意**：内置PS7版本已内置 PowerShell 7，无需额外安装。

### Windows 便携版（无内置PS7）

1. 下载 `xxgcxy-wifi_x.x.x_x64-setup_no-ps7.exe`
2. 确保系统已安装 PowerShell 7
3. 双击运行安装程序

### Linux

1. 下载 `xywdl.sh` 脚本
2. 添加执行权限: `chmod +x xywdl.sh`
3. 安装 pwsh (PowerShell): 参考 https://docs.microsoft.com/zh-cn/powershell/scripting/install/installing-powershell-on-linux
4. 运行脚本: `./xywdl.sh`

## 技术架构

### 组件说明

| 组件 | 说明 |
|------|------|
| `xywdl.bat` | Windows 启动器，自动查找并调用内置或系统 PowerShell |
| `xywdl.ps1` | 校园网登录 PowerShell 脚本 |
| `src-tauri/bin/_pw7_/` | 内置的 PowerShell 7 便携版 |
| `xxgcxy-wifi.exe` | Tauri 应用主程序 |

### PowerShell 查找顺序

`xywdl.bat` 按以下顺序查找 PowerShell：

1. `安装目录\_pw7_\pwsh.exe` - 内置的 PS7（优先）
2. `安装目录\..in\_pw7_\pwsh.exe`
3. 系统 `pwsh.exe`（PowerShell 7）
4. 系统 `powershell.exe`（Windows PowerShell 5.1，回退）

### 内置 PowerShell 7 许可

本项目内置的 PowerShell 7 便携版来自 [github.com/PowerShell/PowerShell](https://github.com/PowerShell/PowerShell)，遵循 [MIT 开源协议](https://github.com/PowerShell/PowerShell/blob/master/LICENSE.txt)。

## 项目结构

```
xxgcxy-wifi/
├── index.html              # 前端界面
├── package.json            # Node.js 依赖配置
├── xywdl.ps1              # 校园网登录脚本
├── xywdl.bat              # Windows 启动器
├── xywdl.sh               # Linux 启动脚本
├── src-tauri/             # Tauri 后端目录
│   ├── Cargo.toml          # Rust 依赖配置
│   ├── tauri.conf.json     # Tauri 配置
│   ├── bin/_pw7_/         # 内置 PowerShell 7 便携版
│   └── src/
│       ├── lib.rs          # 主要业务逻辑
│       └── main.rs         # 程序入口
└── dist/                   # 前端构建输出
```

## 开发

### 环境要求

- Rust 1.70+
- Node.js 18+
- Windows 10/11 或 Linux
- Git LFS (用于存储 PS7 便携版文件)

### 构建步骤

1. 克隆仓库并拉取 LFS 文件:
   ```bash
   git clone https://github.com/Thatgfsj/XXGCXY-CampusNet-AutoLogin.git
   cd XXGCXY-CampusNet-AutoLogin
   git lfs pull
   ```

2. 安装前端依赖:
   ```bash
   npm ci
   ```

3. 构建 Tauri 应用:
   ```bash
   npx @tauri-apps/cli build
   ```

### 相关仓库

- **win-portable 分支**: Windows 便携版（含内置PS7）
- **win-system-ps7 分支**: Windows 版（需系统PS7）
- **linux-sh 分支**: Linux 纯脚本版

## 许可证

MIT License

作者: Thatgfsj
