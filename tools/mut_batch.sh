#!/bin/bash
# Một LÔ NGẮN đột biến tự động, ĐỒNG BỘ, chạy tiếp được (lệnh Giang 2026-10-17 mục 0):
#   bash tools/mut_batch.sh <crate> <tệp.rs> <từ_dòng> <đến_dòng> <kết_quả.jsonl> [giây=420]
# Dùng `tools/mut.py run --resume --max-seconds`: bỏ qua đột biến đã có dòng, ghi từng đột biến ngay khi xong, dừng sạch giữa hai đột biến khi hết giờ.
# Dừng vì hết giờ ⇒ in "hết --max-seconds"; chạy lại CHÍNH XÁC cùng lệnh để tiếp tục. Xong lô ⇒ "0 đột biến cần chạy".
cd "$(dirname "$0")/.." || exit 2
bash tools/require_preflight.sh || exit 2   # S2.c: tự từ chối khi preflight FAIL
crate="$1"; file="$2"; a="$3"; b="$4"; out="$5"; secs="${6:-420}"
export PYTHONIOENCODING=utf-8 GRIT_LLVM_DIR="${GRIT_LLVM_DIR:-D:/Tools/llvm-20.1.8}" GRIT_RT_LIB="${GRIT_RT_LIB:-D:/Grit/runtime/gt-rt/target/release/gt_rt.lib}"
export PATH="$GRIT_LLVM_DIR/bin:$PATH"
export GRIT_MUT_TEST="cargo test -p $crate -- --test-threads=2"
lines="$(python - "$file" "$a" "$b" <<'PY'
import os, subprocess, sys
f, a, b = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
out = subprocess.run([sys.executable, 'tools/mut.py', 'list', f], capture_output=True, text=True, encoding='utf-8').stdout
ls = sorted({int(l.split(':')[1]) for l in out.splitlines() if l.count(':') >= 2 and l.split(':')[1].isdigit() and a <= int(l.split(':')[1]) <= b})
print(','.join(f'{os.path.basename(f)}:{n}' for n in ls))
PY
)"
[ -z "$lines" ] && { echo "không có đột biến tự động nào trong $file:$a-$b"; exit 0; }
export GRIT_MUT_LINES="$lines"
python tools/mut.py run --workers 1 --resume --max-seconds "$secs" --out "$out" "$file" < /dev/null
