#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""校园网 Portal 认证与保活核心（linux-ui 轻量版）

纯标准库实现；协议语义对齐 src-tauri/src/lib.rs 与 xywdl.sh：
  探针   双 generate_204 并发，204=在线 / 3xx=待登录 / 其它=离线
  嗅探   从 302 Location 提取 vlan / wlanacname / wlanacIp / mac / wlanuserip
  认证   GET /quickauth.do，15 个参数，Chrome UA + Referer，禁跟随重定向，严格 code=="0"
  保活   默认 15s 周期；登录成功后 3s 复验；业务拒绝指数退避；传输层失败 60s 冷却
  重连   断网时按配置 SSID 用 nmcli 重连（60s 防风暴）

配置复用桌面端路径（XDG_CONFIG_HOME 或 ~/.config 下的 xxgcxy-wifi/）：
  login_profile.json / login_credential.bin（Linux 明文）/ logs/

用法：
  python3 portal_core.py status   # 打印状态快照(JSON)
  python3 portal_core.py login    # 执行一次登录
  python3 portal_core.py watch    # 前台保活，Ctrl-C 结束
"""

from __future__ import annotations

import http.client
import json
import os
import re
import subprocess
import sys
import threading
import time
import urllib.parse
import uuid
from dataclasses import asdict, dataclass, fields
from datetime import datetime
from pathlib import Path

# ---------------------------------------------------------------- 常量 ----

USER_AGENT = ("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
              "(KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36")
PROBE_URLS = ("http://connect.rom.miui.com/generate_204",
              "http://connectivitycheck.platform.hicloud.com/generate_204")
SNIFF_URLS = PROBE_URLS + ("http://1.1.1.1",)
OP_NAMES = {"yd": "移动", "lt": "联通", "dx": "电信"}
OP_SUFFIX = {"yd": "xxgcyd", "lt": "xxgclt", "dx": "xxgcdx"}

CONFIG_HOME = Path(os.environ.get("XDG_CONFIG_HOME") or (Path.home() / ".config"))
BASE_DIR = CONFIG_HOME / "xxgcxy-wifi"
PROFILE_FILE = BASE_DIR / "login_profile.json"
CRED_FILE = BASE_DIR / "login_credential.bin"
LOG_DIR = BASE_DIR / "logs"
AUTOSTART_FILE = CONFIG_HOME / "autostart" / "xxgcxy-ui.desktop"
PROFILE_DIR = BASE_DIR  # 供外部引用的别名


# ------------------------------------------------------------ 日志 ----

def _log(level: str, text: str) -> None:
    """写入当日日志文件（与控制台/脚本同目录），失败静默。"""
    try:
        LOG_DIR.mkdir(parents=True, exist_ok=True)
        today = datetime.now().strftime("%Y-%m-%d")
        yesterday = datetime.fromtimestamp(time.time() - 86400).strftime("%Y-%m-%d")
        for f in LOG_DIR.glob("xywdl-*.log"):
            if f.stem.replace("xywdl-", "") not in (today, yesterday):
                f.unlink(missing_ok=True)
        with open(LOG_DIR / ("xywdl-" + today + ".log"), "a", encoding="utf-8") as fp:
            fp.write("[" + datetime.now().strftime("%H:%M:%S") + "] ["
                     + level + "] " + text + "\n")
    except OSError:
        pass


# ------------------------------------------------------------ 配置 ----

@dataclass
class LoginProfile:
    user_id: str = ""
    operator: str = ""
    ssid: str = ""
    base_url: str = ""
    wlan_ac_name: str = ""
    wlan_ac_ip: str = ""
    vlan: str = ""
    wlan_user_ip: str = ""
    mac_address: str = ""
    portal_page_id: str = "3"
    portal_type: str = "0"
    version: str = "0"
    bind_ctrl_id: str = ""
    hostname: str = ""
    updated_at: str = ""


def _strip_bom(text: str) -> str:
    return text.lstrip("\ufeff")


def load_profile() -> LoginProfile:
    profile = LoginProfile()
    try:
        raw = PROFILE_FILE.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return profile
    try:
        data = json.loads(_strip_bom(raw))
    except ValueError:
        return profile
    if not isinstance(data, dict):
        return profile
    for f in fields(LoginProfile):
        value = data.get(f.name)
        if value is not None:
            setattr(profile, f.name, str(value))
    return profile


def load_password() -> str:
    try:
        raw = CRED_FILE.read_bytes()
    except OSError:
        return ""
    if raw.startswith(b"\x01\x00\x00\x00\xd0\x8c\x9d\xdf"):
        return ""  # Windows DPAPI 密文，Linux 无法解密
    try:
        return raw.decode("utf-8").strip().strip("\ufeff")
    except UnicodeDecodeError:
        return ""


def save_profile(profile: LoginProfile, password: str) -> None:
    """写入 login_profile.json + login_credential.bin（Linux 明文）。"""
    uid = profile.user_id.strip().strip("@")
    suffix = OP_SUFFIX.get(profile.operator, "xxgcyd")
    if "@" not in uid:
        uid = uid + "@" + suffix
    profile.user_id = uid
    profile.updated_at = datetime.now().astimezone().isoformat(timespec="seconds")
    BASE_DIR.mkdir(parents=True, exist_ok=True)
    PROFILE_FILE.write_text(
        json.dumps(asdict(profile), ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8")
    CRED_FILE.write_bytes(password.encode("utf-8"))
    _log("info", "配置已保存: " + uid + " operator=" + profile.operator)


def normalize_mac(raw: str) -> str:
    value = (raw or "").strip().lower().replace("-", ":")
    return value if re.fullmatch(r"[0-9a-f:]{11,17}", value) else ""


def is_dummy_ip(ip: str) -> bool:
    t = (ip or "").strip()
    return (not t) or t in ("0.0.0.0", "127.0.0.1", "10.0.0.1") \
        or t.startswith("169.254.")


def is_dummy_mac(mac: str) -> bool:
    t = (mac or "").strip().lower()
    return (not t) or t in ("00:00:00:00:00:00", "ff:ff:ff:ff:ff:ff") or len(t) < 11


# ------------------------------------------------- 地址提纯与校验 ----

def _split_url(url: str):
    """宽松解析：缺 scheme 时补 http://，返回 (scheme, netloc, path, query)。"""
    t = (url or "").strip()
    if not t:
        return None
    if "://" not in t:
        t = "http://" + t
    p = urllib.parse.urlsplit(t)
    if not p.netloc:
        return None
    return p.scheme.lower(), p.netloc, p.path or "/", p.query


