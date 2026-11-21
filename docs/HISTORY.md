# HISTORY — lịch sử dự án Grit, nén từ nhật ký gốc (GÓI 12, 2026-11-21)

> Thay cho nhật ký chỉ-nối-thêm: `PROGRESS` (849 dòng), `HANDOFF_ARCHIVE`, `RUN_REPORTS` (+`_INDEX`), `RUNLIST`, `CI_REPORT`, `CLOUD_GATE_RESULT`, `GITHUB_SUSPENSION`, `PR_integration_to_main`,
> cùng `docs/archive/` và `audit/`. Tệp này CHỈ giữ dòng thời gian, con số mốc và bài học;
> chi tiết từng lô, lệnh và đầu ra thô còn nguyên trong git: `git show e8a8ae5:docs/<TÊN>.md` (xem mục 6). Số liệu ở đây được chép từ các tệp đó, **không đo lại** (`CHƯA KIỂM` lại).
> Quyết định thiết kế: `docs/DECISIONS.md` (ADR). Lỗi thật: `docs/FIELD_BUGS.md`. Trạng thái hiện tại: `docs/STATUS.md`.

## 1. Tháng 10/2026 — dựng Giai đoạn 1 (M0–M11)
| Mốc | Ngày | Kết quả chính |
|---|---|---|
| M0 đặc tả | 10-05 | Q1–Q14 đóng; `docs/spec/` FINAL; ký hiệu Unicode + bí danh ASCII; làm lại từ đầu (Q1: bỏ Seed cũ) |
| M1 lexer, M2 parser/AST/fmt/diag | 10-05 | tên token chốt (ADR-010); parser Pratt có khôi phục lỗi, giới hạn sâu 256; gt-fmt gắn comment theo token (ADR-016) |
| M3 giải tên | 10-05/06 | ADR-017: tệp = module, prelude là đầu vào tường minh, E0201–E0205 |
| M4 kiểu, M4.5 vùng nhớ + capability | 10-06 | Giang duyệt Q23–Q24; `Arena`, `Cap<T>`, region ở chữ ký, `#lang` |
| M5 MIR + dataflow | 10-06 | ADR-020; một nơi duy nhất đọc AST sau M4; không unwinding |
| M6 mượn/sở hữu | 10-07 | NLL 2018 sound-không-complete; corpus 169 xấu/187 đúng; oracle động độc lập (ADR-024); đạt CÓ ĐIỀU KIỆN (đột biến 361/591/554 sống) |
| M7 nghĩa vụ số học | 10-07/08 | khoảng + zone, bộ giải vét cạn thứ hai; oracle chương trình; đạt CÓ ĐIỀU KIỆN; "luật bộ sinh" (lỗ hổng tìm bằng cách khác ⇒ mở rộng oracle) |
| M8 dừng/hiệu ứng/đa luồng | 10-08/09 | E07xx/E08xx/E05xx; `Mutex`/`Atomic`/`Send`; điều kiện cứng mutability bên trong; đạt CÓ ĐIỀU KIỆN |
| M9 `gritc` + secret + cổng phán quyết | 10-09 | mã thoát phân vùng 0/1/2/3 + 70/71; attestation SHA-256; quét secret VÀNG |
| M10 LLVM + IR Guard + `gt-rt` | 10-12/13 | `Assert` → abort 70; Guard 5 luật; đột biến Guard 40 (38 giết lần đầu) |
| M11 Rig v0 | 10-13 | khoá băm gói; cổng xanh 742 test |

Sau M11 (10-13…10-18): dogfood sáu công cụ (`wc`, `json`, `sha256`, `sortlines`, `wordfreq`, `stats`); `Vec`/`Text` trên `Arena`; ADR-032 (đơn hình hoá, trait, `Ord`, sắp xếp trộn); hồ sơ `release`/`release-o2` + Guard-post (ADR-036); Q79–Q82 duyệt (10-14/15); khớp mẫu qua tham chiếu (Q87/ADR-040); gói `std` (Q84/Q86); rà fail-open (`FAIL_OPEN_AUDIT`, R3: 1292 vị trí đã đọc) và các FB-13/14/15/17/18. Cổng 2026-10-14: 836 đạt/0 đỏ/156 bộ.

