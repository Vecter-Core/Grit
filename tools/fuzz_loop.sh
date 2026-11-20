#!/usr/bin/env bash
# Fuzz TÍCH LUỸ qua đêm bằng MỘT lệnh (P3.a, 2026-11-17):   bash tools/fuzz_loop.sh <tổng_giờ>        (vd 8 ⇒ chạy tối đa 8 giờ)
# - Windows: tự tìm thư mục chứa clang_rt.asan_dynamic-x86_64.dll của MSVC (vswhere) và đặt PATH.
# - Dựng fuzz MỘT lần (`cargo +nightly fuzz build`), rồi chạy phiên 10 phút (đổi bằng FUZZ_SESSION_SEC) XEN KẼ lexer / parser bằng tools/fuzz_session.sh.
# - MỖI vòng: preflight (thiếu RAM/ảo hoặc có cargo khác ⇒ ngủ 2 phút rồi thử lại, tối đa 30 lần/vòng), bọc track_guard (chỉ docs/FUZZ_LOG.jsonl được đổi).
# - docs/FUZZ_LOG.jsonl ghi kèm BĂM NGUỒN (`src`) của crate; dừng khi CẢ HAI đạt 24 giờ TRÊN BĂM HIỆN TẠI hoặc hết <tổng_giờ>.
# - Ctrl+C an toàn: dừng sau phiên đang chạy; chạy lại cùng lệnh để tiếp tục (tổng tích luỹ nằm trong FUZZ_LOG). In tổng mỗi vòng.
# Điều kiện #2 (GRIT_PLAN §7) chỉ được ghi "ĐẠT" khi `python tools/fuzz_total.py` báo ≥ 24 giờ MỖI loại trên băm hiện tại.
set -u
HOURS="${1:-8}"
# GÓI 4 S5.b: `--parallel` chạy lexer và parser ĐỒNG THỜI (hai tiến trình riêng, mỗi mục tiêu có giờ đồng hồ thật riêng) NẾU RAM/bộ nhớ ảo trống đủ
# (FUZZ_PAR_MIN_GB, mặc định 10); không đủ ⇒ tự lùi về chế độ lần lượt. `--plan` chỉ in kế hoạch (MODE=...) rồi thoát (dùng cho test).
WANT_PAR=0; PLAN_ONLY=0
for a in "$@"; do [ "$a" = "--parallel" ] && WANT_PAR=1; [ "$a" = "--plan" ] && PLAN_ONLY=1; done
free_gb() {
  if [ -n "${FUZZ_FAKE_FREE_GB:-}" ]; then echo "$FUZZ_FAKE_FREE_GB"; return; fi
  if command -v powershell.exe >/dev/null 2>&1 && [ "$(uname -s | cut -c1-5)" = "MINGW" ]; then
    kb=$(powershell.exe -NoProfile -Command "(Get-CimInstance Win32_OperatingSystem).FreeVirtualMemory" 2>/dev/null | tr -d '\r\n '); echo $(( ${kb:-0} / 1048576 ))
  else
    kb=$(awk '/^MemAvailable:/ {print $2}' /proc/meminfo 2>/dev/null); echo $(( ${kb:-0} / 1048576 ))
  fi
}
PAR_MIN="${FUZZ_PAR_MIN_GB:-10}"
MODE=sequential; MODE_WHY="không đặt --parallel"
if [ "$WANT_PAR" = 1 ]; then
  g=$(free_gb)
  if [ "$g" -ge "$PAR_MIN" ]; then MODE=parallel; MODE_WHY="trống ${g} GB >= ${PAR_MIN} GB"; else MODE_WHY="trống ${g} GB < ${PAR_MIN} GB ⇒ lùi về lần lượt"; fi
fi
echo "fuzz_loop: MODE=$MODE ($MODE_WHY)"
[ "$PLAN_ONLY" = 1 ] && exit 0
SEC="${FUZZ_SESSION_SEC:-600}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT" || exit 2
export PYTHONIOENCODING=utf-8
# --- ASan DLL (Windows) ---
if command -v cygpath >/dev/null 2>&1; then
  VSW="/c/Program Files (x86)/Microsoft Visual Studio/Installer/vswhere.exe"
  if [ -x "$VSW" ]; then
    VS="$("$VSW" -latest -property installationPath 2>/dev/null | tr -d '\r')"
    DLL="$(find "$(cygpath -u "$VS")/VC/Tools/MSVC" -path '*Hostx64/x64/clang_rt.asan_dynamic-x86_64.dll' 2>/dev/null | sort | tail -1)"
    if [ -n "$DLL" ]; then export PATH="$(dirname "$DLL"):$PATH"; echo "fuzz_loop: ASan DLL $(dirname "$DLL")"; else echo "fuzz_loop: KHÔNG thấy clang_rt.asan_dynamic — dừng"; exit 3; fi
  else echo "fuzz_loop: thiếu vswhere — dừng"; exit 3; fi
