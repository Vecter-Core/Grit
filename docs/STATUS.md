# STATUS — trạng thái hiện tại của Grit (ĐỌC TỆP NÀY ĐẦU TIÊN)

> Lập 2026-11-21 (GÓI 12) bằng cách GỘP và nén: `HANDOFF`, `PROJECT_STATUS`, `READINESS`, `PENDING_GIANG`, `SEED_FINISH_LINE`, `KNOWN_LIMITATIONS_DIGEST`. Cập nhật tại commit 44b7fd3.
> Số liệu bên dưới được CHÉP từ các tài liệu đó (ngày ghi kèm), **không đo lại** trừ chỗ ghi "ĐO 2026-11-21". Bản gốc còn nguyên trong git: `git show e8a8ae5:docs/<TÊN>.md`.
> Lịch sử từng gói: `docs/HISTORY.md`. Kế hoạch tiếp: `docs/ROADMAP.md` (gần) và `docs/PHASE2_PLAN.md` (Giai đoạn 2). Giới hạn kỹ thuật chi tiết: `docs/KNOWN_LIMITATIONS.md`.

## 1. Dự án trong 6 dòng
1. **Grit**: ngôn ngữ (`.gt`) an toàn theo thiết kế, viết cho AI viết code; ký hiệu toán/logic (mỗi ký hiệu có bí danh ASCII). Chủ dự án **Giang** (trả lời bằng TIẾNG VIỆT).
2. `gritc` (Rust, workspace gồm 21 crate `gt-*` + `rig`; `runtime/gt-rt` là workspace riêng): kiểu → MIR → mượn/sở hữu → nghĩa vụ số học → dừng/hiệu ứng/đa luồng → quét secret → PHÁN QUYẾT → LLVM → IR Guard.
3. **Fail-closed**: ĐỎ (`E…`) hoặc ICE (`E9901`) ⇒ không binary, không chạy; "không biết" = chưa chứng minh = không qua; VÀNG (`S…`) vẫn chạy và cảnh báo.
4. Bằng chứng an toàn = corpus có oracle + đột biến + đọc mã (cùng họ model). **KHÔNG phải chứng minh hình thức; chưa có kiểm toán độc lập** (`docs/SAFETY_CLAIMS.md`).
5. M0–M11 có mã; có `lib/std`, 6 công cụ dogfood (`examples/`), 4 capstone (`capstone/`), 3 app mẫu (`apps/`), LSP mức tối thiểu. **Giai đoạn 2 chưa bắt đầu và CẤM mở mã khi chưa có lệnh của Giang.**
6. Vai trò: Giang = chủ dự án; Claude = kiến trúc sư; người thực thi = Claude Sonnet 5.5 (Giang CHỈ dùng Claude từ 2026-11-19).

## 2. Nhãn và cổng
Nhãn hiện hành: **`1.0.0-rc7`** (ứng viên, **CHƯA** phải `1.0.0-seed`). Nguồn sự thật: `version` của `crates/gt-driver/Cargo.toml` (`tools/version_docs.py` khoá các tài liệu khớp mã).

| Mục | Trạng thái (nguồn, ngày) |
|---|---|
| Cổng đầy đủ Windows rc7 | **GREEN** — `GATE_TREE=c0d7b267…`, 1054 test / 0 lỗi / 215 bộ (HISTORY GÓI 11, 2026-11-20) |
| Cổng đầy đủ Linux rc7 | **CHƯA chạy** (rc6 GREEN ở WSL2 trên `16c37f0`, `GATE_TREE` khớp Windows; 2026-11-20) |
| CI GitHub | **KHOÁ**: tài khoản `gn3305970-lang` bị đình chỉ (HTTP 403), Actions HTTP 422 (2026-11-20). Run xanh cuối: Linux rc4 trên `1793b7f` (1028/0/207, 2026-11-19) |
| Nhánh (ĐO 2026-11-21 trong clone này) | `origin` = `Vecter-Core/Grit`; `main` = `origin/main` = `e8a8ae5` = điểm xuất phát của nhánh phiên `claude/cool-cray-hqha6n`. Tài liệu cũ ghi `main` = `e450c69`/`17beed4` cho kho `gn3305970-lang` (đã đình chỉ): **hai kho, hai số, chưa đối chiếu được** — `CHƯA KIỂM`. Quy tắc `main` (chỉ fast-forward sau CI xanh; chỉ Giang đẩy): AGENTS.md |
| Máy này (đo 2026-11-21) | Linux, `cargo 1.98.1` (đúng bản ghim): dựng `gritc` **không-LLVM** được; thiếu LLVM 20 (có 17/18) ⇒ backend `llvm` và test chạy nhị phân thật **CHƯA dựng được ở đây** |
| Test không-LLVM (ĐO 2026-11-21) | trước dọn: 194 bộ/903 test đạt/0 lỗi (≈ 2.329 s; 5 bộ chiếm 82% thời gian); sau dọn: 821 test (−111). Chi tiết, cách đo và việc còn dở: `docs/HISTORY.md` mục 7. Cổng đầy đủ và test LLVM **CHƯA chạy** |

