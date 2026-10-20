# PHÂN TÍCH DỰ ÁN GRIT — cái gì chạy được, cái gì lỗi, cái gì chưa có

Nguồn số liệu: `audit/RESULTS.md` (tự sinh từ `audit/run_all.sh`; mỗi giai đoạn có log và lệnh để chạy lại). **Không tệp nào của dự án bị sửa**: các thử nghiệm cần vá mã chạy trên **bản sao ngoài dự án** (`audit/patches/*.patch`).

Quy ước bằng chứng — mỗi kết luận gắn một nhãn:
**[CHẠY]** tôi đã chạy lệnh thật và thấy kết quả ở môi trường này · **[CHẠY·BẢN SAO]** chạy trên bản sao có vá (nói rõ vá gì) · **[ĐỌC MÃ]** suy ra từ đọc mã/tài liệu, chưa chạy được · **[DỰ ÁN TỰ KHAI]** tài liệu của dự án nói vậy, tôi chưa kiểm lại.

---

## 0. Tóm tắt

**Dự án là gì:** trình biên dịch `gritc` + trình quản lý gói `rig` cho ngôn ngữ Grit, viết bằng Rust: 22 crate, ~41 nghìn dòng mã sản phẩm + ~41 nghìn dòng test, 1 477 tệp theo dõi, 881 chương trình `.gt` trong corpus. Dự án tự báo "M0–M11 đã có mã", chưa làm M12 (thư viện lõi thật) và M13 (LSP), và tự liệt kê mình **chưa đủ điều kiện sang Phase 2** (READINESS: 3 điều kiện đạt, 4 có điều kiện, 5 chưa).

**Chạy được ở đây (Linux, LLVM hệ thống) và kết quả:**
- 17 crate "thuần" (lexer → … → secrets) dựng và test **nguyên trạng**: **716 test đạt, 0 lỗi** (134 bộ). `cargo audit`: 0 lỗ hổng trong 50 phụ thuộc. `cargo fmt`: sạch. Corpus 855 ca qua dòng lệnh `gritc`: **855/855 khớp** `expect`. Tất định (D9) đạt trên mẫu thử. [CHẠY / CHẠY·BẢN SAO]
- Backend LLVM + IR Guard + gt-rt + `rig` + 6 công cụ dogfood **không chạy được nguyên trạng** (chỉ Windows x86_64 MSVC + LLVM 20.1.8) nhưng chạy được trên **bản sao có port Linux (+62/−26 dòng, 5 tệp)**: **221 test native đạt, 5 lỗi (cả 5 là lỗi riêng của cổng Windows)**; IR sinh trên Linux **giống từng byte** golden Windows (trừ 2 dòng triple/datalayout). [CHẠY·BẢN SAO]

**Phát hiện quan trọng nhất (đều tái hiện được bằng lệnh):**

| # | Mức | Phát hiện |
|---|---|---|
| F-01 | **NGHIÊM TRỌNG — âm tính giả của kiểm tra an toàn** | Bộ kiểm sở hữu (M6) **không thấy nhiều toán hạng của CÙNG MỘT câu lệnh**: `(b, b)`, `two(b, b)`, `P { a: b, b: b }`, `[b, b]`, `g(b, b.v)` với `b` không-Copy đều được phán quyết **XANH**. Hệ quả chạy thật: nhân đôi `&!` (hai tham chiếu khả biến cùng ghi một chỗ), nhân đôi `Cap<…>`, nhân đôi tay cầm `Arena` ⇒ **dùng-sau-reset: chương trình XANH đọc nhầm giá trị (99 thay vì 1234) ở cả 3 hồ sơ dev/release/release-o2**. |
| F-02 | CAO | `gritc --no-default-features` (bản không LLVM, được tài liệu hoá là hỗ trợ) **không biên dịch**; toàn bộ dự án chỉ Windows+MSVC; 9 test không được cô lập theo feature. |
| F-03 | TRUNG BÌNH | `gritc` **tự sập** (SIGABRT/tràn ngăn xếp, mã 134, không phải ICE mã 2) với kiểu kích thước vô hạn `▢ L { next: Option<L> }`; M4 chấp nhận kiểu đó. |
| F-04 | TRUNG BÌNH | `[T]`/`str` theo giá trị chỉ bị chặn ở `let`; tham số/trường/trả về thì qua (trái spec §1). |
| F-05 | TRUNG BÌNH | Mô hình capability: gọi nguồn capability **qua giá trị hàm** (`mk: ƒ() → Cap<Out>`) bypass E0310; mục §7#12 tự chấm "đạt" nên cần xét lại. |
| F-06 | TRUNG BÌNH | Chương trình HỢP LỆ, dừng, check XANH nhưng native **bị HĐH giết** (mảng 20 MB trên stack; đệ quy 2 triệu tầng) và được báo là mã 2 (= "ICE"); D3 đòi abort có kiểm soát mã 70. |
| F-07 | THẤP–TB | ICE (E9901) khi tên biến/tham số đặt ở vị trí kiểu (`ƒ f(n: i32) → n`): 5 ICE/24 000 mẫu fuzz, một nguyên nhân (`collect.rs:660`). |
| F-08 | TRUNG BÌNH (quy trình) | 8/72 mã chẩn đoán thiếu tài liệu/mẫu theo GRIT_PLAN §0.5; `gritc explain E9901` lỗi; READINESS #4 ghi "đạt" là sai. Không có CI, không có `deny.toml`. |

