#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""linux-ui/portal_core.py 单元测试

运行：python3 linux-ui/tests/test_portal_core.py
覆盖：URL 提纯/校验、quickauth 地址推导、MAC 归一化、响应判定、
      配置文件读写，以及基于本地 mock 网关的端到端登录（成功/业务拒绝）。
"""

import json
import sys
import tempfile
import threading
import unittest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import portal_core as core  # noqa: E402


class MockPortal:
    """最小 Portal 网关：302 劫持 + generate_204 + quickauth.do。"""

    def __init__(self, code="0"):
        self.code = code
        self.online = False
        self.auth_query = ""
        outer = self

        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *_args):
                pass

            def do_GET(self):
                path, _, query = self.path.partition("?")
                base = "http://127.0.0.1:" + str(outer.port)
                if path == "/generate_204":
                    if outer.online:
                        self.send_response(204)
                        self.send_header("Content-Length", "0")
                        self.end_headers()
                    else:
                        self.send_response(302)
                        self.send_header("Location", base + "/portal.do?wlanuserip="
                                         "10.12.34.56&wlanacname=AC-TEST&vlan=100"
                                         "&mac=18-c0-4d-82-11-22&wlanacIp=172.18.252.1")
                        self.send_header("Content-Length", "0")
                        self.end_headers()
                    return
                if path == "/portal.do":
                    self.send_response(302)
                    self.send_header("Location", base + "/portal.do?wlanuserip="
                                     "10.12.34.56&wlanacname=AC-TEST&vlan=100"
                                     "&mac=18-c0-4d-82-11-22&wlanacIp=172.18.252.1")
                    self.send_header("Content-Length", "0")
                    self.end_headers()
                    return
                if path == "/quickauth.do":
                    outer.auth_query = query
                    if outer.code == "0":
                        outer.online = True
                        body = json.dumps({"code": 0, "message": "认证成功"},
                                          ensure_ascii=False).encode("utf-8")
                    elif outer.code == "1":
                        body = json.dumps({"code": "1", "message": "账号不存在"},
                                          ensure_ascii=False).encode("utf-8")
                    else:
                        body = json.dumps({"code": outer.code, "msg": "其他错误"},
                                          ensure_ascii=False).encode("utf-8")
                    self.send_response(200)
                    self.send_header("Content-Type", "application/json; charset=utf-8")
                    self.send_header("Content-Length", str(len(body)))
                    self.end_headers()
                    self.wfile.write(body)
                    return
                self.send_response(404)
                self.send_header("Content-Length", "0")
                self.end_headers()

        self.server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self.port = self.server.server_address[1]
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)

    def __enter__(self):
        self.thread.start()
        return self

    def __exit__(self, *_exc):
        self.server.shutdown()
        self.server.server_close()


class BaseCase(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        root = Path(self.tmp.name)
        self._saved = (core.CONFIG_HOME, core.BASE_DIR, core.PROFILE_FILE,
                       core.CRED_FILE, core.LOG_DIR, core.AUTOSTART_FILE,
                       core.PROBE_URLS, core.SNIFF_URLS)
        core.CONFIG_HOME = root
        core.BASE_DIR = root / "xxgcxy-wifi"
        core.PROFILE_FILE = core.BASE_DIR / "login_profile.json"
        core.CRED_FILE = core.BASE_DIR / "login_credential.bin"
        core.LOG_DIR = core.BASE_DIR / "logs"
        core.AUTOSTART_FILE = root / "autostart" / "xxgcxy-ui.desktop"

    def tearDown(self):
        (core.CONFIG_HOME, core.BASE_DIR, core.PROFILE_FILE, core.CRED_FILE,
         core.LOG_DIR, core.AUTOSTART_FILE, core.PROBE_URLS,
         core.SNIFF_URLS) = self._saved
        self.tmp.cleanup()


class TestParsing(BaseCase):
    def test_parse_absolute_portal_url(self):
        got = core.parse_portal_url(
            "http://172.18.252.12:6060/portal.do?wlanuserip=10.4.74.239"
            "&wlanacname=AuteWifi-XXGC&wlanacIp=172.18.252.1&vlan=31002201"
            "&mac=f4%3A6a%3Add%3Ae5%3A4a%3A7b&ssid=XinKe_Hist_Stu_2.4G")
        self.assertEqual(got["base_url"], "http://172.18.252.12:6060/portal.do")
        self.assertEqual(got["vlan"], "31002201")
        self.assertEqual(got["wlan_ac_name"], "AuteWifi-XXGC")
        self.assertEqual(got["mac_address"], "f4:6a:dd:e5:4a:7b")
        self.assertEqual(got["ssid"], "XinKe_Hist_Stu_2.4G")

    def test_parse_relative_location(self):
        got = core.parse_portal_url("/portal.do?wlanuserip=10.1.2.3&vlan=100")
        self.assertNotIn("base_url", got)
        self.assertEqual(got["wlan_user_ip"], "10.1.2.3")
        self.assertEqual(got["vlan"], "100")

    def test_parse_garbage(self):
        self.assertEqual(core.parse_portal_url(""), {})
        self.assertEqual(core.parse_portal_url("   "), {})

    def test_quickauth_url(self):
        self.assertEqual(core.resolve_quickauth_url("http://1.2.3.4:6060/portal.do"),
                         "http://1.2.3.4:6060/quickauth.do")
        self.assertEqual(core.resolve_quickauth_url("http://1.2.3.4:6060/quickauth.do"),
                         "http://1.2.3.4:6060/quickauth.do")
        self.assertEqual(core.resolve_quickauth_url("http://1.2.3.4:6060"),
                         "http://1.2.3.4:6060/quickauth.do")
        self.assertEqual(core.resolve_quickauth_url("http://1.2.3.4:6060/a/b/portal.do"),
                         "http://1.2.3.4:6060/a/b/quickauth.do")

    def test_valid_portal_url(self):
        self.assertTrue(core.is_valid_portal_base_url("http://172.18.252.12:6060/portal.do"))
        self.assertTrue(core.is_valid_portal_base_url("http://10.1.1.1/eportal/index.do"))
        self.assertFalse(core.is_valid_portal_base_url(
            "http://connect.rom.miui.com/generate_204"))
        self.assertFalse(core.is_valid_portal_base_url("http://www.baidu.com"))
        self.assertFalse(core.is_valid_portal_base_url("ftp://1.2.3.4/portal.do"))

    def test_normalize_mac(self):
        self.assertEqual(core.normalize_mac("AA-BB-CC-DD-EE-FF"), "aa:bb:cc:dd:ee:ff")
        self.assertEqual(core.normalize_mac(""), "")
        self.assertTrue(core.is_dummy_mac("00:00:00:00:00:00"))

    def test_judge_body(self):
        self.assertEqual(core._judge_body('{"code":0,"message":"认证成功"}'),
                         ("0", "认证成功"))
        self.assertEqual(core._judge_body('{"code":"44","msg":"非法接入"}'),
                         ("44", "非法接入"))
        self.assertEqual(core._judge_body('{"code":"1"}'), ("1", ""))
        self.assertEqual(core._judge_body("not json"), ("", ""))


class TestConfig(BaseCase):
    def test_roundtrip(self):
        profile = core.LoginProfile(operator="yd", base_url="http://1.1.1.1/portal.do")
        core.save_profile(profile, "secret")
        loaded = core.load_profile()
        self.assertTrue(loaded.user_id.endswith("@xxgcyd"))
        self.assertEqual(core.load_password(), "secret")

    def test_missing_files(self):
        self.assertEqual(core.load_profile().user_id, "")
        self.assertEqual(core.load_password(), "")


class TestLoginE2E(BaseCase):
    def _prepare(self, base):
        core.PROBE_URLS = (base + "/generate_204",)
        core.SNIFF_URLS = (base + "/generate_204",)
        profile = core.LoginProfile(
            user_id="2023110301@xxgcyd", operator="yd",
            base_url=base + "/portal.do", wlan_ac_ip="172.18.252.1")
        core.save_profile(profile, "mypassword")

    def test_login_success(self):
        with MockPortal("0") as portal:
            base = "http://127.0.0.1:" + str(portal.port)
            self._prepare(base)
            result = core.login()
            self.assertTrue(result["ok"], result)
            self.assertEqual(result["code"], "0")
            self.assertIn("userid=2023110301%40xxgcyd", portal.auth_query)
            self.assertIn("vlan=100", portal.auth_query)
            self.assertIn("passwd=mypassword", portal.auth_query)  # 明文密码按参数编码
            # 嗅探到的参数应回写配置
            saved = core.load_profile()
            self.assertEqual(saved.vlan, "100")
            self.assertEqual(saved.wlan_ac_name, "AC-TEST")

    def test_login_business_reject(self):
        with MockPortal("1") as portal:
            base = "http://127.0.0.1:" + str(portal.port)
            self._prepare(base)
            result = core.login()
            self.assertFalse(result["ok"])
            self.assertEqual(result["code"], "1")
            self.assertIn("账号不存在", result["message"])

    def test_login_without_config(self):
        self.assertEqual(core.login()["message"], "未配置账号或密码")


if __name__ == "__main__":
    unittest.main(verbosity=2)
