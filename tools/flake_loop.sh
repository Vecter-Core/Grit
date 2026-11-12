#!/bin/bash
# Chạy lặp MỘT test N lần để tìm flake (Seed 1.0 S7): ghi từng lượt vào <log> (tsv: lượt, mã thoát, giây) và LƯU ĐẦY ĐỦ đầu ra của lượt đỏ.
# Dùng: bash tools/flake_loop.sh <gói> <tệp-test> <tên-test> [N=100] [log=target/tmp/flake.tsv]   ví dụ: gt-driver llvm_sort random_arrays_with_duplicates_match_stable_sort 100
# Dựng test MỘT lần (cargo test --no-run) rồi chạy trực tiếp nhị phân test: không dựng lại giữa các lượt. Mã thoát 0 = mọi lượt xanh; 1 = có lượt đỏ.
set -u
cd "$(dirname "$0")/.." || exit 1
pkg=$1; tf=$2; name=$3; n=${4:-100}; log=${5:-target/tmp/flake.tsv}
mkdir -p "$(dirname "$log")"
bin=$(cargo test -p "$pkg" --test "$tf" --no-run --message-format=json 2>/dev/null | python3 -c "
import sys,json
exe=None
for l in sys.stdin:
    try: d=json.loads(l)
    except Exception: continue
    if d.get('reason')=='compiler-artifact' and d.get('executable') and d['target']['name']=='$tf': exe=d['executable']
print(exe or '')")
[ -n "$bin" ] || { echo "không dựng được test $tf"; exit 2; }
: > "$log"; bad=0
for i in $(seq 1 "$n"); do
  t0=$(date +%s)
  out=$("$bin" "$name" 2>&1); rc=$?
  t1=$(date +%s)
  printf '%s\t%s\t%s\n' "$i" "$rc" "$((t1-t0))" >> "$log"
  if [ "$rc" != 0 ]; then bad=$((bad+1)); echo "$out" > "${log%.tsv}.fail$i.txt"; fi
done
echo "FLAKE_LOOP $pkg/$tf/$name: $n lượt, $bad đỏ (log: $log)"
[ "$bad" = 0 ]
