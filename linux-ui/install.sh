#!/usr/bin/env bash
# linux-ui 安装/卸载脚本：复制文件、写应用菜单项与桌面图标（无需 root）
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}"
DEST="$DATA_HOME/xxgcxy-wifi-ui"
APPS="$DATA_HOME/applications"
DESKTOP_DIR="${XDG_DESKTOP_DIR:-$HOME/Desktop}"
DESK_FILE="$APPS/xxgcxy-ui.desktop"
ICON_SRC="$HERE/../src-tauri/icons/icon.png"
PY_BIN="$(command -v python3 || true)"

usage() {
    echo "用法: ./install.sh [--uninstall]"
}

uninstall() {
    rm -rf "$DEST"
    rm -f "$APPS/xxgcxy-ui.desktop" "$DESKTOP_DIR/校园网保活.desktop"
    echo "[+] 已卸载 linux-ui（配置目录 ~/.config/xxgcxy-wifi 未删除）"
}

if [[ "${1:-}" == "--uninstall" ]]; then
    uninstall
    exit 0
fi
if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
    usage
    exit 0
fi
if [[ -z "$PY_BIN" ]]; then
    echo "[!] 未找到 python3" >&2
    exit 1
fi
if ! "$PY_BIN" -c 'import gi; gi.require_version("Gtk", "4.0")' 2>/dev/null; then
    echo "[!] 缺少 PyGObject / GTK4，请安装 python3-gi（Debian/Ubuntu: sudo apt install python3-gi）" >&2
    exit 1
fi

mkdir -p "$DEST" "$APPS" "$DESKTOP_DIR"
install -m 0644 "$HERE/portal_core.py" "$DEST/portal_core.py"
install -m 0644 "$HERE/xxgcxy_ui.py" "$DEST/xxgcxy_ui.py"
if [[ -f "$ICON_SRC" ]]; then
    install -m 0644 "$ICON_SRC" "$DEST/xxgcxy-icon.png"
    ICON="$DEST/xxgcxy-icon.png"
else
    ICON="network-wireless"
fi

write_desktop() {
    cat > "$1" <<EOF
[Desktop Entry]
Type=Application
Version=1.0
Name=校园网保活
GenericName=CampusNet KeepAlive
Comment=新乡工程学院校园网认证与保活（GTK4 轻量界面）
Exec=$PY_BIN $DEST/xxgcxy_ui.py
Icon=$ICON
Terminal=false
Path=$DEST
Categories=Network;
StartupNotify=true
EOF
    chmod 0755 "$1"
}

write_desktop "$DESK_FILE"
write_desktop "$DESKTOP_DIR/校园网保活.desktop"

command -v update-desktop-database >/dev/null 2>&1 && \
    update-desktop-database "$APPS" 2>/dev/null || true

echo "[+] 已安装到: $DEST"
echo "    应用菜单项: $DESK_FILE"
echo "    桌面图标:   $DESKTOP_DIR/校园网保活.desktop"
echo "    直接运行:   $PY_BIN $DEST/xxgcxy_ui.py"
