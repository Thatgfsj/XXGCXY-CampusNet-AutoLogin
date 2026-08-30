#!/bin/bash
# tests/test_sh_judge.sh — 验证 xywdl.sh 认证结果判定逻辑 (grep 锚定)
# 直接内联复刻 xywdl.sh 里的判定段落 (已修复版)
set -u
PASS=0; FAIL=0

judge() {
  local RESPONSE="$1"
  if echo "$RESPONSE" | grep -qE '"code"[[:space:]]*:[[:space:]]*0([^0-9]|$)' \
     || echo "$RESPONSE" | grep -q "success" \
     || echo "$RESPONSE" | grep -q "认证成功"; then
    echo "success"
  elif echo "$RESPONSE" | grep -qE '"code"[[:space:]]*:[[:space:]]*1([^0-9]|$)' \
       || echo "$RESPONSE" | grep -q "账号不存在"; then
    echo "fail1"
  elif echo "$RESPONSE" | grep -qE '"code"[[:space:]]*:[[:space:]]*44([^0-9]|$)' \
       || echo "$RESPONSE" | grep -q "非法接入"; then
    echo "fail44"
  else
    echo "unknown"
  fi
}

check() {
  local got; got=$(judge "$1")
  if [ "$got" = "$2" ]; then PASS=$((PASS+1)); echo "  [PASS] $3"
  else FAIL=$((FAIL+1)); echo "  [FAIL] $3 got=$got want=$2"; fi
}

check '{"code":0,"msg":"ok"}' success "code=0 认证成功"
check '{"code":1,"msg":"账号不存在"}' fail1 "code=1 账号不存在"
check '{"code":44,"msg":"非法接入"}' fail44 "code=44 非法接入"
check '{"code":99,"msg":"未知"}' unknown "code=99 未知"
check '{"code":10,"msg":"参数错误"}' unknown "code=10 边界:不应判为账号不存在"
check '{"code":100,"msg":"服务器错误"}' unknown "code=100 边界"
check '{"code":123,"msg":"其他"}' unknown "code=123 边界"
check '{"code":440,"msg":"VLAN校验"}' unknown "code=440 边界:不应判为非法接入"
check 'success' success "明文 success"
check '{"info":"账号不存在"}' fail1 "明文 账号不存在"
check '{"info":"非法接入"}' fail44 "明文 非法接入"

echo ""
echo "===== 结果: $PASS 通过, $FAIL 失败 ====="
[ $FAIL -eq 0 ]
