#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
xywdl_sender — 校园网认证请求发送器 (Python 保底层, 跨平台)
============================================================

用途:
  xywdl.ps1 / xywdl.sh 的"最强保底"发送层。当 PowerShell Invoke-WebRequest
  或 C# sender 都不可用时, 用系统 python3 发送认证请求。

  - 跨平台: Windows / Linux / macOS 都可用 (Python 3.6+)
  - 零依赖: 只用标准库 urllib, 不需要 pip 安装任何包
  - 语义与 C# sender 完全一致, 主脚本可共用一套判定逻辑

语义:
  exit 0   = 成功发出请求并拿到响应体 (stdout 为响应体, 即使 4xx/5xx)
  exit 1   = 网络层失败 (连接失败 / 超时 / DNS 失败等)
  exit 2   = 参数错误 (没有收到 URL)

用法 (由脚本调用, 不直接给用户用):
  echo "http://.../quickauth.do?userid=..." | python3 sender.py
  或: python3 sender.py "http://.../quickauth.do?userid=..."
"""

import sys

if sys.version_info < (3, 6):
    sys.stderr.write("[sender] 需要 Python 3.6+\n")
    sys.exit(1)


def _read_stdin_url() -> str:
    """从 stdin 读完整 URL。

    用原始字节读再按 utf-8-sig 解码: 同时解决两个问题——
      - PS 管道用带 BOM 的 UTF8 编码传字符串, utf-8-sig 会把 BOM 剥掉
      - 避免文本模式按 locale 编码误读中文/特殊字符
    """
    raw = sys.stdin.buffer.read()
    try:
        s = raw.decode("utf-8-sig")
    except UnicodeDecodeError:
        s = raw.decode("latin-1")
    # 兜底: 剥掉所有开头的 BOM (极端情况可能叠多个)
    return s.lstrip("\ufeff").strip()


def decode_body(raw: bytes, content_type: str = "") -> str:
    """优先按 header 里的 charset 解码, 其次 UTF-8, 最后 Latin-1 兜底。"""
    charset = None
    if content_type:
        # 从 "text/html; charset=gbk" 之类里抠出 charset
        for part in content_type.split(";"):
            part = part.strip().lower()
            if part.startswith("charset="):
                charset = part.split("=", 1)[1].strip().strip('"')
                break
    for enc in (charset, "utf-8", "gbk", "latin-1"):
        if not enc:
            continue
        try:
            return raw.decode(enc)
        except (UnicodeDecodeError, LookupError):
            continue
    return raw.decode("utf-8", errors="replace")


def main() -> int:
    url = (sys.argv[1].strip() if len(sys.argv) > 1
           else _read_stdin_url())
    if not url:
        sys.stderr.write("[sender] 未提供 URL\n")
        return 2

    try:
        import urllib.request
        import urllib.error

        # 禁用代理, 直连 (与 PS -Proxy $null / C# req.Proxy=null 一致)
        opener = urllib.request.build_opener(
            urllib.request.ProxyHandler({}),
        )
        req = urllib.request.Request(url, headers={
            "User-Agent": "XXGCXY-CampusNet-AutoLogin/2.0",
            "Accept": "text/html,application/json,*/*",
        })

        try:
            with opener.open(req, timeout=15) as resp:
                raw = resp.read()
                content_type = resp.headers.get("Content-Type", "")
                sys.stdout.write(decode_body(raw, content_type))
                return 0
        except urllib.error.HTTPError as e:
            # 4xx/5xx: 拿 body 交给主脚本判定, 算"发出去了"
            raw = e.read()
            content_type = e.headers.get("Content-Type", "")
            sys.stdout.write(decode_body(raw, content_type))
            return 0
    except Exception as ex:  # noqa: BLE001 - 保底层要兜住一切异常
        sys.stderr.write("[sender] 请求失败: %s\n" % ex)
        return 1


if __name__ == "__main__":
    sys.exit(main())
