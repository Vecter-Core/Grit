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
Yêu cầu của Giang: kế hoạch hoàn thiện Grit; duyệt test và xoá có chọn lọc theo "E2E/snapshot trước, hồi quy khi có lỗi thật, fuzz để tìm sập"; nén tài liệu. Máy: Linux, `cargo 1.98.1`, `--no-default-features` (không có LLVM 20 ⇒ **test chạy nhị phân thật và cổng ĐẦY ĐỦ CHƯA chạy**).

**Số đo trước khi sửa (baseline không-LLVM, `--test-threads=2`; chưa tính `term_oracle` — chạy quá lâu lúc đó — và 7 bộ `rig`, cần LLVM; khử trùng theo danh tính binary):** 181 bộ, 863 test đạt, 0 lỗi, tổng ≈ 2.263 s.
Thời gian KHÔNG nằm ở số lượng test: `gt-fmt/comments.rs` 1.337 s (59%), oracle borrowck 295 s, oracle gt-oblig 285 s, `exhaustive` 60 s, `ghost_wide_oracle` 52 s ⇒ **5 bộ chiếm 89,7%**; 176 bộ còn lại mỗi bộ dưới 25 s.
`opt-level = 2` cho gt-fmt/lexer/parser/ast/span/diag (như gt-oblig; vẫn giữ kiểm tràn và debug-assertions): `comments.rs` 1.337 s ⇒ 680–695 s, 9/9 đạt, không đổi test nào. Giảm thêm cần LẤY MẪU vị trí chèn (đổi độ chặt của cổng: **chưa làm, chờ Giang**).
**Lần chạy cuối sau dọn (`cargo test --workspace --no-default-features`, 2026-11-21, cây mã tại `0588bba`; các commit sau chỉ đổi tài liệu):** 199 bộ có kết quả, 781 đạt, 37 lỗi, 3 ignore (= 821 test); tổng ≈ 2.054 s gồm `term_oracle` 477 s (baseline chưa tính). **Cả 37 lỗi nằm trong 7 bộ `rig`** (3+6+5+7+3+4+9) với thông báo nguyên văn "rig: bản rig này dựng không có backend llvm (--no-default-features)"; `crates/rig` không đổi so với `e8a8ae5`. Mọi bộ khác 0 lỗi. So cùng phạm vi (bỏ `term_oracle`, `rig`): ≈ 2.263 s ⇒ ≈ 1.577 s (−30%).
`tools/doc_test.py`: 25/220 khối `run` sai, vì bản `gritc` không-LLVM chỉ có thông dịch và thông dịch không có thân cho intrinsic I/O ("không có thân MIR cho $prelude::root_out"); các tài liệu đó không bị sửa. Cần máy có LLVM 20 để xác nhận lại.

**Cách quyết định xoá — đo độ phủ dòng RIÊNG của từng tệp test** (`-C instrument-coverage`, 173 binary, mỗi binary một ảnh; bỏ các bộ nặng nên ứng viên trông "riêng" hơn thực tế ⇒ nghiêng về GIỮ):
- Chín bộ `corpus.rs` theo crate (mỗi bộ chạy MỘT phần pipeline qua API riêng) ⇒ thay bằng `crates/gt-driver/tests/corpus_e2e.rs` (989 mẫu, 20 s, đòi ĐÚNG một mã lỗi). Xoá hẳn sáu bộ (types, mir, resolve, term m8, + phần trùng của oblig, parser, `gate.rs`); **GIỮ** `gt-borrowck/corpus.rs` (khẳng định `defensive_counters`) và `gt-dataflow/corpus.rs` (bộ kiểm bất biến "đã gán trước khi đọc", `analyses.rs:413–504`, không pass nào của `gritc` chạy): hai bộ này phủ riêng 40 dòng mà E2E không thấy.
- Test cấu trúc nội bộ: `gt-ast/ast.rs` (790 dòng phủ 40 dòng src), `gt-span/span.rs` (`Span::len/is_empty/to` không có nơi gọi trong mã sản phẩm), `gt-stack`, `file_id`, `lifetime_args`, `trivia`; sáu máy in gỡ lỗi `#[ignore]`.
- Sáu tệp hình-dạng-AST/tên-token (parser `expressions/items/statements/recovery_and_depth`, lexer `m1_first/token_names`) phủ riêng 154 dòng, hầu hết là NHÁNH THÔNG BÁO LỖI ⇒ chuyển thành 11 mẫu `tests/fail/E0101_*.gt` + snapshot + 2 mẫu pass + `tests/lexer/all_tokens` (91 token, kiểm âm tính) ⇒ đo lại: 0 dòng riêng ⇒ xoá.
- **Kết quả:** 931 ⇒ 821 test (−111, +1 E2E), 16 tệp test xoá, 8 cắt; mã test Rust ≈ 50,7k ⇒ 44,8k dòng (−12%); tệp theo dõi 2.248 ⇒ 1.881; `docs/` 280 ⇒ 178 tệp. Phần lớn khối lượng test còn lại là oracle/bộ sinh/test giết đột biến (hạ tầng bằng chứng, cố ý giữ), không phải test thừa.
- **Kiểm đột biến thật sau khi xoá `expressions.rs`:** 4 đột biến độ ưu tiên/kết hợp của parser (Mul cùng cấp Add; Sub kết hợp phải; And cùng cấp Or; Implies kết hợp trái) đều bị bắt — **chỉ bởi `gt-fmt/tests/parens_oracle.rs`** (`golden_and_parens` và E2E KHÔNG bắt). ⇒ `parens_oracle` là tuyến bảo vệ duy nhất cho độ ưu tiên: không được xoá.
- Kiểm cổng (nhẹ, ĐO 2026-11-21): `clippy --workspace --all-targets -D warnings` rc=0, `fmt --check` OK, `check_expectations` (550 fail/439 pass/13 warn/9 lexer) OK, `nul`, `platform_tests`, unittest công cụ đã sửa OK.

