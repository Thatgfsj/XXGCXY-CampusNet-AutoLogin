# XXGCXY-CampusNet-AutoLogin

新乡工程学院校园网自动登录助手 —— 基于 Tauri 2.x 的 Windows / Linux 桌面应用，自动检测 / 重连校园网 WiFi 并完成 Portal 认证登录。

> 校园网认证机制详解讲解（感兴趣的话推荐查看）：[AUTH_MECHANISM.md](./AUTH_MECHANISM.md)
> 
> 脚本前身：https://github.com/Thatgfsj/XXGC-CampusNet-AutoLogin
> 
> 开发者文档（API、架构、问题修复史）：[JSDOC.md](./JSDOC.md)

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

| 版本 | 说明 | 内置 PS7 | 需要系统 PS 5.1+ |
|------|------|:---------:|:-------------:|
| **Windows NSIS / MSI** | Windows 安装器，体积小 | ❌ | ✅ |
| **Linux** | .deb / .rpm / tar.gz | 👻 | 👻 |

> v1.9.0+ 起不再内置 PowerShell 7 移植版。Windows 10/11 自带 PowerShell 5.1，已支持 DPAPI 加密。

## 功能

- **⚡ Rust 进程内原生直发认证 (Sub-100ms Instant Login, v2.0.9+)** — 桌面端核心认证移入 Rust 进程内，使用原生 Windows `CryptUnprotectData` API 原位解密与 `reqwest` 异步直发，彻底根除系统 PowerShell 5.1 冷启动与多轮 WMI 扫描的 9.5 秒固有等待，实现 80 毫秒以内闪电秒登；保留外部脚本无缝降级兜底
- **🚀 外部脚本冷启动与网卡探查深度瘦身 (v2.0.9+)** — 剔除 `xywdl.bat` 启动时重复拉起 PowerShell 查询版本的 1.75 秒冗余空转；为 `xywdl.ps1` 引入网卡对象单例缓存，消除单次认证重复扫描 3 轮 WMI 的 1.5 秒开销
- **🔥 保持移动热点开启守护 (v2.0.8+)** — 突破校园网单账号/单设备限制，针对学生开电脑热点供手机/平板共享上网的痛点，采用 Windows 原生 WinRT 实时守护热点，防无设备连接超时休眠自动关闭，断网自愈后联动唤醒
- **🎨 深色极客毛玻璃 UI (v2.0.8+)** — 全新 Dark Acrylic 磨砂亚克力微光画布，140px 发光动态网络健康雷达仪表环，学生数字身份名片（支持移动/联通/电信品牌专属色彩徽章与快捷抽屉），可折叠等宽终端动态日志
- **🚀 兼容字母学号与补卡后缀 (v2.0.8+)** — 解除纯数字限制，全面支持字母与数字组合账号（如补卡后缀 `ls` / `lls` 及前缀字母工号），输入误带 `@` 自动智能剥离
- **🤫 开机静默自启防打扰 (v2.0.8+)** — 注册表 `--autostart` 标志联动原生窗口初始隐藏，重启电脑开机后彻底静默驻留系统托盘，绝无弹窗闪烁打扰
- **可视化登录配置 (v1.9.0+)** — 首次启动弹窗引导,运营商/学号/密码/Portal URL 表单填写,小白也能自助配置;"设置"页(原"网络配置"页)有"更改账号信息"按钮可随时再改
- **Portal URL 智能清洗与防双问号 (v2.0.5+)** — 粘贴重定向长链接自动提纯 BaseURL 并抽取网络参数；四重防御性净化杜绝双问号导致的学号密码丢失与“设备不在正常状态”报错
- **JSON 响应精准判定与真实提示透传 (v2.0.5+)** — 优先采用 JSON 解析器统一处理字符/数字 code，直接展示 AC 服务端真实中文提示，彻底告别“未知错误 99”
- **纯净日志排版 (v2.0.5+)** — 消除前端多余空白时间戳行，输出紧凑清晰
- **请求发送多层级保底 (v2.0.0+)** — 登录请求发送失败自动降级: Windows 依次尝试 PowerShell → C# → Python;Linux 依次尝试 curl → Python。任一层成功即可完成认证
- **自动检测网络状态** — 实时监测 WiFi 连接和互联网访问
- **自动重连 WiFi** — 断网时自动连接预设的 WiFi
- **自动登录校园网** — 连接后自动执行认证脚本
- **系统托盘运行** — 最小化到托盘,后台静默运行
- **开机自启** — 可选的开机自动启动
- **凭证加密存储** — 密码通过 Windows DPAPI 加密, 不存明文
- **中文界面** — 安装包和 UI 均支持简体中文

