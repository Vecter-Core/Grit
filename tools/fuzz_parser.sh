#!/usr/bin/env bash
# Chạy N phiên fuzz gt-parser, mỗi phiên T giây, giữ corpus giữa các phiên. Seed từ tests/.
# Điều kiện sạch: không crash/panic, không E9901 (target tự assert), luôn kết thúc (-timeout), mọi Diagnostic có span hợp lệ.
# Dùng:  tools/fuzz_parser.sh [số_phiên=6] [giây_mỗi_phiên=600]
# Windows: PATH phải có thư mục chứa clang_rt.asan_dynamic-x86_64.dll của MSVC (ADR-009).
set -u
N="${1:-6}"; T="${2:-600}"
cd "$(dirname "$0")/../crates/gt-parser" || exit 2
mkdir -p fuzz/logs fuzz/corpus/parse
for d in pass fail warn lexer; do cp ../../tests/$d/*.gt fuzz/corpus/parse/ 2>/dev/null; done
export ASAN_OPTIONS="quarantine_size_mb=8:allocator_may_return_null=1"
fails=0
for i in $(seq 1 "$N"); do
  log="fuzz/logs/session_$i.log"
  echo "== phiên $i/$N, ${T}s: $(date -u +%FT%TZ)" | tee -a fuzz/logs/summary.log
  cargo +${FUZZ_TOOLCHAIN:-nightly} fuzz run parse -- -max_total_time="$T" -max_len=1024 -timeout=10 -rss_limit_mb=3072 -print_final_stats=1 > "$log" 2>&1
  code=$?
  runs=$(grep -o 'number_of_executed_units: [0-9]*' "$log" | tail -1)
  echo "   exit=$code $runs" | tee -a fuzz/logs/summary.log
  if [ "$code" -ne 0 ]; then fails=$((fails+1)); fi
done
echo "== xong: $fails/$N phiên lỗi" | tee -a fuzz/logs/summary.log
exit "$fails"
