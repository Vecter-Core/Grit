#!/bin/bash
# So sánh dev ≡ release theo lô nhỏ (ADR-036 §6c, Q77). Chạy lại TOÀN BỘ bộ đối chiếu ở hồ sơ release (GT_PROFILE=release); mỗi bộ độc lập so với
# mô hình/thông dịch/kỳ vọng cố định (đã xanh ở dev) ⇒ release xanh ở cùng kỳ vọng = dev ≡ release. Kết quả ghi NỐI vào docs/RELEASE_DIFF_RESULTS.jsonl sau MỖI lô.
# Dùng: tools/release_cmp.sh <thư mục gốc> [lô...]   (mặc định: mọi lô)
cd "$1" || exit 1
shift
export TEMP='D:\Grit\target\tmp' TMP='D:\Grit\target\tmp' TMPDIR=/d/Grit/target/tmp
export GRIT_LLVM_DIR="${GRIT_LLVM_DIR:-D:/Tools/llvm-20.1.8}"; export PATH="$(cygpath -u "$GRIT_LLVM_DIR")/bin:$PATH"
export GRIT_RT_LIB="${GRIT_RT_LIB:-D:/Grit/runtime/gt-rt/target/release/gt_rt.lib}"
export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_BUILD_JOBS=2
export GT_PROFILE="${GT_PROFILE:-release-o2}"
OUT=D:/Grit/docs/RELEASE_O2_DIFF_RESULTS.jsonl
LOG=/d/Grit/target/tmp/release_o2_cmp.log
mkdir -p /d/Grit/target/tmp
# tên lô | biến môi trường | tệp test
declare -a BATCHES=()
add() { BATCHES+=("$1|$2|$3"); }
for s in 130000 130100 130200 130300 130400; do add "gtv-$s" "GT_GTV_START=$s GT_GTV_COUNT=100 GT_GTV_OUT=$OUT" llvm_diff_gtv; done
add gtv-term "GT_GTV_SET=term GT_GTV_START=131000 GT_GTV_COUNT=100 GT_GTV_OUT=$OUT" llvm_diff_gtv
add gtv-elem "GT_GTV_SET=elem GT_GTV_START=132000 GT_GTV_COUNT=150 GT_GTV_OUT=$OUT" llvm_diff_gtv
add gtv-text "GT_GTV_SET=text GT_GTV_START=133000 GT_GTV_COUNT=150 GT_GTV_OUT=$OUT" llvm_diff_gtv
add gtv-f64 "GT_GTV_SET=f64 GT_GTV_START=134000 GT_GTV_COUNT=150 GT_GTV_OUT=$OUT" llvm_diff_gtv
add diff-bit "GT_DIFF_START=135000 GT_DIFF_COUNT=150 GT_DIFF_OUT=$OUT" llvm_diff
add abort-index "GT_ABORT_KIND=index GT_ABORT_START=136000 GT_ABORT_COUNT=60 GT_ABORT_OUT=$OUT" llvm_diff_abort
add abort-divrem "GT_ABORT_KIND=divrem GT_ABORT_START=136100 GT_ABORT_COUNT=60 GT_ABORT_OUT=$OUT" llvm_diff_abort
add abort-pre "GT_ABORT_KIND=pre GT_ABORT_START=136200 GT_ABORT_COUNT=60 GT_ABORT_OUT=$OUT" llvm_diff_abort
add bits "GT_BITS_START=137000 GT_BITS_COUNT=150 GT_BITS_OUT=$OUT" llvm_diff_bits
WANT="$*"
for b in "${BATCHES[@]}"; do
  IFS='|' read -r name envs test <<< "$b"
  if [ -n "$WANT" ] && [[ " $WANT " != *" $name "* ]]; then continue; fi
  t0=$(date +%s)
  env $envs cargo test -p gt-driver --test "$test" -- --ignored batch --test-threads=1 --nocapture > "/d/Grit/target/tmp/rco2_$name.txt" 2>&1
  rc=$?
  t1=$(date +%s)
  echo "BATCH $name rc=$rc secs=$((t1-t0))" | tee -a "$LOG"
  echo "{\"batch\":\"$name\",\"profile\":\"$GT_PROFILE\",\"rc\":$rc,\"secs\":$((t1-t0))}" >> "$OUT"
done
echo "DONE" | tee -a "$LOG"
