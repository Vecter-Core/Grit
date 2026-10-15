# READINESS — "dùng được thật" chưa? (kiểm 2026-10-14, sau ADR-036 bước iv; viết cho Giang)

Mọi con số dưới đây lấy từ lần chạy/lần đọc thật trong phiên này hoặc từ `docs/DOGFOOD.md`, `docs/PROGRESS.md` (có ghi nguồn). Cái gì CHƯA kiểm thì nói rõ. **Không tuyên bố Phase 1 xong.** Không có M12/M13.

## a. Sáu công cụ viết bằng Grit (dự án Rig, `examples/`)
| công cụ | dòng Grit | đúng (bằng chứng) | tốc độ (-O0 `dev`, mọi check giữ) | khoảng trống / giới hạn |
|---|---|---|---|---|
| `wc` | 65 | `dogfood.rs` 3 test: 10 tệp biên + 32 MiB + 2 đường lỗi, khớp tham chiếu | 32 MiB: 0,28 s (≈ 114 MiB/s; đo 2026-10-14 bằng `tools/bench_profile.py`) | chỉ đếm dòng/từ/byte; không đọc stdin |
| `json` (kiểm cú pháp + đếm/độ sâu) | 326 | 6 test: 49 hợp lệ + 63 sai viết tay, 12 UTF-8 sai, lồng sâu đến 10⁶ tầng; khớp TỪNG KÝ TỰ với bản Rust | ≈ 13 MiB trong 0,23–0,27 s (49–57 MiB/s; DOGFOOD.md) | KHÔNG giải mã escape, KHÔNG đổi số sang giá trị; lồng tối đa 100 tầng theo ngân sách |
| `sha256` | 160 | 5 test: 5 vector NIST, 20 độ dài biên, 200 tệp ngẫu nhiên, 1 MiB và 32 MiB | 32 MiB: 1,41–1,46 s ≈ 22 MiB/s | thuần u32/u64; không có thư viện băm dùng chung |
| `sortlines` | 105 + 86 (msort) | 8 test: 15 ca nhỏ, dòng 1 MiB, byte lạ; khớp tham chiếu | 32 MiB: 2,43 s so với Rust `sort_by` 2,02 s (×1,2); 4 MiB dòng rỗng ×3,8 (DOGFOOD.md) | tự viết merge sort (chưa có `Vec::sort`); ổn định không quan sát được qua so sánh toàn dòng |
| `wordfreq` | 169 + 86 | 6 test: 14 ca nhỏ, 9 ca UTF-8 sai (lỗi có kiểm soát), 1/4/32 MiB, 200 000 từ phân biệt; khớp từng byte với Rust HashMap | ×4,0–×4,8 so với Rust (32 MiB: 7,23 s; đỉnh 234 MiB) — chậm nhất (DOGFOOD.md; phương sai chưa đo) | không có bảng băm ⇒ đếm bằng sắp xếp + gộp; sao chép toàn tệp (bộ nhớ ≈ 7× tệp) |
| `stats` (dùng `f64`) | 136 | 4 test: 14 ca nhỏ, 11 ca lỗi, 200 tệp ngẫu nhiên, 10⁶ số và 32 MiB; khớp từng byte | 10⁶ số 0,15–0,17 s (×1,3 so Rust) | in số nguyên làm tròn (không in `f64`); chung giả định toán với bản tham chiếu (KNOWN_LIMITATIONS) |
Tổng: 32 test dogfood (3+6+5+8+6+4). **Chưa kiểm:** phương sai tốc độ ở `wordfreq`; hành vi trên hệ điều hành khác Windows; tệp lớn hơn 32 MiB; chạy song song nhiều tiến trình.

