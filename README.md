# XXGCXY-CampusNet-AutoLogin (Linux 纯脚本版)

校园网自动登录脚本 - 适用于 Linux 的纯 Shell 脚本版本，无需图形界面。

## 功能特性

- **自动检测网络状态** - 实时监测网络连通性
- **自动登录校园网** - 自动执行校园网认证
- **配置保存** - 自动保存账号信息，下次无需重新输入
- **多运营商支持** - 支持移动、联通、电信

## 系统要求

- Linux 操作系统
- PowerShell 7 或 pwsh（[安装指南](https://docs.microsoft.com/zh-cn/powershell/scripting/install/installing-powershell-on-linux)）

### 安装 PowerShell 7

**Ubuntu/Debian:**
```bash
# 下载 Microsoft 存储库包
wget https://packages.microsoft.com/config/ubuntu/$(lsb_release -rs)/packages-microsoft-prod.deb

# 安装包
sudo dpkg -i packages-microsoft-prod.deb

# 更新并安装 PowerShell
sudo apt update && sudo apt install powershell
```

**CentOS/RHEL:**
```bash
sudo rpm --import https://packages.microsoft.com/keys/microsoft.asc
sudo curl -o /etc/yum.repos.d/microsoft.repo https://packages.microsoft.com/config/rhel/7/prod.repo
sudo yum install -y powershell
```

**Arch Linux:**
```bash
sudo pacman -S powershell-bin
```

安装完成后，运行 `pwsh` 验证。

## 使用方法

### 1. 下载脚本

```bash
wget https://raw.githubusercontent.com/Thatgfsj/XXGCXY-CampusNet-AutoLogin/linux-sh/xywdl.sh
chmod +x xywdl.sh
```

### 2. 运行脚本

```bash
./xywdl.sh
```

### 3. 首次配置

首次运行时会提示您：
1. 选择运营商（移动/联通/电信）
2. 输入学号
3. 输入校园网密码
4. 脚本会自动检测登录参数

配置信息保存在 `~/.config/xxgcxy-wifi/login_config.json`

### 4. 自动登录

下次运行时，脚本会自动读取保存的配置并登录。

## 命令行参数

```bash
./xywdl.sh          # 读取配置并自动登录
```

## 常见问题

### Q: 提示 "pwsh: command not found"
A: 请先安装 PowerShell 7，参见上文"安装 PowerShell 7"

### Q: 提示 "账号不存在"
A: 请检查学号和运营商选择是否正确

### Q: 提示 "非法接入"
A: 请检查 VLAN ID 或 MAC 地址是否正确，可能需要重新获取登录参数

### Q: 如何退出登录？
A: 在浏览器访问 http://2.2.2.2 并点击退出

## 自动化登录

可以使用 cron 实现开机自动登录：

```bash
crontab -e
```

添加以下行（每5分钟检测一次）：
```
*/5 * * * * /path/to/xywdl.sh
```

## 版本说明

| 分支 | 说明 |
|------|------|
| main | Tauri 桌面应用完整版 |
| **linux-sh** | Linux 纯脚本版（本分支） |
| win-portable | Windows 版（含内置PS7） |
| win-system-ps7 | Windows 版（需系统PS7） |

## 项目结构（Linux分支）

```
linux-sh/
├── xywdl.sh              # Linux 登录脚本（仅此文件）
└── README.md            # 本说明文件
```

## 许可证

MIT License

## 作者

[Thatgfsj](https://github.com/Thatgfsj)

---

如有问题或建议，欢迎提交 [Issue](https://github.com/Thatgfsj/XXGCXY-CampusNet-AutoLogin/issues)

> **声明**：本脚本所有经验仅供自动化协议学习研究，请遵守各高校管理相关要求。
