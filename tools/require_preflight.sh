#!/bin/bash
# Cổng tự động cho công cụ chạy NẶNG (GÓI 3 S2.c): chạy tools/preflight.sh; FAIL ⇒ in rõ và thoát 2. Dùng: bash tools/require_preflight.sh   (hoặc: . tools/require_preflight.sh khi cần)
# GRIT_PREFLIGHT_DIR (chỉ để kiểm thử, xem tools/test_require_preflight.py) trỏ preflight tới kho khác.
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
dir="${GRIT_PREFLIGHT_DIR:-$(cd "$here/.." && pwd)}"
if ! bash "$here/preflight.sh" "$dir"; then
  echo "PREFLIGHT FAIL: TỪ CHỐI chạy lô nặng (sửa nguyên nhân rồi chạy lại; không có cách bỏ qua trong công cụ này)" >&2
  exit 2
fi
