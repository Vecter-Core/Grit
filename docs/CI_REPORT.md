# CI_REPORT — GitHub Actions cho `gn3305970-lang/Grit` (GÓI 4 S0, 2026-11-19)

Kho riêng tư; `origin` = `https://github.com/gn3305970-lang/Grit.git` (đã `git remote rename origin old-origin` rồi `git remote add origin …`; kho cũ `Vecter-Core` giữ ở `old-origin`). Nhánh mặc định của kho (đọc bằng `gh repo view`, KHÔNG đổi): **`windows-work`** (Giang quyết nếu muốn đổi). Workflow `.github/workflows/ci.yml`: `permissions: contents: read`, không `pull_request_target`, không secret, không cron, ghim SHA (`actions/checkout` `11d5960a…` v4.4.0, `actions/cache` `0057852b…` v4, `cargo-deny-action` `70f81531…` v2), kích hoạt `push` (mọi nhánh) + `workflow_dispatch` (đầu vào `only=all|tsan`), `concurrency` huỷ run cũ cùng nhánh.

## Kết quả cuối XÁC NHẬN LẠI trên commit cuối (run `44243992871`, commit `884af7c`): TẤT CẢ XANH — `linux-gate` 72 phút, `windows-core` 21 phút, `tsan` 11 phút (dựng lạnh), `determinism` 2,7, `linux-nollvm` 1, `deny` 0,5. Bảng dưới là run `44233333186` (commit `15fe834`), cùng kết quả.

## Kết quả của run `44233333186` (commit `15fe834`, 2026-11-18)
| job | nền | kết quả | thời lượng |
|---|---|---|---|
| `linux-gate` (cổng đầy đủ `tools/gate.sh` + capstone K1–K4 dev/release-o2) | ubuntu-24.04 | **XANH** | 74 phút |
| `windows-core` (BẢN RÚT GỌN) | windows-2022 | **XANH** | 26 phút (×2 khi tính phút) |
| `linux-nollvm`, `deny`, `determinism` (dựng `gritc` hai lần cho cùng sha256) | ubuntu-24.04 | **XANH** | 1,6 / 0,3 / 2,5 phút |
| `tsan` | ubuntu-24.04 | xanh (kết quả TSan ở `docs/TSAN_RESULTS.md`) | 2,2 phút |
Cổng Linux đầy đủ: 1024 test đạt / 0 lỗi / 206 bộ (Windows có 3 test chỉ chạy trên Windows ⇒ 1027). **Windows rút gọn (nói thẳng):** chạy `gtrt, fmt, clippy -D warnings, nul, expect (mọi unittest công cụ), doc, deletions, failopen, track` + `cargo test` của 8 crate lõi (lexer, parser, resolve, types, mir, guard, term, effects). KHÔNG chạy trên CI Windows: `audit` và test toàn bộ workspace (gt-fmt ≈ 17 phút, gt-driver ≈ 20 phút, oblig/borrowck oracle…) vì phút Windows tính gấp đôi; chúng đã chạy xanh trên máy Windows chính (`docs/RUN_REPORTS.md`, cổng `89438fd`: 1027/0/206).

