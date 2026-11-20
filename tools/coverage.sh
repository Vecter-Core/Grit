#!/bin/bash
# ĐỘ PHỦ NHÁNH của các crate quyết định của checker (GÓI 4 S4; THỦ CÔNG, CHƯA CHẠY trong gói này). Dùng: bash tools/coverage.sh [crate...]
# Cần: `cargo install cargo-llvm-cov --locked` và `rustup toolchain install nightly --component llvm-tools-preview` (đo NHÁNH cần nightly: `--branch`).
# Chạy trên Linux CI/Ngoài máy chính (đo độ phủ làm test chậm 2–5 lần). Kết quả: target/coverage/<crate>.json + dòng COVERAGE_ROW.
# Sau đó đọc các nhánh KHÔNG được chạy ở các hàm quyết định (docs/DECISION_COVERAGE.md): mỗi nhánh một thí nghiệm (chương trình hoặc test dựng) HOẶC ghi lý do không đạt tới.
# KHÔNG sửa mã để tăng phần trăm. Nhánh phòng thủ ICE bất khả đạt tới thì ghi đúng như vậy.
set -u
cd "$(dirname "$0")/.." || exit 2
bash tools/require_preflight.sh || exit 2
CRATES="${*:-gt-oblig gt-borrowck gt-types gt-term gt-effects gt-guard}"
command -v cargo-llvm-cov >/dev/null 2>&1 || { echo "COVERAGE KHÔNG CHẠY ĐƯỢC: thiếu cargo-llvm-cov"; exit 3; }
mkdir -p target/coverage
for c in $CRATES; do
  CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}" cargo +${COV_TOOLCHAIN:-nightly} llvm-cov --branch -p "$c" --json --output-path "target/coverage/$c.json" -- --test-threads=2 > "target/coverage/$c.log" 2>&1
  echo "COVERAGE_ROW $c exit=$? json=target/coverage/$c.json"
done
