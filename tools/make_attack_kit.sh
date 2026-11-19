#!/usr/bin/env bash
# GÓI 8 S0c: tạo KIT cho phiên tấn công HỘP ĐEN (không có mã nguồn Grit).
# Dùng:  bash tools/make_attack_kit.sh <thư_mục_đích>   (thư mục đích PHẢI chưa tồn tại hoặc rỗng)
# Kit chứa: gritc.exe (release), rig.exe, gt_rt.lib (runtime), LLVM-C.dll (+ cách đặt PATH), lib/std (gói std; gritc đọc qua GRIT_STD),
#           tài liệu để KHÔNG phát hiện lại cái đã biết, docs/spec, docs/errors, docs/E_PROMPT_ATTACK.md.
# KHÔNG chứa: crates/, tests/, lib/core (prelude đã nhúng vào gritc lúc dựng), mã nguồn Rust, lịch sử git, bí mật.
# Kiểm sau khi tạo: gritc check và gritc run một chương trình mẫu từ chính thư mục kit (Windows).
set -euo pipefail
DEST="${1:?dùng: bash tools/make_attack_kit.sh <thư_mục_đích>}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LLVM_BIN="${GRIT_LLVM_DIR:-D:/Tools/llvm-20.1.8}/bin"
if [ -e "$DEST" ] && [ -n "$(ls -A "$DEST" 2>/dev/null || true)" ]; then
  echo "KIT: thư mục đích không rỗng: $DEST" >&2; exit 2
fi
mkdir -p "$DEST/bin" "$DEST/runtime" "$DEST/lib" "$DEST/docs" "$DEST/work"
DEST="$(cd "$DEST" && pwd)"  # đường dẫn POSIX tuyệt đối: "C:/…" có dấu : làm vỡ PATH của Git Bash

copy() { cp "$1" "$2" || { echo "KIT: thiếu $1" >&2; exit 3; }; }
copy "$ROOT/target/release/gritc.exe" "$DEST/bin/gritc.exe"
copy "$ROOT/target/release/rig.exe" "$DEST/bin/rig.exe"
copy "$ROOT/runtime/gt-rt/target/release/gt_rt.lib" "$DEST/runtime/gt_rt.lib"
copy "$LLVM_BIN/LLVM-C.dll" "$DEST/bin/LLVM-C.dll"
# gói std (đầu vào của gritc qua GRIT_STD); lib/core KHÔNG cần (prelude đã nhúng)
cp -r "$ROOT/lib/std" "$DEST/lib/std"

for f in GRIT_PLAN.md docs/AI_GUIDE.md docs/STD_REFERENCE.md docs/LANGUAGE_REFERENCE.md docs/SAFETY_CLAIMS.md \
         docs/SEED_LIMITATIONS.md docs/KNOWN_LIMITATIONS.md docs/E_PROMPT_ATTACK.md; do
  if [ -f "$ROOT/$f" ]; then cp "$ROOT/$f" "$DEST/docs/$(basename "$f")"; else echo "KIT: bỏ qua (không có) $f"; fi
done
cp -r "$ROOT/docs/spec" "$DEST/docs/spec"
cp -r "$ROOT/docs/errors" "$DEST/docs/errors"

cat > "$DEST/SETUP.md" <<'EOF'
# Đặt môi trường (Windows, Git Bash hoặc PowerShell)
PATH phải chứa thư mục `bin` của kit (có LLVM-C.dll) để gritc chạy:
  export PATH="$PWD/bin:$PATH"      # Git Bash
  $env:PATH = "$PWD\bin;$env:PATH"  # PowerShell
  export GRIT_STD="$PWD/lib/std"    # gói std
  export GRIT_RT_LIB="$PWD/runtime/gt_rt.lib"   # để `gritc run` (backend LLVM)
Kiểm: bin/gritc.exe --version  (phải in 1.0.0-rc…)
Làm việc TRONG thư mục work/. KHÔNG ghi vào các thư mục khác.
EOF

# kiểm: gritc check + run một chương trình mẫu từ chính kit
mkdir -p "$DEST/work/selftest"
printf '%s\n' 'ƒ main() → i32 ! ∅ { 7 }' > "$DEST/work/selftest/main.gt"
export GRIT_STD="$DEST/lib/std" GRIT_RT_LIB="$DEST/runtime/gt_rt.lib" PATH="$DEST/bin:$PATH"
"$DEST/bin/gritc.exe" --version
"$DEST/bin/gritc.exe" check "$DEST/work/selftest/main.gt" && echo "KIT_CHECK_OK"
"$DEST/bin/gritc.exe" run "$DEST/work/selftest/main.gt"; echo "KIT_RUN_EXIT=$?"
echo "KIT_OK: $DEST"
