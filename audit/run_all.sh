#!/usr/bin/env bash
# audit/run_all.sh — CHẠY TOÀN BỘ dự án Grit theo từng giai đoạn, ghi kết quả vào audit/RESULTS.md. Có điểm lưu: chạy lại tự tiếp tục.
#
# NGUYÊN TẮC (lệnh của chủ dự án): KHÔNG sửa tệp nào của dự án; chỉ ghi vào audit/ (state.tsv, results/, RESULTS.md, logs/). Mọi thử nghiệm
# cần vá mã (vd dựng gritc không có LLVM) chạy trên BẢN SAO ngoài dự án ($AUDIT_WORK). Mỗi giai đoạn tự kiểm `git status` trước/sau và
# báo nếu dự án bị đổi.
#
# DÙNG:
#   bash audit/run_all.sh                 chạy mọi giai đoạn CHƯA xong (PASS/FAIL/BLOCKED/TIMEOUT/SKIP đã ghi thì bỏ qua) — dùng để "tiếp tục"
#   bash audit/run_all.sh --list          in bảng trạng thái
#   bash audit/run_all.sh --only ID,ID    chạy ép các giai đoạn đó (kể cả đã xong)
#   bash audit/run_all.sh --group C       chỉ các giai đoạn có ID bắt đầu bằng "C"
#   bash audit/run_all.sh --retry-failed  chạy lại cả giai đoạn FAIL/TIMEOUT
#   bash audit/run_all.sh --next N        dừng sau khi chạy N giai đoạn
#   bash audit/run_all.sh --report        chỉ dựng lại RESULTS.md
#   bash audit/run_all.sh --ids           in danh sách giai đoạn (dùng cho report.py)
# Mã thoát của giai đoạn: 0 PASS · 1 FAIL · 77 BLOCKED (không chạy được ở môi trường này) · 78 SKIP · 124 TIMEOUT.
set -u -o pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
AUDIT="$ROOT/audit"; LOGS="$AUDIT/logs"; RES="$AUDIT/results"; STATE="$AUDIT/state.tsv"
WORK="${AUDIT_WORK:-/tmp/grit_audit_work}"
COPY="$WORK/grit"
TEST_THREADS="${TEST_THREADS:-2}"           # quy tắc của dự án (docs/ENVIRONMENT.md §3): test với --test-threads=2
export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}" PYTHONIOENCODING=utf-8
export CARGO_TERM_COLOR=never RUST_BACKTRACE=1
# 17 crate chạy được NGUYÊN TRẠNG trên Linux (không phụ thuộc LLVM/Windows)
PURE_CRATES=(gt-span gt-lexer gt-diag gt-ast gt-parser gt-stack gt-fmt gt-resolve gt-types gt-mir gt-dataflow gt-borrowck gt-oblig gt-effects gt-conc gt-term gt-secrets)
EXCL_NATIVE=(--exclude gt-llvm --exclude gt-codegen-llvm --exclude gt-guard --exclude gt-driver --exclude rig)
mkdir -p "$LOGS" "$RES"

