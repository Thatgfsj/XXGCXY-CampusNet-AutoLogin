#!/bin/bash
# 新乡工程学院校园网登录脚本 (Linux版)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONFIG_FILE="$HOME/.config/xxgcxy-wifi/login_config.json"

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
WHITE='\033[1;37m'
NC='\033[0m' # No Color

log_info() { echo -e "${CYAN}$1${NC}"; }
log_success() { echo -e "${GREEN}$1${NC}"; }
log_warn() { echo -e "${YELLOW}$1${NC}"; }
log_error() { echo -e "${RED}$1${NC}"; }

echo -e "${CYAN}"
echo "========================================"
echo "  新乡工程学院校园网登录脚本 (Linux版)"
echo "========================================"
echo -e "${NC}"

# 加载配置
load_config() {
    if [[ -f "$CONFIG_FILE" ]]; then
        CONFIG=$(cat "$CONFIG_FILE")
        BASE_URL=$(echo "$CONFIG" | grep -o '"BaseURL":"[^"]*"' | cut -d'"' -f4)
        WLAC_NAME=$(echo "$CONFIG" | grep -o '"WlanAcName":"[^"]*"' | cut -d'"' -f4)
        WLAC_IP=$(echo "$CONFIG" | grep -o '"WlanAcIp":"[^"]*"' | cut -d'"' -f4)
        VLAN=$(echo "$CONFIG" | grep -o '"Vlan":"[^"]*"' | cut -d'"' -f4)
        WLAC_USER_IP=$(echo "$CONFIG" | grep -o '"WlanUserIp":"[^"]*"' | cut -d'"' -f4)
        MAC=$(echo "$CONFIG" | grep -o '"MacAddress":"[^"]*"' | cut -d'"' -f4)
        PASSWORD=$(echo "$CONFIG" | grep -o '"Password":"[^"]*"' | cut -d'"' -f4)
        USER_ID=$(echo "$CONFIG" | grep -o '"UserId":"[^"]*"' | cut -d'"' -f4)
        return 0
    fi
    return 1
}

# 保存配置
save_config() {
    mkdir -p "$(dirname "$CONFIG_FILE")"
    cat > "$CONFIG_FILE" << EOF
{
    "BaseURL": "$BASE_URL",
    "WlanAcName": "$WLAC_NAME",
    "WlanAcIp": "$WLAC_IP",
    "Vlan": "$VLAN",
    "WlanUserIp": "$WLAC_USER_IP",
    "MacAddress": "$MAC",
    "UserId": "$USER_ID",
    "Password": "$PASSWORD"
}
EOF
    chmod 600 "$CONFIG_FILE"
    log_success "配置已保存到: $CONFIG_FILE"
}

# 获取本机IP
get_local_ip() {
    ip route get 1 | grep -oP 'src \K[^ ]+' 2>/dev/null || \
    ip addr show | grep 'inet ' | grep -v '127.0.0.1' | head -1 | awk '{print $2}' | cut -d'/' -f1
}

# 获取MAC地址
get_mac_address() {
    local iface=$(ip route get 1 2>/dev/null | grep -oP 'dev \K[^ ]+' | head -1)
    if [[ -n "$iface" ]]; then
        cat "/sys/class/net/$iface/address" 2>/dev/null | tr '[:lower:]' ':' || echo ""
    fi
}

