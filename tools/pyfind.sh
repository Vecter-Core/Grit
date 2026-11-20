#!/bin/bash
# pyfind: chọn trình thông dịch Python dùng chung (GÓI 11 S0.b). Dùng: `. tools/pyfind.sh` rồi gọi `$PY ...`.
# Thứ tự: biến PYTHON (nếu chạy được) → `python` → `python3`. Trên Ubuntu không có gói python-is-python3 chỉ có `python3`; trên Windows `python3` có thể là
# lối tắt của Microsoft Store nên thử `python` TRƯỚC. Không tìm thấy ⇒ PY rỗng và in lỗi rõ cách cài.
_py_ok() { [ -n "$1" ] && "$1" -c "import sys; sys.exit(0 if sys.version_info[0] == 3 else 1)" >/dev/null 2>&1; }
PY=""
for _c in "${PYTHON:-}" python python3; do
  if _py_ok "$_c"; then PY="$_c"; break; fi
done
if [ -z "$PY" ]; then
  echo "pyfind: không tìm thấy Python 3 (thử: \$PYTHON, python, python3). Ubuntu: sudo apt install python3 (và python-is-python3 nếu muốn lệnh python)" >&2
fi
export PY
