# HANDOFF — đọc tệp này đầu tiên để tiếp tục dự án (bất kỳ phiên/model nào)

> Cập nhật SAU MỖI bước nhỏ (mục 2 "Trạng thái" và mục 3 "Việc tiếp theo"). Nếu bị ngắt giữa chừng, mọi thứ ở đây + `git log` là đủ để tiếp tục.

## 0. Dự án trong 5 dòng
1. **Grit**: ngôn ngữ lập trình mới (`.gt`) **thiết kế cho AI viết code**; ký hiệu toán học/logic ngắn gọn (có bí danh ASCII). Chủ dự án: **Giang** (trả lời bằng TIẾNG VIỆT).
2. Compiler `gritc` (Phase 1 "Seed") viết bằng **Rust**, workspace `D:\Grit`, nhiều crate `gt-*` (xem `crates/TASK.md`, GRIT_PLAN §6.1).
3. Nguyên tắc cốt lõi: **fail-closed** — ĐỎ (E…) hoặc ICE ⇒ không sinh binary, không chạy; "không biết" = chưa chứng minh = không được qua; VÀNG (S…, secret) vẫn chạy, in cảnh báo.
4. An toàn bằng: kiểu + sở hữu/mượn (M6) + nghĩa vụ số học bằng khoảng+zone (M7) + dừng/hiệu ứng/đa luồng (M8) + bộ quét secret (M9); bằng chứng = corpus fail/pass + oracle động + đột biến, KHÔNG phải chứng minh hình thức.
5. Đang làm: **M10** (codegen LLVM, IR Guard, runtime); M0–M9 đã có mã.

**Tệp BẮT BUỘC đọc trước khi làm gì:** `GRIT_PLAN.md` (kế hoạch, D1–D29, §7 điều kiện cứng), `.claude/skills/grit-contract.md` (luật làm việc), `docs/DECISIONS.md` (ADR-001…029), `docs/OPEN_QUESTIONS.md`, `docs/KNOWN_LIMITATIONS.md`, `docs/PROGRESS.md`, `docs/ENVIRONMENT.md`. Khi làm đúng một mảng: `docs/spec/semantics.md`, `docs/spec/error_codes.md`, `docs/errors/<mã>.md`.

## 1. Luật làm việc (Giang quy định; vi phạm = lỗi nghiêm trọng)
- **Phản biện chủ động:** thấy lệnh sai kỹ thuật/quá nặng cho máy thì NÓI THẲNG lý do và hệ quả, đừng im lặng làm theo. Không tự duyệt thay Giang; quyết định cài đặt nội bộ thì chọn phương án an toàn nhất (fail-closed) và ghi lý do vào ADR; chỉ HỎI khi phải thêm cú pháp/từ khoá mới của ngôn ngữ.
- **Không báo "xong" khi chưa chạy thật.** Số liệu trong báo cáo phải là số THẬT lấy từ lần chạy thật (test, oracle, đột biến), kèm ghi rõ phần chưa chạy/chưa đủ. Sai sót của chính mình thì nói thẳng (đã có tiền lệ: commit làm hỏng corpus gt-resolve; vector SHA-256 chép sai).
- **Không qua loa:** không `todo!()`, không `#[ignore]` để qua cổng, không stub giả vờ chạy; chưa làm được ⇒ ghi `docs/KNOWN_LIMITATIONS.md` và để compiler TỪ CHỐI case đó.
- **False negative của kiểm tra an toàn = bug nghiêm trọng nhất:** thêm vào corpus vĩnh viễn; luật bộ sinh (grit-contract §3.3): lỗ hổng tìm bằng cách khác ⇒ mở rộng bộ sinh/oracle để tự tìm được, kiểm bằng cách TẮT bản sửa, ghi kết quả vào PROGRESS.
- **Mỗi mã lỗi mới:** `docs/errors/<mã>.md` (≥3 fail + ≥3 pass), dòng trong `docs/spec/error_codes.md`, sổ `crates/gt-diag/src/code.rs` (KNOWN), `tools/check_expectations.py` (DOC_REQUIRED / S_DOC_REQUIRED / NOTE_DOC_REQUIRED), mẫu trong `tests/{fail,pass,warn}` có dòng `// expect: …`.
- **Commit từng bước nhỏ**, chỉ commit khi Giang yêu cầu hoặc theo lệnh đang áp dụng; kết thúc commit message bằng dòng `Co-Authored-By` theo hướng dẫn của hệ thống. Không phá hoại (`reset --hard`, xoá tệp lạ) khi chưa xem kỹ.
- Đột biến (mutation) đầy đủ cho M6/M7/M8/M10-guard; M4/M5 chỉ có ngoại lệ có mục tiêu (xem grit-contract §3.3).
- Python in tiếng Việt cần `PYTHONIOENCODING=utf-8`; KHÔNG dùng `\` trong chuỗi Python gửi qua heredoc shell và tránh heredoc dài có dấu `'` (lệnh bị cắt): dùng công cụ Write/Edit cho tệp lớn.

