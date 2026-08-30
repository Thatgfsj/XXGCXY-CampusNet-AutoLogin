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

        code_key = getattr(server, "code", "0")
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
        # 兼容:BaseHTTPRequestHandler 只给了 self.path
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
    ap.add_argument("--log", default="mock_portal.log")
    args = ap.parse_args()

    server = HTTPServer(("127.0.0.1", args.port), Handler)
    server.code = args.code
    server.log_path = args.log
    print(f"MOCK_READY port={args.port} code={args.code} log={args.log}", flush=True)
    server.serve_forever()


if __name__ == "__main__":
    main()
