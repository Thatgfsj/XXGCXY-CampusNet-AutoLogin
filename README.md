# XXGCXY-CampusNet-AutoLogin

校园网自动登录助手 - 一个基于 Tauri 2.x 开发的 Windows 桌面应用，用于自动检测和重连校园网 WiFi，并执行自动登录脚本。

## 功能特性

- **自动检测网络状态** - 实时监测 WiFi 连接和互联网访问状态
- **自动重连 WiFi** - 检测到断网时自动连接预设的 WiFi 网络
- **自动登录校园网** - WiFi 连接后自动执行登录脚本
- **系统托盘运行** - 关闭窗口后最小化到托盘，后台运行
- **开机自启动** - 可选的开机自动启动功能
- **中文安装界面** - 安装包支持简体中文界面
- **内置 PowerShell 7** - 安装包已内置 PS7，无需手动安装

## 安装

### Windows 安装包（推荐）

1. 下载最新的 `xxgcxy-wifi_x.x.x_x64-setup.exe` 安装包
2. 双击运行安装程序，按照提示完成安装
3. 安装完成后，桌面会生成快捷方式

> **注意**：安装包已内置 PowerShell 7，无需额外安装或网络连接。

### Windows 便携版

下载 `xxgcxy-wifi.exe` 独立运行程序（不包含 PowerShell 7，需确保系统已安装 PowerShell 7）。

### Linux

下载并运行 `xywdl.sh` 脚本（需要系统已安装 PowerShell 7 或 pwsh）。

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
2. `安装目录\..\bin\_pw7_\pwsh.exe`
3. 系统 `pwsh.exe`（PowerShell 7）
4. 系统 `powershell.exe`（Windows PowerShell 5.1，回退）

### 内置 PowerShell 7 许可

本项目内置的 PowerShell 7 便携版来自 [github.com/PowerShell/PowerShell](https://github.com/PowerShell/PowerShell)，遵循 [MIT 开源协议](https://github.com/PowerShell/PowerShell/blob/master/LICENSE.txt)。

## 项目结构

```
xxgcxy-wifi/
├── index.html              # 前端界面
├── package.json            # Node.js 依赖配置
├── xywdl.ps1               # 校园网登录脚本
├── xywdl.bat               # Windows 启动器
├── xywdl.sh                # Linux 启动脚本
├── src-tauri/              # Tauri 后端目录
│   ├── Cargo.toml          # Rust 依赖配置
│   ├── tauri.conf.json     # Tauri 配置
│   ├── bin/_pw7_/          # 内置 PowerShell 7 便携版
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

### 构建步骤

```bash
# 安装依赖
npm install

# 开发模式运行（需要系统已安装 PowerShell 7）
npm run tauri dev

# 构建生产版本
npm run tauri build
```

> **Windows 构建说明**：PowerShell 7 便携版已内置在 `src-tauri/bin/_pw7_/`，会自动打包进安装包。登录脚本会优先使用内置 PS7，如找不到则回退使用系统 `powershell`。

### 构建产物

- **Windows 安装包**: `src-tauri/target/release/bundle/nsis/xxgcxy-wifi_x.x.x_x64-setup.exe`
- **Windows MSI**: `src-tauri/target/release/bundle/msi/xxgcxy-wifi_x.x.x_x64.msi`
- **Windows 独立程序**: `src-tauri/target/release/xxgcxy-wifi.exe`
- **Linux DEB**: `src-tauri/target/release/bundle/deb/xxgcxy-wifi_x.x.x_amd64.deb`

## 首次配置

1. 启动程序
2. 点击 **"网络配置"** 按钮
3. 程序会自动扫描附近的 WiFi 网络
4. 选择您的 **主网络**（优先连接）和 **备用网络**
5. 设置检测间隔（默认 15 秒，建议 10-60 秒）
6. 点击 **"保存配置"**

### 登录脚本配置

1. 退出校园网（浏览器输入 2.2.2.2，点击退出）
2. 等待弹出校园网登录页面时，复制地址栏中的 URL
3. 程序会弹出脚本配置窗口，粘贴 URL 并填写个人信息

## 注意事项

1. **配置文件安全** - `xywdl.ps1` 包含您的账号密码，请妥善保管
2. **网络检测间隔** - 建议设置 10-60 秒，过短可能影响性能
3. **首次运行** - 需要先配置 WiFi 网络才能正常使用
4. **管理员权限** - 连接某些 WiFi 可能需要管理员权限

## 许可证

MIT License

## 开源许可

- **本项目**：MIT License
- **PowerShell 7**：MIT License（来源：github.com/PowerShell/PowerShell）

## 作者

[Thatgfsj](https://github.com/Thatgfsj)

---

如有问题或建议，欢迎提交 [Issue](https://github.com/Thatgfsj/XXGCXY-CampusNet-AutoLogin/issues)

> **声明**：本脚本所有经验仅供自动化协议学习研究，请遵守各高校管理相关要求。
test
