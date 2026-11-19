#!/bin/bash
# gate_clean: chạy cổng TRONG một bản clone sạch (GÓI 10, S1.a). Băm cổng gồm tệp chưa theo dõi ⇒ tệp lạ trong kho gốc làm vô hiệu cổng; clone thì không.
#   bash tools/gate_clean.sh <commit> <thư_mục_clone> [kho=.]
# KHÔNG ghi vào kho gốc, KHÔNG xoá gì: thư mục đích phải CHƯA tồn tại hoặc rỗng, và nằm ngoài kho gốc.
# Biến: GATE_STAGES (chạy một phần), GATE_BUDGET_SEC, GATE_CLEAN_SKIP_PREFLIGHT=1 (chỉ kiểm thử công cụ này).
# In: GATE_CLEAN_COMMIT, rồi các dòng GATE_RESULT/GATE_TREE/TEST_PASSED/GATE_STAGE_TREE của cổng; thoát theo mã của cổng.
commit="$1"; dest="$2"; repo="${3:-.}"
[ -n "$commit" ] && [ -n "$dest" ] || { echo "dùng: gate_clean.sh <commit> <thư_mục> [kho]"; exit 2; }
repo="$(cd "$(cd "$repo" && git rev-parse --show-toplevel)" && pwd)" || exit 2
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
mkdir -p "$dest" || exit 2
absdest="$(cd "$dest" && pwd)"
case "$absdest/" in "$repo"/*) echo "GATE_CLEAN_FAIL: đích nằm TRONG kho gốc"; exit 2 ;; esac
[ -z "$(ls -A "$absdest")" ] || { echo "GATE_CLEAN_FAIL: thư mục đích không rỗng ($absdest); KHÔNG xoá giúp"; exit 2; }
full="$(git -C "$repo" rev-parse --verify "$commit^{commit}")" || { echo "GATE_CLEAN_FAIL: commit lạ $commit"; exit 2; }
git clone -q --no-hardlinks "$repo" "$absdest" || { echo "GATE_CLEAN_FAIL: clone lỗi"; exit 2; }
git -C "$absdest" -c advice.detachedHead=false checkout -q "$full" || { echo "GATE_CLEAN_FAIL: checkout lỗi"; exit 2; }
[ "$(git -C "$absdest" rev-parse HEAD)" = "$full" ] || { echo "GATE_CLEAN_FAIL: HEAD khác commit yêu cầu"; exit 2; }
echo "GATE_CLEAN_COMMIT=$full dir=$absdest"
if [ -z "${GATE_CLEAN_SKIP_PREFLIGHT:-}" ]; then bash "$absdest/tools/preflight.sh" "$absdest" || { echo "GATE_RESULT=RED"; echo "GATE_CLEAN_FAIL: preflight"; exit 2; }; fi
bash "$absdest/tools/gate.sh" "$absdest"
