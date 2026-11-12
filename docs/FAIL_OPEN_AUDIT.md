# FAIL_OPEN_AUDIT — rà soát "im lặng chấp nhận" ở các crate quyết định (2026-10-17)

<!-- FOS:BEGIN -->**Đã rà / tổng chỗ khớp = 1292 / 1292** (CHƯA RÀ 0; `python tools/fail_open_scan.py stats`; dấu vân tay ở `docs/FAIL_OPEN_LEDGER.jsonl`). "Đọc mã" KHÁC "chứng minh".<!-- FOS:END -->

KẾT QUẢ TÓM TẮT (cập nhật R3, 2026-10-18): đợt 1 sửa 6 chỗ (G1, S1, B1, B2, B3, D1); đợt 2 sửa thêm (mục 7.1); R3 rà NỐT 1292/1292 chỗ khớp bằng sổ cái và tìm thêm 3 lỗi thật (FB-17, FB-18, FB-19, mục 8) cùng 6 sửa phòng thủ; phần còn lại AN TOÀN/GHI NHẬN theo ĐỌC mã (không phải chứng minh). Bảng "cách kiểm chứng" ở mục 0.

Mục tiêu (lệnh Giang): tìm chỗ mà dữ liệu thiếu / lỗi / `None` / nhánh mặc định bị đọc thành "ĐÚNG/an toàn". Quy tắc đối chiếu: `grit-contract.md` §3.3 "không có khẳng định tường minh thì không phải đã chứng minh".
Phương pháp: `grep` mẫu `unwrap_or(`, `unwrap_or_default`, `.ok()`, `let _ =`, `_ =>`, `.get(`, `continue`/`return` sớm, `map_or`, `is_some_and`, `is_none_or`, rồi ĐỌC ngữ cảnh từng lần gặp ở các crate ưu tiên. Mỗi dòng: `tệp:dòng | mẫu | kết luận | lý do / cách sửa`. Kết luận: **AN TOÀN** (thiếu ⇒ từ chối/ICE hoặc không ảnh hưởng phán quyết), **SỬA** (fail-open thật hoặc đủ nghi: sửa với test hồi quy, commit riêng, FIELD_BUGS), **GHI NHẬN** (chấp nhận có lý do, nêu giới hạn).

## 0. Các mục đã sửa và CÁCH KIỂM CHỨNG (nói đúng mức bằng chứng)
| mục | sửa | bằng chứng | giới hạn |
|---|---|---|---|
| G1 (gt-guard) | địa chỉ `gritrt_*` chỉ được là callee trực tiếp của `call`; `invoke`/`callbr` bị từ chối | test hồi quy commit ĐỎ trước (`a76ae54`) rồi xanh (`575e6d8`); gt-guard 28/28; FB-10 | đột biến chính thức cho quy tắc mới CHƯA chạy |
| S1 (gt-secrets) | `ty_has_secret` quá 12 tầng ⇒ `true` (bảo thủ) | test đỏ trước (độ sâu 13 ⇒ không cảnh báo), rồi xanh; gt-secrets 15/15 | bộ quét là cảnh báo vàng, không phải kiểm tra an toàn |
| B1 (gt-borrowck) | mã chẩn đoán sai định dạng ⇒ `E9901` thay vì bị nuốt | test đơn vị `diag_code`; **test và sửa cùng một commit** (không dựng được đỏ trước vì hàm tách mới) | không có đường nguồn Grit nào chạm tới (mã là hằng trong crate) |
| B2/B3 (gt-borrowck) | thiếu slot region / ADT / tổ hợp (phép chiếu, kiểu) lạ ⇒ `defend` (ICE E9901) | KHÔNG dựng được phản ví dụ từ mã nguồn Grit (không với tới được khi M4 đúng); bằng chứng là bất biến sẵn có `defensive_counters()==(0,0)` trên oracle/corpus vẫn xanh sau sửa (gt-borrowck: toàn bộ test xanh) | không có test ĐỎ→XANH; chỉ chứng minh sửa không làm chương trình hợp lệ chạm chốt chặn |
| D1 (gt-driver) | `mir_dump` không bỏ instance thiếu khoá khỏi băm (`?missing:<node>`) | `tests/instance_keys.rs`: mọi instance của chương trình generic đều có khoá | nhánh `?missing` không có test chạm trực tiếp (cần `Artifacts` hỏng) |


## 1. gt-guard (`crates/gt-guard/src/lib.rs`) — ưu tiên 1, ĐỌC TOÀN BỘ 405 dòng
| vị trí | mẫu | kết luận | lý do |
|---|---|---|---|
| 46 | `is_some_and` (`support_fn`) | AN TOÀN | tên không khớp ba dạng hỗ trợ ⇒ không phải hàm hỗ trợ ⇒ phải thuộc bảng instance, nếu không bị từ chối |
| 55–56 | `CallSiteValue::try_from(..).ok()?`, `get_called_fn_value()?` trong `callee_name` | **ĐÃ SỬA (G1)** | trả `None` cho lời gọi GIÁN TIẾP (con trỏ hàm) và cho mọi opcode không phải `call` (`invoke`, `callbr`): các luật "lớp nghĩa vụ của lời gọi runtime" (`required_class`) và "đích mọi lời gọi" (3b) chỉ chạy khi `Some` ⇒ gọi `gritrt_*` qua con trỏ, hoặc `invoke`/`callbr`, KHÔNG bị kiểm. Sửa: (a) cấm `invoke`/`callbr`; (b) địa chỉ của hàm `gritrt_*` chỉ được dùng làm đích gọi TRỰC TIẾP |
| 62–67 | `?` chuỗi trong `obl_class` | AN TOÀN | thiếu metadata ⇒ `None` ⇒ nhánh "thiếu metadata !gt.obl" ⇒ vi phạm |
| 73–75 | `is_ok_and(ordering != NotAtomic)` | AN TOÀN | chỉ gọi cho Load/Store (luôn `Ok`) |
| 127–129 | `continue` bỏ qua hàm không thân / hàm hỗ trợ | AN TOÀN | khai báo không cần thuộc bảng instance (đích gọi bị luật 3b kiểm); hàm hỗ trợ vẫn chịu mọi luật còn lại |
| 141, 297 | `get_node_values().unwrap_or_default()` | AN TOÀN | thiếu metadata ⇒ tập rỗng ⇒ lệch `defined` hoặc "thiếu !gt.attest" ⇒ vi phạm |
| 146 | `split('=').next().unwrap_or("")` | AN TOÀN | rỗng ⇒ lệch tập |
| 207–209 | `map_or_else(.., "?")` loại abort không hằng | AN TOÀN | khoá "?" riêng; tối ưu đổi loại hằng thành không hằng làm số lần của loại gốc GIẢM ⇒ R1 báo |
| 248 | `post.get(..).unwrap_or(0)` | AN TOÀN | thiếu ⇒ 0 < n ⇒ vi phạm |
| 270 | `callee_name().is_some_and(fused)` | GHI NHẬN | FMA gọi gián tiếp không phát hiện — nhưng gián tiếp `llvm.*` không tồn tại (intrinsic không lấy địa chỉ được); xem G1 |
| 342 | `get_function(..).is_some_and(body>0)` | AN TOÀN | không thấy hàm ⇒ "chưa định nghĩa" ⇒ từ chối trừ `llvm.*` |
| 349 | mọi `llvm.*` được phép | GHI NHẬN | verifier LLVM từ chối tên intrinsic không tồn tại; `llvm.assume` bị cấm riêng; không dùng danh sách trắng intrinsic (nới sau nếu cần) |
| 358–363 | `prev.is_some_and`, `get_function(..).is_some_and(noreturn)` | AN TOÀN | thiếu ⇒ `ok = false` ⇒ từ chối `unreachable` |
| 372–386 | miễn cờ trên LỆNH GEP (+ biểu thức hằng GEP sau FB-9) | GHI NHẬN | cờ GEP (`inbounds`, `nuw`) KHÔNG phải số học chương trình; sửa FB-9 ghi ở FIELD_BUGS |
| 376 | `split(", !").next().unwrap_or(&text)` | AN TOÀN | |

