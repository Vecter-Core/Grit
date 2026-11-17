# PR (CHƯA MỞ) — hợp nhất `integration` vào `main`

**Trạng thái:** bản thảo mô tả PR, soạn 2026-11-18 (GÓI 3 S4.c). **KHÔNG mở PR, KHÔNG push `main`** (luật `AGENTS.md`). Chỉ Giang mở PR/quyết hợp nhất. Mọi số liệu dưới đây lấy từ lệnh đã chạy ngày 2026-11-18 tại HEAD `9f2fe6e` (cập nhật lại trước khi mở).

## Kiểm tra quan hệ nhánh (lệnh đã chạy)
- `git merge-base --is-ancestor main integration` ⇒ **đúng** (`main` = `cc23c24` là tổ tiên của `integration`): hợp nhất là **fast-forward** được; không có xung đột.
- `git rev-list --count main..integration` ⇒ **165** commit.
- `git diff --stat main..integration` ⇒ **707 tệp**, +27 824 / −745 dòng. Riêng `crates runtime lib tests`: 183 tệp, +8 322 / −460. Nhóm tệp đổi nhiều nhất: `audit/probes` 181 (dữ liệu thử), `docs/archive` 87 (di chuyển), `audit/results` 72, `tests/fail` 66, `crates/gt-driver` 38, `tests/pass` 29, `docs/errors` 18, `capstone/solutions` 16, `crates/gt-types` 14.
- Điều kiện B7 (xem `docs/GIANG_DECISIONS_2026-11-18.md`) — **CHƯA đủ**: (1) repo đẩy được — **đã đẩy `windows-work` lên `gn3305970-lang/Grit` (riêng tư)** nhưng `origin` còn trỏ kho cũ; (2) CI GitHub chạy xanh một lần trên Linux và Windows — **CHƯA chạy lần nào**; (3) nhãn loại A sửa — **đã sửa** (xem GIANG_DECISIONS).

## Mô tả PR (dán khi mở)
**Tiêu đề:** Hợp nhất nhánh `integration` (Seed 1.0 rc3: hợp nhất hai dòng Windows/Linux, Arena chia sẻ, std, capstone, kiểm chứng) vào `main`

**Nhóm commit (KHÔNG squash — `FIELD_BUGS.md`, `RUN_REPORTS.md` trỏ tới hash từng commit):**
1. **Dòng Linux (Seed 1.0, S1–S8):** M6 qua giá trị hàm (ADR-C01/C02), Arena chia sẻ `&'a Arena` (ADR-C03), `std` bằng Grit (Vec API, sort, Hash, HashMap), `gen_docs` sinh STD_REFERENCE, fail-open audit, đóng băng (ADR-C04, `FREEZE.md`), capstone K1–K4, FB-C1…C27.
2. **Dòng Windows (G1/G2, P0–P4, GÓI 3):** hợp nhất `c3ba7de`, FB-C28…C31, FB-17/18/19, đọc mã đối kháng (`TCB_CODE_AUDIT*.md`), LSP hover/definition/symbol, `fuzz_loop`, `spawn_failed`, công cụ nặng tự từ chối khi preflight FAIL, cổng gắn băm cây.
3. **Tài liệu:** `PROJECT_STATUS`, `READINESS`, `RUNLIST`, `RUN_REPORTS`, `HANDOFF`, lưu trữ `docs/archive/`.

**Bằng chứng cổng (nói thẳng):** cổng liền mạch xanh gần nhất ghi ở `docs/RUN_REPORTS.md` (commit `13e2df9`, 1023/0/204; xanh MỚI cho rc3 sẽ ghi sau S5.c) — chỉ giá trị trên đúng hash đó.

**Rủi ro / CHƯA KIỂM:** CI GitHub chưa chạy; capstone K1/K2/K4 trên Windows và phiên sạch chưa; VS Code thật chưa; TCB: chỉ ≈ 3,8 % dòng được đọc chéo (`CROSS_REVIEW_COVERAGE.md`); fuzz ≪ 24 h; không TSan/loom. Phase 1 **chưa** được tuyên bố xong; nhãn `1.0.0-seed` chưa đạt (A2 chưa).

**Cách hợp nhất đề xuất:** `git checkout main && git merge --ff-only integration` hoặc nút "Rebase and merge"/"Create a merge commit" của GitHub; **không** "Squash and merge".
