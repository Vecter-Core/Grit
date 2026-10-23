# GHI CHÚ MERGE với bản Windows của Giang

Bối cảnh: nhánh `claude/gallant-darwin-m251cq` (origin) được làm từ container Linux; bản Windows của Giang có thể đã tiến xa hơn (báo cáo gần nhất của bản đó: cổng 943 test, FB-15, E0313, `fn_value_gen`, oracle match 13 kiểu) và CHƯA lên origin (2026-10-21). Quy ước để merge sau dễ:
- Commit NHỎ; không đổi tên tệp/mã lỗi/tài liệu đã có.
- FB mới dùng tiền tố **FB-C**n (đã dùng FB-C1…FB-C8); ADR mới dùng tiền tố **ADR-C**nn; mã lỗi mới chỉ khi bắt buộc và ghi dưới đây ("có thể trùng số").
- Số liệu test của nhánh này KHÔNG so được với 943 của bản Windows (khác nền tảng, khác tập tệp).

## Mã lỗi mới của nhánh này
(chưa có — các sửa F-01…F-08 dùng lại E0401/E0301/E0310; ghi thêm ở đây nếu S2 buộc phải thêm)

## Tệp bị sửa nhiều (nơi dễ xung đột)
- `crates/gt-borrowck/src/check.rs` (`check_uses`: bản sao trạng thái + áp move; FB-C1)
- `crates/gt-types/src/check/call.rs` (`call`: CapUse qua giá trị hàm FB-C3; S1 sẽ sửa effects/⊳ của đường dẫn kết hợp)
- `crates/gt-types/src/collect.rs` (`sized_ty`, `contains_itself_by_value`, Local ở vị trí kiểu: FB-C2/C4)
- `crates/gt-types/src/traits.rs` (`Pick.effects`), `crates/gt-oblig/src/driver.rs` (`escaped_sites`, E0901 cho `⊳` qua giá trị: FB-C5/FB-C6), `crates/gt-borrowck/tests/support/progen.rs` (+ `oracle.rs` phân loại): kịch bản `fn_value_ref`, `KINDS` 18→19
- `crates/gt-llvm/build.rs`, `crates/gt-driver/{build.rs,src/native.rs,src/cli.rs}`, `crates/rig/build.rs`, `runtime/gt-rt/src/lib.rs` (port Linux, ngăn xếp 256 MiB)
- `tools/gate.sh` (viết lại: đa nền, điểm lưu theo đơn vị, giai đoạn nul/doc), thêm `tools/preflight.sh`, `tools/nul_scan.py`
- `docs/HANDOFF.md`, `docs/FIELD_BUGS.md`, `docs/KNOWN_LIMITATIONS.md`, `docs/DELETIONS.md` (nối thêm cuối tệp)
- Test đổi vì kiểu đệ quy theo giá trị nay bị E0301: `gt-types/tests/spawn.rs`, `gt-oblig/tests/pure_preds_interior_mut.rs`
- `docs/errors/{E0301,E0310,E0401}.md` (nối danh sách mẫu); E1201–E1204, E1301, E9901 là tệp mới
- `tests/golden/borrowck/diagnostics.txt` (snapshot, thêm 14 dòng)

## Cập nhật Seed 1.0 S2–S8 (2026-10-23) — NÊN ĐỌC TRƯỚC KHI merge với bản Windows
**Không có mã lỗi mới** (E0901, E0503, E0301, E0313… chỉ dùng thêm; `E0313` ở nhánh này = "kiểu lang dùng sai" khi `⊐ Copy` cho Arena/Mutex/Atomic/Cell/Vec/Text/Secret/Cap — có thể KHÁC E0313 của bản Windows). FB mới: FB-C10…C27 (không có FB-C11); ADR mới: ADR-C01…C05.