# 自动检测参数
auto_detect_params() {
    log_info "正在尝试自动获取登录参数..."

    local response=$(curl -s -o /dev/null -w "%{redirect_url}" --max-redirs 0 http://www.qq.com 2>/dev/null)

    if [[ -n "$response" && "$response" != "http://www.qq.com"* ]]; then
        log_info "捕获到重定向: $response"
        parse_redirect_url "$response"
        return 0
    fi

    local local_ip=$(get_local_ip)
    local local_mac=$(get_mac_address)

    response=$(curl -s -o /dev/null -w "%{redirect_url}" --max-redirs 0 --max-time 5 "http://172.18.252.12:6060" 2>/dev/null)
    if [[ -n "$response" ]]; then
        log_info "捕获到重定向: $response"
        parse_redirect_url "$response"
        if [[ -z "$WLAC_USER_IP" ]]; then WLAC_USER_IP="$local_ip"; fi
        if [[ -z "$MAC" ]]; then MAC="$local_mac"; fi
        return 0
    fi

    return 1
}

# 解析重定向URL
parse_redirect_url() {
    local url="$1"
    local decoded=$(python3 -c "import urllib.parse; print(urllib.parse.unquote('$url'))" 2>/dev/null || echo "$url")

    BASE_URL=$(echo "$decoded" | grep -oP '^(http://[^/]+(/\w+\.do))' | head -1)
    WLAC_USER_IP=$(echo "$decoded" | grep -oP 'wlanuserip=([^&]+)' | cut -d'=' -f2)
    WLAC_NAME=$(echo "$decoded" | grep -oP 'wlanacname=([^&]+)' | cut -d'=' -f2)
    WLAC_IP=$(echo "$decoded" | grep -oP 'wlanacIp=([^&]+)' | cut -d'=' -f2)
    MAC=$(echo "$decoded" | grep -oP 'mac=([^&]+)' | cut -d'=' -f2 | tr '[:upper:]' ':')
    VLAN=$(echo "$decoded" | grep -oP 'vlan=([^&]+)' | cut -d'=' -f2)
}

# 手动输入参数
manual_input() {
    echo -e "${YELLOW}请按以下步骤操作：${NC}"
    echo "1. 连接校园网（如果已经登录访问 2.2.2.2 来退出）"
    echo "2. 打开浏览器访问任意网站（如 www.qq.com）"
    echo "3. 浏览器会自动重定向到登录页面"
    echo "4. 复制浏览器地址栏中的完整URL地址"
    echo "5. 将URL粘贴到下面"
    echo

    read -p "请粘贴校园网登录链接: " manual_url

    while [[ -z "$manual_url" || "$manual_url" != *"/portal.do"* ]]; do
        log_error "URL格式不正确，请输入包含 /portal.do 的重定向URL"
        read -p "请重新粘贴重定向URL: " manual_url
    done

    parse_redirect_url "$manual_url"
    log_success "URL解析成功！"
}

# 选择运营商
select_operator() {
    echo -e "${YELLOW}请选择运营商:${NC}"
    echo "  1. 移动 (@xxgcyd)"
    echo "  2. 联通 (@xxgclt)"
    echo "  3. 电信 (@xxgcdx)"

    while true; do
        read -p "请输入对应数字 (1/2/3): " choice
        case $choice in
            1) SUFFIX="@xxgcyd"; OPERATOR="移动"; break;;
            2) SUFFIX="@xxgclt"; OPERATOR="联通"; break;;
            3) SUFFIX="@xxgcdx"; OPERATOR="电信"; break;;
            *) log_error "无效选择，请重新输入";;
        esac
    done
}

# 输入账号
input_credentials() {
    select_operator

    read -p "请输入学号（纯数字）: " student_id
    while ! [[ "$student_id" =~ ^[0-9]+$ ]] || [[ -z "$student_id" ]]; do
        log_error "学号必须是纯数字！"
        read -p "请重新输入学号: " student_id
    done

    USER_ID="${student_id}${SUFFIX}"
    log_info "完整账号: $USER_ID ($OPERATOR)"

    read -s -p "请输入校园网密码: " password
    echo
    read -s -p "请再次输入密码确认: " password2
    echo

    while [[ -z "$password" ]]; do
        log_error "密码不能为空！"
        read -s -p "请输入校园网密码: " password
        echo
    done

    while [[ "$password" != "$password2" ]]; do
        log_error "两次输入的密码不一致！"
        read -s -p "请输入校园网密码: " password
        echo
        read -s -p "请再次输入密码确认: " password2
        echo
    done

    PASSWORD="$password"
}

