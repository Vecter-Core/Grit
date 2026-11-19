#!/bin/bash
# make_export: bản sao NGOÀI kho (GÓI 10, S0). Không đổi cây, không xoá gì.
#   bash tools/make_export.sh [thư_mục_đích=D:/Grit_export] [thư_mục_dự_án=.] [thẻ=seed-1.0-rc6]
# Tạo: grit-<hash>.bundle (--all, đã verify), grit-<hash>-tree.tar (git archive của thẻ, KHÔNG lịch sử),
# SHA256SUMS, RESTORE.md, REFS.txt. Đích phải NẰM NGOÀI kho. Mỗi lần chạy dùng tên theo <hash> HEAD ⇒ không ghi đè bản cũ.
out="${1:-D:/Grit_export}"
dir="${2:-.}"
tag="${3:-seed-1.0-rc6}"
cd "$dir" || exit 2
root="$(git rev-parse --show-toplevel)" || exit 2
mkdir -p "$out" || exit 2
absout="$(cd "$out" && pwd)"
case "$absout/" in "$root"/*) echo "EXPORT_FAIL: đích $absout nằm TRONG kho"; exit 2 ;; esac
h="$(git rev-parse --short HEAD)" || exit 2
git rev-parse -q --verify "refs/tags/$tag" >/dev/null || { echo "EXPORT_FAIL: không có thẻ $tag"; exit 2; }
b="$absout/grit-$h.bundle"
t="$absout/grit-$h-tree.tar"
git bundle create "$b" --all >/dev/null 2>&1 || { echo "EXPORT_FAIL: không tạo được bundle"; exit 1; }
git bundle verify "$b" >/dev/null 2>&1 || { echo "EXPORT_FAIL: bundle verify lỗi"; exit 1; }
git archive --format=tar --prefix="Grit-$tag/" -o "$t" "$tag" || { echo "EXPORT_FAIL: git archive lỗi"; exit 1; }
{
  echo "# git for-each-ref (tạo từ HEAD $h)"
  git for-each-ref --format='%(refname) %(objectname)'
  echo
  echo "# git log -1 của từng nhánh/thẻ"
  git for-each-ref --format='%(refname)' refs/heads refs/tags refs/remotes | while IFS= read -r r; do
    echo "== $r"; git log -1 --format='%H %an %ad %s' --date=short "$r" 2>/dev/null
  done
} > "$absout/REFS.txt"
cat > "$absout/RESTORE.md" <<EOF
# Khôi phục Grit từ bản sao ngoài máy (tạo từ HEAD $h, thẻ $tag)

Tệp: grit-$h.bundle (toàn bộ lịch sử), grit-$h-tree.tar (chỉ cây mã của $tag, dự phòng nếu bundle hỏng), SHA256SUMS, REFS.txt.

1. Kiểm toàn vẹn: sha256sum -c SHA256SUMS (hoặc Get-FileHash trên Windows).
2. git bundle verify grit-$h.bundle
3. git clone grit-$h.bundle Grit
4. cd Grit; git checkout $tag
5. Nếu bundle hỏng: tar -xf grit-$h-tree.tar (cây mã, không có lịch sử).

Dựng: cần Rust (rustup, bản trong rust-toolchain nếu có) và LLVM 20.1.8 (đặt biến GRIT_LLVM_DIR, vd D:/Tools/llvm-20.1.8 trên Windows).
Dựng runtime trước: cargo build --release --manifest-path runtime/gt-rt/Cargo.toml
Gate: bash tools/preflight.sh <thư mục>; bash tools/gate.sh <thư mục> (chi tiết: docs/ENVIRONMENT.md, docs/HANDOFF.md mục 8).
EOF
if command -v sha256sum >/dev/null 2>&1; then
  ( cd "$absout" && sha256sum "grit-$h.bundle" "grit-$h-tree.tar" REFS.txt RESTORE.md > SHA256SUMS )
else
  echo "EXPORT_FAIL: thiếu sha256sum"; exit 1
fi
echo "EXPORT_OK $absout ($(du -k "$b" | cut -f1) KB bundle; $(du -k "$t" | cut -f1) KB tar) HEAD=$h tag=$tag"
