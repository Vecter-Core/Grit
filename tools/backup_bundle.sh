#!/bin/bash
# backup_bundle: sao lưu kho bằng `git bundle` sau mỗi cổng XANH (lệnh Giang 2026-10-17, R1e).
#   bash tools/backup_bundle.sh [thư_mục_dự_án] [thư_mục_sao_lưu=D:/Grit_backup]
# Giữ 3 bản gần nhất; `git bundle verify` mỗi bản mới. LƯU Ý: sao lưu CÙNG Ổ không bảo vệ khỏi hỏng ổ; việc đẩy lên remote do Giang quyết (agent KHÔNG push).
dir="${1:-.}"
out="${2:-D:/Grit_backup}"
cd "$dir" || exit 2
mkdir -p "$out" || exit 2
h="$(git rev-parse --short HEAD)" || exit 2
f="$out/grit-$h.bundle"
git bundle create "$f" --all >/dev/null 2>&1 || { echo "BUNDLE_FAIL: không tạo được $f"; exit 1; }
git bundle verify "$f" >/dev/null 2>&1 || { echo "BUNDLE_FAIL: verify lỗi $f"; exit 1; }
# giữ 3 bản mới nhất theo thời gian sửa
ls -1t "$out"/grit-*.bundle 2>/dev/null | tail -n +4 | while IFS= read -r old; do rm -f "$old"; done
echo "BUNDLE_OK $f ($(du -k "$f" | cut -f1) KB; còn $(ls -1 "$out"/grit-*.bundle | wc -l) bản)"
