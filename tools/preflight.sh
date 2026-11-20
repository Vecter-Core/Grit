#!/bin/bash
# Preflight trước mỗi lô nặng. Trên Windows giữ ngưỡng D:/C:/bộ nhớ ảo; Linux kiểm đĩa và RAM.
# Dùng: bash tools/preflight.sh [thư_mục_dự_án]. Thiếu điều kiện thì thử PREFLIGHT_TRIES lần, cách nhau PREFLIGHT_SLEEP giây.
# PREFLIGHT_SKIP_RESOURCES=1 chỉ bỏ kiểm tài nguyên/tiến trình (dành cho unit test của công cụ).
dir="${1:-.}"
tries="${PREFLIGHT_TRIES:-3}"
nap="${PREFLIGHT_SLEEP:-60}"
min_d="${PREFLIGHT_MIN_D_GB:-10}"
min_c="${PREFLIGHT_MIN_C_GB:-3}"
min_vm="${PREFLIGHT_MIN_VM_GB:-4}"
if command -v python3 >/dev/null 2>&1; then PY=python3; else PY=python; fi
export PYTHONIOENCODING=utf-8
has_nul() {
  [ -f "$1" ] || return 1
  [ "$(tr -cd '\000' < "$1" | wc -c)" -gt 0 ]
}
# GÓI 11 S0.a: phiên bản rustc phải KHỚP dòng `rustc=` của tools/TOOLCHAIN (rustc 1.100 làm clippy -D warnings đỏ ở cổng WSL đầu tiên).
# Không có tools/TOOLCHAIN trong thư mục dự án ⇒ bỏ qua (kho tạm của test). PREFLIGHT_RUSTC_VERSION chỉ để KIỂM THỬ (giả lập đầu ra `rustc --version`).
check_toolchain() {
  local f="$dir/tools/TOOLCHAIN" want have
  [ -f "$f" ] || return 0
  want=$(awk -F= '$1=="rustc" {gsub(/[[:space:]]/,"",$2); print $2}' "$f")
  [ -n "$want" ] || { echo "PREFLIGHT: tools/TOOLCHAIN không có dòng rustc="; return 1; }
  if [ -n "${PREFLIGHT_RUSTC_VERSION:-}" ]; then have="$PREFLIGHT_RUSTC_VERSION"
  else have=$( (cd "$dir" && rustc --version 2>/dev/null) | awk '{print $2}'); fi
  [ -n "$have" ] || { echo "PREFLIGHT: không chạy được rustc (cài rustup rồi: rustup toolchain install $want --profile minimal -c rustfmt -c clippy)"; return 1; }
  if [ "$have" != "$want" ]; then
    echo "PREFLIGHT: rustc $have KHÁC bản ghim $want (tools/TOOLCHAIN). Sửa: rustup toolchain install $want --profile minimal -c rustfmt -c clippy (rust-toolchain.toml sẽ tự chọn nó trong kho), hoặc đặt RUSTUP_TOOLCHAIN=$want"
    return 1
  fi
  return 0
}
check_once() {
  local bad=0 g="$dir/.git"
  check_toolchain || return 1
  [ -d "$g" ] || { echo "PREFLIGHT: không có $g"; return 1; }
  # index là nhị phân nên kiểm chữ ký DIRC; các tệp cấu trúc văn bản phải không rỗng và không có NUL.
  if [ ! -s "$g/index" ]; then echo "PREFLIGHT: $g/index rỗng hoặc thiếu"; bad=1
  elif [ "$(head -c4 "$g/index")" != "DIRC" ]; then echo "PREFLIGHT: $g/index sai chữ ký DIRC"; bad=1; fi
  for f in "$g/HEAD" "$g/config"; do
    if [ ! -s "$f" ]; then echo "PREFLIGHT: $f rỗng hoặc thiếu"; bad=1; fi
    if has_nul "$f"; then echo "PREFLIGHT: $f chứa byte NUL"; bad=1; fi
  done
  if [ -f "$g/packed-refs" ] && has_nul "$g/packed-refs"; then echo "PREFLIGHT: packed-refs chứa byte NUL"; bad=1; fi
  if [ -d "$g/refs" ]; then
    while IFS= read -r f; do
      if [ ! -s "$f" ]; then echo "PREFLIGHT: ref rỗng: $f"; bad=1; fi
      if has_nul "$f"; then echo "PREFLIGHT: ref chứa NUL: $f"; bad=1; fi
    done < <(find "$g/refs" -type f 2>/dev/null)
  fi
  [ "$bad" -eq 0 ] || return 1
  local tmpbase="${TMPDIR:-/tmp}/preflight_fsck.$$"
  if ! ( cd "$dir" && git fsck --full --no-progress >"$tmpbase" 2>&1 ); then
    echo "PREFLIGHT: git fsck --full lỗi:"; head -5 "$tmpbase"; rm -f "$tmpbase"; return 1
  fi
  rm -f "$tmpbase"
  if ! ( cd "$dir" && git status --short >/dev/null 2>&1 ); then echo "PREFLIGHT: git status không chạy được"; return 1; fi
  if [ -f "$dir/tools/nul_scan.py" ]; then
    local nulout="${TMPDIR:-/tmp}/preflight_nul.$$"
    if ! (cd "$dir" && $PY tools/nul_scan.py .) >"$nulout" 2>&1; then
      echo "PREFLIGHT: NUL scan lỗi:"; tail -5 "$nulout"; rm -f "$nulout"; return 1
    fi
    rm -f "$nulout"
  fi
  if [ "${PREFLIGHT_SKIP_RESOURCES:-0}" != "1" ]; then
    case "$(uname -s)" in
      MINGW*|MSYS*|CYGWIN*)
        local d_gb c_gb vm_kb procs
        d_gb=$(df -Pk /d 2>/dev/null | awk 'NR==2{printf "%d", $4/1048576}')
        c_gb=$(df -Pk /c 2>/dev/null | awk 'NR==2{printf "%d", $4/1048576}')
        [ "${d_gb:-0}" -ge "$min_d" ] || { echo "PREFLIGHT: D: còn ${d_gb:-?} GB < $min_d"; return 1; }
        [ "${c_gb:-0}" -ge "$min_c" ] || { echo "PREFLIGHT: C: còn ${c_gb:-?} GB < $min_c"; return 1; }
        vm_kb=$(powershell.exe -NoProfile -Command "(Get-CimInstance Win32_OperatingSystem).FreeVirtualMemory" 2>/dev/null | tr -d '\r\n ')
        if [ -z "$vm_kb" ] || [ "$((vm_kb / 1048576))" -lt "$min_vm" ]; then echo "PREFLIGHT: bộ nhớ ảo khả dụng ${vm_kb:-?} KB < $min_vm GB"; return 1; fi
        procs=$(powershell.exe -NoProfile -Command "@(Get-Process cargo,rustc,link,cl -ErrorAction SilentlyContinue).Count" 2>/dev/null | tr -d '\r\n ')
        [ "${procs:-0}" = "0" ] || { echo "PREFLIGHT: còn $procs tiến trình cargo/rustc/link/cl đang chạy"; return 1; }
        ;;
      *)
        local avail_kb mem_kb others
        avail_kb=$(df -Pk "$dir" 2>/dev/null | awk 'NR==2 {print $4}')
        [ -n "$avail_kb" ] && [ "$avail_kb" -ge "${PREFLIGHT_MIN_DISK_KB:-5242880}" ] || { echo "PREFLIGHT: ổ chứa kho còn $(( ${avail_kb:-0}/1024 )) MB < 5120 MB"; return 1; }
        mem_kb=$(awk '/^MemAvailable:/ {print $2}' /proc/meminfo 2>/dev/null)
        [ -n "$mem_kb" ] && [ "$mem_kb" -ge 2097152 ] || { echo "PREFLIGHT: RAM khả dụng ${mem_kb:-?} KB < 2097152 KB"; return 1; }
        others=$(ps -eo pid=,comm= 2>/dev/null | awk '$2=="cargo" || $2=="rustc" || $2=="link" || $2=="cl" {print $1}' | tr '\n' ' ')
        [ -z "$others" ] || { echo "PREFLIGHT: tiến trình build đang chạy (pid: $others)"; return 1; }
        ;;
    esac
  fi
  return 0
}
i=1
while [ "$i" -le "$tries" ]; do
  if check_once; then echo "PREFLIGHT_OK"; echo "PREFLIGHT=OK"; exit 0; fi
  echo "PREFLIGHT: lần $i/$tries chưa đạt"
  [ "$i" -lt "$tries" ] && sleep "$nap"
  i=$((i + 1))
done
echo "PREFLIGHT_FAIL: dừng theo quy tắc (3 lần thử)"
echo "PREFLIGHT=FAIL"
exit 2