**Chưa làm / chờ quyết định:** (1) cổng đầy đủ trong clone sạch + test LLVM (cần máy có LLVM 20); (2) lấy mẫu `comments.rs`; (3) còn ứng viên white-box CHƯA có số đo đủ: `gt-dataflow/properties.rs` (0 dòng riêng nhưng chưa thử đột biến), `gt-resolve/names.rs` và `gt-types/{adt,functions,literals,traits_generics,...}.rs` (compile-fail viết trong Rust, nên chuyển dần sang mẫu `.gt`), `gt-diag/*`; (4) hai mẫu trong `KNOWN_EXTRA` (E0401_move_in_while, E1001_raw_ptr_return) nên viết lại tối thiểu; (5) luật "mỗi tệp xoá một dòng DELETIONS" làm `docs/DELETIONS.md` phình (205 ⇒ 602 dòng): đề xuất cho phép dòng theo thư mục (Giang quyết, vì đụng cổng).

### GÓI 12 phần 2 (2026-11-21, Claude Opus 5.5) — hoàn thiện Giai đoạn 1: đo trước, sửa sau
Lệnh Giang: kế hoạch tối ưu từng M, gộp chỗ thừa, viết chỗ thiếu của Giai đoạn 1 ("gt-borrowck thua Rust là sao"), đọc ngôn ngữ khác.
- **Đo**: 15 idiom Rust NLL ⇒ gt-borrowck chấp nhận 14 TRƯỚC khi sửa; lỗ duy nhất: mượn hai pha. Idiom M4/M7/M8 ⇒ ép mảng→lát thiếu; vòng
  `x / k`, Euclid ĐỎ; `==` trên tuple/`Option` thiếu. Mã lặp trong `crates/*/src`: 2,6% dòng (không thừa nhiều).
- **Đã làm** (mỗi việc: test ĐỎ trước, mẫu tấn công giữ ĐỎ, ADR): mượn hai pha (ADR-C14), ép unsize (ADR-C15), quan hệ chia/dư cho M7/M8
  (ADR-C16), gộp `empty_output` ở gt-types; kế hoạch từng M ở `docs/ROADMAP.md` mục 0b.
- **Bằng chứng**: oracle borrowck 6/6 (20 000 chương trình); term_oracle 1 500 chương trình, 0 âm tính giả (37 có chia được chấp nhận); oracle
  `zone::shift_upper` > 10 000 nghiệm; gt-oblig 21 bộ (golden không đổi). Đột biến: 29 (27 TỰ ĐỘNG `tools/mut.py`, 2 tay): lượt đầu 17 giết /
  9 sống / 1 không biên dịch ⇒ thêm 3 mẫu ⇒ còn 1 sống là đột biến TƯƠNG ĐƯƠNG (`zone.rs` `if self.empty` → false: khi rỗng mọi truy vấn bị cờ
  chặn trước). Một đột biến sống ở lượt đầu là UNSOUND thật (cho `%` khẳng định `a % b < a`) — chính bộ đột biến tìm ra lỗ hổng của bộ TEST.
