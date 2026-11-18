# DECISION_COVERAGE — độ phủ NHÁNH của các hàm quyết định (GÓI 5 S3, 2026-11-19)

**Trạng thái: ĐÃ CHẠY trên Linux CI (2 lần).** Công cụ: `cargo +nightly llvm-cov --branch` (rustc nightly, thành phần `llvm-tools-preview` của rustup — KHÔNG phải LLVM 20.1.8 của gritc). Job `coverage` của `.github/workflows/ci.yml` (`workflow_dispatch only=coverage`, ≈ 55 phút/lần): chạy MỌI test (corpus, oracle, đơn vị) của `gt-oblig gt-borrowck gt-types gt-term gt-effects gt-guard gt-driver`. Báo cáo: `python tools/coverage_report.py cov.json.gz` với danh sách hàm ở `tools/coverage_targets.txt`; dữ liệu từng nhánh/vùng chưa chạy: `docs/DECISION_COVERAGE_DATA.md`.

**Đọc đúng:** đây là độ phủ của MÃ TRÌNH BIÊN DỊCH (Rust) khi chạy bộ test, không phải của chương trình Grit. "Nhánh" = một hướng true/false của điều kiện (`if`, `&&`/`||`, `if let`, guard); llvm-cov KHÔNG đếm riêng từng cánh `match` (nhưng cánh chưa chạy hiện ở mục "vùng mã" trong tệp dữ liệu: 100 nhánh + ≈ 363 vùng chưa chạy ở lần 2). Phủ 100 % nhánh KHÔNG chứng minh hàm đúng.

## Số liệu thật (CI run 44285398006 = lần 1, 44294890309 = lần 2 sau khi thêm 8 mẫu corpus)
* Tổng nhánh (true+false) trong 35 hàm/tệp quyết định: **730**. Chưa chạy: lần 1 = **104**, lần 2 = **100** (thêm 8 mẫu corpus phủ được 4 nhánh: `pat.rs` 404, 456, 463 và `validate.rs` 184).
* Số thí nghiệm: 8 mẫu corpus (7 `fail`, 1 `pass`; `tests/*/*_cov.gt`) + 1 chương trình dò trực tiếp (`spawn` với kiểu mảng lồng 70 tầng ⇒ ĐỎ E0312+E0501 "kiểu lồng quá sâu để kiểm": fail-closed, nhánh `send.rs` `depth > 64`; KHÔNG đo lại độ phủ cho nó) + 1 thử trượt (xem E5). **FB-C mới: 0** (không thí nghiệm nào cho thấy chương trình sai bị chấp nhận hoặc check bị xoá nhầm).

## Bảng theo hàm (nhánh = true+false)
| tệp | hàm | nhánh | đã chạy | chưa chạy |
|---|---|---:|---:|---:|
| gt-oblig/src/analysis.rs | judge | 0 | 0 | 0 |
| gt-oblig/src/analysis.rs | judge_core | 2 | 2 | 0 |
| gt-oblig/src/analysis.rs | judge_ob | 0 | 0 | 0 |
| gt-oblig/src/analysis.rs | judge_pres | 0 | 0 | 0 |
| gt-oblig/src/analysis.rs | judge_posts | 36 | 35 | 1 |
| gt-oblig/src/analysis.rs | hyps_for | 14 | 12 | 2 |
| gt-oblig/src/analysis.rs | assign_local | 42 | 41 | 1 |
| gt-oblig/src/analysis.rs | fixpoint | 14 | 11 | 3 |
| gt-oblig/src/analysis.rs | narrow | 8 | 8 | 0 |
| gt-oblig/src/analysis.rs | widen | 4 | 2 | 2 |
| gt-oblig/src/analysis.rs | check_post_fixpoint | 6 | 6 | 0 |
| gt-oblig/src/driver.rs | analyze_program_with | 40 | 29 | 11 |
| gt-oblig/src/driver.rs | escaped_sites | 10 | 9 | 1 |
| gt-borrowck/src/check.rs | conflicts | 12 | 12 | 0 |
| gt-borrowck/src/check.rs | check_uses | 22 | 21 | 1 |
| gt-borrowck/src/check.rs | terminator_effect | 0 | 0 | 0 |
| gt-borrowck/src/regions.rs | outl | 0 | 0 | 0 |
| gt-borrowck/src/regions.rs | relate | 16 | 13 | 3 |
| gt-borrowck/src/solve.rs | * | 8 | 7 | 1 |
| gt-types/src/send.rs | * | 20 | 17 | 3 |
| gt-types/src/db.rs | select_impl | 8 | 5 | 3 |
| gt-types/src/check/pat.rs | * | 88 | 69 | 19 |
| gt-types/src/check/call.rs | * | 86 | 66 | 20 |
| gt-types/src/validate.rs | * | 54 | 47 | 7 |
| gt-term/src/lib.rs | check_termination | 56 | 52 | 4 |
| gt-term/src/lib.rs | instance_graph | 54 | 44 | 10 |
| gt-term/src/lib.rs | inferred_candidates | 4 | 4 | 0 |
| gt-term/src/lib.rs | sccs | 18 | 17 | 1 |
| gt-effects/src/lib.rs | check_effects | 22 | 16 | 6 |
| gt-guard/src/lib.rs | check_module | 42 | 42 | 0 |
| gt-guard/src/lib.rs | check_post | 10 | 10 | 0 |
| gt-guard/src/lib.rs | allowed_llvm_intrinsic | 0 | 0 | 0 |
| gt-guard/src/lib.rs | runtime_address_violations | 16 | 15 | 1 |
| gt-guard/src/lib.rs | abort_profile | 4 | 4 | 0 |
| gt-driver/src/verdict.rs | decide | 14 | 14 | 0 |

