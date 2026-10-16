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
