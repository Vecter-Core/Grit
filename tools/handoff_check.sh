#!/bin/bash
# Cảnh báo HANDOFF lỗi thời (KHÔNG BAO GIỜ làm cổng đỏ): đọc hash ở dòng "Cập nhật tại commit <hash>" của docs/HANDOFF.md,
# in số commit đã qua kể từ đó; CẢNH BÁO nếu > 5. Dùng: bash tools/handoff_check.sh [thư_mục_dự_án]
cd "${1:-.}" || exit 0
h=$(grep -m1 -oE 'Cập nhật tại commit [0-9a-f]{7,40}' docs/HANDOFF.md | grep -oE '[0-9a-f]{7,40}$')
if [ -z "$h" ]; then echo "HANDOFF_WARN: docs/HANDOFF.md không có dòng 'Cập nhật tại commit <hash>'"; exit 0; fi
n=$(git rev-list --count "$h..HEAD" 2>/dev/null)
if [ -z "$n" ]; then echo "HANDOFF_WARN: hash $h trong HANDOFF không có trong git"; exit 0; fi
echo "HANDOFF: cập nhật tại commit $h, đã qua $n commit"
[ "$n" -gt 5 ] && echo "HANDOFF_WARN: HANDOFF lỗi thời ($n > 5 commit kể từ $h) — CẬP NHẬT docs/HANDOFF.md"
exit 0
