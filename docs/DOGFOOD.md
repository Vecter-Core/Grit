# DOGFOOD — dùng Grit thật (ghi từ 2026-10-13)

Mỗi dòng = một khoảng trống/bug gặp khi viết công cụ thật. Cột "gốc": gritc | lib | codegen | runtime | spec. "Chặn" = chặn công cụ `wc` hay không. Bug thật sửa theo `docs/FIELD_BUGS.md`.

## Công cụ đầu tiên: `examples/wc` (dự án Rig, 1 gói, ~30 dòng Grit)
`wc.exe <đường-dẫn>` in `<dòng> <từ> <byte> <đường-dẫn>`. Từ = dãy byte không phải ` `, `\n`, `\t`, `\r`. Kiểm bằng `crates/rig/tests/dogfood.rs` trên 10 tệp (rỗng, 1 xuống dòng, không xuống dòng cuối, CRLF, chỉ khoảng trắng, đủ 256 byte, NUL/byte cao, văn bản UTF-8, 100 KB ngẫu nhiên, đường dẫn có dấu cách) + tệp 32 MiB + 2 đường lỗi. Tất cả khớp bản tham chiếu Rust. **Đo thật:** 32 MiB trong 0.44 s ≈ 72 MiB/s (LLVM -O0, mọi check giữ; gồm cả đọc tệp). Lỗi: không có tệp ⇒ `abort: io_error` mã 70; thiếu đối số ⇒ `abort: arg_out_of_range` mã 70; không in gì ra stdout.

## Khoảng trống THEO THỨ TỰ CHẶN (thứ tự gặp khi build tool)
| # | ngày | triệu chứng | gốc | chặn wc? | đã sửa? / test |
|---|---|---|---|---|---|
| 1 | 2026-10-13 | `E0201 root_out chưa được định nghĩa`: không có cách in ra | lib (prelude) | CÓ | Có: `▢ Out`, `Cap<Out>::write_u64/write_str`, `root_out()` (prelude + fixture gt-types khoá cùng nhau); test dogfood |
| 2 | 〃 | không có tham số dòng lệnh để biết tệp nào | lib | CÓ | Có: `Cap<Env>::arg(i) → &str` + runtime `gritrt_arg_*`; test "thiếu đối số" |
| 3 | 〃 | `File` chỉ có `len`, không đọc được byte | lib | CÓ | Có: `File::bytes(&self) → &[u8]` (đọc NGUYÊN tệp vào bộ nhớ, rò rỉ có chủ đích) |
| 4 | 〃 | `gritc: backend llvm chưa hỗ trợ: hàm không có thân MIR: $prelude::root_env` — MỌI hàm prelude của `Cap<…>`/`File` chưa hạ được | codegen | CÓ | Có: intrinsic `RootCap`, `EnvArg`, `FsRead`, `FileLen`, `FileBytes`, `OutU64`, `OutStr` (+ `gritrt_fs_read`, `gritrt_file_*`, `gritrt_write_*`) |
| 5 | 〃 | không có kênh báo lỗi I/O (`read` trả `File`, không `Result`) ⇒ mọi lỗi I/O = abort mã 70 + dòng `io_error: …` ở stderr | lib/spec | KHÔNG cho wc v0 | **ĐÃ SỬA 2026-10-13 (lệnh 7a):** thêm `Cap<Fs>::open → Result<File, IoError>` (`IoError {NotFound, Denied, Invalid, Other}`) + `core::exit(code) → !`; `read` (abort) giữ để không vỡ corpus; wc dùng `open`, lỗi ⇒ in lý do, thoát mã 2; test `wc_error_paths_are_controlled`. Còn thiếu: stderr riêng (chưa có capability), `?` |
| 6 | 〃 | (ĐÃ ĐÍNH CHÍNH 2026-10-13) báo cáo bước 5 viết sai rằng `root_*` gọi ở đâu cũng được. Đo thật: hàm thường gọi `root_fs()`/`root_out()` bị `E0310` (chỉ `main` cấp cao được gọi nguồn capability, D29). Phần còn lại đúng: capability gốc là hàm prelude (giả lập), chưa do runtime cấp cho `main` | lib/spec | KHÔNG | Đúng sẵn từ M4.5; thêm test hồi quy cho `root_out`: fail/E0310_root_out_in_helper, pass/E0310_ok_out_passed_down |
| 7 | 〃 | `N0602`: `b[i]` với `b ≔ f.bytes()` và vòng `i < b.len()` KHÔNG được chứng minh (giữ check lúc chạy); cùng mẫu với tham số `xs: &[T]` thì chứng minh được | gritc (bộ giải M7, quan hệ zone chưa gắn `b.len()` của biến cục bộ mượn từ lời gọi) | KHÔNG (check runtime đúng, chỉ chậm) | Chưa; ghi KNOWN_LIMITATIONS |
| 8 | 〃 | In số/chuỗi chưa có định dạng: chỉ `write_u64` và `write_str`; không có `String`/`Vec`/format | lib | KHÔNG | Chưa (M12) |
| 9 | 〃 | IR Guard không phân lớp nghĩa vụ cho lời gọi runtime I/O (`gritrt_fs_read/write_*/arg_*`) | gritc (gt-guard) | KHÔNG | Chưa; KNOWN_LIMITATIONS |
| 10 | 〃 | tên tệp của người dùng với dấu cách, UTF-8 ok; đường dẫn không phải UTF-8 ⇒ `arg` chuyển lossy (U+FFFD) | runtime | KHÔNG | Chưa |