Chi tiết, tái hiện và hướng sửa ở §2. Phần còn lại: §1 kết quả chạy, §3 cái chưa có so với kế hoạch, §4 rủi ro đọc mã, §5 việc chỉ làm được trên Windows, §6 thứ tự ưu tiên.

---

## 1. Kết quả chạy theo nhóm (chi tiết: RESULTS.md)

| Nhóm | Kết quả | Nhãn |
|---|---|---|
| A. Môi trường/kiểm kê | Linux x86_64, rustc 1.97 (dự án dùng 1.98.1), LLVM hệ thống 18.1 (+ `libllvm20` 20.1.2); kho git nông (50 commit) | CHẠY |
| B. Chất lượng tĩnh | `fmt` **đạt**; `cargo audit` **đạt** (0/50); `clippy -D warnings` **2 lỗi, cả hai trong tệp test** (xem F-09); không có CI, không có `deny.toml` (**FAIL**); 8 mã thiếu tài liệu/mẫu (**FAIL**) | CHẠY |
| C. Test 17 crate thuần | **716 đạt / 0 lỗi / 6 ignore** — gồm gt-fmt 915 s (chèn comment sau mọi token), gt-oblig 275 s (vét cạn, oracle), gt-term 359 s, gt-borrowck 204 s (oracle động 20 000 chương trình). Hai test "ĐỎ có chủ ý" ở commit `f7de5e1` đã **xanh** ở HEAD | CHẠY |
| D/E. Nguyên trạng gt-driver/rig/gt-llvm/gt-rt | **BLOCKED**: `gt-llvm/build.rs` đòi `LLVM-C.lib`; `native.rs` gọi `link.exe`; `gt-rt` có `compile_error!` cho mọi target ngoài Windows; `--no-default-features` không biên dịch | CHẠY |
| F. Công cụ Python | `check_expectations.py` **đạt** (881 `.gt`, 72 mã, 246 tham chiếu); 4 bộ unittest công cụ **đạt** (34 test) | CHẠY |
| G. `gritc` (thông dịch) trên bản sao vá 17 dòng | gate.rs 19/19 (cổng ĐỎ/ICE ⇒ không output, canary E0601/E0307, chèn lỗi), corpus CLI 855/855, `fmt` lũy đẳng trên mọi mẫu pass, tất định, `explain` (6 mã lỗi — xem F-08), 181 probe tự viết (xem §2), 3 kiểm metamorphic — `fmt` không đổi phán quyết (841 mẫu: 816 giữ nguyên, 25 bị fmt từ chối vì cú pháp hỏng cố ý, **0 đổi**), alias ASCII ≡ canonical (816 mẫu, **0 khác**), xáo thứ tự khai báo (429 mẫu × 2 hoán vị, **0 khác**) | CHẠY·BẢN SAO |
| G09. Vi sai M7 `--checks=all ≡ proven` | bộ sinh độc lập, 3 000 chương trình số học (1 228 bị từ chối), **10 632 lượt chạy** (4 017 abort có kiểm soát) ⇒ **0 khác biệt, 0 ICE**. Không tìm thấy kiểm tra nào bị `--checks=proven` bỏ sót ở lớp chương trình này | CHẠY·BẢN SAO |
| P01. Kiểm chứng **bản sửa F-01 đề xuất** (9 dòng, trên bản sao thứ 3 `grit_fix`, dự án không bị chạm) | probe `FALSE_NEGATIVE` **23 → 9** (14 probe F-01 `m6_same_stmt_*`/`m6_dup_*` đều bị chặn E0401, kể cả demo dùng-sau-reset; 9 còn lại là F-03/04/05/09 không liên quan); corpus qua CLI **855/855 vẫn khớp**; `gt-borrowck` **57 đạt / 0 lỗi** (gồm oracle động 20 000 chương trình, 206 s). Chưa chạy `gate.sh` đầy đủ và chưa chạy gt-oblig/gt-term trên bản sửa — việc của chủ dự án (§5) | CHẠY·BẢN SAO |
| G14. Vi sai M7 trên chỉ số mảng/lát (nơi M7 xoá kiểm tra biên) | 2 500 chương trình (1 495 bị từ chối), **6 030 lượt chạy** (2 481 abort) ⇒ **0 khác biệt, 0 ICE**; vùng này là chỗ 69 đột biến "xoá thêm check" của M7 còn mở (§3) mà tôi không tìm thấy khác biệt | CHẠY·BẢN SAO |
| G10. Fuzz đột biến token qua CLI | giai đoạn G10 (seed 11): 8 000 mẫu ⇒ 2 068 xanh / 5 931 đỏ / **1 ICE**; cộng các lượt chạy tay trước đó: 24 000 mẫu, 0 crash, 0 treo, **5 ICE** (một nguyên nhân, F-07). Mẫu ICE lưu ở `results/fuzz_artifacts/mutfuzz/ICE_0.gt` | CHẠY·BẢN SAO |
| L. Backend native trên bản port Linux | gt-driver **135 đạt / 5 lỗi** (cổng Windows), rig **47/0** (kể cả 6 công cụ dogfood so với `hashlib`/`serde_json`/mô hình Rust), gt-guard+codegen+llvm+gt-rt **39/0**; vi sai thông dịch ≡ exe native trên số học ngẫu nhiên (L05): 600 chương trình, 2 088 lượt chạy (767 abort), **0 khác biệt** | CHẠY·BẢN SAO |
| I. Fuzz thật (libFuzzer+ASan) | lexer 300 s = **2 667 202 lượt**, parser 300 s = **544 794 lượt**, **0 crash/timeout/oom** (nightly + cargo-fuzz). Rất ngắn so với 24 h dự án yêu cầu (tích luỹ của dự án chỉ 1,17 h lexer + 2,00 h parser) — chỉ là kiểm khói | CHẠY·BẢN SAO |

