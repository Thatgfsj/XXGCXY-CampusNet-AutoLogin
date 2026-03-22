# XXGCXY-CampusNet-AutoLogin

校园网自动登录助手 - 一个基于 Tauri 2.x 开发的 Windows 桌面应用，用于自动检测和重连校园网 WiFi，并执行自动登录脚本。

本项目内置 PowerShell 7 便携版（来源： https://github.com/PowerShell/PowerShell ） ，PowerShell 7 遵循 MIT 开源协议，本项目对其的使用符合该协议要求。

## 功能特性

- **自动检测网络状态** - 实时监测 WiFi 连接和互联网访问状态
- **自动重连 WiFi** - 检测到断网时自动连接预设的 WiFi 网络
- **自动登录校园网** - WiFi 连接后自动执行登录脚本
- **系统托盘运行** - 关闭窗口后最小化到托盘，后台运行
- **开机自启动** - 可选的开机自动启动功能
- **中文安装界面** - 安装包支持简体中文界面

## 使用方法

### 安装

1. 下载最新的 `xxgcxy-wifi_1.0.0_x64.msi` 或 `xxgcxy-wifi_1.1.2_x64-setup.exe` 安装包
2. 双击运行安装程序，按照提示完成安装
3. 安装完成后，桌面会生成快捷方式

### 首次配置

1. 启动程序
2. 点击 **"网络配置"** 按钮
3. 程序会自动扫描附近的 WiFi 网络
4. 选择您的 **主网络**（优先连接）和 **备用网络**
5. 设置检测间隔（默认 15 秒，建议 10-60 秒）
6. 点击 **"保存配置"**

### 登录脚本配置

1.退出校园网（浏览器输入2.2.2.2，点击退出）
2.等待弹出校园网登录页面的时候，复制上方的网站（172.xx.xxx.xxx:6060:/portal.do?xxxxxxxxxxxxxxxxxxxxx这一串）
3.等待应用弹出脚本窗口，第一次会让你填写校园网信息，先把刚刚复制的网站复制进去，再输入个人信息即可

脚本源码在https://www.bilibili.com/opus/1174460821448687621
或者https://github.com/Thatgfsj/XXGC-CampusNet-AutoLogin
实在不会用可以复制上方的链接，让AI教你如何使用。

### 日常使用

- **最小化到托盘** - 点击窗口关闭按钮（X），程序会隐藏到系统托盘继续运行
- **托盘菜单** - 右键托盘图标可显示窗口、立即检测或退出程序
- **自动检测** - 开启后程序会按设定间隔自动检测网络状态
- **手动检测** - 点击 "立即检测网络" 按钮可随时检查网络状态

## 开发框架

### 技术栈

| 技术 | 版本 | 用途 |
|------|------|------|
| [Tauri](https://tauri.app/) | 2.10.3 | 跨平台桌面应用框架 |
| [Rust](https://www.rust-lang.org/) | 1.x | 后端逻辑实现 |
| [HTML/CSS/JavaScript](https://developer.mozilla.org/) | - | 前端界面 |
| [Vite](https://vitejs.dev/) | 5.4.21 | 前端构建工具 |

### 项目结构

```
xxgcxy-wifi/
├── index.html              # 前端界面
├── package.json            # Node.js 依赖配置
├── vite.config.js          # Vite 配置
├── xywdl.ps1               # 校园网登录脚本
├── src-tauri/              # Tauri 后端目录
│   ├── Cargo.toml          # Rust 依赖配置
│   ├── tauri.conf.json     # Tauri 配置文件
│   ├── src/
│   │   ├── lib.rs          # 主要业务逻辑
│   │   └── main.rs         # 程序入口
│   └── icons/              # 应用图标资源
└── dist/                   # 前端构建输出
```

### 核心模块

#### 1. 单例检测 (`check_single_instance`)
使用 Windows Mutex 确保同一时间只有一个程序实例运行。

#### 2. WiFi 管理
- `scan_wifi()` - 使用 `netsh wlan show networks` 扫描可用 WiFi
- `connect_wifi()` - 使用 `netsh wlan connect` 连接指定 WiFi
- `get_connected_wifi()` - 使用 `netsh wlan show interfaces` 获取当前连接

#### 3. 网络检测 (`check_network`)
通过 HTTP 请求检测网络状态：
- 优先访问 `https://example.com/`
- 检测是否被重定向到校园网登录页面
- 识别常见的校园网认证系统（drcom、eportal、srun 等）

#### 4. 系统托盘
- 创建托盘图标和右键菜单
- 窗口关闭时隐藏而非退出
- 支持托盘菜单操作

#### 5. 开机自启动
通过 Windows 注册表实现：
```
HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run
```

### 编译构建

#### 开发环境要求
- Rust 1.70+
- Node.js 18+
- Windows 10/11

#### 构建步骤

```bash
# 安装依赖
npm install

# 开发模式运行
npm run tauri dev

# 构建生产版本
npm run tauri build
```

构建产物位于 `src-tauri/target/release/bundle/` 目录：
- `msi/xxgcxy-wifi_1.0.0_x64_zh-CN.msi` - MSI 安装包（中文界面）
- `nsis/xxgcxy-wifi_1.0.0_x64-setup.exe` - NSIS 安装包（中文界面）

### Tauri 配置说明

```json
{
  "productName": "xxgcxy-wifi",
  "identifier": "com.xxgcxy.wifi",
  "bundle": {
    "windows": {
      "wix": { "language": "zh-CN" },
      "nsis": { "languages": ["SimpChinese"] }
    },
    "resources": ["../xywdl.ps1"]
  }
}
```

## 注意事项

1. **登录脚本安全** - `xywdl.ps1` 包含您的账号密码，请妥善保管
2. **网络检测间隔** - 建议设置 10-60 秒，过短可能影响性能
3. **首次运行** - 需要先配置 WiFi 网络才能正常使用
4. **管理员权限** - 连接某些 WiFi 可能需要管理员权限

## 许可证

MIT License

## 作者

[Thatgfsj](https://github.com/Thatgfsj)

---

如有问题或建议，欢迎提交 [Issue](https://github.com/Thatgfsj/XXGCXY-CampusNet-AutoLogin/issues)
