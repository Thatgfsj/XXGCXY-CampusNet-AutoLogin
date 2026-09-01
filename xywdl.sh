#!/bin/bash
# 新乡工程学院校园网登录脚本 (Linux版, v1.9.0+ 简化)
#
# 配置文件 (跟 Rust 端 Linux 路径一致):
#   $HOME/.config/xxgcxy-wifi/login_profile.json  - JSON 元数据 (snake_case)
#   $HOME/.config/xxgcxy-wifi/login_credential.bin - 密码 (Linux 上是明文,Windows 上是 DPAPI 加密)
#
# 兼容 PS 5.1+ 的 .bin 格式: 本脚本只读明文。
# Windows 上 .bin 是 DPAPI 加密的, 不能用本脚本。
# Windows 用户请用 xywdl.bat -> xywdl.ps1。

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROFILE_DIR="$HOME/.config/xxgcxy-wifi"
PROFILE_FILE="$PROFILE_DIR/login_profile.json"
CRED_FILE="$PROFILE_DIR/login_credential.bin"

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
WHITE='\033[1;37m'
NC='\033[0m'

log_info() { echo -e "${CYAN}$1${NC}"; }
log_success() { echo -e "${GREEN}$1${NC}"; }
log_warn() { echo -e "${YELLOW}$1${NC}"; }
log_error() { echo -e "${RED}$1${NC}"; }

echo -e "${CYAN}"
echo "========================================"
echo "  新乡工程学院校园网登录脚本 (Linux版, v1.9.0+)"
echo "========================================"
echo -e "${NC}"

# 检查 profile 文件
if [[ ! -f "$PROFILE_FILE" ]]; then
    log_error "[!] 未找到登录配置: $PROFILE_FILE"
    log_warn "    请先在 UI 主页或设置页填写校园网账号信息。"
    exit 2
fi

if [[ ! -f "$CRED_FILE" ]]; then
    log_error "[!] 未找到密码文件: $CRED_FILE"
    log_warn "    请重新在 UI 中保存配置。"
    exit 2
fi

# 用 python3 解析 JSON (Linux 一般都装了, 比 jq 通用)
if ! command -v python3 &>/dev/null; then
    log_error "[!] 需要 python3 解析配置, 但找不到"
    exit 1
fi

PROFILE_JSON=$(cat "$PROFILE_FILE")
USER_ID=$(echo "$PROFILE_JSON" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('user_id',''))")
OPERATOR=$(echo "$PROFILE_JSON" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('operator',''))")
BASE_URL=$(echo "$PROFILE_JSON" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('base_url',''))")
WLAN_AC_NAME=$(echo "$PROFILE_JSON" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('wlan_ac_name',''))")
WLAN_AC_IP=$(echo "$PROFILE_JSON" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('wlan_ac_ip',''))")
VLAN=$(echo "$PROFILE_JSON" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('vlan',''))")
PROFILE_MAC=$(echo "$PROFILE_JSON" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('mac_address',''))")
WLAN_USER_IP=$(echo "$PROFILE_JSON" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('wlan_user_ip',''))")
SSID=$(echo "$PROFILE_JSON" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('ssid',''))")
PORTAL_PAGE_ID=$(echo "$PROFILE_JSON" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('portal_page_id','3'))")
PORTAL_TYPE=$(echo "$PROFILE_JSON" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('portal_type','0'))")
VERSION=$(echo "$PROFILE_JSON" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('version','0'))")
BIND_CTRL_ID=$(echo "$PROFILE_JSON" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('bind_ctrl_id',''))")
HOSTNAME_VAL=$(echo "$PROFILE_JSON" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('hostname',''))")

# 校验必需字段
# mac_address / wlan_user_ip 是可选的: UI 允许留空, 运行时由本机自动取兜底
for f in "user_id:$USER_ID" "base_url:$BASE_URL" "vlan:$VLAN"; do
    key="${f%%:*}"
    val="${f#*:}"
    if [[ -z "$val" ]]; then
        log_error "[!] 登录配置缺少字段: $key"
        exit 2
    fi
done

# 读密码 (Linux 上是明文)
PASSWORD=$(cat "$CRED_FILE")

