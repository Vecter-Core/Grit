#!/bin/bash
# preflight: kiểm kho + máy TRƯỚC mỗi lô nặng (lệnh Giang 2026-10-17, R1a; sự cố .git/index/ref/tệp bị ghi NUL khi phiên bị ngắt).
#   bash tools/preflight.sh [thư_mục_dự_án]        thoát 0 = sẵn sàng; thoát 2 = KHÔNG sẵn sàng sau mọi lần thử (DỪNG theo quy tắc c(3))
# Kiểm: (1) .git/index, HEAD, refs/**, packed-refs không có byte NUL và không rỗng bất thường; (2) `git fsck --full` không lỗi; (3) D: còn >= PREFLIGHT_MIN_D_GB (10) và
# C: còn >= PREFLIGHT_MIN_C_GB (3); (4) bộ nhớ ảo khả dụng >= PREFLIGHT_MIN_VM_GB (4); (5) không có tiến trình cargo/rustc/link khác; (6) `git status` chạy được.
# Thiếu thì ngủ PREFLIGHT_SLEEP (60) giây rồi thử lại tối đa PREFLIGHT_TRIES (3) lần. Biến môi trường dùng để KIỂM THỬ: PREFLIGHT_SKIP_RESOURCES=1 bỏ (3)(4)(5).
dir="${1:-.}"
tries="${PREFLIGHT_TRIES:-3}"
nap="${PREFLIGHT_SLEEP:-60}"
min_d="${PREFLIGHT_MIN_D_GB:-10}"
min_c="${PREFLIGHT_MIN_C_GB:-3}"
min_vm="${PREFLIGHT_MIN_VM_GB:-4}"

has_nul() { # tệp có byte NUL không (rỗng cũng coi là có vấn đề với các tệp bắt buộc có nội dung)
  [ -f "$1" ] || return 1
  [ "$(tr -cd '\000' < "$1" | wc -c)" -gt 0 ]
}

check_once() {
  local bad=0 g="$dir/.git"
  [ -d "$g" ] || { echo "PREFLIGHT: không có $g"; return 1; }
  # .git/index là tệp NHỊ PHÂN (có NUL hợp lệ): kiểm chữ ký "DIRC" ở 4 byte đầu và kích thước, không kiểm NUL; HEAD là văn bản: không được có NUL
  if [ ! -s "$g/index" ]; then echo "PREFLIGHT: $g/index rỗng hoặc thiếu"; bad=1
  elif [ "$(head -c4 "$g/index")" != "DIRC" ]; then echo "PREFLIGHT: $g/index sai chữ ký (chứa byte NUL/hỏng)"; bad=1; fi
  if [ ! -s "$g/HEAD" ]; then echo "PREFLIGHT: $g/HEAD rỗng hoặc thiếu"; bad=1; fi
  if has_nul "$g/HEAD"; then echo "PREFLIGHT: $g/HEAD chứa byte NUL"; bad=1; fi
  if [ -f "$g/packed-refs" ] && has_nul "$g/packed-refs"; then echo "PREFLIGHT: packed-refs chứa byte NUL"; bad=1; fi
  if [ -d "$g/refs" ]; then
    while IFS= read -r f; do
      if [ ! -s "$f" ]; then echo "PREFLIGHT: ref rỗng: $f"; bad=1; fi
      if has_nul "$f"; then echo "PREFLIGHT: ref chứa NUL: $f"; bad=1; fi
    done < <(find "$g/refs" -type f 2>/dev/null)
  fi
  [ $bad -eq 0 ] || return 1
  if ! ( cd "$dir" && git fsck --full >/dev/null 2>"${TMPDIR:-/tmp}/preflight_fsck.$$" ); then
    echo "PREFLIGHT: git fsck --full lỗi:"; head -5 "${TMPDIR:-/tmp}/preflight_fsck.$$"; rm -f "${TMPDIR:-/tmp}/preflight_fsck.$$"; return 1
  fi
  rm -f "${TMPDIR:-/tmp}/preflight_fsck.$$"
  if ! ( cd "$dir" && git status --short >/dev/null 2>&1 ); then echo "PREFLIGHT: git status không chạy được"; return 1; fi
  if [ "${PREFLIGHT_SKIP_RESOURCES:-0}" != "1" ]; then
    local d_gb c_gb vm_kb procs
    d_gb=$(df -Pk /d 2>/dev/null | awk 'NR==2{printf "%d", $4/1048576}')
    c_gb=$(df -Pk /c 2>/dev/null | awk 'NR==2{printf "%d", $4/1048576}')
    [ "${d_gb:-0}" -ge "$min_d" ] || { echo "PREFLIGHT: D: còn ${d_gb:-?} GB < $min_d"; return 1; }
    [ "${c_gb:-0}" -ge "$min_c" ] || { echo "PREFLIGHT: C: còn ${c_gb:-?} GB < $min_c"; return 1; }
    vm_kb=$(powershell -NoProfile -Command "(Get-CimInstance Win32_OperatingSystem).FreeVirtualMemory" 2>/dev/null | tr -d '\r\n ')
    if [ -z "$vm_kb" ] || [ "$((vm_kb / 1048576))" -lt "$min_vm" ]; then echo "PREFLIGHT: bộ nhớ ảo khả dụng ${vm_kb:-?} KB < $min_vm GB"; return 1; fi
    procs=$(powershell -NoProfile -Command "@(Get-Process cargo,rustc,link,cl -ErrorAction SilentlyContinue).Count" 2>/dev/null | tr -d '\r\n ')
    if [ "${procs:-0}" != "0" ]; then echo "PREFLIGHT: còn $procs tiến trình cargo/rustc/link/cl đang chạy"; return 1; fi
  fi
  return 0
}

i=1
while [ "$i" -le "$tries" ]; do
  if check_once; then echo "PREFLIGHT_OK"; exit 0; fi
  echo "PREFLIGHT: lần $i/$tries chưa đạt"
  [ "$i" -lt "$tries" ] && sleep "$nap"
  i=$((i + 1))
done
echo "PREFLIGHT_FAIL: dừng theo quy tắc (c)(3)"
exit 2
