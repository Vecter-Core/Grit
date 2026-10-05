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

## Ca nhiều tệp (ADR-017b, M3)
Một ca = **một thư mục** `tests/{pass,fail,warn}/<ca>/` chứa **≥2 tệp `*.gt`** và một tệp tên `expect` có đúng một dòng
`expect: pass` | `expect: fail Exxxx` | `expect: warn Sxxx` (có thể thêm dòng `note: …`). Tên thư mục theo quy ước tên tệp đơn
(`Exxxx_mô_tả` cho fail, `NN_mô_tả` cho pass). **Tên tệp bỏ `.gt` là tên module** (phải là ident hợp lệ, duy nhất không phân biệt
hoa/thường); tệp trong thư mục KHÔNG có dòng `// expect:` (kỳ vọng thuộc về cả ca). Thứ tự tệp không có ý nghĩa.
Thư mục ca được liệt kê ở cột "Test mẫu" của `error_codes.md` như tệp đơn (không có đuôi `.gt`).

## Snapshot thông báo lỗi (plan §6.2)
`tests/snapshots/<tên>.snap` (lexer/parser: E0101, E0102, E12xx) và `tests/snapshots_resolve/<tên>.snap` (giải tên: E02xx) khoá từng byte
đầu ra người-đọc (không màu) và JSON của mọi chẩn đoán. Snapshot thiếu/khác → test ĐỎ; không bao giờ tự cập nhật ngầm.
Cập nhật có chủ đích: `GRIT_UPDATE_SNAPSHOTS=1 cargo test -p gt-parser --test snapshots` (hoặc `-p gt-resolve --test snapshots`), rồi đọc `git diff`.

## Tài liệu mã lỗi (plan §0.5)
Mỗi mã đã triển khai (danh sách `DOC_REQUIRED` trong `tools/check_expectations.py`; M3: E0101, E0102, E0201–E0205) phải có
`docs/errors/<mã>.md` với ≥3 dòng `- fail: <ref>` và ≥3 dòng `- pass: <ref>` (`ref` = đường dẫn tương đối `tests/`, không đuôi);
script kiểm tệp tồn tại và đúng kỳ vọng (`fail <mã>` / `pass`).

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