# 显示网络信息
display_info() {
    echo -e "${CYAN}--- 当前网络信息 ---${NC}"
    echo -e "  认证地址: ${WHITE}${BASE_URL}${NC}"
    echo -e "  AC名称:   ${WHITE}${WLAC_NAME}${NC}"
    echo -e "  用户IP:  ${WHITE}${WLAC_USER_IP}${NC}"
    echo -e "  MAC地址: ${WHITE}${MAC}${NC}"
    echo -e "  VLAN:    ${WHITE}${VLAN}${NC}"
}

# 执行认证
do_authenticate() {
    log_info "正在执行认证..."

    local local_ip=$(get_local_ip)
    local local_mac=$(get_mac_address)

    if [[ -n "$local_ip" ]]; then WLAC_USER_IP="$local_ip"; fi
    if [[ -n "$local_mac" ]]; then MAC="$local_mac"; fi

    local auth_url="${BASE_URL/portal.do/quickauth.do}"

    local timestamp=$(date +%s000)
    local uuid=$(cat /proc/sys/kernel/random/uuid)

    local encoded_userid=$(python3 -c "import urllib.parse; print(urllib.parse.quote('$USER_ID'))" 2>/dev/null)
    local encoded_pwd=$(python3 -c "import urllib.parse; print(urllib.parse.quote('$PASSWORD'))" 2>/dev/null)
    local encoded_wlacname=$(python3 -c "import urllib.parse; print(urllib.parse.quote('$WLAC_NAME'))" 2>/dev/null)
    local encoded_hostname=$(python3 -c "import urllib.parse; print(urllib.parse.quote('$HOSTNAME'))" 2>/dev/null)

    local query="userid=${encoded_userid}&passwd=${encoded_pwd}&wlanuserip=${WLAC_USER_IP}&wlanacname=${encoded_wlacname}&wlanacIp=${WLAC_IP}&vlan=${VLAN}&mac=${MAC}&version=0&portalpageid=3&timestamp=${timestamp}&uuid=${uuid}&portaltype=0&hostname=${encoded_hostname}"

    local full_url="${auth_url}?${query}"
    log_info "请求地址: $full_url"

    local response=$(curl -s -w "\n%{http_code}" "$full_url" 2>&1)
    local http_code=$(echo "$response" | tail -1)
    local body=$(echo "$response" | head -n -1)

    echo -e "${CYAN}=== 认证响应 ===${NC}"
    echo -e "HTTP状态码: ${GREEN}${http_code}${NC}"
    echo -e "响应内容: ${WHITE}${body}${NC}"

    if echo "$body" | grep -q '"code":0' || echo "$body" | grep -q "success" || echo "$body" | grep -q "认证成功"; then
        echo
        log_success "认证成功！您已连接到互联网。"
        log_info "账号: $USER_ID"
        return 0
    elif echo "$body" | grep -q '"code":1' || echo "$body" | grep -q "账号不存在"; then
        log_error "认证失败：账号不存在，请检查学号和运营商是否正确"
    elif echo "$body" | grep -q '"code":44' || echo "$body" | grep -q "非法接入"; then
        log_error "认证失败：非法接入，请检查VLAN ID或MAC地址是否正确"
    else
        log_warn "认证结果未知，请检查账号密码是否正确"
    fi

    return 1
}

# 主流程
main() {
    if load_config; then
        log_info "已找到保存的配置，自动登录中..."
        display_info
        if do_authenticate; then
            exit 0
        fi
    fi

    log_info "=== 步骤1：自动获取登录参数 ==="
    if auto_detect_params; then
        display_info
    else
        log_warn "自动获取参数失败，请手动输入"
        manual_input
        display_info
        input_credentials

        read -p "是否保存配置？(y/N): " save
        if [[ "$save" == "y" || "$save" == "Y" ]]; then
            save_config
        fi
    fi

    if [[ -z "$PASSWORD" ]]; then
        input_credentials
        save_config
    fi

    echo
    log_info "=== 步骤3：开始认证 ==="
    do_authenticate

    read -p "按 Enter 键退出脚本" dummy
}

main
