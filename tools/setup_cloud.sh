#!/bin/bash
# Cài đặt từ kho mới sang máy Linux (container đám mây): Rust, LLVM 20, dựng runtime + gritc + rig, chạy `gritc doctor`. IDEMPOTENT (chạy lại an toàn, bỏ qua bước đã xong).
# Dùng: bash tools/setup_cloud.sh            (in thời gian từng bước; thoát 0 = XANH)
# Nền đã kiểm: Ubuntu 24.04 x86_64 (apt `libllvm20` = LLVM 20.1.2). Windows: KHÔNG dùng script này (xem BUILDING.md; chưa kiểm).
set -u
# Cờ (GÓI 11 S0.b): --check = KHÔNG cài gì, in OK hoặc danh sách thiếu kèm LỆNH CHÍNH XÁC (thoát 0 nếu đủ, 1 nếu thiếu);
#   --install-system = cho phép cài gói HỆ THỐNG bằng `sudo -n apt-get` (không hỏi mật khẩu; thất bại ⇒ in dòng `sudo apt install ...` cho Giang);
#   --fuzz = cũng cài cargo-fuzz (cấp người dùng). Mặc định chỉ cài thứ CẤP NGƯỜI DÙNG (toolchain ghim, cargo-audit --locked).
CHECK=0; SYS=0; FUZZ=0
for a in "$@"; do case "$a" in --check) CHECK=1 ;; --install-system) SYS=1 ;; --fuzz) FUZZ=1 ;; *) echo "setup_cloud: cờ lạ $a (dùng --check, --install-system, --fuzz)" >&2; exit 2 ;; esac; done
cd "$(dirname "$0")/.." || exit 1
ROOT="$(pwd)"
T0=$(date +%s)
step_t=$T0
SUDO=""
[ "$(id -u)" != 0 ] && command -v sudo >/dev/null 2>&1 && SUDO="sudo"
tick() { local now; now=$(date +%s); echo "  [$(( now - step_t )) s] $1"; step_t=$now; }
die() { echo "setup_cloud: LỖI: $1" >&2; echo "SETUP_RESULT=RED"; exit 1; }
echo "== setup_cloud: $(uname -sm), thư mục $ROOT"

MISSING_SYS=""; MISSING_USER=""
want_ver() { awk -F= -v k="$1" '$1==k {gsub(/[[:space:]]/,"",$2); print $2}' "$ROOT/tools/TOOLCHAIN" 2>/dev/null; }
RUSTV="$(want_ver rustc)"; RUSTV="${RUSTV:-1.98.1}"
. "$ROOT/tools/pyfind.sh"
# --- gói HỆ THỐNG (apt) ---
need_pkg() { # $1 = tên gói apt; $2 = lệnh/tệp kiểm
  if ! eval "$2" >/dev/null 2>&1; then MISSING_SYS="$MISSING_SYS $1"; fi
}
need_pkg build-essential "command -v cc && command -v make"
need_pkg curl "command -v curl"
need_pkg git "command -v git"
need_pkg python3 "[ -n \"$PY\" ]"
need_pkg libffi-dev "ls /usr/include/ffi.h /usr/include/*/ffi.h"
need_pkg libzstd-dev "ls /usr/include/zstd.h"
need_pkg zlib1g-dev "ls /usr/include/zlib.h"
need_pkg libxml2-dev "ls /usr/include/libxml2/libxml/parser.h"
find_llvm_dir() {
  for d in "${GRIT_LLVM_LIB_DIR:-}" /usr/lib/x86_64-linux-gnu /usr/lib64 /usr/lib/llvm-20/lib "$HOME/.grit/llvm/lib"; do
    [ -n "$d" ] && [ -f "$d/libLLVM-20.so" ] && { echo "$d"; return 0; }
  done
  return 1
}
LLVMDIR="$(find_llvm_dir)" || LLVMDIR=""
[ -n "$LLVMDIR" ] || MISSING_SYS="$MISSING_SYS libllvm20"
# TSan (tuỳ chọn): clang — chỉ liệt kê, KHÔNG làm --check đỏ
OPT_SYS=""; command -v clang >/dev/null 2>&1 || OPT_SYS="clang"
# --- CẤP NGƯỜI DÙNG ---
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
command -v rustup >/dev/null 2>&1 || MISSING_USER="$MISSING_USER|curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain none"
if command -v rustup >/dev/null 2>&1; then
  rustup toolchain list 2>/dev/null | grep -q "^$RUSTV" || MISSING_USER="$MISSING_USER|rustup toolchain install $RUSTV --profile minimal -c rustfmt -c clippy"