# ---------- danh sách giai đoạn: ID|timeout(giây)|opt(core/opt)|tên ----------
STAGES=(
  "A00_env|120|core|Môi trường: công cụ, phiên bản, tài nguyên"
  "A01_fetch|600|core|Tải phụ thuộc theo Cargo.lock (--locked)"
  "A02_inventory|300|core|Kiểm kê dự án: crate, tệp, LOC, tệp nhị phân bị theo dõi, CI/deny"
  "B01_fmt|300|core|cargo fmt --all --check"
  "B02_clippy|1800|core|clippy -D warnings (17 crate thuần)"
  "B03_audit|1200|core|cargo audit (lỗ hổng phụ thuộc)"
  "B04_ci_deny|60|core|CI (ci.yml) và deny.toml có tồn tại không"
  "B05_codes|60|core|Mọi mã chẩn đoán đã biết có tài liệu + ≥3 mẫu fail + ≥3 mẫu pass (GRIT_PLAN §0.5)"
  "C00_build|1800|core|Dựng 17 crate thuần (mọi target, kể cả test)"
)
for c in "${PURE_CRATES[@]}"; do STAGES+=("C_${c}|3600|core|cargo test -p ${c}"); done
STAGES+=(
  "D01_native_probe|900|core|Dựng nguyên trạng gt-driver/rig/gt-llvm (cần LLVM 20 + Windows)"
  "E01_gtrt|600|core|runtime/gt-rt (runtime native, chỉ Windows x86_64)"
  "F01_expect|300|core|tools/check_expectations.py (nhất quán corpus + mã lỗi)"
  "F02_tool_tests|600|core|unittest của tools/*.py"
  "G00_copy_patch|1200|core|Bản sao NGOÀI dự án + vá tối thiểu để gritc dựng không cần LLVM"
  "G01_driver_tests|3600|core|[bản sao] cargo test -p gt-driver --no-default-features"
  "G02_rig_tests|3600|core|[bản sao] cargo test -p rig --no-default-features"
  "G03_corpus_cli|1800|core|[bản sao] gritc check trên toàn bộ tests/{pass,fail,warn}"
  "G04_run_programs|1800|core|[bản sao] gritc run: mẫu, bench, 6 công cụ examples/"
  "G05_fmt_idempotent|1200|core|[bản sao] gritc fmt: tính lũy đẳng trên corpus pass"
  "G06_determinism|1200|core|[bản sao] gritc build hai lần ⇒ cùng băm (D9)"
  "G07_explain|300|core|[bản sao] gritc explain cho mọi mã trong KNOWN"
  "G08_probes|600|core|[bản sao] ~110 chương trình THỬ TẤN CÔNG tự viết (audit/probes): tìm âm tính giả"
  "G09_diff_checks|1500|core|[bản sao] vi sai M7: --checks=all ≡ --checks=proven trên số học ngẫu nhiên (bộ sinh riêng)"
  "G10_mutfuzz|1500|core|[bản sao] fuzz đột biến token qua CLI gritc (ICE/crash/treo)"
  "L01_linux_port|1500|core|[bản sao 2] port Linux (vá 5 tệp): gt-rt + gritc có backend LLVM 20 + liên kết bằng cc"
  "L02_native_driver_tests|5400|core|[bản sao 2] cargo test -p gt-driver (backend LLVM thật: llvm*, release*, threads, vec, utf8…)"
  "L03_native_rig_tests|3600|core|[bản sao 2] cargo test -p rig (rig build/run/test + 6 công cụ dogfood bằng exe native)"
  "L04_native_other_crates|1800|core|[bản sao 2] cargo test -p gt-guard -p gt-codegen-llvm -p gt-llvm + gt-rt"
  "L05_native_diff|1800|core|[bản sao 2] vi sai thông dịch ≡ exe native trên số học ngẫu nhiên (bộ sinh riêng)"
  "H01_llvm_probe|300|opt|Thử lấy LLVM 20 cho Linux (khả năng port)"
  "I01_fuzz_tools|1500|opt|Công cụ fuzz: nightly + cargo-fuzz (cài nếu thiếu, ngoài dự án)"
  "I02_fuzz_lex|900|opt|[bản sao] cargo-fuzz lexer ${FUZZ_SECONDS:-300}s (ASan, libFuzzer)"
  "I03_fuzz_parse|900|opt|[bản sao] cargo-fuzz parser ${FUZZ_SECONDS:-300}s (ASan, libFuzzer)"
)

# ---------- tiện ích ----------
note() { printf 'SUMMARY: %s\n' "$*"; }
in_copy() { [ -d "$COPY/.git" ] || { echo "chưa có bản sao $COPY (chạy G00_copy_patch trước)"; return 77; }; }
gritc_bin() { echo "$COPY/target/debug/gritc"; }

# ---------- các giai đoạn ----------
st_A00_env() {
  echo "== uname"; uname -a
  echo "== rust"; rustc -Vv; cargo -V; rustup toolchain list 2>&1 | head -5
  echo "== python"; python3 --version
  echo "== llvm"; llvm-config --version 2>&1 || echo "(không có llvm-config)"
  echo "== tài nguyên"; echo "cpu=$(nproc)"; free -m | sed -n 1,2p; df -h "$ROOT" | tail -1
  echo "== git"; git -C "$ROOT" log -1 --format='%H %ad %s' --date=iso
  echo "số commit nhìn thấy: $(git -C "$ROOT" rev-list --count HEAD) (kho $( [ -f "$ROOT/.git/shallow" ] && echo nông || echo đầy đủ ))"
  echo "== công cụ tuỳ chọn"
  for t in cargo-audit cargo-deny cargo-fuzz cargo-mutants wine clang lld; do
    if command -v "$t" >/dev/null 2>&1; then echo "có: $t"; else echo "THIẾU: $t"; fi
  done
  note "Linux $(uname -m), $(rustc -V | cut -d' ' -f2), LLVM hệ thống $(llvm-config --version 2>/dev/null || echo none); dự án ghim LLVM 20.1.8 + Windows MSVC"
  return 0
}