## b. Khoảng trống THẬT để người khác viết công cụ ~500 dòng (thử bằng chương trình thật hôm nay, không suy đoán)
Thứ tự theo mức CHẶN. Mỗi dòng ghi "kết quả thử".
**CHẶN (không viết được hoặc phải đi đường vòng lớn):**
1. **Không có bảng băm/map** (`HashMap`, `BTreeMap`: `E0201 tên chưa được định nghĩa`). Mọi việc đếm/tra cứu/khử trùng theo khoá phải dùng mảng + sắp xếp (như `wordfreq`: chậm ×4–5 so Rust) hoặc tìm tuyến tính. Đây là khoảng trống lớn nhất.
2. **Không có stdin và không ghi tệp** (`root_in` không tồn tại; `Cap<Fs>::write` không có). Công cụ kiểu lọc/đường ống (đọc stdin) và công cụ sinh tệp không viết được; chỉ đọc tệp qua đối số và ghi stdout/stderr.
3. **Đối số dòng lệnh chỉ có `arg(i)` + `arg_count`** (không có lặp/`--cờ`; `env.args()` không tồn tại). Cờ phải tự phân tích bằng tay trên `&str` (khả thi vì `str::len/slice` và so sánh `==` đều có, nhưng chưa có `find/split/trim`).
4. **Thư viện chuỗi/`Vec` rất mỏng:** `Vec` chỉ có `new/push/reserve/len/as_slice/as_mut_slice` (không `pop/get/insert/remove/clear/sort`); `Text` chỉ có `new/push_str/push_char/push_utf8/from_utf8/len/as_str/as_bytes`; `str` chỉ có `len/slice`. Không `find/split/trim/parse`. Công cụ ~500 dòng sẽ phải tự viết các hàm này (như `msort`). Là "chặn" theo nghĩa phải viết thêm vài trăm dòng lib, chứ không bất khả thi.
**LÀM KHÓ NHƯNG KHÔNG CHẶN (đi vòng được):**
5. **`Option`/`Result` tối thiểu:** chỉ `is_some` và `unwrap`; `unwrap` trên giá trị chưa chứng minh `is_some` ⇒ ĐỎ `E0902` (hợp đồng `⊳ self.is_some()` không kiểm được lúc chạy vì `Option` không `Copy`) — thử: `ƒ f(o: Option<u64>) → u64 { o.unwrap() }` ⇒ đỏ; `⎇ o.is_some() { o.unwrap() }` ⇒ xanh. Không có `map/unwrap_or/and_then`; **không có toán tử `?`** (ký tự không thuộc ngôn ngữ, `E0101`). Mọi chỗ phải `⌥` tường minh ⇒ dài dòng. (FB-6 ghi lại.)
6. **Định dạng đầu ra:** chỉ `write_u64`, `write_str`, `write_bytes` (không số có dấu, `f64`, hex, đệm); `format`/chuỗi mẫu không tồn tại. `stats` và `sha256` tự viết hàm in số/hex.
7. **Số:** không `abs/min/max/pow` (`E0309`); `i128/u128` qua `check` nhưng **backend từ chối** (`thoát 3`: "nhân có kiểm tràn 128 bit (cần __muloti4)"); `f32` và `%` trên số thực bị `E0319` (Q79, Q80 chờ Giang); không in `f64`.
8. **Ngôn ngữ:** không closure (`|x| …`: `E0101`); không phân rã tuple trong `≔` (`(a, b) ≔ …`: `E0101`; dùng `.0/.1`); `T: Ord` không dùng được toán tử `<` trên `T` (`E0301`; phải gọi `.cmp`); không `format!`.
9. **Dừng (M8):** vòng/đệ quy có bước không chứng minh được giảm (nhân đôi, bước = kết quả hàm) bị `E0702` ⇒ phải dùng mẫu "nhiên liệu" (`fuel ← fuel - 1`) hoặc ngân sách độ sâu; đệ quy tương hỗ chỉ giảm ở MỘT cạnh bị `E0702`. **E0703** đã nới (hàm chuyển tiếp/bọc được chấp nhận qua đồ thị instance; chu trình thật vẫn đỏ đúng); nhưng cạnh trait nằm trong SCC không kiểm được vẫn bị từ chối bảo thủ.
10. **FB-3 (E0311 oan)** `o: &X<'a>` trong `⊐ <'a> Tr for X<'a>` bị đỏ dù đúng; né bằng `&Self`. FAIL-CLOSED (từ chối chương trình đúng), ưu tiên thấp.
11. **Bộ nhớ:** mỗi `Vec`/`Text` cần MỘT `Arena` riêng (`&!Arena` độc quyền); `T` cần drop bị `E0318`; không có giải phóng từng phần tử (chỉ `reset` arena).
12. **Hiệu năng:** chỉ có `-O0`; hồ sơ `release` hiện ≈ `dev` (chưa nhanh hơn; sha256 chậm hơn ×0,65). Backend O2 cho ×1,3–×3,4 theo min nhưng cần Giang duyệt (Q82).
13. **M7:** `b[i]` với `b` là lát cục bộ mượn từ lời gọi chưa chứng minh được (N0602, giữ check lúc chạy — chậm, không sai).
**Không kiểm:** quản lý phụ thuộc ngoài (Rig chỉ có khoá `rig.lock` băm gói cục bộ, chưa thử kho gói); LSP/IDE (M13 chưa làm).

