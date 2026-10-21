# CAPSTONE — bốn chương trình thật cho Seed 1.0 (cố định 2026-10-21)

Mục đích: bằng chứng cho tầm nhìn "AI viết Grit": một subagent MỚI, chỉ đọc `docs/AI_GUIDE.md` + `docs/STD_REFERENCE.md` + đề bài dưới đây + chạy `gritc`/`rig`, viết được bốn chương trình cho **đầu ra đúng** với bản tham chiếu Rust độc lập, ở `dev` và `release-o2`. Đề bài KHÔNG được nới sau khi đã giao; thay đổi đặc tả = ADR + làm lại cả bốn.

- **Bản tham chiếu Rust**: `capstone/reference/` (Cargo riêng, không phụ thuộc, dựng bằng `cargo build --release` trong thư mục đó) — là NGUỒN SỰ THẬT của hành vi; đề bài dưới đây là bản dịch của nó.
- **Bộ tệp đầu vào**: `capstone/gen_cases.py <thư-mục>` sinh tất định ~87 ca (+ ca lớn với `--large`): rỗng, biên, lỗi, ngẫu nhiên, lớn. Mỗi ca: `args` (mỗi dòng một đối số; `@DIR@` = thư mục của ca), `stdin` (tuỳ chọn), tệp dữ liệu.
- **Bộ so sánh**: `capstone/check.py --k <chương-trình> --cand <tệp-thực-thi>` chạy ứng viên và tham chiếu trong hai bản sao thư mục ca, so **mã thoát, stdout, stderr (nguyên văn) và mọi tệp ghi ra**.
- Mỗi chương trình là một gói Rig (`rig new`) dựa vào gói `std`; tên tệp thực thi: `csvstat`, `calc`, `kvstore`, `grep-lite`.
- Quy ước chung: byte (không phải ký tự) là đơn vị; dòng tách bằng `\n` (dòng cuối không có `\n` vẫn là dòng; `\r` là một byte thường); đầu ra ghi bằng byte đúng như đặc tả; lỗi dùng đối số in `usage:` ra stderr, thoát 2.

## K1 — `csvstat [FILE|-]`
Đọc CSV từ `FILE` (hoặc stdin nếu không có đối số hoặc đối số là `-`). Dòng đầu là tiêu đề (tách bằng `,`, không có trích dẫn). Mỗi dòng dữ liệu không rỗng phải có đúng số trường của tiêu đề. Dòng rỗng bị bỏ qua (vẫn được tính vào số dòng).
- Một cột là **số** nếu MỌI trường của nó là số nguyên i64 hợp lệ: tuỳ chọn một `-`, rồi 1–19 chữ số, giá trị vừa i64 (không có `+`, không khoảng trắng, trường rỗng KHÔNG hợp lệ); một trường không hợp lệ làm cột đó **không phải số** (và ngừng cộng dồn cột đó).
- Đầu ra, mỗi cột một dòng theo thứ tự cột: `<tên>: non-numeric` | `<tên>: count=0` (không có dòng dữ liệu) | `<tên>: count=<n> sum=<tổng> min=<min> max=<max> mean=<q>.<dddd>`. `mean` = tổng/n, **cắt về 0**, đúng 4 chữ số thập phân; số âm có dấu `-` kể cả khi phần nguyên là 0 (`-0.5000`).
- Không có dòng nào (stdin rỗng): không in gì, thoát 0.
- Lỗi dữ liệu → stderr, thoát 1, dừng ở lỗi đầu tiên (số dòng 1-based, tính cả tiêu đề): `csvstat: line N: expected K fields, found J` hoặc `csvstat: line N: overflow` (tổng i64 của một cột còn là-số bị tràn tại dòng N).
- Không đọc được tệp: stderr `csvstat: cannot read FILE`, thoát 2. Hơn một đối số: `usage: csvstat [FILE|-]`, thoát 2.