## 2. 10/21–10/23 — dòng Seed 1.0 (Linux, container) và hợp nhất
S0 nền (`preflight`, giai đoạn `nul`) → S1 FB-C5/C6, M6 qua giá trị hàm → S2 thư viện/ngôn ngữ cho capstone K1–K4 (`Cap<In>`, ghi tệp, `Vec` API, sắp xếp, `HashMap`, in số) → S3 tài liệu (QUICKSTART, GRIT_BOOK, STD_REFERENCE sinh tự động, `doc_test`) → S4 dùng thật (4 subagent viết K1–K4) → S5 đóng gói (`setup_cloud.sh`, CI, `gritc doctor`) → S6 LSP + schema JSON → S7 cứng hoá (đọc mã đối kháng ⇒ 13 lỗ hổng thật FB-C15…C27 mà corpus/oracle/đột biến trước **không** bắt) → S8 FREEZE/CHANGELOG. Linux và Windows hai dòng làm việc riêng, hợp nhất ở `c3ba7de` (nhánh `integration`).

## 3. 11/12–11/18 — G1, G2, P0–P4 (kiểm chứng nặng sau hợp nhất)
- **G1 (11-12):** hậu merge: F-01…F-08 kiểm đỏ-khi-hoàn-nguyên; flake `llvm_sort` không tái hiện (0/100 seed ⇒ giới hạn trên ≈ 3% tin cậy 95%, **không kết luận "đã hết"**); đọc mã H1–H10/Guard/gt-rt sửa FB-C28/C29/C30; cổng GREEN **1017/0/202** (`c5aa45b`). Reader cùng họ model ⇒ chưa đạt "đọc chéo khác họ".
- **G2 (11-12→17):** probe_diff 20.000 seed (đột biến 361/591/554: 0 vi phạm tiền đề); codegen dev/release-o2: mismatch 0; benchmark `sortlines` Grit ≈ 3,0 s vs Rust `-O` ≈ 0,77 s (≈ ×3,9); capstone HashMap (ADR-C09); R11b (Claude đọc 503 dòng của Codex: 17 mục, 1 xác nhận FB-C31).
- **P0–P4 (11-17/18):** đồng bộ `integration`, cổng **1023/0/204** (`13e2df9`, rc2); phiếu "Giang quyết" ⇒ ủy quyền kiến trúc sư (STATUS mục 8).

## 4. Các gói gần đây (Giang CHỈ dùng Claude từ 11-19)
| Gói | Ngày | Nhãn | Kết quả / việc chính |
|---|---|---|---|
| GÓI 3 | 11-18 | rc3 | cổng **1027/0/206** (`89438fd`); đẩy `windows-work` + thẻ `seed-1.0-rc3` |
| GÓI 4 | 11-19 | rc3 | `origin` đổi kho; CI GitHub xanh (Linux đầy đủ 1024/0/206 + capstone; Windows bản rút gọn); TSan 8 mẫu × 50 lần: không phát hiện |
| GÓI 5 | 11-19 | rc3 | `main` fast-forward một lần; TSan có đối chứng (13–30/30 phát hiện, âm 0/30); độ phủ nhánh hàm quyết định (730 nhánh, 100 chưa chạy) |
| GÓI 6 | 11-19 | rc4 | Linux GREEN **1028/0/207** (CI run `44365492150`, `1793b7f`); `LIMITS_AUDIT` (17 giới hạn); 3 app mẫu `apps/`; **Actions bị khoá (HTTP 422)** |
| GÓI 7 | 11-19 | rc5 | kho công khai; quét 542 commit: không bí mật thật; `SECURITY.md`; **FB-C32** (vòng không dừng được chấp nhận khi > 24 biến bóng) và **FB-C33** (đệ quy qua `spawn`) sửa |
| GÓI 8 | 11-20 | rc6 | **FB-C34** (spawn qua generic); ICE 33 hợp đồng; E1001 oan tuple lồng; Windows GREEN **1045/0/213** (`16c37f0`); **GitHub đình chỉ tài khoản (HTTP 403)** |
| GÓI 9 | 11-20 | rc6 | CHỈ tài liệu: `PHASE2_PLAN`, `PHASE2_ADR_DRAFTS`, `PHASE2_FACTCHECK` (44 khẳng định: 29 đúng, 7 một phần, 1 sai, 7 chưa kiểm được) |
| GÓI 10 | 11-20 | rc6 | Linux GREEN cục bộ (WSL2, `16c37f0`, `GATE_TREE` khớp Windows); `make_export.sh` (bản sao ngoài máy, `git fsck` rc=0); `gate_clean.sh`; đột biến gt-term: 27/110 sống sót |
| GÓI 11 | 11-20 | rc7 | **FB-C35** (đệ quy qua thân hợp đồng; `gt-term` đổi); Windows GREEN **1054/0/215** (`GATE_TREE=c0d7b267…`, 35 m 38 s liền mạch cuối; hai lần đỏ do lỗi TỰ GÂY: clippy `collapsible_if`, snapshot borrowck); S3/S4 chưa làm |

