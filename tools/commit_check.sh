#!/bin/bash
# Kiểm lúc commit (lệnh Giang 2026-10-16 mục 0c): nếu phần đã stage có tệp bị XOÁ (dòng D của `git diff --cached --name-status`) thì docs/DELETIONS.md phải
# được cập nhật CÙNG commit với một dòng `| đường dẫn | ... |` cho từng tệp đó. Thiếu ⇒ in cảnh báo LỚN và thoát 1 (hook pre-commit chặn commit).
# Dùng: bash tools/commit_check.sh [thư_mục_dự_án]     Cài hook: bash tools/install_hooks.sh
cd "${1:-.}" || exit 2
bad=0
while IFS= read -r f; do
  [ -z "$f" ] && continue
  if ! git diff --cached -- docs/DELETIONS.md 2>/dev/null | grep -Fq "+| $f |"; then
    echo "!!!!! COMMIT XOÁ TỆP ĐÃ THEO DÕI: $f"
    echo "!!!!!   cần thêm dòng '| $f | <commit> | <lý do> |' vào docs/DELETIONS.md TRONG CÙNG commit"
    bad=1
  fi
done < <(git diff --cached --name-status 2>/dev/null | awk '$1 == "D" {print $2}')
exit $bad