## Những thứ ĐÃ nghĩ có thể chặn nhưng KHÔNG chặn wc (đã kiểm bằng cách viết và chạy)
- Gọi qua trait (`Callee::TraitMethod`): wc không dùng. Số thực: không dùng. 128 bit: không dùng. `String`/`Vec`: không dùng (đếm trên lát byte). Chúng sẽ chặn công cụ lớn hơn (xử lý văn bản, đếm tần suất từ ⇒ cần `Vec`/`HashMap` + trait).
- `⌥`/enum, tuple, lát, `u8`, vòng `↓`: đều dùng được trong dự án thật.

## Công cụ 2: bộ phân tích JSON (`examples/json`) — khoảng trống THEO THỨ TỰ CHẶN, ghi TRƯỚC khi code (2026-10-13)
Đã thử bằng chương trình thật (`gritc check`/`run`) trước khi kết luận:
| # | khoảng trống | kiểm bằng | chặn? | quyết định |
|---|---|---|---|---|
| J1 | **Cây giá trị đệ quy** `enum Value<'a>` chứa `&'a Node<'a>`; cấp phát nút | thử `j2.gt`: enum đệ quy qua tham chiếu arena + `ar.alloc(..)` ⇒ gritc check XANH (borrowck/termination) và chạy đúng | không còn | cần `Arena::{new,alloc,reset}` ở codegen (đã thêm: intrinsic + `gritrt_arena_*`, lớp IR Guard E0405) |
| J2 | `Vec` | không có trong prelude | KHÔNG nếu dùng arena + danh sách liên kết | tool dùng `Node::Cons(&Value, &Node)`; `Vec` chưa làm (ghi lại: chặn công cụ cần truy cập ngẫu nhiên/sắp xếp) |
| J3 | `String` | không có | KHÔNG nếu giữ chuỗi/số dạng khoảng `(đầu, dài)` trên bộ đệm tệp | tool KHÔNG giải mã escape/số; chỉ kiểm cú pháp (đủ cho yêu cầu: đếm, độ sâu, hợp lệ). Ghi rõ là hạn chế |
| J4 | `Result`/`IoError` | thiếu | không chặn nhưng yêu cầu | thêm `open` (xem hàng 5 phía trên) và đổi wc |
| J5 | Số thực | `Ty::Float` ⇒ `Unsupported` ở codegen (đã thử ở nhóm test "chưa hỗ trợ") | KHÔNG nếu không cần GIÁ TRỊ số | số JSON được kiểm cú pháp, không đổi sang `f64`; chặn mọi tool cần giá trị thập phân |
| J6 | Nhân/chia 128 bit | `Unsupported` | KHÔNG | không dùng |
| J7 | Gọi qua trait / generic có ràng buộc | `Callee::TraitMethod` ⇒ `Unsupported` | KHÔNG nếu tránh trait | tool không dùng trait. **Codegen HIỆN hạ được generic KHÔNG ràng buộc** (đơn hình hoá hàm + ADT, đã test `id<T>`, `Option<i32>`, `Atomic<T>`, `Result<T,T>`); KHÔNG hạ được lời gọi phương thức trait (cả `⊐ Trait for T` lẫn `T: Bound` gọi `t.method()`) |
| J8 | Đệ quy sâu làm tràn stack gốc | thiết kế | PHẢI xử lý | tham số ngân sách độ sâu `depth` (↓ depth, M8 chứng minh dừng) ⇒ lồng quá `MAX_DEPTH` trả `Err(TooDeep)` có kiểm soát, KHÔNG đệ quy sâu hơn ngân sách; stack gốc của exe 1 MiB mặc định (link.exe) đủ cho ngân sách |