Số liệu so với tài liệu dự án: HANDOFF ghi "902 đạt / 0 lỗi / 174 bộ" (cổng 2026-10-17, Windows). Cộng phần chạy được ở đây: 716 + 135 + 47 + 39 = **937 đạt** (corpus đã lớn hơn: 881 `.gt` so với 866 trong HANDOFF) — **không có test nào đỏ vì lỗi logic**, chỉ 5 đỏ vì khác biệt nền tảng.

---

## 2. Lỗi/thiếu sót đã XÁC NHẬN (có tái hiện)

Cách tái hiện chung: `bash audit/run_all.sh --only G00_copy_patch,G08_probes` (dựng `gritc` trên bản sao vá rồi chạy `audit/probes/*.gt`); hoặc `gritc check <tệp>` trực tiếp. Mỗi probe có dòng `// probe:` (kỳ vọng) và `// why:`.

### F-01 — M6 không kiểm nhiều toán hạng trong cùng một câu lệnh · NGHIÊM TRỌNG · [CHẠY·BẢN SAO]
Theo định nghĩa của chính dự án (grit-contract §3.3, GRIT_PLAN §9): *"false negative của kiểm tra an toàn — mức cao nhất"*.

**Tái hiện tối thiểu** (mọi chương trình dưới đều được `gritc check` phán quyết **green**; so với `b1; b2` hai câu lệnh riêng thì bị E0401):
```
▢ B { v: i32 }
ƒ f(b: B) → (B, B) ! ∅ { (b, b) }            // tuple
ƒ two(a: B, b: B) ! ∅ { }
ƒ g(b: B) ! ∅ { two(b, b); }                   // hai đối số cùng một giá trị
ƒ h(b: B) → [B; 2] ! ∅ { [b, b] }              // mảng
ƒ k(b: B, n: i32) ! ∅ { }  ƒ m(b: B) ! ∅ { k(b, b.v); }   // đọc b.v sau khi b đã move
ƒ dup<T>(x: T) → (T, T) ! ∅ { (x, x) }         // nhân đôi MỌI kiểu affine
```
Probe: `m6_same_stmt_*` (10 tệp), `m6_dup_*` (4 tệp) — cộng 1 đối chứng dương `m6_control_read_then_move_ok`.

**Hệ quả chạy thật:**
1. `dup(&!x)` ⇒ hai `&!` cùng ghi một chỗ; thông dịch trả 20 (cả hai ghi vào `x`). Hiện IR **chưa** có `noalias` (đã kiểm: `grep noalias` rỗng, kết quả native vẫn đúng 2) nên chưa miscompile, nhưng 8.G định dùng `noalias` suy ra từ borrowck ⇒ sẽ biến lỗi này thành miscompile.
2. `dup(root_out())` ⇒ hai `Cap<Out>` từ một (phá tính affine của capability, D28/D29).
3. **Dùng-sau-reset thật** (`audit/probes/m6_dup_arena_reuse_demo.gt`, giai đoạn L06): nhân đôi tay cầm `Arena`, `alloc` qua bản này, `reset` qua bản kia, `alloc` lại, rồi đọc tham chiếu cũ — chương trình XANH, native trả **99 thay vì 1234** ở dev, release và release-o2. Viết trực tiếp (không `dup`) thì bị E0405 đúng. E0405 chỉ theo dõi theo *biến*, nên bị qua mặt khi có hai tay cầm.

