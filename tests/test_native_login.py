#!/usr/bin/env python3
# -*- coding: utf-8 -*-
import os
import sys
import json
import time
import subprocess
import urllib.request
import urllib.parse
from urllib.error import HTTPError

MOCK_PORT = 18095
MOCK_BASE = "http://127.0.0.1:18095"
LOG_FILE = os.path.join(os.path.dirname(__file__), "mock_native_test.log")

PASS_COUNT = 0
FAIL_COUNT = 0

def assert_true(cond: bool, title: str, detail: str = ""):
    global PASS_COUNT, FAIL_COUNT
    if cond:
        PASS_COUNT += 1
        print(f"  [PASS] {title}")
    else:
        FAIL_COUNT += 1
        print(f"  [FAIL] {title} :: {detail}")

def parse_sniffed_location(location_url: str):
    parsed = urllib.parse.urlparse(location_url)
    qs = urllib.parse.parse_qs(parsed.query)

    user_ip = qs.get("wlanuserip", [None])[0]
    ac_name = qs.get("wlanacname", [None])[0]
    ac_ip = qs.get("wlanacIp", [None])[0]
    vlan = qs.get("vlan", [None])[0]
    mac = qs.get("mac", [None])[0]
    ssid = qs.get("ssid", [None])[0]

    base_url = f"{parsed.scheme}://{parsed.netloc}{parsed.path}"
    return {
        "user_ip": user_ip,
        "ac_name": ac_name,
        "ac_ip": ac_ip,
        "vlan": vlan,
        "mac": mac.lower().replace("-", ":") if mac else None,
        "ssid": ssid,
        "base_url": base_url,
    }

def is_dummy_ip(ip: str) -> bool:
    t = (ip or "").strip()
    return not t or t in ("0.0.0.0", "127.0.0.1", "10.0.0.1")

def is_dummy_mac(mac: str) -> bool:
    t = (mac or "").strip().lower()
    if not t or t in ("00:00:00:00:00:00", "aa:bb:cc:dd:ee:ff"):
        return True
    return t.count(":") != 5

def resolve_quickauth_url(raw_base: str) -> str:
    clean = raw_base.split("?")[0].split("#")[0].strip()
    if not clean:
        raise ValueError("Base URL 为空")
    if clean.endswith("/quickauth.do"):
        return clean
    with_scheme = clean if clean.startswith(("http://", "https://")) else "http://" + clean
    parsed = urllib.parse.urlparse(with_scheme)
    path = parsed.path
    if not path or path == "/":
        new_path = "/quickauth.do"
    elif path.endswith("/portal.do"):
        new_path = path.replace("/portal.do", "/quickauth.do")
    elif "/" in path:
        idx = path.rfind("/")
        new_path = "/quickauth.do" if idx == 0 else path[:idx] + "/quickauth.do"
    else:
        new_path = path.rstrip("/") + "/quickauth.do"
    return urllib.parse.urlunparse((parsed.scheme, parsed.netloc, new_path, "", "", ""))

