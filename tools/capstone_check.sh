#!/usr/bin/env bash
set -euo pipefail
export PYTHONIOENCODING=utf-8

if [[ $# -ne 2 ]]; then
  echo "Dùng: tools/capstone_check.sh <tệp.gt> <K1|K2|K3|K4>" >&2
  exit 3
fi

SRC=$(cd "$(dirname "$1")" && pwd)/$(basename "$1")
K=$2
case "$K" in
  K1) KIND=csvstat ;;
  K2) KIND=calc ;;
  K3) KIND=kvstore ;;
  K4) KIND=grep-lite ;;
  *) echo "K phải là K1, K2, K3 hoặc K4" >&2; exit 3 ;;
esac
[[ -f "$SRC" ]] || { echo "Không thấy tệp: $SRC" >&2; exit 3; }

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
PYTHON=${PYTHON:-python}
GRITC=${GRITC:-"$ROOT/target/debug/gritc.exe"}
export GRITC
TMP=$(mktemp -d "${TMPDIR:-/tmp}/grit-capstone.XXXXXX")
trap 'rm -rf "$TMP"' EXIT
# Windows (Git Bash): Python native cần đường dẫn dạng C:/… (không phải /tmp/…)
if command -v cygpath >/dev/null 2>&1; then TMP=$(cygpath -m "$TMP"); fi
PKG="$TMP/pkg"
mkdir -p "$PKG/src"
cp "$SRC" "$PKG/src/main.gt"

cargo build --release --manifest-path "$ROOT/capstone/reference/Cargo.toml"
"$PYTHON" "$ROOT/capstone/gen_cases.py" "$TMP/cases" --large

REF="$ROOT/capstone/reference/target/release/$KIND"
if [[ -f "$REF.exe" ]]; then REF="$REF.exe"; fi
[[ -f "$REF" ]] || { echo "Thiếu bản tham chiếu: $REF" >&2; exit 3; }

for PROFILE in dev release-o2; do
  CAND="$TMP/candidate-$PROFILE.exe"
  echo "=== $K $PROFILE ==="
  "$PYTHON" "$ROOT/capstone/build_pkg.py" "$PKG" "$PROFILE" "$CAND"
  sleep 5  # Windows Defender quét exe mới: chạy ngay có thể bị "Access is denied" (WinError 5)
  "$PYTHON" "$ROOT/capstone/check.py" --k "$KIND" --large --cand "$CAND" --ref "$REF" --cases "$TMP/cases"
done
