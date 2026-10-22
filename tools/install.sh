#!/bin/bash
# Cài Grit trên Linux: chạy tools/setup_cloud.sh (Rust, LLVM 20, dựng gritc + rig) rồi đặt liên kết `gritc`/`rig` vào ~/.local/bin.
# Dùng: bash tools/install.sh [thư-mục-đích-mặc-định ~/.local/bin]. Windows: pwsh tools/install.ps1 (CHƯA kiểm).
set -u
cd "$(dirname "$0")/.." || exit 1
ROOT="$(pwd)"
bash tools/setup_cloud.sh || exit 1
DEST="${1:-$HOME/.local/bin}"
mkdir -p "$DEST" || exit 1
ln -sf "$ROOT/target/release/gritc" "$DEST/gritc"
ln -sf "$ROOT/target/release/rig" "$DEST/rig"
echo "đã liên kết gritc, rig vào $DEST (thêm vào PATH nếu chưa có: export PATH=\"$DEST:\$PATH\")."
echo "lưu ý: gritc nhớ đường dẫn runtime (runtime/gt-rt/target/release) theo thư mục kho này — ĐỪNG di chuyển/xoá kho."