**Tệp có nguy cơ xung đột cao (đã sửa nhiều):**
- `crates/gt-codegen-llvm/src/lower.rs` (Len/`==` `&str`/`recheck`-phía-codegen: không; nhưng: prescan `&!` ordering, thứ tự khởi tạo static theo bao đóng, `entry_alloca` cho `io.res/cas.res/rep.i`, chú thích Guard) — **nơi dễ nhất xung đột nếu bản Windows cũng sửa codegen**.
- `crates/gt-mir/src/build/expr.rs` (`index_place` đánh giá đầu mút trước `Len`; `recheck_place` mới; `assert_range`), `tests/golden/mir/aggregates.mir` (thứ tự `Len` đổi).
- `crates/gt-types/src/{db.rs,validate.rs,check/mod.rs,check/expr.rs,collect.rs}` (FB-C10/13/14/16/17/18/19/24 + `FnSigInfo.has_contracts`), `lib/core/prelude.gt` (`Cell::get` đòi `T: Copy`) ⇒ băm attestation đổi.
- `crates/gt-oblig/src/{analysis.rs,driver.rs}` (`unrep_atom_id`, `UNREP_MAX_BLOCKS`, E0901 cho `Callee::TraitMethod` có `⊳/⊲`, `escaped_sites` liệt kê tường minh), `crates/gt-conc/src/lib.rs` (`&!` ordering), `crates/gt-term/src/lib.rs` (`Rvalue::Repeat`), `crates/gt-guard/src/lib.rs` (luật địa chỉ cho mọi khai báo ngoài; bảng `RUNTIME` 27 hàm), `runtime/gt-rt/src/lib.rs` (`gritrt_bytes_eq`, `gritrt_stdin_read`, `gritrt_fs_write`, ngăn xếp 256 MiB).
- `crates/gt-driver/src/{cli.rs,lsp.rs,lib.rs,native.rs,render.rs}` (+ `Cargo.toml`: `serde_json`, version `1.0.0-rc2`), `crates/rig/src/cli.rs` (std tự thêm), `crates/gt-driver/fuzz/` (mới).
- `tests/golden_ll/{checked_add,slice_sum,static_atomic}.ll`: bless lại khi đổi phiên bản/prelude — GIỮ dòng `target datalayout/triple` kiểu **Windows** (`so sánh bỏ qua dòng target`); khi merge, chạy lại `GT_UPDATE_GOLDEN=1` rồi khôi phục dòng target.
- `lib/std/src/std.gt` (mở rộng lớn), `docs/STD_REFERENCE.md` (SINH bởi `tools/gen_docs.py` — KHÔNG sửa tay; sinh lại sau merge), `docs/AI_GUIDE.md`, `docs/{GRIT_BOOK,LANGUAGE_REFERENCE,QUICKSTART,QUICKSTART_CLOUD,SAFETY_CLAIMS}.md` (mới).
- Công cụ: `tools/gate.sh` (giai đoạn `failopen`, đa nền, điểm lưu), `tools/{preflight.sh,setup_cloud.sh,install.sh,install.ps1,doc_test.py,gen_docs.py,fail_open_scan.py,fail_open_sync.py,flake_loop.sh,fuzz_loop.sh,fuzz_check.sh}`, `tools/fuzz_session.sh` (thêm `check`, phát hiện bận trên Linux), `.github/workflows/ci.yml` (Windows chỉ viết, chưa chạy).
- Dữ liệu/báo cáo nối thêm: `docs/{FIELD_BUGS,KNOWN_LIMITATIONS,DELETIONS,RUN_REPORTS,FUZZ_LOG,STD2_TEST_RESULTS,CAPSTONE_LOG,FAIL_OPEN_AUDIT}.md|jsonl`; mới: `docs/{M7_DANGER_MUTANTS.md,MUTATION_M7_DANGER.jsonl,fail_open_classification.tsv,fail_open_baseline.json,adversarial/,SEED_LIMITATIONS.md,spec/FREEZE.md}`, `CHANGELOG.md`.
**Việc phải làm khi merge:** (1) `cargo test --workspace` hai bên, so FB-C và FB của bản Windows để không trùng số; (2) chạy lại `GT_UPDATE_GOLDEN=1` cho `golden_ll` rồi khôi phục dòng target; (3) `python3 tools/gen_docs.py`; (4) `bash tools/gate.sh` đầy đủ trên Windows (CHƯA từng chạy ở nhánh này) — đặc biệt `crates/gt-llvm/build.rs`, `native.rs` (đường dẫn runtime `libgt_rt.a` ↔ `gt_rt.lib`), `ci.yml`.
