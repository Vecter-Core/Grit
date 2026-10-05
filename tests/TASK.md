# Việc cần làm: quy ước & script kiểm kỳ vọng mẫu (đơn giản — giao cho AI khác)
1. `tests/README.md`: mô tả header `// expect: pass | fail Exxxx | warn Sxxx`; tên file fail = `<mã>_<mô tả>.gt`.
2. `tools/check_expectations.py`: duyệt `tests/{pass,fail,warn}`, kiểm (a) mỗi file có đúng một dòng expect,
   (b) tên file fail khớp mã trong expect, (c) mọi mã expect có trong `docs/spec/error_codes.md`. Thoát khác 0 nếu sai.
Chưa cần parse `.gt` (parser ở M2).
