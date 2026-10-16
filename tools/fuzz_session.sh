#!/usr/bin/env bash
# Một phiên fuzz TÍCH LUỸ (ADR-037): chạy MỘT phiên `lex` hoặc `parse` (mặc định 600 giây) rồi GHI NỐI một dòng JSON vào docs/FUZZ_LOG.jsonl.
# Chỉ chạy khi MÁY RẢNH (quy tắc môi trường: không hai cargo cùng lúc; ASan Windows dễ hết commit bộ nhớ khi build song song): nếu thấy
# cargo/rustc/link/gritc/test đang chạy thì KHÔNG chạy, ghi dòng "skipped_busy" và thoát mã 0.
# Dùng:  tools/fuzz_session.sh <lex|parse> [giây=600]      (cần nightly + cargo-fuzz + ASan DLL trên PATH, xem ADR-009)
# Tổng tích luỹ: python tools/fuzz_total.py
set -u
T="${1:-}"; SEC="${2:-600}"
case "$T" in
  lex) CRATE=gt-lexer; SCRIPT=fuzz_lexer.sh ;;
  parse) CRATE=gt-parser; SCRIPT=fuzz_parser.sh ;;
  *) echo "dùng: $0 <lex|parse> [giây]"; exit 3 ;;
esac
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LOG="$ROOT/docs/FUZZ_LOG.jsonl"
TRACK_SNAP_FUZZ="$(mktemp)"; TRACK_IGNORE='docs/FUZZ_LOG.jsonl' bash "$ROOT/tools/track_guard.sh" snapshot "$TRACK_SNAP_FUZZ" "$ROOT"
ts() { date -u +%Y-%m-%dT%H:%M:%SZ; }
busy=$(tasklist 2>/dev/null | grep -i -E "^(cargo|rustc|link|gritc|cl|clang|rig)\.exe|^[a-z_0-9]+-[0-9a-f]{16}\.exe" | head -3)
if [ -n "$busy" ]; then
  echo "{\"ts\":\"$(ts)\",\"target\":\"$T\",\"status\":\"skipped_busy\",\"seconds\":0,\"runs\":0}" >> "$LOG"
  echo "máy bận, bỏ qua: $busy"
  exit 0
fi
bash "$ROOT/tools/$SCRIPT" 1 "$SEC" > /dev/null 2>&1
code=$?
SLOG="$ROOT/crates/$CRATE/fuzz/logs/session_1.log"
runs=$(grep -o 'number_of_executed_units: [0-9]*' "$SLOG" 2>/dev/null | tail -1 | grep -o '[0-9]*$')
runs="${runs:-0}"
status=ok; [ "$code" -ne 0 ] && status=fail
echo "{\"ts\":\"$(ts)\",\"target\":\"$T\",\"status\":\"$status\",\"seconds\":$SEC,\"runs\":$runs,\"exit\":$code}" >> "$LOG"
# lưới tệp-đã-theo-dõi: phiên fuzz không được đổi gì ngoài FUZZ_LOG (ghi nối có chủ ý)
if [ -n "${TRACK_SNAP_FUZZ:-}" ]; then TRACK_IGNORE='docs/FUZZ_LOG.jsonl' bash "$ROOT/tools/track_guard.sh" verify "$TRACK_SNAP_FUZZ" "$ROOT" || echo "!!!!! fuzz_session: tệp đã theo dõi bị đổi"; fi
echo "phiên $T: $status, $runs lượt, ${SEC}s"
exit "$code"