def is_valid_portal_base_url(url: str) -> bool:
    parsed = _split_url(url)
    if not parsed:
        return False
    scheme, netloc, path, _ = parsed
    if scheme not in ("http", "https"):
        return False
    host = netloc.split("@")[-1].split(":")[0].lower()
    if any(h in host for h in ("miui.com", "hicloud.com", "qq.com", "baidu.com",
                               "bing.com", "msftconnecttest.com", "apple.com")):
        return False
    low = path.lower()
    return ("portal" in low or "quickauth" in low
            or low.endswith(".do") or "eportal" in low)


def parse_portal_url(url: str) -> dict:
    """把整条认证页 URL 提纯成可落盘字段；只返回识别到的字段。

    同时支持网关下发的相对 Location（如 "/portal.do?wlanuserip=..."）：
    此时没有 host，只提取 query 参数，不产出 base_url。
    """
    text = (url or "").strip()
    if not text:
        return {}
    out = {}
    parsed = _split_url(text)
    if parsed:
        scheme, netloc, path, query = parsed
        base = scheme + "://" + netloc + path
        if is_valid_portal_base_url(base):
            out["base_url"] = base
    else:
        # 相对路径：urlsplit 能单独解析 query
        query = urllib.parse.urlsplit(text).query
    params = {}
    for key, values in urllib.parse.parse_qs(query, keep_blank_values=True).items():
        params[key.lower()] = values[0] if values else ""
    mapping = {
        "wlanuserip": "wlan_user_ip",
        "wlanacname": "wlan_ac_name",
        "wlanacip": "wlan_ac_ip",
        "vlan": "vlan",
        "ssid": "ssid",
        "hostname": "hostname",
    }
    for src, dst in mapping.items():
        if params.get(src):
            out[dst] = params[src]
    if params.get("mac"):
        mac = normalize_mac(params["mac"])
        if mac:
            out["mac_address"] = mac
    return out