## Công cụ 2: kết quả (2026-10-13) — `examples/json` (Grit, ~330 dòng, dự án Rig)
**Đo thật:** 6 test (`crates/rig/tests/dogfood_json.rs`), tất cả ĐẠT. Tool ≡ bản tham chiếu Rust (khớp TỪNG KÝ TỰ đầu ra kể cả vị trí byte lỗi và mã) trên: 49 tài liệu hợp lệ + 63 không hợp lệ viết tay, 12 chuỗi UTF-8 sai (nhị phân), số liệu đếm/độ sâu kiểm tay, lồng sâu quanh biên (1…1 000 000 tầng, mảng và đối tượng), 2 tệp lớn, **700 tài liệu đột biến ngẫu nhiên** (52 hợp lệ / 648 không). Hợp lệ khớp thêm với `serde_json` (oracle độc lập) trên mọi tài liệu có ngân sách đủ: **0 bất đồng**. Khác biệt có chủ đích duy nhất: `1e999` — đúng cú pháp RFC 8259, tool chấp nhận (chỉ kiểm cú pháp), `serde_json` từ chối vì tràn f64 ("number out of range"); test coi là đồng ý.
**Hiệu năng (LLVM -O0, mọi check giữ, cây trong arena):** ~13 MiB trong 0.23–0.27 s ≈ 49–57 MiB/s.
**Lồng sâu:** ngân sách 201 đơn vị, mỗi tầng tiêu 2 ⇒ tối đa **100 tầng** hợp lệ (101 ⇒ `nesting too deep` có kiểm soát, thoát 0). 1 000 000 tầng mảng/đối tượng và 5 000 000 `[` (tệp 5 MB) đều trả lỗi có kiểm soát, KHÔNG tràn stack.

### Khoảng trống/bug gặp khi viết (thứ tự gặp) — "gốc" = gritc | lib | codegen | spec
| # | triệu chứng | gốc | đã sửa? |
|---|---|---|---|
| 11 | định nghĩa `enum R { Ok(..), Err(..) }` làm `Ok(f)`/`Err(IoError::..)` không qualify trong CÙNG tệp báo `E0301` (pattern kiểu `R`): tên biến thể của enum người dùng CHE `Ok`/`Err` của prelude | spec (phạm vi tên; hành vi hợp lý nhưng gây bất ngờ) | không đổi ngôn ngữ; tool đặt tên `Good`/`Bad`; ghi KNOWN_LIMITATIONS |
| 12 | `E0702` trên đệ quy tương hỗ `value ⇄ arr/obj` với `↓ budget` (chỉ GIẢM Ở MỘT CẠNH của chu trình): M8 đòi MỌI cạnh trong SCC giảm nghiêm ngặt (thử `m1/m2`: cả trọng số `2n+1`/`2n` cũng bị từ chối) | gritc (M8 bảo thủ, đúng fail-closed) | không; tool tiêu 2 đơn vị ngân sách mỗi tầng; KNOWN_LIMITATIONS |
| 13 | `E0702` trên vòng `⟳ … ↓ len - i` khi bước tiến là `i ← i + n` với `n` là KẾT QUẢ hàm (không chặn dưới được; `len - i` có thể âm) | gritc (bộ giải M8) | không; dùng "nhiên liệu" `fuel ← fuel - 1` (dừng được chứng minh, đủ vì mỗi vòng tiến ≥ 1 byte) |
| 14 | `(a, b) ≔ f()` (phân rã tuple trong `≔`) ⇒ `E0101 mong đợi ; hoặc }` | spec/parser (cú pháp chưa có) | không (cần cú pháp mới ⇒ OPEN_QUESTIONS); dùng `.0/.1` |
| 15 | Không có `Vec`/`String`/`f64`: cây dùng arena + danh sách liên kết; chuỗi/số chỉ là khoảng `(đầu, dài)` — tool KHÔNG giải mã escape, KHÔNG đổi số sang số thực | lib/codegen | không (ngoài phạm vi tối thiểu); chặn mọi tool cần GIÁ TRỊ chuỗi/số |
| 16 | Không có `argc` (thiếu đối số = abort `arg_out_of_range` mã 70, không phải lỗi dùng có kiểm soát) và không có stderr riêng (lỗi ghi ra stdout, thoát mã 2) | lib | không |
| 17 | sự cố công cụ của tôi (không phải của gritc): ghi đè `main.gt` bằng script Python lỗi làm tệp rỗng; khôi phục từ lần ghi trước. Bài học: sửa tệp nguồn Grit bằng công cụ Write/Edit, không bằng script heredoc có `\u` | quy trình | n/a |