## 2. Trạng thái
- **Milestone xong về mã:** M0–M9. M6, M7, M8 chỉ **"đạt có điều kiện"** (xem mục 5). M9 (gritc, cổng phán quyết, secret) cài xong, cổng đầy đủ xanh (687 test), Giang đã duyệt hướng ADR-028.
- **Commit cuối trước tệp này:** `76d722f` ("M9: gritc, cổng phán quyết, bộ quét secret, E1001; ADR-029 (M10) dự thảo"). Cây làm việc sạch trước khi viết HANDOFF.
- **Đang ở:** vừa viết HANDOFF; ADR-029 (M10) đã được Giang duyệt kèm điều kiện (mục 4). **Chưa có dòng mã M10 nào.** Bước đang làm: xem mục 3 (đánh dấu `[ĐANG LÀM]`).
- **Số thật gần nhất:** cổng đầy đủ `tools/gate.sh`: 687 test pass / 0 lỗi; 67 mã lỗi; 788 tệp `.gt` (391 fail, 13 warn, 358 pass). `target/` ~5,8 GB, `.git/` ~3,3 MB.
- Crate hiện có: gt-span, gt-lexer, gt-diag, gt-ast, gt-parser, gt-stack, gt-fmt, gt-resolve, gt-types, gt-mir (kèm `interp` = bộ chạy tham chiếu), gt-dataflow, gt-borrowck, gt-oblig (kèm gt-edge gộp, `unsafe_check`), gt-effects, gt-conc, gt-term, gt-secrets, gt-driver (lib + binary `gritc`). `gt-edge` chưa tách (lệch kế hoạch, đã ghi).

## 3. Việc tiếp theo (theo thứ tự; mỗi bước nhỏ = một commit + cập nhật mục 2/3 + kiểm nhẹ: `cargo fmt`, test các crate bị đổi)
1. `[XONG 2026-10-12]` Ghi SHA-256 gói LLVM 20.1.8 + `LLVM-C.dll` vào ADR-029 và `docs/ENVIRONMENT.md` (băm tự tính khi tải; GitHub công bố `f229769f11d6a6edc8ada599c0cda964b7dee6ab1a08c6cf9dd7f513e85b107f`), ghi: gritc cần `LLVM-C.dll` trên PATH khi chạy; binary do gritc sinh KHÔNG phụ thuộc `LLVM-C.dll` (phải kiểm bằng `dumpbin /dependents`); `libxml2s.lib` rỗng chỉ là workaround dự phòng.
2. `[XONG 2026-10-12]` ADR-029 phụ lục: Q59–Q65 quyết định cuối; vấn đề `gritc.exe` không khởi động khi thiếu `LLVM-C.dll` ⇒ cargo feature `llvm` + nạp DLL trễ.
3. `[ĐANG LÀM]` **Spike bước 0** (≤ 1 giờ, ngoài repo hoặc crate tạm): `inkwell 0.10.0` feature `llvm20-1-no-llvm-linking` + `build.rs` phát `rustc-link-search=native=$GRIT_LLVM_DIR/lib` và `rustc-link-lib=dylib=LLVM-C`; in IR + xuất object. Hỏng ⇒ dùng `llvm-sys` thô hoặc workaround `libxml2s.lib` rỗng; ghi kết quả vào ADR TRƯỚC khi viết mã codegen.
4. `runtime/gt-rt` (staticlib, `panic = abort`): `gritrt_start`, `gritrt_abort(kind)` (mã thoát **70**), mutex, tạo luồng.
5. `gt-codegen-llvm` (crate DUY NHẤT có `unsafe`/FFI): số nguyên/bool/số học (wrap), `Assert` ⇒ kiểm + `gritrt_abort`; `Unreachable` ⇒ abort, **KHÔNG BAO GIỜ** `unreachable` của LLVM (chỉ sau `call @gritrt_abort noreturn`).
6. Luồng điều khiển, hàm, struct/enum/tuple/mảng/tham chiếu; `Atomic` ⇒ atomics LLVM thật theo ordering E0503; `Mutex::with` ⇒ khoá thật qua gt-rt; `spawn` ⇒ luồng HĐH thật.
7. `gt-guard` (IR Guard, mã E1101): LLVM verifier, đối chiếu attestation, thao tác nhạy cảm có Obligation-ID đã giải, từ chối `unreachable` không do codegen cố ý.
8. `gritc build -o prog.exe` / `run` với `--backend=llvm|interpreter` (mặc định theo ADR), JSON `"backend":"llvm"`; test mức tiến trình: ĐỎ/ICE ⇒ KHÔNG có tệp exe và không chạy; abort ⇒ mã 70.
9. 3 chương trình mẫu chạy được thật bằng binary (số học/vòng lặp/đệ quy có `↓`; mảng/lát có biên; đa luồng Atomic + Mutex tĩnh + spawn) + test đối chiếu `interpreter ≡ llvm` + benchmark nền (số thật).
10. Cổng đầy đủ **MỘT lần** khi M10 xong; báo ngắn; cập nhật HANDOFF/PROGRESS; dừng chờ duyệt.