def resolve_quickauth_url(base_url: str) -> str:
    parsed = _split_url(base_url)
    if not parsed:
        raise ValueError("Portal URL 无效")
    scheme, netloc, path, _ = parsed
    path = path or "/"
    if path.rstrip("/").endswith("/quickauth.do"):
        path = path.rstrip("/")
    elif path.endswith("/portal.do"):
        path = path[: -len("/portal.do")] + "/quickauth.do"
    elif path.endswith(".do"):
        path = re.sub(r"/[^/]*\.do$", "/quickauth.do", path)
    else:
        path = path.rstrip("/") + "/quickauth.do"
    return scheme + "://" + netloc + path


# ------------------------------------------------- 本机网络信息 ----

def _run(cmd: list, timeout: float = 5.0) -> str:
    try:
        got = subprocess.run(cmd, capture_output=True, text=True,
                             timeout=timeout, check=False)
        return (got.stdout or "") + (got.stderr or "")
    except (OSError, subprocess.SubprocessError):
        return ""


def detect_network() -> dict:
    info = {"ssid": "", "ip": "", "mac": "", "iface": "", "wifi_connected": False}
    for line in _run(["nmcli", "-t", "-f", "ACTIVE,SSID", "dev", "wifi"]).splitlines():
        if line.startswith(("是:", "yes:")):
            ssid = line.split(":", 1)[1].strip()
            if ssid:
                info["ssid"] = ssid
                break
    route = _run(["ip", "route", "get", "1.1.1.1"])
    m_dev = re.search(r"\bdev\s+(\S+)", route)
    m_src = re.search(r"\bsrc\s+(\S+)", route)
    if m_dev:
        info["iface"] = m_dev.group(1)
    if m_src:
        info["ip"] = m_src.group(1)
    if info["iface"]:
        try:
            info["mac"] = Path("/sys/class/net/" + info["iface"] + "/address") \
                .read_text(encoding="utf-8").strip().lower()
        except OSError:
            pass
    if not info["ssid"]:
        for line in _run(["nmcli", "-t", "-f", "DEVICE,STATE,CONNECTION", "dev"]).splitlines():
            parts = line.split(":")
            if len(parts) >= 3 and parts[1].startswith("connected") and parts[2]:
                info["ssid"] = parts[2].strip()
                info["wifi_connected"] = True
                break
    else:
        info["wifi_connected"] = True
    return info


# ------------------------------------------------- 原始 HTTP ----

def _raw_get(url: str, timeout: float, headers: dict | None = None) -> tuple:
    """返回 (status, headers_dict, body_bytes)；不跟随重定向、不走系统代理。"""
    parsed = _split_url(url)
    if not parsed:
        raise ValueError("URL 无效")
    scheme, netloc, path, query = parsed
    host = netloc.split("@")[-1].split(":")[0]
    try:
        port = int(netloc.rsplit(":", 1)[1])
    except (IndexError, ValueError):
        port = 443 if scheme == "https" else 80
    target = path + ("?" + query if query else "")
    req_headers = {
        "User-Agent": USER_AGENT,
        "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        "Accept-Language": "zh-CN,zh;q=0.9,en;q=0.8",
        "Connection": "close",
    }
    if headers:
        req_headers.update(headers)
    klass = http.client.HTTPSConnection if scheme == "https" else http.client.HTTPConnection
    conn = klass(host, port, timeout=timeout)
    try:
        conn.request("GET", target, headers=req_headers)
        resp = conn.getresponse()
        body = resp.read(65536)
        return resp.status, dict(resp.getheaders()), body
    finally:
        conn.close()


