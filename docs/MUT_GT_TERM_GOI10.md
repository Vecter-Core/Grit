# MUT_GT_TERM_GOI10 — đột biến hàm quyết định của gt-term (GÓI 10, S3.b; 2026-11-20)

Phạm vi: `crates/gt-term/src/lib.rs` dòng 293–690 (`instance_graph`, `discover`, `conservative_graph`, `dyn_fn_arg`, `takes_fn_param`, `has_back_edge`: mã sửa ở FB-C32/33/34). Công cụ: `tools/mut.py` qua bọc `tools/mut_range.py` (lọc theo khoảng dòng). 1 worker; bản sao làm việc `D:/Grit_mut/w0`.
Bộ test dùng: `cargo test -p gt-term --test spawn_recursion --test instance_graph --test conservative_fallback --test termination --test fail_closed_loops -- --test-threads=2` (baseline xanh). `term_oracle` KHÔNG dùng được làm baseline (thoát mã 143 ở worker; CHƯA điều tra). Timeout mỗi đột biến 150 s (một đột biến `!= → ==` ở union-find treo ⇒ tính là bị bắt).

## Số liệu (từ `docs/MUT_GT_TERM_GOI10.jsonl`)
- 110 đột biến, **nguồn: 110 tự động, 0 tay**. Bị bắt 80, không hợp lệ (lỗi biên dịch) 3, **sống sót 27**.
- Chạy lại ca sống sót với bộ test rộng hơn (`m8_corpus`, `ghost_wide_oracle`, `limit_hint`, `ghost_limit_soundness`): DỪNG sớm vì chậm (≈ 80 s mỗi ca); chỉ có 6 kết quả (`docs/MUT_GT_TERM_GOI10_R2.jsonl`): các dòng 300/309/318 — 300 `→false`, 309 `>= → >` và `→false` vẫn sống sót; 300 `→true`, 309 `→true`, 318 `→true` bị bắt. Phần còn lại CHƯA chạy lại.

## GÓI 11 S1: cập nhật (2026-11-20)
- Test mới `crates/gt-term/tests/mutants_goi11.rs` (3 test; xanh trên mã gốc): cận đồ thị đúng từng nút (bộ đếm `conservative_fallbacks`), đệ quy qua từng loại cạnh bị ĐỎ, đối chứng không đỏ oan. Chạy lại ca sống sót (`tools/mut_range.py --only-survivors`, bộ test = mutants_goi11 + spawn_recursion + instance_graph + conservative_fallback + termination + fail_closed_loops): **còn 25 sống sót** (trước 27; nguồn: 100% tự động, 0 tay). Sổ: `docs/MUT_GT_TERM_GOI10_G11.jsonl`.
- ĐO bằng bộ đếm trong BẢN SAO ngoài kho (`D:/Grit_rev`, không đụng kho): chạy `gritc check` trên 974 đầu vào (tests/pass, fail, samples, adversarial, examples, apps, capstone) + 45 test của gt-term (`GT_TERM_PROGRAMS=150`): các sự kiện "làm đột biến khác bản gốc" **H318, H323t, H340, H361, H372, H385, H426, H431, H443, H495, H559, H576, H671, H683 xảy ra 0 lần**; H323f xảy ra ~18 100 lần nhưng số nút `TERMLEN` giống hệt khi bỏ cả vòng `t.instances` trên 437 chương trình (tests/pass + samples): vòng đó dư thừa với đồng đẳng này.
- **Hạn chế của phép đo:** "0 lần" nghĩa là KHÔNG thấy trong tập chương trình đã chạy, KHÔNG chứng minh nhánh không tới được. Nền: Windows, `gritc` dev trong bản sao.