Bài học lặp lại: (1) mỗi lỗ hổng soundness thật đều bị tìm bằng ĐỌC MÃ hoặc bằng oracle mới, không phải bằng test có sẵn ⇒ luật bộ sinh (AGENTS.md); (2) cổng đỏ do lỗi tự gây (fmt/clippy/snapshot) xảy ra thường ⇒ luôn chạy cổng trong CLONE sạch; (3) "đạt có điều kiện" chỉ nghĩa bằng chứng thực nghiệm đủ, không phải chứng minh.

## 5. Sự cố GitHub (2026-11-19/20)
Kho `gn3305970-lang/Grit` từng là origin của GÓI 4–8. Ngày 11-19 Actions bị khoá (HTTP 422); ngày 11-20 tài khoản bị đình chỉ (`HTTP 403: Your account was suspended`), push và `gh` đều thất bại; mọi commit sau `4a520f1` chỉ có cục bộ + bundle. Giang xử lý qua GitHub Support. Không có CI nào cho rc5–rc7. Origin trong clone hiện tại (2026-11-21) là `Vecter-Core/Grit`.

## 6. Tài liệu đã gộp — tìm bản gốc ở đâu
Mọi tệp dưới đây bị xoá ở GÓI 12 (lý do và danh sách đầy đủ: `docs/DELETIONS.md`); bản gốc: `git show e8a8ae5:<đường dẫn>`.
| Đã gộp vào | Tệp gốc |
|---|---|
| `STATUS.md` | `HANDOFF`, `PROJECT_STATUS`, `READINESS`, `PENDING_GIANG`, `SEED_FINISH_LINE`, `KNOWN_LIMITATIONS_DIGEST` |
| `HISTORY.md` (tệp này) | `PROGRESS`, `HANDOFF_ARCHIVE`, `RUN_REPORTS`, `RUN_REPORTS_INDEX`, `RUNLIST`, `CI_REPORT`, `CLOUD_GATE_RESULT`, `GITHUB_SUSPENSION`, `PR_integration_to_main` |
| `PHASE2_PLAN.md` | `PHASE2_PROPOSAL_OPUS` (đã được `PHASE2_PLAN` thay thế) |
| bỏ hẳn (quy trình đã xong) | `docs/archive/` (87 tệp: `MERGE_NOTES`, `SEED_PLAN`, báo cáo tấn công hộp đen, script nháp), `audit/` (kiểm toán ngoài một lần: 274 tệp, ≈ 181 chương trình thăm dò; các phát hiện đã thành hồi quy test/FB), `E_PROMPT_CLAUDE` (R11b xong), `E_PROMPT_GPT` (Giang không còn dùng GPT) |
| GIỮ vì còn chức năng | `NEXT_PRODUCT_BATCH` (việc mã đang chờ), `APPS_LOG`, `GIANG_DECISIONS_2026-11-18` + `GIANG_DOC6_READING` (hồ sơ ủy quyền/hướng dẫn đọc của Giang), `E_PROMPT_ATTACK`/`E_PROMPT_OPUS` (kịch bản phiên tấn công/R11c còn chờ), `CLOUD_GATE_PROMPT`, `CAPSTONE_PROMPTS`, `CAPSTONE_LOG*`, `G2_CLEAN_SESSION` |

## 7. GÓI 12 (2026-11-21) — dọn test, nén tài liệu, ROADMAP
Xem `docs/STATUS.md` mục 2 và danh sách thay đổi trong lịch sử git của nhánh `claude/cool-cray-hqha6n`. Số đo thời gian test và kết quả từng bước được ghi thêm ở mục này khi hoàn tất từng bước.
