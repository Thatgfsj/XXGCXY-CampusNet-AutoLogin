#!/usr/bin/env bash
# ============================================================
#  tests/test_sh_judge.sh
#  真机调用 xywdl.sh 执行端到端认证与边界判定测试
#
#  覆盖:
#    A. 缺失/损坏配置 (退出码 2/3)
#    B. 认证结果判定 (code 0/1/44/99/10/100/123/440 等)
#    C. 真实 AC 错误与成功响应
#    D. 302 劫持与假 200 页面安全防御 (非 0 退出)
# ============================================================
set -u

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
XYWDL_SH="$PROJECT_DIR/xywdl.sh"

PYTHON=""
if python3 -c 'import sys' >/dev/null 2>&1; then
    PYTHON="python3"
elif python -c 'import sys' >/dev/null 2>&1; then
    PYTHON="python"
else
    echo "[!] 未找到可用的 Python 解释器"
    exit 1
fi

export http_proxy="" https_proxy="" all_proxy="" HTTP_PROXY="" HTTPS_PROXY="" ALL_PROXY=""

to_native_path() {
    if command -v cygpath >/dev/null 2>&1; then
        cygpath -w "$1"
    else
        echo "$1"
    fi
}

TEST_ROOT=$(mktemp -d 2>/dev/null || mktemp -d -t 'xywdl_sh_test')
CODEFILE="$TEST_ROOT/mock_code.txt"
MOCK_LOG="$TEST_ROOT/mock.log"
PORT=18099

echo "0" > "$CODEFILE"

# 启动 mock 服务器 (动态通过 codefile 切换响应)
$PYTHON "$(to_native_path "$SCRIPT_DIR/mock_portal.py")" --port $PORT --codefile "$(to_native_path "$CODEFILE")" --log "$(to_native_path "$MOCK_LOG")" >/dev/null 2>&1 &
MOCK_PID=$!

cleanup() {
    if [ -n "${MOCK_PID:-}" ]; then
        kill $MOCK_PID 2>/dev/null || true
    fi
    rm -rf "$TEST_ROOT" 2>/dev/null || true
}
trap cleanup EXIT INT TERM

# 等待 mock 就绪
READY=0
for i in $(seq 1 30); do
    if curl -s -m 1 --noproxy '*' "http://127.0.0.1:$PORT/healthz" >/dev/null 2>&1; then
        READY=1
        break
    fi
    sleep 0.1
done

if [ $READY -ne 1 ]; then
    echo "[!] Mock portal 启动失败 (端口 $PORT)"
    exit 1
fi

PASS=0
FAIL=0
FAILURES=()

assert_eq() {
    local got="$1"
    local want="$2"
    local name="$3"
    if [ "$got" -eq "$want" ]; then
        PASS=$((PASS + 1))
        echo "  [PASS] $name"
    else
        FAIL=$((FAIL + 1))
        FAILURES+=("$name (got=$got, want=$want)")
        echo "  [FAIL] $name got=$got want=$want"
    fi
}

assert_ne() {
    local got="$1"
    local bad="$2"
    local name="$3"
    if [ "$got" -ne "$bad" ]; then
        PASS=$((PASS + 1))
        echo "  [PASS] $name"
    else
        FAIL=$((FAIL + 1))
        FAILURES+=("$name (got=$got, expected not $bad)")
        echo "  [FAIL] $name got=$got expected not $bad"
    fi
}

create_profile() {
    local target_dir="$1"
    local base_url="${2:-http://127.0.0.1:$PORT/portal.do}"
    local user_id="${3:-2021110101@xxgcyd}"
    local vlan="${4:-100}"
    mkdir -p "$target_dir/.config/xxgcxy-wifi"
    cat <<EOF > "$target_dir/.config/xxgcxy-wifi/login_profile.json"
{
  "user_id": "$user_id",
  "operator": "yd",
  "ssid": "XXGC-WiFi",
  "base_url": "$base_url",
  "wlan_ac_name": "XXGC-AC",
  "wlan_ac_ip": "172.18.252.1",
  "vlan": "$vlan",
  "wlan_user_ip": "10.0.0.88",
  "mac_address": "aa:bb:cc:dd:ee:ff",
  "portal_page_id": "3",
  "portal_type": "0",
  "version": "0",
  "bind_ctrl_id": "",
  "hostname": "TEST-PC",
  "updated_at": "2026-08-30T00:00:00"
}
EOF
}

create_cred() {
    local target_dir="$1"
    local password="${2-TestPass123}"
    mkdir -p "$target_dir/.config/xxgcxy-wifi"
    printf "%s" "$password" > "$target_dir/.config/xxgcxy-wifi/login_credential.bin"
}

