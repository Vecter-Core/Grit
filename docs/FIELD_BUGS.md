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
