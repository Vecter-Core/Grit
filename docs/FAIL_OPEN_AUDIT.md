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