def _decode(body: bytes) -> str:
    for enc in ("utf-8", "gb18030"):
        try:
            return body.decode(enc)
        except UnicodeDecodeError:
            continue
    return body.decode("utf-8", "replace")


def _check_one(url: str, timeout: float = 3.0):
    """返回 ("online"|"login"|"offline", location)。"""
    try:
        status, headers, body = _raw_get(url, timeout)
    except (OSError, ValueError, http.client.HTTPException):
        return "offline", ""
    if 300 <= status < 400:
        return "login", headers.get("Location", "") or headers.get("location", "")
    if status == 204:
        return "online", ""
    if 200 <= status < 300:
        # generate_204 正常联网不应返回 200 页面；出现即视为 Portal 劫持
        return "login", ""
    _ = body
    return "offline", ""


def probe(timeout: float = 3.0) -> str:
    """双探针并发：任一在线即在线；任一待登录即待登录。"""
    results = []
    lock = threading.Lock()

    def work(url):
        got = _check_one(url, timeout)
        with lock:
            results.append(got)

    threads = [threading.Thread(target=work, args=(u,), daemon=True) for u in PROBE_URLS]
    for t in threads:
        t.start()
    for t in threads:
        t.join(timeout + 1.0)
    states = [state for state, _ in results]
    if "online" in states:
        return "online"
    if "login" in states:
        return "login"
    return "offline"


def sniff_portal_params() -> dict:
    """发不跟随重定向的探测，从 302 Location 提取真实网关参数。"""
    for url in SNIFF_URLS:
        try:
            status, headers, _ = _raw_get(url, 2.5)
        except (OSError, ValueError, http.client.HTTPException):
            continue
        if 300 <= status < 400:
            location = headers.get("Location") or headers.get("location") or ""
            if location:
                parsed = parse_portal_url(location)
                if parsed:
                    return parsed
    return {}


def reconnect_wifi(ssid: str) -> bool:
    if not ssid:
        return False
    got = _run(["nmcli", "device", "wifi", "connect", ssid], timeout=25)
    ok = "successfully" in got.lower() or "已成功" in got or "成功" in got
    _log("info" if ok else "warn",
         "WiFi 重连 " + ssid + ": " + ("成功" if ok else got.strip()[:160]))
    return ok


# ------------------------------------------------- 认证 ----

def _build_auth_url(profile: LoginProfile, password: str, ssid: str,
                    ip: str, mac: str) -> str:
    auth_url = resolve_quickauth_url(profile.base_url)
    hostname = profile.hostname.strip() or os.uname().nodename
    params = [
        ("userid", profile.user_id), ("passwd", password),
        ("wlanuserip", ip), ("wlanacname", profile.wlan_ac_name),
        ("wlanacIp", profile.wlan_ac_ip), ("ssid", ssid),
        ("vlan", profile.vlan), ("mac", mac),
        ("version", profile.version or "0"),
        ("portalpageid", profile.portal_page_id or "3"),
        ("timestamp", str(int(time.time() * 1000))),
        ("uuid", str(uuid.uuid4())),
        ("portaltype", profile.portal_type or "0"),
        ("hostname", hostname), ("bindCtrlId", profile.bind_ctrl_id),
    ]
    query = urllib.parse.urlencode(params, quote_via=urllib.parse.quote, safe="")
    return auth_url + "?" + query


