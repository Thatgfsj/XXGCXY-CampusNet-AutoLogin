# 校园网自动重连工具 - 规范文档

## 1. 项目概述

- **项目名称**: 校园网自动重连工具 (CampusNet Auto-Reconnect)
- **项目类型**: Windows桌面应用程序 (Rust + Tauri)
- **核心功能**: 校园网环境下自动检测断线并快速重连，支持自动登录校园网portal
- **目标用户**: 校园网用户

## 2. 功能规范

### 2.1 核心功能

1. **后台与静默运行**
   - 手动双击运行时正常展示主窗口
   - 开机自启（带 `--autostart` 标志）时初始完全静默隐藏主窗口，纯后台常驻系统托盘，不弹窗打扰
   - 系统托盘图标显示状态，右键托盘图标可显示/隐藏主窗口

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

6. **移动热点常开守护 (v2.0.8+)**
   - 突破校园网单账号/单设备在线限制
   - 采用 Windows 原生 WinRT API 实时探测移动热点状态
   - 防止因超时无设备连接被系统自动休眠关闭
   - 掉线重连与认证成功后联动拉起，保障共享网络持续不断线

7. **深色极客毛玻璃 UI (v2.0.8+)**
   - 采用 Dark Acrylic 磨砂亚克力微光画布
   - 140px 发光动态网络健康雷达仪表环（Hero Gauge），在线/待认证/掉线/探测中状态呼吸变色
   - 学生身份数字名片（Profile Card），支持移动/联通/电信品牌色彩徽章与快捷修改抽屉
   - 智能平滑滑动开关，可折叠终端等宽语义高亮日志

8. **复合账号与补卡后缀支持 (v2.0.8+)**
   - 支持字母与数字混合账号（如补卡后缀 `ls` / `lls` 或工号字母前缀）
   - 前端与后端自动剥离用户误填的 `@` 及后缀

9. **开机自启静默保活 (v2.0.8+)**
   - 注册表与桌面快捷方式注册 `--autostart` 启动参数
   - 开机启动后主窗口不展示，直接进入托盘后台工作，免除弹窗打扰

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

### 3.2.z 强健性与防御性净化规范 (v2.0.5+)

- **Portal URL 四重防御性清洗**:
  - 输入框支持 `onpaste` 粘贴自动解析，并在解析/保存时将长重定向 URL 强制提纯为纯净 BaseURL（剥除 `?` 和 `#` 及其后所有参数）。
  - 前端、Rust 后端、PowerShell、Bash 四层拦截，确保构造认证请求时始终使用纯净 `http://host:port/quickauth.do?...`，彻底杜绝双问号 `??` 导致 AC 遗失账号密码。
- **响应精准反序列化与错误透传**:
  - 优先调用 JSON 反序列化器，统一将 `code` 转为字符串处理，兼容 `"0"`/`"1"` 与纯数字 `0`/`1`。
  - 直接透传校园网 AC 服务端返回的真实中文提示（如“设备不在正常状态,无法认证上网,请稍后”），杜绝误报未知错误 99。
- **网络就绪防抢跑与编码兼容**:
  - 过滤 `169.254.*` 临时 IP，新增最多 3 秒的 DHCP 延迟重试。
  - PowerShell 脚本固化 UTF-8 BOM 规范，保证 Windows PS 5.1 解析中文时安全无截断。
- **日志组件排版优化**:
  - 前端 `addLog` 过滤空行与空白字符，去除批处理与脚本的多余空行，杜绝空白时间戳行。

### 3.3 数据存储

**WiFi 配置**(`config.json`):
```json
{
  "primary_ssid": "主网络名称",
  "backup_ssid": "备用网络名称",
  "check_interval": 15,
  "hotspot_keepalive": false,
  "test_hosts": ["https://example.com/", "http://connect.rom.miui.com/generate_204"]
}
```

**校园网登录配置 (v1.9.0+,login_profile.json)**:
```json
{
  "user_id": "2021110101@xxgcyd",
  "operator": "yd",
  "ssid": "XXGC-Student",
  "base_url": "http://172.16.x.x:6060/portal.do",
  "wlan_ac_name": "XXGC-AC-01",
  "wlan_ac_ip": "172.16.0.1",
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
18. ✅ **(v2.0.0+)** 登录请求发送多层级保底 (PowerShell → C# → Python)
19. ✅ **(v2.0.5+)** Portal URL 四重防御性净化，杜绝双问号导致账号密码丢失和 AC 报“设备不在正常状态”
20. ✅ **(v2.0.5+)** 校园网 AC 服务端错误响应精准识别，退出码准确且透传真实中文提示
21. ✅ **(v2.0.5+)** 前端日志输出过滤空行，排版紧凑工整无空白时间戳行
22. ✅ **(v2.0.8+)** 桌面端 UI 深度重构为深色极客毛玻璃风格，提供 140px 发光动态网络健康雷达仪表环、专属运营商学生名片、可折叠终端等宽日志与平滑滑动开关，保持底层 IPC 接口完全兼容
23. ✅ **(v2.0.8+)** 支持 Windows 移动热点常开守护 (Hotspot Keep-Alive)，利用原生 WinRT API 实时检测与重新唤醒，防止无设备连接自动关闭，持续共享校园网连接
24. ✅ **(v2.0.8+)** 支持字母与数字组合账号（兼容补卡后缀如 ls/lls 及前缀字母工号，输入含 @ 自动剥离），解除纯数字限制
25. ✅ **(v2.0.8+)** 开机自启动静默运行优化（注册表与 desktop 声明 --autostart，窗口初始设为不可见，开机绝不弹窗打扰用户）
26. ✅ **(v2.0.9+)** 认证极速响应与卡顿彻底根治：三层发送器统一模拟 Chrome 浏览器标头（防止 AC/WAF 防火墙意外重置连接），单层内网超时缩短至 6 秒（消除 90 秒超时假死），解除前端 30 秒登录全局锁（手动立即检测秒级唤醒），日志去重防抖与 SSID 容错匹配
