#!/bin/bash
# Cổng đầy đủ: fmt, clippy -D warnings, audit, test --workspace, check_expectations (+ unittest của script)
cd "$1" || exit 1
export TEMP='D:\Grit\target\tmp' TMP='D:\Grit\target\tmp' TMPDIR=/d/Grit/target/tmp
export GRIT_LLVM_DIR="${GRIT_LLVM_DIR:-D:/Tools/llvm-20.1.8}"; export PATH="$(cygpath -u "$GRIT_LLVM_DIR")/bin:$PATH"
export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_BUILD_JOBS=2
[ -n "$2" ] && export CARGO_TARGET_DIR="$2"
echo "== gt-rt (workspace riêng, M10)"; cargo fmt --manifest-path runtime/gt-rt/Cargo.toml --check && cargo clippy --manifest-path runtime/gt-rt/Cargo.toml --all-targets -- -D warnings 2>&1 | tail -2; cargo build --release --manifest-path runtime/gt-rt/Cargo.toml 2>&1 | tail -1; cargo test --manifest-path runtime/gt-rt/Cargo.toml 2>&1 | grep "test result" | head -1
echo "== fmt"; cargo fmt --all --check && echo FMT_OK
echo "== clippy"; cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -5; echo "CLIPPY_EXIT=${PIPESTATUS[0]}"
echo "== audit"; cargo audit 2>&1 | tail -3; echo "AUDIT_EXIT=${PIPESTATUS[0]}"
echo "== test"; cargo test --workspace -- --test-threads=2 2>&1 | tee /tmp/gate_test.txt | grep -E "test result|FAILED|panicked" | awk '/test result/ {p+=$4; f+=$6; n++} /FAILED|panicked/ {print} END {print "TEST_PASSED="p" FAILED="f" SUITES="n; if (n==0) print "TEST_STAGE_INVALID: không có dòng `test result` (lỗi biên dịch/liên kết?) — KHÔNG được coi là xanh"}'
echo "== check_expectations"; PYTHONIOENCODING=utf-8 python tools/check_expectations.py | tail -3
[ -f tools/test_check_expectations.py ] && PYTHONIOENCODING=utf-8 python -m unittest tools/test_check_expectations.py 2>&1 | tail -3