- **Sự cố tự gây**: lô đột biến đầu bị lưới `track` dừng vì tôi sửa tệp trong lúc chạy (đúng thiết kế); chạy lại với cây sạch. Giai đoạn
  `failopen`: 3 mục mới (2 nhánh ICE `_ => return ice`, 1 `_ => None` bảo thủ) được PHÂN LOẠI trong sổ cái rồi mới ghi baseline.
- **Kiểm cuối**: 264 test của các crate đổi đạt; clippy -D warnings rc=0; fmt; check_expectations; nul; failopen 0 mới; platform_tests;
  mọi unittest công cụ của cổng đạt. **CHƯA**: cổng đầy đủ trong clone sạch, test nhị phân LLVM (máy không có LLVM 20).


### GÓI 12 phần 3 (2026-11-22, Claude Opus 5.5) — thẩm định đánh giá quản lý bộ nhớ của Gemini
Lệnh Giang: "đánh giá của Gemini có thật không? chỗ dùng được thì làm, chỗ sai thì đề xuất".
- **Đúng (đã là thiết kế Grit):** không GC (GRIT_PLAN); allocator tường minh kiểu Zig (D26, `Vec::new(&ar)`); Arena giải phóng theo lô và borrowck
  cấm dùng sau Arena; Perceus chỉ để tham khảo.
- **Sai/lệch:** `Cap<Mem>` không tồn tại (thật là `Cap<Alloc>` + hiệu ứng `mem`); `Drop` một mình không trả bộ nhớ nào trong Arena (cần
  `GlobalAlloc`); `std::HashMap` đã có `remove` tái dùng dung lượng; thời gian thực là Phase 2 (8.I); ví dụ web server chưa áp dụng được (Grit chưa có mạng).
- **Đã làm:** `std::Slab` (ADR-C17) + oracle `std_slab.rs` (40×120 thao tác, so mô hình Rust, `capacity == đỉnh sống`); 2 đột biến tay bị giết;
  `STD_REFERENCE.md` sinh lại; băm `std` trong 8 `rig.lock`.
- **Đề xuất (chưa làm):** `GlobalAlloc` + hạ `Drop` (ADR-C18) — lỗ hổng Phase 1 thật so với GRIT_PLAN §4.0(a).
- **CHƯA KIỂM (lúc đó):** exe LLVM, cổng đầy đủ trong clone sạch. *(Phần 4: exe LLVM của `Slab` ĐÃ KIỂM — `std_slab.rs` dựng 10 exe.)*

### GÓI 12 phần 4 (2026-11-22/23, Claude Opus 5.5) — ADR-C18 bước 1: `Drop` người dùng chạy thật
Lệnh Giang: "duyệt, làm đi" (duyệt ADR-C18).
- **Môi trường:** container đám mây CÀI ĐƯỢC LLVM 20 (`apt libllvm20` 20.1.2) bằng `bash tools/setup_cloud.sh --install-system`; `gritc doctor`
  XANH ⇒ từ phần này mọi khẳng định về exe LLVM là ĐÃ CHẠY, không còn "CHƯA KIỂM vì không có LLVM".
- **Đo trước:** `⊐ Drop for` ở mã người dùng ⇒ E0313; `user_drop.rs` ĐỎ 4/4. Sau khi bỏ lệnh cấm (trước khi sửa M8): đệ quy qua `drop` được
  CHẤP NHẬN (XANH) — chương trình không dừng lọt qua M8 ⇒ thêm cạnh gọi ngầm (mẫu `E0703_recursion_through_drop*`).
- **Đã làm:** luật M4 (E0313), `TypeDb::drop_glue`, elaboration drop (`gt-dataflow/src/elaborate.rs`, chạy giữa M5 và M6), M6 đọc thẻ nông,
  M8 cạnh tới `drop`, glue ở thông dịch và LLVM (ADR-C18 mục "BƯỚC 1"). Đặc tả: `docs/spec/semantics.md` §5a.
