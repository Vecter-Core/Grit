# R11c — phiên Claude OPUS MỚI đọc tấn công (CÙNG HỌ, ĐỘC LẬP YẾU), tự chứa

**Trạng thái: R11c CHỜ PHIÊN OPUS (Giang mở).** Soạn 2026-11-19 (GÓI 4 S1.b). **Ghi chú trung thực:** Giang chỉ dùng Claude. Phiên này CÙNG HỌ model với người viết gần như toàn bộ mã nên là đọc ĐỘC LẬP YẾU (khác phiên, khác ngữ cảnh, không khác họ). Kết quả phải ghi dòng "CÙNG HỌ, ĐỘC LẬP YẾU" và KHÔNG được dùng để tuyên bố "đã kiểm toán" hay nâng trạng thái TCB. Bạn phải là một phiên MỚI, không phải phiên đã viết mã dự án.

## Bối cảnh (đọc 5 phút)
Grit là ngôn ngữ cho AI viết; trình biên dịch `gritc` (Rust, repo `D:\Grit`, nhánh `windows-work`) **từ chối** (ĐỎ `E…`/ICE `E9901`) mọi chương trình mà nó không chứng minh được an toàn và khi đó **không sinh binary** (fail-closed). Lỗi nghiêm trọng nhất là **false negative**: chấp nhận chương trình sai, sinh mã sai hoặc UB. Chỉ ≈ 3,8 % dòng TCB từng được đọc chéo khác họ (`docs/CROSS_REVIEW_COVERAGE.md`): `runtime/gt-rt` + `crates/gt-guard` tại hash G1.5 `c5aa45b` (GPT-6). Phần còn lại ≈ 63 000 dòng chỉ có corpus, oracle, đột biến, bộ sinh. Việc của bạn là tìm chương trình `.gt` (hoặc test Rust) mà `gritc` chấp nhận hoặc chạy sai.

Đọc trước (≤ 30 phút): `AGENTS.md`, `docs/TCB_REVIEW_GUIDE.md`, `docs/CROSS_REVIEW_COVERAGE.md`, `docs/TCB_CODE_AUDIT.md`, `docs/TCB_CODE_AUDIT_CLAUDE.md`, `docs/KNOWN_LIMITATIONS.md`, `docs/AI_GUIDE.md`.

## Vai trò
Kẻ tấn công, CHỈ ĐỌC (và thêm test/FB khi tìm được lỗi, xem quy tắc). Mọi "an toàn" phải có thí nghiệm kèm kết quả thật.

## Phạm vi (xếp ưu tiên; tổng ≈ 8 000 dòng — KHÔNG đọc hết repo)
**(1) Mọi vùng "chưa đọc chéo" đã THAY ĐỔI SAU hash G1.5 `c5aa45b`:** `git log --format=%h c5aa45b..HEAD -- runtime crates` và `git blame`; ưu tiên `runtime/gt-rt/src/lib.rs` (FB-C31 `ffi_bytes`, `spawn_failed`, `thread_stack_bytes`, test-fault), `crates/gt-guard` (allowlist intrinsic, test GUARD-02), `crates/gt-types/src/validate.rs::validate_copy_impl` (FB-17), `crates/gt-mir/src/interp.rs` (u128, FB-18), `crates/gt-driver/src/lsp.rs` (LSP chỉ hiển thị).