# 软必填自动探测: wlan_ac_name / wlan_ac_ip 缺失时尝试从 base_url 拿
# (跟 PS 端 Get-AutoPortalParams 行为一致)
# 校园网 AC 通常会 302 重定向到 portal.do?wlanacname=...&wlanacIp=...
if [[ -z "$WLAN_AC_NAME" || -z "$WLAN_AC_IP" ]]; then
    if [[ -n "$BASE_URL" ]]; then
        PROBE_URL="$BASE_URL"
        if [[ ! "$PROBE_URL" =~ ^https?:// ]]; then
            PROBE_URL="http://$PROBE_URL"
        fi
        LOCATION=$(curl -s -o /dev/null -D - --max-redirs 0 --noproxy '*' --max-time 8 \
            "$PROBE_URL" 2>/dev/null | grep -i '^location:' | head -1 | sed 's/^[Ll]ocation:[[:space:]]*//' | tr -d '\r\n')
        if [[ -n "$LOCATION" ]]; then
            if [[ -z "$WLAN_AC_NAME" ]]; then
                WLAN_AC_NAME=$(echo "$LOCATION" | grep -oE 'wlanacname=[^&]+' | head -1 | sed 's/^wlanacname=//' | sed 's/%20/ /g')
            fi
            if [[ -z "$WLAN_AC_IP" ]]; then
                WLAN_AC_IP=$(echo "$LOCATION" | grep -oE 'wlanacIp=[^&]+' | head -1 | sed 's/^wlanacIp=//')
            fi
            if [[ -n "$WLAN_AC_NAME" || -n "$WLAN_AC_IP" ]]; then
                log_warn "    [✓] 自动探测成功: wlan_ac_name=$WLAN_AC_NAME, wlan_ac_ip=$WLAN_AC_IP"
            fi
        fi
    fi
fi

echo ""
log_info "========================================"
log_info "  校园网自动登录脚本 (Linux版, v1.9.0+)"
log_info "========================================"
log_info ""
log_info "[*] 步骤 0: 加载登录配置..."
log_info "    学号: $USER_ID"
log_info "    运营商: $OPERATOR"
log_info "    Portal URL: $BASE_URL"
log_info "    VLAN: $VLAN"
log_info "    SSID: $SSID"
log_info "    AC 名称: $WLAN_AC_NAME"
log_info "    AC IP: $WLAN_AC_IP"

log_info "[*] 步骤 0 完成 - 配置验证通过"
log_info ""

log_info "[*] 步骤 0.5: 读取密码..."
if [[ -z "$PASSWORD" ]]; then
    log_error "[!] 卡在: 步骤 0.5 - 密码文件为空"
    exit 3
fi
log_info "[*] 步骤 0.5 完成 - 密码读取成功 (已隐藏)"
log_info ""

log_info "[*] 步骤 1: 获取运行时网络信息..."
log_info "    当前 SSID: $SSID"
if [[ -z "$WLAN_USER_IP" ]]; then
    WLAN_USER_IP=$(ip route get 1 2>/dev/null | grep -oP 'src \K[^ ]+' | head -1)
fi
if [[ -z "$WLAN_USER_IP" ]]; then
    log_warn "[*] 拿不到本机 IP, wlanuserip 留空"
else
    log_info "    本机 IP: $WLAN_USER_IP"
fi

LIVE_MAC=""
if command -v ip &>/dev/null; then
    IFACE=$(ip route get 1 2>/dev/null | grep -oP 'dev \K[^ ]+' | head -1)
    if [[ -n "$IFACE" ]] && [[ -f "/sys/class/net/$IFACE/address" ]]; then
        LIVE_MAC=$(cat "/sys/class/net/$IFACE/address" 2>/dev/null)
    fi
fi
if [[ -n "$LIVE_MAC" ]]; then
    MAC=$(echo "$LIVE_MAC" | tr '[:upper:]' '[:lower:]')
else
    MAC=$(echo "$PROFILE_MAC" | tr '[:upper:]' '[:lower:]')
fi
log_info "    本机 MAC: $MAC"
log_info "[*] 步骤 1 完成"
log_info ""

# 构造 quickauth.do URL
log_info "[*] 步骤 2: 构造认证请求 URL..."
AUTH_URL=$(echo "$BASE_URL" | sed -E 's|/[A-Za-z0-9_-]+\.do$|/quickauth.do|')
[[ -z "$HOSTNAME_VAL" ]] && HOSTNAME_VAL=$(hostname)

TIMESTAMP=$(date +%s)000
UUID=$(cat /proc/sys/kernel/random/uuid 2>/dev/null || python3 -c "import uuid; print(uuid.uuid4())")

# 用 python3 做 URL 编码 (含 passwd 等可能含特殊字符的字段)
QUERY=$(USER_ID="$USER_ID" PASSWORD="$PASSWORD" WLAN_USER_IP="$WLAN_USER_IP" \
WLAN_AC_NAME="$WLAN_AC_NAME" WLAN_AC_IP="$WLAN_AC_IP" SSID="$SSID" VLAN="$VLAN" \
MAC="$MAC" VERSION="$VERSION" PORTAL_PAGE_ID="$PORTAL_PAGE_ID" TIMESTAMP="$TIMESTAMP" \
UUID="$UUID" PORTAL_TYPE="$PORTAL_TYPE" HOSTNAME_VAL="$HOSTNAME_VAL" BIND_CTRL_ID="$BIND_CTRL_ID" \
python3 -c "
import urllib.parse, os
params = {
    'userid': os.environ['USER_ID'],
    'passwd': os.environ['PASSWORD'],
    'wlanuserip': os.environ['WLAN_USER_IP'],
    'wlanacname': os.environ['WLAN_AC_NAME'],
    'wlanacIp': os.environ['WLAN_AC_IP'],
    'ssid': os.environ['SSID'],
    'vlan': os.environ['VLAN'],
    'mac': os.environ['MAC'],
    'version': os.environ['VERSION'],
    'portalpageid': os.environ['PORTAL_PAGE_ID'],
    'timestamp': os.environ['TIMESTAMP'],
    'uuid': os.environ['UUID'],
    'portaltype': os.environ['PORTAL_TYPE'],
    'hostname': os.environ['HOSTNAME_VAL'],
    'bindCtrlId': os.environ['BIND_CTRL_ID'],
}
print(urllib.parse.urlencode(params))
")

REQUEST_URL="${AUTH_URL}?${QUERY}"
log_info "[*] 步骤 2 完成"
log_info ""

# 发送 (两层降级: curl → python3 sender.py)
# 完整 URL 通过 stdin 传给 sender, 避免明文密码出现在进程命令行
log_info "[*] 步骤 3: 发送认证请求 (两层降级)..."
RESPONSE=""

# 第 1 层: curl (默认主力, 带 noproxy 避免系统代理干扰)
if curl --version >/dev/null 2>&1; then
    log_info "    [第 1 层] curl..."
    HTTP_CODE=$(curl -s -o /tmp/xywdl_response.txt -w "%{http_code}" \
        --max-redirs 0 --noproxy '*' --max-time 30 \
        "$REQUEST_URL" 2>/dev/null || echo "000")
    # 000 = 连接层失败; 4xx/5xx 也算"发出去了", body 交给判定
    if [[ "$HTTP_CODE" != "000" ]]; then
        RESPONSE=$(cat /tmp/xywdl_response.txt 2>/dev/null || echo "")
        rm -f /tmp/xywdl_response.txt
        log_info "    [第 1 层] 成功, HTTP 状态码: $HTTP_CODE"
    else
        log_warn "    [第 1 层] curl 发送失败 (HTTP_CODE=000), 降级到 python3..."
    fi
else
    log_warn "    未找到 curl, 直接使用 python3..."
fi

# 第 2 层: python3 sender.py (纯标准库, 跨平台最强保底)
if [[ -z "$RESPONSE" ]]; then
    log_info "    [第 2 层] python3 sender.py..."
    PY_SENDER="$SCRIPT_DIR/src/sender/sender.py"
    if [[ ! -f "$PY_SENDER" ]]; then
        PY_SENDER="$SCRIPT_DIR/sender.py"
    fi
    if command -v python3 >/dev/null 2>&1 && [[ -f "$PY_SENDER" ]]; then
        if RESPONSE=$(printf '%s' "$REQUEST_URL" | python3 "$PY_SENDER" 2>/tmp/xywdl_py_err.txt); then
            log_info "    [第 2 层] 成功"
            rm -f /tmp/xywdl_py_err.txt
        else
            PY_ERR=$(cat /tmp/xywdl_py_err.txt 2>/dev/null)
            rm -f /tmp/xywdl_py_err.txt
            log_error "[!] 卡在: 步骤 3 - python3 sender.py 失败: $PY_ERR"
            exit 99
        fi
    else
        log_error "[!] 卡在: 步骤 3 - 所有发送层均失败 (无 curl 且无 python3/sender.py)"
        exit 99
    fi
fi
log_info "[*] 步骤 3 完成"
log_info ""

log_info "[*] 步骤 4: 判定认证结果..."
log_info "    响应: $RESPONSE"

# 判定结果 (跟 PS 端一致)
# 注意: code 匹配必须锚定 "后面不能紧跟数字", 否则 "code":10/100/123 会被误判成
# "code":1 (账号不存在), "code":440 会被误判成 "code":44 (非法接入)。
# grep -E 不支持 lookahead, 用 ([^0-9]|$) 达到同样效果。
if echo "$RESPONSE" | grep -qE '"code"[[:space:]]*:[[:space:]]*0([^0-9]|$)' \
   || echo "$RESPONSE" | grep -q "success" \
   || echo "$RESPONSE" | grep -q "认证成功"; then
    log_success "[+] 认证成功,已连接到互联网"
    exit 0
elif echo "$RESPONSE" | grep -qE '"code"[[:space:]]*:[[:space:]]*1([^0-9]|$)' \
     || echo "$RESPONSE" | grep -q "账号不存在"; then
    log_error "[!] 卡在: 步骤 4 - 认证失败:账号不存在"
    exit 1
elif echo "$RESPONSE" | grep -qE '"code"[[:space:]]*:[[:space:]]*44([^0-9]|$)' \
     || echo "$RESPONSE" | grep -q "非法接入"; then
    log_error "[!] 卡在: 步骤 4 - 认证失败:非法接入 (VLAN/MAC 不匹配)"
    exit 44
else
    log_warn "[!] 卡在: 步骤 4 - 认证结果未知"
    exit 99
fi
