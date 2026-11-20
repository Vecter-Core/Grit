#!/usr/bin/env bash
# Một phiên fuzz TÍCH LUỸ (ADR-037): chạy MỘT phiên `lex` hoặc `parse` (mặc định 600 giây) rồi GHI NỐI một dòng JSON vào docs/FUZZ_LOG.jsonl.
# Chỉ chạy khi MÁY RẢNH (quy tắc môi trường: không hai cargo cùng lúc; ASan Windows dễ hết commit bộ nhớ khi build song song): nếu thấy
# cargo/rustc/link/gritc/test đang chạy thì KHÔNG chạy, ghi dòng "skipped_busy" và thoát mã 0.
# Dùng:  tools/fuzz_session.sh <lex|parse|check> [giây=600]      (cần nightly + cargo-fuzz; Windows: ASan DLL trên PATH, xem ADR-009)
# Tổng tích luỹ: python tools/fuzz_total.py
set -u
T="${1:-}"; SEC="${2:-600}"
case "$T" in
  lex) CRATE=gt-lexer; SCRIPT=fuzz_lexer.sh ;;
  parse) CRATE=gt-parser; SCRIPT=fuzz_parser.sh ;;
  check) CRATE=gt-driver; SCRIPT=fuzz_check.sh ;;
  *) echo "dùng: $0 <lex|parse|check> [giây]"; exit 3 ;;
esac
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LOG="$ROOT/docs/FUZZ_LOG.jsonl"
TRACK_SNAP_FUZZ="$(mktemp)"; TRACK_IGNORE='docs/FUZZ_LOG.jsonl' bash "$ROOT/tools/track_guard.sh" snapshot "$TRACK_SNAP_FUZZ" "$ROOT"
ts() { date -u +%Y-%m-%dT%H:%M:%SZ; }
if [ "${FUZZ_PEER:-0}" = 1 ]; then
  busy=""   # chế độ song song của fuzz_loop.sh: tiến trình cargo/fuzz của phiên bạn là CHÍNH MÌNH; fuzz_loop đã kiểm kho + RAM
elif command -v tasklist >/dev/null 2>&1; then
  busy=$(tasklist 2>/dev/null | grep -i -E "^(cargo|rustc|link|gritc|cl|clang|rig)\.exe|^[a-z_0-9]+-[0-9a-f]{16}\.exe" | head -3)
else
  # Linux: tiến trình cargo/rustc/gritc/rig hoặc tệp test (`<tên>-<16 hex>`) đang chạy (trừ chính phiên này, vốn gọi `cargo fuzz`)
  busy=$(pgrep -a -x -f '(cargo|rustc|gritc|rig)( .*)?' 2>/dev/null | grep -v -E "fuzz|fuzz_session|fuzz_loop" | head -3; pgrep -a -f 'target/debug/deps/[a-z_0-9]+-[0-9a-f]{16}' 2>/dev/null | head -3)
fi
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
# băm NGUỒN của chính crate (và phụ thuộc) để tích luỹ chỉ tính trên mã hiện tại (P3.a)
case "$T" in lex) SRCDIRS="crates/gt-lexer/src crates/gt-lexer/fuzz/fuzz_targets";; parse) SRCDIRS="crates/gt-parser/src crates/gt-parser/fuzz/fuzz_targets crates/gt-lexer/src crates/gt-ast/src crates/gt-span/src crates/gt-diag/src";; *) SRCDIRS="crates/gt-driver/src";; esac
SRCH="$(cd "$ROOT" && git ls-files -s $SRCDIRS | sha1sum | cut -c1-12)"
# GÓI 12 P2.a: trên Linux ghi thêm nguồn + nền (WSL ⇒ "wsl-linux"; Linux khác ⇒ "linux-local"), bản hệ điều hành và nhân. Windows giữ định dạng cũ (không có `source` = "windows").
EXTRA=""
case "$(uname -s)" in
  Linux)
    SRCNAME=linux-local; grep -qi microsoft /proc/version 2>/dev/null && SRCNAME=wsl-linux
    OSNAME="$(. /etc/os-release 2>/dev/null; echo "${PRETTY_NAME:-linux}" | tr -d '"\\')"; KERN="$(uname -r | tr -d '"\\')"
    EXTRA=",\"source\":\"$SRCNAME\",\"os\":\"$OSNAME\",\"kernel\":\"$KERN\"" ;;
esac
echo "{\"ts\":\"$(ts)\",\"target\":\"$T\",\"status\":\"$status\",\"seconds\":$SEC,\"runs\":$runs,\"exit\":$code,\"src\":\"$SRCH\"$EXTRA}" >> "$LOG"
# lưới tệp-đã-theo-dõi: phiên fuzz không được đổi gì ngoài FUZZ_LOG (ghi nối có chủ ý)
if [ -n "${TRACK_SNAP_FUZZ:-}" ]; then TRACK_IGNORE='docs/FUZZ_LOG.jsonl' bash "$ROOT/tools/track_guard.sh" verify "$TRACK_SNAP_FUZZ" "$ROOT" || echo "!!!!! fuzz_session: tệp đã theo dõi bị đổi"; fi
echo "phiên $T: $status, $runs lượt, ${SEC}s"
exit "$code"