## c. GRIT_PLAN §7 — bảng điều kiện chuyển giai đoạn (trạng thái THẬT)
Trạng thái: **đạt** / **đạt có điều kiện** / **chưa**. Phase 1 CHƯA xong.
| # | điều kiện (tóm tắt) | trạng thái | bằng chứng và phần còn thiếu |
|---|---|---|---|
| 1 | Toàn bộ test xanh; không còn false negative đã biết ở corpus borrowck/đa luồng/biên | **đạt có điều kiện** | Cổng đầy đủ 2026-10-14: 836 đạt / 0 đỏ / 156 bộ, GATE_RESULT=GREEN. Còn false negative TIỀM ẨN đã ghi: đột biến 361/591/554 của `regions.rs` sống (lập luận đơn điệu chờ Giang chấp nhận); "lỗ hổng âm tính giả tìm muộn" (FB-5); bảng ép kiểu D1 chưa quét từng ô |
| 2 | Fuzz lexer/parser/typechecker mỗi cái ≥ 24 giờ tích luỹ | **chưa** | Chỉ có lexer và parser, 6 phiên × 10 phút mỗi cái (ASan Windows giới hạn phiên); typechecker CHƯA có fuzz; tổng giờ ≪ 24 (KNOWN_LIMITATIONS, PROGRESS) |
| 3 | 3 chương trình thật (M12) chạy đúng; benchmark nền được ghi | **đạt có điều kiện** | Có 6 công cụ thật khớp tham chiếu + `docs/BENCHMARK.md` (nền + release vs dev). Nhưng KHÔNG gọi là M12 (chưa viết M12 theo lệnh); benchmark chỉ trên một máy, nhiễu lớn do Defender |
| 4 | Mọi mã lỗi có tài liệu + test đủ | **đạt** | `tools/check_expectations.py` + unittest xanh trong cổng (kiểm cấu trúc: mỗi mã trong `DOC_REQUIRED` có ≥3 fail + ≥3 pass); chưa kiểm ngữ nghĩa từng tài liệu bằng người |
| 5 | Output tái lập 100% (build hai lần so băm) | **đạt có điều kiện** | `rig.rs::builds_are_reproducible_byte_for_byte` (dự án 3 gói, hồ sơ dev, `/Brepro`). CHƯA có test tái lập cho hồ sơ `release`, cho từng công cụ dogfood, hoặc trên máy khác |
| 6 | Giang xem `KNOWN_LIMITATIONS.md` và `PROGRESS.md` | **chưa** | Chờ Giang |
| 7 | ADR chốt giao diện Obligation IR ở dạng ổn định | **chưa** | ADR-025 mô tả IR nghĩa vụ + tuần tự hoá JSON (schema 1, id tất định) và đã cài; chưa có ADR nào tuyên bố "đóng băng/ổn định" cho Phase 2 |
| 8 | M13 (LSP) xong + schema JSON chẩn đoán đóng băng | **chưa** | M13 chưa làm; schema chẩn đoán v1 có quy tắc "chỉ thêm trường" nhưng chưa tuyên bố đóng băng |
| 9 | Mutability bên trong: mở lại đột biến 238/361/591/554 trước khi có kiểu mutability bên trong | **đạt có điều kiện** | 238 đã GIẾT (và là đối chứng dương: 73 chương trình bị đột biến 238 chấp nhận). 361/591/554: kiểm BẰNG MÁY (2026-10-15, `probe_diff`, 24 143 chương trình): (i) bản đột biến chấp nhận mà gốc từ chối = **0**; tiền đề 'chỉ thêm cạnh' đúng 24 143/24 143; (ii) tập chênh 1 503/1 169/985 chương trình đều là chương trình ĐÚNG bị chặn nhầm, oracle sạch; mọi luật chẩn đoán đơn điệu theo ràng buộc (MUTANT_ARGUMENTS A.1). Điều kiện: lập luận đơn điệu là đọc mã + đo, không phải chứng minh hình thức; tập chênh 554/591 chỉ ≈ 3 hình dạng; Giang cần chấp nhận có điều kiện |
| 10 | Đột biến M7 "xoá thêm check" chưa giải quyết + chế độ giữ mọi check là mặc định | **chưa** (nửa sau **đạt**) | Nửa sau: mặc định `--checks=all` (có test) ĐÃ đạt. Nửa đầu: **69 đột biến NGUY HIỂM còn sống** (`analysis.rs` 62 + `driver.rs` 7; 1 148 đột biến gt-oblig: 826 bắt / 232 sống / 90 không biên dịch) — chưa giải quyết (KNOWN_LIMITATIONS, PROGRESS); theo điều kiện cứng, M9 chưa được coi là xong. Đột biến CastFits f64 mới (bước f64) là thêm, không thay |
| 11 | E0601 (M7) và E0307 (M5) triển khai + qua test | **đạt** | Có fixture `E0601_literal_out_of_range`, `E0307_trailing_semicolon` và nằm trong cổng xanh |
| 12 | Mô hình capability: không hàm ngoài `main` gọi nguồn capability; nguồn mới có mẫu E0310 | **đạt** | E0310 chặn (fixtures `tests/fail/E0310_*`); mọi nguồn mới (`root_out/err/env/alloc`) đã có mẫu; chưa quét lại các nguồn tương lai |
**Kết luận (đếm lại):** đạt = #4, #11, #12 (**3**); đạt có điều kiện = #1, #3, #5, #9 (**4**); chưa = #2, #6, #7, #8, #10 (**5**). Phase 1 CHƯA xong.

