#!/bin/bash
# Giai đoạn `deletions` của cổng (lệnh Giang 2026-10-16 mục 0b): mọi tệp từng bị xoá trong lịch sử git phải có dòng `| đường dẫn | commit | lý do |` ở
# docs/DELETIONS.md; xoá dưới tests/, docs/errors/, lib/, crates/*/src/ mà lý do RỖNG ⇒ lỗi. Thoát 0 = đủ; 1 = thiếu/rỗng (in tên tệp và commit).
#   bash tools/deletions_check.sh [thư_mục_dự_án]
cd "${1:-.}" || exit 2
list=docs/DELETIONS.md
bad=0; n=0
while IFS= read -r line; do
  case "$line" in
    "@@"*) commit="${line#@@}"; continue ;;
    "") continue ;;
  esac
  path="$line"; n=$((n + 1))
  row="$(grep -F -m1 "| $path |" "$list" 2>/dev/null)"
  if [ -z "$row" ]; then
    echo "DELETIONS_MISSING: $path (xoá ở commit $commit) không có dòng trong $list"; bad=1; continue
  fi
  reason="$(echo "$row" | awk -F'|' '{print $4}' | tr -d '[:space:]')"
  if [ -z "$reason" ] && echo "$path" | grep -Eq '^(tests/|docs/errors/|lib/|crates/[^/]+/src/)'; then
    echo "DELETIONS_NO_REASON: $path (commit $commit) có dòng nhưng LÝ DO RỖNG"; bad=1
  fi
done < <(git log --diff-filter=D --name-only --pretty=format:'@@%h' 2>/dev/null)
if [ "$bad" = 0 ]; then echo "DELETIONS_OK ($n tệp từng bị xoá, đều có dòng)"; fi
exit $bad