## K2 — `calc`
Đọc stdin; mỗi dòng không rỗng (dòng chỉ gồm dấu cách/tab bị bỏ qua) là một biểu thức số nguyên i64. Ngữ pháp: `expr := term (('+'|'-') term)*`; `term := unary (('*'|'/') unary)*`; `unary := '-' unary | primary`; `primary := số | '(' expr ')'`. Dấu cách và tab được bỏ qua giữa các token. `/` cắt về 0. Số = chuỗi chữ số (không dấu).
- Mỗi dòng in một dòng: kết quả thập phân, hoặc `error: <thông báo> at <cột>` (cột 1-based theo byte). Thông báo (cố định): `unexpected end of input`, `unexpected character`, `expected ')'`, `trailing input`, `integer too large` (số > i64::MAX; cột = đầu số), `overflow` và `division by zero` (cột = toán tử; `overflow` cũng cho `-x` khi x = i64::MIN, cột = dấu `-`, và `MIN / -1`).
- Thoát 0 nếu mọi dòng đúng, 1 nếu có ít nhất một dòng lỗi. Mọi đầu ra ở stdout. `\r` là ký tự lạ (`unexpected character`).
- Chỉ dùng số học kiểm tràn (không wrap); độ sâu ngoặc phải xử lý được ít nhất 40 tầng.

## K3 — `kvstore`
Đọc lệnh từ stdin, mỗi dòng một lệnh (dòng rỗng bị bỏ qua). Lệnh và đối số tách bằng MỘT dấu cách đầu tiên. Khoá không rỗng, không chứa `=`, dấu cách, tab; giá trị là mọi byte còn lại của dòng (có thể rỗng, có thể chứa dấu cách).
- `set KEY VALUE` → `ok` (hoặc `error: bad key`); `get KEY` → giá trị hoặc `(nil)`; `del KEY` → `1` (có) hoặc `0`; `count` → số khoá; `dump` → mọi cặp `KEY=VALUE`, sắp xếp theo khoá (so byte), mỗi cặp một dòng.
- `save FILE` ghi `KEY=VALUE\n` sắp xếp theo khoá → `ok` hoặc `error: cannot write`; `load FILE` đọc từng dòng `KEY=VALUE` (tách ở `=` ĐẦU TIÊN; khoá hợp lệ) và `set` từng cặp (đè lên khoá cũ) → `loaded N`; dòng sai → `error: bad line N` (N 1-based) và KHÔNG áp dụng cặp nào; không đọc được → `error: cannot read`.
- `error: usage` khi thiếu/thừa đối số của lệnh đã biết (`set`/`get`/`del`/`save`/`load` cần đối số; `count`/`dump` không được có); `error: unknown command` cho lệnh lạ (phân biệt hoa/thường). Thoát 0.
- Lưu ý thiết kế: cần cấu trúc băm (hoặc tương đương) có xoá; chương trình nhận hàng chục nghìn lệnh.

## K4 — `grep-lite [-c] [-n] PATTERN FILE...`
Tìm chuỗi con (byte) `PATTERN` trong các tệp. Cờ `-c`, `-n` chỉ được nhận TRƯỚC `PATTERN`; `--` kết thúc cờ; từ đối số đầu tiên không phải cờ trở đi mọi thứ là tham số vị trí (nên `-n` sau PATTERN là tên tệp). Cần ≥ 1 tệp, nếu không `usage: grep-lite [-c] [-n] PATTERN FILE...`, thoát 2. PATTERN rỗng khớp mọi dòng.
- Mặc định mỗi dòng khớp in `FILE:text`; với `-n`: `FILE:LINENO:text`; với `-c`: mỗi tệp một dòng `FILE:COUNT` và KHÔNG in dòng khớp (`-c` thắng `-n`). `FILE` in đúng như được truyền.
- Tệp không mở được: stderr `grep-lite: FILE: cannot open`, tiếp tục tệp sau. Thoát: 2 nếu có tệp lỗi, ngược lại 0 nếu có dòng khớp, 1 nếu không khớp dòng nào.

## BẢNG KHOẢNG TRỐNG (lập từ yêu cầu K1–K4 + các thăm dò thật, 2026-10-21; trạng thái được cập nhật ở `docs/RUN_REPORTS.md` sau mỗi bước)
"Chặn" theo thứ tự làm. Quyết định mặc định theo lệnh Seed 1.0: làm trong `lib/std` bằng Grit; intrinsic mới ≤ 6.