## 3. Luật làm việc (bản đầy đủ: `AGENTS.md`, `.claude/skills/grit-contract.md`)
- Chỉ báo số liệu THẬT từ lệnh đã chạy; ghi `CHƯA KIỂM` cho phần chưa kiểm. Không báo xong khi chưa có cổng liền mạch xanh trên đúng mã đó.
- Lỗi soundness/false negative = lỗi nghiêm trọng nhất: test ĐỎ trước, sửa sau trong commit riêng, ghi `docs/FIELD_BUGS.md`, mở rộng bộ sinh/oracle, kiểm bằng cách tắt bản sửa.
- Cổng đỏ ⇒ sửa rồi chạy LẠI TOÀN BỘ cổng. Không `#[ignore]`/`todo!()`/stub để qua. Không đổi TCB/READINESS khi chưa có cổng xanh.
- Xoá tệp đã theo dõi: ghi `docs/DELETIONS.md` CÙNG commit (`tools/commit_check.sh`). Không `reset --hard`, `clean`, force-push, push `main`.
- Một Cargo tại một thời điểm; `CARGO_BUILD_JOBS=2`, `--test-threads=2`; lô nặng ≤ 15 phút; `bash tools/preflight.sh` trước mỗi lô.
- Không thêm cú pháp/từ khoá/intrinsic nếu chưa có ADR + hợp đồng TCB. Phiên viết chương trình Grit chỉ sửa `apps/` và ghi `docs/USER_BUGS.md`.

## 4. Điều kiện chuyển Giai đoạn (GRIT_PLAN §7) — 12 điều kiện (kiểm 2026-11-18, giữ nguyên tới nay)
Đạt: **#4** (mã lỗi có tài liệu + test), **#7** (Obligation IR đóng băng, ADR-039), **#11** (E0601/E0307), **#12** (capability). Đạt có điều kiện: **#1** (test xanh; còn false negative tiềm ẩn), **#3** (6 dogfood + benchmark, một máy), **#5** (tái lập ở `dev`; chưa `release`), **#9** (đột biến 361/591/554: chấp nhận bằng lập luận + 24.143 chương trình đo máy), **#10** (191 đột biến M7 nhóm nguy hiểm: 30 chỉ có lập luận; `--checks=all` mặc định). **Chưa**: **#2** (fuzz lexer 1,2 h / parser 2,0 h trên 24 h; `fuzz.yml` chưa có `check`), **#6** (Giang đọc KNOWN_LIMITATIONS), **#8** (schema JSON đã khoá; LSP mới có chẩn đoán/định dạng/hover/định nghĩa cấp item). **Phase 1 CHƯA xong.**

## 5. Điều kiện nhãn `1.0.0-seed` (sáu mục A1–A6; định nghĩa: mục 9 dưới)
| Mục | Trạng thái | Ghi chú |
|---|---|---|
| A1 dùng được | có điều kiện | dogfood + capstone chạy đúng; capstone K1/K2/K4 trên Windows và phiên sạch viết lại **CHƯA** |
| A2 an toàn | **CHƯA** | fuzz ≪ 24 h; không TSan/loom đầy đủ; đọc chéo khác họ chỉ ở `gt-rt` + `gt-guard` (≈ 2.650 dòng); ≈ 63.000 dòng checker chưa đọc chéo |
| A3 cổng xanh liền mạch | đạt | Windows rc7 xanh (mục 2); Linux rc7 **CHƯA** |
| A4 tái lập & di động | có điều kiện | exe Grit tái lập (dev, release, release-o2); tái lập của chính `gritc` chưa đo |
| A5 tài liệu kiểm được | đạt | `tools/doc_test.py` (207 khối) + `STD_REFERENCE` khoá từng byte |
| A6 đóng băng | có điều kiện | `docs/spec/FREEZE.md`; M13 chưa đủ |

