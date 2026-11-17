#!/bin/bash
# Cổng đầy đủ: gtrt, fmt, clippy -D warnings, audit, test --workspace, check_expectations (+ unittest công cụ), nul, doc, deletions, failopen, track.
# Dùng: bash tools/gate.sh <thư-mục-dự-án> [thư-mục-target]
# Mã thoát: 0 = XANH (mọi giai đoạn hợp lệ và đạt TRÊN CÙNG MỘT MÃ); 1 = ĐỎ (một giai đoạn đỏ HOẶC không hợp lệ: vd giai đoạn test không biên dịch được ⇒
# không có dòng `test result` ⇒ TEST_STAGE_INVALID, KHÔNG được coi là xanh); 3 = CHƯA ĐỦ (hết ngân sách thời gian GATE_BUDGET_SEC; chạy lại cùng lệnh để tiếp).
# Dòng cuối luôn là `GATE_RESULT=GREEN|RED|INCOMPLETE` và `GATE_PLATFORM=linux|windows`.
#
# Chạy từng đơn vị CÓ ĐIỂM LƯU (lô nặng <= 10 phút/lệnh trên môi trường đám mây): `GATE_BUDGET_SEC=540 bash tools/gate.sh .` dừng sau đơn vị cuối còn trong
# ngân sách; chạy lại cùng lệnh thì bỏ qua đơn vị đã đạt. Điểm lưu gắn với KHOÁ = HEAD + băm `git diff HEAD` + tệp mới không ignore: đổi mã ⇒ khoá khác ⇒ làm lại
# từ đầu (cổng "liền mạch" nghĩa là mọi đơn vị đạt trên CÙNG một khoá). `GATE_RESET=1` xoá điểm lưu.
# Biến để KIỂM THỬ cổng (tools/test_gate.py): GATE_STAGES="test ..." chỉ chạy các giai đoạn đó; GATE_TEST_CMD thay lệnh của giai đoạn test (chạy một cục, không điểm lưu).
cd "$1" || exit 1
ROOT="$(pwd)"
TOOL_DIR="$(cd "$(dirname "$0")" && pwd)"
case "$(uname -s)" in MINGW*|MSYS*|CYGWIN*) PLAT=windows ;; *) PLAT=linux ;; esac
if command -v python3 >/dev/null 2>&1; then PY=python3; else PY=python; fi
export PYTHONIOENCODING=utf-8
if [ "$PLAT" = windows ]; then
  # nền chính của dự án (docs/ENVIRONMENT.md): TEMP ở ổ D:, LLVM 20.1.8 bản dựng sẵn
  export TEMP='D:\Grit\target\tmp' TMP='D:\Grit\target\tmp' TMPDIR=/d/Grit/target/tmp
  export GRIT_LLVM_DIR="${GRIT_LLVM_DIR:-D:/Tools/llvm-20.1.8}"; export PATH="$(cygpath -u "$GRIT_LLVM_DIR")/bin:$PATH"
  TMPD=/d/Grit/target/tmp
  JOBS=2
else
  TMPD="$ROOT/target/tmp"
  JOBS=$(nproc 2>/dev/null || echo 2); [ "$JOBS" -gt 4 ] && JOBS=4
  export TMPDIR="$TMPD"
fi
mkdir -p "$TMPD"
export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-$JOBS}"
[ -n "$2" ] && export CARGO_TARGET_DIR="$2"
# runtime gt-rt có workspace riêng ⇒ KHÔNG dùng chung CARGO_TARGET_DIR (gritc tìm bản dựng cạnh mã nguồn)
# S2.c (GÓI 3): cổng ĐẦY ĐỦ (không đặt GATE_STAGES) tự từ chối khi preflight FAIL; GATE_STAGES chỉ dành cho kiểm thử cổng/chạy một phần
if [ -z "${GATE_STAGES:-}" ] || [ -n "${GATE_FORCE_PREFLIGHT:-}" ]; then bash "$TOOL_DIR/require_preflight.sh" || { echo "GATE_RESULT=RED"; exit 2; }; fi
bash "$(dirname "$0")/handoff_check.sh" "$1" || true   # chỉ cảnh báo, KHÔNG làm cổng đỏ
if [ -f tools/pre-commit.hook ] && ! cmp -s tools/pre-commit.hook .git/hooks/pre-commit 2>/dev/null; then
  echo "HOOK_WARN: .git/hooks/pre-commit chưa cài hoặc khác tools/pre-commit.hook — chạy: bash tools/install_hooks.sh"