| # | Khoảng trống | Chặn | Cách làm | Trạng thái |
|---|---|---|---|---|
| G1 | `gritc run/check <tệp>`: tên tệp là tên module (`01_add.gt` bị từ chối) | mọi chương trình | tệp được chỉ định là module gốc bất kể tên | CHƯA |
| G2 | `Vec` giữ `&!Arena` ĐỘC QUYỀN ⇒ mỗi Vec/Text cần Arena riêng; không thể có bảng băm/nhiều Vec trên một Arena | K3 (HashMap), K1/K4 (Vec nhiều) | R6: Arena chia sẻ `&'a Arena` cho `Vec`/`Text` (ADR-038 Q83) | CHƯA |
| G3 | `Vec` chỉ có `new/push/reserve/len/as_slice/as_mut_slice`: thiếu `pop/clear/truncate/insert/remove/get/first/last/extend/contains` | K1–K4 | intrinsic `Vec::truncate` (1 cái), còn lại bằng Grit | CHƯA |
| G4 | `str`/`Text`: không có byte của `&str`, `find/split/trim/starts_with/ends_with/parse/join`, số → chữ | K1–K4 | intrinsic `str::as_bytes` (+ kiểm UTF-8 khi tạo `&str` từ byte), còn lại bằng Grit | CHƯA |
| G5 | stdin: không có `Cap<In>` | K1 (stdin), K2, K3 | intrinsic `Cap<In>::read_all` | CHƯA |
| G6 | ghi tệp: `Cap<Fs>` chỉ có `read/open/sub` | K3 (`save`) | intrinsic `Cap<Fs>::write` | CHƯA |
| G7 | I/O lỗi bằng `Result`: có `Cap<Fs>::open → Result<File, IoError>`; `read` vẫn abort | K1, K3, K4 | dùng `open`; `read_all`/`write` trả `Result` | MỘT PHẦN (đã có `open`) |
| G8 | phân rã tuple `(a, b) ≔ f();` (E0101) | tiện lợi (K1, K2) | desugar ở parser (ADR-C03) | CHƯA |
| G9 | toán tử `?` | tiện lợi (K1, K3) | desugar ở parser, chỉ `Result` (ADR-C04) | CHƯA |
| G10 | `HashMap` | K3 | `lib/std` bằng Grit trên Arena chia sẻ + trait `Hash`/`Eq` | CHƯA |
| G11 | in số có dấu, thập phân cố định, hex; phân tích `u64/i64` (trả `Result`) | K1, K2, K3 | `lib/std` bằng Grit | CHƯA |
| G12 | `min/max/abs/pow` kiểm tràn, cộng/nhân/trừ kiểm tràn trả `Option` | K1, K2 | `lib/std` bằng Grit (so sánh, không dùng 128 bit) | CHƯA |
| G13 | sắp xếp (khoá `dump`) | K3 | `lib/std`: sắp xếp trộn ổn định trên `Vec` (đã có mẫu trong `examples/sortlines`) | CHƯA |
| G14 | `rig new` không tự thêm `std`; sửa mã phải `rig lock` | tiện dụng AI | `rig new` thêm `std`; thông báo nhắc `rig lock` | CHƯA |
| G15 | stderr: `Cap<ErrOut>` chỉ `write_u64/write_str` | K1/K4 báo lỗi | đủ (thông báo là hằng + số + tên tệp là `&str` từ `arg`) | ĐỦ |
| G16 | tràn ngăn xếp/đệ quy sâu (K2 ngoặc 40 tầng) | K2 | `main` đã chạy trên ngăn xếp 256 MiB (F-06) | ĐÃ SỬA |

Không làm (theo lệnh): closure, macro, `dyn`, async, `unsafe` cho người dùng, `f32`, đối số lifetime lồng, đổi ngữ nghĩa mượn. Cái không làm → `docs/KNOWN_LIMITATIONS.md` / `docs/SEED_LIMITATIONS.md` kèm cách né.