## 安装

### Windows（推荐）

1. 下载最新的 `xxgcxy-wifi_x.x.x_x64-setup.exe`
2. 双击运行，按提示完成安装
3. 首次运行需配置 WiFi 和账号信息

**系统要求**:
- **Windows 10 / 11**: 自带 PowerShell 5.1 + .NET Framework 4.x，**无需任何额外操作**（开箱即用）
- **Windows 8 / 8.1 / Server 2012-2012R2**: 自带 PowerShell 4.0，需升级到 PowerShell 5.1
  - 下载 WMF 5.1: <https://www.microsoft.com/en-us/download/details.aspx?id=54616>
- **Windows 7 SP1 / Server 2008 R2 SP1**: 自带 PowerShell 2.0，**必须**手动安装 WMF 5.1
  - 下载 WMF 5.1: <https://www.microsoft.com/en-us/download/details.aspx?id=54616>
  - 安装后重启
- **Windows XP / Vista / Server 2003/2008**: **不支持**（太老）

> 自 v1.9.0 起不再内置 PowerShell 7 移植版。Windows 自带的 PowerShell 5.1+ 完整支持 DPAPI `ProtectedData`（.NET Framework 4.x）。v2.0.0 起登录请求发送带多层级降级（PowerShell → C# → Python），任一层可用即可认证，脚本已做 Win 7/8 兼容（WMI fallback 处理 Get-NetAdapter 不存在的问题）。

### Linux

```bash
chmod +x xywdl.sh
./xywdl.sh     # 需要 curl 或 python3 (登录请求发送会自动选择可用的一层)
```

## 项目结构