## 4. Quyết định đã có cho M10 (Giang duyệt 2026-10-12)
LLVM **20.1.8** bản dựng sẵn tại `D:\Tools\llvm-20.1.8`; binding **inkwell 0.10.0**; liên kết ĐỘNG `LLVM-C.lib`↔`LLVM-C.dll`. Giữ nguyên ràng buộc: `Assert` ⇒ kiểm lúc chạy rồi abort (D3); `Unreachable` ⇒ abort; IR Guard từ chối `unreachable` không do codegen sinh có chủ đích; Mutex/Atomic* ⇒ atomics LLVM thật; `spawn` ⇒ luồng HĐH thật qua gt-rt tối thiểu; mã thoát abort riêng (70); **thông dịch MIR KHÔNG phải bằng chứng cho codegen**; mặc định `--checks=all`. Đột biến 361/591/554 KHÔNG chặn M10.

## 5. Điều kiện cứng còn mở / đột biến chưa giải quyết (KHÔNG được quên)
- **GRIT_PLAN §7 điểm 10:** 69 đột biến "xoá thêm check" nguy hiểm ở `gt-oblig/src/analysis.rs` (62) + `driver.rs` (7) chưa giải quyết ⇒ `--checks=all` là MẶC ĐỊNH, KHÔNG đổi sang `proven` khi chưa giải quyết + Giang duyệt (danh sách: KNOWN_LIMITATIONS mục "gt-oblig").
- **Điều kiện cứng mutability bên trong (§7 điểm 9–10):** 238 đã giết; **361/591/554** (`gt-borrowck/src/regions.rs`) còn sống — cả ba làm checker CHẶT hơn (bảng ở ADR-027 §0); Giang chưa chấp nhận lập luận; M8 = "đạt có điều kiện". Quy tắc bất biến cho Mutex/Atomic chưa chứng minh là cần bằng oracle (tắt chỉ Mutex/Atomic ⇒ 0 âm tính giả; tắt cả nhóm ⇒ 24/3000); khi 8.D cho tham chiếu trong `Arg`/luồng có phạm vi PHẢI thêm tấn công riêng cho Mutex.
- **`gt-secrets`:** 24 đột biến sống sót (phần lớn biên số học tương đương; bộ quét kinh nghiệm), lần chạy cuối dừng ở 57/75; **`unsafe_check.rs`:** chưa có số liệu đột biến cuối sau khi bỏ mã thừa. `cli.rs` 7 sống sót (hiển thị). Chi tiết: KNOWN_LIMITATIONS "Đột biến M9".
- **Điểm 11:** E0601/E0307 qua driver thật — ĐÃ có test; giữ canary `selftest`.
- Giới hạn khác đã ghi: E1301 không có đầu vào sinh ra; `spawn` thông dịch tuần tự; Phase 1 bảo thủ về tham chiếu qua luồng (Q51); E1001: mọi `⚠` ở mã người dùng bị từ chối.