### Khoảng trống còn lại (tổng hợp, theo thứ tự chặn công cụ TIẾP THEO)
1. `Vec`, `String`, `f64`, gọi qua trait (`Callee::TraitMethod` ⇒ `Unsupported`), nhân/chia 128 bit — chặn mọi công cụ cần giá trị chuỗi/số, bảng băm, sắp xếp, trait.
2. Phân rã trong `≔`; `?`; stderr; `argc`.
3. M7 chưa chứng minh `b[i]` cho `b` cục bộ mượn từ lời gọi (check runtime giữ — chậm, không sai).
4. Mọi người duyệt TCB mới đang `CHỜ GIANG` (docs/TCB_INTRINSICS.md).

## Công cụ 3: sha256 (`examples/sha256`, Grit ~190 dòng, dự án Rig) — 2026-10-13
### Khoảng trống THEO THỨ TỰ CHẶN, ghi trước khi code (đã thử bằng chương trình thật `p1/p2/p3`)
| # | khoảng trống | thử | chặn? | xử lý |
|---|---|---|---|---|
| S1 | `band/bor/bxor/bnot/rotr/rotl` KHÔNG tồn tại (Q9 nói là hàm lib/core nhưng prelude chỉ có `shl/shr`) | `band(1u32,3u32)` ⇒ `E0201 tên chưa được định nghĩa` | CÓ | thêm 6 hàm u32 vào prelude (một nguồn) |
| S2 | `shl/shr` có trong prelude nhưng KHÔNG hạ được ở codegen | `shl(1u32,3u32)` check XANH, backend llvm `chưa hỗ trợ: hàm không có thân MIR: $prelude::shl` | CÓ | intrinsic `Bit(op)` ở codegen (+ thông dịch `lang_ops`); `n ≥ 32` ⇒ abort `precondition` |
| S3 | mảng cố định, `[0u32; 64]`, `u32/u64`, `+% *%` | `p3.gt` | KHÔNG (đã hạ được, đúng) | — |
| S4 | in byte (hex) ra stdout | `write_str` chỉ nhận `&str` | nhẹ | `Cap<Out>::write_bytes(&[u8])` (cùng hàm runtime `gritrt_write_str`) |
| S5 | u64: không có phép dịch u64 | thiết kế | KHÔNG (độ dài bit tách bằng `/ 256` và `% 256`) | ghi: chưa có họ phép bit u64/u8 |
### Kết quả đo thật
- **Đúng:** 5 test `crates/rig/tests/dogfood_sha256.rs` ĐẠT: 5 vector NIST viết cứng (rỗng, `abc`, 448 bit, 896 bit, 1 triệu `a`); 20 độ dài biên đệm (0,1,2,54,55,56,57,63,64,65,119,120,121,127,128,129,191,192,255,256); **200 tệp ngẫu nhiên**; 1 MiB và 32 MiB — tất cả khớp TỪNG KÝ TỰ với `hashlib` (Python/OpenSSL, độc lập).
- **Tốc độ (LLVM -O0, mọi check giữ):** 32 MiB trong 1.46 s ≈ **21.9 MiB/s** (1 MiB: 10.9 MiB/s gồm cả khởi động).
- **Check còn lại (báo theo lệnh 2b):** chế độ mặc định `--checks=all`: **106 check giữ** (index 42, overflow 34, `pre` 16, slice_range 7, cast_lossy 4, div_zero 2, rem_zero 1), 66 trong số đó "sẽ bị xoá nếu `proven`"; chế độ `--checks=proven`: **24 còn lại** (index 18, overflow 5, cast_lossy 1), 66 xoá. **MỌI tiền điều kiện dịch/xoay (`n < 32`) được M7 CHỨNG MINH — kể cả lượng dịch KHÔNG hằng `((7 - j) * 4) as u32` ở `report` — nên ở `proven` không còn check `precondition` nào**; ở `all` 16 check `pre` vẫn được giữ theo chính sách. Codegen còn tự kiểm `n < 32` (phòng thủ) ở mỗi phép dịch/xoay: với lượng dịch hằng IRBuilder gấp hằng thành `br i1 true`, KHÔNG thêm lệnh so sánh lúc chạy.
- **So sánh codegen với phép bit** (`llvm_diff_bits.rs`, 4 lô × 25 biểu thức u32 ngẫu nhiên có band/bor/bxor/bnot/shl/shr/rotr/rotl với lượng dịch hằng/che mặt nạ/thô + `+% -% *%` + `+ - *`): **ba nguồn khớp (thông dịch, exe LLVM, đánh giá Rust của cây biểu thức) trên 352 phép chạy, 0 bất đồng**; abort khớp: overflow 116, precondition 51; 185 chạy thường; 12/100 chương trình bị cổng ĐỎ E0901 (hằng chắc chắn tràn) ⇒ bỏ. ĐỘ NHẠY kiểm bằng cách đổi `rotr`↔`rotl` ở codegen ⇒ lô ĐỎ (86 s), đã hoàn nguyên.
### Bug / khoảng trống gặp khi viết
| # | triệu chứng | gốc | đã sửa? |
|---|---|---|---|
| 18 | `E0702` trên vòng `⟳ j < rem ↓ rem - j` TRONG hàm có >24 biến nguyên (`digest` ban đầu): M7 giảm độ chính xác (N0605) nên mất quan hệ `j < rem` | gritc (bộ giải M7, ZONE_MAX_VARS = 24) | không đổi bộ giải; tách `copy_tail`/`put_length` thành hàm riêng ít biến (cách làm có tài liệu); KNOWN_LIMITATIONS |
| 19 | không có phép bit u64/u8, `rotr/rotl` chỉ u32; tên theo kiểu (`bandN`) sẽ cần nếu thêm | lib | chưa (không chặn) |
| 20 | `bnot`, `shl`... là HÀM nên biểu thức bit dài khó đọc (`bxor(bxor(rotr(..),..),..)`) | spec (Q9 có chủ đích) | không |