def _judge_body(body: str) -> tuple:
    """返回 (code, message)。优先 JSON，兼容 code 为字符串/数字。"""
    try:
        data = json.loads(body)
        if isinstance(data, dict):
            raw_code = data.get("code", "")
            code = "" if raw_code is None else str(raw_code).strip()
            msg = data.get("message") or data.get("msg") or data.get("info") or ""
            return code, str(msg)
    except ValueError:
        pass
    m_code = re.search(r'"code"\s*:\s*"?([0-9]+)', body)
    m_msg = re.search(r'"(?:message|msg|info)"\s*:\s*"([^"]*)"', body)
    return (m_code.group(1) if m_code else ""), (m_msg.group(1) if m_msg else "")


def login(profile: LoginProfile | None = None, password: str | None = None) -> dict:
    """执行一次认证；返回 {ok, code, message, elapsed_ms}。阻塞，最长约 10s。"""
    started = time.monotonic()
    profile = profile or load_profile()
    password = load_password() if password is None else password
    result = {"ok": False, "code": "", "message": "", "elapsed_ms": 0}

    if not profile.user_id.strip() or not password:
        result["message"] = "未配置账号或密码"
        return result
    if not profile.base_url.strip():
        result["message"] = "未配置 Portal URL"
        return result

    sniffed = sniff_portal_params()
    if sniffed:
        changed = False
        for key in ("wlan_user_ip", "mac_address", "vlan", "wlan_ac_name",
                    "wlan_ac_ip", "base_url"):
            value = sniffed.get(key, "")
            if not value:
                continue
            if key in ("wlan_user_ip", "wlan_ac_ip") and is_dummy_ip(value):
                continue
            if key == "mac_address" and is_dummy_mac(value):
                continue
            if getattr(profile, key, "") != value:
                setattr(profile, key, value)
                changed = True
        if changed:
            try:
                save_profile(profile, password)
            except OSError:
                pass

    net = detect_network()
    ssid = profile.ssid.strip() or net["ssid"]
    ip = profile.wlan_user_ip.strip() or net["ip"]
    mac = normalize_mac(profile.mac_address) or net["mac"]
    if is_dummy_ip(ip) or is_dummy_mac(mac):
        result["code"] = "-1"
        result["message"] = ("未获取到有效网卡参数 (IP='" + ip + "', MAC='"
                             + mac + "')，拒绝发送")
        _log("error", result["message"])
        return result

    try:
        url = _build_auth_url(profile, password, ssid, ip, mac)
        referer = url.split("/quickauth.do")[0] + "/portal.do"
        status, _, body = _raw_get(url, 6.0, {
            "Referer": referer, "X-Requested-With": "XMLHttpRequest"})
    except (OSError, ValueError, http.client.HTTPException) as exc:
        result["code"] = "-1"
        result["message"] = "发送失败: " + str(exc)
        result["elapsed_ms"] = int((time.monotonic() - started) * 1000)
        _log("error", result["message"])
        return result

    code, message = _judge_body(_decode(body))
    result["code"] = code or str(status)
    result["elapsed_ms"] = int((time.monotonic() - started) * 1000)
    if 300 <= status < 400:
        result["message"] = "网关返回重定向状态码 " + str(status)
        _log("warn", result["message"])
        return result
    if code == "0":
        time.sleep(0.4)
        if probe() == "online":
            result["ok"] = True
            result["message"] = message or "认证成功"
            _log("success", "认证成功并复验通过，耗时 "
                 + str(result["elapsed_ms"]) + " ms")
        else:
            result["message"] = "网关返回 code=0，但 204 复验未通过"
            _log("warn", result["message"])
    else:
        result["message"] = message or ("认证未通过 (code=" + result["code"] + ")")
        _log("warn", "认证未通过 code=" + result["code"] + ": " + result["message"])
    return result


# ------------------------------------------------- 开机自启 ----

def _desktop_entry() -> str:
    here = Path(__file__).resolve()
    ui = here.parent / "xxgcxy_ui.py"
    return ("[Desktop Entry]\n"
            "Type=Application\n"
            "Name=校园网保活\n"
            "Exec=/usr/bin/env python3 " + str(ui) + "\n"
            "Terminal=false\n"
            "X-GNOME-Autostart-enabled=true\n")


