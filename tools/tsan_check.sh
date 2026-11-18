#!/bin/bash
# ThreadSanitizer cho Grit (GÓI 4 S3; đường (b) của docs/TSAN_FEASIBILITY.md). CHỈ LINUX. Đây là BUILD PHÂN TÍCH: lấy IR do `gritc` sinh (GRITC_DUMP_IR, hồ sơ dev, ĐÃ qua
# Guard), chèn `sanitize_thread` (tools/tsan_inject.py), biên dịch bằng `clang -fsanitize=thread`, liên kết `gt-rt` dựng với TSan. KHÔNG phải binary đã qua Guard cuối.
#   bash tools/tsan_check.sh [N=50]      (N = số lần chạy mỗi mẫu)
# Đối chứng DƯƠNG trước (IR có đua dữ liệu đã biết PHẢI bị báo), đối chứng ÂM (ghi nguyên tử KHÔNG bị báo); đối chứng dương trượt ⇒ thoát 4 "KHÔNG KIỂM ĐƯỢC".
# Dòng kết quả có tiền tố TSAN_ để dán vào docs/TSAN_RESULTS.md. "Không báo gì" = "không phát hiện trong các mẫu đã chạy", KHÔNG phải "không có đua dữ liệu".
set -u
cd "$(dirname "$0")/.." || exit 2
N="${1:-50}"
GRITC="${GRITC:-}"
if [ -z "$GRITC" ]; then for c in target/debug/gritc target/release/gritc; do [ -x "$c" ] && GRITC="$c" && break; done; fi
[ -x "${GRITC:-/nonexistent}" ] || { echo "TSAN_RESULT KHÔNG KIỂM ĐƯỢC: không thấy gritc (đặt GRITC=...)"; exit 3; }
failed=0
CLANG="${CLANG:-clang-20}"
RTA="${TSAN_RT_LIB:-runtime/gt-rt/target/x86_64-unknown-linux-gnu/release/libgt_rt.a}"
OUT=target/tsan; rm -rf "$OUT"; mkdir -p "$OUT"
LIBS="-lpthread -ldl -lm -lgcc_s"
PY=python3; command -v python3 >/dev/null 2>&1 || PY=python
export TSAN_OPTIONS="${TSAN_OPTIONS:-halt_on_error=0 second_deadlock_stack=1}"
command -v "$CLANG" >/dev/null 2>&1 || { echo "TSAN_RESULT KHÔNG KIỂM ĐƯỢC: không có $CLANG"; exit 3; }
[ -f "$RTA" ] || { echo "TSAN_RESULT KHÔNG KIỂM ĐƯỢC: thiếu gt-rt dựng với TSan: $RTA"; exit 3; }
link() { "$CLANG" -fsanitize=thread -O1 -g "$1" "$RTA" $LIBS -o "$2" 2>"$2.link.log"; }
races() { grep -c "WARNING: ThreadSanitizer: data race" "$1" 2>/dev/null || true; }
echo "TSAN_ENV clang=$($CLANG --version | head -1) N=$N"
ctl_fail=0
for c in race_pthread race_gritrt_spawn norace_atomic_pthread; do
  $PY tools/tsan_inject.py tests/tsan/$c.ll > "$OUT/$c.ll"
  if ! link "$OUT/$c.ll" "$OUT/$c.exe"; then echo "TSAN_CONTROL $c LINK_FAIL"; head -5 "$OUT/$c.exe.link.log"; ctl_fail=1; continue; fi
  hits=0
  for i in 1 2 3; do "$OUT/$c.exe" > "$OUT/$c.out.$i" 2>&1; h=$(races "$OUT/$c.out.$i"); [ "${h:-0}" -gt 0 ] && hits=$((hits+1)); done
  case "$c" in
    norace*) if [ "$hits" = 0 ]; then echo "TSAN_CONTROL $c OK (không báo, đúng)"; else echo "TSAN_CONTROL $c BÁO SAI ($hits/3)"; ctl_fail=1; fi ;;
    *) if [ "$hits" -ge 1 ]; then echo "TSAN_CONTROL $c CAUGHT ($hits/3 lần chạy báo data race)"; else echo "TSAN_CONTROL $c MISSED"; ctl_fail=1; fi ;;
  esac
done
if [ "$ctl_fail" != 0 ]; then echo "TSAN_RESULT KHÔNG KIỂM ĐƯỢC: đối chứng không đạt (xem TSAN_CONTROL ở trên)"; exit 4; fi
total_reports=0
for s in threads two_locks; do
  src="tests/samples/$s.gt"
  want=$(grep -o "expect-result: .*" "$src" | head -1 | sed 's/expect-result: //')
  if ! GRITC_DUMP_IR="$OUT/$s.raw.ll" "$GRITC" build --backend=llvm --profile=dev -o "$OUT/$s.plain" "$src" > "$OUT/$s.build.log" 2>&1; then echo "TSAN_SAMPLE $s BUILD_FAIL"; tail -3 "$OUT/$s.build.log"; failed=$((failed+1)); continue; fi
  $PY tools/tsan_inject.py "$OUT/$s.raw.ll" > "$OUT/$s.tsan.ll"
  if ! link "$OUT/$s.tsan.ll" "$OUT/$s.exe"; then echo "TSAN_SAMPLE $s LINK_FAIL"; head -5 "$OUT/$s.exe.link.log"; failed=$((failed+1)); continue; fi
  reports=0; wrong=0; bad_exit=0
  for i in $(seq 1 "$N"); do
    rm -f "$OUT/res.txt"
    GRITRT_RESULT_FILE="$OUT/res.txt" timeout 120 "$OUT/$s.exe" > "$OUT/$s.out.$i" 2>&1; rc=$?
    [ "$rc" != 0 ] && [ "$rc" != 66 ] && bad_exit=$((bad_exit+1))
    h=$(races "$OUT/$s.out.$i"); reports=$((reports + ${h:-0}))
    got=$(cat "$OUT/res.txt" 2>/dev/null || true); [ -n "$want" ] && [ "$got" != "$want" ] && wrong=$((wrong+1))
    [ "${h:-0}" = 0 ] && rm -f "$OUT/$s.out.$i"
  done
  total_reports=$((total_reports + reports))
  echo "TSAN_SAMPLE $s runs=$N expect=$want wrong_result=$wrong bad_exit=$bad_exit data_race_reports=$reports"
done
[ "$failed" != 0 ] && { echo "TSAN_RESULT KHÔNG KIỂM ĐƯỢC: $failed mẫu không dựng/liên kết được"; exit 5; }
echo "TSAN_RESULT reports_total=$total_reports (đối chứng dương đã bắt; 'không báo' = không phát hiện trong các mẫu đã chạy)"
ls "$OUT"/*.out.* 2>/dev/null | head -3 | while read -r f; do echo "--- $f"; head -40 "$f"; done
exit 0