## Nhật ký các lần chạy (đỏ ⇒ nguyên nhân ⇒ sửa)
| run | commit | kết quả | nguyên nhân đỏ (đọc log nguyên văn) | phân loại | sửa |
|---|---|---|---|---|---|
| `44186710455`, `44217326243`, `44217331380`, `44217734581` (workflow CŨ chưa từng chạy; 4 lần do tôi đẩy nhánh/thẻ GÓI 3, 2 lần huỷ) | `869e819`, `89438fd`, `2faf88f` | đỏ/huỷ | (1) Windows: `tar: D\:\a\_temp/llvm: Cannot open` (đường dẫn `D:` bị tar hiểu là máy từ xa); (2) `cargo deny`: `error[wildcard]` cho phụ thuộc path trong workspace; (3) Linux: `test_preflight` 1 lỗi (test `test_low_disk_threshold_is_enforced` đặt biến chỉ có tác dụng ở Windows) | (1) lỗi WORKFLOW; (2) cấu hình `deny.toml`; (3) lỗi TEST không portable (không phải lỗi Grit) | vòng 1–2 bên dưới |
| `44220898409` | `faadac2` | huỷ bởi push kế | — | — | — |
| `44221150612` (vòng 1) | `75c4772` | `deny` ✓ `nollvm` ✓ `determinism` ✓; **Linux ✗ 51 phút** (chỉ `test_preflight`, 1024 test đạt/0 đỏ); **Windows ✗ 7 phút** | Windows `expect`: mọi unittest công cụ gọi `bash` bằng `subprocess` ⇒ `CreateProcess` chọn `System32\bash.exe` (WSL), không phải Git Bash; ngoài ra checkout nông ⇒ `deletions` đạt RỖNG ("0 tệp từng bị xoá") | WORKFLOW + công cụ | thêm `PREFLIGHT_MIN_DISK_KB` (test chạy được trên Linux); `shutil.which("bash")` ở các công cụ/test; `fetch-depth: 0`; `deny.toml`: `allow-wildcard-paths = true` (phụ thuộc PATH nội bộ không phải wildcard; wildcard của crates.io vẫn bị cấm) |
| `44227524436` (vòng 2) | `e7b82bc` | **Linux ✓ 49 phút**; Windows ✗ 6 phút (cùng nguyên nhân `bash`: sửa PATH bằng `GITHUB_PATH` KHÔNG đủ vì `CreateProcess` tìm `System32` trước PATH) | lỗi công cụ | sửa gọi `bash` qua `shutil.which` |
| `44233333186` (vòng 3) | `15fe834` | **TẤT CẢ XANH** (Linux 74, Windows 26, ...) | — | — |
| `44241971280` (dispatch `only=tsan`) | `e819bc2` | `tsan` ✓ 8 phút (mẫu mở rộng 8 chương trình) | — | — |
| `44243992871` (commit cuối, đủ mọi job) | `884af7c` | **TẤT CẢ XANH** | — | — |
Số vòng sửa: Linux 2 (preflight test; đổi `bash`), Windows 3. **Lỗi THẬT của Grit tìm được qua CI: KHÔNG** (mọi đỏ là workflow/cấu hình/test không portable). Một phát hiện thật về QUY TRÌNH: giai đoạn `deletions` đạt RỖNG trên checkout nông (đã sửa bằng `fetch-depth: 0`); chạy `docs/RUN_REPORTS` cũ trên Windows chính không bị ảnh hưởng.

## Phút đã dùng (ước tính từ thời lượng job thật qua `gh api`)
Tới run `44241971280`: Linux ≈ 414 phút, Windows ≈ 41 phút (×2 = 82) ⇒ ≈ 495 phút; cộng run cuối `44243992871` (Linux ≈ 87, Windows ≈ 21 ×2 = 42) ⇒ **≈ 624 phút** trên ngân sách ≈ 2 000 phút/tháng của gói miễn phí repo riêng tư (CHƯA kiểm lại trong Billing; Giang kiểm). Mỗi lần push đầy đủ tốn thêm ≈ 75 (Linux) + 52 (Windows ×2) + vài phút ≈ 130 phút ⇒ chỉ nên đẩy khi cần; dùng `[skip ci]` cho commit tài liệu và `workflow_dispatch only=tsan` để lặp TSan (≈ 3–8 phút). KHÔNG có cron, không vòng tự kích hoạt.

## CHƯA KIỂM
Trạng thái Billing thật; CI trên Windows cho phần test lớn (gt-driver, gt-fmt, oblig, borrowck); `cargo audit` trên Windows; nhánh `main` (không đẩy).


