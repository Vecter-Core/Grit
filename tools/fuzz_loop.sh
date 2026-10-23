#!/usr/bin/env bash
# Vòng fuzz TÍCH LUỸ có TRẦN (Seed 1.0 S7.d; ADR-037): chạy luân phiên các phiên `lex` / `parse` / `check` bằng tools/fuzz_session.sh cho tới khi
# đủ ngân sách giây TỔNG, rồi in tổng tích luỹ (tools/fuzz_total.py). Mặc định tổng 7200 giây (2 giờ), mỗi phiên 600 giây.
# Điều kiện #2 (GRIT_PLAN §7) chỉ có thể ghi "ĐANG TÍCH LUỸ": 24 giờ mỗi loại KHÔNG đạt bằng vòng này (nói thẳng).
# Dùng:  tools/fuzz_loop.sh [tổng_giây=7200] [giây_mỗi_phiên=600] [lex,parse,check]
# Máy phải RẢNH (không hai cargo cùng lúc): fuzz_session.sh tự bỏ qua + ghi `skipped_busy` nếu thấy cargo/rustc/gritc đang chạy; vòng này DỪNG
# nếu gặp phiên `skipped_busy` hai lần liên tiếp (không đốt thời gian vô ích), và DỪNG ngay nếu một phiên báo `fail` (phải điều tra, không chạy tiếp).
# Bọc theo lưới tệp-đã-theo-dõi: vòng không được đổi gì ngoài docs/FUZZ_LOG.jsonl.
set -u
TOTAL="${1:-7200}"; SEC="${2:-600}"; KINDS="${3:-lex,parse,check}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SNAP="$(mktemp)"
TRACK_IGNORE='docs/FUZZ_LOG.jsonl' bash "$ROOT/tools/track_guard.sh" snapshot "$SNAP" "$ROOT"
used=0; busy=0; i=0
IFS=',' read -r -a ks <<< "$KINDS"
while [ "$used" -lt "$TOTAL" ]; do
  k="${ks[$((i % ${#ks[@]}))]}"; i=$((i+1))
  left=$((TOTAL - used)); s=$SEC; [ "$left" -lt "$s" ] && s=$left
  out="$(bash "$ROOT/tools/fuzz_session.sh" "$k" "$s" 2>&1)"; code=$?
  echo "$out" | tail -2
  if echo "$out" | grep -q "máy bận"; then busy=$((busy+1)); [ "$busy" -ge 2 ] && { echo "fuzz_loop: máy bận hai lần liên tiếp — dừng"; break; }; continue; fi
  busy=0
  if [ "$code" -ne 0 ]; then echo "fuzz_loop: phiên $k LỖI (exit $code) — dừng; điều tra trước khi chạy tiếp"; TRACK_IGNORE='docs/FUZZ_LOG.jsonl' bash "$ROOT/tools/track_guard.sh" verify "$SNAP" "$ROOT"; python3 "$ROOT/tools/fuzz_total.py"; exit "$code"; fi
  used=$((used + s))
done
TRACK_IGNORE='docs/FUZZ_LOG.jsonl' bash "$ROOT/tools/track_guard.sh" verify "$SNAP" "$ROOT" || { echo "fuzz_loop: tệp đã theo dõi bị đổi"; exit 1; }
python3 "$ROOT/tools/fuzz_total.py"
echo "fuzz_loop: đã chạy ~${used}s trong trần ${TOTAL}s"
