#!/bin/bash
# chạy cargo test với retry khi LNK1104/LNK1108/hết bộ nhớ: t.sh <args cargo test>
# TEMP/TMP chuyển sang D: (ổ C: có thể đầy; link.exe ghi tệp tạm vào TEMP)
export TEMP='D:\Grit\target\tmp' TMP='D:\Grit\target\tmp' TMPDIR=/d/Grit/target/tmp
export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_BUILD_JOBS=1
for i in 1 2 3 4; do
  out=$(cargo test "$@" 2>&1)
  if echo "$out" | grep -qE 'LNK1104|LNK1108|1455|metadata stub'; then sleep 10; continue; fi
  break
done
echo "$out"