- **Bằng chứng:** `user_drop.rs` 6 test (5 viết tay + oracle 60 chương trình ngẫu nhiên), MỖI ca chạy thông dịch VÀ exe LLVM, so mô hình phạm vi
  độc lập; 15 mẫu corpus mới; snapshot E04xx chỉ THÊM 3 dòng; golden IR chỉ đổi băm attestation (MIR có trường mới). Đột biến TỰ ĐỘNG
  `elaborate.rs` 107: lượt đầu 49 giết/46 sống ⇒ sửa BỘ SINH 3 lần ⇒ cuối 70 giết / 25 sống tương đương (có lập luận) / 12 không biên dịch
  (`docs/MUT_DROP_ELAB_GOI12.md`). Phát hiện đáng ghi: điều kiện literal `⎇ true/false` bị bước hạ MIR cắt nhánh nên bộ sinh ban đầu gần như
  KHÔNG thử cờ drop — chỉ đột biến mới lộ ra.
- **Hồi quy bắt trước commit:** `deep_tuple_types` (tuple lồng 100 tầng toàn `u32`) bị E0318 giả do giới hạn độ sâu glue 64 ⇒ glue rỗng khi chương
  trình không có `drop` người dùng + giới hạn 1024 (chương trình cũ không đổi hành vi).
- **Chạy (có LLVM, theo lô ≤15 phút):** 55 + 47 + 22 + 2 + 43 + 9 bộ và rig — 0 lỗi sau khi sửa (4 lỗi giữa chừng: 3 golden IR = băm attestation,
  1 hồi quy độ sâu ở trên); clippy `-D warnings` có/không LLVM rc=0; fmt; check_expectations; fail-open 0 nghi vấn (32 mục mới đã phân loại).
- **Kiểm thêm phần 3:** `std::Slab` trên exe LLVM (`std_slab.rs` dựng 10 exe, khớp mô hình).
- **CHƯA:** cổng đầy đủ `tools/gate_clean.sh` trong clone sạch; Windows; ADR-C18 bước 2 (`GlobalAlloc`/`Owned<T>`, cần hợp đồng TCB) và bước 3.


### GÓI 12 phần 5 (2026-11-23, Claude Opus 5.5) — cổng clone sạch; ADR-C18 bước 2 `Owned<T>`
Lệnh Giang: "tiếp đi".
- **Cổng clone sạch trên `102d968` (`tools/gate_clean.sh`, 4 lô ≤ ~15 phút có điểm lưu):** gtrt, fmt, clippy, audit PASS; **test 955 đạt / 0 lỗi / 202
  bộ** (mọi crate, có LLVM, rig 48); expect, nul, doc, deletions PASS; **failopen ĐỎ** ⇒ `GATE_RESULT=RED`: 5 mục fail-open MỚI của phần 4 đã được
  phân loại trong `docs/fail_open_classification.tsv` nhưng `docs/fail_open_baseline.json` CHƯA được ghi lại (lỗi quy trình của tôi, không phải lỗi mã).
  Sửa ở `3b17f1f` (ghi baseline 9 mục, đều đã phân loại) và chạy lại: **cổng clone sạch trên `3b17f1f` XANH liền mạch** — test 957/0, 202 bộ
  (có LLVM 20.1.2 apt, Linux), gtrt/fmt/clippy/audit/expect/nul/doc/deletions/failopen/track đều đạt, `GATE_TREE=389b0f0a…`.
- **ADR-C18 bước 2 — `Owned<T>` (`9159d32`):** hợp đồng TCB viết TRƯỚC (`docs/TCB_INTRINSICS.md` B5). Prelude `#lang(owned)`, M4 (E0313 literal/Copy,
  E0301 đệ quy qua Owned, E0318 `Arena::alloc` của kiểu có Drop/Owned — lỗ ngữ nghĩa có từ bước 1: Arena âm thầm bỏ drop), glue `Free`, thông dịch,
  LLVM, runtime `gritrt_heap_alloc/free` (bảng RUNTIME IR Guard lớp E0401) có sổ khối sống ở dev.
- **Bằng chứng:** 8 ca × 2 backend (giá trị đúng + 0 khối sống); oracle ngẫu nhiên có biến `Owned<D>` × 2 backend, 0 khối sống; double-free chèn bị kiểm
  dev bắt (mã 70, kể cả `GRITRT_HEAP_CHECK=0`); đột biến tay bỏ `free` (LLVM, thông dịch) bị bắt; 11 mẫu corpus; golden IR chỉ đổi băm attestation;
  `unsafe` gt-rt 72 → 77 (đều có SAFETY); fail-open 16 mục mới phân loại.
