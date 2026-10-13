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