**Nguyên nhân gốc [ĐỌC MÃ + xác nhận bằng chạy]:** `crates/gt-borrowck/src/check.rs`
- `run()` (dòng 656–697) tính trạng thái chưa-khởi-tạo `us` **một lần ở đầu mỗi câu lệnh/terminator** rồi gọi `check_uses(&mp, &us, &accs, …)` (dòng 675 và 695);
- `check_uses` (dòng 389–430) duyệt `accs` — mọi truy cập của câu lệnh, gồm nhiều `Operand::Move(p)` của `Rvalue::Aggregate` (dòng 149–153) và `args` của `Call` (dòng 203–205) — nhưng **mọi truy cập đều đối chiếu với cùng trạng thái đầu câu lệnh**; việc move của toán hạng trước chưa được áp trước khi kiểm toán hạng sau. Hiệu ứng move chỉ được áp sau cả câu lệnh (`un.statement_effect`, dòng 679).
- Bằng chứng khoanh vùng: `y ≔ x; z ≔ x;` (hai câu lệnh) bị E0401; `g(b.v, b)` (đọc rồi move) đúng là hợp lệ; `h(&b, b)` và `h(b, &b)` bị E0406/E0401 (vì có khoản vay); chỉ trường hợp "move/đọc chính chỗ đó hai lần trong một câu lệnh, không qua khoản vay" lọt.

**Vì sao lọt qua lưới của dự án:** bộ sinh `progen` của oracle M6 có 18 loại kịch bản (move, reborrow, partial_move, arena_reset, call_regionsig, …) nhưng **không có kịch bản "cùng một biến làm hai toán hạng của một câu lệnh"**; oracle động là thông dịch (sao chép giá trị) nên cũng không tự thấy. Đây đúng là điểm mù mà grit-contract §3.3 ("oracle chỉ phủ những cấu trúc nó sinh ra") đã cảnh báo.

**Hướng sửa (đề xuất, chưa áp dụng vào dự án; đã kiểm chứng trên bản sao ở giai đoạn P01 — `audit/patches/proposed_F01_check_uses.patch`, ~9 dòng):** trong `check_uses` giữ một bản sao cục bộ của `us`; sau khi kiểm mỗi truy cập `Kind::Move`, áp ngay hiệu ứng "đã move" của chỗ đó lên bản sao (cùng logic với `MaybeUninit::statement_effect`) trước khi kiểm truy cập kế tiếp; thêm 14 probe trên làm mẫu `tests/fail/E0401_*` (đã có trong `audit/probes/`), thêm kịch bản `dup_operand` vào `progen` rồi kiểm "tắt bản sửa" theo luật bộ sinh của dự án. Cập nhật READINESS §c #1 từ "đạt có điều kiện" sang "chưa" cho tới khi sửa.