## GÓI 5 (2026-11-19): thay đổi workflow và phút
* Job `changes` (≈ 0,3 phút) quyết định chạy `tsan` khi push; `tsan` ≈ 11 phút khi chạy; `concurrency` huỷ lần chạy cũ theo nhánh (`ci-<workflow>-<ref>`).
* `workflow_dispatch`: `only` = `all` | `tsan` | `coverage`; `full=true` thêm `audit` + test toàn workspace vào Windows (≈ 74 phút tính phí; Giang bấm trước mỗi thẻ phát hành: Actions → ci → Run workflow → chọn nhánh, `only=all`, `full=true`).
* Job `coverage` (chỉ dispatch): ≈ 55 phút Linux/lần, đã chạy 2 lần. Job `deny` thêm đối chứng wildcard (`cargo install cargo-deny` ≈ 3–5 phút).
* **Phút ước tính tới nay ≈ 624 + tsan 11 + coverage 2×55 = ≈ 745 phút** (chưa tính run kiểm tra cuối của gói; CHƯA kiểm Billing).
* **Workflow nào tốn phút:** `linux-gate` (≈ 72–87), `windows-core` (≈ 21 ×2 = 42 tính phí), `coverage` (≈ 55, thủ công), `tsan` (≈ 11, có điều kiện), `determinism` (≈ 3), `linux-nollvm` (≈ 1), `deny` (≈ 0,5; +3–5 với cargo-deny), `changes` (≈ 0,3).
* **Việc của Giang:** vào GitHub → Settings → Billing and plans → Usage: xem phút còn lại; đặt **Spending limit = 0 USD** / cảnh báo chi tiêu về 0 nếu chưa (để không bị tính tiền khi hết phút miễn phí).
* Run kiểm cuối GÓI 5 `44304931977`: linux-gate ≈ 46 phút, windows-core ≈ 21 (×2), deny/determinism/nollvm vài phút ⇒ cộng ≈ 100 phút tính phí; `44312549679` (deny) ≈ 4. **Tổng ước tính ≈ 850 phút** (CHƯA kiểm Billing).

## GÓI 6 (2026-11-19): CI bị khoá
Run kiểm cuối `44379290084` (commit `d1e36b7`) KHÔNG còn: `gh run list` rỗng, `GET /actions/runs` ⇒ `total_count: 0`, và `gh workflow run` ⇒ HTTP 422 "Actions has been disabled for this user". Dừng việc CI (điều kiện dừng e). Bằng chứng CI cuối còn lại là run `44365492150` (commit `1793b7f`): `linux-gate` GREEN (cổng đầy đủ, 1028/0/207, `GATE_TREE=19dc2326…`), `windows-core`, `determinism`, `deny`, `linux-nollvm` xanh; bước diff `--list` đỏ đúng (danh sách cũ) rồi đã sửa. **Việc của Giang:** vào GitHub → Settings → Billing and plans / Actions: xem vì sao Actions bị tắt cho tài khoản (hết phút? chính sách?), bật lại nếu cần, kiểm Spending limit = 0 USD.

## GÓI 7 S0e — rà workflow cho kho CÔNG KHAI (2026-11-19)
| tiêu chí | `ci.yml` | kết quả |
|---|---|---|
| `pull_request_target` / `workflow_run` chạy mã từ PR | không dùng; trigger chỉ `push` và `workflow_dispatch` | đạt |
| `permissions` tối thiểu | cấp workflow `contents: read`, không job nào nâng quyền | đạt |
| secret | không dùng `secrets.*` | đạt |
| action ghim SHA | checkout, cache, upload-artifact, cargo-deny-action đều ghim SHA 40 ký tự | đạt |
| chèn `${{ github.event.* }}`/`inputs.*` vào shell | **LỆCH**: job `changes` nội suy `github.event.before`, `github.sha`, `inputs.only` thẳng vào `run:` | **đã sửa**: truyền qua `env:` (`EVENT_BEFORE`, `EVENT_SHA`, `INPUT_ONLY`, `EVENT_NAME`), shell chỉ đọc biến môi trường |
| runner tự host | chỉ `ubuntu-24.04`, `windows-2022` | đạt |
| cron / schedule | không có | đạt |
| còn nội suy trong `run:` | chỉ trong khoá cache/`env:`/`if:` (không phải script) | đạt |
Ghi chú: `cargo install cargo-audit/cargo-deny/cargo-llvm-cov --locked` chưa ghim phiên bản công cụ (rủi ro chuỗi cung ứng nhỏ; ghi nhận, chưa đổi).
