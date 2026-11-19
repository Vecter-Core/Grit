# Quy trình hồ sơ lỗi thật (ADR-029 Q65)

Mọi bug thật gặp khi dùng gritc/M10 trở đi đều có HỒ SƠ và TEST HỒI QUY trước khi sửa. Không sửa lặng lẽ.

## Mỗi bug = một mục dưới đây, theo mẫu
- **Mã:** `FB-<số tăng dần>`; **Ngày:**; **Người gặp / ngữ cảnh:** (chương trình đang viết, lệnh gritc, backend).
- **Triệu chứng (số thật):** đầu ra, mã thoát, backend (`llvm`/`interpreter`), phán quyết.
- **Chương trình tối thiểu tái hiện:** nguồn `.gt` ≤ 30 dòng (đưa vào `tests/` khi sửa).
- **Phân loại:** (a) sai tích cực/âm tính của checker (M4–M9) — nghiêm trọng nhất nếu âm tính giả (chương trình sai được chấp nhận); (b) codegen/gt-rt sai (E1101 hay kết quả sai khi chạy); (c) công cụ/môi trường; (d) tài liệu/thông báo.
- **Nguyên nhân gốc:** (không né bằng đổi tên / thêm ngoại lệ — truy ra gốc).
- **Sửa + test hồi quy:** commit, tên test (≥ 1 test mới mà TRƯỚC sửa thì đỏ).
- **Ảnh hưởng tới điều kiện cứng:** có chạm điều kiện mutability bên trong / `--checks=all` / cổng ĐỎ-ICE không? Nếu có phải báo Giang.

## Danh sách
### FB-1 — ICE giả (E9901) trong borrowck cho mọi hàm có `⊳`/`⊲` và tham số tham chiếu (2026-10-13)
- **Triệu chứng:** `ƒ h(xs: &[u32], p: u32) → u32 ! ∅ ⊳ p <= 3u32 { p }` ⇒ `gritc check` phán quyết ICE: "chốt chặn phòng thủ bị chạm tới: tham số không khớp RegionSig của chính hàm" (borrowck `main::h⊳0`). Fail-closed nên không có chương trình sai nào lọt, nhưng chương trình HỢP LỆ bị từ chối. ~40% chương trình của bộ sinh oracle M7 dính khi so sánh codegen.
- **Phân loại:** (a) checker M6 (sai tích cực dạng ICE), tìm thấy bằng so sánh ngẫu nhiên codegen ≡ thông dịch (không phải do người dùng).
- **Nguyên nhân gốc:** thân hợp đồng (`Body.kind = Pre/Post`) có `node` = nút của hợp đồng, nhưng `RegionSig` được lập theo nút của HÀM; `gt-borrowck/src/regions.rs::setup_universals` tra `regions.get(&body.node)` ⇒ không thấy ⇒ coi tham số có vị trí region là "không khớp". M6 không có mẫu `⊳` + tham số tham chiếu nên không lộ.
- **Sửa:** thêm `Body.sig_node` (nút của hàm có chữ ký; = `node` với hàm/const/static, = hàm chứa với `⊳`/`⊲`); borrowck tra theo `sig_node`; codegen dùng `sig_node` để lấy tham số kiểu.
- **Test hồi quy:** `tests/pass/FB1_ok_pre_with_slice_param`, `FB1_ok_pre_with_ref_param`, `FB1_ok_post_with_ref_param` (cả ba ĐỎ ICE trước sửa, XANH sau), chạy qua corpus của gt-driver; so sánh ngẫu nhiên `gt-driver/tests/llvm_diff.rs` giữ không cho ICE (coi ICE là bất đồng).
- **Vì sao lọt qua các lưới cũ (ghi theo lệnh của Giang 2026-10-13):** (1) bộ sinh của oracle M6 (`gt-borrowck/tests/support/progen`) KHÔNG sinh hợp đồng `⊳`/`⊲`; (2) bộ sinh của oracle M7 (`gt-oblig/tests/support/obgen`) CÓ sinh `⊳`/`⊲` cùng tham số lát/tham chiếu nhưng oracle chỉ chạy M4→M5→M7, KHÔNG chạy borrowck; (3) corpus M6 không có mẫu nào kết hợp hợp đồng với tham số tham chiếu. Không lưới nào chứa TỔ HỢP (hợp đồng × tham số tham chiếu/lát × borrowck).
- **Lưới mới:** oracle M7 (`gt-oblig/tests/oracle.rs`) nay chạy thêm `gt_borrowck::check_program` trên MỌI chương trình sinh ra và cấm E9901 (E04xx hợp lệ vẫn được phép vì chương trình ngẫu nhiên có thể bị từ chối đúng). KIỂM BẰNG TẮT BẢN SỬA: đổi `regions.rs` về tra `body.node` ⇒ oracle ĐỎ ngay ở seed #1 ("ICE trong borrowck `main::h1⊳0`: … tham số không khớp RegionSig"), bật lại ⇒ xanh (198 s cả bộ).
- **Chạm điều kiện cứng?** Không (chỉ bỏ từ chối thừa; không làm checker lỏng hơn cho chương trình sai: RegionSig đúng của hàm được dùng).


### (2026-10-13) Công cụ 2 (JSON): KHÔNG phát hiện bug sai-đúng nào của gritc/codegen
Viết `examples/json` (330 dòng) và so sánh với tham chiếu Rust + serde_json trên ~900 tài liệu (gồm 700 đột biến ngẫu nhiên, lồng tới 1 triệu tầng): 0 bất đồng. Các điểm vướng gặp là KHOẢNG TRỐNG/ràng buộc có chủ đích (E0702 bảo thủ, che tên biến thể, thiếu `Vec/String/f64`), ghi ở `docs/DOGFOOD.md` #11–#17, không phải bug cần test hồi quy. Số liệu "0 bất đồng" chỉ nói về lớp đầu vào đã thử; không chứng minh tool đúng với mọi tài liệu.

