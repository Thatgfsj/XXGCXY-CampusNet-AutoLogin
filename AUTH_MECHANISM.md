# 校园网 Web 认证（Portal Authentication）机制详解

> 本文档详细解析新乡工程学院校园网的认证协议、请求结构，以及本项目中 PowerShell 脚本的自动化实现原理。

---

## 目录

1. [什么是 Portal 认证](#1-什么是-portal-认证)
2. [认证流程全景](#2-认证流程全景)
3. [协议分析 —— 请求详解](#3-协议分析--请求详解)
4. [动态参数获取策略](#4-动态参数获取策略)
5. [脚本实现原理](#5-脚本实现原理)
6. [安全设计 —— 凭证管理](#6-安全设计--凭证管理)
7. [响应码语义化处理](#7-响应码语义化处理)
8. [方案演进：从 Python 到 PowerShell](#8-方案演进从-python-到-powershell)

---

## 1. 什么是 Portal 认证

**Portal Authentication（Web 认证）** 是目前国内高校广泛使用的一种网络准入控制方案。它的核心特征是：

- 用户连接 WiFi 后，设备已分配到 IP 地址，但**默认无法访问外网**
- 当用户打开浏览器访问任意 HTTP 网站时，**AC（无线接入控制器）会劫持该请求并 302 重定向**到认证服务器的 Portal 页面
- 用户在 Portal 页面输入账号密码，认证服务器验证通过后，将该设备的 MAC/IP 加入放行列表，设备获得外网访问权限

这种机制的底层依赖是 **AC + RADIUS 服务器** 的协同工作，但客户端只需要关注一个环节：**向认证服务器发送一个携带正确参数的 HTTP GET 请求**。

> **关键洞察**：整个认证过程本质上就是一次 HTTP GET 请求。浏览器渲染的登录页面只是"皮囊"，核心是请求本身。这也是本项目能够用纯脚本替代浏览器的理论基础。

---

## 2. 认证流程全景

```
┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐
│  客户端   │     │    AC    │     │ Portal   │     │  RADIUS  │
│ (你的电脑) │     │ (接入层)  │     │  Server  │     │  Server  │
└────┬─────┘     └────┬─────┘     └────┬─────┘     └────┬─────┘
     │                │               │               │
     │ 1. 关联WiFi    │               │               │
     │───────────────>│               │               │
     │                │               │               │
     │ 2. DHCP分配IP  │               │               │
     │<───────────────│               │               │
     │                │               │               │
     │ 3. 访问任意HTTP网站             │               │
     │───────────────>│               │               │
     │                │ 4. 劫持+302重定向到Portal      │
     │                │──────────────>│               │
     │                │               │               │
     │ 5. Portal页面（含wlanuserip/mac/vlan等参数在URL中）│
     │<───────────────────────────────│               │
     │                │               │               │
     │ 6. GET 认证请求（携带用户名/密码/IP/MAC等）     │
     │──────────────────────────────>│               │
     │                │               │ 7. RADIUS验证  │
     │                │               │──────────────>│
     │                │               │ 8. 验证结果    │
     │                │               │<──────────────│
     │                │               │               │
     │ 9. 认证结果返回 │               │               │
     │<───────────────────────────────│               │
     │                │               │               │
     │ 10. 上网！    │               │               │
```

**本项目做的事情**：跳过步骤 3~5 的浏览器交互，直接从步骤 6 开始——构造 HTTP GET 请求直连认证服务器。

---

## 3. 协议分析 —— 请求详解

### 3.1 请求基本信息

| 属性 | 值 |
|------|-----|
| **目标服务器** | `http://<学校认证服务器>:6060/quickauth.do` |
| **请求方法** | `GET` |
| **Content-Type** | 无需（参数通过 Query String 传递） |

### 3.2 查询字符串参数详解

认证请求的 Query String 包含以下参数，分为四类：

#### 第一类：用户凭证

| 参数 | 格式 | 说明 | 示例 |
|------|------|------|------|
| `userid` | `[学号]@[运营商后缀]` | 用户身份标识 | `20210101001@xxgcyd` |
| `passwd` | 明文（需 URL 编码） | 校园网密码 | `mypassword123` |

**运营商后缀对照表：**

| 运营商 | 后缀 | 选项编号 |
|--------|------|----------|
| 移动 | `@xxgcyd` | 1 |
| 联通 | `@xxgclt` | 2 |
| 电信 | `@xxgcdx` | 3 |

#### 第二类：设备与网络环境参数（动态变化）

| 参数 | 说明 | 来源 |
|------|------|------|
| `wlanuserip` | 客户端当前分配的 IPv4 地址 | 动态获取 |
| `mac` | 客户端无线网卡 MAC 地址 | 动态获取 |
| `vlan` | 客户端所属虚拟局域网 ID | 重定向 URL 中提取 |
| `hostname` | 客户端主机名 | `$env:COMPUTERNAME` |

#### 第三类：接入点固定参数（同一校区不变）

| 参数 | 说明 | 性质 |
|------|------|------|
| `wlanacname` | AC 名称 | 固定 |
| `wlanacIp` | AC IP 地址 | 固定 |
| `ssid` | WiFi SSID | 固定（指定SSID连接时） |
| `version` | 协议版本号 | 固定（`"0"`） |
| `portalpageid` | 门户页面 ID | 固定（`"3"`） |
| `portaltype` | 门户类型 | 固定（`"0"`） |
| `bindCtrlId` | 绑定控制 ID | 固定（通常为空） |

#### 第四类：请求唯一性参数

| 参数 | 说明 | 生成方式 |
|------|------|----------|
| `uuid` | 通用唯一标识符 | `[guid]::NewGuid()` |
| `timestamp` | 毫秒级 Unix 时间戳 | `Get-Date -UFormat %s` × 1000 |

### 3.3 完整请求示例

```
GET http://172.18.252.12:6060/quickauth.do?userid=20210101001%40xxgcyd&passwd=mypassword123&wlanuserip=10.10.50.100&wlanacname=XXGC-AC-01&wlanacIp=172.18.252.1&ssid=XXGC-WiFi&vlan=1050&mac=aa%3Abb%3Acc%3Add%3Aee%3Aff&version=0&portalpageid=3&timestamp=1680000000000&uuid=xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx&portaltype=0&hostname=MYPC&bindCtrlId=
```

---

## 4. 动态参数获取策略

### 4.1 为什么选择 URL 解析方案

动态参数（IP、MAC、VLAN）的获取是实现自动化的关键障碍。我们尝试了两种方案：

| 方案 | 方法 | 问题 |
|------|------|------|
| **方案 A：系统查询** | 使用 `Get-NetAdapter`、`Get-NetIPAddress` 等 cmdlet 直接查询 | VLAN 无法通过本地系统查询获取；虚拟机虚拟网卡干扰（虚拟 IP 与学校汇聚层格式相似，导致误判） |
| **方案 B：URL 解析** ✓ | 解析 AC 重定向的 Portal URL，从中提取全部参数 | 依赖于捕获到正确的重定向 URL |

**最终方案**：以 URL 解析为主（自动提取 BaseURL、WlanAcName、VLAN、MAC 等），以系统查询为辅（IP 地址兜底，避免 URL 中 IP 为 0.0.0.0 的边界情况）。

### 4.2 重定向 URL 解析机制（`RedirectUrlParser`）

Portal 重定向 URL 的典型格式：

```
http://172.18.252.12:6060/portal.do?wlanuserip=10.10.50.100&wlanacname=XXGC-AC-01&wlanacIp=172.18.252.1&mac=AA:BB:CC:DD:EE:FF&vlan=1050&hostname=MYPC&rand=123456
```

解析步骤：
1. **提取 BaseURL** — 正则匹配 `http://<host>/xxx.do`，后续将 `xxx.do` 替换为 `quickauth.do` 得到真正的认证请求地址
2. **拆分 Query String** — 按 `&` 分割 → 按 `=` 分割提取键值对
3. **字段映射** — 将 URL 参数名映射为内部配置字段名
4. **MAC 标准化** — 统一转换为小写冒号格式（`aa:bb:cc:dd:ee:ff`）

### 4.3 自动检测流程

脚本启动后的自动检测采用**两级回退**策略：

```
方法1: Invoke-WebRequest http://www.qq.com -MaximumRedirection 0
       └── 若 AC 返回 302 Location → 从 Location 头提取重定向 URL → 解析参数

方法2 (回退): Invoke-WebRequest http://172.18.252.12:6060 -MaximumRedirection 0
       └── 若 AC 返回 302 Location → 解析参数后，用本地 IP/MAC 兜底补全

方法3 (手动): 用户复制浏览器地址栏中的 Portal URL 粘贴到终端
```

### 4.4 虚拟机干扰问题的解决

在系统查询方案（方案 A）中，虚拟机虚拟网卡的 IP 地址格式与学校网络汇聚层相似，导致脚本误将虚拟 IP 作为 `wlanuserip` 发送。

**解决方法**：在 `Get-NetAdapter` 筛选中加入黑名单过滤：
- `Virtual` — VirtualBox 虚拟网卡
- `VMware` — VMware 虚拟网卡
- `Hyper-V` — Hyper-V 虚拟交换机
- `VirtualBox`

只保留 `InterfaceDescription` 匹配 `Wi-Fi|Wireless|WLAN` 且 `Status` 为 `Up` 的真实物理网卡。

---

## 5. 脚本实现原理

### 5.1 核心通信 —— 跳过浏览器渲染

传统 Python 方案使用 `requests` + `BeautifulSoup` 模拟浏览器完整流程（页面加载 → Cookie 管理 → 表单提交），每个环节都可能成为故障点。

本项目直接使用 `Invoke-WebRequest`（PowerShell 原生 cmdlet）构造 HTTP GET 请求，跳过：
- 页面渲染
- Cookie 校验
- JavaScript 执行
- 前端表单提交逻辑

**仅保留核心**：一次 GET 请求 → 一次响应解析。

### 5.2 配置管理 —— 结构化分离

所有参数封装在 `NetworkConfig` 类中，与业务逻辑解耦。固定参数（BaseURL、AC 信息等）通过 JSON 配置文件持久化，避免硬编码散落在代码各处。

### 5.3 请求标准化

为防止请求解析异常，脚本对参数做严格标准化：
- **URL 编码** — 用户名、密码、主机名、AC 名称等含特殊字符的参数，通过 `[Uri]::EscapeDataString()` 编码
- **UUID 生成** — 每次请求生成唯一 UUID，满足服务器唯一性校验
- **毫秒级时间戳** — 防止服务器端请求去重

### 5.4 高容错处理

认证请求全链路覆盖异常捕获：
- 网络层异常 — 超时、DNS 失败、SSL 错误
- HTTP 层异常 — 4xx/5xx 状态码
- 语义化结果解析 — 基于返回内容中的 `code` 字段做精准判断

---

## 6. 安全设计 —— 凭证管理

### 6.1 加密存储

密码**不存明文**。使用 Windows DPAPI（数据保护 API）进行加密：

```
用户输入密码 (SecureString)
    ↓
ConvertTo-SecureString (AsPlainText)
    ↓
ConvertFrom-SecureString (DPAPI 加密)
    ↓
Base64 编码字符串写入 JSON 配置文件
```

读取时反向操作：

```
JSON 文件中的 Base64 密文
    ↓
ConvertTo-SecureString (解密)
    ↓
Marshal.SecureStringToBSTR (还原明文，仅存在于内存)
```

### 6.2 文件保护

配置文件采取多层保护：
- **隐藏属性** — `[System.IO.FileAttributes]::Hidden`
- **存储路径** — `$env:APPDATA\xxgc_campus_net_config.txt`（用户 AppData 目录）
- **DPAPI 绑定** — 加密后的密文仅能由当前用户在当前机器上解密（DPAPI 默认绑定 user+machine）

### 6.3 密码输入

`Read-Host -AsSecureString` — 密码输入时终端不回显，直接进入 `SecureString` 对象，明文字符串仅在内存中短暂存在并被立即释放。

---

## 7. 响应码语义化处理

服务器返回的 JSON 响应中，`code` 字段代表认证结果：

| code | 含义 | 脚本处理 |
|------|------|----------|
| `0` | 认证成功 | 输出成功提示，退出 |
| `1` | 账号不存在 | 提示检查学号和运营商选择 |
| `44` | 非法接入 | 提示检查 VLAN ID 和 MAC 地址 |

> 脚本同时兼容非 JSON 格式的旧版响应（关键字匹配 `success` / `认证成功` / `账号不存在` / `非法接入`）。

---

## 8. 方案演进：从 Python 到 PowerShell

### 8.1 为什么放弃 Python

最初使用 Python 实现，依赖库包括：
- `requests` — HTTP 通信
- `beautifulsoup4` — 页面解析
- `netifaces` / `getmac` — 网卡信息获取

**遇到的问题：**
- **体积臃肿** — Python 解释器 + 依赖库打包后体积庞大
- **环境依赖** — 需要在目标机器上安装 Python，或使用 PyInstaller 打包（触发杀软误报）
- **Windows 集成差** — Python 原生不支持 Windows DPAPI、SecureString 等安全特性，需要额外 ctypes 调用
- **浏览器模拟开销** — 模拟页面渲染、Cookie 管理等流程引入了不必要的复杂度

### 8.2 PowerShell 方案的优势

| 维度 | Python 方案 | PowerShell 方案 |
|------|-------------|-----------------|
| **环境依赖** | 需安装 Python 解释器 | Windows 系统内置（PS 5.1+） |
| **DPAPI 加密** | 需 ctypes 调用 Win32 API | 原生 `ConvertFrom-SecureString` |
| **网络命令** | `requests` 库 | 原生 `Invoke-WebRequest` |
| **网卡查询** | `netifaces` (第三方) | 原生 `Get-NetAdapter` |
| **部署体积** | ~50MB+（含解释器） | 纯脚本 <30KB |
| **安全性** | 手动实现密码保护 | `SecureString` + DPAPI 原生支持 |
| **启动速度** | Python 解释器冷启动 ~1-2s | PowerShell 即时执行 |

### 8.3 项目现状

当前项目已发展为 **Tauri 桌面应用**（Rust 后端 + Web 前端），PowerShell 脚本（`xywdl.ps1`）作为认证执行引擎被 Tauri 应用通过子进程调用，实现了：

- **GUI 操作** — 用户选择 WiFi、配置账号
- **系统托盘常驻** — 后台监控网络状态
- **自动重连** — 断网自动重连 WiFi + 重新认证
- **开机自启** — 无需用户手动操作

---

## 附录：关键代码位置

| 模块 | 文件 | 核心功能 |
|------|------|----------|
| 认证客户端 | `xywdl.ps1` → `AuthenticationClient` | 完整的登录流程编排 |
| 网络配置 | `xywdl.ps1` → `NetworkConfig` | 所有认证参数的结构化定义 |
| URL 解析器 | `xywdl.ps1` → `RedirectUrlParser` | 从重定向 URL 提取参数 |
| 网卡辅助 | `xywdl.ps1` → `NetworkInterfaceHelper` | Wi-Fi IP/MAC/SSID 获取 |
| 配置持久化 | `xywdl.ps1` → `ConfigManager` | JSON 配置的读写与加密 |
| 运营商配置 | `xywdl.ps1` → `DomainConfig` | 运营商后缀映射 |
| Tauri 后端 | `src-tauri/src/lib.rs` | 桌面应用主逻辑（Rust） |
| 前端界面 | `index.html` | Web UI（HTML/CSS/JS） |