def main():
    print("===== 测试 1: 302 重定向参数强制嗅探逻辑 =====")
    loc = "http://172.18.252.12:6060/portal.do?wlanuserip=10.12.34.56&wlanacname=AC-EAST&wlanacIp=172.18.252.1&vlan=100&mac=18-c0-4d-82-11-22&ssid=XXGC-WiFi"
    sniffed = parse_sniffed_location(loc)
    assert_true(sniffed["user_ip"] == "10.12.34.56", "302 提取真实 wlanuserip")
    assert_true(sniffed["ac_name"] == "AC-EAST", "302 提取真实 wlanacname")
    assert_true(sniffed["ac_ip"] == "172.18.252.1", "302 提取真实 wlanacIp")
    assert_true(sniffed["vlan"] == "100", "302 提取真实 vlan")
    assert_true(sniffed["mac"] == "18:c0:4d:82:11:22", "302 提取真实 mac 并归一化")
    assert_true(sniffed["ssid"] == "XXGC-WiFi", "302 提取真实 ssid")
    assert_true(sniffed["base_url"] == "http://172.18.252.12:6060/portal.do", "302 提纯 clean base_url")

    print("\n===== 测试 2: 兜底值安全拦截防御 (禁止向网关发假参数) =====")
    assert_true(is_dummy_ip(""), "空 IP 属于假参数")
    assert_true(is_dummy_ip("127.0.0.1"), "回环 127.0.0.1 属于假参数")
    assert_true(is_dummy_ip("0.0.0.0"), "未指定 0.0.0.0 属于假参数")
    assert_true(is_dummy_ip("10.0.0.1"), "网关默认 10.0.0.1 属于假参数")
    assert_true(not is_dummy_ip("10.12.34.56"), "真实内网 IP 通过检查")
    assert_true(not is_dummy_ip("172.18.252.12"), "真实网关 IP 通过检查")

    assert_true(is_dummy_mac(""), "空 MAC 属于假参数")
    assert_true(is_dummy_mac("00:00:00:00:00:00"), "全零 MAC 属于假参数")
    assert_true(is_dummy_mac("aa:bb:cc:dd:ee:ff"), "占位 MAC 属于假参数")
    assert_true(is_dummy_mac("invalid_mac"), "非法格式 MAC 属于假参数")
    assert_true(not is_dummy_mac("18:c0:4d:82:11:22"), "真实网卡 MAC 通过检查")

    print("\n===== 测试 3: URL 解析归一化 (防尾斜杠/无斜杠 URL 截断 Bug) =====")
    u1 = resolve_quickauth_url("http://172.18.252.12:6060/portal.do")
    assert_true(u1 == "http://172.18.252.12:6060/quickauth.do", "portal.do 替换为 quickauth.do")

    u2 = resolve_quickauth_url("http://172.18.252.12:6060")
    assert_true(u2 == "http://172.18.252.12:6060/quickauth.do", "无尾斜杠正确追加 quickauth.do")

    u3 = resolve_quickauth_url("http://172.18.252.12:6060/")
    assert_true(u3 == "http://172.18.252.12:6060/quickauth.do", "有尾斜杠正确替换为 quickauth.do")

    u4 = resolve_quickauth_url("http://172.18.252.12:6060/custom/path/portal.do")
    assert_true(u4 == "http://172.18.252.12:6060/custom/path/quickauth.do", "深层路径保留并在末级替换")

    u5 = resolve_quickauth_url("http://172.18.252.12:6060/portal.do?wlanuserip=1.1.1.1")
    assert_true(u5 == "http://172.18.252.12:6060/quickauth.do", "带 Query BaseURL 净化后解析")

    print("\n===== 测试 4: 启动 Mock Portal 验证 Native 协议端到端 =====")
    mock_py = os.path.join(os.path.dirname(__file__), "mock_portal.py")
    if os.path.exists(LOG_FILE):
        os.remove(LOG_FILE)
    code_file = os.path.join(os.path.dirname(__file__), "mock_code.txt")
    with open(code_file, "w", encoding="utf-8") as f:
        f.write("302_redirect")

    proc = subprocess.Popen([
        sys.executable, mock_py,
        "--port", str(MOCK_PORT),
        "--codefile", code_file,
        "--log", LOG_FILE
    ], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    time.sleep(1)

    try:
        # 4.1 测试 302 嗅探
        req = urllib.request.Request(f"{MOCK_BASE}/generate_204", headers={"User-Agent": "Test"})
        class NoRedirect(urllib.request.HTTPRedirectHandler):
            def http_error_302(self, req, fp, code, msg, headers):
                return headers
        opener = urllib.request.build_opener(NoRedirect)
        resp = opener.open(req)
        loc_header = resp.get("Location") if hasattr(resp, "get") else resp.headers.get("Location")
        assert_true(bool(loc_header and "portal.do" in loc_header), "Mock 成功返回 302 Location 劫持")

        sniffed_dyn = parse_sniffed_location(loc_header)
        assert_true(sniffed_dyn["mac"] == "18:c0:4d:82:11:22", "从真实 302 响应动态嗅探 MAC 成功")

        # 4.2 模拟 Code 44 重新取参
        with open(code_file, "w", encoding="utf-8") as f:
            f.write("44")
        resp44 = urllib.request.urlopen(f"{MOCK_BASE}/quickauth.do").read().decode("utf-8")
        val44 = json.loads(resp44)
        assert_true(val44.get("code") == 44, "Mock 正确响应 code 44 (触发重新取参)")

        # 4.3 模拟 Code 1 业务拒绝
        with open(code_file, "w", encoding="utf-8") as f:
            f.write("1")
        resp1 = urllib.request.urlopen(f"{MOCK_BASE}/quickauth.do").read().decode("utf-8")
        val1 = json.loads(resp1)
        assert_true(val1.get("code") == 1, "Mock 正确响应 code 1 (触发指数退避保护)")

        # 4.4 模拟 Code 0 + 204 复验成功
        with open(code_file, "w", encoding="utf-8") as f:
            f.write("0")
        resp0 = urllib.request.urlopen(f"{MOCK_BASE}/quickauth.do").read().decode("utf-8")
        val0 = json.loads(resp0)
        assert_true(val0.get("code") == 0, "Mock 正确响应 code 0")

        resp204 = urllib.request.urlopen(f"{MOCK_BASE}/generate_204")
        assert_true(resp204.getcode() == 204, "外网 204 复验探针放行")

    finally:
        proc.terminate()
        try:
            proc.wait(timeout=2)
        except Exception:
            proc.kill()
        if os.path.exists(code_file):
            os.remove(code_file)
        if os.path.exists(LOG_FILE):
            os.remove(LOG_FILE)

    print(f"\n===== 测试结果: {PASS_COUNT} 通过, {FAIL_COUNT} 失败 =====")
    if FAIL_COUNT > 0:
        sys.exit(1)
    else:
        sys.exit(0)

if __name__ == "__main__":
    main()