```
├── index.html              # 前端界面 (HTML/CSS/JS)
├── package.json            # Node.js 依赖
├── xywdl.ps1               # 校园网认证脚本 (PowerShell)
├── xywdl.bat               # Windows 启动器（找系统 pwsh / powershell）
├── xywdl.sh                # Linux 启动脚本
├── src/sender/             # 请求发送保底层 (v2.0.0+)
│   ├── sender.cs           # C# 源码 (预编译为 xywdl_sender.exe)
│   ├── sender.py           # Python 源码 (纯标准库, 跨平台)
│   └── xywdl_sender.exe    # 预编译 C# 发送器 (~5KB, .NET Framework 4.x)
├── AUTH_MECHANISM.md       # 校园网认证机制详解
├── JSDOC.md                # 项目技术文档（含 API、架构、问题修复史）
├── src-tauri/              # Tauri 后端 (Rust)
│   ├── Cargo.toml
│   ├── tauri.conf.json
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
- .NET Framework 4.x（仅 Windows 上构建 C# sender 用，本机 Windows 自带）

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

## 更新日志

- **v2.2.1**：**修复 Hero Gauge 雷达动画偏心/换算异常，彻底根除 UTF-8 BOM 导致的自动认证失效**：
  1. **Hero Gauge 旋转与圆周几何修复**：将仪表盘 `<circle r="54">` 的 SVG `stroke-dasharray` 精确校准为理论圆周长 `339.3`（彻底消除 38px 的重叠缝隙）；将检测中的旋转动画由内层 `<circle>` 的偏心绕点迁移至外层 `.radar-svg` 以中心为原点的原生平滑旋转（`radarSvgSpin`），彻底解决检测网络时仪表环偏心乱晃的动画异常；
  2. **全面防御性清洗 UTF-8 BOM 头**：Rust 端的 `load_config`、`load_campus_net_info`、`get_login_profile`、`is_login_configured` 以及启动时的 `AppState` 初始化全面挂载 `strip_bom` 过滤函数，彻底根除 PowerShell 写入带 BOM 导致 JSON 反序列化失败、使得内存中 `primary_ssid` 为空的系统级隐患；
  3. **自动化保活探测与防御性双重触发**：Rust 后端 `check_network` 优化为即使未显式保存 SSID 也允许任意已连接 WiFi 进行判定与登录；前端 `checkNetwork` 机制升级，只要检测到 WiFi 已连接但无外网连通（`status.needs_login || (!status.internet_ok && status.wifi_connected)`），无缝主动发起 Web Portal 认证；并在手动点击“立即检测”时重置登录冷却计时，彻底恢复全自动化无感联网；
  4. **AppState 启动预热**：Rust 启动阶段直接从磁盘读取并解析已有配置并同步至内存，无需等待前端首个 IPC 命令即可具备完整的 SSID 守护上下文。
- **v2.2.0**：**恢复历史命名规范（`xxgcxy-wifi`）并支持 Windows 安装后原生中文文件名与快捷方式**：
  1. **包名与构建产物规范回归**：统一项目包名、Tauri 构建目标以及各平台发布文件名前缀为 `xxgcxy-wifi`，保持历史下载规范一致性（`xxgcxy-wifi_2.2.0_x64-setup.exe`、`xxgcxy-wifi_2.2.0_amd64.deb` 等）；
  2. **安装后自动转为中文名**：NSIS 安装器钩子（`installer.nsi`）在安装完成之后自动将目标可执行文件复制并命名为 `新乡工程校园网保活.exe`；
  3. **中文快捷方式与桌面/开始菜单对齐**：自动清理默认英文快捷方式，在桌面与“开始”菜单创建专属「新乡工程校园网保活」快捷方式及卸载入口；
  4. **原生进程无缝转交与自启注册表适配**：Rust 后端主入口实现双向探测，当以旧名或安装向导默认拉起时无缝交接至 `新乡工程校园网保活.exe`；开机自启动注册表项及托盘气泡提示全面使用中文名「新乡工程校园网保活」，且向前兼容旧版自启项。
- **v2.1.1**：**变异模糊测试（Fuzz Audit）、严格作用域净化与多语言数据契约工业级核验**。针对历史演进可能带来的隐式断层展开全盘深度排查：
  1. **变异模糊测试套件（6万次注入全量通过）**：建立自动化变异引擎，对 Portal URL 提纯、URL 编码器、服务端响应体解析、多网卡与热点冲突、历史配置向下兼容进行 60,004 次极端边界注入，验证无内存损坏、无状态死锁、无异常抛出；
  2. **Rust 后端深层安全扫描**：实现全生产代码 0 处裸 `.unwrap()`、0 处 `panic!()`，Win32 Unsafe 块内存与句柄生命周期严密闭环，切片访问全量前置守卫校验；
  3. **前端 ES 模块严格作用域净化**：显式声明顶层模块变量（`connectedSsid`, `firstRunTimerId`, `isFirstRunSetup`, `portalParsing`），根除严格模式下的全局变量污染与潜在 `ReferenceError`；
  4. **跨语言契约三端一致性核验**：全面对齐 Rust、前端 UI、PowerShell 脚本间的 14 个核心配置字段，确保老版本升级与空值回退平滑无感。
- **v2.1.0**：**五维子代理全链路深度审计与底层网络协议栈工业级加固**。基于五个专项技术子代理对 Rust 原生发包、C# 独立发送器、PowerShell 认证引擎、Python 跨平台保底层及前端交互闭环的系统性实机审计，实施针对性加固：
  1. **Rust 原生直发与探活加固**：彻底修复 `generate_uuid()` 中按位运算符优先级错误导致的 UUID 偶尔畸变为 37~38 位的严重缺陷（完全符合 RFC 4122 v4 标准）；重构 `get_wlan_network_info()` 适配中文 Windows（支持“名称”与所有活动网卡遍历，彻底避免回退假 IP `10.0.0.1` 导致的认证拒绝）；优化 `check_url` 探针，遇到 302 重定向严格判定为需登录（消除非预期重定向被误判为已连通的漏洞），使用无损流式读取兼容 GBK 编码 Portal 页面；探活备用节点替换为国内高可用端点（华为/MIUI）；
  2. **Python 保底层 (`sender.py`)**：注入 `NoRedirectHandler` 禁止跟随 302 重定向（与 C# / PowerShell 严格对齐，防止认证成功后跟随跳转至外网未放行地址引发异常）；标准输出全面切换为 `sys.stdout.buffer.write` 强制输出 UTF-8 字节流，杜绝 Windows 中文控制台下 `UnicodeEncodeError` 崩溃；
  3. **C# 独立发送器 (`sender.cs`)**：控制台输出强制指定 UTF-8；加入 `\uFEFF` 与空白字符循环清洗；显式开启 TLS 1.1 / TLS 1.2 现代安全协议；禁用 KeepAlive 及时释放连接；
  4. **前端与托盘交互闭环**：托盘菜单“执行登录脚本”全面接入 `runLoginScript()`，享受统一状态机、防抖冷却与互斥锁保护，杜绝与心跳检测的并发竞态。
- **v2.0.9**：**全面根治认证卡死与连接过慢问题，引入 Rust 进程内原生直发（Sub-100ms 闪电登录）**。重构桌面端执行链路，核心认证直接移入 Rust 进程内，使用 Windows 原生 `CryptUnprotectData` 解密与异步 `reqwest` 直发，彻底根除系统的 PowerShell 5.1 冷启动与多轮 WMI 扫描开销，认证耗时从 9.5 秒断崖式降低至 80 毫秒以内；保留外部脚本作为无缝降级兜底；PowerShell、C#、Python 三层请求发送器统一注入标准 Chrome 浏览器标头与中文语言环境及 `Referer`，彻底解决校园网 AC/WAF 防火墙误判并重置连接（“基础连接已经关闭: 连接被意外关闭” / “Remote end closed connection without response”）；针对内网环境将单层请求超时由 30 秒缩减至 6 秒，消除了原三层降级累加长达 90 秒的假死卡顿；优化外部脚本，去除 `xywdl.bat` 重复调用 PowerShell 查版本的 1.75 秒损耗，并为 `xywdl.ps1` 加入网卡对象单例缓存；移除前端登录后 30 秒全局阻塞锁，改为即时释放，用户手动点击“立即检测并重连”秒级响应；前端日志防抖，消除“重连后需要登录校园网...”多重重复输出；优化 SSID 大小写容错匹配与心跳自适应。
- **v2.0.8**：桌面端客户端 UI 全面重构为**深色极客毛玻璃（Dark Acrylic Glassmorphism）**风格；引入 140px 动态发光网络健康雷达环（Hero Gauge），支持在线/待登录/掉线状态呼吸动效与扫描雷达；全新学生身份名片（Profile Card）支持运营商专属色彩徽章与快捷更改；控制开关采用现代扁平平滑开关；活动动态终端升级为等宽字体并支持语义化高亮（绿标/红标/蓝标）与折叠；**新增“保持移动热点开启 (Hotspot Keep-Alive)”功能**（利用 Windows 原生 WinRT API 实时守护移动热点，防无设备连接超时休眠关闭，突破校园网单账号限制，保障手机/平板多设备持续共享上网）；**修复账号输入限制**（支持字母与数字组合，完美兼容补卡后缀如 `ls` / `lls` 以及前缀字母工号，输入含 `@` 自动剥离）；**彻底解决重启开机自启弹窗问题**（注册表注入 `--autostart` 标志且窗口初始设为不可见，开机绝对静默在托盘保活，手动双击才弹出窗口）；移除冗余“保存并登录”按钮；输入框示例参数全面脱敏匿名化；默认窗口优化为 460×800 并增加最小尺寸约束；CI/CD 修复弃用警告并支持正式发布。
- **v2.0.5**：消除前端日志多余空行；建立 Portal URL 四重防御性清洗体系（输入框粘贴自动提纯/保存提纯/Rust落盘提纯/脚本剥离），杜绝双问号导致账号密码丢失和 AC 报“设备不在正常状态”；重构响应判定逻辑（优先 JSON 解析并兼容双引号 code，直接透传真实错误信息，告别未知错误 99）；脚本增加 169.254 APIPA IP 过滤与 DHCP 延迟重试；自动化测试套件扩展至 30 项且全量通过。
- **v2.0.4**：修复 SSID/MAC 留空被误判为缺少字段、修复 PS 管道双 BOM 导致保底层发送失败、Portal URL 解析防连点锁。
- **v2.0.3**：登录脚本添加详细步骤日志，每个阶段显示 `[步骤 X/5]` 标记，失败时明确显示 `[!] 卡在: 步骤 N`，三层发送各显示 `[第 N 层]` 及具体错误原因。
- **v2.0.2**：修复登录配置缺少 SSID 字段导致无法登录（SSID 留空时运行时自动检测当前 WiFi）；更新版本号至 2.0.2。
- **v2.0.1**：修复桌面端调用登录脚本时 bat 失败路径的 `pause` 永久挂死；发送超时统一改为 30 秒；登录脚本实际输出回显到 UI 日志。
- **v2.0.0**：修复登录无法使用（MAC 留空被误判为缺少字段）、请求发送多层级降级（Windows: PowerShell→C#→Python；Linux: curl→Python）、跨平台 Python 保底层、Portal URL 解析按钮防连点、清理死代码。详见 [JSDOC.md §9](JSDOC.md)。
- **v1.9.1**：修复认证结果判定正则误匹配（`code:10/100/123` 被误判为“账号不存在”、`code:440` 被误判为“非法接入”）、请求日志脱敏密码（`passwd=***`）、统一 PS 端参数 URL 编码；新增自动化测试套件（`tests/`）与 Rust 单元测试。详见 [JSDOC.md §9](JSDOC.md)。
- **v1.9.0**：登录模块解耦为 JSON 模板 + 渲染器；可视化登录配置屏；DPAPI 加密密码；取消内置 PS7。

## 许可证

[MIT License](LICENSE) © Thatgfsj
