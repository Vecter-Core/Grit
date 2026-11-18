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
CR="${TSAN_CTL_RUNS:-30}"   # số lần chạy MỖI đối chứng (GÓI 5 S1.b); tỉ lệ phát hiện = hits/CR
rate_lines=""
# run_rate EXE LABEL: chạy CR lần, đếm số lần có báo data race; đặt biến hits
run_rate() {
  hits=0
  for i in $(seq 1 "$CR"); do
    rm -f "$OUT/rate.res"
    GRITRT_RESULT_FILE="$OUT/rate.res" timeout 120 "$1" > "$OUT/rate.out" 2>&1
    h=$(races "$OUT/rate.out"); [ "${h:-0}" -gt 0 ] && hits=$((hits+1))
  done
}
# control NAME KIND(pos|neg) EXE: ghi tỉ lệ; pos phải có hits>=1; neg phải 0
control() {
  run_rate "$3"
  case "$2" in
    neg) if [ "$hits" = 0 ]; then echo "TSAN_CONTROL $1 OK (đối chứng âm: 0/$CR báo, đúng)"; else echo "TSAN_CONTROL $1 BÁO SAI ($hits/$CR)"; ctl_fail=1; fi ;;
    *) if [ "$hits" -ge 1 ]; then echo "TSAN_CONTROL $1 CAUGHT detection_rate=$hits/$CR"; else echo "TSAN_CONTROL $1 MISSED detection_rate=0/$CR (KHÔNG ĐỦ NHẠY)"; ctl_fail=1; fi
       rate_lines="$rate_lines $1=$hits/$CR" ;;
  esac
}
for c in race_pthread race_gritrt_spawn race_narrow_ir norace_atomic_pthread; do
  $PY tools/tsan_inject.py tests/tsan/$c.ll > "$OUT/$c.ll"
  if ! link "$OUT/$c.ll" "$OUT/$c.exe"; then echo "TSAN_CONTROL $c LINK_FAIL"; head -5 "$OUT/$c.exe.link.log"; ctl_fail=1; continue; fi
  case "$c" in norace*) control "$c" neg "$OUT/$c.exe" ;; *) control "$c" pos "$OUT/$c.exe" ;; esac
done
# Đối chứng qua ĐÚNG pipeline Grit (GÓI 5 S1.a/b): chương trình Grit -> IR của gritc -> (bỏ lệnh gọi khoá bằng tools/tsan_inject.py --drop-locks) -> TSan.
# Không bỏ khoá = đối chứng âm (cùng chương trình, cùng đường, PHẢI không báo).
grit_control() {  # NAME SRC
  n="$1"; src="$2"
  if ! GRITC_DUMP_IR="$OUT/$n.raw.ll" "$GRITC" build --backend=llvm --profile=dev -o "$OUT/$n.plain" "$src" > "$OUT/$n.build.log" 2>&1; then echo "TSAN_CONTROL $n BUILD_FAIL"; tail -3 "$OUT/$n.build.log"; ctl_fail=1; return; fi
  $PY tools/tsan_inject.py --drop-locks "$OUT/$n.raw.ll" > "$OUT/$n.nolock.ll"
  $PY tools/tsan_inject.py "$OUT/$n.raw.ll" > "$OUT/$n.lock.ll"
  if ! link "$OUT/$n.nolock.ll" "$OUT/$n.nolock.exe" || ! link "$OUT/$n.lock.ll" "$OUT/$n.lock.exe"; then echo "TSAN_CONTROL $n LINK_FAIL"; ctl_fail=1; return; fi
  control "grit_${n}_nolock" pos "$OUT/$n.nolock.exe"
  control "grit_${n}_lock" neg "$OUT/$n.lock.exe"
}
grit_control two_locks tests/samples/two_locks.gt
grit_control narrow_droplock tests/tsan/narrow_droplock.gt
echo "TSAN_RATES (đối chứng dương, $CR lần/đối chứng):$rate_lines"
if [ "$ctl_fail" != 0 ]; then echo "TSAN_RESULT KHÔNG KIỂM ĐƯỢC: đối chứng không đạt (xem TSAN_CONTROL ở trên)"; exit 4; fi
total_reports=0
SAMPLES="tests/samples/threads.gt tests/samples/two_locks.gt"
for f in tests/pass/E0501_ok_spawn_cell_by_value.gt tests/pass/E0501_ok_spawn_enum_array.gt tests/pass/E0501_ok_spawn_int.gt tests/pass/E0501_ok_spawn_struct.gt tests/pass/E0501_ok_static_atomic.gt tests/pass/E0501_ok_static_mutex.gt tests/pass/E0502_ok_spawn_worker.gt; do
  [ -f "$f" ] && grep -q "ƒ main()" "$f" && SAMPLES="$SAMPLES $f"
done
for src in $SAMPLES; do
  s=$(basename "$src" .gt)
  want=""
  if ! GRITC_DUMP_IR="$OUT/$s.raw.ll" "$GRITC" build --backend=llvm --profile=dev -o "$OUT/$s.plain" "$src" > "$OUT/$s.build.log" 2>&1; then echo "TSAN_SAMPLE $s BUILD_FAIL"; tail -3 "$OUT/$s.build.log"; failed=$((failed+1)); continue; fi
  $PY tools/tsan_inject.py "$OUT/$s.raw.ll" > "$OUT/$s.tsan.ll"
  if ! link "$OUT/$s.tsan.ll" "$OUT/$s.exe"; then echo "TSAN_SAMPLE $s LINK_FAIL"; head -5 "$OUT/$s.exe.link.log"; failed=$((failed+1)); continue; fi
  reports=0; wrong=0; bad_exit=0
  "$CLANG" -fsanitize=thread -O1 -S -emit-llvm "$OUT/$s.tsan.ll" -o "$OUT/$s.inst.ll" 2>/dev/null
  inst=$(grep -c "__tsan_" "$OUT/$s.inst.ll" 2>/dev/null || true)
  for i in $(seq 1 "$N"); do
    rm -f "$OUT/res.txt"
    GRITRT_RESULT_FILE="$OUT/res.txt" timeout 120 "$OUT/$s.exe" > "$OUT/$s.out.$i" 2>&1; rc=$?
    [ "$rc" != 0 ] && [ "$rc" != 66 ] && bad_exit=$((bad_exit+1))
    h=$(races "$OUT/$s.out.$i"); reports=$((reports + ${h:-0}))
    [ "${h:-0}" = 0 ] && rm -f "$OUT/$s.out.$i"
  done
  total_reports=$((total_reports + reports))
  echo "TSAN_SAMPLE $s runs=$N instrumented___tsan_refs=${inst:-0} bad_exit=$bad_exit data_race_reports=$reports"
done
[ "$failed" != 0 ] && { echo "TSAN_RESULT KHÔNG KIỂM ĐƯỢC: $failed mẫu không dựng/liên kết được"; exit 5; }
echo "TSAN_RESULT reports_total=$total_reports (đối chứng dương đã bắt; 'không báo' = không phát hiện trong các mẫu đã chạy)"
ls "$OUT"/*.out.* 2>/dev/null | head -3 | while read -r f; do echo "--- $f"; head -40 "$f"; done
exit 0