## 2. gt-secrets (`crates/gt-secrets/src/lib.rs`) — ưu tiên 2 (bộ quét kinh nghiệm: cảnh báo VÀNG, KHÔNG phải kiểm tra an toàn — KNOWN_LIMITATIONS "gritc / M9")
| vị trí | mẫu | kết luận | lý do |
|---|---|---|---|
| 165 | `str_body ... unwrap_or(tok)` | AN TOÀN | không bóc được nháy ⇒ quét nguyên token |
| 203–233 | `continue` trong S102 | AN TOÀN | bỏ qua token không phải định danh/không có gán/đã là S101 — định nghĩa của luật |
| 257–259 | `depth > 12 ⇒ false` (`ty_has_secret`) | **ĐÃ SỬA (S1)** | kiểu lồng > 12 tầng mất taint ⇒ không cảnh báo S201/S202. Sửa: quá sâu ⇒ `true` (cảnh báo bảo thủ) |
| 260–266 | `lang.get(Secret)`, `adts.get(d).is_some_and` | AN TOÀN | thiếu ⇒ không có kiểu Secret/ADT để mang taint |
| 298, 305 | `_ => Vec::new()/None` trong `rvalue_operands`/`place_local_of_rvalue` | GHI NHẬN | `Ref/Len/Discriminant` đi qua `place_local_of_rvalue`; các Rvalue còn lại không mang toán hạng đáng lan truyền |
| 323, 332, 336 | `unwrap_or_default` chữ ký/hiệu ứng callee | GHI NHẬN | chữ ký luôn có sau M4; nếu thiếu ⇒ không S202 (bỏ sót cảnh báo, không bỏ sót lỗi an toàn). Nâng thành ICE nếu bộ quét trở thành kiểm tra an toàn |
| 391–394 | `continue` không terminator/không `Call` | AN TOÀN | |
| 458, 547, 553 | `Code::parse(..).unwrap_or_else(E9901)` | AN TOÀN | mã lạ ⇒ ICE, không im lặng |
| 480–489 | `#allow` thiếu lý do ⇒ E1401 | AN TOÀN | từ chối |
| 506–507 | `Impl/Fn ⇒ {}` khi gom `#allow` | AN TOÀN | `#allow` trên thành viên impl KHÔNG được áp ⇒ cảnh báo vẫn hiện (thêm cảnh báo, không bớt) |
| 564 | `let _ = r.node` | AN TOÀN | vô hại |

## 3. gt-borrowck (`crates/gt-borrowck/src`) — đọc 30 chỗ gặp
| vị trí | mẫu | kết luận | lý do |
|---|---|---|---|
| check.rs:80, 241 | `_ => {}` trong hiệu ứng câu lệnh/terminator | AN TOÀN | các loại còn lại (Nop, StorageLive, FakeRead…) không tạo/huỷ khoản vay |
| check.rs:301 | `Code::parse(code).ok()?` trong `push` | **ĐÃ SỬA (B1)** | mã chẩn đoán không đăng ký ⇒ chẩn đoán bị NUỐT (vi phạm biến mất). Sửa: lùi về `E9901`; test: mọi chuỗi mã `"E04xx"` trong crate parse được |
| check.rs:325 | `let Some(&ad)=lang.get(Arena) else {return false}` | GHI NHẬN | prelude luôn có Arena (test prelude một nguồn); thiếu ⇒ không E0405. Giữ, có test prelude |
| check.rs:334, 337 | `has_own_drop`: `_ => false` | AN TOÀN | chỉ ADT có Drop |
| check.rs:445, 462 | `find_move_site`: nhãn/vị trí | AN TOÀN | chỉ ảnh hưởng nhãn |
| lib.rs:99 | `cause_span(..).unwrap_or(body.span)` | AN TOÀN | chỉ vị trí |
| regions.rs:125 | `univ_map.get` | AN TOÀN | nhánh tạo mới |
| regions.rs:170–199 | `rsig` thiếu/không khớp | AN TOÀN | gọi `defend` ⇒ ICE E9901 |
| regions.rs:233 | `slots.first().cloned().unwrap_or_default()` trong `place_slots` | **ĐÃ SỬA (B2)** | `Deref` của tham chiếu không có slot ⇒ bó region rỗng ⇒ thiếu ràng buộc reborrow, KHÔNG ICE |
| regions.rs:258 | `_ => {}` trong `place_slots` | **ĐÃ SỬA (B2)** | tổ hợp (phép chiếu, kiểu) lạ bị bỏ qua lặng lẽ ⇒ slot sai |
| regions.rs:272–291 | `adt_inv.get`; `None => Vec::new()` ADT không định nghĩa | **ĐÃ SỬA (B3)** | ADT thiếu ⇒ coi là KHÔNG bất biến ⇒ nới; sửa: `defend` |
| regions.rs:304, 359 | `_ => false/{}` | AN TOÀN | kiểu không có vị trí region |
| regions.rs:340, 674 | `adts.get(d).map_or(0, lt_params)` | **ĐÃ SỬA (B3)** | ADT thiếu ⇒ 0 tham số lifetime ⇒ không ràng buộc; sửa: `defend` |
| regions.rs:423 | `let _ = pty` | AN TOÀN | |
| regions.rs:502–503, 551, 567, 592 | tra trait/chữ ký | AN TOÀN | thiếu ⇒ `defend` (đã có) |
| regions.rs:709 | `names.get(r)` | AN TOÀN | chỉ tên hiển thị |
Bảo vệ sẵn có: `defensive_counters()` phải bằng (0,0) trên mọi chương trình qua M4 (khẳng định trong `oracle.rs`).

## 4. gt-driver (`crates/gt-driver/src`) — đọc 40 chỗ gặp (trừ render.rs: chỉ định dạng)
| vị trí | mẫu | kết luận | lý do |
|---|---|---|---|
| cli.rs:146–152 | `GRITC_FAULT` sai định dạng ⇒ `None` | AN TOÀN | chèn lỗi chỉ làm gritc từ chối thêm; không có lỗi ⇒ chạy bình thường |
| cli.rs:257, 429; native.rs:183, 207, 217, 244, 266–267, 277; runner.rs:138, 142 | `let _ = remove_file/write/kill` | AN TOÀN | dọn dẹp tệp tạm, không ảnh hưởng phán quyết |
| cli.rs:363 | `value.unwrap_or("()")` | AN TOÀN | chỉ in |
| cli.rs:438 | `SourceFile::new(..).ok()` | AN TOÀN | chỉ cách in chẩn đoán; tệp KHÔNG bị sửa |
| native.rs:44 | `PATH` `unwrap_or_default` | AN TOÀN | rỗng ⇒ không thấy DLL ⇒ lỗi Env rõ |
| native.rs:91–111 | `GRITC_FAULT` ⇒ `None` | AN TOÀN | như trên |
| native.rs:249, 273–275 | đọc tệp kết quả/stdout/stderr `.ok()/unwrap_or_default` | GHI NHẬN | thiếu tệp kết quả ⇒ `value = None`; phán quyết chạy dựa MÃ THOÁT (0/70/khác) chứ không dựa tệp |
| native.rs:289–292 | tên abort `unwrap_or("unknown")`, mã `clamp/unwrap_or(255)` | AN TOÀN | mã lạ ⇒ `ProgramExit(255)` (khác 0) |
| pipeline.rs:223 | `_ => record Done` ở `alias` | AN TOÀN | chỉ ghi nhãn pass phụ khi không có lỗi chèn |
| pipeline.rs:441 | `fn_keys.get(&i.node)?` trong `mir_dump` | **ĐÃ SỬA (D1)** | instance thiếu khoá bị BỎ khỏi băm bảng instance của attestation (attestation phủ ít hơn). Sửa: đưa dấu `?missing` vào danh sách để băm đổi; test bất biến: mọi instance có khoá |
| verdict.rs (không có hit) | — | AN TOÀN | `decide`/`allows_output` đã đọc: ĐỎ/ICE/thiếu pass bắt buộc ⇒ không output |