## d. TCB (phần tin cậy không kiểm được bằng chính compiler) và cột "người duyệt"
- **Intrinsic/runtime trong `docs/TCB_INTRINSICS.md`: 42 dòng, 6 nhóm; CẢ 42 dòng có cột "người duyệt" = `CHỜ GIANG`** (không dòng nào được duyệt). Nhóm: A năng lực/I/O (15), B đồng thời (5), B2 Arena (3), B3 `Vec` (5), B4 `Text`/`str` (8), B5 `f64` (6). Cộng nhóm C: runtime `gt-rt` (staticlib Rust) với bảng đóng `RUNTIME` của IR Guard có **23** hàm.
- Các thành phần TCB KHÁC (không nằm trong bảng trên): bộ giải M7 (khoảng + zone: quyết định XOÁ/GIỮ check), checker mượn M6, kiểm dừng/hiệu ứng/đa luồng M8, quét secret M9; IR Guard (`gt-guard`: luật Guard-pre và Guard-post); lớp FFI LLVM `gt-llvm/shim.rs` (21 chỗ `unsafe`, chỗ DUY NHẤT); **LLVM 20.1.8** (bộ sinh mã; từ ADR-036 còn có 3 pass IR `mem2reg,sroa,early-cse` chạy SAU Guard-pre, được Guard-post kiểm lại); trình liên kết MSVC `link.exe` và thư viện hệ thống; thư viện chuẩn Rust của runtime; Windows. Các mục này chưa có cột "người duyệt" riêng.
- Q chờ Giang liên quan: Q79 (`f32`), Q80 (`%` số thực), Q81 (đường "chứng minh ghi lại" cho `instcombine`), Q82 (backend O2 trong TCB).

DỪNG ở đây, chờ Giang duyệt trước khi làm bất cứ việc nào tiếp theo.

## CẬP NHẬT 2026-10-15 (Giang ủy quyền kiến trúc sư duyệt TCB và đột biến)
- **Điều kiện #9 (361/591/554): CHẤP NHẬN có điều kiện** (kiến trúc sư): lập luận đơn điệu được chấp nhận vì đã kiểm BẰNG MÁY trên 24 143 chương trình (0 vi phạm tiền đề, 0 chương trình (i), đối chứng dương bắt được 238). Điều kiện mở lại: (i) nếu một luật ĐỎ mới của M6 được thêm, kiểm kê A.1 (MUTANT_ARGUMENTS) phải cập nhật và đo lại `probe_diff`; (ii) `probe_diff` bản đủ ngưỡng (`GT_PROBE_N=4000`, ≈ 6 phút) chạy TRƯỚC mỗi lần đóng milestone (không nằm trong cổng thường).
- **TCB (d):** cả 42 dòng đã có trạng thái `ỦY QUYỀN: KIẾN TRÚC SƯ DUYỆT HỢP ĐỒNG` (14 dòng CÓ ĐIỀU KIỆN, 28 dòng không điều kiện; xem TCB_INTRINSICS.md). Đây là duyệt HỢP ĐỒNG, không phải kiểm toán mã.
- **ĐIỀU KIỆN CỔNG MỚI:** nhóm CAO (H1–H10) PHẢI qua **ĐỌC MÃ ĐỐI KHÁNG** (`docs/TCB_CODE_AUDIT.md`, mục E) trước khi tuyên bố TCB đã duyệt. Cho tới khi đó: "chưa có người thứ hai đọc mã".
