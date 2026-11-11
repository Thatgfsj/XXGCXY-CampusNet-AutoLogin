#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""新乡工程学院 · 校园网保活 —— GTK4 原生界面（单文件，仅 GTK4 + 标准库）。

界面只是外壳：探针 / 登录 / 保活全部在 portal_core.KeepAliveService 工作线程里跑，
事件经 GLib.idle_add 回到主线程后才更新控件；主线程严禁调用 login() / probe()。
"""

import os
import sys
import time
from datetime import datetime

import gi

gi.require_version("Gtk", "4.0")
gi.require_version("Gdk", "4.0")
from gi.repository import Gdk, GLib, Gtk, Pango  # noqa: E402

# 无论从哪个目录启动，都要能 import 到同目录下的 portal_core.py
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
try:
    import portal_core as core
    CORE_OK, CORE_MSG = True, ""
except Exception as exc:  # 核心未落盘时优雅降级：界面仍能打开
    core, CORE_OK, CORE_MSG = None, False, "%s: %s" % (type(exc).__name__, exc)

APP_ID = "com.xxgcxy.campusnet.keepalive"
APP_TITLE = "新乡工程学院 · 校园网保活"
OP_KEYS = ("yd", "lt", "dx")   # 与 portal_core.OPERATORS 的键一致
OP_NAMES = ("移动", "联通", "电信")
LEVELS = ("info", "success", "warn", "error")
LOG_COLORS = {"info": "#94a3b8", "success": "#10b981", "warn": "#f59e0b", "error": "#ef4444"}
# 内部状态 -> (中文文案, 样式类)；退避态文案由 next_retry 动态覆盖
STATES = {"booting": ("启动中", "busy"), "checking": ("检测中", "busy"),
          "online": ("在线", "online"), "needs_login": ("待登录", "warn"),
          "logging_in": ("登录中", "busy"), "backoff": ("退避中", "warn"),
          "offline": ("离线", "err"), "stopped": ("已停止", "idle"),
          "error": ("异常", "err")}
KINDS = ("online", "warn", "err", "busy", "idle")

# 深色主题（配色参考仓库 index.html 的玻璃拟态暗色风格）
CSS = """
window, window.background { background-color: #0a0b12; background-image: none; color: #f3f4f6; }
headerbar { background-color: #0d0f18; background-image: none; border-bottom: 1px solid rgba(255,255,255,0.07); box-shadow: none; min-height: 42px; }
headerbar label, headerbar button { color: #f3f4f6; }
headerbar button:hover { background-image: none; background-color: rgba(255,255,255,0.10); }
.badge { background-color: rgba(255,255,255,0.08); border-radius: 8px; padding: 2px 8px; color: #a5b4fc; font-size: 11px; font-weight: 700; }
.card { background-color: rgba(22,25,39,0.72); border: 1px solid rgba(255,255,255,0.08); border-radius: 18px; padding: 14px; }
label.hero { font-size: 22px; font-weight: 800; } label.sub { font-size: 12px; color: #94a3b8; }
label.dim { font-size: 11px; color: #64748b; } label.val { font-size: 13px; font-weight: 700; color: #e2e8f0; }
label.section { font-size: 13px; font-weight: 700; color: #cbd5e1; } label.err { font-size: 12px; color: #ef4444; }
label.ok { font-size: 12px; color: #10b981; }
.pill { background-color: rgba(255,255,255,0.04); border: 1px solid rgba(255,255,255,0.06); border-radius: 12px; padding: 8px 12px; }
.dot { min-width: 20px; min-height: 20px; border-radius: 99px; margin-top: 6px; background-color: #64748b; }
.dot.online { background-color: #10b981; } .dot.warn { background-color: #f59e0b; } .dot.err { background-color: #ef4444; } .dot.busy { background-color: #06b6d4; }
label.c-online { color: #10b981; } label.c-warn { color: #f59e0b; } label.c-err { color: #ef4444; } label.c-busy { color: #06b6d4; } label.c-idle { color: #cbd5e1; }
button.primary { background-image: none; background-color: #6366f1; color: #ffffff; border: none; border-radius: 12px; font-weight: 700; padding: 10px 16px; } button.primary:hover { background-color: #4f46e5; } button.primary:disabled { background-color: rgba(99,102,241,0.30); color: rgba(255,255,255,0.45); }
button.ghost { background-image: none; background-color: rgba(255,255,255,0.06); color: #cbd5e1; border: 1px solid rgba(255,255,255,0.10); border-radius: 10px; padding: 8px 14px; font-weight: 600; } button.ghost:hover { background-color: rgba(255,255,255,0.12); color: #ffffff; }
switch { background-color: rgba(255,255,255,0.14); border-radius: 99px; } switch:checked { background-color: #6366f1; } switch > slider { background-color: #ffffff; border-radius: 99px; }
entry, passwordentry, dropdown, dropdown button { background-image: none; background-color: rgba(255,255,255,0.05); color: #f3f4f6; border: 1px solid rgba(255,255,255,0.10); border-radius: 10px; padding: 6px 10px; } entry:focus, passwordentry:focus { border-color: rgba(99,102,241,0.85); } dropdown button { border: none; }
textview { background-color: transparent; color: #cbd5e1; } .logbox { background-color: rgba(0,0,0,0.22); border-radius: 12px; padding: 6px; }
separator { background-color: rgba(255,255,255,0.07); min-height: 1px; } expander, expander title { color: #cbd5e1; font-size: 13px; }
"""


def margins(widget, top, bottom, start, end):
    """一次性设置四边边距。"""
    widget.set_margin_top(top)
    widget.set_margin_bottom(bottom)
    widget.set_margin_start(start)
    widget.set_margin_end(end)


class MainWindow(Gtk.ApplicationWindow):
    """主窗口：状态卡 / 三个开关 / 日志区 / 设置与退出。"""

    def __init__(self, app):
        super().__init__(application=app, title=APP_TITLE)
        self.set_default_size(520, 780); self.set_size_request(460, 640)   # 默认 / 最小尺寸
        self._ready = False; self._closed = False; self._tick_id = 0
        self._retry_at = None; self._state = "booting"; self._service = None
        self._build_header(); self._build_body()
        if CORE_OK:
            self._init_service()
        else:  # 降级模式：核心模块缺失时界面照常打开，仅提示并禁用功能
            self._set_state("核心模块缺失", "err")
            self.sub.set_text("portal_core.py 不可用，请先放到 linux-ui/ 目录")
            self._append_log("error", "无法导入 portal_core：%s" % CORE_MSG)
            self._append_log("warn", "界面已降级运行：账号设置与登录功能不可用")
            for widget in (self.btn_login, self.sw_keepalive, self.sw_reconnect, self.sw_autostart):
                widget.set_sensitive(False)
        self._ready = True
        self._tick_id = GLib.timeout_add(1000, self._tick)   # 每秒刷新倒计时与快照
        self.connect("close-request", self._on_close)
        self._append_log("info", "界面已就绪")
        if CORE_OK and self._needs_setup():   # 首次启动缺账号/密码时自动弹设置
            GLib.idle_add(self._open_settings)

    # ------------------------------------------------------------------ 构建
    def _build_header(self):
        bar = Gtk.HeaderBar()
        box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        box.append(Gtk.Label(label=APP_TITLE)); box.append(Gtk.Label(label="GTK4", css_classes=["badge"]))
        bar.set_title_widget(box); self.set_titlebar(bar)

    def _build_body(self):
        root = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=10)
        margins(root, 10, 10, 16, 16)
        for widget in (self._build_status_card(), self._build_switch_card(),
                       self._build_log_card(), self._build_footer()):
            root.append(widget)
        self.set_child(root)

    def _build_status_card(self):
        card = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12, css_classes=["card"])
        self.dot = Gtk.Box(css_classes=["dot", "busy"]); self.dot.set_valign(Gtk.Align.START)
        self.hero = Gtk.Label(label="启动中", xalign=0, css_classes=["hero", "c-busy"],
                              ellipsize=Pango.EllipsizeMode.END)
        self.sub = Gtk.Label(label="正在准备保活服务…", xalign=0, css_classes=["sub"],
                             ellipsize=Pango.EllipsizeMode.END)
        head = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=2, hexpand=True)
        head.append(self.hero); head.append(self.sub)
        top = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        top.append(self.dot); top.append(head); card.append(top)
        # 四行信息：SSID / IP / MAC / 最近登录（2×2 胶囊格）
        self.info = {key: Gtk.Label(label="—", xalign=0, css_classes=["val"],
                                    ellipsize=Pango.EllipsizeMode.END)
                     for key in ("ssid", "ip", "mac", "login")}
        grid = Gtk.Grid(column_spacing=8, row_spacing=8, column_homogeneous=True)
        cells = (("ssid", "SSID"), ("ip", "IP 地址"), ("mac", "MAC 地址"), ("login", "最近登录"))
        for pos, (key, title) in enumerate(cells):
            pill = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=2, css_classes=["pill"])
            pill.append(Gtk.Label(label=title, xalign=0, css_classes=["dim"])); pill.append(self.info[key])
            grid.attach(pill, pos % 2, pos // 2, 1, 1)
        card.append(grid)
        self.btn_login = Gtk.Button(label="立即登录", css_classes=["primary"])
        self.btn_login.connect("clicked", self._on_login); card.append(self.btn_login)
        return card

    def _build_switch_card(self):
        card = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=10, css_classes=["card"])
        card.append(Gtk.Label(label="保活设置", xalign=0, css_classes=["section"]))
        self.sw_keepalive = Gtk.Switch(); self.sw_keepalive.connect("notify::active", self._on_keepalive_toggle)
        self.sw_reconnect = Gtk.Switch(); self.sw_reconnect.connect("notify::active", self._on_reconnect_toggle)
        self.sw_autostart = Gtk.Switch(); self.sw_autostart.connect("notify::active", self._on_autostart_toggle)
        rows = (("自动保活", "后台每 15 秒探针一次，掉线自动登录", self.sw_keepalive),
                ("断线重连 WiFi", "认证失败时用 nmcli 重连校园 SSID", self.sw_reconnect),
                ("开机自启", "登录桌面后自动运行本工具", self.sw_autostart))
        for idx, (title, desc, switch) in enumerate(rows):
            if idx:
                card.append(Gtk.Separator())
            text = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=2, hexpand=True)
            text.append(Gtk.Label(label=title, xalign=0, css_classes=["val"]))
            text.append(Gtk.Label(label=desc, xalign=0, css_classes=["dim"], wrap=True))
            switch.set_valign(Gtk.Align.CENTER)
            row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
            row.append(text); row.append(switch); card.append(row)
        return card

    def _build_log_card(self):
        card = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8, css_classes=["card"])
        card.set_vexpand(True)
        title = Gtk.Label(label="运行日志", xalign=0, css_classes=["section"], hexpand=True)
        clear = Gtk.Button(label="清空", css_classes=["ghost"])
        clear.connect("clicked", lambda *_: self.log_buf.set_text(""))
        head = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        head.append(title); head.append(clear); card.append(head)
        self.log_buf = Gtk.TextBuffer()
        for level in LEVELS:   # 按级别上色
            tag = self.log_buf.create_tag("lv_" + level, foreground=LOG_COLORS[level])
            tag.set_property("weight", Pango.Weight.NORMAL if level == "info" else Pango.Weight.BOLD)
        self.log_view = Gtk.TextView(buffer=self.log_buf, editable=False, monospace=True,
                                     cursor_visible=False, wrap_mode=Gtk.WrapMode.WORD_CHAR)
        margins(self.log_view, 6, 6, 6, 6)
        self.log_scroll = Gtk.ScrolledWindow(vexpand=True, hexpand=True)
        self.log_scroll.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
        self.log_scroll.add_css_class("logbox"); self.log_scroll.set_child(self.log_view)
        card.append(self.log_scroll)
        return card

    def _build_footer(self):
        bar = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)
        settings = Gtk.Button(label="账号设置…", css_classes=["ghost"], hexpand=True)
        settings.connect("clicked", self._open_settings)
        quit_btn = Gtk.Button(label="退出", css_classes=["ghost"])
        quit_btn.connect("clicked", lambda *_: self.close())
        bar.append(settings); bar.append(quit_btn)
        return bar

    # ------------------------------------------------------------- 服务装配
    def _init_service(self):
        try:
            self._service = core.KeepAliveService(self._on_core_event, interval=15)
        except Exception as exc:
            self._set_state("服务创建失败", "err")
            self._append_log("error", "KeepAliveService 创建失败：%s" % exc)
            return
        self._service.set_wifi_reconnect(True)
        self.sw_keepalive.set_active(True); self.sw_reconnect.set_active(True)   # 默认开启
        try: self.sw_autostart.set_active(bool(core.is_autostart_enabled()))
        except Exception: self.sw_autostart.set_active(False)
        try: self._service.start()            # 启动即开始保活
        except Exception as exc: self._append_log("error", "保活线程启动失败：%s" % exc)

    def _needs_setup(self):
        try:
            return not (getattr(core.load_profile(), "user_id", "") and core.load_password())
        except Exception:
            return False

    # ---------------------------------------------------------- 工作线程桥接
    def _on_core_event(self, event):
        """工作线程回调：只做数据搬运，绝不在此触碰控件。"""
        if self._closed or not isinstance(event, dict):
            return
        try: GLib.idle_add(self._dispatch_event, dict(event))
        except Exception: pass   # 退出过程中的 idle_add 失败可忽略

    def _dispatch_event(self, event):
        """主线程：按事件类型刷新界面。"""
        if self._closed:
            return False
        kind = event.get("type")
        if kind == "status":
            self._apply_status(event)
        elif kind == "log":
            self._append_log(event.get("level", "info"), event.get("text", ""))
        elif kind == "login":
            if event.get("ok"):
                self._append_log("success", "%s（%s，%dms）" % (
                    event.get("message") or "登录成功", event.get("code") or "ok",
                    int(event.get("elapsed_ms") or 0)))
            else:
                self._append_log("error", "登录失败：%s" % (
                    event.get("message") or event.get("code") or "未知错误"))
            self.btn_login.set_sensitive(True)
        return False

    def _apply_status(self, event):
        state = event.get("state") or "stopped"
        self._state, label, kind = state, *STATES.get(state, (state, "idle"))
        remain = int(event.get("next_retry") or 0) if state == "backoff" else 0
        self._retry_at = time.monotonic() + remain if remain > 0 else None
        if remain > 0:
            label = "退避中 %d 秒" % remain
        self._set_state(label, kind)
        self.sub.set_text(event.get("message") or "")
        self._update_info({"ssid": event.get("ssid"), "ip": event.get("ip"), "mac": event.get("mac")})
        self.btn_login.set_sensitive(state != "logging_in")   # 登录中禁用主按钮
        return False

    def _set_state(self, label, kind):
        self.hero.set_text(label)
        for name in KINDS:
            self.dot.remove_css_class(name); self.hero.remove_css_class("c-" + name)
        self.dot.add_css_class(kind); self.hero.add_css_class("c-" + kind)

    def _update_info(self, values):
        for key, widget in self.info.items():
            widget.set_text(str(values.get(key) or "—"))

    # ------------------------------------------------------------------ 日志
    def _append_log(self, level, text):
        if not text:
            return
        level = level if level in LEVELS else "info"
        stamp = datetime.now().strftime("%H:%M:%S")
        self.log_buf.insert_with_tags_by_name(self.log_buf.get_end_iter(),
                                              "[%s] %s\n" % (stamp, text), "lv_" + level)
        if self.log_buf.get_line_count() > 600:   # 控制内存：裁掉最早 100 行
            self.log_buf.delete(self.log_buf.get_start_iter(), self.log_buf.get_iter_at_line(100))
        GLib.idle_add(self._log_to_bottom)

    def _log_to_bottom(self):
        """自动滚动到底部。"""
        try:
            adj = self.log_scroll.get_vadjustment()
            adj.set_value(max(adj.get_lower(), adj.get_upper() - adj.get_page_size()))
        except Exception: pass
        return False

    # ------------------------------------------------------------------ 交互
    def _on_login(self, _button):
        if not self._service:
            self._append_log("error", "保活服务不可用，无法登录")
            return
        self.btn_login.set_sensitive(False)
        self._append_log("info", "手动触发登录 …")
        try:
            self._service.trigger_login()   # 异步，结果由工作线程事件回报
        except Exception as exc:
            self.btn_login.set_sensitive(True); self._append_log("error", "触发登录失败：%s" % exc)

    def _on_keepalive_toggle(self, switch, _pspec):
        if not self._ready or not self._service:
            return
        active = switch.get_active()
        try: self._service.start() if active else self._service.stop()
        except Exception as exc:
            self._append_log("error", "切换自动保活失败：%s" % exc); return
        self._append_log("info", "自动保活已" + ("开启" if active else "关闭"))
        if not active:
            self._set_state("已停止", "idle"); self.sub.set_text("自动保活已关闭")
            self.btn_login.set_sensitive(True)

    def _on_reconnect_toggle(self, switch, _pspec):
        if not self._ready or not self._service:
            return
        active = switch.get_active()
        try: self._service.set_wifi_reconnect(active)
        except Exception as exc:
            self._append_log("error", "切换断线重连失败：%s" % exc); return
        self._append_log("info", "断线重连 WiFi 已" + ("开启" if active else "关闭"))

    def _on_autostart_toggle(self, switch, _pspec):
        if not self._ready:
            return
        active = switch.get_active()
        try: ok = bool(core.set_autostart(active))
        except Exception as exc:
            ok = False; self._append_log("error", "设置开机自启失败：%s" % exc)
        if not ok:   # 失败时回滚开关，避免界面与磁盘状态不一致
            self._ready = False; switch.set_active(not active); self._ready = True
            self._append_log("error", "开机自启" + ("开启" if active else "关闭") + "失败")
            return
        self._append_log("info", "开机自启已" + ("开启" if active else "关闭"))

    def _open_settings(self, *_args):
        if not CORE_OK:
            self._append_log("error", "portal_core 不可用，无法打开设置")
            return False   # 兼容 GLib.idle_add
        SettingsDialog(self, self._on_settings_saved).present()
        return False

    def _on_settings_saved(self):
        self._append_log("success", "账号设置已保存"); self._refresh_snapshot()

    # ------------------------------------------------------------------ 定时
    def _tick(self):
        if self._closed:
            return False
        if self._retry_at and self._state == "backoff":
            self.hero.set_text("退避中 %d 秒" % max(0, int(self._retry_at - time.monotonic())))
        self._refresh_snapshot()
        return True

    def _refresh_snapshot(self):
        """读服务快照补全 SSID / IP / MAC / 最近登录时间。"""
        if not self._service:
            return
        try: snap = self._service.snapshot()
        except Exception: return
        if isinstance(snap, dict):
            self._update_info({"ssid": snap.get("ssid"), "ip": snap.get("ip"),
                               "mac": snap.get("mac"), "login": snap.get("last_login_at")})

    def _on_close(self, *_args):
        """关闭窗口即停止保活并退出。"""
        if self._closed:
            return False
        self._closed = True
        if self._tick_id:
            GLib.source_remove(self._tick_id); self._tick_id = 0
        if self._service:
            try: self._service.stop()
            except Exception: pass
        return False


class SettingsDialog(Gtk.Window):
    """账号设置：粘贴整条 Portal URL 时自动提纯并回填高级参数。"""

    def __init__(self, parent, on_saved):
        super().__init__(transient_for=parent, modal=True, title="账号设置")
        self.set_default_size(460, 640); self.set_size_request(380, 480)
        self._on_saved = on_saved; self._parsed = {}; self._base_url = ""
        try: profile, password = core.load_profile(), core.load_password()
        except Exception: profile, password = core.LoginProfile(), ""
        bar = Gtk.HeaderBar(); bar.set_title_widget(Gtk.Label(label="账号设置")); self.set_titlebar(bar)

        form = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        margins(form, 16, 16, 16, 16)

        def field(title, widget, hint=None):
            box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
            box.append(Gtk.Label(label=title, xalign=0, css_classes=["dim"]))
            box.append(widget)
            if hint is not None:
                box.append(hint)
            return box

        self.e_user = Gtk.Entry(placeholder_text="学号，如 2023110301@xxgcyd")
        self.e_pwd = Gtk.PasswordEntry(show_peek_icon=True)
        self.dd_op = Gtk.DropDown.new_from_strings(list(OP_NAMES))
        self.e_url = Gtk.Entry(placeholder_text="粘贴浏览器地址栏里的整条 portal.do?… 链接")
        self.url_hint = Gtk.Label(label="粘贴带参数链接后自动提纯 base_url / VLAN / AC",
                                  xalign=0, css_classes=["dim"], wrap=True)
        self.e_vlan = Gtk.Entry(placeholder_text="如 1300")
        self.e_acname = Gtk.Entry(placeholder_text="wlanacname")
        self.e_acip = Gtk.Entry(placeholder_text="wlanacIp")
        self.e_ssid = Gtk.Entry(placeholder_text="校园 WiFi 名称")
        entries = (self.e_user, self.e_pwd, self.e_url, self.e_vlan, self.e_acname,
                   self.e_acip, self.e_ssid)
        for entry in entries:
            entry.set_hexpand(True)
        form.append(field("学号 / 账号", self.e_user))
        form.append(field("密码", self.e_pwd))
        form.append(field("运营商", self.dd_op))
        form.append(field("Portal URL", self.e_url, self.url_hint))
        advanced = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=10)
        for title, widget in (("VLAN", self.e_vlan), ("wlanacname（AC 名称）", self.e_acname),
                              ("wlanacIp（AC 地址）", self.e_acip), ("主 SSID", self.e_ssid)):
            advanced.append(field(title, widget))
        expander = Gtk.Expander()
        expander.set_label_widget(Gtk.Label(label="高级参数", xalign=0, css_classes=["section"]))
        expander.set_child(advanced)
        form.append(expander)
        self.msg = Gtk.Label(label="修改后点「保存」生效", xalign=0, wrap=True, css_classes=["dim"])
        form.append(self.msg)
        buttons = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)
        buttons.set_halign(Gtk.Align.END)
        cancel = Gtk.Button(label="取消", css_classes=["ghost"])
        cancel.connect("clicked", lambda *_: self.destroy())
        save = Gtk.Button(label="保存", css_classes=["primary"]); save.connect("clicked", self._on_save)
        buttons.append(cancel); buttons.append(save); form.append(buttons)
        scroller = Gtk.ScrolledWindow(vexpand=True, hexpand=True)
        scroller.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
        scroller.set_child(form); self.set_child(scroller)

        # 回填已有配置
        self.e_user.set_text(getattr(profile, "user_id", "") or "")
        self.e_pwd.set_text(password or "")
        operator = getattr(profile, "operator", "yd") or "yd"
        if operator in OP_KEYS:
            self.dd_op.set_selected(OP_KEYS.index(operator))
        self._base_url = getattr(profile, "base_url", "") or ""
        for entry, key in ((self.e_vlan, "vlan"), (self.e_acname, "wlan_ac_name"),
                           (self.e_acip, "wlan_ac_ip"), (self.e_ssid, "ssid")):
            entry.set_text(getattr(profile, key, "") or "")
        if self._base_url:
            self.url_hint.set_text("已保存 Portal：%s" % self._base_url)
        self.e_url.connect("changed", self._on_url_changed)

    def _set_msg(self, text, is_error=False):
        self.msg.set_text(text)
        self.msg.remove_css_class("err"); self.msg.remove_css_class("ok")
        self.msg.add_css_class("err" if is_error else "ok")

    def _on_url_changed(self, entry):
        """检测到完整 portal 链接时提纯并回填。"""
        raw = entry.get_text().strip()
        if "?" not in raw or not raw.lower().startswith(("http://", "https://")):
            return
        try: data = core.parse_portal_url(raw)
        except Exception as exc:
            self._set_msg("URL 解析失败：%s" % exc, True); return
        if not isinstance(data, dict) or not data:
            return
        for key, widget in (("wlan_ac_name", self.e_acname), ("wlan_ac_ip", self.e_acip),
                            ("vlan", self.e_vlan), ("ssid", self.e_ssid)):
            if data.get(key):
                widget.set_text(str(data[key]))
        for key in ("wlan_user_ip", "mac_address", "hostname"):
            if data.get(key):
                self._parsed[key] = data[key]
        if data.get("base_url"):
            self._base_url = data["base_url"]
            self.url_hint.set_text("已提纯 Portal：%s" % self._base_url)
        self._set_msg("已从 URL 提纯并回填高级参数")

    def _on_save(self, _button):
        if not CORE_OK:
            self._set_msg("portal_core 不可用，无法保存", True); return
        user, password = self.e_user.get_text().strip(), self.e_pwd.get_text()
        if not user or not password:
            self._set_msg("请填写学号和密码", True); return
        try: profile = core.load_profile()   # 保留界面未暴露的字段
        except Exception: profile = core.LoginProfile()
        profile.user_id = user
        index = self.dd_op.get_selected()
        profile.operator = OP_KEYS[index] if 0 <= index < len(OP_KEYS) else "yd"
        profile.wlan_ac_name = self.e_acname.get_text().strip()
        profile.wlan_ac_ip = self.e_acip.get_text().strip()
        profile.vlan = self.e_vlan.get_text().strip()
        profile.ssid = self.e_ssid.get_text().strip()
        if self._base_url:
            profile.base_url = self._base_url
        for key, value in self._parsed.items():
            setattr(profile, key, value)
        try: core.save_profile(profile, password)
        except Exception as exc:
            self._set_msg("保存失败：%s" % exc, True); return
        self._on_saved(); self.destroy()


class App(Gtk.Application):
    """应用入口：只注册一次 CSS，然后展示主窗口。"""

    def __init__(self):
        super().__init__(application_id=APP_ID)
        self._css_ready = False; self.window = None

    def do_activate(self):
        if not self._css_ready:
            provider = Gtk.CssProvider(); provider.load_from_string(CSS)
            Gtk.StyleContext.add_provider_for_display(
                Gdk.Display.get_default(), provider, Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION)
            self._css_ready = True
        if self.window is None:
            self.window = MainWindow(self)
        self.window.present()


def main(argv=None):
    return App().run(argv if argv is not None else sys.argv)


if __name__ == "__main__":
    sys.exit(main())
