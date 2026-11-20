# COVERAGE_GOI12 — độ phủ nhánh chạy MỘT lần trong WSL (GÓI 12 P6; 2026-11-21)

Nền: WSL2 Ubuntu 24.04.5, clone `~/grit_cov` tại `d2dfd66` (mã sản phẩm rc8), `cargo-llvm-cov 0.9.2`, nightly-2026-10-04 + `llvm-tools-preview` (cấp người dùng), `bash tools/coverage.sh <crate>` từng crate, `COV_TOOLCHAIN=nightly-2026-10-04`. `gt-term` chạy với `GT_TERM_PROGRAMS=150 GT_TERM_RUNS=3` (biến thể nhanh của `term_oracle`; bản đầy đủ quá 15 phút khi đo độ phủ). Phân tích: `tools/coverage_branches.py <json>` (nhánh trong `tools/coverage_targets.txt`; chỉ tính tệp NGUỒN CỦA CHÍNH crate). KHÔNG sửa mã để tăng phần trăm.

| crate | lệnh | kết quả | nhánh chưa chạy (tệp của crate, đo) |
|---|---|---|---|
| gt-guard | coverage.sh | rc=0 | 1 |
| gt-effects | coverage.sh | rc=0 | 6 |
| gt-types | coverage.sh | rc=0 | 41 |
| gt-borrowck | coverage.sh | rc=0 | 5 |
| gt-term | coverage.sh (biến thể nhanh) | rc=0 | 17 |
| gt-oblig | coverage.sh | **rc=124: hết 14 phút (timeout 840 s)** | **CHƯA ĐO** |

- Tổng 70 nhánh chưa chạy ở 5 crate đã đo. **Không so được với mốc 90 nhánh** của GÓI 6: mốc đó gồm gt-oblig và khác cách chạy (gói thử nghiệm đầy đủ so với từng crate riêng); số mới chỉ đếm test của CHÍNH crate đó (test ở gt-driver chạm gt-types/gt-oblig… KHÔNG được tính) nên chắc chắn cao hơn thực tế. Hạn chế đã biết của phép đo.
- **Nhóm D đã thí nghiệm lại bằng chương trình thật** (gritc rc7 dev, Windows; chạm = quan sát đường tới, không phải số đếm độ phủ): `pat.rs:222` hướng false (pattern khoảng trên biến suy kiểu KHÔNG phải Int): `x ≔ 1.5; ⌥ x { 0.0..=2.0 ↦ 1, _ ↦ 0 }` ⇒ **E0301 đỏ đúng** ⇒ thêm `tests/fail/E0301_range_pat_float_infer.gt`. Số đếm độ phủ MỚI cho dòng này: CHƯA đo lại (cần chạy coverage lần nữa).
- **Còn chưa dựng được chương trình chạm tới (nhóm D, vẫn CHƯA):** `pat.rs:347` (tuple pattern trên kiểu Infer không `..`; `⌥ (1, 2) { (a, b) ↦ a }` có kiểu đã biết nên không chạm), `call.rs:13` hướng false (đối số Path tới thứ không phải hàm cho tham số `{poly}`; `poly` chỉ ở prelude). `pat.rs:473` (trường riêng tư khác module) và `call.rs:9/33`: KHÔNG còn nằm trong danh sách chưa chạy ở lần đo này (đã được test hiện có chạm; nguyên nhân chưa tra) ⇒ không cần thí nghiệm mới. Các nhánh nhóm C (ICE phòng thủ, "ĐỌC MÃ"): không đổi.
- 14 nhánh nhóm D ban đầu và 9 nhánh "đã thử không chạm" của `DECISION_COVERAGE_BRANCHES.md`: **chỉ 1 nhánh (pat.rs:222 false) được thí nghiệm mới trong GÓI 12; phần còn lại CHƯA LÀM** (cần thời gian dựng chương trình riêng từng nhánh).
- Dữ liệu thô (ngoài kho): `D:\Grit_export\cov\*.json` và `*_br.tsv`.