## 6. TCB (thứ phải tin mà compiler không tự kiểm)
- Intrinsic/runtime: 42 dòng ở `docs/TCB_INTRINSICS.md`, mọi dòng `ỦY QUYỀN: KIẾN TRÚC SƯ DUYỆT HỢP ĐỒNG` — **chỉ nghĩa hợp đồng được chấp nhận, KHÔNG phải kiểm toán mã**. Nhóm CAO (H1–H10) đã qua đọc mã đối kháng cùng họ model; **chưa có người/model khác họ đọc độc lập toàn bộ** (`docs/TCB_CODE_AUDIT*.md`, `docs/CROSS_REVIEW_COVERAGE.md`).
- Thành phần TCB khác: M6/M7/M8 checker, M9 secrets, `gt-guard`, `gt-llvm/src/shim.rs` (FFI LLVM), LLVM 20, linker, OS.
- Chỉ đổi dòng TCB/READINESS khi có bằng chứng và cổng liền mạch xanh trên đúng mã đó.

## 7. CHỜ GIANG (agent không đánh dấu thay)
**KHẨN**: (1) chép bản sao ngoài máy (`tools/make_export.sh` ⇒ thư mục export, ≈ 15 MB) ra USB/kho khác — mọi commit sau `4a520f1` chỉ có cục bộ vì GitHub bị đình chỉ (`docs/HISTORY.md`, mục GitHub); (2) xử lý đình chỉ qua GitHub Support rồi `gh auth refresh`; (3) kiểm fixture bí mật: các chuỗi giả trong `tests/warn/S101_*` và `crates/gt-secrets/tests/secrets.rs` — nếu trùng khoá THẬT của bạn thì thu hồi (quét 542 commit 2026-11-19: không thấy bí mật thật); (4) email tác giả commit gõ sai tên miền: xem `git show e8a8ae5:docs/PENDING_GIANG.md` mục "Email tác giả commit" (không chép vào đây).
**Quyết định**: giấy phép (repo đã có `LICENSE` GPL-3.0-only; xác nhận); 5 điểm Giai đoạn 2 (`docs/PHASE2_PLAN.md`: cấu trúc, gom cú pháp, thứ tự, hai đường hạ, đổi mặc định — đã ỦY QUYỀN 2026-11-20, đảo được); ADR-C12 (`spawn`/`detach`); duyệt ROADMAP này (`docs/ROADMAP.md`).
**Việc chỉ người làm được**: đọc `KNOWN_LIMITATIONS.md` (#6); thử `editors/vscode` trong VS Code thật (`docs/QUICKSTART_VSCODE.md`); mở hai phiên capstone sạch (Haiku 5.5, Sonnet 5.5; `docs/CAPSTONE.md`); fuzz qua đêm `bash tools/fuzz_loop.sh 26 --parallel` (đóng WSL/trình duyệt để bộ nhớ ảo ≥ 4 GB); cài đặt GitHub (Secret scanning, Push protection, Private vulnerability reporting, Actions chỉ-đọc).
**Việc AI làm được khi có lệnh**: cổng Linux rc7; S3 (fuzz WSL + TSan); S4 (độ phủ nhóm D); fuzz typechecker trong `fuzz.yml`; phiên Opus đọc mã checker (R11c).

## 8. Quyết định đã ỦY QUYỀN cho kiến trúc sư (2026-11-18/20; Giang đảo được bằng một tin nhắn)
- #9 (361/591/554) chấp nhận có điều kiện; #10 (30 đột biến M7 chỉ có lập luận) chấp nhận có điều kiện, **`--checks=proven` giữ THỬ NGHIỆM** (bị từ chối cùng `release*`, không làm mặc định).
- Đọc chéo khác họ: "đạt có điều kiện" CHỈ cho intrinsic/runtime/Guard; không nâng thành "TCB đã kiểm toán". H7/H8 không có TSan/loom: chấp nhận rủi ro cho Seed; nói thẳng trong `SAFETY_CLAIMS`.
- ADR-C09 `HashMap` duyệt (a). M13: nhãn Seed trước; inlay hint chỉ khi Giang thử VS Code thật. `main` hợp nhất qua fast-forward sau CI xanh (đã làm GÓI 5).
- Giai đoạn 2 = bốn dòng A (bảo đảm) B (độc lập backend) C (dùng được cho AI) D (hiệu năng); mọi cú pháp mới gom MỘT đợt "edition"; an toàn trước hiệu năng; hai đường hạ ở L1; chưa đổi mặc định dev.
- ADR-C12 = phương án (b) (tài liệu, không sửa prelude); ADR-C13 (declassify) hoãn; N9 (nhãn đối số) hoãn tới ≥ 3 lỗi đảo đối số xác nhận trong ≥ 2 phiên capstone.
- Q75 stdout không đệm (giữ); Q79 `f32` cắt khỏi Phase 1; Q80 cấm `%` số thực; Q81 không mở `instcombine`; Q82 O2 duyệt có điều kiện; Q85 `?` hoãn.

## 9. Môi trường và lệnh (chi tiết: `docs/ENVIRONMENT.md`)
```sh
bash tools/preflight.sh <thư_mục>                     # trước MỖI lô nặng
bash tools/gate_clean.sh <commit> <thư_mục_ngoài_kho> # cổng đầy đủ, trong CLONE sạch (≈ 40 phút–2 giờ)
cargo build --release --manifest-path runtime/gt-rt/Cargo.toml   # trước test LLVM
cargo test -p <crate> --test <tệp> -- --test-threads=2
python3 tools/check_expectations.py                   # corpus khớp bảng mã lỗi
```
- Windows: Git Bash `C:\Program Files\Git\bin\bash.exe`; LLVM 20.1.8 (`GRIT_LLVM_DIR`); Linux: `llvm-20-dev`/`libLLVM-20.so`, `cc` (`tools/setup_cloud.sh`). Toolchain ghim `rustc 1.98.1` (`rust-toolchain.toml`).
- Chèn lỗi để thử cổng: `GRITC_FAULT=panic:<pass>|skip:<pass>|types:select-impl|ir:*|post:*|mir:pat-*`.
- Đột biến: `tools/mut.py` (≥ một nửa tự động cho M6/M7/M8/Guard; ghi nguồn `tự động`/`tay`).
- Sau khi clone: `bash tools/install_hooks.sh` (hook pre-commit kiểm `DELETIONS`).
- Điều kiện định nghĩa nhãn Seed (A1–A6) còn trong git: `git show e8a8ae5:docs/archive/SEED_PLAN.md`.

## 10. Bản đồ tài liệu (sau GÓI 12)
| Cần gì | Đọc |
|---|---|
| Luật, kế hoạch gốc | `AGENTS.md`, `GRIT_PLAN.md`, `.claude/skills/grit-contract.md`, `Giang quyết.md` |
| Trạng thái / lịch sử / kế hoạch | `docs/STATUS.md` (tệp này), `docs/HISTORY.md`, `docs/ROADMAP.md`, `docs/NEXT_PRODUCT_BATCH.md` (việc MÃ đang chờ), `docs/PHASE2_PLAN.md`, `docs/PHASE2_ADR_DRAFTS.md` |
| Quyết định | `docs/DECISIONS.md` (ADR), `docs/OPEN_QUESTIONS.md`, `docs/GIANG_DECISIONS_2026-11-18.md` (ủy quyền), `docs/GIANG_DOC6_READING.md` (hướng dẫn đọc #6) |
| Kịch bản phiên còn chờ | `docs/E_PROMPT_OPUS.md` (R11c), `docs/E_PROMPT_ATTACK.md` + `tools/make_attack_kit.sh`, `docs/CAPSTONE_PROMPTS.md`, `docs/CLOUD_GATE_PROMPT.md` |
| Lỗi và giới hạn | `docs/FIELD_BUGS.md`, `docs/USER_BUGS.md`, `docs/KNOWN_LIMITATIONS.md`, `docs/SEED_LIMITATIONS.md`, `docs/SAFETY_CLAIMS.md` |
| TCB | `docs/TCB_INTRINSICS.md`, `docs/TCB_REVIEW_GUIDE.md`, `docs/TCB_CODE_AUDIT*.md` |
| Đặc tả, mã lỗi | `docs/spec/`, `docs/errors/` |
| Viết Grit | `docs/START_CODING.md`, `docs/AI_GUIDE.md`, `docs/AI_PITFALLS.md`, `docs/LANGUAGE_REFERENCE.md`, `docs/STD_REFERENCE.md` (SINH), `docs/GRIT_BOOK.md` |
| Số liệu đo (công cụ ghi) | `docs/*.jsonl`, `docs/fail_open_*`, `docs/platform_tests_windows.list` |