fi
STOP=0
trap 'STOP=1; echo "fuzz_loop: nhận ngắt — dừng sau phiên đang chạy"' INT TERM
echo "fuzz_loop: dựng fuzz một lần…"
( cd crates/gt-lexer && cargo +nightly fuzz build lex ) >/dev/null 2>&1 || { echo "fuzz_loop: dựng lex THẤT BẠI"; exit 4; }
( cd crates/gt-parser && cargo +nightly fuzz build parse ) >/dev/null 2>&1 || { echo "fuzz_loop: dựng parse THẤT BẠI"; exit 4; }
. "$(dirname "${BASH_SOURCE[0]}")/pyfind.sh"; [ -n "$PY" ] || exit 2
START=$(date +%s); LIMIT=$($PY -c "print(int(float('$HOURS')*3600))")
if [ "$MODE" = parallel ]; then
  bash tools/preflight.sh "$ROOT" >/dev/null 2>&1 || { echo "fuzz_loop: preflight đầy đủ FAIL trước khi chạy song song — dừng"; exit 2; }
  one_kind() { # $1 = lex|parse ; vòng riêng, kiểm sức khoẻ kho + RAM mỗi vòng (không kiểm "có cargo khác" vì tiến trình bạn cũng là cargo)
    local k="$1" tries=0 left s out code
    while [ "$STOP" = 0 ] && [ $(( $(date +%s) - START )) -lt "$LIMIT" ]; do
      $PY tools/fuzz_total.py --kind-done "$k" && { echo "fuzz_loop[$k]: đạt 24 giờ trên băm hiện tại — xong"; return 0; }
      if ! PREFLIGHT_SKIP_RESOURCES=1 bash tools/preflight.sh "$ROOT" >/dev/null 2>&1 || [ "$(free_gb)" -lt $(( PAR_MIN / 2 )) ]; then
        tries=$((tries+1)); [ "$tries" -ge 30 ] && { echo "fuzz_loop[$k]: kho/RAM chưa đạt 30 lần — dừng"; return 1; }
        echo "fuzz_loop[$k]: kho hoặc RAM chưa đạt ($tries/30) — ngủ 120 s"; sleep 120; continue
      fi
      tries=0
      left=$(( LIMIT - ( $(date +%s) - START ) )); s=$SEC; [ "$left" -lt "$s" ] && s=$left; [ "$s" -lt 30 ] && return 0
      out="$(FUZZ_PEER=1 bash tools/fuzz_session.sh "$k" "$s" 2>&1)"; code=$?
      echo "[$k] $(echo "$out" | tail -1)"
      [ "$code" -ne 0 ] && { echo "fuzz_loop[$k]: phiên LỖI (exit $code) — dừng; điều tra artifacts"; return "$code"; }
    done
    return 0
  }
  one_kind lex & P1=$!
  one_kind parse & P2=$!
  wait $P1; R1=$?; wait $P2; R2=$?
  $PY tools/fuzz_total.py
  echo "fuzz_loop: kết thúc song song sau $(( $(date +%s) - START )) s (lex=$R1 parse=$R2)"
  [ "$R1" = 0 ] && [ "$R2" = 0 ] && exit 0
  exit 1
fi
i=0
while [ "$STOP" = 0 ] && [ $(( $(date +%s) - START )) -lt "$LIMIT" ]; do
  if $PY tools/fuzz_total.py --done-on-current-hash >/dev/null 2>&1; then echo "fuzz_loop: CẢ HAI đạt 24 giờ trên băm hiện tại — xong"; break; fi
  tries=0
  until bash tools/preflight.sh "$ROOT" >/dev/null 2>&1 && ! tasklist 2>/dev/null | grep -qiE "^(cargo|rustc|link|gritc)\.exe"; do
    tries=$((tries+1)); [ "$tries" -ge 30 ] && { echo "fuzz_loop: preflight/ máy bận 30 lần — dừng"; STOP=1; break; }
    echo "fuzz_loop: preflight chưa đạt/máy bận ($tries/30) — ngủ 120 s"; sleep 120; [ "$STOP" = 1 ] && break
  done
  [ "$STOP" = 1 ] && break
  if [ $((i % 2)) = 0 ]; then k=lex; else k=parse; fi; i=$((i+1))
  left=$(( LIMIT - ( $(date +%s) - START ) )); s=$SEC; [ "$left" -lt "$s" ] && s=$left
  [ "$s" -lt 30 ] && break
  out="$(bash tools/fuzz_session.sh "$k" "$s" 2>&1)"; code=$?
  echo "$out" | tail -2
  if [ "$code" -ne 0 ]; then echo "fuzz_loop: phiên $k LỖI (exit $code) — dừng; điều tra artifacts trước khi chạy tiếp"; $PY tools/fuzz_total.py; exit "$code"; fi
  $PY tools/fuzz_total.py
done
$PY tools/fuzz_total.py
echo "fuzz_loop: kết thúc sau $(( $(date +%s) - START )) s"