## Công cụ 4: sortlines (`examples/sortlines`, Grit: main.gt ~100 dòng + msort.gt ~90 dòng, dự án Rig) — 2026-10-14
### Khoảng trống THEO THỨ TỰ CHẶN (ghi khi làm; hầu hết đã đóng ở các bước ADR-032 (i)–(iii))
| # | khoảng trống | chặn? | xử lý |
|---|---|---|---|
| T1 | gọi phương thức qua ràng buộc generic `x.m()` ⇒ codegen thoát 3 | CÓ | bước (i): `select_impl` một nguồn, đơn hình hoá, symbol ASCII, Guard luật instance |
| T2 | không có `Vec`/mảng tăng trưởng | CÓ | bước (ii): `Vec<'a,T>` trên Arena + E0318 + `arena_exhausted` |
| T3 | không có `Ord`/`Ordering`/sắp xếp | CÓ | bước (iii): prelude `Ordering`/`Ord`, `msort.gt` |
| T4 | `Vec::push` gấp đôi ⇒ không có công thức cỡ Arena | nhẹ | thêm `Vec::reserve` (TCB §B3), công thức ADR-034 phụ lục |
| T5 | `⊐ <'a> Ord for Line<'a>` với `o: &Line<'a>` ⇒ E0311 oan (FB-3) | nhẹ | né bằng `&Self`; chưa sửa gốc |
| T6 | M7 không chứng minh được vòng bước tiến biến/nhân đôi (`lo += 2w`, `width *= 2`) | nhẹ | mẫu nhiên liệu (KNOWN_LIMITATIONS) |
| T7 | `Vec` rỗng không suy ra kiểu phần tử (E0301) | nhẹ | luôn có ít nhất một `push`/chú thích |
| T8 | ghi stdout tới TỆP rất chậm (flush mỗi lời gọi) | không (đúng nhưng chậm) | ghi KNOWN_LIMITATIONS; chưa đệm |
### Kết quả đo thật (7 test `crates/rig/tests/dogfood_sortlines.rs` ĐẠT, so với bản tham chiếu Rust `sort_by` theo byte, khớp TỪNG BYTE)
- **Đúng:** 15 ca nhỏ (rỗng, chỉ `\n`, một dòng có/không `\n` cuối, dòng rỗng xen kẽ, CRLF thuần và lẫn, không xuống dòng cuối, UTF-8 + 😀, byte cao 0xff/0x80/0x7f, NUL, trùng lặp, tiền tố, `\r` đơn); một dòng dài 1 MiB (3 ca, gồm hai dòng chung tiền tố 1 MiB); 1 MiB và 32 MiB văn bản ngẫu nhiên (có NUL, 0xff, CR, 0xc3); 10⁶ dòng ngắn; 4 MiB toàn dòng rỗng (4 194 304 dòng) + 1000 dòng rỗng; đường lỗi (thiếu tệp ⇒ mã 2 + `sortlines: <tệp>: no such file` ở stderr, stdout rỗng; thiếu đối số ⇒ `usage: sortlines <file>`, mã 2); công thức Arena (xem ADR-034 phụ lục).
- **Tính ổn định KHÔNG quan sát được** khi so toàn dòng (hai dòng bằng nhau về byte không phân biệt được) nên sortlines KHÔNG tuyên bố đã kiểm ổn định. Tính ổn định của `sort` đã kiểm riêng ở `gt-driver/tests/llvm_sort.rs` bằng hash theo chỉ số gốc (4 bố cục rộng 24/32 byte × 50 000 phần tử, 60 mảng ngẫu nhiên, 10⁶ phần tử, độ nhạy đã thử).
- **Hiệu năng so với Rust `sort_by` (đo, -O0, mọi check giữ; KHÔNG phải cổng; ống pipe, một lần chạy mỗi ca, máy dev) — SAU khi tool gom đầu ra vào một `Vec<u8>` rồi ghi MỘT lần (Q75, 2026-10-14):**

