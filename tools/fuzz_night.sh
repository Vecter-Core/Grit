#!/usr/bin/env bash
# GÓI 14 E2 — fuzz qua đêm bằng MỘT lệnh (Giang gõ trong Git Bash tại D:/Grit; cần WSL Ubuntu-24.04 với nightly ghim + cargo-fuzz, như GÓI 12).
#   bash tools/fuzz_night.sh start [giờ=26] [--sequential]   # CHẠY NỀN-TRƯỚC: để cửa sổ mở suốt đêm (WSL tắt khi không còn phiên nào); Ctrl+C an toàn
#   bash tools/fuzz_night.sh status                          # một màn hình: giờ có/thiếu mỗi mục tiêu trên băm hiện tại, phiên cuối, ước tính, 5 dòng cuối của nhật ký
#   bash tools/fuzz_night.sh import                          # nhập log WSL về kho chính (chống trùng; từ chối nguồn lạ/băm lệch), rồi in lệnh commit
# `start` đồng bộ clone ~/grit_fuzz với nhánh `integration` của kho chính (git fetch /mnt/d/Grit), rồi chạy tools/fuzz_loop.sh (mặc định --parallel; lexer và parser
# mỗi cái một đồng hồ riêng; RAM không đủ ⇒ tự lùi về lần lượt). Bị ngắt (Ctrl+C, WSL tắt, khởi động lại): gõ lại ĐÚNG lệnh `start` — tổng tích luỹ nằm trong
# ~/grit_fuzz/docs/FUZZ_LOG.jsonl (đồng bộ lại clone GIỮ các dòng log chỉ có ở clone: ghi qua ~/.fuzz_log.keep rồi nối dòng chưa có — lỗi tìm thấy khi kiểm tay GÓI 14: `git checkout` từ chối vì log cục bộ khác HEAD); phiên đang dở bị mất (tối đa FUZZ_SESSION_SEC, mặc định 600 s) và KHÔNG ghi dòng nào ⇒ không trùng.
# Biến: FUZZ_DISTRO (Ubuntu-24.04), FUZZ_CLONE (~/grit_fuzz), FUZZ_TOOLCHAIN (nightly-2026-10-04), FUZZ_PAR_MIN_GB (6), FUZZ_SESSION_SEC (600), FUZZ_NIGHT_DRY=1 (chỉ in lệnh WSL).
set -u
cmd="${1:-}"
DISTRO="${FUZZ_DISTRO:-Ubuntu-24.04}"
CLONE="${FUZZ_CLONE:-\$HOME/grit_fuzz}"
TC="${FUZZ_TOOLCHAIN:-nightly-2026-10-04}"
PARMIN="${FUZZ_PAR_MIN_GB:-6}"
SESSEC="${FUZZ_SESSION_SEC:-600}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
wsl_run() { # $1 = chuỗi lệnh bash chạy trong WSL (shell đăng nhập để có cargo)
  if [ "${FUZZ_NIGHT_DRY:-0}" = 1 ]; then echo "WSL[$DISTRO]: $1"; return 0; fi
  wsl.exe -d "$DISTRO" -- bash -lc "$1"
}
case "$cmd" in
  start)
    hours="${2:-26}"; mode="--parallel"
    for a in "$@"; do [ "$a" = "--sequential" ] && mode=""; done
    case "$hours" in --*) hours=26;; esac
    wsl_run "set -e; [ -d $CLONE/.git ] || git clone -q /mnt/d/Grit $CLONE; cd $CLONE; git config core.autocrlf false; git fetch -q /mnt/d/Grit integration; cp -f docs/FUZZ_LOG.jsonl \$HOME/.fuzz_log.keep 2>/dev/null; git checkout -q -f FETCH_HEAD; if [ -f \$HOME/.fuzz_log.keep ]; then awk 'NR==FNR{s[\$0]=1;next} !(\$0 in s)' docs/FUZZ_LOG.jsonl \$HOME/.fuzz_log.keep >> docs/FUZZ_LOG.jsonl; fi; echo \"fuzz_night: clone \$(git rev-parse --show-toplevel) tại \$(git rev-parse --short HEAD)\"; FUZZ_TOOLCHAIN=$TC FUZZ_PAR_MIN_GB=$PARMIN FUZZ_SESSION_SEC=$SESSEC bash tools/fuzz_loop.sh $hours $mode 2>&1 | tee -a \$HOME/fuzz.out"
    ;;
  status)
    wsl_run "cd $CLONE && python3 tools/fuzz_total.py --summary && echo '--- 5 dòng cuối ~/fuzz.out:' && tail -5 \$HOME/fuzz.out"
    ;;
  import)
    wsl_run "cp $CLONE/docs/FUZZ_LOG.jsonl /mnt/d/Grit_export/fuzz_clone_log.jsonl && echo 'đã chép log WSL ra D:\\Grit_export\\fuzz_clone_log.jsonl'" || exit 1
    [ "${FUZZ_NIGHT_DRY:-0}" = 1 ] && exit 0
    cd "$ROOT" || exit 2
    . tools/pyfind.sh; [ -n "$PY" ] || exit 2
    $PY tools/fuzz_import_local.py /d/Grit_export/fuzz_clone_log.jsonl && echo "tiếp: git add docs/FUZZ_LOG.jsonl && git commit -m 'docs: nhập log fuzz WSL' (kèm Author-Model nếu do agent làm)"
    ;;
  *)
    sed -n 2,6p "$0"; exit 2 ;;
esac
