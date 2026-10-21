#!/bin/bash
# Preflight trước MỖI lô nặng (R1). Dùng: bash tools/preflight.sh [thư-mục-dự-án]
# Kiểm: (1) .git không có byte NUL (sự cố 2026-10-16) và `git fsck` sạch; (2) ổ đĩa trống >= 5 GB; (3) RAM trống >= 2 GB;
# (4) không có cargo/rustc nào khác đang chạy (luật: không chạy hai cargo cùng lúc); (5) không có tệp văn bản đã theo dõi chứa NUL.
# Thoát 0 = đủ điều kiện; 1 = KHÔNG (in lý do `PREFLIGHT_FAIL: ...`). Hết dòng cuối là `PREFLIGHT=OK|FAIL`.
# Linux và Windows (git-bash): đọc /proc/meminfo nếu có, nếu không dùng PowerShell.
cd "${1:-.}" || { echo "PREFLIGHT_FAIL: không vào được thư mục"; echo "PREFLIGHT=FAIL"; exit 1; }
fail=0
bad() { echo "PREFLIGHT_FAIL: $1"; fail=1; }
if command -v python3 >/dev/null 2>&1; then PY=python3; else PY=python; fi

# 1. .git: không NUL trong tệp văn bản cấu trúc (HEAD, config, refs, packed-refs), fsck sạch
if [ ! -d .git ]; then bad "không có .git"; else
  for f in .git/HEAD .git/config .git/packed-refs; do
    [ -f "$f" ] && [ "$(tr -d '\000' < "$f" | wc -c)" != "$(wc -c < "$f")" ] && bad "$f chứa byte NUL"
  done
  if [ -d .git/refs ]; then
    while IFS= read -r f; do
      [ "$(tr -d '\000' < "$f" | wc -c)" != "$(wc -c < "$f")" ] && bad "$f chứa byte NUL"
    done < <(find .git/refs -type f 2>/dev/null)
  fi
  git fsck --no-dangling --no-progress >/tmp/preflight_fsck.$$ 2>&1 || bad "git fsck lỗi: $(head -3 /tmp/preflight_fsck.$$ | tr '\n' ' ')"
  rm -f /tmp/preflight_fsck.$$
fi

# 2. ổ đĩa
avail_kb=$(df -Pk . 2>/dev/null | awk 'NR==2 {print $4}')
if [ -n "$avail_kb" ]; then [ "$avail_kb" -ge 5242880 ] || bad "ổ đĩa trống $((avail_kb/1024)) MB < 5120 MB"; else bad "không đo được ổ đĩa"; fi

# 3. RAM trống
if [ -r /proc/meminfo ]; then
  mem_kb=$(awk '/^MemAvailable:/ {print $2}' /proc/meminfo)
else
  mem_kb=$(powershell.exe -NoProfile -Command "(Get-CimInstance Win32_OperatingSystem).FreePhysicalMemory" 2>/dev/null | tr -d '\r')
fi
if [ -n "$mem_kb" ]; then [ "$mem_kb" -ge 2097152 ] || bad "RAM trống $((mem_kb/1024)) MB < 2048 MB"; else bad "không đo được RAM"; fi

# 4. cargo/rustc khác (loại trừ chính tiến trình này và cha của nó)
others=$(ps -eo pid,comm 2>/dev/null | awk '$2=="cargo" || $2=="rustc" {print $1}' | grep -v "^$$\$" | tr '\n' ' ')
[ -z "$others" ] || bad "đang có cargo/rustc chạy (pid: $others)"

# 5. NUL trong tệp văn bản đã theo dõi
if [ -f tools/nul_scan.py ]; then $PY tools/nul_scan.py . >/tmp/preflight_nul.$$ 2>&1 || bad "$(tail -2 /tmp/preflight_nul.$$ | tr '\n' ' ')"; rm -f /tmp/preflight_nul.$$; fi

if [ $fail = 0 ]; then echo "PREFLIGHT=OK"; exit 0; else echo "PREFLIGHT=FAIL"; exit 1; fi