## 5. gt-mir (`crates/gt-mir/src`) — CHỈ các chỗ quyết định (176 hit; KHÔNG đọc từng dòng, xem "Chưa rà")
Đã đọc mọi `_ => {}`, `_ => None/false/true`, `_ => Ok(())` trong `build/*.rs`, `lib.rs`, `types.rs` (12 chỗ): `expr.rs:92` (bảng ép kiểu mất thông tin: lạ ⇒ `true` = chèn kiểm tra, bảo thủ); `:377`, `:657`, `:729–735`, `:764`, `:988`, `:1535`, `:1574` (không nhận ra ⇒ đường tổng quát/tạo tạm, không bỏ câu lệnh); `matching.rs:138` (`destructures`, đối chiếu chéo với gt-types bằng `pat_modes`); `lib.rs:66` (`index_items` bỏ qua mục không có thân). KẾT LUẬN: không thấy chỗ nào BỎ lệnh hạ MIR im lặng; mọi lỗi dựng đi qua `ice(..)` ⇒ E9901.

## 6. gt-types (`crates/gt-types/src`) — CHỈ mẫu quyết định (217 hit; KHÔNG đọc từng dòng)
Đã đọc: `check/mod.rs:891` (`place_mutable`: local thiếu trong bảng ⇒ coi là ghi được, `Ty::Err` ⇒ ghi được: chỉ để không kéo thêm lỗi dây chuyền sau một lỗi kiểu đã báo; **GHI NHẬN**), `db.rs:44` (`imut`: tham số kiểu không tra được ⇒ coi là có ràng buộc, bảo thủ), `check/pat.rs` (mã mới Q87, có đột biến). Chưa đọc phần còn lại.

## 7. Đợt 2 (lệnh 2026-10-17 lần 3): gt-conc, gt-types, gt-effects, gt-mir, gt-term/oblig, các crate còn lại
Phương pháp như trên (đọc ngữ cảnh từng chỗ khớp danh sách tìm kiếm); mỗi chỗ SỬA đi theo quy trình: thực nghiệm → phân loại → test ĐỎ (commit riêng, hoàn nguyên thấy đỏ) → sửa → commit.