## Phân loại 100 nhánh chưa chạy (nhóm; liệt kê từng dòng ở tệp dữ liệu)
Mỗi nhóm ghi cơ sở. **"Lập luận" = đọc mã, KHÔNG phải bằng chứng thực nghiệm**; "thí nghiệm" = có chương trình thật chạy qua `gritc`.

| nhóm | nhánh (ước) | cơ sở | hướng sai nếu nhánh hỏng |
|---|---:|---|---|
| A. Hook chỉ-kiểm-thử / hằng cấu hình (`select_impl_fault`, `select_impl_first`, `DROP_TRAIT_EDGES`, `ALLOW_INTRINSIC_VALUES`) | 5 | không thể bật từ chương trình Grit (cấu hình biên dịch/biến kiểm thử); các hook được test riêng của chúng dùng | không áp dụng |
| B. Giới hạn tài nguyên (`fixpoint` UNREP_MAX_BLOCKS/MAX_BLOCK_STEPS, `widen` môi trường chết, `send` depth>64) | 5 | đạt tới từ chương trình Grit CỰC LỚN/SÂU. `send` depth>64: THÍ NGHIỆM ⇒ ĐỎ (fail-closed). `fixpoint`: thử 16 000 `⎇` — `gritc check` chưa xong sau >15 phút (đã dừng): KHÔNG kết luận; lập luận: trả `None` = "không chứng minh" ⇒ check được GIỮ lại (xem comment `analysis.rs` ~dòng 1062: ICE E9901 chứ không đoán) | an toàn (giữ check/ĐỎ) — CHƯA CHỨNG MINH bằng thí nghiệm |
| C. Nhánh phòng thủ `let … else`/ICE khi trạng thái do giai đoạn trước đảm bảo (`driver.rs` ICE E9901, `escaped_sites`, `solve.rs`, `regions.rs::relate`+`defend`, `pat.rs`/`call.rs`/`validate.rs` `let Some(..) else`, `term` `by_node`/`terminator`, `effects` `sig_by_node`/trait lookup, `guard` callee) | ≈ 45 | chưa tìm được chương trình Grit hợp lệ tới nhánh (nhiều nhánh cần định danh đã giải tên sai — resolver đã bắt trước). Lập luận: đa số kết thúc bằng ICE E9901 (đỏ) hoặc `defend` (ghi nhận) ⇒ không sinh binary | đỏ/ICE (an toàn); nhánh `continue` (bỏ qua khối thiếu terminator) CHƯA CHỨNG MINH |
| D. Nhánh ngữ nghĩa đạt tới được từ chương trình người dùng, CHƯA có thí nghiệm (`pat.rs` kiểu `Infer`/literal, truy cập riêng tư giữa module, `call.rs` đường dẫn `::`, quyền capability 287–289, hiệu ứng; `validate.rs`; `term` `b.loops`; `hyps_for`/`assign_local`/`judge_posts`; `send.rs` kiểu đệ quy) | ≈ 45 | CHƯA THÍ NGHIỆM (hết ngân sách gói). Nhiều nhánh `hyps_for`/`assign_local` chỉ BỚT giả thiết ⇒ ít chứng minh hơn ⇒ check được giữ (hướng an toàn, lập luận) | còn LÀ ỨNG VIÊN false-negative cho tới khi thí nghiệm; ưu tiên cao nhất cho `call.rs` quyền capability và `term` |

Con số nhóm là ước lượng theo tên hàm, không phải phân loại từng dòng: bảng từng dòng (tệp:dòng:hướng, mã nguồn) ở `docs/DECISION_COVERAGE_DATA.md`.

## E5 (thử trượt, ghi thật)
Chương trình 16 000 lệnh `⎇` liên tiếp trong một `main` để chạm `UNREP_MAX_BLOCKS` (=32 768 khối): `gritc check` chạy >15 phút chưa xong và bị dừng ⇒ **không có kết quả cho nhánh này**; đồng thời cho thấy thời gian kiểm siêu tuyến tính theo kích cỡ hàm (không phải lỗi an toàn; ghi vào KNOWN_LIMITATIONS nếu cần).

## Quy tắc đã giữ
Không sửa mã để tăng phần trăm; 8 mẫu corpus đều khẳng định kết quả (mã lỗi/pass), không chỉ "chạm dòng". Việc kế: thí nghiệm nhóm D (và chạy lại job `coverage`: `gh workflow run ci.yml -f only=coverage`).
