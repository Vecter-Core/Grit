#!/bin/bash
# Cổng đầy đủ: fmt, clippy -D warnings, audit, test --workspace, check_expectations (+ unittest của script).
# Mã thoát: 0 = XANH (mọi giai đoạn hợp lệ và đạt); 1 = ĐỎ (một giai đoạn đỏ HOẶC không hợp lệ: vd giai đoạn test không biên dịch được ⇒
# không có dòng `test result` ⇒ TEST_STAGE_INVALID, KHÔNG được coi là xanh). Dòng cuối luôn là `GATE_RESULT=GREEN|RED`.
# Biến để KIỂM THỬ cổng (tools/test_gate.py): GATE_STAGES="test ..." chỉ chạy các giai đoạn đó (mặc định: gtrt fmt clippy audit test expect);
# GATE_TEST_CMD thay lệnh của giai đoạn test.
cd "$1" || exit 1
export TEMP='D:\Grit\target\tmp' TMP='D:\Grit\target\tmp' TMPDIR=/d/Grit/target/tmp
export GRIT_LLVM_DIR="${GRIT_LLVM_DIR:-D:/Tools/llvm-20.1.8}"; export PATH="$(cygpath -u "$GRIT_LLVM_DIR")/bin:$PATH"
export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_BUILD_JOBS=2
[ -n "$2" ] && export CARGO_TARGET_DIR="$2"
bash "$(dirname "$0")/handoff_check.sh" "$1" || true   # chỉ cảnh báo, KHÔNG làm cổng đỏ
STAGES="${GATE_STAGES:-gtrt fmt clippy audit test expect track}"
rc=0
red() { echo "GATE_RED: $1"; rc=1; }
has() { case " $STAGES " in *" $1 "*) return 0 ;; esac; return 1; }

# Giai đoạn `track` (lệnh Giang 2026-10-16): TỆP ĐÃ THEO DÕI KHÔNG ĐƯỢC ĐỔI trong lúc cổng chạy. Ảnh chụp TRƯỚC mọi giai đoạn và SAU tất cả; so sánh
# từng đường dẫn (tệp theo dõi bị sửa/xoá — băm `git diff HEAD` của riêng tệp — hoặc tệp mới KHÔNG bị ignore xuất hiện) ⇒ ĐỎ nêu tên tệp.
# Cây đang làm dở lúc bắt đầu vẫn dùng được: so ảnh chụp trước với sau, không đòi cây sạch. Ghi vào `target/` (bị ignore) không tính.
tracked_snapshot() {
  { git diff HEAD --name-only 2>/dev/null | while IFS= read -r f; do
      echo "$f $(git diff HEAD -- "$f" 2>/dev/null | sha256sum | cut -c1-16)"
    done
    git ls-files --others --exclude-standard 2>/dev/null | while IFS= read -r f; do echo "?? $f"; done
  } | LC_ALL=C sort
}
if has track; then TRACK_BEFORE="$(tracked_snapshot)"; fi

if has gtrt; then
  echo "== gt-rt (workspace riêng, M10)"
  cargo fmt --manifest-path runtime/gt-rt/Cargo.toml --check || red "gt-rt fmt"
  cargo clippy --manifest-path runtime/gt-rt/Cargo.toml --all-targets -- -D warnings 2>&1 | tail -2; [ "${PIPESTATUS[0]}" = 0 ] || red "gt-rt clippy"
  cargo build --release --manifest-path runtime/gt-rt/Cargo.toml 2>&1 | tail -1; [ "${PIPESTATUS[0]}" = 0 ] || red "gt-rt build release"
  cargo test --manifest-path runtime/gt-rt/Cargo.toml 2>&1 | grep "test result" | head -1; [ "${PIPESTATUS[0]}" = 0 ] || red "gt-rt test"
fi
if has fmt; then
  echo "== fmt"; cargo fmt --all --check && echo FMT_OK || red "fmt"
fi
if has clippy; then
  echo "== clippy"; cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -5; c=${PIPESTATUS[0]}; echo "CLIPPY_EXIT=$c"
  [ "$c" = 0 ] || red "clippy"
fi
if has audit; then
  echo "== audit"; cargo audit 2>&1 | tail -3; c=${PIPESTATUS[0]}; echo "AUDIT_EXIT=$c"
  [ "$c" = 0 ] || red "audit"
fi
if has test; then
  echo "== test"
  CMD="${GATE_TEST_CMD:-cargo test --workspace --no-fail-fast -- --test-threads=2}"
  mkdir -p /d/Grit/target/tmp
  OUT=/d/Grit/target/tmp/gate_test.txt
  bash -c "$CMD" > "$OUT" 2>&1; c=$?
  cp "$OUT" /d/Grit/target/tmp/gate_test_last.txt
  grep -E "panicked" -A4 "$OUT" | head -40
  grep -E "test result|FAILED|panicked" "$OUT" | awk '/test result/ {p+=$4; f+=$6; n++} /FAILED|panicked/ {print} END {print "TEST_PASSED="p" FAILED="f" SUITES="n}'
  n=$(grep -c "test result" "$OUT"); f=$(grep -E "test result" "$OUT" | awk '{f+=$6} END {print f+0}')
  if [ "$c" != 0 ]; then echo "TEST_CMD_EXIT=$c"; red "lệnh test thoát mã $c"; fi
  if [ "$n" = 0 ]; then echo "TEST_STAGE_INVALID: không có dòng \`test result\` (lỗi biên dịch/liên kết?) — KHÔNG được coi là xanh"; red "giai đoạn test không hợp lệ"; fi
  if [ "$f" != 0 ]; then red "có test đỏ ($f)"; fi
fi
if has expect; then
  echo "== check_expectations"
  PYTHONIOENCODING=utf-8 python tools/check_expectations.py | tail -3; [ "${PIPESTATUS[0]}" = 0 ] || red "check_expectations"
  if [ -f tools/test_check_expectations.py ]; then
    PYTHONIOENCODING=utf-8 python -m unittest tools/test_check_expectations.py 2>&1 | tail -3; [ "${PIPESTATUS[0]}" = 0 ] || red "unittest check_expectations"
  fi
  if [ -f tools/test_gate.py ]; then
    PYTHONIOENCODING=utf-8 python -m unittest tools/test_gate.py 2>&1 | tail -3; [ "${PIPESTATUS[0]}" = 0 ] || red "unittest gate"
  fi
fi
if has track; then
  echo "== track (tệp đã theo dõi không bị đổi)"
  TRACK_AFTER="$(tracked_snapshot)"
  if [ "$TRACK_BEFORE" != "$TRACK_AFTER" ]; then
    echo "TRACK_CHANGED: cổng làm ĐỔI cây làm việc (tệp theo dõi bị sửa/xoá hoặc tệp mới không ignore):"
    diff <(echo "$TRACK_BEFORE") <(echo "$TRACK_AFTER") | grep -E '^[<>]' | sed 's/^/  /' | head -40
    red "tệp đã theo dõi bị đổi trong lúc cổng chạy"
  else
    echo "TRACK_OK"
  fi
fi
if [ "$rc" = 0 ]; then echo "GATE_RESULT=GREEN"; else echo "GATE_RESULT=RED"; fi
exit $rc
