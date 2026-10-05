# Corpus mẫu `.gt` — "đặc tả sống" (M0)

Mỗi chương trình mẫu là một file `.gt` nằm đúng thư mục theo kết quả kỳ vọng. Dòng **đầu tiên** của file
bắt buộc là dòng kỳ vọng (đúng một dòng `// expect:` trong cả file):

| Thư mục | Dòng đầu | Ý nghĩa | Tên file |
|---|---|---|---|
| `tests/pass/` | `// expect: pass` | qua toàn bộ pass (có thể kèm GHI CHÚ N…, ghi trong dòng `// note:`) | `NN_mô_tả.gt` |
| `tests/fail/` | `// expect: fail Exxxx` | phải bị chặn **đúng mã đó** (không chỉ "có lỗi") | `Exxxx_mô_tả.gt` |
| `tests/warn/` | `// expect: warn Sxxx` | VÀNG: vẫn chạy (exit 0), phải in cảnh báo đúng mã | `Sxxx_mô_tả.gt` |
| `tests/lexer/` | `// expect: lexer` | chuỗi token kỳ vọng nằm ở file `<tên>.tokens` cùng thư mục | `mô_tả.gt` + `.tokens` |

Dòng `// note:` (tuỳ chọn) giải thích ý đồ của mẫu. File `.tokens`: mỗi dòng một token (`KIND` hoặc
`KIND(giá trị)`), dòng bắt đầu bằng `#` là comment.

Thư mục `golden/`, `fuzz/`, `bench/` dành cho milestone sau (plan §9) — hiện chỉ có `TASK.md` ghi chú.

## Quy tắc
- Mọi mã dùng trong `expect` phải có trong `docs/spec/error_codes.md`; mỗi file `fail/` và `warn/` phải được
  liệt kê ở cột "Test mẫu" của mã tương ứng (không có mẫu mồ côi).
- Mẫu **không** được chứa ký tự bidi / zero-width / non-NFC, trừ bốn mẫu cố ý độc hại `E1201`–`E1204`
  (và các mẫu đó **phải** thực sự chứa ký tự mục tiêu). Đây là phòng "Trojan Source" ngay trong corpus.
- Mẫu mới cho một mã lỗi chỉ được thêm cùng lúc với dòng tương ứng trong `error_codes.md`.
- Khi phát hiện một chương trình sai lọt qua gritc (false negative) — thêm ngay vào `tests/fail/` vĩnh viễn
  (CLAUDE.md §3.3).

## Kiểm tra
```
python tools/check_expectations.py    # exit 0 = corpus nhất quán, 1 = có lỗi
```
Script **chỉ** kiểm tính nhất quán của corpus (header, tên file, mã lỗi, Unicode, `.tokens`). Nó **chưa** chạy
`gritc` — runner đối chiếu phán quyết thật với `expect` sẽ được viết cùng driver (M2/M9).