def is_autostart_enabled() -> bool:
    return AUTOSTART_FILE.is_file()


def set_autostart(enabled: bool) -> bool:
    try:
        if enabled:
            AUTOSTART_FILE.parent.mkdir(parents=True, exist_ok=True)
            AUTOSTART_FILE.write_text(_desktop_entry(), encoding="utf-8")
        else:
            AUTOSTART_FILE.unlink(missing_ok=True)
        return is_autostart_enabled() == enabled
    except OSError:
        return False


# ------------------------------------------------- 保活状态机 ----

class KeepAliveService:
    """后台线程保活；所有回调都在工作线程触发，GTK 侧需 GLib.idle_add。"""

    def __init__(self, on_event=None, interval: int = 15):
        self._emit = on_event or (lambda _e: None)
        self._interval = max(5, int(interval))
        self._stop = threading.Event()
        self._wake = threading.Event()
        self._manual = threading.Event()
        self._thread = None
        self._lock = threading.Lock()
        self._state = "stopped"
        self._message = ""
        self._next_retry = 0.0
        self._last_login_at = ""
        self._wifi_reconnect = True
        self._net = {"ssid": "", "ip": "", "mac": ""}
        self._last_trigger = 0.0
        self._last_fail_transport = 0.0
        self._next_reconnect = 0.0

    # -- 控制面 --
    def start(self) -> None:
        if self.is_running():
            return
        self._stop.clear()
        self._wake.clear()
        self._thread = threading.Thread(target=self._run, name="keepalive", daemon=True)
        self._thread.start()

    def stop(self) -> None:
        self._stop.set()
        self._wake.set()
        if self._thread and self._thread.is_alive():
            self._thread.join(timeout=8)
        self._thread = None
        self._set_state("stopped", "已停止保活")

    def is_running(self) -> bool:
        return bool(self._thread and self._thread.is_alive())

    def trigger_login(self) -> None:
        now = time.monotonic()
        if now - self._last_trigger < 3:
            return
        self._last_trigger = now
        self._manual.set()
        self._wake.set()

    def set_interval(self, seconds: int) -> None:
        self._interval = max(5, min(300, int(seconds)))

    def set_wifi_reconnect(self, enabled: bool) -> None:
        self._wifi_reconnect = bool(enabled)

    def snapshot(self) -> dict:
        with self._lock:
            return {
                "state": self._state, "message": self._message,
                "ssid": self._net["ssid"], "ip": self._net["ip"],
                "mac": self._net["mac"], "online": self._state == "online",
                "last_login_at": self._last_login_at,
                "next_retry": int(max(0, self._next_retry - time.monotonic())),
            }

    # -- 事件 --
    def _set_state(self, state: str, message: str = "",
                   next_retry: float = 0.0) -> None:
        with self._lock:
            self._state = state
            self._message = message
            self._next_retry = next_retry
            snap = {"ssid": self._net["ssid"], "ip": self._net["ip"],
                    "mac": self._net["mac"]}
        self._emit({"type": "status", "state": state,
                    "ssid": snap["ssid"], "ip": snap["ip"], "mac": snap["mac"],
                    "online": state == "online", "message": message,
                    "next_retry": int(max(0, next_retry - time.monotonic()))})

    def _log(self, level: str, text: str) -> None:
        _log(level, text)
        self._emit({"type": "log", "level": level, "text": text})

    def _sleep(self, seconds: float) -> bool:
        """可被手动触发/停止打断的等待；返回 False 表示已请求停止。"""
        if self._stop.is_set():
            return False
        self._wake.wait(max(0.5, seconds))
        return not self._stop.is_set()

    # -- 主循环 --
    def _run(self) -> None:
        self._log("info", "保活已启动")
        backoff = 10
        transport_until = 0.0
        while not self._stop.is_set():
            manual = self._manual.is_set()
            self._manual.clear()
            self._wake.clear()
            profile = load_profile()
            password = load_password()
            net = detect_network()
            with self._lock:
                self._net = {"ssid": net["ssid"], "ip": net["ip"], "mac": net["mac"]}

            if not profile.user_id.strip() or not password:
                self._set_state("error", "未配置账号或密码，请打开账号设置")
                self._sleep(self._interval)
                continue

            if manual:
                self._log("info", "手动立即登录…")
                if self._do_login(profile, password):
                    backoff, transport_until = 10, 0.0
                    self._sleep(3)
                continue

            self._set_state("checking", "正在检测网络…")
            state = probe()
            if self._manual.is_set():
                continue
            if state == "online":
                self._set_state("online", "网络在线")
                backoff, transport_until = 10, 0.0
                self._sleep(self._interval)
                continue
            if state == "login":
                now = time.monotonic()
                if now < transport_until:
                    wait = int(transport_until - now)
                    self._set_state("needs_login",
                                    "传输层冷却中，" + str(wait) + "s 后重试",
                                    transport_until)
                    self._sleep(min(wait, 5))
                    continue
                if self._do_login(profile, password):
                    backoff, transport_until = 10, 0.0
                    self._sleep(3)
                    continue
                now = time.monotonic()
                if self._last_fail_transport and now - self._last_fail_transport < 1.0:
                    transport_until = now + 60
                    self._set_state("needs_login", "传输层失败，60s 冷却",
                                    transport_until)
                    self._sleep(5)
                    continue
                wait = min(backoff, 120)
                self._set_state("backoff", "认证被拒，" + str(wait) + "s 后重试",
                                now + wait)
                self._sleep(wait)
                backoff = min(backoff * 2, 120)
                continue

            if self._wifi_reconnect and profile.ssid.strip():
                now = time.monotonic()
                if now >= self._next_reconnect:
                    self._next_reconnect = now + 60
                    self._log("warn", "WiFi 断线，尝试重连 " + profile.ssid)
                    reconnect_wifi(profile.ssid)
            self._set_state("offline", "WiFi 未连接")
            self._sleep(self._interval)
        self._set_state("stopped", "已停止保活")

    def _do_login(self, profile: LoginProfile, password: str) -> bool:
        self._set_state("logging_in", "正在发送认证…")
        result = login(profile, password)
        self._last_fail_transport = (time.monotonic()
                                     if result["code"] == "-1" else 0.0)
        if result["ok"]:
            self._last_login_at = datetime.now().strftime("%H:%M:%S")
            self._emit({"type": "login", "ok": True, "code": result["code"],
                        "message": result["message"],
                        "elapsed_ms": result["elapsed_ms"]})
            self._set_state("online", "认证成功（" + str(result["elapsed_ms"]) + " ms）")
            return True
        self._emit({"type": "login", "ok": False, "code": result["code"],
                    "message": result["message"],
                    "elapsed_ms": result["elapsed_ms"]})
        self._log("warn", "登录失败: " + result["message"])
        self._set_state("needs_login", result["message"])
        return False


# ------------------------------------------------- 命令行 ----

def _cli() -> int:
    action = sys.argv[1] if len(sys.argv) > 1 else "status"
    if action == "status":
        net = detect_network()
        print(json.dumps({"probe": probe(), "network": net,
                          "configured": bool(load_profile().user_id.strip())
                          and bool(load_password())},
                         ensure_ascii=False, indent=2))
        return 0
    if action == "login":
        result = login()
        print(json.dumps(result, ensure_ascii=False, indent=2))
        return 0 if result["ok"] else 1
    if action == "watch":
        def on_event(event):
            print(json.dumps(event, ensure_ascii=False), flush=True)
        service = KeepAliveService(on_event)
        service.start()
        try:
            while True:
                time.sleep(1)
        except KeyboardInterrupt:
            service.stop()
        return 0
    print("用法: portal_core.py [status|login|watch]", file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(_cli())