st_A01_fetch() { cd "$ROOT" && cargo fetch --locked; }

st_A02_inventory() {
  cd "$ROOT" || return 1
  echo "== thành viên workspace (cargo metadata)"
  cargo metadata --locked --no-deps --format-version 1 2>/dev/null \
    | python3 -c 'import json,sys; m=json.load(sys.stdin); print("\n".join(sorted(p["name"] for p in m["packages"])))' > "$LOGS/members.txt"
  cat "$LOGS/members.txt"
  echo "== thư mục crates/ không nằm trong workspace"
  for d in crates/*/; do n=$(basename "$d"); grep -qx "$n" "$LOGS/members.txt" || echo "NGOÀI WORKSPACE: $n"; done
  echo "== số tệp theo loại (tệp được git theo dõi)"
  git ls-files | sed -E 's/.*\.([A-Za-z0-9_]+)$/\1/' | sort | uniq -c | sort -rn | head -12
  echo "== dòng mã Rust (src, không tính tests/fuzz)"
  git ls-files 'crates/*/src/*.rs' 'runtime/gt-rt/src/*.rs' | xargs cat | wc -l
  echo "== dòng mã Rust của test"
  git ls-files 'crates/*/tests/*.rs' | xargs cat | wc -l
  echo "== tệp nhị phân bị git theo dõi (không nên có)"
  git ls-files | grep -iE '\.(exe|dll|lib|so|o|a)$' || echo "(không có)"
  echo "== CI / deny / tệp giữ chỗ TASK.md"
  ls .github/workflows/ 2>&1; ls deny.toml 2>&1; git ls-files | grep 'TASK.md'
  echo "== corpus .gt"
  for k in pass fail warn lexer; do echo "tests/$k: $(find tests/$k -name '*.gt' | wc -l) tệp .gt"; done
  echo "== tài liệu lỗi"; echo "docs/errors/*.md: $(ls docs/errors/*.md | wc -l)"
  note "workspace $(wc -l < "$LOGS/members.txt") crate; $(git ls-files | wc -l) tệp theo dõi"
  return 0
}

st_B01_fmt() { cd "$ROOT" && cargo fmt --all --check; }

st_B02_clippy() {
  cd "$ROOT" && cargo clippy --locked --workspace "${EXCL_NATIVE[@]}" --all-targets --keep-going -- -D warnings
}

st_B03_audit() {
  cd "$ROOT" || return 1
  if ! cargo audit --version >/dev/null 2>&1; then
    echo "cargo-audit chưa cài: thử cài (ngoài dự án, vào ~/.cargo/bin)"
    cargo install cargo-audit --locked || { echo "không cài được cargo-audit"; note "cargo-audit không cài được"; return 77; }
  fi
  cargo audit 2>&1 | tee "$LOGS/audit.out"; local rc=${PIPESTATUS[0]}
  if [ "$rc" != 0 ] && grep -qiE "couldn't fetch|failed to fetch|network|could not resolve|connection|certificate|403|proxy" "$LOGS/audit.out"; then
    note "cargo audit không tải được cơ sở dữ liệu advisory (mạng bị chặn)"; return 77
  fi
  return "$rc"
}

st_B04_ci_deny() {
  cd "$ROOT" || return 1
  local rc=0
  if ls .github/workflows/ci.yml >/dev/null 2>&1; then echo "có ci.yml"; else echo "THIẾU .github/workflows/ci.yml (chỉ có TASK.md giữ chỗ)"; rc=1; fi
  if [ -f deny.toml ]; then echo "có deny.toml"; else echo "THIẾU deny.toml (chỉ có deny.toml.TASK.md)"; rc=1; fi
  echo "-- GRIT_PLAN §0.7 đòi: clippy -D warnings, cargo deny, cargo audit chạy trong CI"
  [ "$rc" = 0 ] || note "dự án CHƯA có CI và CHƯA có deny.toml dù GRIT_PLAN §0.7 yêu cầu"
  return "$rc"
}

st_B05_codes() { python3 "$AUDIT/check_codes.py"; }

st_C00_build() { cd "$ROOT" && cargo build --locked --workspace "${EXCL_NATIVE[@]}" --all-targets; }

crate_test() { cd "$ROOT" && cargo test --locked -p "$1" --no-fail-fast -- --test-threads="$TEST_THREADS"; }
for c in "${PURE_CRATES[@]}"; do eval "st_C_${c//-/_}() { crate_test $c; }"; done

st_D01_native_probe() {
  cd "$ROOT" || return 1
  local fails=0 total=0
  for spec in "gt-driver|" "gt-driver|--no-default-features" "rig|--no-default-features" "gt-llvm|"; do
    local p="${spec%%|*}" f="${spec#*|}"
    total=$((total+1))
    echo "================ cargo build --locked -p $p $f"
    if cargo build --locked -p "$p" $f 2>&1 | tail -22; [ "${PIPESTATUS[0]}" != 0 ]; then
      fails=$((fails+1)); echo ">>> THẤT BẠI: -p $p $f"
    else
      echo ">>> THÀNH CÔNG: -p $p $f"
    fi
  done
  note "$fails/$total lệnh dựng nguyên trạng thất bại"
  [ "$fails" = 0 ] && return 0
  [ "$fails" = "$total" ] && return 77
  return 1
}

st_E01_gtrt() {
  cd "$ROOT" || return 1
  echo "== fmt gt-rt"; cargo fmt --manifest-path runtime/gt-rt/Cargo.toml --check && echo FMT_OK
  echo "== dựng gt-rt (release)"
  cargo build --locked --release --manifest-path runtime/gt-rt/Cargo.toml 2>&1 | tee "$LOGS/gtrt_build.out" | tail -12
  if grep -q "chỉ hỗ trợ x86_64 Windows" "$LOGS/gtrt_build.out"; then
    note "gt-rt dừng bằng compile_error! trên Linux: runtime native chỉ có cho x86_64 Windows (ADR-029)"; return 77
  fi
  cargo test --locked --manifest-path runtime/gt-rt/Cargo.toml 2>&1 | tail -15
}

st_F01_expect() { cd "$ROOT" && python3 tools/check_expectations.py; }

st_F02_tool_tests() {
  cd "$ROOT" || return 1
  local rc=0
  for t in test_check_expectations test_mut_guard test_mut_resume test_gate; do
    [ -f "tools/$t.py" ] || { echo "(không có tools/$t.py)"; continue; }
    echo "================ unittest tools/$t.py"
    python3 -m unittest "tools/$t.py" 2>&1 | tail -25; [ "${PIPESTATUS[0]}" = 0 ] || { echo ">>> LỖI: $t"; rc=1; }
  done
  return "$rc"
}

st_G00_copy_patch() {
  mkdir -p "$WORK" || return 1
  rm -rf "$COPY"
  git clone -q "$ROOT" "$COPY" || return 1
  cd "$COPY" || return 1
  echo "bản sao tại $COPY ở commit $(git log -1 --format=%h)"
  patch -p1 < "$AUDIT/patches/driver_nollvm.patch" || return 1
  echo "== dựng gritc (không LLVM) trong bản sao"
  cargo build --locked -p gt-driver --no-default-features --all-targets || return 1
  ls -la "$COPY/target/debug/gritc" && "$COPY/target/debug/gritc" --version 2>&1 | head -2
  note "đã vá 1 tệp (crates/gt-driver/src/cli.rs, +17 dòng) trong BẢN SAO để gritc dựng không cần LLVM"
}

st_G01_driver_tests() {
  in_copy || return $?
  cd "$COPY" && cargo test --locked -p gt-driver --no-default-features --no-fail-fast -- --test-threads="$TEST_THREADS"
}

st_G02_rig_tests() {
  in_copy || return $?
  cd "$COPY" && cargo test --locked -p rig --no-default-features --no-fail-fast -- --test-threads="$TEST_THREADS"
}

st_G03_corpus_cli() {
  in_copy || return $?
  [ -x "$(gritc_bin)" ] || { echo "chưa có gritc"; return 77; }
  python3 "$AUDIT/corpus_cli.py" --gritc "$(gritc_bin)" --tests "$ROOT/tests" --jobs 4
}

st_G04_run_programs() {
  in_copy || return $?
  local g; g="$(gritc_bin)"; [ -x "$g" ] || return 77
  local n=0 ok=0 bad=0 abort=0 lim=0 other=0
  echo "== tests/samples + tests/bench (chạy bằng thông dịch MIR)"
  for f in "$ROOT"/tests/samples/*.gt "$ROOT"/tests/bench/*.gt; do
    n=$((n+1)); local exp; exp=$(grep -m1 -oE '// expect-result: *[^ ]+' "$f" | sed 's/.*: *//')
    timeout 600 "$g" run "$f" --backend=interpreter --json > "$LOGS/run_one.json" 2> "$LOGS/run_one.err"; local rc=$?
    local val; val=$(python3 -c 'import json,sys
try:
  j=json.load(open(sys.argv[1])); print(j.get("run",{}).get("value"), j.get("verdict"))
except Exception as e: print("NOJSON")' "$LOGS/run_one.json")
    echo "$(basename "$f"): exit=$rc kỳ vọng=${exp:-?} nhận=$val"
    case $rc in 0) ok=$((ok+1));; 70) abort=$((abort+1));; 71) lim=$((lim+1));; *) other=$((other+1));; esac
  done
  echo "== 6 công cụ dogfood (examples/*/src) — cần Cap<Fs>/args/IO của runtime native"
  for d in "$ROOT"/examples/*/; do
    local name; name=$(basename "$d")
    timeout 300 "$g" run "$d/src" --backend=interpreter > "$LOGS/run_ex.out" 2>&1; local rc=$?
    echo "examples/$name: exit=$rc :: $(tail -1 "$LOGS/run_ex.out" | cut -c1-200)"
  done
  note "mẫu/bench chạy bằng thông dịch: $n chương trình; exit0=$ok abort70=$abort hết giới hạn71=$lim khác=$other"
  return 0
}

st_G05_fmt_idempotent() {
  in_copy || return $?
  local g; g="$(gritc_bin)"; [ -x "$g" ] || return 77
  local d="$WORK/fmt"; rm -rf "$d"; mkdir -p "$d"
  local n=0 rej=0 bad=0 changed=0
  for f in "$ROOT"/tests/pass/*.gt; do
    n=$((n+1)); local b; b=$(basename "$f"); cp "$f" "$d/$b"
    if ! "$g" fmt "$d/$b" >/dev/null 2>"$d/err"; then rej=$((rej+1)); echo "TỪ CHỐI: $b :: $(head -1 "$d/err" | cut -c1-150)"; continue; fi
    cp "$d/$b" "$d/$b.1"
    "$g" fmt "$d/$b" >/dev/null 2>&1; if ! cmp -s "$d/$b" "$d/$b.1"; then bad=$((bad+1)); echo "KHÔNG LŨY ĐẲNG: $b"; fi
    cmp -s "$f" "$d/$b" || changed=$((changed+1))
  done
  note "gritc fmt trên $n mẫu pass: bị từ chối $rej; không lũy đẳng $bad; (mẫu gốc chưa ở dạng chuẩn: $changed)"
  [ "$bad" = 0 ] && [ "$rej" = 0 ]
}

st_G06_determinism() {
  in_copy || return $?
  local g; g="$(gritc_bin)"; [ -x "$g" ] || return 77
  local d="$WORK/det"; rm -rf "$d"; mkdir -p "$d"
  local n=0 diff=0 nogo=0
  for f in $(ls "$ROOT"/tests/pass/*.gt | awk 'NR%8==1'); do
    n=$((n+1)); local b; b=$(basename "$f" .gt)
    if ! "$g" build "$f" --backend=interpreter -o "$d/$b.1.gtmir" >/dev/null 2>&1; then nogo=$((nogo+1)); continue; fi
    "$g" build "$f" --backend=interpreter -o "$d/$b.2.gtmir" >/dev/null 2>&1
    cmp -s "$d/$b.1.gtmir" "$d/$b.2.gtmir" || { diff=$((diff+1)); echo "KHÁC NHAU: $b"; }
    "$g" check "$f" --json > "$d/$b.j1" 2>/dev/null; "$g" check "$f" --json > "$d/$b.j2" 2>/dev/null
    cmp -s "$d/$b.j1" "$d/$b.j2" || { diff=$((diff+1)); echo "JSON KHÁC NHAU: $b"; }
  done
  note "tính tất định (D9): $n mẫu dựng hai lần; khác nhau $diff; không dựng được $nogo"
  [ "$diff" = 0 ]
}

st_G07_explain() {
  in_copy || return $?
  local g; g="$(gritc_bin)"; [ -x "$g" ] || return 77
  local n=0 bad=0
  for c in $(python3 -c 'import re,sys; s=open(sys.argv[1],encoding="utf-8").read(); m=re.search(r"const KNOWN: &\[&str\] = &\[(.*?)\];",s,re.S); print(" ".join(re.findall(r"\"([ENS]\d{3,4})\"",m.group(1))))' "$ROOT/crates/gt-diag/src/code.rs"); do
    n=$((n+1))
    if ! "$g" explain "$c" > "$WORK/explain.out" 2>&1 || [ ! -s "$WORK/explain.out" ]; then bad=$((bad+1)); echo "explain $c LỖI"; fi
  done
  note "gritc explain: $n mã trong KNOWN (code.rs); lỗi $bad"
  [ "$bad" = 0 ]
}

st_G08_probes() {
  in_copy || return $?
  local g; g="$(gritc_bin)"; [ -x "$g" ] || return 77
  python3 "$AUDIT/probes_run.py" --gritc "$g" --dir "$AUDIT/probes"
}

st_G09_diff_checks() {
  in_copy || return $?
  local g; g="$(gritc_bin)"; [ -x "$g" ] || return 77
  python3 "$AUDIT/diff_checks.py" --gritc "$g" --n "${DIFF_N:-3000}" --seed 5 --jobs 2
}

st_G10_mutfuzz() {
  in_copy || return $?
  local g; g="$(gritc_bin)"; [ -x "$g" ] || return 77
  mkdir -p "$AUDIT/results/fuzz_artifacts/mutfuzz"
  python3 "$AUDIT/mutfuzz.py" --gritc "$g" --n "${MUTFUZZ_N:-8000}" --seed 11 --jobs 2 --out "$AUDIT/results/fuzz_artifacts/mutfuzz"
}

LCOPY="$WORK/grit_linux"
in_lcopy() { [ -d "$LCOPY/.git" ] || { echo "chưa có $LCOPY (chạy L01_linux_port trước)"; return 77; }; }
lgritc() { echo "$LCOPY/target/debug/gritc"; }

st_L01_linux_port() {
  [ -e /usr/lib/x86_64-linux-gnu/libLLVM-20.so ] || { note "thiếu libLLVM-20.so (gói libllvm20): không port được"; return 77; }
  mkdir -p "$WORK" && rm -rf "$LCOPY" && git clone -q "$ROOT" "$LCOPY" || return 1
  cd "$LCOPY" && patch -p1 < "$AUDIT/patches/linux_port.patch" || return 1
  echo "== gt-rt (staticlib, Linux)"
  (cd runtime/gt-rt && cargo build --release --locked 2>&1 | tail -3 && cargo test --locked 2>&1 | grep -E 'test result|FAILED') || return 1
  echo "== gritc có backend LLVM"
  GRIT_LLVM_DIR=/usr cargo build --locked -p gt-driver --all-targets 2>&1 | tail -3 || return 1
  local g; g="$(lgritc)"
  printf 'ƒ main() → i32 ! ∅ { μs ≔ 0; μi ≔ 0; ⟳ i < 10 ↓ 10 - i { s ← s + i; i ← i + 1; } s }\n' > "$WORK/n1.gt"
  "$g" run "$WORK/n1.gt" 2>&1 | tail -2
  "$g" build "$WORK/n1.gt" -o "$WORK/n1.exe" >/dev/null 2>&1 && file "$WORK/n1.exe" | cut -c1-100
  note "port Linux: vá 5 tệp (gt-llvm/build.rs, gt-driver/build.rs+native.rs, rig/build.rs, gt-rt/src/lib.rs, +62/−26 dòng); LLVM hệ thống $(ls /usr/lib/x86_64-linux-gnu/libLLVM.so.20* 2>/dev/null | head -1 | sed 's/.*libLLVM.so.//') (gói libllvm20) thay vì 20.1.8 của dự án"
}

st_L02_native_driver_tests() {
  in_lcopy || return $?
  cd "$LCOPY" && GRIT_LLVM_DIR=/usr cargo test --locked -p gt-driver --no-fail-fast -- --test-threads="$TEST_THREADS"
}

st_L03_native_rig_tests() {
  in_lcopy || return $?
  cd "$LCOPY" && GRIT_LLVM_DIR=/usr cargo test --locked -p rig --no-fail-fast -- --test-threads="$TEST_THREADS"
}

st_L04_native_other_crates() {
  in_lcopy || return $?
  cd "$LCOPY" && GRIT_LLVM_DIR=/usr cargo test --locked -p gt-guard -p gt-codegen-llvm -p gt-llvm --no-fail-fast -- --test-threads="$TEST_THREADS"
  local rc=$?
  (cd runtime/gt-rt && cargo test --locked 2>&1 | grep -E 'test result|FAILED')
  return $rc
}

st_L05_native_diff() {
  in_lcopy || return $?
  local g; g="$(lgritc)"; [ -x "$g" ] || return 77
  python3 "$AUDIT/diff_checks.py" --gritc "$g" --n "${NATIVE_DIFF_N:-600}" --seed 3 --jobs 2 "--left=--backend=interpreter" "--right=--backend=llvm"
}

st_H01_llvm_probe() {
  echo "== apt"; apt-cache policy llvm-20 libllvm20 llvm-20-dev 2>&1 | head -12
  echo "== tải LLVM 20.1.8 bản Linux từ GitHub (chỉ HEAD)"
  local ca=(); [ -f /root/.ccr/ca-bundle.crt ] && ca=(--cacert /root/.ccr/ca-bundle.crt)
  curl -sS -m 25 "${ca[@]}" -I "https://github.com/llvm/llvm-project/releases/download/llvmorg-20.1.8/LLVM-20.1.8-Linux-X64.tar.xz" 2>&1 | head -5
  echo "== apt.llvm.org"; curl -sS -m 20 "${ca[@]}" -I https://apt.llvm.org/llvm.sh 2>&1 | head -3
  note "kết quả thăm dò xem log; chưa dựng gì"
  return 77
}

st_I01_fuzz_tools() {
  rustup toolchain list
  if ! rustup toolchain list | grep -q '^nightly'; then
    rustup toolchain install nightly --profile minimal --no-self-update || { note "không cài được nightly"; return 77; }
  fi
  if ! cargo fuzz --help >/dev/null 2>&1; then
    cargo install cargo-fuzz --locked || { note "không cài được cargo-fuzz"; return 77; }
  fi
  cargo +nightly --version && cargo fuzz --version
  note "nightly + cargo-fuzz sẵn sàng"
}

fuzz_one() { # target crate
  in_copy || return $?
  command -v cargo-fuzz >/dev/null 2>&1 || { note "chưa có cargo-fuzz (chạy I01_fuzz_tools)"; return 77; }
  local secs="${FUZZ_SECONDS:-300}" dir="$COPY/crates/$2/fuzz"
  cd "$dir" || return 1
  cargo +nightly fuzz run "$1" -- -max_total_time="$secs" -rss_limit_mb=4096 2>&1 | tail -40
  local rc=${PIPESTATUS[0]}
  local n; n=$(ls "$dir/artifacts/$1" 2>/dev/null | wc -l)
  mkdir -p "$AUDIT/results/fuzz_artifacts/$1"
  if [ "$n" -gt 0 ]; then cp "$dir/artifacts/$1"/* "$AUDIT/results/fuzz_artifacts/$1/" 2>/dev/null; fi
  note "fuzz $1: ${secs}s, thoát $rc, $n tệp artifact (crash/timeout/oom) — chép vào audit/results/fuzz_artifacts/$1 nếu có"
  [ "$rc" = 0 ] && [ "$n" = 0 ]
}

st_I02_fuzz_lex() { fuzz_one lex gt-lexer; }
st_I03_fuzz_parse() { fuzz_one parse gt-parser; }

# ---------- khung chạy ----------
state_get() { awk -F'\t' -v id="$1" '$1==id {print $2}' "$STATE" 2>/dev/null; }
state_put() { # id status rc start secs note — có khoá tệp để hai bộ chạy song song không ghi đè nhau
  (
    flock 9
    local tmp; tmp="$(mktemp)"
    { grep -v -P "^$1\t" "$STATE" 2>/dev/null || true; printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$1" "$2" "$3" "$4" "$5" "$6"; } > "$tmp"
    sort -t $'\t' -k1,1 "$tmp" > "$STATE"; rm -f "$tmp"
  ) 9> "$STATE.lock"
}
repo_snapshot() { git -C "$ROOT" status --porcelain --untracked-files=all -- . ':!audit' 2>/dev/null | sort; }

run_stage() {
  local spec="$1" id tmo opt title
  IFS='|' read -r id tmo opt title <<< "$spec"
  local log="$LOGS/$id.log" start; start="$(date -u +%FT%TZ)"
  echo "##### $id — $title (timeout ${tmo}s)"
  state_put "$id" RUNNING - "$start" 0 ""
  local before after t0 rc
  before="$(repo_snapshot)"; t0=$(date +%s)
  timeout -k 30 "$tmo" bash "$AUDIT/run_all.sh" --exec "$id" > "$log" 2>&1; rc=$?
  local secs=$(( $(date +%s) - t0 ))
  after="$(repo_snapshot)"
  local status note=""
  case $rc in 0) status=PASS;; 77) status=BLOCKED;; 78) status=SKIP;; 124|137) status=TIMEOUT;; *) status=FAIL;; esac
  if [ "$before" != "$after" ]; then
    note="DỰ ÁN BỊ ĐỔI trong lúc chạy: $(diff <(echo "$before") <(echo "$after") | grep '^[<>]' | head -5 | tr '\n' ';')"
    echo "!!!! $note" | tee -a "$log"
  fi
  python3 "$AUDIT/summarize.py" "$id" "$log" "$status" "$rc" "$secs" "$start" "$title" "$note"
  state_put "$id" "$status" "$rc" "$start" "$secs" "$note"
  python3 "$AUDIT/report.py"
  echo "##### $id => $status (${secs}s)"
}

list_table() {
  printf '%-22s %-9s %-5s %-6s %s\n' ID TRẠNG-THÁI THOÁT GIÂY TÊN
  for spec in "${STAGES[@]}"; do
    IFS='|' read -r id tmo opt title <<< "$spec"
    local s; s="$(awk -F'\t' -v id="$id" '$1==id {print $2"\t"$3"\t"$5}' "$STATE" 2>/dev/null)"
    if [ -n "$s" ]; then IFS=$'\t' read -r a b c <<< "$s"; printf '%-22s %-9s %-5s %-6s %s\n' "$id" "$a" "$b" "$c" "$title"
    else printf '%-22s %-9s %-5s %-6s %s\n' "$id" PENDING "" "" "$title"; fi
  done
}

main() {
  if [ "${1:-}" = "--exec" ]; then
    set +e; local fn="st_${2//-/_}"; "$fn"; exit $?
  fi
  if [ "${1:-}" = "--ids" ]; then printf '%s\n' "${STAGES[@]}"; exit 0; fi
  if [ "${1:-}" = "--list" ]; then list_table; exit 0; fi
  if [ "${1:-}" = "--report" ]; then python3 "$AUDIT/report.py"; exit 0; fi
  local only="" group="" retry=0 next=0 from=""
  while [ $# -gt 0 ]; do
    case "$1" in
      --only) only="$2"; shift 2;;
      --group) group="$2"; shift 2;;
      --retry-failed) retry=1; shift;;
      --next) next="$2"; shift 2;;
      --from) from="$2"; shift 2;;
      *) echo "đối số lạ: $1"; exit 3;;
    esac
  done
  local ran=0 started=0
  [ -z "$from" ] && started=1
  for spec in "${STAGES[@]}"; do
    IFS='|' read -r id tmo opt title <<< "$spec"
    [ "$started" = 0 ] && { [ "$id" = "$from" ] && started=1 || continue; }
    [ -n "$group" ] && [[ "$id" != "$group"* ]] && continue
    if [ -n "$only" ]; then case ",$only," in *",$id,"*) ;; *) continue;; esac
    else
      local s; s="$(state_get "$id")"
      case "$s" in
        PASS|BLOCKED|SKIP) continue;;
        FAIL|TIMEOUT) [ "$retry" = 1 ] || continue;;
      esac
    fi
    run_stage "$spec"; ran=$((ran+1))
    [ "$next" != 0 ] && [ "$ran" -ge "$next" ] && break
  done
  echo "Đã chạy $ran giai đoạn. Bảng trạng thái:"; list_table
  python3 "$AUDIT/report.py"
}
main "$@"