## 25 ca sống sót hiện tại (phân loại; hướng: chấp-nhận-thêm / chặt-thêm / trung-tính)
| tệp:dòng | đột biến | phân loại | căn cứ |
|---|---|---|---|
| `crates/gt-term/src/lib.rs:318` | condition → false | tương đương (đo) | H318=0: mọi hàm không generic đã nằm trong `t.instances`; vòng gốc dư thừa |
| `crates/gt-term/src/lib.rs:323` | condition → false | tương đương (đo) | bỏ vòng `t.instances`: số nút TERMLEN giống hệt trên 437 chương trình (tests/pass + samples); H323t=0 (không instance thiếu thân) |
| `crates/gt-term/src/lib.rs:323` | condition → phủ định | tương đương (đo) | bỏ vòng `t.instances`: số nút TERMLEN giống hệt trên 437 chương trình (tests/pass + samples); H323t=0 (không instance thiếu thân) |
| `crates/gt-term/src/lib.rs:323` | condition → true | tương đương (đo) | bỏ vòng `t.instances`: số nút TERMLEN giống hệt trên 437 chương trình (tests/pass + samples); H323t=0 (không instance thiếu thân) |
| `crates/gt-term/src/lib.rs:340` | continue → break | nhánh chết (đo) | H340=0 |
| `crates/gt-term/src/lib.rs:361` | xoá câu lệnh | tương đương (đo) | H361=0: mọi hằng hàm dùng như giá trị đã có instance từ `t.instances` |
| `crates/gt-term/src/lib.rs:372` | continue → break | nhánh chết (đo) | H372=0: khối không có terminator |
| `crates/gt-term/src/lib.rs:385` | xoá câu lệnh | tương đương (đo) | H385=0 |
| `crates/gt-term/src/lib.rs:426` | condition → false | nhánh chết (đo) | H426or=0, H426xor=0: self_ty luôn cụ thể sau thế |
| `crates/gt-term/src/lib.rs:426` | || → && | nhánh chết (đo) | H426or=0, H426xor=0: self_ty luôn cụ thể sau thế |
| `crates/gt-term/src/lib.rs:431` | condition → true | nhánh chết (đo) | H431=0 |
| `crates/gt-term/src/lib.rs:433` | condition → true | tương đương sản xuất | chỉ khác khi chốt kiểm thử `set_drop_trait_edges_for_test` bật; CHƯA chạy test dùng chốt (gt-driver llvm_diff_gtv) |
| `crates/gt-term/src/lib.rs:443` | < → <= | nhánh chết (đo) | H443=0: vòng đệm không chạy |
| `crates/gt-term/src/lib.rs:445` | false → true | nhánh chết (đo) | cùng vòng đệm |
| `crates/gt-term/src/lib.rs:495` | continue → break | nhánh chết (đo) | H495=0 |
| `crates/gt-term/src/lib.rs:507` | condition → true | tương đương (toán) | ra==rb ⇒ parent[rb]=ra là phép gán lại gốc cho chính nó |
| `crates/gt-term/src/lib.rs:554` | false → true | chặt thêm |  |
| `crates/gt-term/src/lib.rs:559` | true → false | phòng thủ (đo) | H559=0; hướng chấp-nhận-thêm |
| `crates/gt-term/src/lib.rs:576` | true → false | phòng thủ (đo) | H576=0; hướng chấp-nhận-thêm |
| `crates/gt-term/src/lib.rs:615` | && → || | chặt thêm |  |
| `crates/gt-term/src/lib.rs:615` | condition → true | chặt thêm |  |
| `crates/gt-term/src/lib.rs:671` | true → false | phòng thủ (đo) | H671=0; hướng chấp-nhận-thêm |
| `crates/gt-term/src/lib.rs:675` | condition → true | chặt thêm |  |
| `crates/gt-term/src/lib.rs:683` | true → false | phòng thủ (đo) | H683=0; hướng chấp-nhận-thêm |
| `crates/gt-term/src/lib.rs:686` | false → true | chặt thêm |  |

**Tóm tắt:** chặt-thêm (fail-closed, không âm tính giả): 451, 554, 615 ×2, 675, 686. Tương đương/dư thừa có đo: 318, 323 ×3, 361, 385, 507, 433 (chỉ khi chốt kiểm thử bật). Nhánh chết/phòng thủ 0 lần: 340, 372, 426 ×2, 431, 443, 445, 495, 559, 576, 671, 683. **Chưa có con nào theo hướng chấp-nhận-thêm mà có chương trình thật làm gt-term chấp nhận chương trình không dừng**, nhưng 559, 576, 671, 683 (và các nhánh chết) KHÔNG có test, chỉ có đo 0 lần: ghi `docs/KNOWN_LIMITATIONS.md` ("false negative tiềm ẩn").
