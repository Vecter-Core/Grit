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

## GÓI 6 S1 — cập nhật (2026-11-19)
* **Số nhánh chưa chạy: trước 100 (cuối GÓI 5) → sau 90** (lần 1 của gói, CI run `44344008609`: 91; lần 2, run `44353435090`: 90; tổng nhánh 730 → 734 vì bản sửa `gt-term` thêm nhánh mới). Trong 90 có 1 nhánh MỚI chưa chạy (`gt-term/lib.rs:908`, điều kiện lời nhắc E0702) — ghi rõ ở bảng. Phủ được nhờ 17 mẫu corpus mới (`tests/*/*_cov.gt`): `judge_posts` (hậu điều kiện nhắc tham số bị mượn), `assoc_value` (hai nhánh E0309), `reject_intrinsic_value` (intrinsic làm giá trị ngoài `main`), `pat.rs` (khoảng float/char, biến thể không qualify, struct pattern trên tên hàm, shadow_hint), `analyze_program_with` (literal âm ngoài phạm vi).
* **BẢNG TỪNG NHÁNH** (tệp, dòng, cột, hàm, hướng, điều kiện, nhóm A/B/C/D, nguồn, thí nghiệm hoặc lý do): `docs/DECISION_COVERAGE_BRANCHES.md`, sinh bằng `python tools/coverage_table.py cov.json.gz` (ĐẾM THẬT, thay cho phần "ước lượng theo tên hàm" ở trên; phần ước lượng đó SAI: nhóm D thật là **23**, không phải ≈ 45; C = 57, B = 6, A = 4).
* **FB-C mới: 0.** Mọi thí nghiệm đạt tới nhánh đều cho kết quả đúng (đỏ đúng mã hoặc xanh đúng). Hai thí nghiệm capability (`call.rs`): intrinsic làm giá trị ngoài `main` ⇒ E0313 đúng; `Cap::sub` làm giá trị ngoài `main` ⇒ xanh vì cần `&Cap` để gọi (không đúc quyền; backend vẫn từ chối); `root_*` ngoài `main` ⇒ E0310 (qua hàm; qua struct); `root_fs` làm giá trị hàm trong struct ⇒ E0313; truyền cap qua generic `id(root_fs())` TRONG `main` ⇒ xanh (hợp lệ). Các thí nghiệm capability này chạy trực tiếp (`target/tmp`), KHÔNG vào corpus trừ intrinsic-ngoài-main. Nhánh `289` (hàm prelude trả `Cap` có tham số `Cap` ngoài self) KHÔNG đạt tới được vì prelude không có hàm như vậy.
* **Tìm thấy ở đường này (không phải sai âm tính):** `nhóm D CHƯA`: 14 nhánh còn chưa thí nghiệm; 9 nhánh đã thử nhưng chương trình không chạm (xem cột nguồn); 1 trong `gt-term` (`b.loops`) là ICE phòng thủ cho thân không có chữ ký hàm, đọc mã cho thấy không đạt tới (hằng/static không có vòng lặp).

## GÓI 7 S2.c (2026-11-19) — nhóm D: thí nghiệm làm được, KHÔNG đo lại độ phủ
**Số nhánh chưa chạy trước = 90 (GÓI 6). Sau: CHƯA ĐO** — job `coverage` cần GitHub Actions (đang bị khoá cho tài khoản: `gh workflow run` ⇒ HTTP 422 "Actions has been disabled for this user") và máy cục bộ không có `cargo-llvm-cov`. Không báo số giả. Khi Actions mở lại: `gh workflow run ci.yml --ref windows-work -f only=coverage` một lần rồi `python tools/coverage_table.py` để cập nhật bảng.
Thí nghiệm đã làm (đều có khẳng định kết quả, đã vào corpus hoặc ghi ở đây; KHÔNG sửa mã sản phẩm để tăng phần trăm):
| nhánh | thí nghiệm | kết quả | ghi vào |
|---|---|---|---|
| `pat.rs:473` pattern struct nêu trường riêng tư ở module khác | `tests/fail/E0203_struct_pat_private_field_cov.gt` | E0203 ĐỎ (nhánh `!public && !is_within` đạt tới theo lập luận; chưa đo) | corpus |
| `call.rs:9/13/33` đối số `{poly}` không phải Path | `tests/fail/E0301_poly_arg_not_path_cov.gt` (kết quả lời gọi hàm truyền vào `Mutex::with`) | E0301 ĐỎ cho cả hàm thuần lẫn hàm `io`: **giới hạn**: đối số `{poly}` phải là TÊN hàm; fail-closed, KHÔNG fail-open | corpus + AI_PITFALLS |
| `send.rs:122/124` (`seen`, kiểu đệ quy khi kiểm Send) | `▢ T { v: u32, kids: Vec<T> }` rồi `spawn(w, t)` | E0501 ngay ở `Vec` (chứa `&` trong Phase 1) và E0315; không có con trỏ sở hữu Send nào ⇒ ADT đệ quy theo giá trị vô hạn (E0301). **Kết luận (lập luận): nhánh không tới được trong Phase 1; nên chuyển nhóm D → C** | đây |
| `pat.rs:347` tuple trên `Ty::Infer`, `pat.rs:222` range trên biến Infer | `(a, b)` trên tuple đã biết; `⌥ x { 0..=3 …}` với `x ≔ 5` | xanh nhưng KHÔNG xác nhận chạm nhánh (không có công cụ đo); không đưa vào corpus vì không khẳng định gì mới | đây |
Còn lại chưa thí nghiệm: `analysis.rs:978/983/907` (đã thử GÓI 6), `driver.rs:203`, `check.rs:424`, `call.rs:168/289/367/512/553`, `validate.rs:140`, `lib.rs:461`.