**(2) HÀM QUYẾT ĐỊNH của checker** (đã chọn bằng cách liệt kê tệp/hàm; số dòng là xấp xỉ):
| ưu tiên | tệp / hàm | ≈ dòng | vì sao quan trọng |
|---|---|---:|---|
| 1 | `crates/gt-oblig/src/analysis.rs`: `judge`, `judge_core`, `judge_ob`, `judge_pres`, `judge_posts`, `hyps_for`, `assign_local`, `fixpoint`, `narrow`, `widen`, `check_post_fixpoint`; `src/driver.rs`: `analyze_program_with`, `escaped_sites` | 1 200 + 790 | **quyết định XOÁ/GIỮ check** lúc chạy (mặc định `--checks=all` giữ hết; `--checks=proven` thử nghiệm). 30 đột biến sống chỉ có lập luận; sai ở đây ⇒ xoá nhầm check |
| 2 | `crates/gt-borrowck/src/check.rs`: `conflicts`, `check_uses`, `terminator_effect`; `src/regions.rs`: `outl`, `relate`, luật WF; `src/solve.rs` | 1 630 | sở hữu/mượn/Arena; 3 đột biến (361/591/554) sống bằng lập luận đơn điệu; Arena chia sẻ `&'a Arena` (ADR-C03/C08) |
| 3 | `crates/gt-types/src/send.rs` (Send/Sync), `db.rs::select_impl`, `check/pat.rs` (khớp mẫu qua tham chiếu, ADR-040), `check/call.rs` (E0313 `callee_pos`, capability), `validate.rs` | 150 + 450 + 565 + 620 + 335 | Send qua luồng, chọn impl, quyền capability (điều kiện cứng #12) |
| 4 | `crates/gt-term/src/lib.rs` (chứng minh dừng: `check_termination`, `instance_graph`, `inferred_candidates`, `sccs`) | 930 | vòng lặp/đệ quy không dừng được chấp nhận; đột biến `lib.rs:361` tương đương CÓ ĐO |
| 5 | `crates/gt-effects/src/lib.rs` (`check_effects`) | 395 | hiệu ứng `io`/`mem`/`lock`; callee lạ ⇒ `io` |
| 6 | `crates/gt-guard/src/lib.rs` (`check_module`, `check_post`, `allowed_llvm_intrinsic`, `runtime_address_violations`, `abort_profile`) | 520 | bộ kiểm LLVM IR độc lập sau codegen; đếm abort theo (hàm, loại) không theo vị trí (GUARD-02) |
| 7 | `crates/gt-driver/src/verdict.rs::decide`, `pipeline.rs` | 590 | cổng phán quyết: chỉ xanh/vàng mới chạy |


## Nghi vấn mở đã biết (7, từ R11b — hãy thử bác bỏ hoặc xác nhận)
ARENA-01 (kích thước/căn lề tổng quát; release không bật kiểm chồng lấn), VEC-01 (tràn `2*cap`/`len+n`), ATOMIC-01 (thứ tự atomic, không TSan/loom), GUARD-02, ATTEST-01 (attestation là chuỗi metadata), UNSAFE-01 (chưa có kiểm kê `unsafe` tự động), argv đầu-cuối (surrogate lẻ qua shell).

## Quy tắc BẮT BUỘC
1. Mỗi nghi vấn: một chương trình `.gt` hoặc test Rust tấn công + kết quả thật (lệnh, mã thoát, đầu ra rút gọn); kết luận `xác nhận` / `bác bỏ trong phạm vi thí nghiệm` / `chưa kiểm được` (+ lý do). **Cấm "an toàn" không bằng chứng.**
2. **Cấm** đổi trạng thái TCB (`TCB_INTRINSICS.md`, `STATUS.md`) và **cấm sửa mã** (chỉ thêm test + FB): lỗi thật ⇒ test ĐỎ trước (commit riêng) → báo kiến trúc sư (hoặc sửa nếu hỏng bộ nhớ đạt tới từ Grit) → FB-C trong `docs/FIELD_BUGS.md`. Mọi commit có `Author-Model: <tên model thật>`.
3. Quy trình repo: không chạy hai `cargo` cùng lúc; `CARGO_BUILD_JOBS=2`, `--test-threads=2`; lô nặng ≤ 15 phút; `bash tools/preflight.sh D:/Grit` trước lô nặng. Không `git reset --hard`/`clean`/force-push, không push `main`. Không chạy `gate.sh` đầy đủ.
4. Mẫu tấn công: `tests/fail/*`, `crates/*/tests/support/progen.rs`, `docs/AI_GUIDE.md`. Chương trình bị chấp nhận mà theo `docs/spec/` phải bị từ chối là false negative.

## Giao kết
Ghi `docs/TCB_CODE_AUDIT_OPUS.md`: **tên model thật, hash HEAD đọc, ngày, dòng "CÙNG HỌ, ĐỘC LẬP YẾU"**; tóm tắt (xác nhận / bác bỏ / chưa kiểm được); bảng theo từng hàng ưu tiên (tệp, hàm đã đọc, số dòng đọc thật, thí nghiệm, kết quả); bảng 7 nghi vấn mở; lỗi mới (FB-C…); **phần KHÔNG đọc**; đề xuất bước kế.
