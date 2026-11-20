# PENDING_GIANG — việc CHỜ GIANG (lập 2026-11-17; agent KHÔNG tự đánh dấu thay Giang)

## GÓI 12 (2026-11-20) — việc CHỜ GIANG (mới nhất; agent KHÔNG tự đánh dấu thay Giang)
1. **Đọc `docs/REMOTE_STATE.md`** (nguồn gốc lần đẩy lên `Vecter-Core/Grit`, kho CÔNG KHAI, hai chỗ lệch quy trình: đẩy khi lệnh GÓI 10 cấm và `main` lên khi cổng Linux chưa chạy; cây `main` khác cây đã qua cổng đúng một dòng `license`). Quyết định: có giữ kho công khai không? Có chạy cổng trên cây Vecter-Core không?
2. **Fuzz qua đêm trong WSL (điều kiện #1; việc chặn Seed lớn nhất)** — lệnh ở `docs/HANDOFF.md` mục "Fuzz qua đêm (WSL)". Tắt chế độ ngủ Windows trước (Cài đặt ▸ Nguồn ▸ Ngủ = Không bao giờ khi cắm điện). Tuỳ chọn `%UserProfile%\.wslconfig`: `[wsl2]` `memory=10GB` `swap=8GB` (agent KHÔNG sửa tệp này).
3. **TSan trong WSL chưa chạy được: thiếu `clang`.** Giang chạy: `wsl -d Ubuntu-24.04 -- apt install -y clang` (trong WSL bạn là root; agent không tự cài theo lệnh GÓI 12). Rồi báo phiên Claude chạy lại `tools/tsan_check.sh`.
4. **Tệp tạm do agent để lại ngoài kho (agent không xoá được theo luật):** `D:\p1_g12.sh` (script tạm ở gốc ổ D); trong `D:\Grit_export\`: `p1_g12.sh`, `p1_wsl.out`, `fz_g12.sh`, `gw_g12.sh`, `fuzz_clone_log.jsonl`, `g12_win_gate*.log`; thư mục `D:\Grit_cnt` (bản sao đếm byte memcpy), `D:\Grit_g12w`, `D:\Grit_g12w2` (clone cổng). Xoá khi không cần. Trong WSL: `~/grit_g12`, `~/grit_fuzz`, `~/p1.sh`, `~/fz.sh`.
5. **Chép `D:\Grit_export` (bundle mới nhất) ra ngoài máy** (việc cũ, vẫn mở).
6. Quyết định ADR cho mã lỗi mới "vòng khởi tạo static" (`NEXT_PRODUCT_BATCH` #12) và duyệt đổi `_ =>` sang match vét cạn (#11).

## KHẨN 0 (GÓI 10): SAO LƯU NGOÀI MÁY — làm trước
1. **Chép `D:\Grit_export` ra ngoài máy** (USB, kho lưu trữ đám mây, hoặc email cho chính mình; cỡ ≈ 15 MB: bundle ≈ 3,9 MB, tar ≈ 10,9 MB). Đã kiểm khôi phục thật (clone bundle, `git fsck --full` rc=0, thẻ `seed-1.0-rc6` = `16c37f0`). Mọi bản sao hiện nay (C: và D:) có thể cùng một ổ.
2. Đọc thông báo đình chỉ GitHub (xem `docs/GITHUB_SUSPENSION.md`).
3. Tuỳ chọn: tạo kho RIÊNG TƯ ở dịch vụ khác bằng tài khoản của CHÍNH Giang (GitLab, Codeberg…), rồi Giang TỰ chạy `git remote add backup <url>`, `git push backup --all`, `git push backup --tags`. Agent KHÔNG giữ thông tin đăng nhập và KHÔNG đẩy. Đây là sao lưu, không thay GitHub.

## CỔNG LINUX CỤC BỘ: ĐÃ XONG (GÓI 10, Giang cho phép cài Ubuntu 24.04; GREEN, xem `docs/CLOUD_GATE_RESULT.md`). Phần dưới là lịch sử:
`wsl -l -v` (2026-11-20) chỉ có `docker-desktop` (Stopped; bản nội bộ của Docker, không dùng được) và `OpenClawGateway` (Running; KHÔNG phải của agent, agent không đụng). KHÔNG có bản Ubuntu/Debian dùng chung ⇒ cổng Linux (S1.c–e) BỎ QUA. Agent không cài hộ và không đổi cấu hình hệ thống. Các bước cho Giang (PowerShell thường):
```text
wsl --install -d Ubuntu-24.04      # cần tạo tên người dùng/mật khẩu WSL; có thể phải khởi động lại
wsl -d Ubuntu-24.04 -- sudo apt update
wsl -d Ubuntu-24.04 -- sudo apt install -y build-essential git curl python3 pkg-config
```
Cần thêm (nhờ phiên Claude làm sau): Rust bằng rustup, LLVM 20 (xem `tools/setup_cloud.sh`) và `%UserProfile%\.wslconfig` đề nghị `[wsl2]` `memory=10GB` `swap=8GB` (preflight cần ≥4 GB bộ nhớ ảo). Ở lần sau: `git clone -c core.autocrlf=false /mnt/d/Grit ~/grit_lin` (KHÔNG chạy trong /mnt/d), `git checkout seed-1.0-rc6` (= `16c37f0`), cổng đầy đủ, so `GATE_TREE` với `b828b9812b7a6bc39d2ed6c1b42edf90c52c5d784188e8f62602c04d4335ffa4`.

## KHẨN: TÀI KHOẢN GITHUB BỊ ĐÌNH CHỈ (GÓI 8, 2026-11-20)
`gh auth status` (chỉ-đọc) ⇒ `The token in keyring is invalid … Error: HTTP 403: Sorry. Your account was suspended`; `git push origin integration:windows-work` ⇒ `HTTP 403`; `gh repo view` ⇒ cùng lỗi. Đây rất có thể là nguyên nhân Actions bị khoá từ GÓI 6 (`Actions has been disabled for this user`, HTTP 422) và là lý do KHÔNG đẩy được `windows-work` ở cuối GÓI 8. Agent KHÔNG thử đường vòng (không đổi tài khoản, không dùng token khác).
**Việc Giang:** đăng nhập github.com bằng trình duyệt, đọc thông báo đình chỉ và làm theo hướng dẫn kháng nghị của GitHub Support (https://support.github.com). Sau khi khôi phục: `gh auth refresh -h github.com`, rồi `git push origin integration:windows-work` (`windows-work` trên GitHub đang ở `4a520f1`; cục bộ `integration` ở HEAD, mã rc6 `16c37f0`, thẻ cục bộ `seed-1.0-rc6`, bundle `grit-final.bundle` ở D:/Grit/.backup và C:/Users/PC/GritBackup).
Hệ quả: GÓI 7 (sau `4a520f1`) và GÓI 8 chưa có commit nào trên GitHub; không CI; không `main`. Mọi bằng chứng của GÓI 8 là cục bộ.


## GIAI ĐOẠN 2: CẦN GIANG DUYỆT (GÓI 9, 2026-11-20; agent KHÔNG đánh dấu thay Giang)
Tài liệu ĐỀ XUẤT: `docs/PHASE2_PLAN.md` (đọc mục 0 trước), `docs/PHASE2_ADR_DRAFTS.md`, `docs/PHASE2_FACTCHECK.md`, `docs/PHASE2_PROPOSAL_OPUS.md`. Chưa có dòng mã Giai đoạn 2 nào; chưa sửa `DECISIONS.md`/`TCB_INTRINSICS.md`.
- [ ] Duyệt CẤU TRÚC Giai đoạn 2 (bốn dòng việc A/B/C/D, giai đoạn con 2.0–2.4, điều kiện vào) — ADR-P2-00.
- [ ] Duyệt QUY TẮC GOM CÚ PHÁP: mọi cú pháp mới vào MỘT đợt edition sau N8; lexer/parser đóng băng lại, fuzz 24 giờ một lần — ADR-P2-01.
- [ ] Xác nhận THỨ TỰ "an toàn trước hiệu năng" (xoá Secret và ADR thiết kế lên đầu; hiệu năng sau) thay cho thứ tự của đề xuất Opus.
- [ ] Duyệt HAI ĐƯỜNG HẠ ở L1 (`MIR→LLVM` cũ và `GLIR→LLVM` mới, cờ chỉ có trong test, bỏ đường cũ chỉ khi trùng trên toàn corpus + bộ sinh) — ADR-P2-02.
- [ ] Quyết định ĐỔI MẶC ĐỊNH dev sang backend khác (L5) — để SAU: chỉ xét khi COFF/Windows xong và cổng xanh cả hai backend.
Liên quan, đang chờ từ GÓI 8: ADR-C12 (spawn/detach) phải giải trước kênh có kiểu (8.D).

## KIỂM FIXTURE BÍ MẬT (GÓI 8 S0f, ngoại tuyến)
Mục loại (i) và (iv) — việc Giang: mở từng tệp, dòng dưới đây; nếu chuỗi giống token/khoá THẬT của bạn thì THU HỒI ngay. Giá trị không được in ở đây (chỉ 4 ký tự đầu).
* (iv) `tests/warn/S101_private_key_block.gt:3` — loại pem, bắt đầu `----****` — không phân loại được
* (iv) `crates/gt-secrets/tests/secrets.rs:36` — loại github, bắt đầu `gith****` — dạng github_pat_ (checksum chưa biết cách tính đã nêu)
* (iv) `crates/gt-secrets/tests/secrets.rs:38` — loại google, bắt đầu `AIza****` — không phân loại được
* (iv) `crates/gt-secrets/tests/secrets.rs:39` — loại other, bắt đầu `sk_l****` — không phân loại được
* (iv) `crates/gt-secrets/tests/secrets.rs:40` — loại pem, bắt đầu `----****` — không phân loại được
* (iv) `crates/gt-secrets/tests/secrets.rs:263` — loại pem, bắt đầu `----****` — không phân loại được


## Email tác giả commit: tên miền gõ sai (GÓI 8 S0h)
* 439 commit mang email `nguyentruonggiang25092007@gamil.com` (tên miền `gamil`, không phải `gmail`); 1 commit `…@gmail.com` (Vecter); 2 commit `gn3305970@gmail.com`. Ai kiểm soát tên miền `gamil.com` có thể xác minh email đó trên GitHub và nhận ghi công cho các commit ấy. Rủi ro THẤP (chỉ ghi công commit, không quyền truy cập).
* **Agent KHÔNG đổi cấu hình git của bạn** (`git config user.email` hiện là `nguyentruonggiang25092007@gamil.com`).
* **Khuyến nghị:** (1) GitHub → Settings → Emails: bật "Keep my email addresses private" và "Block command line pushes that expose my email"; (2) đặt cho commit MỚI: `git config user.email "<ID>+gn3305970-lang@users.noreply.github.com"` (ID là số trong địa chỉ noreply của bạn); (3) thêm `.mailmap` để `git log` hiển thị gọn — KHÔNG viết lại lịch sử (kho công khai).
* Agent chưa tạo `.mailmap` vì không biết chính xác địa chỉ noreply mới của bạn; nếu bạn cho ID, phiên sau tạo được.

## KHẨN / AN TOÀN KHO CÔNG KHAI (GÓI 7 S0, 2026-11-19)
`gh repo view`: **visibility = PUBLIC**, nhánh mặc định `windows-work`, chưa có giấy phép (`licenseInfo = null`), không phải fork. Quét `python tools/secret_scan_history.py` trên **542 commit, 2 391 tệp (mọi nhánh cục bộ + từ xa, 3 thẻ)**; mẫu đã quét: token GitHub/AWS/Google/Slack/sk-, khối khoá riêng, URL có mật khẩu, `authorization: bearer`, gán `password/secret/token/api_key`, chuỗi entropy cao, tên tệp nguy hiểm (`.env`, `*.pem`, `*.key`, `*.pfx`, `*.p12`, `id_rsa*`, `.git-credentials`, `credentials*`, `*.bundle`, `*.sqlite`, hộp thư). KHÔNG phải bằng chứng "không có".
* **Bí mật thật: KHÔNG thấy.** Các phát hiện mẫu khoá (AWS 17, GitHub 5, Slack 3, Google 2, khối khoá riêng 5 trong tổng 49) đều nằm trong fixture của bộ quét S101 của Grit (`crates/gt-secrets/tests/`, `tests/warn/S101_*`, `audit/probes/secret_*`, `docs/errors/S101.md`, `docs/LANGUAGE_REFERENCE.md`). Tệp ghi nhãn "GIẢ"; khoá AWS có dấu `EXAMPLE`; token GitHub/Slack/Google trong fixture có entropy cao nên KHÔNG chứng minh được là giả chỉ bằng đọc (agent không được và không có cách kiểm bằng cách dùng thử). **Việc Giang (1 phút):** mở `tests/warn/S101_github_token.gt` và `crates/gt-secrets/tests/secrets.rs`, xem các chuỗi `ghp_…`, `xoxb-…`, `AIza…` có phải bạn tự gõ bừa không; nếu từng là khoá thật thì THU HỒI ngay (gỡ khỏi lịch sử không đủ vì kho đã công khai).
* **Tên tệp nguy hiểm đã từng thêm:** 0 (4 tệp `*.db` trong `--help/kvstore/…` là dữ liệu test).
* **Thông tin cá nhân (chỉ liệt kê; KHÔNG sửa lịch sử):** (1) email trong metadata commit: 439 commit `Nguyen Truong Giang <ngu****@gamil.com>` (tên miền gõ sai `gamil`), 1 commit `Vecter <ngu****@gmail.com>`, 2 commit `gn3305970@gmail.com` (qua GitHub), 93 commit `Claude <noreply@anthropic.com>`, 7 commit `Codex <codex@local>`. Email thật nay công khai vĩnh viễn trong lịch sử. **Đề xuất:** bật G5 bên dưới và đặt `git config user.email <id>+<tên>@users.noreply.github.com` cho commit MỚI; xoá email cũ khỏi lịch sử là việc viết lại lịch sử, quyết định của Giang, agent không làm. (2) Đường dẫn `C:\Users\PC` (tên tài khoản Windows chung chung) 7 dòng trong 4 commit tài liệu: rủi ro thấp. (3) 3 chuỗi giống số điện thoại là số giả trong test/PROGRESS. (4) Không thấy từ khoá tài chính/đầu tư/ngân hàng/TCBS nào trong dòng thêm vào ở mọi commit.
* **Gói 7 đã làm:** `SECURITY.md`, `.github/ISSUE_TEMPLATE/user_bug.yml`, README nói thẳng mức tin cậy, workflow không còn nội suy `${{ github.event.* }}` vào shell (`docs/CI_REPORT.md`).

## Hai phát hiện soundness của GÓI 7 (đã sửa) và một câu hỏi thiết kế (GÓI 7)
* **FB-C32** (đã sửa, rc5): vòng không dừng được chấp nhận khi hàm có > 24 biến bóng (`docs/FIELD_BUGS.md`).
* **FB-C33** (đã sửa, rc5): đệ quy qua `spawn` không bị kiểm dừng. Do một phiên kiểm toán khác (thư mục `attack/`, KHÔNG được commit; chưa rõ phiên nào) tìm ra và ghi vào `docs/FIELD_BUGS.md` đang sửa dở trong cùng cây làm việc; tôi tái hiện độc lập và sửa. Phiên đó làm cổng của tôi bị vô hiệu một lần (tệp theo dõi đổi giữa lúc chạy). **Việc Giang:** xác nhận phiên `attack/` là của bạn; thư mục `attack/` còn chưa commit (không có `git add`).
* **Câu hỏi thiết kế (từ `attack/b01_spawn_in_pure_fn.gt`):** hàm khai báo `! ∅` gọi `spawn` vẫn XANH; `docs/TCB_INTRINSICS.md` ghi `spawn` có hiệu ứng `detach`, còn M8 ghi hiệu ứng của lời gọi = hiệu ứng worker trừ `lock`. Hai tài liệu mâu thuẫn. **Đề xuất:** coi `spawn` cần hiệu ứng `detach` ở hàm gọi (siết lại, fail-closed) — đổi ngữ nghĩa nên cần quyết định của bạn; mặc định khi chờ: giữ nguyên.

## Cài đặt GitHub Giang nên chỉnh (agent KHÔNG tự làm; GÓI 7 S0g)
| # | cài đặt | vì sao |
|---|---|---|
| G1 | Settings → Code security: bật **Secret scanning** và **Push protection** | chặn đẩy nhầm khoá thật |
| G2 | Settings → Actions → General → Fork pull request workflows: **Require approval for all outside collaborators** | repo công khai: người ngoài không tự chạy workflow |
| G3 | Settings → Actions → General → Workflow permissions: **Read repository contents** | mặc định chỉ đọc (workflow đã khai `contents: read`) |
| G4 | Settings → Code security: bật **Private vulnerability reporting** | `SECURITY.md` dựa vào mục này |
| G5 | Settings → Emails: Keep my email addresses private + Block command line pushes that expose my email | xem thông tin cá nhân ở trên |

## Giấy phép (GÓI 7 S0f) — quyết định của Giang
Hiện CHƯA có LICENSE ⇒ theo mặc định **mọi quyền được bảo lưu** (người khác xem được mã nhưng không có quyền dùng lại, sửa, phân phối). Agent không chọn thay. Ba hướng: (a) **giữ bảo lưu**: an toàn pháp lý, không ai được dùng, không có cộng đồng; (b) **cho phép dùng lại** như MIT/Apache-2.0: ai cũng dùng/sửa/bán kể cả đóng mã (Apache-2.0 có thêm điều khoản bằng sáng chế); (c) **copyleft** như AGPL-3.0: ai dùng/sửa phải mở mã bản sửa, kể cả khi chạy dịch vụ mạng, cản dùng thương mại đóng. **Mặc định khi chờ:** giữ nguyên, chưa cấp phép.


> **CẬP NHẬT 2026-11-18 (GÓI 3, S0b) — REMOTE:** `git ls-remote origin` (URL cũ `https://github.com/Vecter-Core/Grit.git`) trả nguyên văn `remote: Repository not found.` / `fatal: repository 'https://github.com/Vecter-Core/Grit.git/' not found`. `gh auth status`: đăng nhập `gn3305970-lang`, scopes `repo, workflow, read:org, gist, codespace`. Theo lệnh Giang, kho đúng là `https://github.com/gn3305970-lang/Grit.git` (`gh repo view`: **PRIVATE, ban đầu rỗng**). Đã đẩy `windows-work` tới URL này bằng `git push <url> HEAD:refs/heads/windows-work` (KHÔNG đổi cấu hình `origin`, KHÔNG đẩy thẻ). **Giang làm nếu muốn `origin` trỏ kho mới:** `git remote set-url origin https://github.com/gn3305970-lang/Grit.git` (agent không tự đổi). Kho cũ `Vecter-Core/Grit` có thể đã bị xoá/đổi tên/mất quyền: Giang tự kiểm trên trình duyệt. Sao lưu: bundle ở `D:\Grit\.backup`, bản thứ hai ở `C:\Users\PC\GritBackup`, bản thứ ba là kho GitHub riêng tư.

Mỗi mục: đề xuất của kiến trúc sư/agent và **hành vi mặc định trong lúc chờ**. Agent làm tiếp mọi việc không phụ thuộc mục này.

| # | CHỜ GIANG | Đề xuất | Mặc định trong lúc chờ |
|---|---|---|---|
| 1 | ~~Hợp nhất `integration` vào `main` (B7)~~ **ĐÓNG (GÓI 5, 2026-11-19): main = e450c69, đẩy fast-forward 1 lần lên origin (không PR vì remote mới chưa có main; commit [skip ci], không kích CI; CI xanh gần nhất ở 884af7c, sau đó chỉ đổi tài liệu).** Việc của Giang còn lại: đổi nhánh mặc định GitHub sang `main` khi sẵn sàng | — | — |
| 2 | Quyền ghi `D:\Grit_backup` (ACL chỉ cho đọc) | cấp quyền ghi cho tài khoản chạy agent | bundle ở `D:\Grit\.backup` (có `git bundle verify`) + push `origin/windows-work` |
| 3 | Đọc các mục rủi ro (điều kiện #6, B6) | đọc `docs/GIANG_DOC6_READING.md` (≈ 60–90 phút; có dự thảo ADR-C10, chỉ có hiệu lực khi bạn xác nhận đã đọc) | #6 giữ "CHƯA"; agent KHÔNG tự đánh dấu |
| 4 | ~~Xác nhận Q23 và Q35–Q42~~ — **ĐÃ DUYỆT từ trước (2026-10-06/07); nhãn đã sửa 2026-11-18** (`GIANG_DECISIONS_2026-11-18.md` A1–A3) | không cần làm gì thêm (đảo được bằng một tin nhắn) | xong |
| 5 | ~~Điều kiện #9/#10~~ — **chấp nhận có điều kiện theo ỦY QUYỀN 2026-11-18** (B1/B2; `--checks=proven` giữ THỬ NGHIỆM, bị từ chối cùng `release*`, có test) | Giang đảo được; nên liếc lại lần cuối khi đọc `GIANG_DOC6_READING.md` | giữ "đạt có điều kiện" |
| 6 | ~~Rủi ro H7 `Atomic`~~ (B4) — **chấp nhận theo ỦY QUYỀN 2026-11-18**; `SAFETY_CLAIMS` nói "an toàn theo kiểu; chưa có kiểm động đua dữ liệu"; đánh giá TSan ở `docs/TSAN_FEASIBILITY.md` (khuyến nghị làm đường (b) trên Linux sau CI) | Giang đảo được | KHÔNG gọi là đã duyệt/kiểm động |
| 7 | Có làm Giai đoạn 2 / M13 đầy đủ (inlay hint) không (B8) | Seed trước; inlay hint chỉ khi bạn thử VS Code thật (#12) và thấy cần; KHÔNG sửa điều kiện #8 | KHÔNG làm Giai đoạn 2; M13 dừng ở hover/definition/symbol |
| 8 | **R11c — phiên Claude OPUS MỚI** (chỉ đọc; CÙNG HỌ, ĐỘC LẬP YẾU) đọc tấn công checker M6/M7/M8/kiểu và mọi vùng đổi sau hash G1.5 (≈ 63 000 dòng chưa đọc chéo; `docs/CROSS_REVIEW_COVERAGE.md`) | mở phiên Opus với `docs/E_PROMPT_OPUS.md`; kết quả `docs/TCB_CODE_AUDIT_OPUS.md`; **không còn phiên GPT** (Giang chỉ dùng Claude) | RUNLIST "R11c CHỜ PHIÊN OPUS (Giang mở)" |
| 9 | Phiên sạch viết capstone K1–K4 (C3) | **hai phiên Claude khác model**: Haiku 5.5 (yếu) và Sonnet 5.5 (mạnh); chỉ đọc `AI_GUIDE` + `STD_REFERENCE` + đề (`docs/CAPSTONE_PROMPTS.md`, nhật ký `docs/CAPSTONE_LOG_TEMPLATE.md`); đây là kiểm tra CÙNG họ | RUNLIST "CAPSTONE WINDOWS: CHỜ PHIÊN SẠCH" |
| 10 | Chạy `bash tools/fuzz_loop.sh <giờ>` qua đêm (điều kiện #2). **Số thật 2026-11-18** (`python tools/fuzz_total.py`): lexer 1,22 giờ, parser 2,02 giờ tích luỹ (phần lớn trên các băm nguồn CŨ); trên BĂM NGUỒN HIỆN TẠI chỉ lexer 0,05 giờ, parser 0,02 giờ / 24. `fuzz_loop.sh` dừng khi cả hai đạt 24 giờ trên băm hiện tại ⇒ cần ≈ 48 giờ máy | đề xuất: máy Linux đám mây khi có CI (libFuzzer không bị giới hạn phiên ASan như Windows); hoặc 3–5 đêm trên máy Windows | #2 ghi "ĐANG TÍCH LUỸ" kèm số thật; mã lexer/parser đổi thì băm đổi và phải tích luỹ lại |
| 11 | ~~Remote~~ — **XONG (GÓI 4 S0, 2026-11-19):** `git remote rename origin old-origin` rồi `git remote add origin https://github.com/gn3305970-lang/Grit.git` (kho riêng tư). Nhánh mặc định của kho: `windows-work` (Giang tự quyết nếu muốn đổi). **Đảo lại:** `git remote remove origin && git remote rename old-origin origin` | không cần làm gì thêm; `old-origin` (kho cũ, "Repository not found") vẫn còn, KHÔNG xoá | CI GitHub đã chạy xanh: `docs/CI_REPORT.md` |
| 12 | Thử `editors/vscode` trong VS Code thật (5 bước `docs/QUICKSTART_VSCODE.md`) | Giang tự thử, báo kết quả | LSP/VS Code ghi "CHƯA THỬ" |
| 13 | Kiểm Billing (phút Actions còn lại, kho riêng tư ≈ 2 000 phút/tháng; Windows ×2); CI đã dùng ≈ 624 phút ước tính (`docs/CI_REPORT.md`) | vào Settings ▸ Billing, báo agent số còn lại | mỗi push đầy đủ tốn ≈ 130 phút: dùng `[skip ci]` cho commit tài liệu |
| 14 | Chạy fuzz: `bash tools/fuzz_loop.sh 26 --parallel` (lexer và parser đồng thời, ≈ 24 giờ nếu bộ nhớ ảo trống ≥ 10 GB), xem `docs/HANDOFF.md` | lexer/parser ĐÃ ĐÓNG BĂNG (`docs/spec/FREEZE.md`) | điều kiện #2 ghi "ĐANG TÍCH LUỸ" |

| 15 | **GitHub Actions bị tắt cho tài khoản** (GÓI 6): `Actions has been disabled for this user`; mọi run cũ biến mất | kiểm Settings → Billing and plans / Actions; bật lại; xem `docs/CI_REPORT.md` mục GÓI 6; sau đó chạy cổng đầy đủ rc4 (`only=all`, `full=true`) | không có CI mới cho `d1e36b7` trở đi |
| 16 | Đọc `docs/SEED_FINISH_LINE.md` (đích của nhãn Seed: cái gì CHẶN Seed, cái gì chỉ chặn Giai đoạn 2) | đọc rồi quyết ưu tiên | — |