| ca | sortlines.gt | Rust sort_by | tỉ lệ thời gian | đỉnh working set (sortlines) / ước tính Rust |
|---|---|---|---|---|
| 1 MiB văn bản | 0,09 s | 0,040 s | ×2,4 | 7 / 3 MiB |
| 32 MiB văn bản | 2,43 s | 2,02 s | ×1,2 | 118 / 89 MiB |
| 10⁶ dòng ngắn (4,5 MB) | 0,75 s | 0,53 s | ×1,4 | 42 / 24 MiB |
| 4 MiB toàn dòng rỗng | 0,99 s | 0,26 s | ×3,8 | 139 / 72 MiB |
- **Bản đo TRƯỚC đó đã bị thay:** khi in từng dòng bằng hai lời gọi `write_*` (mỗi lời gọi flush stdout) tool chậm ×3,6–×27 (32 MiB: 7,24 s; 10⁶ dòng: 3,63 s; 4 MiB rỗng: 7,20 s) — thời gian chủ yếu là syscall ghi, không phải sắp xếp. Gom đầu ra một lần giảm 3–10×. Bộ nhớ đỉnh tăng bằng một Arena thứ ba (kích thước tệp: 86 → 118 MiB ở 32 MiB). Ra TỆP (PowerShell `Start-Process -RedirectStandardOutput`) trước đây 129,5 s cho 10⁶ dòng: CHƯA đo lại sau thay đổi.
- Rust: thời gian đo TRONG tiến trình thử (không gồm khởi động); bộ nhớ Rust là ƯỚC TÍNH (tệp·2 + 16·dòng), không đo; sortlines.gt thời gian gồm khởi động tiến trình + đọc/ghi. Chưa đo lặp lại nhiều lần nên bỏ qua phương sai.

