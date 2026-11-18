#!/bin/bash
# GÓI 6 S0.a: kiểm quy tắc nhánh `main`. Cách dùng:  bash tools/main_check.sh <commit CI xanh gần nhất> [<đầu main hoặc ứng viên>=HEAD]
# Quy tắc: main = commit có CI xanh, hoặc commit CHỈ-TÀI-LIỆU ngay sau đó; chỉ fast-forward. In `git diff --name-only <xanh>..<đầu>`; OK nếu mọi tệp đổi thuộc loại KHÔNG đổi hành vi
# sản phẩm: docs/, *.md ở gốc, tests/, crates/*/tests/, tools/, .github/, apps/. Tệp khác (crates/*/src, runtime/, Cargo.*, lib/, ...) ⇒ NOT_OK (cần CI xanh trên chính commit đó).
# Thoát 0 = OK, 1 = NOT_OK, 2 = sai cách dùng. Phân biệt: "tài liệu" (docs/, *.md) với "công cụ/test/CI" (vẫn OK nhưng được liệt kê riêng để người duyệt thấy).
set -u
G="${1:-}"; H="${2:-HEAD}"
[ -n "$G" ] || { echo "dùng: bash tools/main_check.sh <commit CI xanh> [<đầu>=HEAD]"; exit 2; }
git rev-parse --verify -q "$G^{commit}" >/dev/null || { echo "commit không tồn tại: $G"; exit 2; }
git rev-parse --verify -q "$H^{commit}" >/dev/null || { echo "commit không tồn tại: $H"; exit 2; }
git merge-base --is-ancestor "$G" "$H" || { echo "MAIN_CHECK NOT_OK: $G không phải tổ tiên của $H"; exit 1; }
docs=0; aux=0; other=0
while IFS= read -r f; do
  [ -z "$f" ] && continue
  case "$f" in
    docs/*|*.md) echo "  doc    $f"; docs=$((docs+1)) ;;
    tests/*|crates/*/tests/*|tools/*|.github/*|apps/*) echo "  công cụ/test/CI  $f"; aux=$((aux+1)) ;;
    *) echo "  ĐỔI-HÀNH-VI?  $f"; other=$((other+1)) ;;
  esac
done < <(git diff --name-only "$G..$H")
echo "MAIN_CHECK files: tài liệu=$docs công_cụ/test/CI=$aux khác=$other (từ $G tới $H)"
if [ "$other" = 0 ]; then echo "MAIN_CHECK OK"; exit 0; fi
echo "MAIN_CHECK NOT_OK: có tệp ngoài docs/công cụ/test/CI; cần CI xanh trên chính commit đích"; exit 1
