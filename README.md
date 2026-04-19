# XXGCXY-CampusNet-AutoLogin - Windows 便携版

校园网自动登录助手 - Windows 便携版，内置 PowerShell 7，开箱即用。

## 下载地址

**GitHub Releases**: https://github.com/Thatgfsj/XXGCXY-CampusNet-AutoLogin/releases/tag/v1.4.0-all

## 版本说明

本版本已内置 PowerShell 7.6.0 便携版，无需额外安装，开箱即用。

### 内置 PowerShell 7 许可声明

本版本内置的 PowerShell 7 便携版遵循 MIT 开源协议。

来源: https://github.com/PowerShell/PowerShell
许可证: MIT License

## 功能特性

- **自动检测网络状态** - 实时监测 WiFi 连接和互联网访问状态
- **自动重连 WiFi** - 检测到断网时自动连接预设的 WiFi 网络
- **自动登录校园网** - WiFi 连接后自动执行登录脚本
- **开机自启动** - 可选的开机自动启动功能
- **内置 PowerShell 7** - 已内置 PS7，无需手动安装

## 安装说明

1. 下载 `xxgcxy-wifi_1.4.0_x64-setup.exe`
2. 双击运行安装程序，按照提示完成安装
3. 安装完成后，桌面会生成快捷方式

## 技术架构

| 组件 | 说明 |
|------|------|
| `xywdl.bat` | Windows 启动器，自动查找并调用内置 PowerShell |
| `xywdl.ps1` | 校园网登录 PowerShell 脚本 |
| `src-tauri/bin/_pw7_/` | 内置的 PowerShell 7.6.0 便携版 |
| `xxgcxy-wifi.exe` | Tauri 应用主程序 |

### PowerShell 查找顺序

1. `安装目录\_pw7_\pwsh.exe` - 内置的 PS7（优先）
2. 系统 `pwsh.exe`（PowerShell 7）
3. 系统 `powershell.exe`（Windows PowerShell 5.1，回退）

## WiFi扫描权限说明

WiFi扫描功能需要以下权限之一：

1. **以管理员身份运行** - 右键程序选择"以管理员身份运行"
2. **开启位置服务** - 前往 设置 > 隐私与安全 > 位置服务 开启

如果遇到WiFi扫描失败，请尝试上述方法。

## 许可证

MIT License

作者: Thatgfsj
