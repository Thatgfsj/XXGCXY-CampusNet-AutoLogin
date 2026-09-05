#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
mock_portal.py — 校园网认证 Mock 服务器
用于端到端测试 xywdl.ps1 / xywdl.sh 的登录流程:
  - 收到 GET 请求,记录完整 query string 供断言
  - 按 --code 参数返回 {"code":N,...} 或 success/账号不存在/非法接入 文本
用法:
  python mock_portal.py --port 18080 --code 0 [--codefile <path>] [--log <path>]
"""
import argparse
import json
import sys
import time
from http.server import BaseHTTPRequestHandler, HTTPServer

RESPONSE_TEMPLATES = {
    "0": '{"code":0,"msg":"ok","result":1}',
    "1": '{"code":1,"msg":"账号不存在","result":0}',
    "44": '{"code":44,"msg":"非法接入","result":0}',
    "99": '{"code":99,"msg":"未知错误","result":0}',
    "10": '{"code":10,"msg":"参数错误","result":0}',
    "100": '{"code":100,"msg":"服务器内部错误","result":0}',
    "123": '{"code":123,"msg":"其他错误","result":0}',
    "440": '{"code":440,"msg":"VLAN 校验失败","result":0}',
    "success_text": '{"result":1,"info":"success"}',
    "auth_success_cn": '{"result":1,"info":"认证成功"}',
    "no_user_text": '{"result":0,"info":"账号不存在"}',
    "illegal_text": '{"result":0,"info":"非法接入"}',
    "ac_device_error": '{"code":"1","rec":null,"message":"设备不在正常状态,无法认证上网,请稍后","wlanacIp":null}',
    "ac_string_zero": '{"code":"0","rec":null,"message":"success","wlanacIp":null}',
}


class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        path, _, query = self.partial_path.partition('?')
        # 记录请求
        rec = {
            "path": path,
            "query": query,
            "params": _parse_query(query),
            "time": time.time(),
        }
        with open(server.log_path, "a", encoding="utf-8") as f:
            f.write(json.dumps(rec, ensure_ascii=False) + "\n")

        # 动态读取 codefile (如果提供)
        code_key = getattr(server, "code", "0")
        if server.codefile:
            try:
                with open(server.codefile, "r", encoding="utf-8") as cf:
                    code_key = cf.read().strip()
            except Exception:
                pass

        # 1. 互联网连通性 / 204 探针
        if path == "/generate_204":
            if code_key in ("302_redirect", "302"):
                loc = "http://127.0.0.1:18080/portal.do?wlanuserip=10.12.34.56&wlanacname=AC-TEST&vlan=100&mac=18-c0-4d-82-11-22&wlanacIp=172.18.252.1"
                self.send_response(302)
                self.send_header("Location", loc)
                self.send_header("Content-Length", "0")
                self.end_headers()
                return
            elif code_key == "fake_200_html":
                html = b"<!DOCTYPE html><html><head><title>Portal Login</title></head><body>Redirect to login</body></html>"
                self.send_response(200)
                self.send_header("Content-Type", "text/html; charset=utf-8")
                self.send_header("Content-Length", str(len(html)))
                self.end_headers()
                self.wfile.write(html)
                return
            else:
                self.send_response(204)
                self.send_header("Content-Length", "0")
                self.end_headers()
                return

        # 2. 302 劫持响应 (用于参数嗅探与拦截)
        if code_key in ("302_redirect", "302"):
            loc = "http://127.0.0.1:18080/portal.do?wlanuserip=10.12.34.56&wlanacname=AC-TEST&vlan=100&mac=18-c0-4d-82-11-22&wlanacIp=172.18.252.1"
            self.send_response(302)
            self.send_header("Location", loc)
            self.send_header("Content-Length", "0")
            self.end_headers()
            return

        # 3. 假 200 页面 (Captive Portal 拦截但返回 200 HTML)
        if code_key == "fake_200_html":
            html = b"<!DOCTYPE html><html><head><title>Portal Login</title></head><body>Please login to campus net</body></html>"
            self.send_response(200)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.send_header("Content-Length", str(len(html)))
            self.end_headers()
            self.wfile.write(html)
            return

        body = RESPONSE_TEMPLATES.get(code_key, RESPONSE_TEMPLATES["0"]).encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, fmt, *args):
        sys.stderr.write("[mock] " + fmt % args + "\n")

    @property
    def partial_path(self):
        return self.path


def _parse_query(query: str) -> dict:
    from urllib.parse import unquote_plus
    out = {}
    for kv in query.split("&"):
        if not kv:
            continue
        if "=" in kv:
            k, _, v = kv.partition("=")
        else:
            k, v = kv, ""
        out[k] = unquote_plus(v)
    return out


def main():
    global server
    ap = argparse.ArgumentParser()
    ap.add_argument("--port", type=int, default=18080)
    ap.add_argument("--code", default="0")
    ap.add_argument("--codefile", default=None)
    ap.add_argument("--log", default="mock_portal.log")
    args = ap.parse_args()

    server = HTTPServer(("127.0.0.1", args.port), Handler)
    server.code = args.code
    server.codefile = args.codefile
    server.log_path = args.log
    print(f"MOCK_READY port={args.port} code={args.code} log={args.log}", flush=True)
    server.serve_forever()


if __name__ == "__main__":
    main()
