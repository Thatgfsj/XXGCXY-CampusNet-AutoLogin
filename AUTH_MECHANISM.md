# 校园网 Web 认证（Portal Authentication）机制详解

> 本文档解析xxgcxy校园网的 Portal 认证协议、客户端自动化实现原理及关键设计决策。

---

## 1. 什么是 Portal 认证

**Portal Authentication（Web 认证）** 是高校通用的网络准入方案：

1. 设备连接 WiFi → DHCP 分配 IP → **默认无法访问外网**
2. 浏览器访问任意 HTTP 网站 → **AC 劫持请求并 302 重定向**到认证服务器 Portal 页面
3. 用户提交凭证 → 服务器验证 → 将该设备 MAC/IP 加入放行列表 → 可以上网

底层依赖 **AC + RADIUS** 协同，但客户端只需关注一件事：**向认证服务器发一个携带正确参数的 HTTP GET 请求**。

---

## 2. 认证流程

```
客户端                    AC                    Portal Server           RADIUS
  │                        │                         │                    │
  │──关联WiFi──────────────>│                         │                    │
  │<─DHCP分配IP────────────│                         │                    │
  │──访问任意HTTP──────────>│                         │                    │
  │                        │──劫持302重定向──────────>│                    │
  │<─Portal页面(URL含IP/MAC/VLAN等参数)───────────────│                    │
  │──GET认证请求(凭证+设备参数)───────────────────────>│                    │
  │                        │                         │──RADIUS验证──────>│
  │                        │                         │<─验证结果─────────│
  │<─认证结果────────────────────────────────────────│                    │
```

**本项目做的事**：跳过浏览器交互环节，直接构造 GET 请求发往认证服务器。

---

## 3. 协议分析

### 3.1 请求基本信息

| 属性 | 值 |
|------|-----|
| **端点** | `http://<认证服务器>:6060/quickauth.do` |
| **方法** | `GET`，参数通过 Query String 传递 |

### 3.2 参数分类

#### 用户凭证

| 参数 | 格式 | 示例 |
|------|------|------|
| `userid` | `[学号]@[运营商后缀]` | `20210101001@xxgcyd` |
| `passwd` | 明文（需 URL 编码） | `mypassword123` |

运营商后缀：移动 `@xxgcyd` / 联通 `@xxgclt` / 电信 `@xxgcdx`

#### 设备参数（动态获取）

| 参数 | 说明 | 来源 |
|------|------|------|
| `wlanuserip` | 客户端 IPv4 | 网卡查询 / URL 提取 |
| `mac` | 无线网卡 MAC | 网卡查询 / URL 提取 |
| `vlan` | 虚拟局域网 ID | **只能从重定向 URL 提取** |
| `hostname` | 主机名 | `$env:COMPUTERNAME` |

#### 接入点固定参数

| 参数 | 典型值 | 说明 |
|------|--------|------|
| `wlanacname` | `XXGC-AC-01` | AC 名称 |
| `wlanacIp` | `172.18.252.1` | AC IP |
| `portalpageid` | `3` | 门户页面 ID |
| `portaltype` | `0` | 门户类型 |
| `version` | `0` | 协议版本 |
| `bindCtrlId` | (空) | 绑定控制 ID |

#### 唯一性参数（每次请求生成）

| 参数 | 生成方式 |
|------|----------|
| `uuid` | `[guid]::NewGuid()` |
| `timestamp` | 毫秒级 Unix 时间戳 |

### 3.3 完整请求示例

```
GET http://172.18.252.12:6060/quickauth.do?userid=20210101001%40xxgcyd&passwd=xxx
  &wlanuserip=10.10.50.100&wlanacname=XXGC-AC-01&wlanacIp=172.18.252.1
  &vlan=1050&mac=aa:bb:cc:dd:ee:ff&version=0&portalpageid=3&timestamp=1680000000000
  &uuid=xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx&portaltype=0&hostname=MYPC&bindCtrlId=
```

---

## 4. 动态参数获取

### 4.1 URL 解析 vs 系统查询

| 方案 | 做法 | 致命问题 |
|------|------|----------|
| 系统查询 | `Get-NetAdapter` / `Get-NetIPAddress` | **VLAN 无法通过本地查询获取**；虚拟机网卡产生虚假结果 |
| **URL 解析** ✓ | 解析 AC 重定向 URL 中的 Query String | 需成功捕获重定向 |

**结论**：URL 解析为主（提取 BaseURL/VLAN/MAC），系统查询兜底（IP 地址补全）。

### 4.2 重定向 URL 结构

```
http://172.18.252.12:6060/portal.do?wlanuserip=10.10.50.100&wlanacname=XXGC-AC-01
  &wlanacIp=172.18.252.1&mac=AA:BB:CC:DD:EE:FF&vlan=1050&hostname=MYPC&rand=123456
```

解析逻辑：
1. 正则提取 `http://<host>/xxx.do` → 替换 `xxx.do` 为 `quickauth.do`
2. 按 `&` 拆 Query String → 按 `=` 拆键值对 → 映射到内部字段
3. MAC 标准化为小写冒号格式

### 4.3 自动检测（两级回退）

```
① GET http://www.qq.com (MaxRedirect=0)
   └─ 若 302 → 从 Location 解析参数

② GET http://172.18.252.12:6060 (MaxRedirect=0)
   └─ 若 302 → 解析后用本地 IP/MAC 补全

③ 手动粘贴 Portal URL
```

