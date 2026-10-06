#!/bin/bash
# Cổng đầy đủ: fmt, clippy -D warnings, audit, test --workspace, check_expectations (+ unittest của script)
cd "$1" || exit 1
export TEMP='D:\Grit\target\tmp' TMP='D:\Grit\target\tmp' TMPDIR=/d/Grit/target/tmp
export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_BUILD_JOBS=1
[ -n "$2" ] && export CARGO_TARGET_DIR="$2"
echo "== fmt"; cargo fmt --all --check && echo FMT_OK
echo "== clippy"; cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -5; echo "CLIPPY_EXIT=${PIPESTATUS[0]}"
echo "== audit"; cargo audit 2>&1 | tail -3; echo "AUDIT_EXIT=${PIPESTATUS[0]}"
echo "== test"; cargo test --workspace 2>&1 | tee /tmp/gate_test.txt | grep -E "test result|FAILED|panicked" | awk '/test result/ {p+=$4; f+=$6} /FAILED|panicked/ {print} END {print "TEST_PASSED="p" FAILED="f}'
echo "== check_expectations"; PYTHONIOENCODING=utf-8 python tools/check_expectations.py | tail -3
[ -f tools/test_check_expectations.py ] && PYTHONIOENCODING=utf-8 python -m unittest tools/test_check_expectations.py 2>&1 | tail -3