### 7.1 Các chỗ đã SỬA (đợt 2)
| mã | vị trí | phân loại | sửa | bằng chứng |
|---|---|---|---|---|
| S-A | `gt-types/src/send.rs` (ADT không có trong db ⇒ `Ok`; `Mutex` thiếu đối số ⇒ `Ok`) | chỉ qua dựng bộ nhớ | `Err` nội bộ ⇒ E9901 | 2 test đỏ→xanh (`send::tests`), đột biến |
| S-E | `gt-conc`: phương thức `Atomic` làm giá trị né kiểm ordering | LỖI THẬT (đạt tới được) | `Const::FnItem` trỏ Atomic ⇒ E0503 | test đỏ→xanh, corpus, bộ sinh |
| FB-13 | `gt-mir/src/build/expr.rs`: `Res::Assoc` làm giá trị ⇒ đệ quy vô hạn | LỖI THẬT (gritc chết) | `Const::FnItem`; ICE nếu không hạ được | test đỏ (tràn ngăn xếp)→xanh |
| FB-14 | `gt-types` pat/exhaust: literal u128 bị kẹp, miền cắt | LỖI THẬT (E0305 sai) | ADR-041; oracle | test đỏ→xanh, oracle 13 kiểu |
| S-C | 4 nơi nuốt mã chẩn đoán sai định dạng (gt-mir `diag`, gt-resolve `mk`, gt-parser `push_diag`, gt-types `Env::err`) | chỉ qua dựng bộ nhớ | `Code::parse_or_internal` | 4 test đỏ→xanh (`lex_error`: enum đóng, không dựng được mã sai) |
| S-D | `gt-mir/src/validate.rs`: Ty::Err/Infer trong MIR | chỉ qua dựng bộ nhớ | validator từ chối | test đỏ→xanh, đột biến đoạn mới |
| — | `gt-effects` dòng 50: thân Fn không có chữ ký bị `continue` | chỉ qua dựng bộ nhớ | ICE E9901 | test đỏ→xanh |
| — | `needs_drop`/`has_own_drop`: ADT lạ ⇒ "không cần drop" (nuôi E0318, E0407) | chỉ qua dựng bộ nhớ | bảo thủ + ICE | test đỏ→xanh (gt-types, gt-borrowck) |
| FB-15 | `gt-types/check/call.rs::apply_pick_types`: hiệu ứng RỖNG cho hàm kết hợp làm giá trị | LỖI THẬT (`! ∅` gọi được `io`/`mem`) | `pick_effects` | test đỏ→xanh, corpus, bộ sinh |
| mục 2 | intrinsic của lib/core làm giá trị (né precondition, E0310, E0405…); `root_fs` làm giá trị trong `main` (điều kiện #12) | LỖI THẬT (check xanh sai: 26/43 chương trình thử) | E0313 ở M4 | `docs/INTRINSIC_AS_VALUE.md`, test đỏ→xanh, corpus, bộ sinh |
| mục 4 | `exhaust.rs`: miền `char` coi liền khối | bảo thủ (E0305 oan) | hai đoạn | oracle char đỏ→xanh |

### 7.2 Đã ĐỌC và kết luận AN TOÀN / GHI NHẬN (đợt 2)
- **gt-conc/src/lib.rs (toàn bộ 20 chỗ):** `variant_of` mọi nhánh `None` ⇒ E0503 (bảo thủ); `check_orderings`: `lang.get(AtomicOrdering)` thiếu ⇒ return (prelude luôn có; GHI NHẬN); `sig_by_node` None ⇒ `continue` (GHI NHẬN: thành viên impl Atomic luôn có chữ ký; không dựng được test vì cần TypesOutput hỏng); `o(k).unwrap_or(4)` (thiếu tham số ordering ⇒ SeqCst mạnh nhất, chỉ khi chữ ký prelude thiếu tham số; GHI NHẬN); `Callee::TraitMethod`/`Value` không rà (Atomic không có thành viên trait; giá trị hàm Atomic nay bị E0503).
- **gt-effects (toàn bộ 384 dòng):** ngoài chỗ đã sửa: `Callee::Value` không có kiểu hàm ⇒ E9901; `Levels::new` kiểu không `#level` bị bỏ qua (nếu dùng trong `lock(L)` thì E0502 "thiếu #level"); `Callee::TraitMethod` không bung `poly`/`lock(..)` của thành viên trait (GHI NHẬN: trait prelude `Ord`/`Drop` không có marker); Drop-glue không tính hiệu ứng: AN TOÀN vì `Drop::drop ! ∅` bắt buộc (E0802) và chỉ prelude được viết `Drop`.
- **gt-types:** `db.rs` `satisfies_bound`/`is_copy`/`needs_drop`/`resolve_trait_method` (đọc; `Ty::Infer ⇒ true` hoãn và có `unresolved` ở `finish`: AN TOÀN); `exhaust.rs` (đọc toàn bộ: `sub_tys` trả rỗng cho ADT lạ — GHI NHẬN, nối với db không nhất quán); `check/mod.rs` `place_mutable`: local thiếu trong bảng ⇒ coi là ghi được (GHI NHẬN: chỉ tránh lỗi dây chuyền sau lỗi đã báo; chưa chứng minh không tới được); `finish` `continue` sau `unresolved` (đã báo lỗi); `collect.rs` `eval_usize` None ⇒ độ dài mảng không xác định (E0301 ở nơi dùng).
- **gt-term:** `by_node.get(..)` None ⇒ `continue` ở hai chỗ (instance không có thân: intrinsic/khai báo; AN TOÀN), terminator không phải `Call` ⇒ `continue`, `fns.get(&b.node)` None / thiếu hợp đồng variant ⇒ `continue` (vòng lặp không biến thể xử lý chỗ khác); `conjuncts` không hiểu ⇒ bỏ (bớt giả thiết = bảo thủ).
- **gt-oblig:** `analysis.rs:1996,2038` khối chưa có trạng thái ⇒ bỏ (khối không tới được); `driver.rs:501-506` hợp đồng Pre/Post không tìm thấy chủ ⇒ `continue` (GHI NHẬN: assert `Precondition` lúc chạy vẫn còn vì chỉ xoá khi chứng minh); `env.rs:448` zone không có cạnh ⇒ không giả thiết (bảo thủ). Các chỗ khác: xem 7.3.
- **gt-mir:** `matching.rs` Q87 (đã có đột biến); `expr.rs` các `unwrap_or(Ty::Err)` ×3 (nay bị S-D chặn ở validator); `mod.rs` `register` (frame rỗng ⇒ local không đăng ký, chỉ ảnh hưởng StorageDead/drop; GHI NHẬN); `ty_of` None ⇒ ICE; `types.rs` `variant_fields` ADT lạ ⇒ rỗng (GHI NHẬN, nối db không nhất quán).
- **gt-lexer (3 chỗ `.ok()`/escape), gt-parser (danh sách), rig (14 chỗ), gt-dataflow (framework `unwrap_or_default` terminator None, `movable_prefix`), gt-resolve (`Found::Silent`, `unwrap_or` chỉ định vị):** đọc, AN TOÀN (fail-closed hoặc chỉ ảnh hưởng vị trí/nhãn).

### 7.3 (ĐÃ ĐÓNG bởi R3, 2026-10-18) — bảng "CHƯA RÀ" cũ được thay bằng sổ cái từng chỗ
Mọi chỗ khớp (1292) đều có dòng trong `docs/FAIL_OPEN_LEDGER.jsonl` với vân tay (`tools/fail_open_scan.py`), kết luận (`an_toan` 1269 / `da_sua` 23 / `nghi_van` 0 / `khong_ra_duoc` 0) và lý do. `python tools/fail_open_scan.py check` ⇒ "CHƯA RÀ 0; nghi vấn chưa xử lý 0". Dòng sổ cái gắn vân tay theo nội dung dòng: sửa dòng đó làm sổ cái "lỗi thời" và `check` đòi rà lại.

## 8. R3 — kết quả rà nốt (lệnh chạy liên tục 2026-10-18)
**"Đọc mã" KHÁC "chứng minh".** Con số 1292/1292 nghĩa là: mỗi chỗ khớp mẫu đã được ĐỌC cùng ngữ cảnh và phân loại, KHÔNG phải mỗi chỗ đã được chứng minh an toàn. Phạm vi: chỉ các mẫu của công cụ (`unwrap_or`, `.get(`, `.ok()`, `let _ =`, `_ =>`, `else { continue/return }`, `map_or`, `is_some_and`, `is_none_or`…) trong `crates/**/src` và `runtime/gt-rt/src`; KHÔNG phủ: fail-open viết dưới dạng khác (vd `if cond { return }` tự chế, `match` với nhánh đầu đã sai), các tệp `tests/`, `build.rs`, `tools/*.py|sh`, mã sinh. Tệp không có chỗ khớp (gt-ast, gt-stack, gt-llvm) không có dòng sổ cái.

### 8.1 LỖI THẬT tìm thấy khi rà R3 (mỗi cái: thí nghiệm → test ĐỎ → sửa → FIELD_BUGS)
| mã | vị trí | loại | sửa / bằng chứng |
|---|---|---|---|
| FB-17 | `gt-types/src/validate.rs::validate_copy_impl` | LỖI THẬT, đạt tới từ Grit: `⊐ <T> Copy for W<T> { }` không `T: Copy` ⇒ `W<Cap<Fs>>`/`W<Arena>` Copy (nhân đôi capability — điều kiện #12; đo bằng `a ≔ W { v: root_fs() }; b ≔ a; c ≔ a;` xanh) | E0311; ma trận hình dạng trường × ràng buộc (`gt-types/tests/copy_impl_bounds.rs`) + 3 fail / 1 pass corpus; test đỏ commit `36ac79d` |
| FB-18 | `gt-mir/src/interp.rs` | LỖI THẬT của backend thông dịch: u128 ≥ 2^127 so sánh/chia/kiểm tràn như số có dấu (`u128::MAX > 5` ⇒ false) | `binop_u128`, kiểm tràn và ép kiểu; `gt-mir/tests/interp_u128.rs`; test đỏ `93aa345` |
| FB-19 | `runtime/gt-rt/src/lib.rs::arena_limit` | LỖI THẬT nhẹ: `GRITRT_ARENA_LIMIT` sai định dạng ⇒ "không giới hạn" | abort `arena_exhausted`; `gt-driver/tests/llvm_vec.rs::malformed_arena_limit_is_not_silently_unlimited` |

### 8.2 Sửa phòng thủ (không dựng được chương trình Grit chạm tới; không có test ĐỎ→XANH — nói thẳng)
- `gt-types/validate.rs`: thiếu ADT/trait trong db ⇒ E9901 (trước: bỏ qua lặng lẽ kiểm Copy/impl); thiếu `region_sig` ⇒ không tương thích (trước: `true`).
- `gt-types/db.rs` `imut`: ADT lạ ⇒ `true` (commit `9407370`, có test xanh; test đỏ có chủ ý ở `6c50640`).
- `gt-borrowck/regions.rs`: đích của `Aggregate::Tuple` không phải tuple ⇒ `defend` (E9901), trước: `return`.
- `gt-secrets/lib.rs`: hiệu ứng callee không tra được ⇒ coi là `io` (bảo thủ) cho S202.
- `gt-resolve/walk.rs`, `lib.rs`: module/tệp không có định nghĩa ⇒ E9901 thay vì bỏ qua cả nội dung.
- `gt-driver/cli.rs::load`, `rig/cli.rs` (`test`, `fmt`): mục thư mục đọc lỗi không còn bị bỏ khỏi tập đầu vào/test (trước: `filter_map(Result::ok)`/`.flatten()` ⇒ có thể xanh trên tập con). Không có test tự động (cần lỗi I/O thư mục).

### 8.3 Giới hạn đã biết phát hiện thêm
- `find_impl` kiểm bound của impl ADT khác với scope RỖNG ⇒ `T: Copy` do người dùng khai báo không được công nhận qua `Option<T>` (từ chối oan, không mất an toàn) — KNOWN_LIMITATIONS, khoá bằng `known_incompleteness_…`.
- Thông dịch MIR: toán tử bit/dịch cho u128 chưa có (báo `Bug`, đóng); so sánh thông dịch ≡ LLVM cho u128 chưa làm (LLVM thiếu chia 128 bit).
## 7. CHƯA RÀ (nói thẳng)
- gt-mir: ~160 hit (`unwrap_or`, `.get(`, `.ok()`) chưa đọc từng dòng; gt-types: ~200 hit chưa đọc từng dòng; gt-oblig/gt-term/gt-effects đã rà ở mục B (FB-7, `FAIL_OPEN` trước); gt-conc, gt-dataflow, gt-resolve, gt-parser, gt-lexer, rig: chưa rà đợt này.
- Kết quả "AN TOÀN" là kết quả ĐỌC MÃ, không phải chứng minh; chỉ các mục SỬA có test.

## F-01…F-08 của kiểm toán ngoài (2026-10-21; nền tảng Linux) — kiểm "đỏ trước": hoàn nguyên TỪNG bản sửa trong cây làm việc, chạy kiểm chứng riêng, khôi phục bằng `git checkout`
| Mục | Bản sửa (commit) | Kiểm khi HOÀN NGUYÊN bản sửa | Kết quả đo | Test giữ lại |
|---|---|---|---|---|
| F-01 / FB-C1 | `1f0aab0` `gt-borrowck/src/check.rs` | `cargo test -p gt-borrowck --test same_stmt` | ĐỎ: 2 test thất bại, 1 đạt (trước sửa) → XANH 3/3 sau sửa | `same_stmt.rs`, 14 mẫu `E0401_{same_stmt,dup}_*` |
| F-02 / FB-C7 | `1f0aab0` `gt-driver/src/cli.rs` | `cargo build -p gt-driver --no-default-features` | ĐỎ: `error[E0433]: cannot find module or crate gt_codegen_llvm` | cổng chạy `test -p gt-driver` ở chế độ mặc định; CI có bước `--no-default-features` (chưa chạy trên GitHub) |
| F-03/F-04/F-07 / FB-C2, FB-C4 | `9af99e9` `gt-types/src/collect.rs` | corpus 904 ca qua `gritc` | ĐỎ: lệch 9 ca (trước sửa) → 0 sau sửa | 10 mẫu `E0301_*` + 1 mẫu pass |
| F-05 / FB-C3 | `d0a9180` `gt-types/src/check/call.rs` | corpus 904 ca | ĐỎ: lệch 1 (`E0310_call_through_fn_value` XANH) → 0 sau sửa | 2 mẫu fail `E0310_call_through_*`, 1 mẫu pass |
| F-06 / FB-C8 | `d0a9180` `gt-rt` `gritrt_start` | `cargo test -p gt-driver --test stack_depth` | ĐỎ: 0 đạt / 2 thất bại (bị HĐH giết) → 2/2 sau sửa | `stack_depth.rs` |
| F-08 | `05b4488` `docs/errors/E12xx,E1301,E9901` | `gritc explain <mã>` | ĐỎ trước sửa (ĐO ở kiểm toán G07: 6 mã báo LỖI) → XANH | `tools/check_expectations.py` + giai đoạn gate |
| F-09 (vụn vặt) | — | — | clippy 2 lỗi test → 0 | gate `clippy` |
Ghi chú trung thực: F-02 và F-08 không có test tự động riêng (F-02 do chính cổng/CI ở chế độ `--no-default-features`; F-08 do `check_expectations.py`); bằng chứng đỏ-trước là lệnh nêu ở cột 3 chạy thật.

## 8. Rà soát toàn bộ Seed 1.0 S7 — 469 vị trí, mỗi vị trí ĐỌC và phân loại (2026-10-23; Linux)

Phương pháp: `python3 tools/fail_open_scan.py` (mẫu: `unwrap_or`, `unwrap_or_default`, `.ok()?`, `let _ =`, nhánh `_ =>`, `map_or(true|false`, `is_none_or`, `is_some_and`, `else {}`; 11 crate quyết định phán quyết; bỏ phần `#[cfg(test)]`). Năm subagent mới (không bối cảnh) mỗi người đọc ~94 vị trí, theo dõi tới nơi quyết định, VIẾT chương trình tái hiện cho mọi nghi vấn và chạy `gritc` thật; kết quả từng vị trí ở `docs/fail_open_classification.tsv` (cột: tệp, dòng, mẫu, phân loại, tham chiếu sửa, lý do, dòng mã chuẩn hoá). Phân loại: **AN TOÀN** (có lý do cụ thể: đường tới đó bị chặn/đóng, hoặc hướng bảo thủ), **SỬA** (có đường thật làm âm tính giả), **GHI NHẬN** (nhánh phòng thủ nuốt lỗi nhưng đã có chốt chặn phía sau; nói thẳng). Người đọc là subagent CÙNG HỌ model với tác giả mã, không phải người độc lập.

| crate | AN TOÀN | GHI NHẬN | SỬA |
|---|---|---|---|
| gt-borrowck | 10 | 0 | 0 |
| gt-codegen-llvm | 26 | 2 | 1 |
| gt-conc | 4 | 1 | 0 |
| gt-driver | 44 | 4 | 0 |
| gt-effects | 5 | 0 | 0 |
| gt-guard | 12 | 2 | 1 |
| gt-mir | 110 | 4 | 0 |
| gt-oblig | 90 | 3 | 4 |
| gt-secrets | 7 | 5 | 0 |
| gt-term | 14 | 0 | 2 |
| gt-types | 103 | 13 | 2 |
| **tổng 469** | **425** | **34** | **10** |

### 8.1 SỬA — cả 10 đã sửa, mỗi cái có test ĐỎ TRƯỚC (commit riêng) và mục `FB-C…` ở `docs/FIELD_BUGS.md`

| vị trí (lúc rà) | vấn đề | sửa |
|---|---|---|
| `gt-codegen-llvm/src/lower.rs:1110` | prescan coi local `AtomicOrdering` là hằng nếu chỉ có MỘT câu lệnh gán trực tiếp, nhưng KHÔNG tính ghi gián tiếp qua `&!o` (gán `*p` đếm cho local `p`, không phải `o`) ⇒ ordering bị cố định sai (cùng giới hạn ở gt-conc::variant_of nên E0503 | FB-C22 |
| `gt-guard/src/lib.rs:57` | callee_name trả None cho lời gọi gián tiếp; luật địa chỉ G1 chỉ cấm lấy địa chỉ hàm gritrt_*, nên khai báo ngoài khác (vd declare @system) có địa chỉ được lưu rồi call gián tiếp thì không bị luật 3b (đích mọi lời gọi) hay luật lớp nghĩa vụ  | FB-C27 |
| `gt-oblig/src/analysis.rs:1181` | nguyên tử `unrep_atom` = UNREP_BASE + bb*1024 + idx tràn sang dải PRED_BASE khi bb ≥ 1024 (6<<20 + 1024*1024 = 7<<20): nghĩa vụ hợp đồng không biểu diễn được ở khối bb=1024+B có thể trùng nguyên tử vị từ thuần (pi=B, local=idx) đã được giả  | FB-C26 |
| `gt-oblig/src/analysis.rs:1369` | nguyên tử `unrep_atom` = UNREP_BASE + bb*1024 + idx tràn sang dải PRED_BASE khi bb ≥ 1024 (6<<20 + 1024*1024 = 7<<20): nghĩa vụ hợp đồng không biểu diễn được ở khối bb=1024+B có thể trùng nguyên tử vị từ thuần (pi=B, local=idx) đã được giả  | FB-C26 |
| `gt-oblig/src/analysis.rs:1510` | nguyên tử `unrep_atom` = UNREP_BASE + bb*1024 + idx tràn sang dải PRED_BASE khi bb ≥ 1024 (6<<20 + 1024*1024 = 7<<20): nghĩa vụ hợp đồng không biểu diễn được ở khối bb=1024+B có thể trùng nguyên tử vị từ thuần (pi=B, local=idx) đã được giả  | FB-C26 |
| `gt-oblig/src/driver.rs:610` | trait_member đúng ở mức này (thử /tmp/fo/t/a3.gt: ⊳ của thành viên impl KHÔNG làm giả thiết, giữ kiểm div_by_zero). NHƯNG cơ chế quanh nó lỗ: ⊳ chỉ được kiểm/chứng minh ở `Callee::Fn` (analysis.rs:1626 judge_pres), còn `Callee::TraitMethod` | FB-C25 |
| `gt-term/src/lib.rs:364` | instance_graph chỉ thu operand hàm-như-giá-trị từ Rvalue::Use/Cast/Aggregate, bỏ Rvalue::Repeat(op, n) nên `[f; 2]` không ghi nhận f bị thoát và lời gọi qua giá trị không có cạnh tới f ⇒ vòng đệ quy qua mảng-lặp bị bỏ sót, thiếu ↓/div vẫn q | FB-C15 |
| `gt-term/src/lib.rs:688` | Tập `escaped` (hàm bị dùng như giá trị) cũng chỉ duyệt Use/Cast/Aggregate, bỏ Repeat ⇒ cùng lỗ hổng: gọi giá trị không nối cạnh tới hàm trong `[f; n]` (cả đồ thị bảo thủ). | FB-C15 |
| `gt-types/src/check/expr.rs:406` | eq_ok trả true cho MỌI Ty::Infer(_) (kể cả biến suy luận tổng quát chưa giải) nên `==` trên biến kiểu chưa biết lọt, về sau biến được unify với f64 hoặc tuple ⇒ bỏ qua cấm `==` số thực (D4, E0303) và 'chưa hỗ trợ' cho tuple; chỉ biến Float  | FB-C19 |
| `gt-types/src/validate.rs:284` | Pattern trường `Ty::Param(_) => continue` ngay trên dòng này bỏ qua kiểm Copy cho trường kiểu tham số với giả định "impl tự khai báo T: Copy", nhưng không bắt buộc impl phải có ràng buộc đó: `⊐ <T> Copy for W<T> {}` hợp lệ ⇒ `W<Cap<Out>>`/` | FB-C16/C17 |

### 8.2 GHI NHẬN — 34 nhánh nuốt lỗi có chốt chặn phía sau (chưa sửa; nói thẳng). Hai nhánh `escaped_sites` (`driver.rs`: `_ =>` của `Rvalue`/`Terminator`) đã được làm CHẶT bằng liệt kê tường minh (không còn im lặng bỏ sót biến thể mới) nên không còn trong danh sách.

- `gt-codegen-llvm/src/lower.rs:1465` (_ =>): `_ => false` đúng cho Int/Ref/RawPtr/Fn/Str (sao chép tham chiếu không sao chép ô); Param/Infer/Err ⇒ true (đóng). Giới hạn: ở dòng 1381 `variant_fields(..).is_ok_and(..)` coi lỗi là 'không có ô' (mở) nhưng static không generic nên Err chỉ khi db hỏng; thử static có Atomic bên trong struct: bị từ chối đúng (exit 3 unsupported).
- `gt-codegen-llvm/src/lower.rs:2218` (_ =>): Cuối intr_of ⇒ None ⇒ unsupported (đóng). Giới hạn: định tuyến intrinsic bằng key.contains("Atomic"/"Cell"/"File"/"Arena") là khớp chuỗi con, chỉ áp cho hàm KHÔNG có thân MIR (prelude) nên người dùng không chen vào được; nên đổi sang so khoá đầy đủ.
- `gt-conc/src/lib.rs:199` (unwrap_or): thiếu tham số ordering mặc định thành SeqCst (hợp lệ cho mọi thao tác) nên sẽ không kiểm nếu chữ ký prelude trôi khỏi dạng (load 1, store 1, compare_exchange 2 ordering); đã kiểm prelude đúng và người dùng không thể thêm impl Atomic (E0311) nên chưa với tới
- `gt-driver/src/cli.rs:152` (is_some_and): Chỉ nhận đuôi `.gt` đúng chữ thường và `filter_map(Result::ok)` ở dòng 151 bỏ im lặng mục thư mục lỗi IO ⇒ `gritc check <thư mục>` có thể XANH mà bỏ sót tệp; hiếm, chưa tái hiện.
- `gt-driver/src/native.rs:335` (let _ =): Xoá .result cũ trước khi chạy; nếu xoá thất bại (quyền) mà chương trình chết trước khi ghi thì `value` có thể là của lần chạy trước. Chỉ ảnh hưởng giá trị báo cáo của run_exe (công cụ test/run), không phán quyết an toàn.
- `gt-driver/src/render.rs:200` (unwrap_or): Thiếu checks_report ⇒ JSON ghi kept/deleted/would_delete rỗng với mode all/proven (không phân biệt 'không có báo cáo' với 'không còn check'); chỉ là đầu ra báo cáo, không đổi phán quyết hay xoá check.
- `gt-driver/src/render.rs:238` (_ =>): RunEnd Limit/Internal/ProgramExit báo aborted=false, abort_kind rỗng; exit_status (71/2/100+s) vẫn đúng nên người đọc `aborted` đơn thuần sẽ coi Internal là không abort.
- `gt-guard/src/lib.rs:78` (_ =>): is_atomic chỉ gồm load/store atomic, atomicrmw, cmpxchg; lệnh fence không thuộc lớp E0503; fence không sinh UB/race dữ liệu và codegen không phát
- `gt-guard/src/lib.rs:334` (is_some_and): cấm FMA theo tiền tố tên llvm.fmuladd/llvm.fma nên sót biến thể như llvm.experimental.constrained.fma hay llvm.vp.fma, và 3b cho phép mọi llvm.*; chỉ ảnh hưởng bit số thực, codegen không phát
- `gt-mir/src/build/expr.rs:861` (_ =>): adt_args trả Vec rỗng nếu kiểu kết quả của lời gọi biến thể không phải Adt, trong khi nhánh as_rvalue tương ứng thì ICE; M4 luôn cho Adt nên không với tới, nên đổi thành ICE cho đồng nhất
- `gt-mir/src/build/expr.rs:1485` (_ =>): phương thức gọi dạng x.m() mà không có self_kind (hàm không có self) bị hạ như nhận theo giá trị, không có khoản mượn; typeck vẫn chấp nhận nhưng downstream từ chối bằng ICE E9901 ở borrowck (đã chạy, exit 2), nên fail-closed; nên báo lỗi rõ ở M4
- `gt-mir/src/interp.rs:1060` (unwrap_or): hi = i128::MAX khi max_val của u128 không vừa i128 ⇒ saturating u128 trong bộ chạy tham chiếu có thể lệch LLVM (V::Int dùng i128); giới hạn của interp ngoài TCB, không ảnh hưởng phán quyết an toàn.
- `gt-mir/src/types.rs:17` (_ =>): region_slots đếm 0 cho Ty::Fn/Param/Infer/Err (Fn 'không đi vào kiểu hàm' theo thiết kế ADR-019/021); an toàn chỉ chừng nào kiểu hàm không mang tham chiếu có region trong chữ ký đã được ADR bảo đảm — chưa kiểm chứng được ở đây.
- `gt-oblig/src/driver.rs:671` (unwrap_or): tails.get(..).unwrap_or(block) là mặc định đúng (chưa chèn lần nào). Giới hạn ngay sau đó: `let Some(orig)=terminator.take() else { return }` im lặng BỎ chèn kiểm khi khối không có terminator (không xảy ra với MIR đã validate; nên ICE).
- `gt-oblig/src/interval.rs:40` (unwrap_or): Ty::min: pow2(bits-1) None ⇒ ZERO sẽ THU HẸP khoảng (không sound) nhưng không tới được: bits ∈ {8,16,32,64,128} (IntTy::bits) hoặc ≤64 trong analysis; pow2 chỉ None khi n≥256. Nên đổi sang expect/ICE.
- `gt-oblig/src/interval.rs:46` (unwrap_or): Ty::period: pow2(bits) None ⇒ ZERO (chia cho 0 ⇒ div_trunc None ⇒ wrap trả toàn kiểu, an toàn); chỉ tới được khi bits≥256, không có.
- `gt-secrets/src/lib.rs:266` (is_some_and): ADT không có trong db ⇒ coi là không chứa Secret ⇒ bỏ cảnh báo S201/S202; db luôn đủ sau M4 và bộ quét chỉ là cảnh báo vàng (nên là ICE nếu thành kiểm tra an toàn)
- `gt-secrets/src/lib.rs:279` (_ =>): kiểu khác (Param/Fn/Str...) coi là không chứa Secret; taint không xuyên hàm/generic là giới hạn đã tuyên bố, bộ quét chỉ vàng
- `gt-secrets/src/lib.rs:324` (unwrap_or_default): chữ ký callee thiếu ⇒ không có hiệu ứng ⇒ không báo S202; chữ ký luôn có sau M4, bộ quét chỉ vàng
- `gt-secrets/src/lib.rs:333` (unwrap_or_default): thành viên trait thiếu ⇒ không hiệu ứng ⇒ không báo S202; luôn có sau M4, chỉ cảnh báo vàng
- `gt-secrets/src/lib.rs:337` (_ =>): gọi qua giá trị mà kiểu toán hạng không phải Fn/không xác định ⇒ không hiệu ứng ⇒ bỏ S202; chỉ cảnh báo vàng
- `gt-types/src/check/call.rs:20` (_ =>): đối số không phải Path tới hàm/local ⇒ không thay marker `poly`; hiệu ứng thật do gt-effects tính từ kiểu đối số; chưa thử đối số là biểu thức khác path (chưa hiểu hết tương tác unify marker)
- `gt-types/src/check/call.rs:23` (_ =>): như dòng 20: nhánh còn lại không có hiệu ứng để thay marker; chưa kiểm chứng tương tác với unify
- `gt-types/src/check/call.rs:111` (unwrap_or): const/static thiếu trong db.consts ⇒ Ty::Err không kèm chẩn đoán tại đây; chặn cuối là gt-mir validate (unresolved_types ⇒ ICE) nhưng chưa tái hiện nhánh này
- `gt-types/src/check/call.rs:243` (unwrap_or): `Self` không có self_ty ⇒ Err im lặng; resolver chặn `Self` ngoài impl và gt-mir validate chặn Err trong MIR; chưa tái hiện
- `gt-types/src/check/call.rs:440` (let _ =): callee có kiểu Err ⇒ không phát lỗi ở đây (lỗi gốc đã báo nơi sinh Err); nếu Err sinh im lặng thì chặn cuối là gt-mir validate unresolved_types
- `gt-types/src/check/mod.rs:438` (unwrap_or): tham số AST nhiều hơn sig.params ⇒ kiểu Ty::Err (độc, unify với mọi thứ ⇒ không còn kiểm kiểu cho tham số đó) mà không báo lỗi; chỉ khi lower chữ ký lệch (bất nhất nội bộ/ đã có lỗi lower); nên ICE.
- `gt-types/src/check/mod.rs:555` (_ =>): has_cap không đi vào trường của Adt người dùng (Adt chứa Cap ở trường không tính là có cap) và `lang(Cap)=None ⇒ false` (tắt E0310): hướng nguồn cap ⇒ chỉ nới nếu tạo được `W{c:Cap}` mà không có cap, nhưng dựng W cần Cap hợp lệ nên chưa tìm được kịch bản; giới hạn thiết kế (cap gốc chỉ qua root_*/main).
- `gt-types/src/check/mod.rs:940` (is_none_or): is_none_or: local thiếu LocalInfo ⇒ coi là mutable (cho gán). Chỉ xảy ra khi biến không được khai báo (vd pattern `Self{..}` trên kiểu không phải ADT, xem pat.rs:384) — khi đó MIR ICE chặn; các đường khai báo khác đều qua declare_local; nên đổi thành `is_some_and`/ICE.
- `gt-types/src/check/pat.rs:67` (_ =>): Binding không phải tên biến thể ⇒ khai báo biến mới và coi là Wild (đúng ngữ nghĩa); hệ quả: gõ nhầm tên biến thể (`Emtpy`) thành bắt-tất-cả, xanh và không cảnh báo (kiểu Rust cũng vậy); không gây mất an toàn bộ nhớ nhưng có thể che lỗi logic.
- `gt-types/src/check/pat.rs:107` (let _ =): bind_all bỏ XPat; các nơi gọi trả Wild KHÔNG báo lỗi riêng: variant_pat (parent/adt/biến thể không tìm thấy), Res::Err|None ở pat_inner. Wild = phủ mọi thứ ⇒ nếu tới được thì vét cạn bị nới; hiện chỉ tới được khi resolver đã lỗi (check_program trả sớm nếu resolve có chẩn đoán ⇒ nhánh chết) hoặc bất nhất nội bộ. Nên ICE thay vì Wild im lặng; chưa tìm được đầu vào tái hiện (p1.gt: tên lạ ⇒ E0201 ĐỎ).
- `gt-types/src/check/pat.rs:384` (unwrap_or): `Self {..}` với Self không phải ADT (vd `⊐ T for i32`) ⇒ unwrap_or(Err)/`let Ty::Adt = .. else return Wild` im lặng, không khai báo biến trường (dùng sau ⇒ Ty::Err không lỗi dây chuyền); check kiểu XANH nhưng hạ MIR ICE E9901 "pattern struct trên kiểu không phải ADT" ⇒ vẫn bị chặn (rc 2); đáng chuyển thành lỗi ngay ở M4.
- `gt-types/src/check/pat.rs:495` (let _ =): bind_fields chỉ gọi sau lỗi E0301/ unify, trừ nhánh `Res::Err|None` ở struct_pat trả Wild không lỗi: chỉ tới được khi resolver lỗi (đã chặn M4 ở check_program), tức nhánh chết; nên ICE.
- `gt-types/src/collect.rs:73` (_ =>): Tên #lang không nhận ra trong prelude bị bỏ qua im lặng (lang=None) thay vì ICE: gõ sai `secret`/`send`/`sync` ở lib/core làm mất kiểm tra tương ứng; chỉ prelude đặt được (người dùng bị E1002) và không có kiểm 'đủ mọi lang item'.

### 8.3 Cổng chống tái phát
`tools/fail_open_scan.py --baseline docs/fail_open_baseline.json` (khoá = tệp + mẫu + dòng mã chuẩn hoá, KHÔNG số dòng) chạy trong cổng (`tools/gate.sh`, đơn vị `failopen`): MỌI vị trí mới ở 11 crate quyết định phải được đọc, phân loại ở `docs/fail_open_classification.tsv` (công cụ: `tools/fail_open_sync.py`) và đưa vào baseline cùng commit, nếu không cổng ĐỎ. Giới hạn nói thẳng: quét theo mẫu chữ, KHÔNG thấy fail-open dạng khác (vd thiếu một nhánh phải có, thứ tự đánh giá như FB-C20, thiếu kiểm tra hoàn toàn như FB-C25); các dạng đó chỉ tìm được bằng đọc mã đối kháng (mục 9).

## 9. Đọc mã đối kháng bằng subagent (Seed 1.0 S7, 2026-10-22/23; Linux)
Năm nhóm TCB, mỗi nhóm một subagent MỚI (cùng họ model với tác giả mã; KHÔNG phải người độc lập) với lời nhắc `docs/adversarial/PROMPT.txt` (nguồn: `docs/E_PROMPT.md`): "tìm lỗi, không xác nhận"; mỗi nghi vấn phải có chương trình Grit tái hiện chạy bằng `gritc` thật. Báo cáo gốc và các chương trình tái hiện: `docs/adversarial/{A..E}/` (A: Arena/Vec/Text/lát/`&str`/File/`str::as_bytes`/`Vec::truncate`; B: capability, I/O, `root_*`/E0310; C: số học, ép kiểu, lát, `--checks=proven`; D: Atomic/Mutex/spawn/Cell/Send-Sync/`■`; E: IR Guard, attestation, bất biến codegen).

**Cách nói đúng về trạng thái TCB:** các intrinsic và bước codegen/Guard thuộc năm nhóm trên "**đã qua đọc mã đối kháng bằng subagent cùng họ model**" — KHÔNG phải "đã duyệt", KHÔNG phải kiểm toán độc lập, KHÔNG phải chứng minh. Mỗi nhóm có phần "không kịp đọc" ghi thẳng trong báo cáo gốc; những phần đó CHƯA được đọc đối kháng.

| nhóm | phát hiện THẬT | xử lý |
|---|---|---|
| A | `⊐ Copy for Arena {}` sao chép tay cầm Arena ⇒ dùng-sau-reset; `Cell::get` không đòi `T: Copy` nhân bản giá trị không-Copy | FB-C17 (cấm Copy cho Arena/Mutex/Atomic/Cell/Vec/Text/Secret/Cap; `Cell::get` đòi `T: Copy`); `⊐ <T> Copy for W<T>` thiếu `T: Copy` ⇒ FB-C16 |
| B | `main` có tham số ở tệp khác được miễn E0310 (điều kiện cứng #12); `has_cap` không đi vào trường struct/enum; `io_result` đặt `alloca` tại chỗ gọi (SIGSEGV sau ~10^7 vòng); ghi vào stdout đã đóng thành công im lặng; `check` xanh nhưng build đỏ cho vài hàm prelude | FB-C18 (hai lỗi đầu); FB-C23; còn lại ghi KNOWN_LIMITATIONS |
| C | kiểm biên lát/chỉ số dùng độ dài CŨ khi `s` bị gán lại trước lúc dùng (đọc/ghi ngoài mảng ở LLVM); thông dịch sai với 128 bit; `unrep_atom` có thể trùng nguyên tử vị từ khi hàm ≥ 1024 khối | FB-C20; ghi KNOWN_LIMITATIONS (thông dịch ngoài TCB; nhân/chia 128 bit chưa hạ ⇒ Unsupported); FB-C26 |
| D | thứ tự khởi tạo `■` chỉ theo phụ thuộc trực tiếp (gọi con trỏ hàm NULL từ chương trình xanh); ordering của Atomic là local bị `&!`; luồng `spawn` chỉ có ngăn xếp 2 MiB | FB-C21; FB-C22; ghi KNOWN_LIMITATIONS |
| E | Guard không đối chiếu số `Assert` biên MIR↔IR (chú thích `lower.rs` nói sai); attestation không phủ hồ sơ/runtime/LLVM; tên tệp `gritrt_*.gt` bị E1101; tài liệu `TCB_INTRINSICS §C` thiếu 5 hàm runtime; địa chỉ khai báo ngoài ≠ `gritrt_*` | chú thích sửa + KNOWN_LIMITATIONS; `TCB_INTRINSICS §C` liệt kê đủ 27; FB-C27 (luật địa chỉ cho mọi khai báo ngoài) |

Kết hợp với mục 8 (quét fail-open), đợt này tìm thêm FB-C15 (đệ quy qua `[f; N]`), FB-C19 (`==` trên biến suy diễn chưa giải), FB-C24 (`imut` ADT lạ), FB-C25 (`⊳` của thành viên trait qua tham số kiểu). **Hệ quả cho độ tin cậy:** 14 lỗ hổng/khoảng trống thật (FB-C15…C27, trừ phần ghi nhận) mà bộ test/oracle/đột biến trước đó KHÔNG bắt — đọc đối kháng có giá trị riêng và phải lặp lại sau thay đổi lớn; kết luận "không tìm thấy thêm" của một nhóm chỉ có nghĩa trong phạm vi nhóm đã đọc.

## G1.3 — đối soát các mẫu scanner sau hợp nhất (2026-11-12; Windows)
Bộ quét Seed 1.0 tìm thấy ba khóa mới so với baseline nhánh Claude; `fail_open_sync.py` tìm thêm mẫu cũ đã đổi/xuất hiện ở cây Windows. Đã đọc cả bốn vị trí hiện hành và thêm phân loại tương ứng trong `docs/fail_open_classification.tsv`:

| vị trí | kết luận hiện tại | căn cứ |
|---|---|---|
| `gt-mir/src/interp.rs:1012` | AN TOÀN | phép checked không hỗ trợ trả `Err(Flow::Bug)`; `run_env` ánh xạ thành `Outcome::Bug`, không thành giá trị thành công. Interpreter ở ngoài TCB. |
| `gt-mir/src/print.rs:65` | AN TOÀN | `writeln!` ghi vào `String` qua `fmt::Write`, chỉ tạo văn bản snapshot MIR; không phải I/O hay đường phán quyết. |
| `gt-secrets/src/lib.rs:339` | AN TOÀN, fail-closed | callee-value không tra được kiểu hàm dùng `unknown() = {io}`, làm S202 bảo thủ hơn thay vì bỏ hiệu ứng. |
| `gt-driver/src/cli.rs:155` | AN TOÀN | từng mục `read_dir` được `map_err` thành lỗi trước khi chỉ lọc phần mở rộng `.gt`; không có `filter_map(Result::ok)` bị bỏ im lặng ở đường hiện hành. |

Đây là rà mã theo luồng lỗi cụ thể; không thêm test hồi quy riêng cho bốn mẫu scanner này. Bộ test/gate hậu merge vẫn phải chạy; kết luận trên không thay thế cho kiểm thử đường lỗi đọc thư mục.