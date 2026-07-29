# 校园网自动重连工具 - 规范文档

## 1. 项目概述

- **项目名称**: 校园网自动重连工具 (CampusNet Auto-Reconnect)
- **项目类型**: Windows桌面应用程序 (Rust + Tauri)
- **核心功能**: 校园网环境下自动检测断线并快速重连，支持自动登录校园网portal
- **目标用户**: 校园网用户

## 2. 功能规范

### 2.1 核心功能

1. **后台运行**
   - 双击启动时隐藏主窗口，后台运行
   - 系统托盘图标显示状态
   - 右键托盘图标可显示/隐藏主窗口

2. **WiFi网络管理**
   - 扫描可用的WiFi网络列表
   - 用户选择要连接的主网络和备用网络
   - 配置保存为JSON格式（ssid配置）
   - 支持多个网络配置（主网络 + 备用网络）

3. **网络状态监控**
   - 每15秒检测一次网络连通性
   - 使用 HTTP 重定向检测：https://example.com/ 和 http://connect.rom.miui.com/generate_204
   - 检测WiFi是否已连接

4. **自动重连机制**
   - 断网时自动尝试重新连接WiFi
   - WiFi连接成功后，如果无法上网，自动运行xywdl.ps1登录校园网

5. **自动登录**
   - 调用 PowerShell 脚本 `xywdl.ps1` 进行校园网认证
   - 登录失败时记录日志

### 2.2 用户交互

1. **首次运行**
   - 弹出主窗口，显示可用WiFi列表
   - 用户选择主网络和备用网络
   - 保存配置到工作目录

2. **正常运行**
   - 托盘图标显示连接状态
   - 右键菜单：显示窗口、立即检测、退出

3. **托盘菜单**
   - 显示窗口
   - 立即检测网络
   - 退出程序

## 3. 技术规范

### 3.1 技术栈

- **框架**: Tauri 2.x (Rust后端 + Web前端)
- **前端**: HTML/CSS/JavaScript (中文界面)
- **后端**: Rust
- **依赖**:
  - `windows` crate: Windows API调用
  - `ping` crate: ICMP ping
  - `serde_json`: JSON序列化
  - `tokio`: 异步运行时

### 3.2.x 校园网信息展示(v1.8.2+)

- 在"网络配置"窗口中展示校园网登录信息:
  - **学号**:从 `%APPDATA%/xxgc_campus_net_config.txt` 的 `UserId` 字段按 `@` 拆出 (v1.9.0+ 改为从 `login_profile.json` 的 `user_id` 字段读取)
  - **运营商**:后缀映射 — `@xxgcyd`=移动、`@xxgclt`=联通、`@xxgcdx`=电信
- 提供"清理校园网信息"按钮,带二次确认,删除登录配置后下次需要重新运行登录脚本
- 新增 Tauri 命令:
  - `load_campus_net_info` → `CampusNetInfo { configured, student_id, operator, ssid }`
  - `clear_campus_net_info` → `Result<()>`

### 3.2.y 登录模块解耦 (v1.9.0+)

- **目标**:把硬编码在 `xywdl.ps1` 中的登录逻辑抽离,改为 JSON 模板 + 渲染器模式,小白用户也能在 UI 里自助配置。
- **新数据源**:
  - `%APPDATA%/xxgcxy-wifi/login_profile.json` —— 非敏感元数据
  - `%APPDATA%/xxgcxy-wifi/login_credential.bin` —— DPAPI 加密的密码
- **新 UI 屏 `#loginConfigScreen`**:
  - 运营商下拉(移动/联通/电信)
  - 学号(纯数字校验,自动拼接 `@xxgcyd/xxgclt/xxgcdx` 后缀)
  - 密码(DOM 不缓存,保存后清空)
  - Portal URL + 解析按钮(粘贴 portal.do 重定向 URL 自动填表)
  - 高级字段折叠(SSID/AC 名称/AC IP/VLAN/MAC/主机名)
  - 「保存」/「保存并登录」/「取消」按钮