fi
STAGES="${GATE_STAGES:-nul gtrt fmt clippy audit test expect doc deletions failopen track}"
rc=0
START=$(date +%s)
BUDGET="${GATE_BUDGET_SEC:-0}"
INCOMPLETE=0
red() { echo "GATE_RED: $1"; rc=1; }
has() { case " $STAGES " in *" $1 "*) return 0 ;; esac; return 1; }
over_budget() { [ "$BUDGET" -gt 0 ] && [ $(( $(date +%s) - START )) -ge "$BUDGET" ]; }

# ---- điểm lưu -------------------------------------------------------------------------------------------------------------------------------
STATE_DIR="$ROOT/target/gate_state"
# BĂM CÂY (GÓI 3 S2.d): HEAD + diff tệp theo dõi (kể cả nhị phân) + NỘI DUNG từng tệp mới không bị ignore (không chỉ tên). Điểm lưu chỉ hợp lệ khi băm này khớp.
tree_hash() { { git rev-parse HEAD 2>/dev/null; git diff HEAD --binary 2>/dev/null | sha256sum | cut -d' ' -f1; git ls-files --others --exclude-standard -z 2>/dev/null | sort -z | xargs -0 -r sha256sum 2>/dev/null | sha256sum | cut -d' ' -f1; } | tr '\n' ' ' | sha256sum | cut -d' ' -f1; }
KEY="$(tree_hash)"
echo "GATE_TREE=$KEY"
mkdir -p "$STATE_DIR"
[ -n "$GATE_RESET" ] && rm -f "$STATE_DIR"/*.tsv
STATE="$STATE_DIR/state.tsv"
if [ -f "$STATE" ] && [ "$(head -1 "$STATE")" != "KEY	$KEY" ]; then echo "GATE_STATE: mã đã đổi so với điểm lưu ⇒ làm lại từ đầu"; rm -f "$STATE"; fi
[ -f "$STATE" ] || printf 'KEY\t%s\n' "$KEY" > "$STATE"
done_unit() { awk -F'\t' -v u="$1" '$1==u && $2=="PASS" {f=1} END {exit !f}' "$STATE"; }
mark_unit() { printf '%s\t%s\t%s\t%s\t%s\n' "$1" "$2" "${3:-0}" "${4:-0}" "$(tree_hash)" >> "$STATE"; }   # đơn-vị, PASS|FAIL, đạt, bộ

# Giai đoạn `track` (lệnh Giang 2026-10-16): TỆP ĐÃ THEO DÕI KHÔNG ĐƯỢC ĐỔI trong lúc cổng chạy. Ảnh chụp TRƯỚC mọi giai đoạn và SAU tất cả; so sánh
# từng đường dẫn (tệp theo dõi bị sửa/xoá — băm `git diff HEAD` của riêng tệp — hoặc tệp mới KHÔNG bị ignore xuất hiện) ⇒ ĐỎ nêu tên tệp.
GUARD="$(dirname "$0")/track_guard.sh"
TRACK_SNAP="$(mktemp)"
if has track; then bash "$GUARD" snapshot "$TRACK_SNAP" .; fi

# chạy một đơn vị có điểm lưu: unit "tên" lệnh... ; trả 0 nếu đạt (kể cả đã đạt từ lần trước)
unit() {
  local name="$1"; shift
  if done_unit "$name"; then echo "== $name: ĐÃ ĐẠT (điểm lưu)"; return 0; fi
  if over_budget; then INCOMPLETE=1; echo "== $name: hoãn (hết ngân sách ${BUDGET}s)"; return 2; fi
  echo "== $name"
  "$@"; local c=$?
  if [ $c = 0 ] && [ "$(tree_hash)" != "$KEY" ]; then echo "GATE_TREE_CHANGED: cây đổi trong giai đoạn $name (băm lúc bắt đầu $KEY) ⇒ KHÔNG hợp lệ"; c=1; fi
  if [ $c = 0 ]; then mark_unit "$name" PASS; else mark_unit "$name" FAIL; red "$name"; fi
  return $c
}

stage_gtrt() {
  cargo fmt --manifest-path runtime/gt-rt/Cargo.toml --check || return 1
  cargo clippy --manifest-path runtime/gt-rt/Cargo.toml --all-targets -- -D warnings 2>&1 | tail -2; [ "${PIPESTATUS[0]}" = 0 ] || return 1
  # dựng RIÊNG target (không thừa kế CARGO_TARGET_DIR): gritc tìm runtime/gt-rt/target/release/{gt_rt.lib|libgt_rt.a}
  env -u CARGO_TARGET_DIR cargo build --release --manifest-path runtime/gt-rt/Cargo.toml 2>&1 | tail -1; [ "${PIPESTATUS[0]}" = 0 ] || return 1
  env -u CARGO_TARGET_DIR cargo test --manifest-path runtime/gt-rt/Cargo.toml 2>&1 | grep "test result" | head -1; [ "${PIPESTATUS[0]}" = 0 ] || return 1
}
stage_fmt() { cargo fmt --all --check && echo FMT_OK; }
stage_clippy() { cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -5; local c=${PIPESTATUS[0]}; echo "CLIPPY_EXIT=$c"; return "$c"; }
stage_audit() { cargo audit 2>&1 | tail -3; local c=${PIPESTATUS[0]}; echo "AUDIT_EXIT=$c"; return "$c"; }
stage_expect() {
  local r=0
  $PY tools/check_expectations.py | tail -3; [ "${PIPESTATUS[0]}" = 0 ] || { echo "GATE_RED: check_expectations"; r=1; }
  for t in test_check_expectations test_mut_guard test_mut_resume test_preflight test_nul_check test_gate test_require_preflight test_unsafe_inventory test_nul_scan test_doc_test test_ai_guide test_gen_docs; do
    if [ -f "tools/$t.py" ]; then
      $PY -m unittest "tools/$t.py" 2>&1 | tail -3; [ "${PIPESTATUS[0]}" = 0 ] || { echo "GATE_RED: unittest $t"; r=1; }
    fi
  done
  return $r
}
# Giai đoạn `nul` (R1): tệp văn bản ĐÃ THEO DÕI không được chứa byte NUL (sự cố 2026-10-16: tệp bị ghi đè bằng NUL); .git cũng không (preflight).
stage_nul() {
  local r=0
  $PY "$TOOL_DIR/nul_scan.py" "$ROOT" || r=1
  if [ -f "$TOOL_DIR/nul_check.sh" ]; then bash "$TOOL_DIR/nul_check.sh" "$ROOT" || r=1; fi
  return "$r"
}
# Giai đoạn `doc` (S3): mọi khối ```grit trong docs/ phải biên dịch/bị từ chối đúng như chú thích.
stage_doc() { if [ -f tools/doc_test.py ]; then $PY tools/doc_test.py; else echo "doc_test.py chưa có (bỏ qua)"; fi; }
stage_deletions() { bash "$(dirname "$0")/deletions_check.sh" .; }
# Giai đoạn `failopen` (Seed 1.0 S7): mọi mẫu "im lặng chấp nhận" MỚI ở 11 crate quyết định phán quyết phải được đọc + phân loại (docs/FAIL_OPEN_AUDIT.md mục 8) rồi vào baseline
stage_failopen() { $PY tools/fail_open_scan.py --baseline docs/fail_open_baseline.json && $PY tools/fail_open_sync.py; }

if has gtrt;       then unit gtrt stage_gtrt; fi
if has fmt;        then unit fmt stage_fmt; fi
if has clippy;     then unit clippy stage_clippy; fi
if has audit;      then unit audit stage_audit; fi

# ---- test: mỗi crate một đơn vị (điểm lưu); GATE_TEST_CMD (kiểm thử cổng) chạy một cục -------------------------------------------------------
test_unit() { # $1 = gói
  local pkg="$1" out="$TMPD/gate_test_$1.txt"
  cargo test -p "$pkg" --no-fail-fast -- --test-threads=2 > "$out" 2>&1; local c=$?
  if [ "$c" != 0 ]; then cp "$out" "$TMPD/gate_test_last.txt"; fi
  grep -E "panicked" -A4 "$out" | head -40
  local n f p
  n=$(grep -c "test result" "$out"); f=$(grep -E "test result" "$out" | awk '{f+=$6} END {print f+0}'); p=$(grep -E "test result" "$out" | awk '{p+=$4} END {print p+0}')
  echo "TEST_UNIT $pkg passed=$p failed=$f suites=$n exit=$c"
  if [ "$c" != 0 ] || [ "$n" = 0 ] || [ "$f" != 0 ]; then tail -100 "$out"; fi
  [ "$c" = 0 ] || return 1
  [ "$n" != 0 ] || { echo "TEST_STAGE_INVALID: $pkg không có dòng \`test result\` — KHÔNG được coi là xanh"; return 1; }
  [ "$f" = 0 ] || return 1
  printf '%s\t%s\t%s\t%s\t%s\n' "TESTSUM:$pkg" "PASS" "$p" "$n" "$KEY" >> "$STATE"
  return 0
}
if has test; then
  if [ -n "$GATE_TEST_CMD" ]; then
    echo "== test (lệnh tuỳ chọn)"
    OUT="$TMPD/gate_test.txt"
    bash -c "$GATE_TEST_CMD" > "$OUT" 2>&1; c=$?
    cp "$OUT" "$TMPD/gate_test_last.txt"
    grep -E "panicked" -A4 "$OUT" | head -40
    grep -E "test result|FAILED|panicked" "$OUT" | awk '/test result/ {p+=$4; f+=$6; n++} /FAILED|panicked/ {print} END {print "TEST_PASSED="p" FAILED="f" SUITES="n}'
    n=$(grep -c "test result" "$OUT"); f=$(grep -E "test result" "$OUT" | awk '{f+=$6} END {print f+0}')
    if [ "$c" != 0 ]; then echo "TEST_CMD_EXIT=$c"; red "lệnh test thoát mã $c"; fi
    if [ "$n" = 0 ]; then echo "TEST_STAGE_INVALID: không có dòng \`test result\` (lỗi biên dịch/liên kết?) — KHÔNG được coi là xanh"; red "giai đoạn test không hợp lệ"; fi
    if [ "$f" != 0 ]; then red "có test đỏ ($f)"; fi
  else
    # thứ tự phụ thuộc: crate nhỏ trước; mỗi gói một đơn vị. Gói thật lấy từ `cargo metadata` để KHÔNG bỏ sót crate mới.
    PKGS=$(cargo metadata --no-deps --format-version 1 2>/dev/null | $PY -c "import sys,json; print(' '.join(p['name'] for p in json.load(sys.stdin)['packages']))")
    [ -n "$PKGS" ] || { echo "TEST_STAGE_INVALID: không liệt kê được gói"; red "giai đoạn test không hợp lệ"; }
    for p in $PKGS; do unit "test:$p" test_unit "$p"; done
    if [ "$INCOMPLETE" = 0 ] && [ "$rc" = 0 ] && [ -n "$PKGS" ]; then
      awk -F'\t' '$1 ~ /^TESTSUM:/ && $2=="PASS" {p+=$3; n+=$4} END {print "TEST_PASSED="p+0" FAILED=0 SUITES="n+0}' "$STATE"
    fi
  fi
fi
if has expect;    then unit expect stage_expect; fi
if has nul;       then unit nul stage_nul; fi
if has doc;       then unit doc stage_doc; fi
if has deletions; then unit deletions stage_deletions; fi
if has failopen;  then unit failopen stage_failopen; fi
if has track; then
  echo "== track (tệp đã theo dõi không bị đổi)"
  if bash "$GUARD" verify "$TRACK_SNAP" .; then :; else red "tệp đã theo dõi bị đổi trong lúc cổng chạy"; fi
  rm -f "$TRACK_SNAP"
fi
# băm từng giai đoạn đã đạt (mỗi giai đoạn ghi băm cây LÚC XONG; xanh liền mạch ⇔ tất cả bằng GATE_TREE)
awk -F'\t' '$2=="PASS" && $1 !~ /^TESTSUM:/ {print "GATE_STAGE_TREE " $1 " " $5}' "$STATE"
if awk -F'\t' -v k="$KEY" '$2=="PASS" && $5!="" && $5!=k {bad=1} END {exit !bad}' "$STATE"; then red "băm giai đoạn khác băm cây (điểm lưu lệch mã)"; fi
echo "GATE_TREE=$KEY"
echo "GATE_PLATFORM=$PLAT"
if [ "$rc" != 0 ]; then echo "GATE_RESULT=RED"; exit 1; fi
if [ "$INCOMPLETE" != 0 ]; then echo "GATE_RESULT=INCOMPLETE"; exit 3; fi
echo "GATE_RESULT=GREEN"
exit 0