### F-02 — Dự án chỉ dựng được trên Windows+MSVC+LLVM 20.1.8; bản "không LLVM" hỏng · CAO · [CHẠY + ĐỌC MÃ]
- `cargo build -p gt-driver --no-default-features` (cách `Cargo.toml` và usage tự mô tả là "gritc chỉ có backend interpreter, không cần LLVM") **lỗi biên dịch**: `cli.rs:25,60,71–73` dùng `gt_codegen_llvm::Opt` vô điều kiện trong khi crate đó là `optional` (E0433 ×5). Vá 17 dòng (`audit/patches/driver_nollvm.patch`, thêm kiểu `Opt` giả khi tắt feature) là đủ để dựng.
- Hệ quả: không có cách nào dựng `gritc` mà không cần LLVM; trên máy không có LLVM-C.lib thì `cargo test --workspace` đỏ ngay ở `gt-llvm/build.rs`.
- 9 test của gt-driver (`release_guard.rs` ×4, `release_traps.rs` ×4, `cli.rs::directory_input_and_version`) **không** có `#![cfg(feature = "llvm")]` như 18 tệp `llvm*.rs` khác ⇒ đỏ thay vì bị loại khi tắt feature (G01).
- Chỉ Windows x86_64: `gt-rt/src/lib.rs:10` `compile_error!`; `native.rs:107` `find_msvc_tools("x86_64-pc-windows-msvc","link.exe")`; `gt-llvm/build.rs` đòi `LLVM-C.lib`; `tools/gate.sh`, `t.sh` dùng `cygpath`/`D:\`.
- **Port Linux thử nghiệm thành công** (L01): vá 5 tệp, +62/−26 dòng (`audit/patches/linux_port.patch`), liên kết `libLLVM-20.so` (20.1.2), `cc -no-pie`, khoá `gritrt_mutex_*` bằng nguyên tử. Kết quả: §1 nhóm L. Khuyến nghị: nếu muốn CI đa nền tảng, các vá này nhỏ và đáng đưa vào dự án chính (sau khi chủ dự án duyệt — không thuộc phạm vi tôi được sửa).

### F-03 — `gritc` tự sập với kiểu kích thước vô hạn · TRUNG BÌNH · [CHẠY·BẢN SAO]
`▢ L { n: i32, next: Option<L> }` (hoặc `⊔ T { Leaf, Node(T, T) }`) được M4 chấp nhận **không có chẩn đoán** (không có mã lỗi nào cho "kiểu đệ quy không qua tham chiếu"; `semantics.md` không nói gì); thông dịch còn chạy được. Khi có dùng (`x ≔ L { n: 1, next: Option::None }`) và backend LLVM dựng kích thước: **tiến trình `gritc` tràn ngăn xếp, mã 134** (`thread 'main' has overflowed its stack`), không phải ICE mã 2 (D17) — `native.rs` chạy trên luồng chính, không bọc bởi `gt_stack::run_with_big_stack` như các pass kiểm tra. Probe `type_recursive_*`, stage L08. Sửa: kiểm vòng kích thước ở `collect.rs` (đồ thị trường theo giá trị) ⇒ E-code mới/E0301; bọc codegen vào `run_with_big_stack` để mọi panic/tràn thành ICE E9901.

### F-04 — Kiểu không biết kích thước (`[T]`, `str`) theo giá trị chỉ bị chặn ở `let` · TRUNG BÌNH · [CHẠY]
`semantics.md:11,13` nói `[T]`/`str` chỉ hợp lệ sau `&`/`&!`. Mã: `check/mod.rs:866 sized()` chỉ có **một** chỗ gọi (`mod.rs:797`, khởi tạo `let`). `ƒ f(x: [i32])`, `▢ S { x: [i32] }`, `ƒ f(x: str)` đều XANH; chạy native "thành công" (trả 0). Probe `type_slice_*`, `type_str_*`. Sửa: gọi `sized` cho tham số, kiểu trả về, trường struct/biến thể, phần tử tuple/mảng, đối số generic.

### F-05 — Capability: bypass E0310 qua giá trị hàm · TRUNG BÌNH · [CHẠY·BẢN SAO]
`ƒ helper(mk: ƒ() → Cap<Out> ! ∅) ! {io} { o ≔ mk(); … }` + `main` gọi `helper(root_out)` ⇒ XANH; `helper` đúc `Cap<Out>` tuỳ ý, lặp bao nhiêu lần cũng được. Nguyên nhân [ĐỌC MÃ]: `CapUse` chỉ được đẩy khi phân giải **đường dẫn tới hàm item** hoặc **phương thức** (`check/call.rs:102,231,281,493`); lời gọi qua biến kiểu `Ty::Fn` không đi qua đó. GRIT_PLAN §7#12 ("KHÔNG tuyên bố an toàn khi còn BẤT KỲ hàm không phải main gọi được hoặc nhắc tới nguồn capability") được dự án tự chấm "đạt" — cần xét lại. Sửa: áp luật `ret có Cap ∧ không tham số nào có Cap ⇒ E0310` cho cả `Callee::Value` có kiểu `Ty::Fn`, hoặc cấm đưa nguồn capability làm giá trị kể cả trong `main`.

### F-06 — Chương trình hợp lệ bị HĐH giết (tràn ngăn xếp native) · TRUNG BÌNH · [CHẠY·BẢN SAO + ĐỌC MÃ]
Check XANH, M8 chứng minh dừng, nhưng: mảng `[7u8; 20000000]` trên stack; `rec(2_000_000)` có `↓ n` ⇒ native: "chương trình bị hệ điều hành dừng" và `gritc run` trả **mã 2** (= ICE theo D17, vì `native.rs:327` ánh xạ tín hiệu thành `RunEnd::Internal`). `gritrt_start` (`gt-rt/src/lib.rs:64`) chỉ gọi `main()`, không có bảo vệ tràn ngăn xếp/trang bảo vệ. Trên Windows stack mặc định 1 MB nên ngưỡng còn thấp hơn. Thông dịch giới hạn đệ quy **200 khung** (`gt-mir/src/interp.rs:125`) ⇒ chương trình đệ quy sâu hơn 200 thoát mã 71. Stage L07. D3 ("lỗi không khôi phục = abort tức thì, mã 70") chưa đúng cho họ lỗi này. M8 chứng minh *dừng*, không chứng minh *cận ngăn xếp*; 8.I (cận tài nguyên) để Phase 2.

### F-07 — ICE khi tên giá trị đặt ở vị trí kiểu · THẤP–TRUNG BÌNH · [CHẠY·BẢN SAO]
`ƒ chk(b: i32) → b ! ∅ { true }` ⇒ `E9901` "MIR … không hợp lệ: kiểu chưa giải quyết (M4 phải chặn trước)" (mã thoát 2) thay vì E0301. Nguyên nhân: `collect.rs:660` `Res::Local(_) | Res::SelfValue => Ty::Err` **không phát chẩn đoán** (tên trỏ tới *hàm* thì báo "là fn, không phải kiểu" đúng — dòng 638–648). `Ty::Err` sau đó được các chỗ khác coi là hợp thức (`infer.rs:125`, `db.rs:191`, `check/mod.rs:908`); lưới an toàn duy nhất là validator MIR (ICE). Fuzz 24 000 mẫu: 5 ICE, **cùng một nguyên nhân**. Fail-closed nên không phải lỗ an toàn, nhưng là mẫu "Err không có chẩn đoán" mà `FAIL_OPEN_AUDIT` đã cảnh báo.

### F-08 — Quy trình/tài liệu · TRUNG BÌNH · [CHẠY]
- **8/72 mã chẩn đoán** chưa đủ "tài liệu + ≥3 mẫu fail + ≥3 mẫu pass" (GRIT_PLAN §0.5, §7#4): `E1201–E1204`, `E1301`, `E9901` không có `docs/errors/*.md`; `E1101` và `N0606` có tài liệu nhưng 0 mẫu (B05). `tools/check_expectations.py` chỉ kiểm danh sách `DOC_REQUIRED` do chính dự án chọn nên không thấy. Hệ quả người dùng: `gritc explain E9901` (mã quan trọng nhất: "báo lỗi gritc") **lỗi** (G07). READINESS §c #4 ghi "đạt" ⇒ nên sửa thành "đạt có điều kiện".
- **Không có CI** (`.github/workflows/` chỉ có `TASK.md` giữ chỗ) **và không có `deny.toml`** (chỉ `deny.toml.TASK.md`), dù GRIT_PLAN §0.7 yêu cầu fmt+clippy+test+deny+audit trong CI và `crates/TASK.md` nhờ "AI khác" làm (B04). Hiện các cổng chỉ chạy tay bằng `tools/gate.sh` (~40 phút trên máy tác giả).
- Spec lệch cài đặt: `semantics.md:69` "`declassify` đọc cần `Cap<Secret>`" nhưng `prelude.gt:66` `declassify(self) → T ! ∅` không đòi gì và không tồn tại `Cap<Secret>` (ADR-019 hoãn cho M12) ⇒ `declassify` gọi tự do chỉ bị cảnh báo vàng S201.

### F-09 — Các vụn vặt đã xác nhận · THẤP
- `clippy -D warnings` trên Linux/1.97 đỏ 2 chỗ **trong test**: `gt-lexer/tests/leak_check.rs:6` import `Command` thừa khi không phải Windows/macOS; `gt-types/tests/match_oracle.rs:5` `doc_lazy_continuation`.
- `D4` (cấm `==` trên số thực) có đường vòng hợp ngữ pháp: `⌥ a { 1.5 ↦ …, _ ↦ … }` (grammar `pat_lit` có `float_lit`; ADR-018 §230 ghi float cần `_`) — là so sánh bằng ngầm. Cần quyết định thiết kế: cấm hay ghi nhận (probe `d4_float_in_match_literal`).
- Alias vòng không dùng (`≡ A ≔ A;`) không bị báo (chỉ báo khi dùng): vô hại.
- Lỗi đầu vào của CLI hiển thị dạng Debug (`đầu vào không hợp lệ: BadModuleName("01_add.gt")`); tên tệp là tên module nên `gritc check tests/pass/01_add.gt` (tên bắt đầu bằng chữ số) bị từ chối — mọi mẫu trong corpus phải được chép sang tên khác mới chạy qua CLI.
- `golden_ll/*.ll` chứa `target triple`/`datalayout` của host ⇒ test golden bám nền tảng.
- 6 tệp `.exe` Windows (`examples/*/target/*.exe`, 150–190 KB) **đang được git theo dõi** (`.gitignore` chỉ chặn `/target` ở gốc); `scratch_edit2.py` (script vá tạm) và `t.sh` (Windows-only) nằm ở gốc repo; 7 tệp `TASK.md` giữ chỗ; `CLAUDE.md` gốc đã mất (bản sao nằm ở `.claude/skills/grit-contract.md`).
- `#[ignore]` trần (không lý do) ở 6 test dump/probe của gt-oblig/gt-term — vô hại (là hàm in), nhưng trái tinh thần "không `#[ignore]`" nếu không ghi lý do.
- Thông dịch không có thân MIR cho intrinsic của prelude: `gritc run` các công cụ dùng `Cap<…>` báo "lỗi nội bộ: không có thân MIR cho $prelude::root_env" (mã 2) thay vì "backend này không hỗ trợ".

### Những thứ tôi thử mà dự án CHỐNG ĐƯỢC (để biết vùng nào đáng tin)
181 probe tự viết, **155 đúng kỳ vọng** (23 lỗ hổng/thiếu sót ở trên, 3 giới hạn đã ghi): D1/D4/D5/D7, match vét cạn (bool, u8, i8, u128 literal cao, enum, tuple, char, Option lồng), E0601–E0604 (literal/tràn/chia 0/biên/ép kiểu/`MIN/-1`), E0307, E0401–E0407 (move, trả tham chiếu local, hai `&!`, move trong vòng lặp, move ra khỏi tham chiếu, Arena), khớp mẫu qua tham chiếu (ADR-040: ghi qua `&`, ghi đè khi đang mượn, move payload), reborrow hai lần, hai `&!` lên khoảng lát chồng nhau, hai phần tử cùng mảng, E0701–E0703 (vòng/đệ quy không biến thể, biến thể không giảm, `⊡` bỏ qua bước tiến, đệ quy tương hỗ), thứ tự khoá E0502 (đảo thứ tự, cùng cấp, tái nhập), E0501 (spawn với tham chiếu/Arena), E0503, E0310 (hàm thường, `main` trong module, phương thức tên `main`, initializer static/const), E1001, E1201–E1204, bộ quét secret (6/8 mẫu; 2 né quét đã được dự án ghi nhận).
Fuzz đột biến token 24 000 mẫu: **0 crash, 0 treo** (chỉ ICE F-07). Vi sai `--checks=all ≡ proven` (10 632 lượt) và thông dịch ≡ native (2 088 lượt): **0 khác biệt**. Ba kiểm metamorphic (fmt / alias / thứ tự khai báo): **0 khác biệt**.

---

## 3. Cái CHƯA CÓ so với GRIT_PLAN

| Hạng mục | Tình trạng | Ghi chú |
|---|---|---|
| CI (`ci.yml`), `deny.toml` | **chưa có** | F-08 |
| M12: `lib/core/*.gt` thật | **chưa**; chỉ có `lib/core/prelude.gt` 182 dòng (chữ ký + intrinsic tin cậy) | thiếu `Box`, `GlobalAlloc`, hàm `entry` nhận capability gốc; capability gốc vẫn do `root_*()` của prelude trả. 6 công cụ dogfood thay cho "3 chương trình thật" |
| M13: LSP | **chưa (theo lệnh)** | §7#8 chưa đạt |
| `gt-edge` | gộp vào `gt-oblig` | lệch plan §6.1, đã ghi ADR (Q49) |
| Fuzz ≥ 24 h mỗi cái | lexer **1,17 h** (4,9 %), parser **2,00 h** (8,3 %) theo `tools/fuzz_total.py`; kiểm kiểu thay bằng bộ sinh có cấu trúc (ADR-037) | `tools/fuzz_loop.sh` chưa tạo |
| Đọc mã đối kháng nhóm TCB cao (`TCB_CODE_AUDIT.md`) | **chưa** | điều kiện cổng của chính dự án |
| Flake `llvm_sort` | chưa điều tra xong | HANDOFF §4; không tái hiện ở bản port (L02 xanh) |
| 69 đột biến "xoá thêm check" của M7 | **còn mở** (§7#10) | `--checks=all` là mặc định để che; vi sai của tôi (G09) không tìm thấy khác biệt |
| Hiệu năng "±5% so với C/Rust" | **chưa** | BENCHMARK: dev ≈ Rust `-O0`, chậm 10–50× so Rust `-O`; `release-o2` nhanh hơn dev ×1,3–3,4 |
| Kiểm tra chéo nền tảng | **chưa**: Windows-only; không macOS/Linux | F-02 |
| `rig`: registry, chữ ký, kiểm capability gói | chưa (Phase 2) | chỉ phụ thuộc đường dẫn; `rig.lock` băm nội dung, so khớp toàn văn |
| Phase 2 (SMT, kernel, unsafe có bằng chứng, closure, dyn, self-host…) | chưa (theo thiết kế) | |
| Đột biến toàn bộ gt-types/gt-mir | không làm (quyết định 2026-10-06) | F-01 nằm đúng ở vùng này (gt-borrowck dùng MIR) |

---

## 4. Rủi ro tìm thấy khi đọc mã (CHƯA có tái hiện — cần người có máy Windows/xác nhận)

1. **Mẫu fail-open còn lại trong các crate mà `FAIL_OPEN_AUDIT.md` ghi "chưa rà"**:
   - `gt-types/src/db.rs:51–53` `imut`: ADT không có trong `adts` ⇒ `return false` ("không có mutability bên trong") — cùng kiểu S-A đã sửa ở `send.rs`; dự án đã ghi nó là "tương đương" trong đột biến (KNOWN_LIMITATIONS).
   - `gt-types/src/check/mod.rs:901` `place_mutable`: local không có trong bảng ⇒ coi là **ghi được**.
   - `gt-conc/src/lib.rs:162–164`: `sig_by_node` = `None` ⇒ `continue` (bỏ qua kiểm ordering) — S-E của HANDOFF mới đóng nửa (phần giá trị hàm đã đóng).
   - `Ty::Err` được coi là thoả mọi ràng buộc/unify (`infer.rs:125`, `db.rs:191`, `expr.rs:392,423`); an toàn chỉ khi mọi `Err` đều đã có chẩn đoán — F-07 chứng tỏ có đường tạo `Err` mà không chẩn đoán.
2. **TCB**: 42/42 dòng intrinsic được "ỦY QUYỀN KIẾN TRÚC SƯ DUYỆT HỢP ĐỒNG" nhưng nhóm CAO (Arena, Vec, Text, Atomic, Mutex, spawn, Cell) chưa có người thứ hai đọc **mã**; khoá `gritrt_mutex_*` là SRWLOCK (bản port của tôi dùng khoá quay — không thay được kiểm thử Windows). Không có ThreadSanitizer/loom (chấp nhận có ý thức, H7).
3. **IR Guard** nghĩa vụ theo *lớp* không theo *điểm* (KNOWN_LIMITATIONS): codegen thêm nhầm một thao tác nhạy cảm vẫn gắn đúng lớp thì lọt.
4. **M7 bộ giải**: 24 biến zone tối đa (`ZONE_MAX_VARS`), vòng thứ 13 của một hàm bị E0702 dù đúng (chặn nhầm, fail-closed).
5. **Quét secret** là heuristic: né bằng escape `\u{41}KIA…` hoặc chia khoá thành hai `□` (đã probe, xác nhận lọt; dự án đã ghi); định danh thuần Cyrillic trông như Latin lọt (đã ghi, KNOWN_LIMITATIONS lexer).
6. `known_pattern` của gt-secrets cấp phát `Vec<char>` ở mỗi vị trí (`starts_with_at`): quét tệp nguồn rất lớn sẽ chậm (dự án tự ghi "đột biến làm vòng quét treo").

---

## 5. Việc chỉ làm được trên máy Windows (tôi không chạy được — ghi để chủ dự án chạy)

- `bash tools/gate.sh D:/Grit` (cổng đầy đủ) trên mã hiện tại, **kèm 14 probe F-01 làm mẫu `tests/fail`** để xem cổng có đỏ đúng chỗ không.
- Đo lại hiệu năng (`tools/bench.py`, `bench_profile.py`); `GT_FULL8=1` (vét cạn 8 bit, ~11 phút); `probe_diff` bản đủ (~29 phút, bắt buộc trước khi đóng milestone theo điều kiện #9); đột biến (`tools/mut.py`) cho vùng F-01; fuzz tích luỹ tới 24 h bằng `tools/fuzz_session.sh`.
- Kiểm F-06 trên Windows: ngưỡng tràn ngăn xếp 1 MB và mã thoát thật (0xC00000FD); kiểm `dumpbin /dependents` (test `built_exe_does_not_depend_on_llvm_dll`) và 3 golden `.ll`.
- Thử cài/chạy LLVM 20.1.8 chính thức thay vì 20.1.2 (bản port dùng gói hệ thống).

---

## 6. Thứ tự ưu tiên đề xuất

1. **F-01 (sửa `check_uses` + 14 mẫu fail + kịch bản `progen`)** — là lỗ hổng an toàn thật, đã có exploit native; chặn mọi tuyên bố "sở hữu/mượn an toàn", "Cap affine", "Arena không dangling".
2. **F-03 + F-04 (tính hợp lệ của kiểu)** — một đợt sửa M4 nhỏ: kiểm kích thước và vòng kích thước; bọc codegen vào `run_with_big_stack`.
3. **F-05** — quyết định thiết kế cho capability qua giá trị hàm, rồi cập nhật §7#12.
4. **F-08** — viết 6 trang `docs/errors/`, thêm mẫu cho E1101/N0606, mở rộng `DOC_REQUIRED` ra toàn bộ `KNOWN`; **thêm CI + deny.toml** (việc "giao AI khác" đã treo từ M0).
5. **F-02** — gỡ phụ thuộc cứng `Opt` khỏi CLI, cô lập test theo feature; cân nhắc đưa bản port Linux vào để có CI đa nền tảng.
6. **F-06** — quyết định D3 cho tràn ngăn xếp (chạy `main` trên luồng có stack lớn + vệ binh, hoặc ghi giới hạn tường minh và ánh xạ tín hiệu thành mã riêng, không phải mã 2).
7. F-07, F-09, và các rủi ro §4 theo `FAIL_OPEN_AUDIT.md`.

---

## 7. Mức độ "đọc toàn bộ dự án" (nói thẳng)

Đã đọc kỹ: GRIT_PLAN, toàn bộ tài liệu trạng thái (HANDOFF, KNOWN_LIMITATIONS, READINESS, FIELD_BUGS, FAIL_OPEN_AUDIT, OPEN_QUESTIONS, đầu PROGRESS/TCB/ENVIRONMENT/BENCHMARK, danh sách 41 ADR), ngữ pháp và ngữ nghĩa, `gt-driver` (pipeline/verdict/runner/cli/native/render), `gt-guard`, `gt-conc`, `gt-effects`, `rig` (lock/project), phần quyết định của `gt-borrowck/check.rs`, `gt-types` (send/db/collect/check một phần), `gt-diag/code.rs`, `gt-stack`, mở đầu `gt-secrets`/`gt-oblig/driver.rs`/`gt-mir/lib.rs`. **Chưa đọc từng dòng** các phần lớn nhất (`gt-codegen-llvm/lower.rs` 3 471 dòng, `gt-oblig/analysis.rs` 2 292, `gt-mir/build/*` ~3 000, phần còn lại của `gt-types/check`, `gt-parser`, `gt-resolve`, `gt-term`, toàn bộ `tests/` và `docs/DECISIONS.md` 323 KB); với các phần này tôi dựa vào việc chạy test, corpus, probe và fuzz thay vì đọc. Vì vậy §4 là danh sách *gợi ý đã đọc*, không phải kiểm toán đầy đủ.
