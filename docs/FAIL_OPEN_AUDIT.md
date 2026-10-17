# FAIL_OPEN_AUDIT — rà soát "im lặng chấp nhận" ở các crate quyết định (2026-10-17)

KẾT QUẢ TÓM TẮT: 6 chỗ fail-open/nghi fail-open đã SỬA (G1, S1, B1, B2, B3, D1); phần còn lại AN TOÀN/GHI NHẬN theo đọc mã. Bảng "cách kiểm chứng" ở mục 0.

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

### 7.3 CHƯA RÀ (nói thẳng; số = số chỗ khớp danh sách tìm kiếm, ƯỚC LƯỢNG từ grep; "đọc" = đọc ngữ cảnh)
| tệp | chỗ khớp | trạng thái |
|---|---|---|
| gt-oblig/src/analysis.rs | 82 | CHƯA đọc (2 chỗ ở 7.2); bộ giải/đột biến M7 đã được kiểm theo cách khác (đột biến đầy đủ, FB-7) |
| gt-mir/src/build/expr.rs | 62 | đọc ~12; CHƯA ~50 |
| gt-codegen-llvm/src/lower.rs | 55 | CHƯA (chặn sau bởi Guard-pre/post) |
| gt-mir/src/interp.rs | 54 | CHƯA (thông dịch tham chiếu, không phải kiểm tra an toàn) |
| gt-types/src/collect.rs | 50 | đọc ~6; CHƯA ~44 |
| gt-oblig/src/env.rs | 32 | đọc 1; CHƯA 31 |
| gt-types/src/check/mod.rs | 31 | đọc ~8; CHƯA ~23 |
| gt-term/src/lib.rs | 29 | đọc ~6; CHƯA ~23 |
| gt-types/src/check/expr.rs | 28 | CHƯA đọc ngữ cảnh (chỉ liệt kê) |
| gt-types/src/check/call.rs | 28 | đọc ~8; CHƯA ~20 |
| gt-types/src/check/pat.rs | 26 | đọc ~8; CHƯA ~18 |
| gt-oblig (interval 20, zone 15, driver 14, unsafe_check 9, prover 5, int 3, ir 2) | 68 | CHƯA (driver: 3 đọc) |
| gt-mir/src (print 20, build/mod 13, matching 11, types 10, lib 8, validate 5) | 67 | đọc ~15; CHƯA ~52 |
| gt-parser (parser 19, exprs 13, types 3, lib 2, pats 1) | 38 | đọc ~10; CHƯA ~28 (parser fail-closed theo thiết kế: `Err(expected)`) |
| gt-types còn lại (ty 11, lib 11, validate 8, mono 7, infer 7, traits 5, region 1) | 50 | đọc ~6; CHƯA ~44 |
| gt-resolve (tables 16, walk 7, model 2, lib/graph 2) | 27 | đọc ~10; CHƯA ~17 |
| gt-lexer | 32 | đọc ~6; CHƯA ~26 |
| gt-fmt (printer 14, attach 2, lib 1), gt-diag (json 7, render 5, code 5), gt-span 15 | 49 | CHƯA (định dạng/hiển thị, không quyết định chấp nhận) |
| gt-dataflow (analyses 8, framework 1) | 9 | đọc 3; CHƯA 6 |
| gt-secrets, gt-borrowck, gt-guard, gt-driver, gt-conc, gt-effects, rig | — | đã rà ở đợt 1 hoặc 2 (xem trên); gt-borrowck/gt-driver/gt-secrets: chỉ các chỗ quyết định (30/40/~15) |

**Tổng ước lượng CHƯA đọc ngữ cảnh: khoảng 500 trong ~900 chỗ khớp (không tính gt-guard/gt-conc/gt-effects đã đọc hết).** Kết quả "AN TOÀN"/"GHI NHẬN" là kết quả ĐỌC MÃ, KHÔNG phải chứng minh; chỉ các mục SỬA có test. 6 trong 11 mục ở đợt 2 là lỗi THẬT đạt tới được từ Grit ⇒ phần CHƯA đọc có thể còn lỗi tương tự — đặc biệt các chỗ "hàm/ADT/kiểu dùng làm giá trị" (nay có bộ sinh riêng) và "thiếu ⇒ coi là ổn" ở gt-types/check.