- **首次启动**:自动弹出登录配置屏(有"稍后"按钮可跳过)
- **之后入口**:"设置"页(原"网络配置"页 v1.9.0+ 改名为"设置")→ 校园网信息卡片 → "更改账号信息"按钮
- **主页不再有独立的"登录（更换）按钮**,所有账号管理统一走"设置"页
- **新 Tauri 命令**:
  - `is_login_configured` → `bool`
  - `get_login_profile` → `Result<LoginProfile>`
  - `save_login_profile(profile, password)` → `Result<()>`
  - `clear_login_profile` → `Result<()>`
  - `parse_portal_url(url)` → `Result<ParsedPortal>`
  - `run_login_with_profile` → `Result<String>`
- **xywdl.ps1 简化**:从 604 行的 6 个类改为 ~280 行的函数式脚本,只读 JSON + DPAPI 解密 + 发请求。
- **DPAPI 链路**:Rust 端 `CryptProtectData`(无 entropy)→ `[b"DPAPI" magic + u32 LE 长度 + 密文]`;PS 端 `ProtectedData.Unprotect($null, CurrentUser)` 解密。
- **不兼容旧 `%APPDATA%/xxgc_campus_net_config.txt`**:首次启动会引导用户重新配置,`clear_campus_net_info` 会同时清理新旧文件。

### 3.3 数据存储

**WiFi 配置**(`config.json`):
```json
{
  "primary_ssid": "主网络名称",
  "backup_ssid": "备用网络名称",
  "check_interval": 15,
  "test_hosts": ["https://example.com/", "http://connect.rom.miui.com/generate_204"]
}
```

**校园网登录配置 (v1.9.0+,login_profile.json)**:
```json
{
  "user_id": "2021110101@xxgcyd",
  "operator": "yd",
  "ssid": "XXGC-Student",
  "base_url": "http://172.18.252.12:6060/portal.do",
  "wlan_ac_name": "XXGC-AC-01",
  "wlan_ac_ip": "172.18.252.12",
  "vlan": "1050",
  "wlan_user_ip": "",
  "mac_address": "aa:bb:cc:dd:ee:ff",
  "portal_page_id": "3",
  "portal_type": "0",
  "version": "0",
  "bind_ctrl_id": "",
  "hostname": "",
  "updated_at": "2026-07-29T12:00:00Z"
}
```

**校园网密码 (v1.9.0+,login_credential.bin)**:
二进制格式: `b"DPAPI" (4 字节 magic) + u32 LE 长度 (4 字节) + CryptProtectData 输出`。Windows DPAPI CurrentUser scope 加密,仅本机本用户可解密。

### 3.4 文件结构

```
wifi/
├── xywdl.ps1          # 校园网登录脚本
├── config.json        # 网络配置
├── SPEC.md           # 本规范文档
└── src/              # Tauri源码
```

## 4. 验收标准

1. ✅ 双击运行后程序在后台启动，不显示窗口
2. ✅ 系统托盘显示图标，右键可打开主窗口
3. ✅ 首次运行时显示WiFi选择界面
4. ✅ 配置保存为JSON格式
5. ✅ 支持主网络和备用网络配置
6. ✅ 每15秒自动检测网络连通性
7. ✅ 断网后自动重连WiFi
8. ✅ WiFi连接后无法上网时自动运行xywdl.ps1
9. ✅ 中文界面，无乱码问题
10. ✅ 程序图标正常显示
11. ✅ "网络配置"窗口展示校园网学号与运营商
12. ✅ "清理校园网信息"按钮可一键删除登录配置
13. ✅ **(v1.9.0+)** 首次启动自动弹出登录配置屏,小白用户也能自助配置账号
14. ✅ **(v1.9.0+)** 主页不再有独立的"登录（更换）"按钮;所有账号管理通过"设置"页(原"网络配置"页 v1.9.0+ 改名为"设置")的"更改账号信息"按钮进入
15. ✅ **(v1.9.0+)** 登录配置从硬编码改为 JSON 模板 + DPAPI 加密
16. ✅ **(v1.9.0+)** Portal URL 一键解析自动填充 SSID/AC/VLAN/MAC 等字段
17. ✅ **(v1.9.0+)** xywdl.ps1 大幅简化,只读 JSON + 解密 + 发请求