- **Thêm sau cổng:** thông dịch ≡ exe dev ≡ release ≡ release-o2 cho chương trình Owned lồng (0 khối sống); đột biến tay hàm kiểm heap runtime 3/3 bị bắt.
- **CHƯA:** DUYỆT hợp đồng B5 (bằng chứng đủ, cổng xanh — tác giả không tự duyệt); đột biến tự động hàm runtime; `Owned: Send`; Windows.

## 8. GIAI ĐOẠN 2 (mở 2026-11-23 theo lệnh Giang — ADR-P2-00)
Thứ tự: A (bảo đảm) → B (độc lập backend) → C (dùng cho AI) → D (hiệu năng) sau cùng. Điều kiện vào chưa đủ (nhãn seed, §7) — Giang chủ động bỏ qua.

### P2-A1 (2026-11-23, Claude Opus 5.5) — xoá Secret có bảo đảm (ADR-P2-03)
- **Đo trước:** không có chỗ nào xoá Secret (FACTCHECK §3) VÀ — phát hiện mới — chương trình người dùng KHÔNG tạo được Secret nào (literal cấm, không hàm tạo).
- **Đã làm:** `Secret::new`; Secret "cần drop" với glue XOÁ; câu lệnh MIR `Wipe` sau mỗi move ra khỏi chỗ có thể chứa Secret (gán: ngay sau; đối số: trên cạnh
  trở về); xoá `_0` khi trả về; runtime `gritrt_secret_wipe` (ghi volatile + rào; IR Guard lớp D13; fail-closed khi null/độ dài sai — lỗ fail-open trong
  bản đầu của chính tôi, sổ fail-open bắt được); `Vec`/`Arena` chứa Secret ⇒ E0318. Đặc tả: `semantics.md` §7; TCB: B6 (CHỜ DUYỆT).
- **Bằng chứng:** `secret_wipe.rs` — oracle 24 chương trình × (thông dịch, exe dev, release, release-o2): số lần xoá = mô hình độc lập; 5 ca tính tay (2 ca
  phải sửa kỳ vọng sau khi ĐỌC MIR: biến tạm của gán đè cũng bị xoá — đúng đặc tả); đột biến tay 3/3 bị bắt; unit gt-rt xoá đúng `len` byte; 3 mẫu corpus.
  Chạy: 66 bộ/372 test của các crate bị đụng + 7 bộ driver/58 test đạt; clippy (có/không LLVM, runtime) sạch.
- **CHƯA:** luật Guard "mọi alloca Secret được xoá"; đột biến tự động; cổng liền mạch trên mã này; `declassify` cần `Cap<Secret>` (ADR-C13, vẫn hoãn).

### P2-A2/A3 bước 1 (2026-11-23, Claude Opus 5.5) — kernel chứng chỉ Farkas (ADR-P2-04)
- **Đã làm:** crate `gt-kernel` — phần TCB (`linear.rs`: dịch nghĩa vụ sang DNF ràng buộc tuyến tính, cái không hiểu ⇒ làm yếu; `check.rs`: kiểm chứng chỉ
  Farkas từng ca + luật `a ∧ ¬a`) tách khỏi phần tìm (`search.rs`, Fourier–Motzkin, NGOÀI TCB). `CertifiedProver{require_kernel}` bật ở M7 và M8:
  mọi "đã chứng minh" phải có chứng chỉ kernel chấp nhận.
- **Bằng chứng:** corpus 105/105 chứng minh zone có chứng chỉ (`kernel_coverage.rs`, `rejected == 0`); oracle vét cạn 3000 nghĩa vụ; chế độ chỉ-tin-kernel:
  63 bộ/240 test (corpus, gt-driver, rig) đạt. Đột biến tự động `docs/MUT_KERNEL_P2.md`: 76 → 72 giết, 4 sống có lập luận; lượt 1 lộ 3 đột biến
  KHÔNG SOUND sống (test độc hại cũ vi phạm nhiều luật cùng lúc) — đã thêm test cô lập từng luật. `mut.py` sửa giết nhóm tiến trình trên POSIX.
