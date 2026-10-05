# PROGRESS

## M0 — Đặc tả & nền móng — **CHƯA ĐẠT "Hoàn thành khi"** (chờ Giang duyệt)
Ngày: 2026-10-05.

### Đã làm (có thật)
- `Cargo.toml` workspace hợp lệ (`cargo metadata` chạy được), `.gitignore`.
- `docs/spec/grammar.ebnf`, `symbols.md` (bảng 1–1 canonical/ASCII), `semantics.md`, `error_codes.md` (~45 mã, có milestone + test mẫu).
- `docs/DECISIONS.md` (ADR-001..003), `OPEN_QUESTIONS.md` (Q1–Q6), `KNOWN_LIMITATIONS.md`.
- **Đo token** (`tools/measure_tokens.py`, proxy tiktoken): số liệu ở ADR-002. Sơ bộ: canonical rẻ hơn ASCII ~4–6% trên cả mẫu, nhưng 10–16/39 ký hiệu đắt hơn alias; `⊤ ⊥` tệ nhất.
- **48 chương trình mẫu**: `tests/pass` 18, `tests/fail` 29 (mỗi file một mã kỳ vọng; 4 file Unicode nguy hiểm ghi bằng byte), `tests/warn` 1 (S101). Header `// expect: pass | fail Exxxx | warn Sxxx`.

### Giao cho AI rẻ hơn (file `TASK.md`, CHƯA làm — M0 chưa "xong" cho tới khi làm)
- `.github/workflows/TASK.md` — CI. · `deny.toml.TASK.md` — cargo-deny.
- `docs/errors/TASK.md` — `Exxxx.md` cho mỗi mã. · `tests/TASK.md` — README + script đối chiếu `expect`.
- `crates/TASK.md`, `runtime/gt-rt/TASK.md`, `lib/core/TASK.md`, `tests/{golden,fuzz,bench}/TASK.md` — chỉ ghi milestone tạo.

### Còn thiếu / rủi ro mới phát hiện
- Cần Giang duyệt đặc tả và trả lời Q1–Q6 trước M1 (CLAUDE.md §4: dừng báo cáo sau milestone đầu).
- Token chỉ là proxy (Q2). `μ` vs XID (Q5) là rủi ro thật cho lexer.
- Seed cũ trong git mâu thuẫn cú pháp (ADR-003, Q1).
- Mẫu chưa được máy parse; chưa có test runner.
- Không chạy clippy/fmt/test: chưa có crate Rust nào.
