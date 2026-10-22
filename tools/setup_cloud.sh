#!/bin/bash
# Cài đặt từ kho mới sang máy Linux (container đám mây): Rust, LLVM 20, dựng runtime + gritc + rig, chạy `gritc doctor`. IDEMPOTENT (chạy lại an toàn, bỏ qua bước đã xong).
# Dùng: bash tools/setup_cloud.sh            (in thời gian từng bước; thoát 0 = XANH)
# Nền đã kiểm: Ubuntu 24.04 x86_64 (apt `libllvm20` = LLVM 20.1.2). Windows: KHÔNG dùng script này (xem BUILDING.md; chưa kiểm).
set -u
cd "$(dirname "$0")/.." || exit 1
ROOT="$(pwd)"
T0=$(date +%s)
step_t=$T0
SUDO=""
[ "$(id -u)" != 0 ] && command -v sudo >/dev/null 2>&1 && SUDO="sudo"
tick() { local now; now=$(date +%s); echo "  [$(( now - step_t )) s] $1"; step_t=$now; }
die() { echo "setup_cloud: LỖI: $1" >&2; echo "SETUP_RESULT=RED"; exit 1; }
echo "== setup_cloud: $(uname -sm), thư mục $ROOT"

# 1. công cụ hệ thống: cc (build-essential), curl, python3, git
need=""
for c in cc curl python3 git; do command -v "$c" >/dev/null 2>&1 || need="$need $c"; done
if [ -n "$need" ]; then
  command -v apt-get >/dev/null 2>&1 || die "thiếu:$need và không có apt-get; cài thủ công rồi chạy lại"
  $SUDO apt-get update -qq && $SUDO apt-get install -y -qq build-essential curl python3 git ca-certificates || die "apt-get không cài được:$need"
fi
tick "công cụ hệ thống (cc, curl, python3, git)"

# 2. Rust (cargo ≥ 1.85, edition 2024)
if ! command -v cargo >/dev/null 2>&1; then
  [ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
fi
if ! command -v cargo >/dev/null 2>&1; then
  curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable >/dev/null 2>&1 || die "không cài được Rust (rustup); kiểm tra mạng tới sh.rustup.rs"
  . "$HOME/.cargo/env"
fi
command -v cargo >/dev/null 2>&1 || die "cargo vẫn không có trong PATH"
tick "Rust $(rustc --version 2>/dev/null | cut -d' ' -f2)"

# 3. LLVM 20 (libLLVM-20.so): ưu tiên gói hệ thống; không có thì thử tải bản dựng sẵn 20.1.8 (ĐƯỜNG DỰ PHÒNG — CHƯA kiểm trên máy khác Ubuntu 24.04)
find_llvm_dir() {
  for d in "${GRIT_LLVM_LIB_DIR:-}" /usr/lib/x86_64-linux-gnu /usr/lib64 /usr/lib/llvm-20/lib "$HOME/.grit/llvm/lib"; do
    [ -n "$d" ] && [ -f "$d/libLLVM-20.so" ] && { echo "$d"; return 0; }
  done
  return 1
}
LLVMDIR="$(find_llvm_dir)" || LLVMDIR=""
if [ -z "$LLVMDIR" ] && command -v apt-get >/dev/null 2>&1; then
  $SUDO apt-get install -y -qq libllvm20 >/dev/null 2>&1 || { $SUDO apt-get update -qq >/dev/null 2>&1; $SUDO apt-get install -y -qq libllvm20 >/dev/null 2>&1; }
  LLVMDIR="$(find_llvm_dir)" || LLVMDIR=""
fi
if [ -z "$LLVMDIR" ]; then
  echo "  gói libllvm20 không có; thử tải LLVM 20.1.8 dựng sẵn (dự phòng, chưa kiểm)..."
  mkdir -p "$HOME/.grit" && cd "$HOME/.grit" || die "không tạo được ~/.grit"
  curl -sSfL -o llvm.tar.xz "https://github.com/llvm/llvm-project/releases/download/llvmorg-20.1.8/LLVM-20.1.8-Linux-X64.tar.xz" || die "không tải được LLVM 20.1.8; cài libllvm20 thủ công hoặc đặt GRIT_LLVM_LIB_DIR"
  rm -rf llvm && mkdir llvm && tar -xJf llvm.tar.xz -C llvm --strip-components=1 || die "giải nén LLVM thất bại"
  [ -f llvm/lib/libLLVM-20.so ] || { f="$(ls llvm/lib/libLLVM.so.20* 2>/dev/null | head -1)"; [ -n "$f" ] && ln -sf "$(basename "$f")" llvm/lib/libLLVM-20.so; }
  cd "$ROOT" || exit 1
  LLVMDIR="$(find_llvm_dir)" || LLVMDIR=""
fi
[ -n "$LLVMDIR" ] || die "không tìm thấy libLLVM-20.so (cài gói libllvm20 hoặc đặt GRIT_LLVM_LIB_DIR tới thư mục chứa nó)"
export GRIT_LLVM_LIB_DIR="$LLVMDIR"
tick "LLVM 20: $LLVMDIR"

# 4. runtime gt-rt (workspace riêng) rồi gritc + rig (release)
cargo build --release --manifest-path runtime/gt-rt/Cargo.toml 2>&1 | tail -1 | grep -q "Finished" || die "dựng runtime/gt-rt thất bại (chạy lại lệnh cargo để xem lỗi)"
tick "dựng runtime gt-rt (release)"
cargo build --release -p gt-driver --bin gritc -p rig 2>&1 | tail -1 | grep -q "Finished" || die "dựng gritc/rig thất bại (chạy: cargo build --release -p gt-driver --bin gritc -p rig)"
tick "dựng gritc + rig (release)"

# 5. kiểm môi trường
"$ROOT/target/release/gritc" doctor || die "gritc doctor ĐỎ (xem các dòng LỖI ở trên)"
tick "gritc doctor"

echo "== xong sau $(( $(date +%s) - T0 )) s. Thêm vào PATH và biến môi trường:"
echo "   export PATH=\"$ROOT/target/release:\$PATH\"; export GRIT_LLVM_LIB_DIR=\"$LLVMDIR\""
echo "SETUP_RESULT=GREEN"