- **Phát hiện:** Farkas thêm 0 chứng minh mới trên corpus vì M7 chỉ xuất giả thiết khoảng + zone ⇒ bước 2: M7 giữ sự kiện tuyến tính tổng quát.
- **Phụ:** danh sách test nền Windows thiếu 10 test có sẵn (bước diff CI sẽ đỏ) — đã bổ sung, kiểm bằng đúng lệnh CI (`PLATFORM_DIFF_OK`).
- **CHƯA:** cổng liền mạch trên mã này; TCB của kernel CHỜ DUYỆT.

### P2-A2 bước 2 (2026-11-23, Claude Opus 5.5) — M7 giữ sự kiện tuyến tính (ADR-P2-05)
- **Đã làm:** `AState.facts` (điều kiện nhánh/assert/hợp đồng số học thuần; xoá cùng `defs`; giao ở điểm hợp/mở rộng) vào `hyps_for`; chứng minh hai bước
  (không sự kiện trước); `CertifiedProver` thử chứng chỉ trước khi tin "sai mọi nơi" (E0901 sai trên mã không tới được); FM bỏ hàng thừa.
- **Bằng chứng:** `linear_facts.rs` (ca tay + oracle 240 chương trình kiểm vét cạn `u8 × u8`): trước 15 `⊳` chứng minh tĩnh (7/72 ca từ đường đi) + 1
  E0901 sai; sau 82/82, 0 sai. Tắt xoá-khi-gán / tắt giao ⇒ ĐỎ. Corpus: chứng minh mới 0 (corpus gần như không có điều kiện tổng/hệ số), không hồi quy.
- **Hồi quy tự gây, bắt bởi `hashmap_collide`:** sự kiện thừa làm FM vượt giới hạn ⇒ E0702 sai trong `std`; sửa bằng hai bước + prune; mẫu hồi quy 265.
- **Quy trình:** commit 03acc63 lọt lỗi clippy (lệnh `clippy | head` nuốt mã thoát) — sửa ở commit kế; từ nay đọc `PIPESTATUS`.
- **Cổng:** clone sạch trên 52cf4f5 (chạy lại từ đầu sau khi sửa sổ fail-open) XANH liền mạch 4 lượt: 970 test đạt, 0 lỗi, 208 bộ, `GATE_TREE=10940c1f…`. Lần chạy trước trên 5d5069d ĐỎ đúng một bước `failopen` (4 chỗ mới chưa phân loại — tôi quên ghi sổ lần thứ hai).
- **CHƯA:** đột biến tự động phần M7 mới; đo thời gian M7 trước/sau; cổng trên Windows.

### P2-B0 + P2-C1 (2026-11-23, Claude Sonnet 5.5) — khảo sát LLVM + `rig test --auto`
- **B0:** bảng phụ thuộc LLVM + số đo thật ở `docs/PHASE2_FACTCHECK.md` mục 12 (dựng không-LLVM xanh 54 s; release 59 s, 6,8 MB; ranh giới L0 gọn trong `gt-llvm`/`gt-codegen-llvm`/`gt-guard`). Mục #2–7, #9 CHƯA ĐO.
- **C1 (ADR-P2-06, NGOÀI TCB):** `rig test --auto` — ứng viên đối số hằng biên, `gritc` làm bộ lọc (E0901 ⇒ ngoài miền), chạy thông dịch, báo abort là phản ví dụ. 6 test E2E (`crates/rig/tests/auto.rs`, đỏ trước), 3 đột biến tay bị bắt.
  Thử trên `lib/std`: `next_cap(usize::MAX[-1])` abort overflow (an toàn); "hết nhiên liệu" ban đầu bị báo như lỗi — sửa thành "chậm" vì mọi `⟳` có biến thể được chứng minh dừng.
- **Sự cố nhỏ:** lần thử trong `lib/std` để lại `lib/std/rig.lock` (tệp lạ làm vô hiệu cổng) — đã xoá đúng tệp đó ngay; từ nay chạy thử công cụ trên BẢN SAO.
- **Quyết định của Giang (2026-11-23):** KHÔNG gán nhãn seed (hay nhãn phiên bản khác) cho tới khi viết Grit hoàn chỉnh như viết Rust.
- **Cổng:** clone sạch trên d558cda XANH liền mạch 4 lượt: 976 test đạt, 0 lỗi, 209 bộ, `GATE_TREE=9d9b5813…`.
- **CHƯA:** đo thời gian `--auto` trên dự án lớn; hỗ trợ f32/f64/char/slice; cổng Windows.
