#!/bin/bash
# track_guard: lưới "tệp đã theo dõi không được đổi" DÙNG CHUNG (gate.sh, mut.py, fuzz_*.sh, probe_diff, benchmark).
#   bash tools/track_guard.sh snapshot <tệp_ảnh> [thư_mục_dự_án]   ghi ảnh chụp cây làm việc
#   bash tools/track_guard.sh verify   <tệp_ảnh> [thư_mục_dự_án]   so với ảnh chụp: khác ⇒ in TRACK_CHANGED + tên tệp, thoát 1; giống ⇒ in TRACK_OK, thoát 0
# Ảnh chụp: mỗi tệp đã theo dõi có thay đổi so với HEAD kèm băm `git diff HEAD` riêng của tệp (xoá cũng là thay đổi), và mỗi tệp mới KHÔNG bị ignore.
# TRACK_IGNORE=<regex egrep>: bỏ qua các đường dẫn hợp lệ được GHI NỐI có chủ ý (ví dụ docs/FUZZ_LOG.jsonl).
# Cây đang làm dở lúc chụp vẫn dùng được (so trước với sau). Ghi vào `target/` (bị ignore) không tính.
mode="$1"; snap="$2"; dir="${3:-.}"
cd "$dir" || { echo "TRACK_GUARD: không vào được $dir"; exit 2; }
snapshot() {
  { git diff HEAD --name-only 2>/dev/null | while IFS= read -r f; do
      echo "$f $(git diff HEAD -- "$f" 2>/dev/null | sha256sum | cut -c1-16)"
    done
    git ls-files --others --exclude-standard 2>/dev/null | while IFS= read -r f; do echo "?? $f"; done
  } | LC_ALL=C sort | { if [ -n "$TRACK_IGNORE" ]; then grep -Ev "$TRACK_IGNORE"; else cat; fi; }
}
case "$mode" in
  snapshot) snapshot > "$snap"; exit 0 ;;
  verify)
    [ -f "$snap" ] || { echo "TRACK_GUARD: thiếu ảnh chụp $snap"; exit 2; }
    now="$(snapshot)"
    if [ "$now" != "$(cat "$snap")" ]; then
      echo "TRACK_CHANGED: tệp đã theo dõi bị sửa/xoá hoặc tệp mới không ignore xuất hiện:"
      diff "$snap" <(echo "$now") | grep -E '^[<>]' | sed 's/^/  /' | head -40
      exit 1
    fi
    echo "TRACK_OK"; exit 0 ;;
  *) echo "dùng: track_guard.sh snapshot|verify <tệp_ảnh> [thư_mục]"; exit 2 ;;
esac
