#!/bin/bash
# nul_check: MỌI tệp văn bản được theo dõi không được chứa byte NUL (lệnh Giang 2026-10-17, R1b; sự cố phiên bị ngắt ghi NUL vào tệp nguồn).
#   bash tools/nul_check.sh [thư_mục_dự_án]     thoát 0 = NUL_OK <số tệp quét>; thoát 1 = NUL_FOUND + tên tệp
# Đuôi quét: .rs .gt .md .toml .py .sh .json .jsonl .txt .yml .lock (cả tệp rỗng 0 byte không bị coi là lỗi ở đây; chỉ NUL).
cd "${1:-.}" || exit 2
exts='rs gt md toml py sh json jsonl txt yml lock'
pat=()
for e in $exts; do pat+=("*.$e"); done
# NGOẠI LỆ có chủ ý: đầu vào fuzz (`fuzz_regressions/`) CỐ Ý chứa NUL để tái hiện crash của lexer/parser (đã kiểm: crash_23817b64.gt có NUL từ commit gốc 6b12823)
ex=(':(exclude)**/fuzz_regressions/**')
total=$(git ls-files -z -- "${pat[@]}" "${ex[@]}" | tr -cd '\000' | wc -c)
found=$(git ls-files -z -- "${pat[@]}" "${ex[@]}" | xargs -0 -r grep -lPa '\x00' 2>/dev/null)
if [ -n "$found" ]; then
  echo "NUL_FOUND: tệp văn bản chứa byte NUL:"
  echo "$found" | sed 's/^/  /'
  exit 1
fi
echo "NUL_OK ($total tệp quét)"
exit 0
