# FREEZE — những gì Seed 1.0 ĐÓNG BĂNG (và những gì KHÔNG)

Phiên bản: `gritc --version` ⇒ `1.0.0-rc2` (nhãn `1.0.0-seed` CHỈ khi đạt cả sáu điều kiện của `docs/SEED_PLAN.md`; điều kiện còn thiếu liệt kê ở `docs/READINESS.md` mục "Seed 1.0" và `docs/RUN_REPORTS.md` S8). Nền đã kiểm: Linux x86_64; Windows: CHƯA kiểm. Quyết định đóng băng: ADR-C04.

Quy tắc chung: một mục ĐÓNG BĂNG chỉ được đổi bằng (1) một ADR mới (`docs/DECISIONS.md`, tiền tố ADR-C hoặc kế tiếp), (2) tăng phiên bản thích hợp, (3) cập nhật `CHANGELOG.md` và các test khoá liên quan. "Thêm" tương thích được phép khi ghi rõ bên dưới; "đổi tên/xoá/đổi nghĩa" thì không.

## Đóng băng
| hạng mục | nội dung đóng băng | nơi khoá / test |
|---|---|---|
| Cú pháp và ký hiệu | `docs/spec/grammar.ebnf`, `docs/spec/symbols.md` (canonical ↔ ASCII 1-1), từ khoá cứng; `gritc fmt` in dạng canonical duy nhất; `parse(fmt(x)) == parse(x)` | corpus + round-trip + test lexer/parser/fmt |
| Ngữ nghĩa lõi | `docs/spec/semantics.md`, `docs/spec/regions.md`: mượn (`&` / `&!`), Arena, hiệu ứng, capability, hợp đồng, quy tắc dừng (`↓`, nhiên liệu), D7 không shadowing, kiểm tràn mặc định | corpus, oracle động, đột biến |
| Mã chẩn đoán | số mã và nghĩa trong `docs/spec/error_codes.md` + `docs/errors/*.md`: KHÔNG tái dùng số; mã mới chỉ thêm; mã đã nghỉ không tái dùng | `tools/check_expectations.py` (trong cổng) |
| JSON chẩn đoán | `docs/spec/diag_schema.json` schema_version 1: chỉ THÊM trường; bộ tiêu thụ bỏ qua trường lạ | test `diag_schema` (đầu ra thật của `gritc check/run --json` hợp lệ với schema) |
| Mã thoát | gritc: 0 xanh · 1 đỏ (hoặc vàng khi `--deny-yellow`) · 2 ICE · 3 lỗi dùng/IO/backend; `run`: 70 abort, 71 hết giới hạn bộ chạy, 100+s trạng thái chương trình; rig: 10 lock lệch, 11 dự án sai, 12 dùng sai, 13 IO | `tests/cli.rs`, `rig.rs` |
| Giao diện LSP | `gritc lsp`: `initialize`, đồng bộ toàn văn, `publishDiagnostics` (một chẩn đoán LSP cho mỗi chẩn đoán gritc), `textDocument/formatting`; chỉ thêm khả năng | `tests/lsp.rs` |
| Thư viện chuẩn | các chữ ký công khai trong `docs/STD_REFERENCE.md` (sinh tự động; test khoá từng byte): hàm hiện có không đổi chữ ký/nghĩa; chỉ THÊM hàm | `docs_lock.rs`, `std2_lib.rs` (≥ 30 chương trình ba nguồn mỗi nhóm) |
| Intrinsic TCB | danh sách và hợp đồng ở `docs/TCB_INTRINSICS.md` (27 hàm runtime trong `gt_guard::RUNTIME`; ngân sách intrinsic mới của Seed: 5/6 đã dùng) | IR Guard bảng `RUNTIME`, `io_intrinsics.rs`, `str_eq.rs` |
| Obligation IR | schema 1 (`gt-oblig/src/ir.rs`, `SCHEMA_VERSION = 1`): chỉ THÊM khoá; JSON cũ là tiền tố của JSON mới (ADR-039, ADR-C04) | `gt-oblig/tests/ir_freeze.rs` |
| Rig | `rig.toml` (khoá cho phép: package/name/version/dependencies path), `rig.lock` (băm nội dung từng gói), lệnh `new/check/build/run/test/add/lock/fmt/clean` | `crates/rig/tests` |
| Dựng tái lập | cùng mã, cùng đường dẫn ⇒ cùng SHA-256 của `gritc` và của nhị phân do `rig build` | `BUILDING.md`, `rig.rs::builds_are_reproducible…` |

## KHÔNG đóng băng
- Mọi API của crate Rust nội bộ (`gt-*`): không phải giao diện công khai.
- Chi tiết triển khai các pass (M4…M10), cách đặt tên local `_N` trong MIR, thứ tự ghi chú `N…`, nội dung thông báo lỗi (chỉ MÃ được đóng băng), định dạng văn bản cho người của `gritc` (JSON mới là giao diện máy).
- Hiệu năng (thời gian dựng/chạy), kích thước nhị phân, hồ sơ tối ưu `release*` (đã có bằng chứng thực nghiệm dev≡release, KHÔNG phải chứng minh).
- Tập đột biến/corpus (chỉ được TĂNG).
- Windows: chưa kiểm ⇒ không có cam kết.
- Mọi tính năng ghi ở `docs/SEED_LIMITATIONS.md` là "chưa có" (closure, `?`, phân rã tuple, `unwrap`, …): có thể thêm sau, KHÔNG phải thay đổi phá vỡ.
