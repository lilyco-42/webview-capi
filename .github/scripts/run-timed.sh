#!/usr/bin/env bash
# 便携版 `timeout` —— 给 CI 步骤 source 用。
#
# 为什么需要它：**macOS 的 GitHub runner 上没有 GNU `timeout`**（coreutils 不是
# 系统自带）。于是 CI 里那些
#
#     out=$(cd "$W/empty" && timeout 15 env PATH=/nonexistent "$LYCO" web 2>&1) || true
#
# 在 macOS 上等于「命令不存在」—— `$LYCO` **根本没被执行**，而 `timeout: command
# not found` 被 `2>&1` 收进了变量里，所以日志里连一行红字都看不到。
#
# 后果取决于判据长什么样，两种都很糟：
#   * 判据是「输出里**没有**某句话」→ 命令没跑当然也没有那句话 → **永远为真**；
#   * 判据是「输出里**有**某句话」→ 永远失败。
# 第 16 步（帮助文本 / lyco list / 实际派发三者一致）的 ③ 就是前一种：
# 它在 macOS 上从 2026-09-23 起一直是 success，但**什么都没查**。
#
# 用法与 GNU `timeout` 同形：
#
#     out=$(run_timed 15 env PATH=/nonexistent "$LYCO" web 2>&1) || true
#
# 返回被执行的命令的退出码；被超时杀掉时是 137（128 + SIGKILL）。
# 只在 bash 下使用（workflow 里都是 `shell: bash`）—— 用了 `local`。
#
# ⚠️ 每个用它的步骤都**必须**配一条自检。而且**只量耗时的自检是不够的**：
# 「什么都没执行就立刻返回」同样很快（macOS 上 `timeout: command not found`
# 就是 rc=127、耗时 0s）—— 那样你只是把一个「永远为真」换成了另一个。
# 三条一起查才有效：
#
#     t0=$(date +%s)
#     rc=0   # 超时返回 137；步骤是 set -e，不加 `|| rc=$?` 会在这里就退出
#     run_timed 2 bash -c 'touch "$1"; sleep 30' _ "$W/rtmark" >/dev/null 2>&1 || rc=$?
#     t1=$(date +%s)
#     [ -f "$W/rtmark" ] || { echo "❌ 没有真的执行命令（rc=$rc）"; exit 1; }
#     [ "$rc" -ge 128 ]  || { echo "❌ 返回 $rc，不是被掐掉的 137"; exit 1; }
#     [ $((t1 - t0)) -lt 15 ] || { echo "❌ 没能把 sleep 30 掐掉"; exit 1; }
run_timed() {
    local secs="$1"
    shift
    "$@" &
    local pid=$!
    # 看门狗：到点就把主命令 SIGKILL 掉
    ( sleep "${secs}"; kill -9 "${pid}" 2>/dev/null || true ) >/dev/null 2>&1 &
    local wd=$!
    local rc=0
    wait "${pid}" 2>/dev/null || rc=$?
    # 主命令已经结束 → 立刻收掉看门狗，别让它在若干秒后去杀一个可能已被复用的 pid
    kill -9 "${wd}" 2>/dev/null || true
    wait "${wd}" 2>/dev/null || true
    return "${rc}"
}