fi
command -v cargo-audit >/dev/null 2>&1 || MISSING_USER="$MISSING_USER|cargo install cargo-audit --locked"
if [ "$FUZZ" = 1 ]; then command -v cargo-fuzz >/dev/null 2>&1 || MISSING_USER="$MISSING_USER|cargo install cargo-fuzz --locked"; fi
sys_line() { echo "sudo apt install -y $(echo "$MISSING_SYS" | sed 's/^ *//')"; }
if [ "$CHECK" = 1 ]; then
  if [ -z "$MISSING_SYS" ] && [ -z "$MISSING_USER" ]; then
    echo "SETUP_CHECK=OK (rustc ghim $RUSTV; LLVM ${LLVMDIR:-?}; python: ${PY:-?})"
    [ -z "$OPT_SYS" ] || echo "  tuỳ chọn (TSan): sudo apt install -y $OPT_SYS"
    exit 0
  fi
  echo "SETUP_CHECK=THIẾU"
  [ -z "$MISSING_SYS" ] || echo "  gói hệ thống: $(sys_line)"
  echo "$MISSING_USER" | tr '|' '
' | sed '/^$/d; s/^/  cấp người dùng: /'
  [ -z "$OPT_SYS" ] || echo "  tuỳ chọn (TSan): sudo apt install -y $OPT_SYS"
  exit 1
fi
# --- cài đặt thật ---
if [ -n "$MISSING_SYS" ]; then
  if [ "$SYS" = 1 ]; then
    if [ "$(id -u)" = 0 ]; then apt-get update -qq && apt-get install -y -qq $MISSING_SYS || die "apt-get không cài được:$MISSING_SYS"
    else sudo -n true 2>/dev/null || die "cần sudo không hỏi mật khẩu cho --install-system; Giang chạy: $(sys_line)"; sudo -n apt-get update -qq && sudo -n apt-get install -y -qq $MISSING_SYS || die "apt-get lỗi; Giang chạy: $(sys_line)"; fi
  else
    die "thiếu gói hệ thống; Giang chạy: $(sys_line)   (hoặc chạy lại với --install-system nếu có sudo -n)"
  fi
fi
tick "gói hệ thống"
if ! command -v rustup >/dev/null 2>&1; then curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain none >/dev/null 2>&1 || die "không cài được rustup"; . "$HOME/.cargo/env"; fi
rustup toolchain list 2>/dev/null | grep -q "^$RUSTV" || rustup toolchain install "$RUSTV" --profile minimal -c rustfmt -c clippy >/dev/null 2>&1 || die "không cài được rustc $RUSTV"
command -v cargo-audit >/dev/null 2>&1 || cargo install cargo-audit --locked >/dev/null 2>&1 || die "không cài được cargo-audit"
[ "$FUZZ" = 1 ] && { command -v cargo-fuzz >/dev/null 2>&1 || cargo install cargo-fuzz --locked >/dev/null 2>&1 || die "không cài được cargo-fuzz"; }
command -v cargo >/dev/null 2>&1 || die "cargo vẫn không có trong PATH"
tick "Rust $(rustc --version 2>/dev/null | cut -d' ' -f2) (ghim $RUSTV), cargo-audit"
export GRIT_LLVM_LIB_DIR="$LLVMDIR"
tick "LLVM 20: $LLVMDIR"

# 4. runtime gt-rt (workspace riêng) rồi gritc + rig (release)
cargo build --release --manifest-path runtime/gt-rt/Cargo.toml 2>&1 | tail -1 | grep -q "Finished" || die "dựng runtime/gt-rt thất bại (chạy lại lệnh cargo để xem lỗi)"
tick "dựng runtime gt-rt (release)"
cargo build --release -p gt-driver -p rig --bin gritc --bin rig 2>&1 | tail -1 | grep -q "Finished" || die "dựng gritc/rig thất bại (chạy: cargo build --release -p gt-driver -p rig --bin gritc --bin rig)"
[ -x target/release/gritc ] && [ -x target/release/rig ] || die "thiếu target/release/gritc hoặc target/release/rig sau khi dựng"
tick "dựng gritc + rig (release)"

# 5. kiểm môi trường
"$ROOT/target/release/gritc" doctor || die "gritc doctor ĐỎ (xem các dòng LỖI ở trên)"
tick "gritc doctor"

echo "== xong sau $(( $(date +%s) - T0 )) s. Thêm vào PATH và biến môi trường:"
echo "   export PATH=\"$ROOT/target/release:\$PATH\"; export GRIT_LLVM_LIB_DIR=\"$LLVMDIR\""
echo "SETUP_RESULT=GREEN"