### 4.4 虚拟机网卡过滤

虚拟机网卡（VirtualBox/VMware/Hyper-V）的 IP 格式与校园网汇聚层相似，会干扰 `wlanuserip` 获取。解决方案：`Get-NetAdapter` 过滤 `InterfaceDescription` 匹配 `Wi-Fi|Wireless|WLAN` 且 `Name` 不包含 `Virtual|VMware|Hyper-V|VirtualBox` 的活跃网卡。

---

## 5. 脚本实现

### 5.1 核心思路

放弃浏览器模拟（页面渲染 / Cookie / JS），仅用 `Invoke-WebRequest` 直接发 GET 请求。一次请求 → 一次响应解析。

### 5.2 配置管理

所有参数封装在 `NetworkConfig` 类中，固定参数通过 JSON 持久化。

### 5.3 标准化处理

- `[Uri]::EscapeDataString()` 对含特殊字符的参数做 URL 编码
- 每次请求生成新 UUID 和毫秒级时间戳

### 5.4 异常覆盖

超时、DNS 失败、SSL 错误、4xx/5xx → 全链路捕获，输出结构化错误信息。

---

## 6. 安全设计

### 6.1 DPAPI 加密

密码不落盘明文。存入 JSON 前经 Windows DPAPI 加密，解密后仅在内存中以 `SecureString` 形式短暂存在。

```
输入 (Read-Host -AsSecureString)
  → ConvertFrom-SecureString (DPAPI 加密)
  → Base64 密文写入 JSON

读取 (ConvertTo-SecureString)
  → Marshal.SecureStringToBSTR (还原明文到内存)
  → 使用后立即 FreeBSTR
```

### 6.2 文件保护

- 路径：`$env:APPDATA\xxgc_campus_net_config.txt`（隐藏属性）
- DPAPI 绑定当前用户 + 机器，脱离原环境无法解密

---

## 7. 认证响应码

| code | 含义 |
|------|------|
| `0` | 成功 |
| `1` | 账号不存在（检查学号和运营商） |
| `44` | 非法接入（检查 VLAN/MAC） |

兼容旧版非 JSON 响应：关键字匹配 `success` / `认证成功` / `账号不存在` / `非法接入`。

---

## 8. 连通性验证 —— 为什么用 204？

### 8.1 问题

认证服务器返回 `code: 0` 只说明**凭证校验通过**，不代表设备已有外网权限。AC 放行可能延迟、IP 绑定可能冲突。因此认证后必须验证外网连通性。

### 8.2 为什么 HTTP 200 不可靠

校园网 AC 在未认证状态下会劫持所有 HTTP 请求，**返回的 Portal 登录页面本身也是 HTTP 200**。用 `status == 200` 判断"已上网"无法区分"Portal 假页面"和"真实网站"。

### 8.3 Captive Portal Detection

业界的标准方案：请求一个**已知响应特征**的外部 URL，比对实际响应与预期。

| 系统 | 检测 URL | 预期响应 |
|------|----------|----------|
| Android | `connectivitycheck.gstatic.com/generate_204` | 204 |
| Apple | `captive.apple.com/hotspot-detect.html` | 200 + `Success` |
| Windows | `msftconnecttest.com/connecttest.txt` | 200 + `Microsoft Connect Test` |

### 8.4 本项目方案

使用 `http://connect.rom.miui.com/generate_204`（国内 CDN，延迟低），三层次判断：

```
第一层：302 重定向
  Location 含 portal/drcom/inode/eportal/srun/authserv → ❌ 未认证
  其他跳转 → ✅ 已连通

第二层：204 No Content
  HTTP 204 → ✅ 已连通（AC 绝不会返回 204，只有真实服务器才会）

第三层：200 正文匹配
  正文含 drcom/校园网认证/portal认证 → ❌ 未认证
  正文含 百度一下/baidu → ✅ 已连通
  其他 → ✅ 已连通（保守策略）
```

**为什么 204 可靠？** 校园网 AC 劫持请求时只会返回 HTTP 200（Portal 页面）或 302（重定向），从不返回 204。204 是应用层约定状态码，只有目标服务器才会产生。收到 204 = 100% 确认请求到达了真实外网服务器。

完整检测流程：

```
① GET https://example.com/ (no redirect)
   ├─ 204/非Portal跳转 → ✅
   └─ Portal跳转/失败 → ②

② GET http://connect.rom.miui.com/generate_204 (no redirect)
   ├─ 204 → ✅
   ├─ 302+Portal关键词 → ❌
   ├─ 200+Portal关键词 → ❌
   └─ 其他 → ✅
```

---

## 附录：关键代码位置

| 模块 | 文件 |
|------|------|
| 认证客户端（流程编排） | `xywdl.ps1` → `AuthenticationClient` |
| 请求参数模型 | `xywdl.ps1` → `NetworkConfig` |
| URL 解析 | `xywdl.ps1` → `RedirectUrlParser` |
| 网卡信息获取 | `xywdl.ps1` → `NetworkInterfaceHelper` |
| 凭证加密存储 | `xywdl.ps1` → `ConfigManager` |
| 连通性检测 | `src-tauri/src/lib.rs` → `check_url()` |
| 运营商配置 | `xywdl.ps1` → `DomainConfig` |
| 前端界面 | `index.html` |
