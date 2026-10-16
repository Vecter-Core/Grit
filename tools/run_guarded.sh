#!/bin/bash
# Chạy một lệnh dưới lưới tệp-đã-theo-dõi: bash tools/run_guarded.sh <lệnh...>   (dùng cho probe_diff, benchmark, fuzz, đột biến tạm)
# Chụp kho trước, chạy, kiểm sau; kho đổi ⇒ in TRACK_CHANGED lớn tiếng và thoát 99 (kể cả khi lệnh thành công).
root="$(cd "$(dirname "$0")/.." && pwd)"
snap="$(mktemp)"
bash "$root/tools/track_guard.sh" snapshot "$snap" "$root"
"$@"; rc=$?
if ! bash "$root/tools/track_guard.sh" verify "$snap" "$root"; then rm -f "$snap"; echo "!!!!! run_guarded: lệnh '$1' đã làm ĐỔI tệp đã theo dõi" >&2; exit 99; fi
rm -f "$snap"; exit $rc
