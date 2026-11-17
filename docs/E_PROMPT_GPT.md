# R11c — review tấn công của model KHÁC HỌ CLAUDE (GPT/Codex), tự chứa

**Trạng thái: R11c CHỜ PHIÊN GPT (Giang mở).** Lời nhắc này tự chứa: người đọc KHÔNG cần biết lịch sử. Soạn 2026-11-18 (HEAD soạn `3572787` + commit sau đó; **ghi hash HEAD thật bạn đọc**). Số liệu dưới đây lấy từ `docs/CROSS_REVIEW_COVERAGE.md` (tạo bằng `python tools/cross_review_coverage.py`); chạy lại tại HEAD của bạn.

## Bối cảnh (đọc 5 phút)
Grit là ngôn ngữ cho AI viết; trình biên dịch `gritc` (Rust, repo `D:\Grit`, nhánh `integration`) **từ chối** (ĐỎ `E…`/ICE `E9901`) mọi chương trình mà nó không chứng minh được an toàn, và khi đó **không sinh binary**. "Fail-closed": không biết = không cho qua. Lỗi nguy hiểm nhất là **false negative** (chấp nhận chương trình sai / sinh mã sai / UB). Gần như toàn bộ mã trusted (TCB, ~65 856 dòng) do **Claude** viết, nên người đọc phải KHÁC họ. Theo `CROSS_REVIEW_COVERAGE.md` mới ≈ **3,8 %** dòng TCB được đọc chéo; **63 208 dòng (checker M6/M7/M8/kiểu/hiệu ứng/MIR/driver) chưa từng được đọc chéo**. Việc của bạn là thu hẹp khoảng đó.

Đọc trước (tối đa 30 phút): `AGENTS.md`, `docs/TCB_REVIEW_GUIDE.md`, `docs/CROSS_REVIEW_COVERAGE.md`, `docs/TCB_CODE_AUDIT.md`, `docs/TCB_CODE_AUDIT_CLAUDE.md`, `docs/KNOWN_LIMITATIONS_DIGEST.md`, `docs/AI_GUIDE.md` (cách viết chương trình `.gt` để tấn công).

## Vai trò
Kẻ tấn công. Mục tiêu: tìm **chương trình `.gt` (hoặc test Rust) mà `gritc` chấp nhận/chạy sai** (vi phạm bộ nhớ, đua dữ liệu, tràn không báo, chia 0, chỉ số ngoài biên, mượn sai, hiệu ứng/capability lọt, vòng lặp không dừng được chấp nhận, secret bị bỏ sót chỉ là cảnh báo VÀNG nên ít ưu tiên). Mọi "an toàn" phải có thí nghiệm kèm kết quả thật.

## Phạm vi (xếp ưu tiên; tổng ≈ 8 000 dòng — KHÔNG đọc hết repo)
**(1) Vùng "chưa đọc chéo" nhỏ, ưu tiên cao nhất (≈ 150 dòng):** mã do Claude Sonnet 5.5 viết ở phiên 2026-11-17/18: `runtime/gt-rt/src/lib.rs` (`write_str`, `ewrite_str`, `utf8_valid`, `bytes_eq` dùng `ffi_bytes`; FB-C31, commit `f9f0f78`), test `crates/gt-guard/tests/guard.rs::every_unlisted_llvm_intrinsic_family_is_rejected` (commit `0eedc79`), `crates/gt-types/src/validate.rs::validate_copy_impl` (FB-17), `crates/gt-mir/src/interp.rs` (`binop_u128`, FB-18). Liệt kê chính xác bằng `git log --format=%h --author-date-order c5aa45b..HEAD -- runtime crates` và `git blame`.

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
1. Mỗi nghi vấn: (a) **một chương trình `.gt` hoặc test Rust tấn công** và **kết quả thật** (lệnh + mã thoát + đầu ra rút gọn); (b) kết luận `xác nhận` / `bác bỏ trong phạm vi thí nghiệm` / `chưa kiểm được` (+ lý do, cần gì). **Cấm "an toàn" không bằng chứng;** phân biệt "không tìm thấy" với "đã chứng minh".
2. **Cấm** đổi trạng thái TCB (`TCB_INTRINSICS.md`, `READINESS.md`) và **cấm sửa mã** (trừ thêm test + FB). Lỗi thật: test **ĐỎ trước** (commit riêng) → sửa (commit riêng, `Author-Model: <tên model thật của bạn>`) → ghi FB-C vào `docs/FIELD_BUGS.md`. Lỗi hỏng bộ nhớ/UB đạt tới từ Grit: sửa ngay theo cách đó rồi báo.
3. Quy trình repo: đừng chạy hai `cargo` cùng lúc; `CARGO_BUILD_JOBS=2`, `--test-threads=2`; lô nặng ≤ 15 phút; chạy `bash tools/preflight.sh D:/Grit` trước lô nặng (Windows: Git Bash `C:\Program Files\Git\bin\bash.exe`). Không `git reset --hard`/`clean`/force-push, không push `main`. Không chạy `gate.sh` đầy đủ (≈ 2 giờ).
4. Mẫu tấn công `.gt`: xem `tests/fail/*` (mỗi mã lỗi có ≥ 3 mẫu), `crates/*/tests/support/progen.rs` (bộ sinh), `docs/AI_GUIDE.md`. Một chương trình bị chấp nhận mà theo `docs/spec/` phải bị từ chối là **false negative**.

## Giao kết
Ghi `docs/TCB_CODE_AUDIT_GPT2.md`: **tên model thật, hash HEAD đọc, ngày**; tóm tắt (số nghi vấn xác nhận / bác bỏ / chưa kiểm được); bảng theo từng hàng ưu tiên (tệp, hàm đã đọc, số dòng đọc thật, thí nghiệm, kết quả); bảng 7 nghi vấn mở; lỗi mới (FB-C…); **phần KHÔNG đọc** (nêu rõ); đề xuất bước kế. Dòng TCB chỉ được ghi "ĐÃ QUA ĐỌC MÃ ĐỐI KHÁNG BẰNG MODEL KHÁC (<model>, <hash>)" bởi kiến trúc sư/Giang sau khi xem báo cáo, không phải bởi bạn.