## Công cụ 5: wordfreq (`examples/wordfreq`, Grit: main.gt ~160 dòng + msort.gt, dự án Rig, dùng `Text`) — 2026-10-14
### Khoảng trống THEO THỨ TỰ CHẶN
| # | khoảng trống | chặn? | xử lý |
|---|---|---|---|
| W1 | `u8 as char` không hạ được ở codegen (thông dịch chạy được) | CÓ (rig build thoát 3) | FB-4: sửa + test hồi quy (commit riêng) |
| W2 | không có bảng băm / map | nhẹ (theo lệnh) | sắp xếp từ rồi đếm đoạn bằng nhau, rồi sắp xếp cặp (O(n log n)); HashMap ghi nhận là khoảng trống của thư viện |
| W3 | không có số → chuỗi (`Display`) | nhẹ | viết tay `push_u64` bằng `push_char` (20 chữ số, hai vòng hữu hạn) |
| W4 | `&str` không có `as_bytes`, so sánh, băm | nhẹ | dùng `Text::as_bytes()` và lát byte; từ là `&[u8]` cắt ở byte ASCII (UTF-8 hợp lệ nhờ cách cắt) |
| W5 | mỗi Text/Vec cần một Arena riêng | nhẹ | 6 Arena (đúng hệ quả ADR-032 phụ lục A và Q72) |
| W6 | `Text::from_utf8` sao chép toàn tệp (không có kiểm UTF-8 tại chỗ trên `&[u8]`) | nhẹ | chấp nhận (bộ nhớ ≈ 7× tệp); khoảng trống: `str::from_utf8(&[u8]) → Option<&str>` không sao chép |
| W7 | không có `Vec::clear/truncate/sort` dựng sẵn | nhẹ | thu vào hai Vec rồi `msort::sort` (một Arena một Vec) |
### Kết quả đo thật (5 test `crates/rig/tests/dogfood_wordfreq.rs` ĐẠT; so với tham chiếu Rust HashMap + sắp xếp, khớp TỪNG BYTE)
- **Đúng:** 14 ca nhỏ (rỗng, chỉ dấu phân cách, một từ, hoà theo byte, hoa/thường, dấu gạch dưới/dấu câu, CRLF, từ UTF-8 (é, 日本語, 😀), dính ASCII-UTF-8, NUL là dấu phân cách, chữ số, 12 và đúng 10 từ phân biệt); 9 ca UTF-8 SAI (0xFF, byte nối lẻ, cắt giữa 2/4 byte, overlong `C0 AF` và `E0 80 80`, surrogate `ED A0 80`, > U+10FFFF, Latin-1 `é`) ⇒ lỗi có kiểm soát mã 2, stdout rỗng, KHÔNG abort; thiếu tệp/thiếu đối số; 1 MiB và 32 MiB văn bản ngẫu nhiên (nhiều từ, tần suất lệch); hai từ 1 MiB; 10⁶ lần `a` (đếm 1 000 000); 200 000 từ phân biệt.
- **Hiệu năng (đo, -O0, mọi check giữ, ống pipe, một lần chạy mỗi ca):** 1 MiB (300 từ) 0,21 s (×4,2 so Rust), đỉnh 12 MiB; 32 MiB (5000 từ) 7,23 s (×4,8), đỉnh 234 MiB; 4 MiB (200 000 từ) 0,76 s (×4,0), đỉnh 33 MiB. Rust: thời gian đo trong tiến trình thử (HashMap + sắp xếp); bộ nhớ Rust không đo. Phương sai chưa đo.