echo "===== A. 缺失/损坏配置 (退出码) ====="
# A1 缺失 profile.json -> exit 2
DIR_A1="$TEST_ROOT/case_a1"
mkdir -p "$DIR_A1"
HOME="$DIR_A1" XDG_CONFIG_HOME="$DIR_A1/.config" bash "$XYWDL_SH" >/dev/null 2>&1 || RET=$?
assert_eq "${RET:-0}" 2 "A1 缺失 profile.json → exit 2"

# A2 存在 profile 但缺失 credential.bin -> exit 2
DIR_A2="$TEST_ROOT/case_a2"
create_profile "$DIR_A2"
RET=0
HOME="$DIR_A2" XDG_CONFIG_HOME="$DIR_A2/.config" bash "$XYWDL_SH" >/dev/null 2>&1 || RET=$?
assert_eq "${RET:-0}" 2 "A2 缺失 credential.bin → exit 2"

# A3 credential.bin 为空 -> exit 3
DIR_A3="$TEST_ROOT/case_a3"
create_profile "$DIR_A3"
create_cred "$DIR_A3" ""
RET=0
HOME="$DIR_A3" XDG_CONFIG_HOME="$DIR_A3/.config" bash "$XYWDL_SH" >/dev/null 2>&1 || RET=$?
assert_eq "${RET:-0}" 3 "A3 密码文件为空 → exit 3"

echo "===== B. 认证结果判定 (真调用 xywdl.sh) ====="
test_cases=(
    "0:0:code=0 认证成功"
    "1:1:code=1 账号不存在"
    "44:44:code=44 非法接入"
    "99:99:code=99 未知错误"
    "10:99:code=10 参数错误(边界:不应误判为exit 44或exit 1)"
    "100:99:code=100 服务器错误(边界:不应误判为exit 44或exit 1)"
    "123:99:code=123 其他错误(边界)"
    "440:99:code=440 VLAN校验(边界:不应误判为exit 44)"
    "success_text:99:明文 success (严格口径: 无 JSON code 不判成功, 与 ps1/Rust 对齐)"
    "auth_success_cn:99:明文 认证成功 (严格口径: 无 JSON code 不判成功)"
    "no_user_text:1:明文 账号不存在"
    "illegal_text:44:明文 非法接入"
    "ac_device_error:1:真实 AC 设备异常提示"
    "ac_string_zero:0:真实 AC 字符串 0 成功"
)

for item in "${test_cases[@]}"; do
    code_key="${item%%:*}"
    rest="${item#*:}"
    expect_code="${rest%%:*}"
    desc="${rest#*:}"

    echo "$code_key" > "$CODEFILE"
    CASE_DIR="$TEST_ROOT/case_$code_key"
    create_profile "$CASE_DIR"
    create_cred "$CASE_DIR"

    RET=0
    HOME="$CASE_DIR" XDG_CONFIG_HOME="$CASE_DIR/.config" bash "$XYWDL_SH" >/dev/null 2>&1 || RET=$?
    assert_eq "${RET:-0}" "$expect_code" "$desc"
done

echo "===== C. 302 劫持与假 200 页面安全防御 ====="
# C1 302 重定向 -> 不应判为成功 (exit code != 0)
echo "302_redirect" > "$CODEFILE"
DIR_C1="$TEST_ROOT/case_302"
create_profile "$DIR_C1"
create_cred "$DIR_C1"
RET=0
HOME="$DIR_C1" XDG_CONFIG_HOME="$DIR_C1/.config" bash "$XYWDL_SH" >/dev/null 2>&1 || RET=$?
assert_ne "${RET:-0}" 0 "C1 302 劫持绝不误判为成功 (exit code != 0)"

# C2 假 200 HTML 页面 -> 不应判为成功 (exit code != 0)
echo "fake_200_html" > "$CODEFILE"
DIR_C2="$TEST_ROOT/case_fake200"
create_profile "$DIR_C2"
create_cred "$DIR_C2"
RET=0
HOME="$DIR_C2" XDG_CONFIG_HOME="$DIR_C2/.config" bash "$XYWDL_SH" >/dev/null 2>&1 || RET=$?
assert_ne "${RET:-0}" 0 "C2 假 200 页面绝不误判为成功 (exit code != 0)"

# C3 含 "successfully" 假 200 页面 -> 不应判为成功 (exit code != 0)
echo "fake_successfully" > "$CODEFILE"
DIR_C3="$TEST_ROOT/case_fake_successfully"
create_profile "$DIR_C3"
create_cred "$DIR_C3"
RET=0
HOME="$DIR_C3" XDG_CONFIG_HOME="$DIR_C3/.config" bash "$XYWDL_SH" >/dev/null 2>&1 || RET=$?
assert_ne "${RET:-0}" 0 "C3 含 successfully 假 200 页面绝不误判为成功 (exit code != 0)"