### FB-2 — IR Guard từ chối nhầm lời gọi hàm hợp đồng (tên Unicode) (2026-10-13)
- **Triệu chứng:** sau khi thêm luật "mọi lời gọi tới hàm chỉ khai báo phải là `llvm.*`/runtime", `gritc build/run` (backend llvm) trên chương trình có `⊳`/`⊲` mà M7 giữ kiểm tra lúc chạy ⇒ ĐỎ E1101 "lời gọi hàm ngoài `main::h1\E2\8A\B20#3`", không có exe. Chương trình hợp lệ bị từ chối (fail-closed, không có chương trình sai lọt).
- **Phân loại:** (b) gt-guard (do chính luật mới của tôi): lấy tên hàm đích từ DẠNG CHỮ của lệnh (`print_to_string`) trong khi LLVM in tên có byte không ASCII ở dạng thoát `\E2\8A\B2` ⇒ `module.get_function(tên_thoát)` không thấy ⇒ coi là hàm ngoài.
- **Phát hiện bởi:** test so sánh `llvm_diff::permanent_and_small_fixed_batch` trong cổng đầy đủ (seed #4 của lô cố định). Các test Guard viết tay trước đó KHÔNG có hàm tên Unicode nên không lộ — cùng kiểu "tổ hợp chưa có trong lưới" như FB-1.
- **Nguyên nhân gốc / sửa:** lấy tên bằng API (`CallSiteValue::get_called_fn_value().get_name()`), không bằng chữ; cũng loại cách đoán `@tên(` khỏi `callee_name`.
- **Test hồi quy:** `gt-guard/tests/guard.rs::unicode_named_functions_are_resolved_by_api_not_by_printed_text` (có thân ⇒ chấp nhận; chỉ khai báo tên Unicode ⇒ từ chối), `gt-driver/tests/llvm.rs::contract_functions_with_unicode_symbol_names_pass_the_guard`.
- **Hệ quả:** bản đột biến gt-guard đã chạy TRƯỚC sửa (57 mutant, 56 giết) là của hàm `callee_name` CŨ ⇒ phải chạy lại sau sửa (xem PROGRESS).

## FB-3 — `⊐ <'a> Ord for Line<'a> { ƒ cmp(&self, o: &Line<'a>) … }` bị E0311 oan (2026-10-14, gặp khi viết kiểu bọc cho sortlines)
- **Ngữ cảnh:** bộ sinh `llvm_diff_gtv` (loại `vec_line_sort`), `gritc check`.
- **Triệu chứng:** `E0311 hứa về lifetime ít hơn trait: region của kết quả phụ thuộc vào đầu vào khác với chữ ký của trait` cho tham số viết tường minh `o: &Line<'a>`; viết `o: &Self` thì XANH. Kết quả trả về là `Ordering` (không có region) nên lỗi là SAI TÍCH CỰC (từ chối chương trình đúng); fail-closed, có cách né.
- **Tái hiện tối thiểu:** `▢ Line<'a> { b: &'a [u8] }` / `⊐ <'a> Copy for Line<'a> { }` / `⊐ <'a> Ord for Line<'a> { ƒ cmp(&self, o: &Line<'a>) → Ordering ! ∅ { … } }`.
- **Phân loại:** (a) sai tích cực của checker M4.5 (không phải âm tính giả ⇒ không mất soundness).
- **Nguyên nhân gốc:** `region.rs::slots` đánh dấu vị trí region "đến từ `Self`" CHỈ cho `TypeKind::SelfType`; trường hợp người dùng viết lại chính kiểu của impl (`Line<'a>`) không được nhận là `Self`, nên số vị trí region của tham số ở impl (2: `&` và `'a`) khác của trait (`&Self`: 1 vị trí không-Self + 1 vị trí Self bị lọc) ⇒ `regions_compatible` trả `false`.
- **Sửa:** CHƯA làm. Cần nhận ra "đường dẫn trùng kiểu của impl với đúng các lifetime của impl" là `Self` (cần DefId đã giải ở `slots`, hiện chỉ có AST) — là thay đổi trong kiểm tra chữ ký trait/impl liên quan soundness; để thiết kế riêng. Né: viết `&Self`. Test hồi quy sẽ viết cùng bản sửa; hiện ghi nhận ở `docs/KNOWN_LIMITATIONS.md`.
- **Ảnh hưởng điều kiện cứng:** không (không chạm mutability bên trong, `--checks=all`, cổng ĐỎ/ICE).

## FB-4 — `u8 as char` không hạ được ở codegen (2026-10-14, gặp khi dựng `examples/wordfreq`)
- **Triệu chứng:** `rig build` ⇒ `backend llvm chưa hỗ trợ: ép kiểu Int(U8) → Char` (thoát 3, không exe); `gritc check`/thông dịch XANH và chạy đúng. Khoảng cách giữa hai backend (fail-closed, không sai ngầm).
- **Tái hiện tối thiểu:** `ƒ main() → u32 ! ∅ { c ≔ 200u8 as char; c as u32 }` (backend llvm).
- **Phân loại:** (b) codegen thiếu (không phải sai tích cực/âm tính của checker); fail-closed.
- **Nguyên nhân gốc:** `Cg::cast` chỉ nhận cặp (số nguyên/bool/char → số nguyên); đích `Char` chưa được xử lý dù M4 cho phép `u8 as char` (mọi u8 là giá trị vô hướng hợp lệ) và thông dịch đã có.
- **Sửa + test hồi quy:** `Cg::cast` nới không dấu u8 → i32 (và `cast_fits` luôn đúng cho cặp này). Test `gt-driver/tests/llvm_text.rs::{u8_as_char_lowers_and_encodes_latin1_scalars, u8_char_round_trip}` (hai backend, số tính tay 6195 và 200): ĐỎ khi bỏ bản sửa (đã thử), XANH khi có.
- **Ảnh hưởng điều kiện cứng:** không.

## FB-5 — `x as i64` từ số thực được M4 CHẤP NHẬN trái Q70 (ÂM TÍNH GIẢ, tìm thấy muộn) (2026-10-14, gặp khi dựng `f64`)
- **Triệu chứng:** `x ≔ 1.5f64; x as i64` ⇒ `gritc check` xanh (chỉ ghi chú N0604); quyết định Q70 cấm `as` số thực → nguyên (chỉ `to_i64` trả `Option`). Tương tự `f32` và `%` trên số thực: M4 chấp nhận rồi mới hỏng ở codegen.
- **Phân loại:** ÂM TÍNH GIẢ của checker M4 (trái quyết định thiết kế). An toàn bộ nhớ không bị ảnh hưởng: cast mất thông tin vẫn bị `CastLossy` bắt lúc chạy (lớp phòng thủ thứ hai).
- **Vì sao lọt qua M4/M7:** KHÔNG có mẫu nào (fail hay pass) dùng `as` từ số thực sang nguyên; `cast` ở M4 coi mọi cặp số là hợp lệ; M5/M7 chỉ sinh/chứng minh `CastLossy`, nên không bao giờ sai ngầm nhưng cũng không phản ánh quyết định cấm. Không có test cho từng ô của bảng ép kiểu.
- **Sửa + hồi quy:** E0319 ở `check/expr.rs::cast`; test `float_restrictions_are_e0319` và 4 mẫu `tests/fail/E0319_*`.

## FB-6 — `Option::unwrap` không dùng được trên `Option` chưa chứng minh `is_some` (E0902) (2026-10-14, gặp khi kiểm "dùng được thật")
- **Triệu chứng:** `ƒ f(o: Option<u64>) → u64 ! ∅ { o.unwrap() }` ⇒ ĐỎ `E0902` ("đối số không sao chép được nên không gọi được hợp đồng lúc chạy"); `⎇ o.is_some() { o.unwrap() }` ⇒ xanh.
- **Phân loại:** FAIL-CLOSED (từ chối chương trình có hợp đồng không kiểm được lúc chạy), hạn chế thư viện/thiết kế: hợp đồng `⊳ self.is_some()` của `unwrap(self)` cần đối số `Copy` để chạy lúc chạy, mà `Option<T>` không `Copy`. Không phải bug an toàn.
- **Né:** `⌥` tường minh hoặc bọc trong `⎇ o.is_some()`. Ưu tiên thấp; ghi để người viết công cụ biết (READINESS.md mục b.5).

## FB-7 — gt-term/M7: "tập nghĩa vụ RỖNG hoặc BỊ CẮT" không được phân biệt với "đã chứng minh" (2026-10-15; lệnh Giang B)
- **Giả thuyết ban đầu của lệnh:** gt-term chấp nhận vòng lặp khi M7 không sinh nghĩa vụ nào ⇒ chương trình không dừng có thể lọt. **Đo thật:** (1) với M7 hiện tại KHÔNG tìm được chương trình thật nào bị chấp nhận sai theo đường này: tập `judged` rỗng ⇔ M7 chứng minh KHÔNG có cạnh lùi khả thi tới đầu vòng (vòng chạy tối đa một lượt, ví dụ thân không bao giờ vào được: `μi ≔ 19u8; ⟳ i <= 16u8 ∧ i < 12u8 ↓ i {…}`) — chấp nhận là ĐÚNG (và là hành vi mà bộ sinh `term_oracle` dựa vào: gom rỗng ⇒ từ chối làm số chương trình chấp nhận tụt 300+ ⇒ 121 trên 2000). (2) NHƯNG sự chấp nhận đó chỉ là hệ quả NGẦM của việc không có nghĩa vụ; không có khẳng định tường minh nào từ M7. Tiêm lỗi `set_drop_variant_obligations_for_test` (nuốt mọi nghĩa vụ biến thể) với logic CŨ: gt-term chấp nhận vòng lặp ĐỨNG YÊN (`⟳ i < n ↓ n - i { }`) và bộ sinh `term_oracle` tự bắt ở chương trình #6 ("ĐƯỢC CHẤP NHẬN nhưng không dừng"). ⇒ **fail-open CÓ THẬT khi nghĩa vụ bị mất/cắt do một lỗi M7 trong tương lai**, dù hiện không có đường tự nhiên.
- **Sửa:** `BodyReport::no_back_edge_heads` (M7 KHẲNG ĐỊNH TƯỜNG MINH đầu vòng nào không có cạnh lùi khả thi); gt-term chỉ chấp nhận tập rỗng khi đầu vòng nằm trong tập đó; rỗng mà không có khẳng định ⇒ E0701/E0702. Test: `gt-term/tests/fault_dropped_obligations.rs` (đỏ với logic cũ, xanh sau sửa), `term_oracle` với `GT_TERM_DROP=1` (đỏ với logic cũ ở #6, xanh sau sửa).
- **Phát hiện thêm (hành vi thật khác giả thuyết): vượt `GHOST_MAX_VARS = 8` KHÔNG bị chấp nhận mà bị CHẶN NHẦM** — vòng lặp ĐÚNG thứ 5 trở đi trong một hàm (mỗi vòng 2 biến bóng) bị E0702/E0701 (dương tính giả, fail-closed). Đã nâng `GHOST_MAX_VARS` 8→24 (= `ZONE_MAX_VARS`; **hiệu chỉnh 2026-10-15:** bản đầu của tôi nâng lên 32 và ghi "≈ 16 vòng" là SAI — đo thật cho thấy giới hạn hiệu dụng là 24 biến = 12 vòng hai biến vì chỉ 24 biến đầu vào zone); vượt 24 vẫn bị từ chối có kiểm soát (KNOWN_LIMITATIONS).
- **Cũng sửa (fail-open phụ):** gt-term `has_back_edge` coi khối thiếu terminator là "không có cạnh lùi" ⇒ nay coi là CÓ; thân có vòng lặp mà không có chữ ký hàm ⇒ E9901 thay vì bỏ qua; gt-effects: khối thiếu terminator ⇒ E9901 (trước: bỏ qua lời gọi ⇒ mất hiệu ứng), `Mutex::with` không xác định được hàm đối số ⇒ E9901 (trước: `return` im lặng ⇒ không kiểm thứ tự khoá), impl↔trait thiếu ⇒ E9901; gt-oblig: biến bóng không biết kiểu nguyên không còn bị khai báo `u64` (giả thiết "không âm" không cơ sở) mà để khoảng `top`.
- **Vì sao lọt qua M8 và bộ sinh:** bộ sinh chỉ kiểm "chấp nhận ∧ phân kỳ" trên chương trình mà M7 chạy BÌNH THƯỜNG; không có kịch bản "nghĩa vụ bị mất/cắt". Nay `term_oracle` có chế độ `GT_TERM_DROP=1` (tiêm lỗi) và `fail_closed_loops.rs` kiểm vòng lặp vượt giới hạn.

## FB-8 — thông dịch MIR không so sánh THỨ TỰ được trên `char` (2026-10-16; phát hiện bởi bộ kiểm std)
- **Triệu chứng:** `'a' < 'b'` qua cổng xanh, exe LLVM chạy đúng, nhưng `gt-mir::interp` trả `Flow::Bug("binop Lt trên Char(97) Char(98)")` (chỉ `==`/`≠` có nhánh chung). Bộ kiểm `std_lib` (`Ord for char`, ba nguồn) lộ ra ở lần chạy đầu: 4/4 chương trình ngẫu nhiên bất đồng.
- **Phân loại:** lỗi của CÔNG CỤ ĐỐI CHIẾU (thông dịch tham chiếu), KHÔNG phải của codegen hay checker an toàn; không phải false negative. Ảnh hưởng: mọi bằng chứng "thông dịch ≡ LLVM" trước đó không phủ so sánh thứ tự `char` (corpus/bộ sinh cũ không dùng).
- **Sửa:** nhánh `(V::Char, V::Char)` trong `binop`. Test hồi quy `crates/gt-mir/tests/interp_char_compare.rs` (13 so sánh), commit ĐỎ trước rồi sửa ở commit riêng.
- **Vì sao lọt:** bộ sinh MIR/`progen`/`obgen` chỉ sinh số nguyên, bool, Option; không có `char`.

## FB-9 — hồ sơ release/release-o2 từ chối MỌI chương trình có `static Mutex` (Guard-post: `nuw` trên biểu thức hằng GEP) (2026-10-16; phát hiện khi chạy lặp H8)
- **Triệu chứng:** `gritc build --profile=release tests/samples/threads.gt` ⇒ `E1101: cờ sinh poison/UB 'nuw' trong call void @gritrt_mutex_lock(ptr getelementptr inbounds nuw (i8, ptr @"static.threads::SUM", i64 8))`. Dev bình thường.
- **Phân loại:** FAIL-CLOSED (từ chối, không chấp nhận sai) nhưng là KHOẢNG TRỐNG: bằng chứng "dev ≡ release" trước đó không bao giờ phủ chương trình có Mutex tĩnh (sáu công cụ dogfood không dùng Mutex).
- **Nguyên nhân:** các pass danh sách trắng (mem2reg/sroa/early-cse) chuẩn hoá GEP struct trên static thành offset byte kèm cờ GEP `inbounds nuw` ở BIỂU THỨC HẰNG làm toán hạng của lời gọi; luật cờ UB của Guard miễn LỆNH `getelementptr` nhưng không miễn biểu thức hằng GEP nằm trong toán hạng lệnh khác.
- **Sửa (Guard, TCB):** trước khi tách token, bỏ cụm `getelementptr inbounds nuw` / `getelementptr nuw` (cờ GEP, cùng loại đã miễn cho lệnh GEP). `add nuw`/`nsw`... vẫn bị từ chối (test `nuw_on_arithmetic_is_still_rejected`). Hồi quy: `threads_repeat.rs` dựng và chạy mẫu `threads`, `two_locks` ở release-o2 (trước sửa: build đỏ E1101; sau sửa 4×300 lần chạy đúng).
- **Chưa làm:** đột biến chính thức cho thay đổi này (chứng cứ hiện có: trước sửa đỏ, sau sửa xanh; hai phía từ chối/chấp nhận có test). Cần ghi vào lô đột biến gt-guard lần sau.

## FB-10 — Guard-post chấp nhận gọi GIÁN TIẾP hàm `gritrt_*` qua con trỏ (FAIL_OPEN_AUDIT G1) (2026-10-17)
- **Phát hiện:** khi đọc đối kháng gt-guard cho `docs/FAIL_OPEN_AUDIT.md`: bảng RUNTIME chỉ kiểm lời gọi `call @gritrt_x` TRỰC TIẾP; một địa chỉ `@gritrt_*` đưa vào store/toán hạng khác (rồi gọi gián tiếp, hoặc qua `invoke`/`callbr`) KHÔNG bị rà ⇒ bỏ qua kiểm tên/chữ ký của bảng đóng.
- **Phân loại:** FAIL-OPEN tiềm tàng trong TCB (không có chương trình gritc sinh ra thực tế; thấy từ đọc mã). Chưa có phản ví dụ chạy thật nào ngoài test dựng tay.
- **Sửa:** test hồi quy commit ĐỎ trước (`runtime_function_addresses_may_only_be_called_directly`), rồi sửa: mọi lần dùng địa chỉ `gritrt_*` phải là callee trực tiếp của `call`; `invoke`/`callbr` bị từ chối. gt-guard 28/28.
- **Còn mở:** đột biến chính thức cho quy tắc mới (chưa chạy).

## FB-11 — năm chỗ "thiếu dữ liệu ⇒ coi là ổn" tìm thấy khi rà FAIL_OPEN_AUDIT (2026-10-17)
- **Nguồn:** `docs/FAIL_OPEN_AUDIT.md` (đọc mã gt-guard, gt-secrets, gt-borrowck, gt-driver; gt-mir/gt-types chỉ rà phần quyết định — xem "CHƯA RÀ"). Cả năm đều là NGHI/TIỀM TÀNG (đọc mã), không có chương trình thật nào đã lọt.
- **S1** gt-secrets: kiểu `Secret` lồng > 12 tầng ⇒ mất taint ⇒ không S201/S202. Sửa: quá sâu ⇒ cảnh báo bảo thủ. Test đỏ trước (độ sâu 13) rồi xanh.
- **B1** gt-borrowck `push`: mã chẩn đoán sai định dạng ⇒ chẩn đoán bị nuốt. Sửa: lùi về E9901. Test đơn vị (cùng commit với sửa).
- **B2/B3** gt-borrowck `place_slots`/`relate`/`walk`/`adt_invariant`: thiếu slot region, ADT thiếu trong cơ sở kiểu, tổ hợp (phép chiếu, kiểu) lạ ⇒ nới lặng lẽ. Sửa: `defend` (ICE E9901). KHÔNG dựng được test đỏ từ mã nguồn Grit; bằng chứng = bộ đếm phòng thủ vẫn (0,0) trên oracle/corpus.
- **D1** gt-driver `mir_dump`: instance thiếu khoá hàm bị bỏ khỏi băm của attestation. Sửa: dấu `?missing:<node>`; test bất biến mọi instance có khoá.
- **Vì sao lọt:** các nhánh "thiếu" không với tới được từ chương trình hợp lệ nên không bộ sinh/đột biến nào chạm; chỉ đọc mã đối kháng thấy. Cùng loại với FB-7.

## FB-12 — bằng chứng ĐỎ→XANH bổ sung cho B1/B2/B3/D1 (FB-11) (2026-10-17; lệnh Giang mục 0 a–c)
Phương pháp: viết test chạm đúng nhánh, hoàn nguyên CHỈ chỗ sửa (checkout tệp nguồn từ commit trước bản sửa hoặc sửa tay một dòng), chạy test, ghi kết quả, áp lại bản sửa, chạy lại. Tệp nguồn khôi phục đúng (`git status` chỉ còn tệp test mới).
- **B1** (`check.rs`, test lib `push_with_a_malformed_code_still_records_a_diagnostic`, đi qua `Checker::push` THẬT): hoàn nguyên `diag_code(code)` ⇒ `Code::parse(code).ok()?` ⇒ **ĐỎ** ("push trả None ⇒ chẩn đoán bị nuốt"); bản sửa ⇒ XANH.
- **B2/B3** (`crates/gt-borrowck/tests/fail_open_ice.rs`, 4 test; sửa MỘT thân MIR thật rồi gọi `check_body` công khai; đối chứng: thân nguyên vẹn không báo gì). Hoàn nguyên `regions.rs` về trước bản sửa (4137b75): cả 4 **ĐỎ** — Deref không slot ⇒ `[]` (im lặng chấp nhận); ADT lạ trong phép gán ⇒ `[]`; ADT lạ sau `&!` ⇒ `[]`; phép chiếu `Field` trên số nguyên ⇒ `["E0401"]` (chẩn đoán SAI trên dữ liệu rác, không phải ICE). Bản sửa ⇒ 4/4 XANH, mỗi ca ra E9901.
- **D1** (`crates/gt-driver/tests/instance_keys.rs`, test `an_instance_without_a_key_changes_the_attestation_hash_instead_of_disappearing`): so `mir_dump` của (a) thiếu khoá hàm của một nút với (b) bảng instance không có nút đó. Hoàn nguyên `mir_dump` về `filter_map` cũ ⇒ **ĐỎ** (hai băm trùng nhau); bản sửa ⇒ XANH.
- **Giới hạn nói thẳng:** dữ liệu thiếu được DỰNG bằng cách sửa cấu trúc trong bộ nhớ, không phải sinh bởi gritc (Phase 1 không sinh ra). Đây chứng minh nhánh phòng thủ hoạt động, không chứng minh có đường thật dẫn tới nhánh đó.

## FB-13 — gt-mir đệ quy vô hạn khi dùng hàm kết hợp làm giá trị (`B::get`) ⇒ gritc chết (tràn ngăn xếp) (2026-10-17; tìm thấy khi làm S-E)
- **Triệu chứng:** `f: ƒ(&B) → u32 ! ∅ ≔ B::get;` ⇒ `thread 'gt-big-stack' has overflowed its stack`. `Res::Assoc` rơi xuống nhánh mặc định của `as_operand`, gọi `expr_into` → `as_rvalue` → `as_operand`… vô hạn.
- **Phân loại:** đạt tới được từ Grit; FAIL-CLOSED (không ra mã) nhưng gritc chết thay vì báo lỗi/ICE. Không sinh mã sai.
- **Sửa:** `Res::Assoc` ⇒ `Const::FnItem` qua `fn_item_const`; đường dẫn không hạ được ⇒ ICE E9901 (không đệ quy). Test đỏ trước (`gt-mir/tests/assoc_fn_value.rs`, tràn ngăn xếp), corpus `pass/assoc_fn_as_value`.

## FB-14 — `⌥` trên `u128`: literal bị kẹp, miền bị cắt ở `i128::MAX` ⇒ `⌥` thiếu nhánh được coi là vét cạn (2026-10-17; S-B)
- **Triệu chứng:** `⌥ x { 0..=200000000000000000000000000000000000000 ↦ 1u32 }` trên `x: u128` ⇒ check XANH; chạy với `x = 3·10^38` ⇒ `abort: unreachable` (mã thoát 70) ở LLVM, "unreachable" ở thông dịch.
- **Phân loại:** lỗi THẬT (false negative của E0305), đạt tới được từ Grit; hậu quả là abort có kiểm soát (không UB), nhưng check đã nói "vét cạn" sai. Cũng: `-2^127` bị đọc thành `-(i128::MAX)`, `v - 1` tràn ở `..i128::MIN`.
- **Sửa (ADR-041):** không bao giờ kẹp; cận không biểu diễn chính xác ⇒ `Opaque`; miền `u128` vô hạn (cần `_`); `i128` chính xác. Test đỏ trước (`gt-types/tests/match_int128.rs`), oracle ngẫu nhiên `match_oracle.rs` (bắt lỗi cũ khi tắt bản sửa), corpus.
- **Đánh đổi:** `⌥` u128 phủ trọn bằng khoảng kín nay bị E0305 (KNOWN_LIMITATIONS).

## FB-15 — hàm kết hợp dùng làm giá trị MẤT hiệu ứng khai báo ⇒ hàm `! ∅` thực hiện được `io`/`mem` mà không bị E0801 (2026-10-17; tìm thấy khi tổng quát hoá S-E)
- **Triệu chứng:** `ƒ g(b: &B) ! ∅ { f ≔ B::talk; f(b) }` với `B::talk ! {io}` ⇒ check XANH. Tương tự `Cap::now`, `Arena::new`… (mọi hàm kết hợp, kể cả intrinsic của prelude).
- **Nguyên nhân:** `apply_pick_types` trả `Ty::Fn(ps, ret, Vec::new())` — hiệu ứng RỖNG; gt-effects lấy hiệu ứng của `Callee::Value` từ kiểu hàm.
- **Phân loại:** lỗi THẬT, đạt tới được từ Grit, FAIL-OPEN của hệ hiệu ứng (hiệu ứng là một điều kiện cứng: `io`/`mem`/`div`/`detach`/`lock`). Chưa kiểm xem `div`/`lock` có thể lách theo cách này không (hiệu ứng của thành viên được tra nguyên vẹn nay, nên cùng sửa).
- **Sửa:** `pick_effects` tra hiệu ứng khai báo của thành viên. Test đỏ trước (`gt-effects/tests/effects.rs`), corpus `fail/E0801_assoc_fn_value_effects` + `pass/assoc_fn_value_effects`. Hệ quả hành vi: chú thích kiểu hàm `! ∅` cho một hàm kết hợp có hiệu ứng nay là E0301 (đúng).
- **Vì sao lọt:** không bộ sinh nào sinh hàm kết hợp làm giá trị (xem mục 3 của lệnh: mở rộng bộ sinh).

## FB-16 — điều kiện cứng #12: hàm không phải `main` đúc capability qua hàm nguồn (`root_fs`) truyền như GIÁ TRỊ (2026-10-18; R2c, ĐO)
- **Triệu chứng (đo bằng feature `allow-intrinsic-values`, tắt E0313):** `ƒ h(g: ƒ() → Cap<Fs> ! ∅) ! {io} { c ≔ g(); x ≔ c.read("/etc/hostname"); } ƒ main() ! {io} { h(root_fs); }` ⇒ check XANH. `h` không phải `main` nhưng đúc được `Cap<Fs>` (vô hạn lần, bỏ qua thu hẹp quyền kiểu `Cap::sub`).
- **Nguyên nhân:** `check_caps` (E0310) chỉ xét `CapUse` ghi ở nơi đường dẫn `root_*` được NHẮC; không thấy lời gọi `g()` qua tham số kiểu hàm; `main` được phép nhắc `root_fs` làm giá trị.
- **Phân loại:** lỗi THẬT, đạt tới được từ Grit, FAIL-OPEN của mô hình capability (điều kiện cứng #12). Đã đóng từ mục 2 (E0313 cho mọi intrinsic, kể cả `root_*`); R2c ĐO xác nhận lỗ hổng tồn tại trước quy tắc.
- **Bằng chứng:** `tests/fail/E0313_cap_mint_via_value_param.gt` (ĐỎ E0313 với bản thường; XANH với feature đo), `gt-types/tests/capabilities.rs::passing_a_capability_source_as_a_value_to_mint_capabilities_elsewhere_is_rejected`.

## FB-17 — `⊐ <T> Copy for W<T>` không ràng buộc `T: Copy` làm W<Arena>/W<Cap<…>>/W<Secret<…>> thành Copy (2026-10-18; R3, ĐO)
- **Triệu chứng:** `▢ W<T> { v: T }  ⊐ <T> Copy for W<T> { }  ƒ main() ! {io} { a ≔ W { v: root_fs() }; b ≔ a; c ≔ a; }` ⇒ check XANH: `Cap<Fs>` (không bao giờ Copy, D13/D28) bị nhân đôi qua vỏ bọc Copy; tương tự `W { v: Arena::new() }` (hai tay cầm cùng một Arena ⇒ reset/drop chồng). Điều kiện cứng #12 (mô hình capability) + tuyến tính Arena.
- **Nguyên nhân:** `validate_copy_impl` (gt-types/src/validate.rs) kiểm trường Copy bằng BIẾN SUY LUẬN cho tham số của ADT (`fresh_args`) và `is_copy(Ty::Infer) = true`; riêng trường kiểu `Param` còn bị `continue` với lời giải thích "ràng buộc do impl tự khai báo" mà KHÔNG kiểm có khai báo thật. Chỗ dùng (`find_impl`) thì kiểm bound nên `<T: Copy>` đúng; chỉ impl KHÔNG bound bị lọt.
- **Phân loại:** lỗi THẬT, đạt tới được từ Grit (4 dòng), fail-open; phát hiện nhờ đọc mã trong R3 (validate.rs:230) rồi dựng thí nghiệm.
- **Sửa:** trường được thay bằng ĐỐI SỐ THẬT của impl (`self_ty` args) và `is_copy` xét với ràng buộc do impl khai báo (`scope_bounds`); không khai báo `T: Copy` ⇒ E0311. Kèm: thiếu ADT/trait trong db trong validate ⇒ E9901 (trước: bỏ qua lặng lẽ); thiếu region_sig ⇒ không tương thích (trước: `true`).
- **Bằng chứng:** `tests/fail/E0311_copy_impl_param_{unbounded,nested_unbounded,option_unbounded}.gt` (ĐỎ chủ ý ở commit 36ac79d với bản cũ: 2/3 xanh sai; Option<T> đã đỏ nhờ biến suy luận chưa giải), `tests/pass/copy_impl_param_bounded.gt`. Chưa chạy: bộ sinh chương trình cho lớp này (xem R3 báo cáo).

## FB-18 — thông dịch MIR tính `u128` ≥ 2^127 như số có dấu (2026-10-18; R3, ĐO)
- **Triệu chứng (đo):** `x ≔ 340282366920938463463374607431768211455u128; x > 5u128` ⇒ thông dịch `Bool(false)` (LLVM: 1); `x / 2u128` ⇒ `Int(0)` (đúng: 2^127−1; LLVM chưa hỗ trợ chia 128 bit ⇒ báo rõ); `a + a` với `a = 10^38` bị coi là tràn (i128 `checked_add`) dù vừa u128; `CastFits` u128→i128 coi mọi giá trị vừa.
- **Nguyên nhân:** `V::Int(i128, IntTy)` lưu u128 bằng bit; `binop`/`CheckedBinaryOp`/`cast` dùng số học i128.
- **Phân loại:** lỗi THẬT của backend thông dịch (kết quả SAI LẶNG LẼ, không phải lỗi bộ kiểm tra); đạt tới bằng `gritc run --backend=interpreter`; thông dịch là chuẩn đối chiếu của nhiều bộ test nên có thể che/lộ sai lệch. Tìm thấy nhờ rà R3 gt-mir/interp.rs:996–999 (clamp `u128` bằng `i128::MAX`).
- **Sửa:** `binop_u128` (so sánh/chia/%/bão hoà/quấn theo `u128`), kiểm tràn `Checked*` theo `u128::overflowing_*`, `cast` (u128→f64, vừa/không vừa giữa u128 và i128) — test `crates/gt-mir/tests/interp_u128.rs` (ĐỎ chủ ý ở 93aa345; kỳ vọng tính độc lập bằng `u128` của Rust).
- **Chưa làm:** so sánh thông dịch ≡ LLVM cho u128 (LLVM chưa có chia 128 bit); toán tử bit/dịch u128 trong thông dịch chưa có (báo `Bug`, đóng).

## FB-19 — `GRITRT_ARENA_LIMIT` sai định dạng bị coi là "không giới hạn" (2026-10-18; R3, ĐO)
- **Triệu chứng (đo với runtime cũ, trước khi dựng lại gt_rt.lib):** `GRITRT_ARENA_LIMIT=2MiB` ⇒ exe chạy bình thường, `exit 0`, không giới hạn nào được áp dụng (`.ok().and_then(parse().ok())`).
- **Phân loại:** lỗi THẬT nhưng nhẹ (núm điều khiển tài nguyên, KHÔNG phải kiểm tra an toàn bộ nhớ): người đặt giới hạn tưởng có mà không có; fail-open của một cấu hình.
- **Sửa:** giá trị sai hoặc không phải UTF-8 ⇒ `arena_exhausted` (abort, mã thoát 70) kèm thông báo; không đặt ⇒ không giới hạn như cũ.
- **Bằng chứng:** `crates/gt-driver/tests/llvm_vec.rs::malformed_arena_limit_is_not_silently_unlimited` (4 giá trị sai + 1 đúng). Test đỏ chứng kiến qua lần chạy với `gt_rt.lib` cũ (không commit riêng vì cần dựng lại runtime release).
## Hồ sơ nhánh `claude/gallant-darwin-m251cq` (tiền tố FB-C để merge không trùng số với bản Windows)
Phát hiện bằng kiểm toán ngoài 2026-10-20/21 (Linux). Mỗi mục: test hồi quy ĐỎ TRƯỚC sửa đã được kiểm bằng cách hoàn nguyên bản sửa (bảng ở `docs/FAIL_OPEN_AUDIT.md` mục "F-01..F-08"). Chi tiết tái hiện: `audit/ANALYSIS.md`.

### FB-C1 — M6 không thấy nhiều toán hạng cùng một câu lệnh (kiểu (a), âm tính giả, NGHIÊM TRỌNG) — ĐÃ SỬA
`(b, b)`, `two(b, b)`, `[b, b]`, `P { a: b, b: b }`, `g(b, b.v)` với `b` không-Copy được phán XANH ⇒ nhân đôi `&!`/`Cap`/tay cầm Arena; native: dùng-sau-reset đọc 99 thay vì 1234. Gốc: `check_uses` đối chiếu mọi truy cập với trạng thái chưa-khởi-tạo ĐẦU câu lệnh. Sửa: bản sao cục bộ + áp move sau mỗi truy cập `Move`. Test: `gt-borrowck/tests/same_stmt.rs`, 14 mẫu `tests/fail/E0401_{same_stmt,dup}_*`. Chạm điều kiện cứng #9 (mutability bên trong): không. Còn thiếu: kịch bản `dup_operand` trong bộ sinh progen (S1).
### FB-C2 — kiểu không biết/vô hạn kích thước được chấp nhận (kiểu (a)) — ĐÃ SỬA
`[T]`/`str` theo giá trị ở tham số/trả về/trường (chỉ `let` bị chặn); kiểu chứa chính nó theo giá trị ⇒ `gritc` tràn ngăn xếp (mã 134) ở codegen. Sửa: `sized_ty` trong `elem_ty`/tham số/trả về; `contains_itself_by_value` (kể cả qua ADT generic, đa hình bung vô hạn) ⇒ E0301. Test: 9 mẫu `tests/fail/E0301_*`, 1 mẫu pass.
### FB-C3 — nguồn capability gọi qua giá trị hàm (kiểu (a), điều kiện #12) — ĐÃ SỬA
`helper(mk: ƒ() → Cap<Out>)` + `main` gọi `helper(root_out)` ⇒ `helper` đúc `Cap<Out>` tuỳ ý. Sửa gốc: lời gọi qua giá trị hàm ghi `CapUse` (E0310), có test đo riêng khi bật feature `allow-intrinsic-values` để không bị E0313 che. Bản Windows sau hợp nhất mặc định từ chối mọi intrinsic lib/core làm giá trị với E0313; test corpus chuyển ca truyền trực tiếp sang E0313, còn ca gọi nguồn qua biến cục bộ ngoài `main` giữ E0310. Đo mới và giới hạn: `docs/INTRINSIC_AS_VALUE.md`. Test: `capabilities.rs::calling_a_root_source_through_a_function_value_is_e0310`, `E0313_intrinsic_root_out_fn_value`, `E0310_call_through_local_fn_value`, pass `206_fn_value_with_cap_param`.
### FB-C4 — tên biến ở vị trí kiểu ⇒ ICE E9901 thay vì chẩn đoán (kiểu (a)/(d), fail-closed) — ĐÃ SỬA
`ƒ f(b: i32) → b` ⇒ `Ty::Err` không chẩn đoán (`collect.rs`). Sửa: E0301. Test `E0301_local_name_in_type_position`.
### FB-C5 — hàm kết hợp `Kiểu::hàm` dùng làm giá trị MẤT hiệu ứng (kiểu (a), âm tính giả) — ĐÃ SỬA (S1.iii)
`Arena::new` (`{mem}`), `S::mk` (`{io}`) truyền vào tham số thuần hoặc gọi qua biến trong hàm thuần: XANH; gọi trực tiếp: E0801. Gốc: `call.rs` nhánh đường dẫn kết hợp dựng `Ty::Fn(ps, ret, Vec::new())`. Sửa: `Pick.effects` (traits.rs) → `Ty::Fn`. Test: 4 mẫu fail (`E0801_assoc_fn_value_*`, `E0301_assoc_fn_value_*`) + 3 mẫu pass; đỏ trước ở commit `a5da840` (corpus lệch 5), xanh ở `fdce836`.
### FB-C6 — tiền điều kiện `⊳` không được kiểm khi gọi qua giá trị hàm (kiểu (a), âm tính giả) — ĐÃ SỬA (S1.ii)
`f ≔ shl; f(1u32, 40u32)` XANH (trực tiếp: E0901); native: `shl` với n ≥ 32 là poison/UB của LLVM; thông dịch: ICE "MIR không hợp lệ". Cùng `rotr`, hàm người dùng có `⊳` (chạy ra 0, ⊳ bị bỏ qua), `Option::unwrap`. Sửa (`gt-oblig/src/driver.rs`): hàm có `⊳` dùng làm giá trị ⇒ E0901 (fail-closed; cách né: bọc trong hàm không có `⊳` tự kiểm). ADR-C02. Test: 4 mẫu fail `E0901_pre_fn_value_*` + 3 pass; đỏ trước `a5c4295`, xanh `f000546`.

**Ghi chú hợp nhất Windows (2026-11-12):** do E0313 chặn intrinsic lib/core làm giá trị sớm hơn, hai mẫu intrinsic `shl` và `Option::unwrap` nay khóa E0313; E0901 giữ cho lời gọi trực tiếp và hàm Grit có hợp đồng.
### FB-C7 — `gritc` hoàn toàn không dựng được ngoài Windows / `--no-default-features` hỏng (kiểu (c)) — ĐÃ SỬA
Kiểu `Opt` thay thế khi tắt feature `llvm`; test `release_*` gắn cfg; port Linux (LLVM 20 `.so`, `libgt_rt.a`, `cc`). Windows: chưa kiểm sau sửa.
### FB-C8 — chương trình hợp lệ bị HĐH giết do ngăn xếp (kiểu (b)) — ĐÃ SỬA (giới hạn 256 MiB)
Đệ quy 2 triệu tầng có `↓`, mảng 20 MB cục bộ ⇒ SIGSEGV, `gritc run` báo mã 2. Sửa: `gritrt_start` chạy `main` trên luồng 256 MiB. Test `gt-driver/tests/stack_depth.rs`. Còn: tràn quá 256 MiB vẫn bị giết (KNOWN_LIMITATIONS).
### FB-C9 — M6: lát lấy từ `Vec` sống qua `Arena::reset` (kiểu (a), âm tính giả, dùng-sau-reset) — ĐÃ SỬA
Tồn tại TRƯỚC R6 (ĐO ở bản `53bfb4d`, `Vec::new(&!ar)`): `s ≔ v.as_slice(); ar.reset(); s[0]` ⇒ XANH. Gốc: `Rvalue::Ref` không thêm ràng buộc WF (`'a: 'b` cho mọi region bên trong kiểu `T` của `&'b T`) nên khoản vay Arena giữ trong `v` không sống tới lần dùng `s`. Sửa: `regions.rs::assign` thêm `outl(region bên trong pslots, [region của khoản vay])`. Test: `E0405_vec_view_alive_across_reset` (đỏ ở commit WIP `R6 (WIP)`, xanh sau sửa); oracle `arena_vec` TỰ bắt khi tắt sửa (`#1241`). Chạm điều kiện #9: không (không đổi mutability bên trong); chạm D27 (Arena): có — đã đóng.

### FB-C10 — ADT `Copy` generic không được coi là Copy dưới `K: Copy` trong phạm vi (kiểu (b), từ chối chương trình hợp lệ; chặn HashMap trong std) — ĐÃ SỬA
`▢ E<K> { k: K } ⊐ <K: Copy> Copy for E<K> { }` + `ƒ f<'a, K: Copy>(v: &Vec<'a, E<K>>)` ⇒ E0318 sai. Gốc: `TypeDb::find_impl` kiểm ràng buộc inline của impl với phạm vi RỖNG. Sửa: truyền `scope` (`db.rs`). Fail-closed giữ nguyên (không có `K: Copy` ⇒ vẫn E0318). Test ĐỎ trước: `traits_generics::generic_copy_adt_is_copy_under_its_scope_bounds_fb_c10` (commit riêng trước bản sửa).

### FB-C12 — thông dịch MIR không ghi được qua lát (`s[i] ← x`, `s = &!a[from..to]`) (kiểu (c), thông dịch ≠ LLVM; ICE fail-closed) — ĐÃ SỬA
Phát hiện bởi bộ kiểm std2 (`[T]::sort` trên mảng): thông dịch `Bug("ghi vào lát")` ⇒ "MIR không hợp lệ", LLVM đúng. Sửa: `interp.rs::set_at` ghi qua cửa sổ `Step::Sub` (chép cửa sổ, ghi, chép lại; kiểm độ dài). Test ĐỎ trước: `gt-mir/tests/interp_slice_write.rs`. Thông dịch nằm NGOÀI TCB (công cụ test): lỗi này là khoảng trống của công cụ đối chiếu, không phải lỗ hổng an toàn.

### FB-C13 — `==`/`≠` trên `&str`, tuple, `&T`: M4 chấp nhận nhưng backend LLVM từ chối; thông dịch so ĐỊA CHỈ cho `&T` (kiểu (b)/(c)) — ĐÃ SỬA
Phát hiện khi viết K4 (`a == "-c"` trên `&str`): cổng XANH, `rig build` ⇒ "backend llvm chưa hỗ trợ: phép hai ngôi Eq trên Ref(false, Str)" (mã 3). Thăm dò thêm: tuple cũng bị LLVM từ chối; `&2u32 == &2u32` trong thông dịch ra `false` (so địa chỉ) — thông dịch và LLVM (nếu cài) sẽ bất đồng. Sửa: (1) `&str == &str` cài ở LLVM bằng `gritrt_bytes_eq` (thuần, so BYTE; Guard + TCB row; ≥ 144 cặp × 2 backend × 3 hồ sơ; đột biến Eq↔Ne bị bắt); (2) M4 chỉ nhận `==` cho số nguyên/bool/char/`&str`; tuple và `&T` (T ≠ str) ⇒ E0301 ("chưa hỗ trợ", dùng `*a == *b`). Hai test cũ (`equality_is_limited_to_comparable_types`, `…floats…d4`) từng khẳng định tuple `==` hợp lệ: ĐÃ SỬA có ghi chú (thu hẹp bề mặt ngôn ngữ, fail-closed). Test ĐỎ trước: `gt-driver/tests/str_eq.rs`, `gt-types/tests/traits_generics.rs::equality_is_limited_to_scalars_and_str_fb_c13`.

### FB-C14 — `as` từ số thực suy diễn sang số nguyên lọt qua M4 (kiểu (a), cấm D1/Q70 bị lách; fail-closed ở backend) — ĐÃ SỬA
Phát hiện bởi người viết LANGUAGE_REFERENCE khi thử: `x ≔ 1.5; x as i32` ⇒ gritc check XANH (chỉ N0604), `gritc run` thoát 3 "backend llvm chưa hỗ trợ: kiểm ép kiểu Float → Int". `x: f64`/`1.5f64` thì E0319 đúng. Gốc: `cast()` so `Ty::Float(_)` sau `shallow`, bỏ sót biến suy diễn số thực. Sửa: dùng `is_float`. Test ĐỎ trước: `traits_generics::float_to_int_cast_through_an_inferred_float_literal_is_e0319_fb_c14`.

### FB-C15 — đệ quy qua `[f; N]` (`Rvalue::Repeat`) không bị E0703 (kiểu (a), âm tính giả; gt-term) — ĐÃ SỬA
Phát hiện bởi đọc mã đối kháng nhóm gt-term (S7). Gốc: tập toán hạng "hàm thoát làm giá trị" (đồ thị instance và tập escaped) bỏ sót `Rvalue::Repeat`. Sửa: `gt-term/src/lib.rs` thêm `Repeat(o,_)` ở cả hai chỗ. Test ĐỎ trước: `gt-term/tests/instance_graph.rs` (commit `7312474`), xanh `de29fc8`.

### FB-C16 — `⊐ <T> Copy for W<T>` không đòi `T: Copy` (kiểu (a), âm tính giả; chạm bất biến Copy ⇒ nhân đôi tài nguyên) — ĐÃ SỬA
`validate_copy_impl` kiểm trường với tham số kiểu chưa thế và phạm vi rỗng. Sửa: thế đối số của `imp.self_ty`, dựng `scope` từ ràng buộc của impl, kiểm từng trường (E0311). Test ĐỎ trước: `traits_generics.rs` (commit `fec1768`), xanh `8ba2da0`.

### FB-C17 — `Copy` cho `Arena/Mutex/Atomic/Cell`, và `Cell::get` không đòi `T: Copy` (kiểu (a)) — ĐÃ SỬA
Cùng commit với FB-C16: `validate_copy_impl` cấm Copy cho `Lang::{Secret,Cap,Vec,Text,Arena,Mutex,Atomic,Cell}` (E0313); prelude tách `Cell::get` sang `impl<T: Copy>`. Hệ quả: băm attestation của prelude đổi (golden `tests/golden_ll` phải bless lại ở lần chốt).

### FB-C18 — `main` có tham số ở tệp khác được miễn E0310; `Cap` bọc trong struct không bị coi là capability (kiểu (a), điều kiện #12) — ĐÃ SỬA
Sửa: `entry` đòi `params.is_empty()` (như `find_main`); `has_cap` đệ quy vào trường struct/enum (có tập `seen` chống vòng). Test ĐỎ trước commit `9711062`, xanh `0b7e60e`.

### FB-C19 — `==` trên biến suy diễn CHƯA giải lúc gặp toán tử lọt qua cấm float (D4, E0303) và cấm tuple (kiểu (a)) — ĐÃ SỬA
`x ≔ None; … v == v …; x ← Some(1.5)` XANH (`eq_ok(Infer) = true`, `is_float` thấy biến chung). Sửa: `Checker.eq_deferred` ghi (node, span, kiểu) khi toán hạng là biến suy diễn chưa biết; `finish` kiểm lại bằng `check_eq_operand` sau `default_literals`. Test ĐỎ trước `1a5d2ae`, xanh `f0188d2`.

### FB-C20 — kiểm biên lát/chỉ số dùng độ dài CŨ khi `s` bị gán lại trước lúc dùng chỗ ⇒ đọc/ghi ngoài mảng ở LLVM (kiểu (a), NGHIÊM TRỌNG: an toàn bộ nhớ) — ĐÃ SỬA
Phát hiện bởi đọc mã đối kháng nhóm C (S7): `μs ≔ &big[0..4096]; t ≔ &s[0..{ s ← &small[0..2]; 2000 }]` ⇒ XANH, native đọc 2000 byte từ mảng 2 phần tử (`2000000013989`); cùng dạng GHI qua `&!` (`victim` bị đè), gán `s[50] ← { s ← …; 9 }`, người nhận `s[50].get({ s ← …; 1 })`. Thông dịch: "MIR không hợp lệ". Gốc (`gt-mir/src/build/expr.rs`): (1) lát tính `Len` TRƯỚC khi đánh giá đầu mút `to`; (2) chỗ `(*s)[i]` đọc `s` ở lúc DÙNG, còn kiểm biên làm lúc DỰNG chỗ — giữa hai thời điểm (giá trị vế phải của gán, đối số của lời gọi phương thức, chỉ số của chỉ số lồng) có thể đổi `s`. Sửa: `index_place` đánh giá `from`/`to` rồi mới đọc `Len`; `recheck_place` phát lại `Len`+`Assert` (dùng lại tạm chỉ số, KHÔNG đánh giá lại biểu thức) cho mọi `Index`/`Subslice` của chỗ ngay trước chỗ dùng bị hoãn (gán, người nhận phương thức, tiền tố của chỉ số không "chỉ đọc"). Chỉ phát khi biểu thức đứng giữa KHÔNG `is_read_only` ⇒ không tốn gì cho mã thường. Test ĐỎ trước: `gt-driver/tests/slice_stale_len.rs` (5 ca × llvm dev/release/release-o2 + thông dịch; commit riêng). Golden `aggregates.mir` đổi thứ tự `Len` (chủ đích) và 3 golden IR bless lại (băm attestation đổi do FB-C17 + sửa này; giữ dòng `target` Windows).

### FB-C21 — thứ tự khởi tạo `■` chỉ theo phụ thuộc TRỰC TIẾP: static đọc qua hàm (kể cả giá trị hàm) bị đọc khi còn ZERO-INIT (kiểu (a), NGHIÊM TRỌNG: gọi con trỏ hàm NULL từ chương trình "xanh") — ĐÃ SỬA
Phát hiện bởi đọc mã đối kháng nhóm D (D10): `■ A ≔ f(); ■ Z ≔ 9; ƒ f() { Z + 1 }` ⇒ A = 1 (đúng: 10); `■ A ≔ run(); ■ F: ƒ(u32)→u32 ≔ id; ƒ run() { F(41) }` ⇒ SIGSEGV (gọi con trỏ NULL); luồng sinh trong initializer thấy Z = 0. Gốc (`lower.rs`): `static_refs` chỉ thu thập static nhắc trong thân initializer. Sửa: codegen ghi cạnh instance→instance (mọi `get_fn`: gọi trực tiếp, giá trị hàm, `spawn`) và instance→static; thứ tự = DFS hậu thứ tự trên BAO ĐÓNG BẮC CẦU; vòng phụ thuộc (kể cả static đọc chính nó qua hàm) ⇒ `Unsupported` (mã 3, KHÔNG chạy). Bảo thủ: một hàm chỉ NHẮC tới `g` mà không gọi cũng tạo cạnh (chỉ làm thêm ràng buộc thứ tự/vòng giả, không bao giờ bỏ sót). Test ĐỎ trước: `gt-driver/tests/static_init_order.rs` (4 ca × 3 hồ sơ).

### FB-C22 — ordering của `Atomic` là local bị mượn `&!` (kiểu (a); lệch bất biến TCB "ordering đúng như mã nguồn") — ĐÃ SỬA
`μo ≔ Relaxed; weaken(&!o); A.store(7, o)` ⇒ M8 và codegen coi là Relaxed (IR `monotonic`) dù lúc chạy `o` = SeqCst. Sửa: `gt-conc::variant_of` và `prescan` của codegen coi local bị `&!` (hay thành phần của nó) là KHÔNG truy ngược được ⇒ E0503. Test ĐỎ trước: `tests/fail/E0503_ordering_mut_borrowed{,_copy}.gt`.

### FB-C23 — `alloca` của `io.res`/`cas.res`/`rep.i` đặt tại chỗ gọi ⇒ cấp ngăn xếp mỗi vòng lặp (kiểu (b), chương trình hợp lệ chết SIGSEGV) — ĐÃ SỬA
40 triệu lần `fs.open("/không-tồn-tại")` trong vòng lặp có `↓` ⇒ SIGSEGV (exit 139) ở dev và release-o2. Sửa: `entry_alloca` (khối đầu hàm). Test ĐỎ trước: `gt-driver/tests/alloca_entry.rs` (mọi `alloca` của IR ở khối entry; dev + release-o2).

### FB-C24 — `imut` coi ADT lạ là "không có mutability bên trong" (kiểu (a), fail-open phòng thủ) — ĐÃ SỬA
`db.rs::imut` trả `false` khi ADT không có trong db ⇒ vị từ trên kiểu đó bị coi là sự kiện ổn định ở M7. Sửa: trả `true` (bảo thủ). Test ĐỎ trước: `db::imut_tests` (không có đường thật dẫn tới đây; chứng minh nhánh phòng thủ).

### FB-C25 — `⊳` của thành viên trait KHÔNG được kiểm khi gọi qua tham số kiểu (kiểu (a), âm tính giả; trái tài liệu "mỗi lời gọi trực tiếp vẫn bị kiểm") — ĐÃ SỬA
`⊏ Tr { ƒ f(&self, n: i32) → i32 ! ∅ ⊳ n > 0; } … ƒ g<T: Tr>(t: &T) { t.f(0) }` ⇒ XANH, chạy ra 0 (gọi trực tiếp `s.f(0)` thì E0901). Hợp đồng ở khai báo trait (không thân) không hề được lowering; hợp đồng ở impl không được áp tại điểm gọi `Callee::TraitMethod`. Sửa (fail-closed, `gt-oblig/src/driver.rs`): lời gọi `Callee::TraitMethod` tới thành viên có `⊳`/`⊲` (ở trait HOẶC ở bất kỳ impl nào) ⇒ E0901 (cách né: gọi trên kiểu cụ thể, hoặc cho thân tự kiểm điều kiện). `FnSigInfo.has_contracts` mới. Test ĐỎ trước: `tests/fail/E0901_pre_trait_method_{via_generic,impl_via_generic}.gt`.

### FB-C26 — nguyên tử "nghĩa vụ không biểu diễn được" tràn sang dải nguyên tử vị từ khi hàm ≥ 1024 khối (kiểu (a), điều kiện #… M7; CHỈ `--checks=proven`) — ĐÃ SỬA (hardening, chưa có PoC)
`unrep_atom = UNREP_BASE + bb*1024 + idx.min(1023)`, `UNREP_BASE + 1024*1024 = PRED_BASE` ⇒ ở khối ≥ 1024 nguyên tử có thể trùng nguyên tử vị từ thuần (pi, local) đã được giả định đúng ⇒ một `⊳` không biểu diễn được bị "chứng minh" sai. 33 cỡ hàm thử bởi người đọc đối kháng KHÔNG tái hiện được; bằng chứng duy nhất là số học (xem `unrep_tests`). Sửa: `unrep_atom_id(bb, idx)` đơn ánh, `UNREP_SLOTS = 32` ô/khối, nằm trọn trong `[UNREP_BASE, PRED_BASE)`; thân vượt `UNREP_MAX_BLOCKS` (32768 khối) ⇒ `fixpoint` từ chối (giữ MỌI check). Test: `analysis::unrep_tests`.

### FB-C27 — IR Guard: địa chỉ của khai báo ngoài không phải `gritrt_*` (vd `@system`) có thể lưu rồi gọi gián tiếp (kiểu (a), phòng thủ; cần lỗi codegen khác để có IR như vậy) — ĐÃ SỬA
Luật "địa chỉ chỉ làm đích gọi trực tiếp" chỉ áp cho `gritrt_*`. Sửa: áp cho MỌI hàm chỉ khai báo trừ `llvm.*`. Test ĐỎ trước: `gt-guard/tests/guard.rs::external_declarations_may_only_be_called_directly_fb_c27`. Không có chương trình Grit nào sinh được IR như vậy (Guard là lớp thứ hai).

### FB-C28 — đường dẫn rỗng `null + 0` thành Rust slice không hợp lệ trong I/O (UB ở ranh giới FFI) — ĐÃ SỬA
`Text::new(&arena).as_str()` có thể là `{ptr: null, len: 0}`; `gritrt_fs_read/open/write` trước đây gọi `slice::from_raw_parts` trực tiếp. Rust yêu cầu con trỏ khác null cả khi độ dài bằng 0. **Đo đỏ trước sửa:** unit test gọi `gritrt_fs_open(null, 0)` dừng ở thông báo tiền điều kiện `slice::from_raw_parts requires the pointer to be aligned and non-null` (Windows status `0xc0000409`). Sửa: dùng `ffi_bytes`, trả `&[]` cho độ dài 0 và từ chối độ dài không thể biểu diễn thành Rust slice; cả ba đường dẫn và dữ liệu ghi cùng dùng helper. Test `gt-rt::tests::fs_empty_paths_accept_null_zero_ffi_slices` xanh sau sửa; kiểm nguồn cho thấy Grit truyền nguyên cặp `{ptr,len}`.

### FB-C29 — worker `spawn` dùng stack mặc định quá nhỏ cho chương trình hợp lệ — ĐÃ SỬA
`gritrt_spawn` trước đây dùng `std::thread::Builder::spawn` không đặt stack; chương trình D14 có mảng cục bộ 3 MB trong worker bị kết thúc bất thường trên Windows (trả 101 qua `gritc run`). Sửa: `main` và mọi worker dùng chung `GRIT_THREAD_STACK_BYTES = 256 MiB`. Test `gt-driver/tests/stack_depth.rs::spawned_worker_large_local_array_runs_natively` đỏ trước sửa (101), xanh sau sửa (main trả 1). Giới hạn còn lại: 256 MiB mỗi thread được reserve; quá nhiều worker có thể làm cạn tài nguyên, và một worker cần hơn 256 MiB vẫn có thể bị HĐH giết.

### FB-C30 — IR Guard cho phép mọi `llvm.*`, gồm intrinsic có thể phát poison — ĐÃ SỬA
Guard coi mọi tên `llvm.*` là hợp lệ nếu LLVM verifier nhận lời gọi. `llvm.abs.i32(x, true)` là lời gọi verifier chấp nhận nhưng cờ `is_int_min_poison` có thể tạo poison. **Đo đỏ trước sửa:** `rejects_non_allowlisted_llvm_intrinsic_with_poison_flag` nhận danh sách vi phạm rỗng `[]`. Sửa: allowlist đóng chỉ gồm các intrinsic/họ độ rộng codegen hiện phát; `llvm.assume` và FMA vẫn bị luật chuyên biệt từ chối. Test đỏ→xanh; `cargo test -p gt-guard --test guard -- --test-threads=2` xanh 35/35. Đây là hardening của lớp Guard; không tìm thấy chương trình Grit hiện hành phát `llvm.abs`.

## FB-C31 — gt-rt: bốn đường ABI còn gọi `slice::from_raw_parts` với độ dài > isize::MAX (2026-11-17; R11b, Claude đọc diff FB-C28)
- **Phát hiện bởi:** đọc diff `1c7deef` (FB-C28: `ffi_bytes` chỉ áp cho `gritrt_fs_read/open/write`): `grep from_raw_parts` ⇒ `gritrt_write_str`, `gritrt_ewrite_str`, `gritrt_utf8_valid`, `gritrt_bytes_eq` vẫn dựng lát bằng `usize::try_from(n).unwrap_or(0)` (trên 64-bit không bao giờ lỗi) rồi `from_raw_parts` — tiền điều kiện Rust "độ dài ≤ isize::MAX" bị vi phạm nếu `n` lớn (UB).
- **Phân loại (nói thẳng):** SIẾT PHÒNG THỦ NHẤT QUÁN, KHÔNG có đường tới được từ Grit (độ dài lát/`&str` bị chặn bởi kích thước cấp phát nên không bao giờ > isize::MAX). Không phải lỗi an toàn bộ nhớ đạt tới từ chương trình Grit; là rủi ro của runtime khi codegen sai.
- **Test ĐỎ trước (commit `262008e`):** `gt-rt::tests::oversized_ffi_lengths_are_rejected_before_from_raw_parts` — bản debug: "unsafe precondition(s) violated: slice::from_raw_parts" ⇒ `panic = "abort"` ⇒ tiến trình test chết `STATUS_STACK_BUFFER_OVERRUN`.
- **Sửa:** bốn đường dùng `ffi_bytes`; `write_str`/`ewrite_str` ⇒ `abort(14)` (io_error); `utf8_valid` ⇒ 0 (không hợp lệ); `bytes_eq` ⇒ 0 (khác). `cargo test --manifest-path runtime/gt-rt/Cargo.toml` 11/0, clippy `-D warnings` sạch.

## FB-C32 — NGHIÊM TRỌNG (âm tính giả): vòng lặp KHÔNG dừng được chứng minh "dừng" khi hàm vượt 24 biến bóng (2026-11-19, GÓI 7 S2.a) — ĐÃ SỬA
- **Phát hiện bởi:** truy vấn "gt-term:908 vì sao 30 biến không bị từ chối" của báo cáo GÓI 6: đọc `crates/gt-oblig/src/analysis.rs` (cấp biến bóng `ghost_count < GHOST_MAX_VARS`, `judge_variant_edge`) rồi dựng thí nghiệm.
- **Cơ chế:** mỗi vòng có biến thể được cấp biến bóng `ghost(v)` chụp giá trị `v` ở đầu vòng; nghĩa vụ ở cạnh lùi là `biến_thể(v) < biến_thể(ghost(v))`. Chỉ 24 biến bóng mỗi hàm; vòng sau đó KHÔNG có biến bóng riêng, nhưng `ghost(v)` CŨ (từ vòng trước dùng cùng `v`) vẫn nằm trong trạng thái và `judge_variant_edge` vẫn thay `v → ghost(v)`. Nếu `v` đã giảm chặt sau vòng trước (`y ← y - 1`), `y < ghost(y)` đúng và vòng cuối (không đổi gì) được "chứng minh" dừng.
- **Tái hiện (xanh trước sửa):** `tests/fail/E0702_ghost_limit_stale_ghost.gt` (31 dòng thu nhỏ bằng script xoá-từng-dòng, giữ điều kiện "xanh và thông dịch không dừng sau 4 s"): vòng đầu `⟳ k0 < 5 ↓ y + 5 - k0`, 22 vòng lót mỗi vòng một biến, `y ← y - 1`, `⟳ z < 5 ↓ y { }` (z không đổi ⇒ không dừng). `gritc check` ⇒ green; `gritc run --backend=interpreter` ⇒ không dừng (timeout 10 s, mã 124). 21 vòng lót ⇒ E0702 (đúng), 22 ⇒ GREEN (sai), 23+ ⇒ E0702 (vòng lót thứ 23 hết bóng).
- **Test ĐỎ trước (commit `48f9fd6`):** `crates/gt-term/tests/ghost_limit_soundness.rs` (2 test) và mẫu corpus `tests/fail/E0702_ghost_limit_stale_ghost.gt`.
- **Sửa (fail-closed):** `judge_variant_edge`: nếu BẤT KỲ biến của biến thể không có biến bóng RIÊNG tại đầu vòng đó thì mục tiêu thành "không biểu diễn được" (Unknown ⇒ E0702), không bao giờ dùng bóng cũ. Hậu quả: không mất chương trình đúng nào mà trước đó được chấp nhận ĐÚNG (vòng thiếu bóng trước đây vốn bị từ chối, trừ trường hợp lỗi này); kết quả của `limit_hint` (25/26 vòng ⇒ E0702) giữ nguyên.
- **Mở rộng oracle:** `crates/gt-term/tests/ghost_wide_oracle.rs` (hàm 8–32 vòng, y dùng chung, giảm y chặt rồi vòng đứng yên). Lần đầu viết RỖNG NGHĨA (400/400 chương trình phân kỳ bị bác; sau đó 0 chương trình được chấp nhận); chỉnh tới khi có cả ba nhóm (chấp nhận+dừng 8, bác+dừng 43, bác+phân kỳ 99 trong 150). **Tắt bản sửa** (`filter(|_| true || all_ghosted)`): oracle ĐỎ ở seed #139 "ĐƯỢC CHẤP NHẬN nhưng không dừng"; bật lại: xanh.
- **Lý do lọt:** bộ sinh `loopgen` chỉ tạo 1–2 vòng nên không bao giờ chạm giới hạn 24; LIMITS_AUDIT (GÓI 6) chỉ thử 24/25/26 vòng *đúng* ("giới hạn ⇒ từ chối oan") mà không thử vòng SAI sau khi hết bóng. Bài học: mỗi giới hạn số phải được thử bằng chương trình SAI vượt giới hạn, không chỉ chương trình đúng.

## FB-C33 — NGHIÊM TRỌNG (âm tính giả, D12): lời gọi đệ quy qua `spawn` không được xét trong kiểm dừng — ĐÃ SỬA (2026-11-19, phát hiện bởi bộ tấn công `attack/`; sửa ở GÓI 7)
- **Hiện tượng:** `ƒ f(n: u64) → () ! ∅ ↓ n { spawn(f, n); }` (biến thể KHÔNG giảm) và `ƒ a(n) → () ! ∅ { spawn(b, n); }` + `ƒ b(n) → () ! ∅ { a(n); }` (không `↓`/`div`) đều được `gritc check` chấp nhận ("phán quyết green"). Cùng cấu trúc gọi trực tiếp, hoặc qua giá trị hàm, đều bị E0702/E0703 (`attack/a03`, `a13`, `b02`).
- **Vi phạm:** D12 (mọi đệ quy phải có `↓` hoặc `div`; đệ quy tương hỗ cần `↓` cho cả SCC); M8 (kiểm giảm nghiêm ngặt).
- **Nguyên nhân (ĐÃ XÁC NHẬN ở GÓI 7):** `crates/gt-term/src/lib.rs` ~680–711 chỉ thêm FnItem làm arg của Call vào tập `escaped`; đồ thị theo instance chỉ nối cạnh tới callee CÓ THÂN. `spawn` là intrinsic bodiless nên cạnh "người gọi → worker" không được dựng.
- **Hệ quả:** chương trình không dừng được bảo đảm; mỗi lượt spawn để lại một luồng 256 MiB, có thể dẫn tới `spawn_failed` (abort 70) khi cạn tài nguyên. Không phải UB bộ nhớ, nhưng là hứa "đã chứng minh dừng" sai.
- **Mức:** âm tính giả của kiểm an toàn ⇒ nghiêm trọng nhất theo grit-contract §3.3.
- **Việc cần làm (kế hoạch của phiên kiểm toán; trạng thái ở mục "Xử lý GÓI 7" dưới):** (1) test ĐỎ: đưa `c01`, `c02`, `a02` vào `tests/fail/` với `E0702`/`E0703` (commit riêng); (2) sửa: coi arg FnItem của `spawn` là cạnh gọi tới worker; (3) mở rộng bộ sinh loopgen/term_oracle có `spawn`; (4) kiểm bằng cách tắt bản sửa.
- **Nghi vấn đi kèm (chưa khẳng định):** `attack/b01_spawn_in_pure_fn.gt` — hàm `! ∅` gọi `spawn` vẫn GREEN; TCB_INTRINSICS ghi spawn có hiệu ứng `detach`, M8 ghi hiệu ứng = worker trừ `lock`. Cần Giang chốt `detach` có bị kiểm hay không.
- **Xử lý GÓI 7 (2026-11-19):** tái hiện độc lập (`attack/a02` ⇒ GREEN trước sửa). **Test ĐỎ trước (commit `f79296e`):** `crates/gt-term/tests/spawn_recursion.rs` (3 chương trình phải bị từ chối + đối chứng worker không đệ quy vẫn xanh) và 3 mẫu `tests/fail/E0703_spawn_*.gt`. **Sửa:** `crates/gt-term/src/lib.rs` `instance_graph` và `conservative_graph`: callee KHÔNG có thân (intrinsic như `spawn`) ⇒ thêm cạnh GIÁN TIẾP tới mọi hàm `FnItem` trong đối số; chu trình qua cạnh này không có `div` ⇒ E0703 (cả khi có `↓`, vì không kiểm được biến thể qua cạnh gián tiếp — fail-closed). Phán quyết đổi: chỉ chương trình đệ quy qua `spawn`/intrinsic nhận hàm. **CHƯA làm:** mở rộng bộ sinh `loopgen`/`term_oracle` với `spawn` (thông dịch MIR không chạy luồng thật: CHƯA KIỂM bằng oracle); nghi vấn `b01` (hàm `! ∅` gọi `spawn`) là câu hỏi thiết kế cho Giang (`docs/PENDING_GIANG.md`).

## FB-C34 — NGHIÊM TRỌNG (âm tính giả, D12): đệ quy qua `spawn` khi hàm được truyền qua một lời gọi generic — ĐÃ SỬA (2026-11-19, GÓI 8 S1b)
- **Phát hiện bởi:** bộ sinh spawn-đệ quy (`tests/spawn_recursion.rs::spawn_family_recursion_without_variant_is_rejected`) khi thêm hình dạng `ƒ f(n) → () ! ∅ { spawn(id(f), n); }` với `ƒ id<T>(x: T) → T ! ∅ { x }`: CHẤP NHẬN (XANH).
- **Cơ chế:** sau FB-C33, cạnh gián tiếp chỉ được thêm khi ĐỐI SỐ của `spawn` là hằng `FnItem` trực tiếp. Ở đây đối số là KẾT QUẢ của lời gọi `id(f)` (biến cục bộ), nên không có cạnh f→f; `f` vẫn bị dùng làm đối số của `id` nhưng `id` không có cạnh tới `spawn`/`f`.
- **Tái hiện (đã chạy `gritc check`):** `ƒ id<T>(x: T) → T ! ∅ { x }` + `ƒ f(n: u64) → () ! ∅ { spawn(id(f), n); }` + `ƒ main() → i32 ! ∅ { f(1u64); 0 }` ⇒ GREEN trước sửa (mỗi lượt để lại một luồng 256 MiB; không chạy).
- **Test ĐỎ trước (commit `a994da9`):** `spawn_family_recursion_without_variant_is_rejected` (4 hình dạng, 4 độ sâu đệm; chấp nhận sai đầu tiên: hình dạng 3, độ sâu 0–3).
- **Sửa (fail-closed):** `crates/gt-term/src/lib.rs` `instance_graph`: callee KHÔNG có thân nhận đối số không phải hằng ⇒ `None` (lùi về đồ thị bảo thủ, không bao giờ bỏ cạnh); `conservative_graph`: callee không có thân ⇒ cạnh gián tiếp tới MỌI hàm bị thoát.
- **Phạm vi:** chỉ ảnh hưởng chương trình có lời gọi intrinsic/không thân với đối số không phải hằng VÀ có hàm bị thoát trong cùng chu trình. Không đổi phán quyết chương trình không đệ quy.
- **CHƯA:** oracle thông dịch với `spawn` (bộ sinh chỉ kiểm phán quyết, không chạy); kiểm bằng cách tắt bản sửa đang làm bằng tay (xem RUN_REPORTS GÓI 8).
