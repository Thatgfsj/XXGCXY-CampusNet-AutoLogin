#!/usr/bin/env python3
# -*- coding: utf-8 -*-
import os
import re
import sys

def main():
    version = sys.argv[1] if len(sys.argv) > 1 else ""
    tag = sys.argv[2] if len(sys.argv) > 2 else f"v{version}"
    output_path = sys.argv[3] if len(sys.argv) > 3 else "artifacts/RELEASE_NOTES.md"

    readme_path = "README.md"
    changelog = ""
    if os.path.exists(readme_path):
        with open(readme_path, "r", encoding="utf-8") as f:
            content = f.read()
        pattern = rf"(?m)^- \*\*v{re.escape(version)}\*\*.*?(?=^- \*\*v|\n## |\Z)"
        m = re.search(pattern, content, re.DOTALL)
        if m:
            changelog = m.group(0).strip()

    if not changelog:
        changelog = f"- **{tag}**：校园网自动登录与保活版本更新。"

    body = f"""{tag} - 校园网自动登录助手

## 更新说明

{changelog}

## 系统要求

- **Windows 10/11**: 自带 PowerShell 5.1+ / .NET Framework 4.x
- **Windows 7/8**: 需装 WMF 5.1（[下载](https://www.microsoft.com/en-us/download/details.aspx?id=54616)）
- **Linux**: 需 curl 或 python3（[安装](https://www.python.org/downloads/)）

## 下载

**Windows (系统 PowerShell 5.1+, 体积小)**
- `xxgcxy-wifi_{version}_x64-setup.exe` - NSIS 安装包 (~5 MB)
- `xxgcxy-wifi_{version}_x64_zh-CN.msi` - MSI 安装包 (~6 MB)

**Linux**
- `xxgcxy-wifi_{version}_amd64.deb` - Debian/Ubuntu 软件包
- `xxgcxy-wifi-v{version}-x86_64-linux.tar.gz` - 通用 Linux 压缩包
- `xxgcxy-wifi-{version}-1.x86_64.rpm` - RPM 软件包

**Cross-platform**
- `xywdl.sh` - 独立 Shell 脚本 (Linux/macOS, 需要 curl 或 python3)

## 升级说明

从旧版本升级：
1. 直接下载安装最新的安装包覆盖安装即可
2. 现有账号与 WiFi 配置自动保留
3. 若之前遇到连接过慢、卡在待认证或被意外掐线，升级后即可极速恢复秒登
"""

    os.makedirs(os.path.dirname(os.path.abspath(output_path)), exist_ok=True)
    with open(output_path, "w", encoding="utf-8") as f:
        f.write(body.strip() + "\n")
    print(f"[+] Successfully generated release notes to {output_path}")

if __name__ == "__main__":
    main()
