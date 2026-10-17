# RUN_REPORTS — báo cáo ngắn sau MỖI mục của RUNLIST (số THẬT; cái gì chưa kiểm)

Quy ước: mỗi mục một tiểu mục `## R<n> — <trạng thái> (ngày)`; nêu việc đã làm, số liệu thật, lỗi thật tìm được (FB-n), cái KHÔNG làm/chưa kiểm, chỗ lệnh sai kỹ thuật + phương án fail-closed đã chọn.

## Ghi chú kỹ thuật về lệnh (không phải mục)
- **Cổng liền mạch vs "không tiến trình nền":** giai đoạn `test` của `tools/gate.sh` chạy `cargo test --workspace` mất ~30–45 phút, vượt giới hạn 10 phút của một lệnh công cụ; không thể chạy "đồng bộ ≤ 15 phút" mà vẫn là MỘT cổng liền mạch. Phương án fail-closed: cổng là NGOẠI LỆ DUY NHẤT chạy tách (WMI, stdin từ /dev/null); phiên chết giữa chừng ⇒ cổng không xanh ⇒ chạy lại từ đầu. Mọi việc nặng khác theo lô đồng bộ ≤ 8 phút (`--max-seconds 480`).

## R1 — XONG (2026-10-18)
- Việc đã làm: `tools/preflight.sh` (+`test_preflight.py` 6 test: kho sạch OK; `.git/index` toàn NUL, ref rỗng/NUL, HEAD NUL, object hỏng ⇒ fsck ĐỎ, ngưỡng đĩa ⇒ ĐỎ thoát 2); `tools/nul_check.sh` + giai đoạn `nul` của cổng (+`test_nul_check.py` 4 test, +1 test cổng `NulStage`); `tools/backup_bundle.sh`; quan hệ git ghi HANDOFF; KNOWN_LIMITATIONS (sự cố NUL, 3 giả thuyết CHƯA kiểm).
- Số thật: quét NUL 1399 tệp văn bản được theo dõi = 0 tệp nhiễm (1 ngoại lệ có chủ ý: `fuzz_regressions/` chứa NUL cố ý, đã kiểm có từ commit gốc 6b12823). 4 tệp nguồn từng bị NUL (gt-effects/src/lib.rs, gt-effects/tests/effects.rs, gt-mir/src/validate.rs, gt-mir/tests/unresolved_types.rs): khôi phục từ HEAD; thay đổi chưa commit duy nhất bị mất (sửa gt-effects) đã làm lại và còn trong HEAD (kiểm bằng grep: có `body_without_a_signature`, nhánh E9901 chữ ký, `Adt(Err)`, `Callee::Fn generics`). Cổng sau R1: `TEST_PASSED=943 FAILED=0 SUITES=185`, fmt/clippy/audit/nul/expect/deletions/track OK. Bundle `D:/Grit_backup/grit-<hash>.bundle` đã `verify` (OK).
- Lỗi của chính công cụ tìm thấy khi viết: preflight lần đầu báo SAI `.git/index` chứa NUL (index là nhị phân, có NUL hợp lệ) ⇒ sửa thành kiểm chữ ký `DIRC`; `nul_check` báo `crash_23817b64.gt` (NUL cố ý) ⇒ ngoại lệ có lý do.
- Chưa kiểm: nguyên nhân sự cố NUL (3 giả thuyết, không phân biệt được); preflight không bắt được hỏng ở tệp nguồn không theo dõi hoặc hỏng KHÔNG phải NUL (cắt cụt); sao lưu cùng ổ.