echo "===== D. base_url 自带参数提纯 (CLI 直接粘贴整条认证 URL) ====="
# 提纯必须发生在必填校验之前：否则脏 URL 场景下脚本连请求都发不出去 (exit 2)
DIRTY_URL="http://127.0.0.1:$PORT/portal.do?wlanuserip=10.4.124.192&wlanacname=AuteWifi-XXGC&mac=f4:6a:dd:e5:4a:7b&vlan=31002201&hostname=TEST-PC&rand=100bd08f91d44a1&url=http%3A%2F%2Fwww.qq.com"
echo "0" > "$CODEFILE"

# assert_eq 用的是 -eq 数值比较, 字符串参数必须走这条
assert_str() {
    local got="$1"
    local want="$2"
    local name="$3"
    if [ "$got" = "$want" ]; then
        PASS=$((PASS + 1))
        echo "  [PASS] $name"
    else
        FAIL=$((FAIL + 1))
        FAILURES+=("$name (got=$got, want=$want)")
        echo "  [FAIL] $name got=$got want=$want"
    fi
}

# 只取 mock 记录里最后一条 quickauth.do 请求的指定参数 (排除嗅探请求干扰)
get_mock_param() {
    $PYTHON -c "
import json, sys
rs = []
for line in open(sys.argv[1], encoding='utf-8'):
    line = line.strip()
    if line:
        rs.append(json.loads(line))
qs = [r for r in rs if str(r.get('path', '')).endswith('quickauth.do')]
rec = qs[-1] if qs else (rs[-1] if rs else {})
print((rec.get('params') or {}).get(sys.argv[2], ''))
" "$(to_native_path "$MOCK_LOG")" "$1" 2>/dev/null || echo ""
}

# D1 脏 base_url + profile 的 vlan / wlan_ac_name / wlan_ac_ip 全空 -> 提纯后应认证成功
#    (vlan 必须用 sed 清空: create_profile 的 ${4:-100} 对空串仍会回落成 100)
DIR_D1="$TEST_ROOT/case_dirty_url"
create_profile "$DIR_D1" "$DIRTY_URL"
sed -i.bak 's|"wlan_ac_name": "XXGC-AC"|"wlan_ac_name": ""|; s|"wlan_ac_ip": "172.18.252.1"|"wlan_ac_ip": ""|; s|"vlan": "100"|"vlan": ""|' "$DIR_D1/.config/xxgcxy-wifi/login_profile.json"
grep -q '"vlan": ""' "$DIR_D1/.config/xxgcxy-wifi/login_profile.json" || echo "  [WARN] D1 profile 清空失败，用例会失真"
create_cred "$DIR_D1"
: > "$MOCK_LOG"
RET=0
HOME="$DIR_D1" XDG_CONFIG_HOME="$DIR_D1/.config" bash "$XYWDL_SH" >/dev/null 2>&1 || RET=$?
assert_eq "${RET:-0}" 0 "D1 脏 URL + 空 vlan/wlan_ac_name → 提纯后认证成功"
assert_str "$(get_mock_param wlanacname)" "AuteWifi-XXGC" "D1 wlanacname 由 URL 提纯补回"
assert_str "$(get_mock_param vlan)" "31002201" "D1 vlan 由 URL 提纯补回"

# D2 显式配置优先: profile 已写 vlan=4000 / wlan_ac_name=XXGC-AC 时, URL 参数不得覆盖
DIR_D2="$TEST_ROOT/case_dirty_url_conflict"
create_profile "$DIR_D2" "$DIRTY_URL" "2021110101@xxgcyd" "4000"
create_cred "$DIR_D2"
: > "$MOCK_LOG"
RET=0
HOME="$DIR_D2" XDG_CONFIG_HOME="$DIR_D2/.config" bash "$XYWDL_SH" >/dev/null 2>&1 || RET=$?
assert_eq "${RET:-0}" 0 "D2 显式配置场景仍认证成功"
assert_str "$(get_mock_param vlan)" "4000" "D2 显式 vlan 未被 URL 参数覆盖"
assert_str "$(get_mock_param wlanacname)" "XXGC-AC" "D2 显式 wlan_ac_name 未被覆盖"

echo ""
echo "===== 结果: $PASS 通过, $FAIL 失败 ====="
if [ $FAIL -gt 0 ]; then
    echo "失败项: ${FAILURES[*]}"
    exit 1
fi
exit 0
