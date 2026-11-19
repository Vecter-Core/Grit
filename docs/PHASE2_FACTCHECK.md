# PHASE2_FACTCHECK — đối chiếu đề xuất Giai đoạn 2 của Opus 5.5 với mã (GÓI 9, 2026-11-20)

**Phạm vi:** CHỈ ĐỌC mã và tài liệu (`git`, `grep`, đọc tệp); KHÔNG chạy cargo, KHÔNG đo, KHÔNG sửa đề xuất (`docs/PHASE2_PROPOSAL_OPUS.md`, thân giữ nguyên) và KHÔNG sửa mã. Cây mã: HEAD `cce0f13` (rc6 = `16c37f0`). Nhãn: **ĐÚNG** (khớp mã), **SAI** (mã nói khác), **ĐÚNG MỘT PHẦN** (ghi rõ phần sai), **KHÔNG KIỂM ĐƯỢC BẰNG ĐỌC MÃ** (cần đo hoặc là nguồn ngoài/đọc nguồn chưa mở; ghi ở Q1.h hoặc mục 9). Dòng tham chiếu là số dòng của HEAD nêu trên.

## 1. Bảng khẳng định (mã hoá: số = mục của đề xuất)

| # | Khẳng định của đề xuất | Nhãn | Bằng chứng (tệp:dòng) |
|---|---|---|---|
| 1 | §0.2 Mặc định là `dev` (LLVM -O0, mọi check giữ); O2 nằm sau IR Guard, ADR-036/Q82 | ĐÚNG | `crates/gt-driver/src/cli.rs:77` (`Opt::Dev`); `docs/DECISIONS.md:1110` (Q82: release chưa là mặc định); `cli.rs:30` (`--checks=all` mặc định) |
| 2 | §0.2 sortlines 33 MB: Grit dev 2,67 s; release-o2 1,14 s; Rust `-O` 0,78 s (theo min) | ĐÚNG MỘT PHẦN | `docs/BENCHMARK.md:55-58`: 2,6747 (run A), 1,1386 (run A), 0,7773 (run B). Ba số lấy từ HAI lượt đo khác nhau; cùng lượt B: dev min 3,3729, o2 min 1,2580; cùng lượt A: Rust `-O` min 0,8297. Tỷ lệ ~1,5× giữ đúng ở cả hai lượt; số tuyệt đối trộn lượt |
| 3 | §0.3 MIR phẳng theo chỉ số: `Body { locals: Vec<LocalDecl>, blocks: Vec<BlockData> }` | ĐÚNG | `crates/gt-mir/src/mir.rs:58,67-68` |
| 4 | §0.3 Arena `gt-rt` là bump theo chunk, cỡ không cố định | ĐÚNG | `runtime/gt-rt/src/lib.rs:494-495` (`chunks: Vec<(addr,cap,used)>`), `:602` (`CHUNK = 1<<20`), `:637` (chunk mới = `max(1 MiB, size+align)`: KHÔNG nhân đôi), `:609-653` |
| 5 | §0.3/§2.3 `GRITRT_ARENA_LIMIT` mặc định không giới hạn | ĐÚNG | `lib.rs:579-595` (`arena_limit()`, `None` khi không đặt), `:638-641` |
| 6 | §0.3 `Vec::push` cấp khối mới + `memcpy`, khối cũ giữ tới `reset` (TCB B3) | ĐÚNG | `crates/gt-codegen-llvm/src/lower.rs:3436-3438` (gấp đôi, tối thiểu 4, cấp khối MỚI trong Arena rồi `memcpy`); `lib.rs:688-704` (`reset` chỉ trả chunk thứ 2 trở đi); `docs/TCB_INTRINSICS.md:49` (B3). Hệ số tăng THỰC: ×2, sàn 4 phần tử (đề xuất chỉ nói "~2×") |
| 7 | §0.3 Enum hạ thành `{ i64 tag, [k x i64] payload }` | ĐÚNG | `lower.rs:4` (chú thích), `:697-713` (`k = ceil(max_size/8)`), tag ghi `:1515-1518`, đọc `:1650-1651` (`i64`). Biến thể cần căn lề > 8 ⇒ `Unsupported` (`:700-703`) |
| 8 | §0.3 Runtime không có cơ chế xoá bộ nhớ Secret | ĐÚNG | xem mục 3 (không có chỗ nào) |
| 9 | §0.3 `spawn`: hộp sở hữu, chưa `join`, chưa kênh | ĐÚNG | `docs/TCB_INTRINSICS.md:39` ("không có `join`; hộp không giải phóng"); `lib.rs:706-719` (`gritrt_alloc`) |
| 10 | §0.3 "Giai đoạn 2 đã có sẵn `gritc serve` … (8.E) và kênh có kiểu (8.D)" | ĐÚNG MỘT PHẦN | CHỈ có trong KẾ HOẠCH (`GRIT_PLAN.md:217` D18 "Phase 2 thêm … `serve`"; `:449` 8.E). Mã chưa có lệnh `serve` (`cli.rs:30` USAGE chỉ có `check build run explain fmt lsp doctor`). "Đã có sẵn" nên đọc là "đã có chỗ trong kế hoạch", không phải "đã có mã" |
| 11 | §2.1b `Drop` của người dùng chưa hạ | ĐÚNG | `docs/KNOWN_LIMITATIONS.md:105` (không elaboration drop), `:233` (Drop người dùng bị bỏ qua) |
| 12 | §2.1 thứ tự "chỗ trước, giá trị sau" (semantics §3a) | KHÔNG KIỂM ĐƯỢC BẰNG ĐỌC MÃ | chưa mở `docs/spec` mục đó; cần đọc spec và thử oracle (Q1.h #7) |
| 13 | §2.4 Arena đặt trước giữ được "kiểm `arena_overlap` của hồ sơ dev" | ĐÚNG (mô tả hiện trạng) | `lib.rs:500-570` (`arena_verify`, `arena_check_on`); tính khả thi của thiết kế mới là suy luận, CHƯA KIỂM |
| 14 | §2.5 gritc debug dựng 0,2–1,1 s; chưa đo gritc release | ĐÚNG | `docs/BENCHMARK.md:5,24` (cột "dựng (gritc debug)"); không có số gritc release trong kho (`grep` rỗng) ⇒ Q1.h #1 |
| 15 | §2.6/§6 ADR-013: AST dùng `Box` có lập luận | ĐÚNG (GÓI 10 S3.d.ii; trước: ĐÚNG MỘT PHẦN) | đã đọc TOÀN ADR-013 (`docs/DECISIONS.md:100-111`): phương án (A) cây `Box` chọn vì khớp mẫu/parser đệ quy tự nhiên và so sánh round-trip là `==`; (B) arena bị loại vì "lợi ích không đáng vì M5 hạ xuống HIR/MIR — đó mới là nơi cần cấu trúc phẳng". Khớp lời trích của đề xuất |
| 16 | §2.7 "Grit không có null nên tham chiếu luôn khác 0 (D3)" làm niche cho `Option<&T>` | ĐÚNG MỘT PHẦN — **SAI cho con trỏ béo** | `lib.rs:278-291` và `:823-826`: một chuỗi/lát RỖNG có thể qua ABI dạng `null + 0` ("A Grit empty string may cross the ABI as null + zero"). Vậy trường `ptr` của `&str`/`&[T]` có thể bằng 0 ⇒ KHÔNG dùng làm niche; chỉ `&T` mỏng mới là ứng viên. Chưa kiểm `D3` có nói "khác 0" hay không (D3 trong `GRIT_PLAN.md` nói về abort) |
| 17 | §2.7/7b nhóm đột biến NGUY HIỂM M7 được chấp nhận bằng lập luận (`M7_DANGER_MUTANTS.md`) | ĐÚNG (tệp có) | `docs/M7_DANGER_MUTANTS.md`, `docs/MUTATION_M7_DANGER.jsonl` tồn tại; nội dung từng đột biến CHƯA đọc lại |
| 18 | §2.8 nền `Send`-like (E0501), `Mutex` có cấp khoá (E0502) | ĐÚNG | `docs/TCB_INTRINSICS.md:33,39` |
| 19 | §3 benchmark "trước/sau trên 6 dogfood (`tools/bench_profile.py`)" | **SAI** | `tools/bench_profile.py` đo `fib` + **5** công cụ (wc, sha256, sortlines, json, stats): `docs/BENCHMARK.md:21`. `examples/` có 6 công cụ (json, sha256, sortlines, stats, wc, wordfreq); `wordfreq` không nằm trong `bench_profile.py` |
| 20 | §4 đột biến tự động `tools/mut.py`; `tools/mut_arena.py` có | ĐÚNG | `tools/mut.py`, `tools/mut_arena.py` tồn tại |
| 21 | §6 cú pháp và ngữ nghĩa lõi ĐÓNG BĂNG (`FREEZE.md`, ADR-C04); lexer/parser khoá tới đủ 24 giờ fuzz | ĐÚNG | `docs/DECISIONS.md:1305` (ADR-C04); `docs/spec/FREEZE.md:30-31` (lúc đóng băng: lexer 0,05 giờ, parser 0,02 giờ) |
| 22 | §6 N2: `⊲` nhắc tham số bị gán lại ⇒ E0902 (KNOWN_LIMITATIONS M7) | ĐÚNG | `docs/KNOWN_LIMITATIONS.md:141` điểm (2) |
| 23 | §6 N7: `SEED_LIMITATIONS` ghi "chưa có `?`" | ĐÚNG | `docs/SEED_LIMITATIONS.md:14` |
| 24 | §6 N8: "FREEZE cho phép THÊM khoá `edition` trong `rig.toml`" | SAI (GÓI 10 S3.d.iii; trước: KHÔNG KIỂM ĐƯỢC) | `grep -n edition docs/spec/FREEZE.md` rỗng: FREEZE.md KHÔNG có câu cho phép thêm khoá. Khoá `edition` cần ADR riêng và sửa FREEZE có ADR (ADR-P2-01); bộ đọc TOML của `rig` từ chối khoá lạ (fail-closed) nên `rig` phải nhận `edition` TRƯỚC |
| 25 | §6 N6: `Cap<T>` hiện là affine | ĐÚNG (GÓI 10 S3.d.i; trước: KHÔNG KIỂM ĐƯỢC) | ĐO bằng `gritc 1.0.0-rc6 check`: `ƒ use1(c: Cap<Fs>) ! ∅ { }` + `ƒ main(c: Cap<Fs>) ! ∅ { use1(c); use1(c); }` ⇒ `E0401` (dùng sau move; test `tests/fail/E0401_cap_used_twice.gt`); `ƒ main(c: Cap<Fs>) ! ∅ { }` (bỏ không dùng) ⇒ XANH. Tức là affine (≤ 1 lần), KHÔNG tuyến tính. Khớp ADR-020/ADR-022 mức kiểu: `Cap` không Copy (`DECISIONS.md:217,226`) |
| 26 | §6 N9: lỗi đảo đối số "rất hay gặp ở mã AI" | KHÔNG KIỂM ĐƯỢC BẰNG ĐỌC MÃ | `docs/USER_BUGS.md` (23 dòng) không có từ khoá "đảo/swap/thứ tự đối số" (`grep` rỗng); đề xuất tự ghi "đo trước". Q1.h #6 |
| 27 | §7.2.1 mặc định `dev` đã là -O0, mọi check giữ | ĐÚNG | như #1 |
| 28 | §7.2.2 `GRIT_PLAN` liệt kê LLVM trong TCB | ĐÚNG | `GRIT_PLAN.md:112` (danh sách TCB Phase 1, mục (3) LLVM). Đề xuất ghi "§3.3": số mục CHƯA đối chiếu |
| 29 | §7.2.3 có hạ tầng so codegen ≡ thông dịch (refpat, obgen, fn_value, std…) | ĐÚNG MỘT PHẦN | có `crates/gt-driver/tests/llvm_diff_refpat.rs`, `fn_value_gen.rs`, `std_lib.rs`, `std2_lib.rs`, `llvm_diff*.rs`; tên `obgen` KHÔNG thấy trong `gt-driver/tests` (có thể ở crate khác; chưa tìm) |
| 30 | §7.2.4/§8.1 "`lower.rs` ~3 650 dòng; Guard 517 dòng" | ĐÚNG | đếm: `lower.rs` 3 654; `gt-guard/src/lib.rs` 517 (mục 5) |
| 31 | §7.2.4 ước lượng x64 8–15 nghìn dòng | KHÔNG KIỂM ĐƯỢC BẰNG ĐỌC MÃ | là ước lượng (đề xuất tự ghi CHƯA KIỂM); không có nguyên mẫu |
| 32 | §7.3 lỗi backend ⇒ mã thoát 3 ("đã đóng băng là lỗi backend") | ĐÚNG MỘT PHẦN | `docs/spec/FREEZE.md:14` ("3 lỗi dùng/IO/backend") và `cli.rs:30` ("3 lỗi dùng/IO"); thực tế `native.rs:48-55`: `Env`→3, `Codegen(Unsupported)`→3, **Guard→1 (ĐỎ E1101)**, `Codegen(Internal)`/`Link`→2 (ICE). Với x64 "phần chưa hỗ trợ ⇒ 3" khớp `Unsupported`; một lỗi sinh mã sai do Guard-post bắt ⇒ mã 1, không phải 3 |
| 33 | §8.1 `gritc` dựng được không cần LLVM: feature `llvm` tuỳ chọn, `--no-default-features` | ĐÚNG | `crates/gt-driver/Cargo.toml` `[features] default=["llvm"]`; `cli.rs:15-27,262-264,288-296,578-583` |
| 34 | §8.1 LLVM nạp động (LLVM-C 20.1.8); FFI thô trong `gt-llvm` (66 dòng) | ĐÚNG | `crates/gt-llvm/Cargo.toml` (mô tả "nạp trễ", feature `llvm20-1-no-llvm-linking`); `lib.rs` 5 + `shim.rs` 61 = 66 dòng |
| 35 | §8.1 JSON có trường `backend`; mã thoát 3 dành cho lỗi backend | ĐÚNG / như #32 | `render.rs:183-184`, `pipeline.rs:123-124` |
| 36 | §8.1 mọi pass M1–M9 không biết LLVM | ĐÚNG | `grep -il "llvm\|inkwell"` trên `crates/*/src` và `Cargo.toml`: chỉ `gt-llvm`, `gt-guard`, `gt-codegen-llvm`, `gt-driver`, và `rig` (chuyển tiếp feature `llvm = ["gt-driver/llvm"]`) |
| 37 | §8.1 IR Guard kiểm trên LLVM IR (`gt-guard` phụ thuộc `gt-llvm`) | ĐÚNG | `crates/gt-guard/Cargo.toml`; xem bảng mục 4 |
| 38 | §8.1 attestation và lớp nghĩa vụ gắn bằng metadata LLVM (`!gt.attest`, `!gt.obl`) | ĐÚNG | tạo: `lower.rs:955-998`; đọc: `gt-guard/src/lib.rs:30-34,62-70,392-411` (mục 4) |
| 39 | §8.1 hồ sơ release gắn với danh sách pass LLVM (ADR-036) | ĐÚNG | `docs/DECISIONS.md:1062,1108-1110` |
| 40 | §8.2 "`gt-rt` giữ C ABI làm giao diện ổn định" | ĐÚNG | `extern "C"` ở mọi hàm `gritrt_*` (`lib.rs:604,…`); bảng `RUNTIME` 27 hàm `gt-guard/src/lib.rs:84-112` |
| 41 | §8.3 L0 "đo gritc bản release" | ĐÚNG (chưa làm) | không có số trong kho ⇒ Q1.h |
| 42 | §1 "7a: tag `u8`/`u16` theo số biến thể" khả thi theo hạ hiện tại | ĐÚNG MỘT PHẦN | tag hiện đọc/ghi `i64` ở nhiều nơi: `lower.rs:1515-1518,1650-1651,2502-2518,2573-2595,2881-2899,3528-3544` (≥ 6 cụm) ⇒ đổi là đổi cả `Option`/`Result`/I/O intrinsic; không chỉ "bố cục" |
| 43 | §7.1, §7.5, nguồn ngoài (Zig, QBE, Cranelift, Goose) | KHÔNG KIỂM ĐƯỢC BẰNG ĐỌC MÃ | nguồn ngoài; đề xuất tự ghi một phần chưa tra gốc |
| 44 | §6.1 Eiffel/SPARK/Dafny/Austral/Unison… | KHÔNG KIỂM ĐƯỢC BẰNG ĐỌC MÃ | đề xuất tự ghi "chưa tra lại tài liệu gốc" |

**Tổng (44 dòng, đếm lại sau GÓI 10 S3.d; trước đó 29/7/1/7):** ĐÚNG **31** · ĐÚNG MỘT PHẦN **6** (#2, 10, 16, 29, 32, 42; #16 có phần SAI) · SAI **2** (#19, #24) · KHÔNG KIỂM ĐƯỢC BẰNG ĐỌC MÃ **5** (#12, 26, 31, 43, 44). Thêm 7 dòng đối chiếu ADR ở mục 7 (6 ĐÚNG, 1 ĐÚNG MỘT PHẦN: ADR-038; ADR-013 nay ĐÚNG).

## 2. Mục bắt buộc (a) Bố cục enum — `crates/gt-codegen-llvm/src/lower.rs`
- **Tag:** `i64` (8 byte), trường 0 của `{ i64, [k x i64] }` (`:697-713`). Ghi tag bằng `build_struct_gep(et, p, 0)` rồi `store i64` (`:1515-1518`); đọc `load i64` (`:1650-1651`). `Result<T,T>` do hạ (`:2881-2899`) và `Result<X, IoError>` do I/O (`:3528-3557`) cũng dùng tag `i64`.
- **Payload:** mỗi biến thể là một struct (`variant_struct` `:649`); payload của enum là mảng `[k x i64]` với `k = ceil(max_size_biến_thể / 8)`, `max_size` lấy từ `TargetData::get_abi_size` (`:697-711`). Biến thể có căn lề ABI > 8 ⇒ `Unsupported("biến thể enum cần căn lề > 8")`. Trường biến thể được truy cập qua `variant_struct` trên con trỏ payload (`:1241-1244`).
- **Cỡ:** `8 + 8*k` byte, cỡ MỌI biến thể bằng nhau (không niche, không tag nhỏ). `Mutex<T,L>` = `{ T, i64 }` (`:677-684`).
- **Hàm liên quan:** `lower_ty` (`:659`), `variant_fields` (`:632`), `variant_struct` (`:649`), `adt_subst` (`:622`), `has_interior_mut` (`:1445-1465`, đếm `AdtKind::Enum`), `adt_of_intr` (`:3096`, nhận diện `Result`), các hàm dựng `Option`/`Result` (`:2502-2599`), `io_result` (`:3502-3557`).

## 3. Mục bắt buộc (b) `gt-rt`: Arena, `Vec::push`, ghi đè bộ nhớ
- **Arena (`runtime/gt-rt/src/lib.rs`):** `CHUNK = 1 MiB` (`:602`). Cấp phát: thử chunk cuối (`:629-635`), hết chỗ thì cấp chunk MỚI cỡ `max(CHUNK, size+align)` bằng `std::alloc::alloc` căn 4096 (`:637-650`); **không nhân đôi, không đặt trước địa chỉ ảo**; giới hạn tổng nếu `GRITRT_ARENA_LIMIT` đặt (`:579-595`, `:638-641`). `reset` (`:688-704`): `dealloc` các chunk từ thứ hai, đặt `used = 0` ở chunk đầu, xoá bảng `live` (kiểm động); **không ghi đè nội dung**.
- **`Vec::push`:** do codegen phát (`lower.rs:3436-3438`), KHÔNG phải hàm `gt-rt`: gấp đôi dung lượng (sàn 4) bằng cấp khối MỚI từ Arena (`gritrt_arena_alloc`) rồi `memcpy` phần đã dùng (`:3150-3167` vùng gọi cấp; `memcpy` bằng `build_memcpy`); khối cũ giữ tới `reset`. Tràn `cap*2`/`cap*size` ⇒ abort `overflow`.
- **MỌI chỗ ghi đè/xoá bộ nhớ (grep `volatile|write_bytes|memset|zero|fill(`):**
  - `ptr::write_bytes` ở `lib.rs:940` và `read_volatile`/`write_volatile` ở `lib.rs:995-996` đều NẰM TRONG `#[cfg(test)]` (module bắt đầu `lib.rs:756`): là test kiểm Arena/chia sẻ, không phải mã chạy của chương trình người dùng.
  - Không có `memset`/`zeroize`/`volatile` nào trong phần sản phẩm của `gt-rt`. `grep "Secret"` trong `lib.rs` và `lower.rs` rỗng (codegen và runtime không biết tới `Secret`).
  - `std::alloc::alloc` KHÔNG khởi tạo (`gritrt_alloc` `:706-719`, `gritrt_arena_alloc` `:609`): bộ nhớ chưa xoá.
  - `lib/core/prelude.gt:68-69`: `Secret<T> { v: T }` là struct thường; `declassify` là hàm không thân (intrinsic) trả `v`.
- **KẾT LUẬN: hiện nay KHÔNG có chỗ nào xoá nội dung `Secret`** — không khi hết phạm vi, không khi `Arena::reset`, không khi thoát tiến trình. Lập luận "Drop chưa hạ" (`KNOWN_LIMITATIONS.md:105,233`) cũng chặn đường xoá qua `Drop`. (Xác nhận bằng đọc; CHƯA xác nhận bằng đo trên exe.)

## 4. Mục bắt buộc (c) `gt-guard`: mọi chỗ dùng LLVM và phân loại phụ thuộc
**Mọi chỗ dùng kiểu/API LLVM (qua `gt_llvm::inkwell`):** `Module` (`lib.rs:16`), `AtomicOrdering` (`:14`), `Attribute`/`AttributeLoc` (`:15`), `AnyValue`/`AnyValueEnum`/`BasicValue`/`CallSiteValue`/`InstructionOpcode`/`InstructionValue` (`:17-19`); hàm: `callee_name` (`:53`), `obl_class` (`:62`, `get_kind_id`, `get_metadata`, `get_node_values`), `is_atomic` (`:72`), `inst_of` (`:154`), `runtime_address_violations` (`:171`, duyệt `get_first_use`), `instance_violations` (`:217`, `get_global_metadata`), `abort_kind_of` (`:297`), `abort_profile` (`:312`), `check_post` (`:342`), `check_module` (`:383`, `m.verify()`, `print_to_string`). **Toàn bộ crate (517 dòng) làm việc trên `Module` LLVM**; không có lớp trừu tượng.

| Luật | Phụ thuộc | Lý do | Dòng |
|---|---|---|---|
| 1. LLVM verifier sạch | LLVM | gọi `Module::verify` | 387-389 |
| 2. `!gt.attest` = attestation của `check` | KHÁI NIỆM độc lập; CƠ CHẾ LLVM | cần chứng minh IR sinh từ MIR đã duyệt (mọi backend); nhưng đọc bằng `get_global_metadata` | 31, 391-411 |
| 3. Thao tác nhạy cảm mang `!gt.obl` đúng lớp | KHÁI NIỆM độc lập; CƠ CHẾ LLVM | bảng `RUNTIME` và lớp (E0310/E0405/E0501/E0502/E0503) là hợp đồng của Grit; nhận diện atomic bằng opcode LLVM và lớp bằng metadata LLVM | 62-70, 72-80, 84-112, 257-266, 424-435 |
| 3b. Đích mọi lời gọi: có thân / intrinsic cho phép / bảng `RUNTIME` | KHÁI NIỆM độc lập (không FFI ngoài runtime); danh sách intrinsic LLVM | `allowed_llvm_intrinsic` là tên LLVM; khái niệm "không gọi ra ngoài" độc lập | 122-151, 436-457 |
| 3c. Không `invoke`/`callbr` | LLVM | opcode LLVM; backend khác không có khái niệm này (tương đương: "không unwinding") | 458-466 |
| (FB-C27) Địa chỉ hàm khai báo chỉ làm đích gọi trực tiếp | KHÁI NIỆM độc lập; CƠ CHẾ LLVM | tránh lách luật qua con trỏ hàm; duyệt use-list LLVM | 171-213 |
| 4. `unreachable` chỉ đúng mẫu abort (`trap.*` + `gritrt_abort` noreturn) | LLVM | `unreachable` là opcode LLVM; khái niệm "sau abort không có đường thoát" độc lập | 467-482 |
| 5. Không cờ UB (`nsw/nuw/exact/fast-math/…`) và không `llvm.assume` | LLVM | cờ poison là riêng LLVM; backend không có bộ tối ưu không có chuyện này | 268-271, 483-510 |
| Instance: tên ASCII; hàm định nghĩa ⊆ bảng instance M4; `!gt.instances` = tập định nghĩa | KHÁI NIỆM độc lập; CƠ CHẾ LLVM | tập hàm = tập instance là bất biến của Grit; đọc bằng `m.get_functions()` + metadata | 217-254 |
| R1 (post) số lời gọi `gritrt_abort` theo (hàm, loại) không giảm | KHÁI NIỆM độc lập; CƠ CHẾ LLVM | chống bộ tối ưu xoá check; backend có tối ưu nào cũng cần | 288-327, 342-352 |
| R2/R3 (post) không FMA, không thuộc tính `unsafe-fp-math…` | LLVM | tên intrinsic/attribute LLVM; ý "không đổi kết quả bit IEEE" độc lập | 274-286, 353-377 |

**Metadata attestation / nghĩa vụ:** tạo ở `crates/gt-codegen-llvm/src/lower.rs:955-998` (`!gt.attest` module `:965`; `!gt.instances` `:966-977`; hằng chuỗi `gt.attestation` `:981`; `!gt.obl` từng lệnh `:987-998`); **đọc ở** `gt-guard/src/lib.rs` (`ATTEST_KEY` `:31`, `OBL_KIND` `:32`, `INSTANCES_KEY` `:34`; đọc `:62-70, 217-254, 392-411`). Hợp đồng giữa hai crate chỉ là chuỗi tên hằng (hai nơi khai báo độc lập). Điểm gọi: `crates/gt-driver/src/native.rs:219-247` (`pre_facts`, `check_module`, `check_post`).
**Kết luận bảng:** ~6/11 dòng là "khái niệm độc lập, cơ chế LLVM"; 4/11 dòng thuần LLVM (verifier, `invoke`, `unreachable`, cờ UB/`assume`, FMA/attrs gộp). Đây là phân tích BẰNG ĐỌC; việc tách (L2) chưa thử nên chưa biết còn luật nào phụ thuộc ngầm.

## 5. Mục bắt buộc (d) `gt-driver`: chọn backend, JSON, mã thoát 3, dựng không LLVM
- **Chọn backend:** feature `llvm` (mặc định bật; `Cargo.toml`), cờ `--backend=llvm|interpreter` (`cli.rs:86-87,98-100`), mặc định `default_backend()` = `llvm` nếu `cfg!(feature="llvm")` (`:51-56`).
- **Trường `backend`:** `CheckResult.backend: &'static str` (`pipeline.rs:123-124`), in ở JSON `render.rs:183-184` và dòng văn bản `render.rs:284-289`; gán `"llvm"` ở `cli.rs:217,254,352,371`.
- **Mã thoát 3:** `NativeError::exit_code` (`native.rs:48-55`): `Env`→3, `Codegen(Unsupported)`→3, `Guard`/`Codegen(Guard)`→1, `Codegen(Internal)`/`Link`→2. Quy ước chung (`FREEZE.md:14`; `cli.rs:30`): 0 qua · 1 đỏ · 2 ICE · 3 lỗi dùng/IO/backend.
- **Dựng không LLVM (`--no-default-features`):** `check`, `fmt`, `explain`, `lsp`, thông dịch (backend `interpreter`); `build_llvm`/`run_llvm` trả lỗi dùng mã 3 (`cli.rs:262-264,288-296`); `gritc doctor` báo ĐỎ "không có backend llvm" (`:578-583`). Mô-đun giả `gt_codegen_llvm` ở `cli.rs:15-27` cho phép phân tích `--profile=`. **CHƯA KIỂM bằng dựng** (không chạy cargo): `cargo build --no-default-features` hiện có xanh hay không (có test `tests/` riêng? chưa tìm) ⇒ Q1.h.

## 6. Mục bắt buộc (e) `spawn`
- **Prelude** (`lib/core/prelude.gt:15`): `ƒ spawn<A: Send>(f: ƒ(A) ! {detach}, a: A) ! {detach} { }` — tức hiệu ứng `{detach}` ở CHỮ KÝ; `A: Send` (E0501).
- **Join:** KHÔNG có (`TCB_INTRINSICS.md:39`; `lib.rs:706-719` hộp không giải phóng).
- **`detach` được kiểm ở `crates/gt-effects/src/lib.rs:272,294-298`:** marker `detach` như `poly` (lấy hiệu ứng của đối số hàm) nhưng BỎ `lock(*)` (worker không chạy lồng trong khoá); `crates/gt-types/src/check/call.rs:27-32` (tham số hàm có marker `poly`/`detach` lấy hiệu ứng của đối số thay marker); `crates/gt-types/src/collect.rs:383-393` (E1002: người dùng KHÔNG được dùng `poly`/`detach`).
- **`TCB_INTRINSICS.md:39`:** hiệu ứng ghi là "`detach` (chỉ prelude)".
- **MÂU THUẪN (còn nguyên, đã đo ở GÓI 8, ADR-C12 `docs/DECISIONS.md:1340-1345`):** quyết định kiến trúc sư (GÓI 8) là hàm gọi `spawn` PHẢI khai `detach`; mã hiện tại: hàm `! ∅` gọi `spawn` vẫn XANH (hiệu ứng của lời gọi = hiệu ứng worker trừ `lock`); người dùng không thể khai `{detach}` (E1002). Trạng thái: **CHƯA ÁP DỤNG** — đã đọc lại mã HEAD `cce0f13`, không có thay đổi nào so với ADR-C12 (`git log` không có commit nào chạm `gt-effects` sau GÓI 8). Đề xuất của Opus (§2.8) không nêu mâu thuẫn này; kênh có kiểu (8.D) sẽ chạm đúng chỗ này.

## 7. Mục bắt buộc (f) Trích dẫn ADR trong đề xuất ↔ `docs/DECISIONS.md`
| ADR | Đề xuất dùng để nói | Đối chiếu | Nhãn |
|---|---|---|---|
| 013 | AST `Box` có lập luận (khớp mẫu, round-trip) | `:100` tiêu đề "gt-ast: cây Box + NodeId dày đặc (M2)"; `:132` nói về printer | ĐÚNG MỘT PHẦN (lý do chính chưa đọc) |
| 020 | MIR phẳng theo chỉ số | `:253` "M5: MIR, khung dataflow…"; `mir.rs:58-68` | ĐÚNG |
| 029 | LLVM trong TCB, FFI thô ở `gt-llvm` | `:778,887` (ranh giới `unsafe`: `gt-llvm/src/shim.rs`, `gt-codegen-llvm::gep`, `runtime/gt-rt`) | ĐÚNG |
| 036 | release-o2 chưa mặc định; Guard-post | `:1062,1108-1110`; `gt-guard` `check_post` `:342` | ĐÚNG |
| 038 | Arena/Vec/map | `:1128`; trạng thái 2026-11-18: ADR-C03 là bản hiện hành (`:1302,1319`): `Vec` giữ `&'a Arena` CHIA SẺ | **ĐÚNG MỘT PHẦN**: đề xuất §0.3/§2.4 viết "M6 giữ `&!Arena` độc quyền" ngầm theo bản cũ; bản hiện hành là `&'a Arena` chia sẻ (`:1303`): luật "`push` vô hiệu mọi tham chiếu cũ" áp cho khoản vay chung, không phải khoá độc quyền. Không phá lập luận §2.4 nhưng nên nêu |
| 039 | đóng băng Obligation IR để chốt giao diện GLIR theo cùng cách | `:1200,1252` | ĐÚNG |
| (thêm) C04, C12, C13 | FREEZE, spawn/detach, declassify | `:1305,1340,1347` | ĐÚNG |

## 8. Số đếm
Xem dòng "Tổng" dưới bảng mục 1 (44 khẳng định). Mục 7 (ADR): 7 dòng = ĐÚNG 5 (020, 029, 036, 039, C04/C12/C13), ĐÚNG MỘT PHẦN 2 (013, 038).

## 9. Mục bắt buộc (g) Cỡ mã thật (đếm bằng `wc -l`, HEAD `cce0f13`)
| Thành phần | Dòng | Ước lượng đề xuất | Ghi chú |
|---|---:|---|---|
| `crates/gt-codegen-llvm/src/lower.rs` | **3 654** | ~3 650 | khớp |
| `crates/gt-codegen-llvm/src/lib.rs` | 57 | — | cộng `lower.rs` = 3 711 |
| `crates/gt-guard/src/lib.rs` | **517** | 517 | khớp (một tệp) |
| `crates/gt-llvm/src/*.rs` | **66** (`lib.rs` 5 + `shim.rs` 61) | 66 | khớp |
| `crates/gt-driver/src/*.rs` | **2 851** (cli 620, lsp 545, pipeline 475, native 445, render 299, sha256 187, runner 146, verdict 112, lib 17, main 5) | — | gồm LSP (545) và SHA-256 (187) không thuộc việc chọn backend |
| Backend x64 riêng "8–15 nghìn dòng" | không đo được | 8 000–15 000 | **ƯỚC LƯỢNG, không có nguyên mẫu**; so với `lower.rs` 3 654 dòng (hạ MIR→LLVM, chưa gồm chọn lệnh/ghi object): hệ số nhân 2,2–4,1× so với codegen LLVM hiện có; KHÔNG dùng làm cam kết. Tham chiếu QBE "10% mã của trình biên dịch công nghiệp" chưa đối chiếu |
| `runtime/gt-rt/src/lib.rs` | 1 104 (test từ dòng 756) | — | phần sản phẩm ≈ 755 dòng |

## 10. Mục bắt buộc (h) DANH SÁCH ĐO SAU KHI CỔNG rc6 XONG (KHÔNG chạy bây giờ)
Điều kiện chung: chạy SAU khi cổng đầy đủ rc6 kết thúc (một Cargo tại một thời điểm; `CARGO_BUILD_JOBS=2`; preflight PASS trước); trong clone sạch nếu có phiên khác ghi vào kho; chỉ báo số đo THẬT; đây là phép đo, không phải cổng. Máy: i5-12450H Windows 11 (số của `BENCHMARK.md`); nhiễu quét tệp của Defender (dùng **min**).
| # | Việc đo | Lệnh chính xác | Công cụ | Đầu ra mong đợi / trả lời được gì |
|---|---|---|---|---|
| 1 | Thời gian dựng `gritc` bản **release** và thời gian `gritc check`/`build` chương trình mẫu bằng gritc release | `CARGO_BUILD_JOBS=2 cargo build --release -p gt-driver` (ghi thời gian tường); sau đó `GRITC=target/release/gritc.exe python tools/bench_profile.py 7` (sửa biến `GRITC` đã được script đọc) | `cargo`, `tools/bench_profile.py` | cột "dựng": giây mỗi bench với gritc release; trả lời "thời gian biên dịch có phải nút thắt không" (mục 2.5/2.6) |
| 2 | Profile `sortlines`: thời gian ở đâu (hạ mã, `memcpy`, kiểm biên, cấp phát) | (a) `gritc build examples/sortlines --profile=dev -o s_dev.exe` rồi chạy trên đầu vào 33 MB của `tools/bench_sortlines_g2.py`; (b) Windows: `xperf`/WPR hoặc Visual Studio "CPU Usage" hoặc `perf` trên Linux (`perf record -g ./s_dev.exe < in; perf report`) | WPR/xperf hoặc perf; `tools/bench_sortlines_g2.py` | bảng % thời gian theo hàm (`gt_entry`, `gritrt_arena_alloc`, `gritrt_arena_verify`, `memcpy`, sort). CHỈ đo mới trả lời được; không đoán |
| 3 | Phần `memcpy` trong `Vec::push` | (a) `gritrt` có biến: đếm số lần `vec_push` gọi `grow` và tổng byte `memcpy` — cần thêm bộ đếm tạm trong BẢN DỰNG THỬ ngoài cây (KHÔNG sửa mã chính) hoặc dùng `perf stat`/`ltrace`-tương đương; (b) so `sortlines` (`reserve` đúng một lần: `BENCHMARK.md` mục `arena_overlap`) với chương trình `push` từng phần tử không `reserve` | bản dựng thử; `perf` | tỷ lệ `memcpy`/tổng; ghi chú: `sortlines` có thể gần như không dùng `push` tăng trưởng ⇒ không dùng nó làm ca đại diện cho mục 2.4 |
| 4 | Cỡ enum THỰC trong IR của 6 công cụ | `gritc build examples/<t> --profile=dev --emit-ir` NẾU tuỳ chọn có (CHƯA tìm trong `USAGE`; nếu không: `gritc build … -o x.exe` rồi đọc `.ll` qua test hiện có `llvm_diff*`; tên `json sha256 sortlines stats wc wordfreq`) ; đếm `type { i64, [k x i64] }` bằng `grep -o "{ i64, \[[0-9]* x i64\] }"` | `grep`; golden `.ll` | bảng cỡ `8+8k` và số lần dùng theo công cụ ⇒ xác định 7a có lợi bao nhiêu byte (mục 2.7) |
| 5 | Thời gian chạy dev vs release-o2 vs Rust `-O` cho cả 6 công cụ trên cùng máy, hai lượt | `BENCH_PROFILE=release-o2 python tools/bench_profile.py 9` + `python tools/bench_sortlines_g2.py` | `tools/bench_profile.py` (hiện CHỈ fib + 5; `wordfreq` ngoài) | số **min** theo công cụ; sửa sai lệch #19 (6 vs 5) |
| 6 | Tần suất lỗi đảo đối số trong `apps/` và `docs/USER_BUGS.md` | `grep -n -i "đảo\|swap\|đổi chỗ\|thứ tự đối số\|argument order\|nhầm đối số" docs/USER_BUGS.md docs/FIELD_BUGS.md apps -r`; rồi đọc từng `E0308` trong lịch sử (`git log --all --oneline -S"E0308" -- apps/`) và phân loại tay | `grep`, `git log -S` | số lỗi đảo đối số / tổng lỗi người dùng trong `USER_BUGS.md` (23 dòng) và `apps/`; nếu ~0 thì N9 KHÔNG đủ điều kiện (đề xuất tự đặt điều kiện này) |
| 7 | Oracle "ghi thẳng vào đích" (`sret`) ≡ bản cũ khi abort giữa chừng (mục 2.1) | cần một bản thử `lower.rs` (ngoài cây chính): chạy `llvm_diff_abort` + `llvm_diff_refpat` trên bản thử | `crates/gt-driver/tests/llvm_diff_abort.rs` | CHỈ đo/chạy được; chưa thiết kế |
| 8 | `cargo build --no-default-features -p gt-driver` xanh? và `gritc check` chạy mà không có LLVM? | `CARGO_BUILD_JOBS=2 cargo build --no-default-features -p gt-driver` rồi `gritc check tests/pass/*.gt` | `cargo` | xác nhận mục 5 (dựng không LLVM) bằng đo, không chỉ bằng đọc |
| 9 | Chi phí commit charge của Arena đặt trước (mục 2.4) trên máy dự án | thử nguyên mẫu `VirtualAlloc(MEM_RESERVE)` ngoài cây + `Get-Process`/`perfmon` "Commit Charge" | PowerShell | số MB commit; trả lời có chạy được trên máy thiếu bộ nhớ ảo không |

## 11. Mục bắt buộc (i) SAI LỆCH giữa đề xuất và mã (ghi, KHÔNG sửa đề xuất, KHÔNG sửa mã)
1. **SAI #19:** "6 dogfood (`tools/bench_profile.py`)": script đo `fib` + 5 công cụ; `wordfreq` không có (`BENCHMARK.md:21`).
2. **SAI một phần #16:** "tham chiếu luôn khác 0": con trỏ béo của chuỗi/lát RỖNG có thể là `null + 0` (`lib.rs:278-291,823-826`); niche chỉ áp cho `&T` mỏng.
3. **ĐÚNG MỘT PHẦN #2:** ba số tuyệt đối trộn hai lượt đo; tỷ lệ ~1,5× giữ ở cả hai.
4. **ĐÚNG MỘT PHẦN #10:** `gritc serve` chưa có mã; chỉ có trong kế hoạch.
5. **ĐÚNG MỘT PHẦN #32:** mã thoát 3 KHÔNG phủ lỗi do Guard bắt (mã 1) hay lỗi nội bộ/liên kết (mã 2); x64 phải theo cùng bảng này.
6. **ĐÚNG MỘT PHẦN #42:** đổi tag `u8/u16` chạm ≥ 6 cụm hạ (`Option`/`Result`/I/O) trong `lower.rs`, không chỉ "bố cục".
7. **Ngầm theo ADR-038 cũ (mục 7):** `Vec` giữ `&'a Arena` chia sẻ từ ADR-C03, không phải `&!Arena` độc quyền.
8. **Mâu thuẫn spawn/detach (mục 6):** không có trong đề xuất; vẫn mở.
9. **Chưa có trong đề xuất:** hệ số tăng `Vec` thực là ×2 sàn 4; chunk Arena không nhân đôi (cho thấy lãng phí "~2×" của đề xuất là của `Vec`, không phải của Arena).