## 6. Câu hỏi đang chờ chủ dự án (xem DECISIONS)
- Chấp nhận lập luận đơn điệu cho 361/591/554 (ADR-027 §0).
- Q59–Q65 (ADR-029): đã duyệt có điều kiện ngày 2026-10-12; mọi quyết định cài đặt nội bộ còn lại do agent chọn (fail-closed) và ghi vào ADR.
- Q65 (quy trình `docs/FIELD_BUGS.md`) và việc dùng thử thật sau M10/M11: đang theo ADR-029 §8.
- Q64 paging file: **Giang tự chuyển sang D: và restart** (đã báo; không dừng chờ).

## 7. Môi trường (chi tiết: `docs/ENVIRONMENT.md`)
- **LLVM:** `D:\Tools\llvm-20.1.8` (bin\llvm-config.exe, bin\LLVM-C.dll, lib\*.lib, include\). Biến `GRIT_LLVM_DIR=D:\Tools\llvm-20.1.8`; `PATH` chứa `%GRIT_LLVM_DIR%\bin` lúc CHẠY gritc. Bản chính thức THIẾU `libxml2s.lib` và `LLVM-20.dll` nên `llvm-sys` mặc định gãy ⇒ dùng `no-llvm-linking` + chỉ thị liên kết riêng.
- **Quy tắc build (Giang):** KHÔNG chạy hai `cargo` cùng lúc; `CARGO_BUILD_JOBS=2`; test `--test-threads=2`; đột biến 1 worker, lô nhỏ, kết quả ghi nối `.jsonl` sau mỗi lô; `GT_FULL8=1` chỉ MỘT lần khi máy rảnh (~11 phút, đã chạy xanh).
- **TEMP/TMP ở D:** `D:\Grit\target\tmp` (ổ C: chỉ ~10 GB trống, chứa paging file). `t.sh` và `tools/gate.sh` đã đặt sẵn `TEMP/TMP/TMPDIR`, `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_BUILD_JOBS=2`, và thử lại khi `LNK1104/LNK1108/1455`.
- **Chạy test:** một crate/một tệp test: `./t.sh -p gt-oblig --test corpus` (thêm `-- tên_test` sau `--` để lọc); cả crate: `./t.sh -p gt-driver`. Oracle dài có biến: `GT_ORACLE_PROGRAMS=n` (borrowck; mặc định 20 000, ~2–6 phút), `GT_TERM_PROGRAMS=n` (dừng; mặc định 1500, ~2–3 phút), M7 oracle (gt-oblig/oracle) ~2 phút; `GT_FULL8=1` vét cạn 8 bit.
- **Cổng đầy đủ:** `bash tools/gate.sh /d/Grit > log 2>&1` (fmt, clippy `-D warnings`, audit, `cargo test --workspace`, check_expectations) — **~15–25 phút**, in `FMT_OK`, `CLIPPY_EXIT`, `AUDIT_EXIT`, `TEST_PASSED=n FAILED=m`; `cargo test --workspace` DỪNG ở crate lỗi đầu tiên nên số `TEST_PASSED` khi đỏ chưa đầy đủ.
- **Đột biến:** `tools/mut.py` (`list`/`run`/`report`); bản sao ở `D:\Grit_mut\w0` (chép crates, tests, lib, docs); `GRIT_MUT_TEST="cargo test -p … -- --test-threads=2"`, `GRIT_MUT_LINES=tệp.rs:dòng,…`, `--only-survivors tệp.jsonl`; một đột biến làm vòng lặp treo chiếm timeout 15 phút/đột biến.
- **Kiểm corpus:** `PYTHONIOENCODING=utf-8 python tools/check_expectations.py`.
- **Chèn lỗi kiểm cổng:** `GRITC_FAULT=panic:<pass>|skip:<pass>` (chỉ làm gritc từ chối thêm).
- **WinError 1455 = thiếu BỘ NHỚ ẢO (commit limit = RAM + paging file), KHÔNG phải thiếu ổ đĩa:** kiểm cài đặt paging file; giữ `CARGO_BUILD_JOBS` thấp khi liên kết LLVM. RAM vật lý 15,6 GB.
- **Công cụ:** `rustc/cargo 1.98.1`, MSVC (VS 2022), Python 3.12 (có `hashlib`), git-bash + PowerShell. Dấu `LNK1104` thoáng qua ⇒ thử lại.
