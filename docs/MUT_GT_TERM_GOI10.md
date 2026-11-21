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

## GÓI 12 P3.b (2026-11-20, Windows; mọi kết quả nguồn **tự động**, 0 tay)
- **Phạm vi:** hàm mới `const_cycles` (FB-C36, `lib.rs` 706–834) — mã quyết định vừa sửa. 12 đột biến (`tools/mut_range.py`, `GRIT_MUT_RANGE=706-834`, 1 worker): vòng 1 **6 bị bắt, 5 sống sót, 1 không hợp lệ** (`docs/MUT_GT_TERM_G12.jsonl`, 12 dòng đầu).
- **Hành động:** 5 ca sống sót chỉ ra khoảng trống test THẬT (không phải tương đương): hằng là toán hạng PHẢI của phép toán (761), là ĐỐI SỐ lời gọi (773), vòng qua phương thức trait (736), cạnh `Const::Item` ở `Aggregate`/toán hạng thứ hai (760). Thêm 4 test vào `crates/gt-term/tests/const_cycle.rs` (12 test, xanh). Vòng 2 (6 dòng cuối của jsonl, `--only-survivors`): **4 bị bắt, 2 sống sót**.
- **2 ca còn sống (phân loại):** `lib.rs:721` `if consts.is_empty()` → false: **tương đương** (thoát sớm; kết quả rỗng giống hệt — suy từ đọc mã). `lib.rs:779` `dyn_fn_arg && takes_fn_param` → `||`: nhánh intrinsic không thân nhận đối số hàm động TRONG khởi tạo `□` — cần lời gọi có hiệu ứng trong initializer hằng (chưa dựng được chương trình hợp lệ) ⇒ **CHƯA CÓ TEST, hướng chấp-nhận-thêm**; ghi KNOWN_LIMITATIONS. Giống nhánh `dyn_fn_arg` của `conservative_graph` đã sống sót từ GÓI 10.
- **25 ca sống sót cũ (293–690):** KHÔNG chạy lại trong GÓI 12 (mã dòng đó không đổi; test mới `const_cycle` không chạm). Trạng thái: 25 sống sót như bảng trên; hướng chấp-nhận-thêm vẫn là các dòng 559, 576, 671 (đo 0 lần). **Mục tiêu "0 ca chấp-nhận-thêm không có test/lập luận" CHƯA ĐẠT** (3 nhánh phòng thủ + 779 mới).
- **Dữ liệu phải hỏng để tới các nhánh phòng thủ (P3.b):** 559 (`may_hold_fn` nhánh `Ref/RawPtr` của kiểu hàm): cần tham số kiểu `&fn` truyền vào intrinsic không thân — người dùng không viết được `&fn` làm đối số `spawn` ở Phase 1 (suy từ chữ ký prelude; CHƯA KIỂM bằng chương trình); 576 (`dyn_fn_arg` `true`): cần local hoặc đối số có kiểu có thể chứa hàm nhưng KHÔNG là `Const::FnItem` — chỉ có khi hàm đi qua generic/`id(f)` (FB-C34); đã có test `spawn_recursion`, nhưng đột biến `true → false` ở đúng nhánh `None` vẫn sống: dữ liệu `Place` có phép chiếu (trường/chỉ số) chứa hàm — hạn chế đã biết (hàng 17); 671 (`has_back_edge`): cần khối đích nằm ngoài `blocks` — MIR hợp lệ không có (validate.rs); các dòng `H…=0` còn lại ở bảng trên ghi sẵn dữ liệu cần hỏng. Những lập luận này là GIẢ THUYẾT: chưa đọc lại `validate.rs` cho 671 và chưa dựng chương trình cho 559/576; KHÔNG có bằng chứng ⇒ **CHƯA GIẢI THÍCH** (KNOWN_LIMITATIONS).

## GÓI 13 B1 (2026-11-21, Windows; nguồn 100% tự động; `docs/MUT_GT_TERM_G13.jsonl`)
- **Chạy lại ĐẦY ĐỦ** 110 đột biến (293–705, 1 worker, 5 lô, `--resume`, bộ test = const_cycle + contract_recursion + mutants_goi11 + termination + instance_graph + conservative_fallback + fail_closed_loops + spawn_recursion): **83 bị bắt, 25 sống sót, 2 không hợp lệ**. (Số dòng của bảng GÓI 10/11 lệch ≈ 8 dòng so với mã hiện tại; bảng dưới dùng số dòng HIỆN TẠI.) `tools/mut.py` nguồn: 110 tự động, 0 tay.
- **Test mới (tests/ của gt-term; XANH trên mã gốc, ĐỎ trên đột biến — kiểm bằng `mut_range`):** `survivors_g13.rs` (vòng chỉ có `⊠` không cần `↓`: giết `has_back_edge` 691 `condition → true` và 702 `false → true`), `survivors_g13_hook.rs` (chốt `set_drop_trait_edges_for_test` đổi phán quyết: giết 440 `condition → true`), `survivors_g13_cons.rs` (đường bảo thủ ép buộc, hàm bị thoát gọi intrinsic không-hàm: giết 630 `condition → true`). **Giết 4 con ⇒ còn 21 sống sót.**
- **ĐO bằng bộ đếm trong BẢN SAO ngoài kho (`D:/Grit_cnt2`, đếm lần chạy đúng/sai tại từng dòng): suite gt-term (biến thể nhanh `GT_TERM_PROGRAMS=150 GT_TERM_RUNS=3`) + 996 chương trình `gritc check` (tests/pass, fail, warn, adversarial, samples, examples, apps, capstone):**
```text
dòng  suite đúng/sai   corpus đúng/sai   phân loại (hướng)
319   7069/260            7825/424
324   25165/0              25930/0
341   0/0              0/0
368   48402/0              29008/0
379   0/0              0/0
392   19754/0              19455/0
433   0/25             0/18
438   24/0              18/0
440   25/0              18/0
451   0/1358           0/1774
453   0/0              0/0
503   0/0              0/0
515   328/2              15/1
562   173/0              288/0
567   0/0              0/0
584   0/0              0/0
630   4/0              0/0
687   0/0              0/0
691   4094/11005          94/541
699   0/0              0/0
702   2/0              2/0
```
| dòng | đột biến | hướng | kết luận |
|---|---|---|---|
| 567, 584, 687, 699 | `None => true`/`return true` → `false` (hàm phòng thủ) | **chấp nhận thêm** | **0 lần** trên suite + 996 chương trình; bất biến được KIỂM trước khi tới gt-term: `gt-mir/src/validate.rs:311,380` (local phải tồn tại), `:366` (mọi khối có terminator), `gt-types/src/collect.rs:1176` (mọi hàm có chữ ký trong `db.fns`). Nhánh chết theo cấu tạo ⇒ không có chương trình tới được; KHÔNG có test (không dựng được). Ghi KNOWN_LIMITATIONS "false negative tiềm ẩn" kèm dòng |
| 341, 379, 453, 503, 451 | `continue → break`, `false → true`, `< → <=` | chấp nhận thêm/tương đương | **0 lần** (vòng/nhánh không bao giờ chạy); cùng lập luận bất biến (nút luôn có thân; `edges` luôn đủ dài) |
| 433 (×2), 438 | `\|\| → &&`, `→ false`, `→ true` | tương đương | điều kiện luôn sai (433: 0 lần đúng) / luôn đúng (438: 0 lần sai) trên mọi đầu vào đo |
| 319, 324 (×3), 368, 392 | bỏ thêm gốc/instance, bỏ `ops.push` | chấp nhận thêm | lập luận CÓ ĐO (GÓI 11): số nút đồ thị giống hệt trên 437 chương trình; mọi hàm đã có instance từ `t.instances` và cạnh gọi; 324: nhánh sai 0 lần. Nhóm này dựa vào ĐO, không có chứng minh |
| 515 | `if ra != rb` → luôn đúng | tương đương | `parent[rb]=ra` khi `ra==rb` là gán lại gốc cho chính nó (toán) |
| 562, 630 (`&&→\|\|`) | thêm cạnh gián tiếp | **chặt thêm** | mutant từ chối nhiều hơn; không làm mất an toàn; chưa có test (cần chương trình `spawn` có hiệu ứng) |
- **Mục tiêu "0 con theo hướng chấp-nhận-thêm mà không có test hay lập luận CÓ ĐO": ĐẠT ở mức ĐO** (mọi con chấp-nhận-thêm còn lại có 0 lần chạy trên 996 chương trình + suite, kèm bất biến được kiểm). Hạn chế: "0 lần" không chứng minh không tới được; dòng 319/324/368/392 chỉ dựa vào so sánh số nút. Dòng 559/576/671 của bảng cũ nay là 567/584/699 (+687): đã có lập luận trên.


## GÓI 14 D1 (2026-11-21, Windows; nguồn 100% tự động; `docs/MUT_GT_TERM_G14.jsonl`, số dòng của `lib.rs` sau GÓI 14 B2)
Chạy lại các con còn sống bằng `mut_range --only-survivors docs/MUT_GT_TERM_G13.jsonl` với `GRIT_MUT_LINES` (1 worker; bộ test = bộ GÓI 13 + `survivors_g14`, `survivors_g14_fallback`). Test mới: `survivors_g14.rs` (7 test: hàm đệ quy KHÔNG được gọi vẫn bị E0703; đệ quy generic qua giá trị hàm gán biến/đối số; generic chỉ có mặt như giá trị), `survivors_g14_fallback.rs` (`spawn(w, 5u64)` không lùi về đồ thị bảo thủ). Mọi test XANH trên mã gốc.
| dòng (lib.rs) | đột biến | kết quả GÓI 14 | ghi chú |
|---|---|---|---|
| 319 | `if fn_params(b.node).is_empty()` → `false` | SỐNG | **TƯƠNG ĐƯƠNG (ĐO)**: vòng gieo gốc dư thừa — `t.instances` đã chứa mọi hàm không generic; hàm đệ quy KHÔNG được gọi vẫn bị bắt khi tắt dòng này |
| 319 | → `true` | bị bắt | |
| 324 | `if by_node.contains_key(&i.node)` → `true`/`false`/phủ định (3 con) | SỐNG | chưa giết sau 2 lần (GÓI 13: chỉ so số nút đồ thị; GÓI 14: không dựng được chương trình mà phán quyết phụ thuộc — instance không thân chỉ chiếm chỗ trong giới hạn đồ thị) |
| 368 | xoá `ops.push(o)` (phát hiện instance của hằng hàm trong `Use`/`Cast`/`Repeat`) | SỐNG | 4 chương trình (đệ quy generic qua `h ≔ g`, qua `ap(g, …)`, generic chỉ xuất hiện như giá trị) đều vẫn bị bắt nhờ đường khác (tập `escaped`/giá trị hàm) ⇒ chưa có chương trình phụ thuộc đúng dòng này |
| 400 (cũ 392) | xoá `ops.extend(cargs.iter())` | SỐNG | như 368 |
| 570 (cũ 562) | `Operand::Const(_) => false` → `true` | **bị bắt** | `survivors_g14_fallback` |
| 638 (cũ 630) | `if dyn_arg && takes_fn_param(..)` → `true` / `false` | bị bắt (cả hai) | bộ test hiện hành (chưa tách test nào) |
| 638 | `&&` → `||` | SỐNG | cần `dyn_arg` đúng mà `takes_fn_param` sai (intrinsic không có tham số hàm nhận giá trị hàm) — chưa dựng được |
Kết quả: 11 đột biến chạy (1 `invalid` do Defender chặn exe, chạy lại ⇒ SỐNG): **4 bị bắt, 7 sống**. Trong số 21 con còn sống sau GÓI 13, **2 con được giết** (570; 638 `condition → true`) ⇒ **≈ 19 còn sống** (chưa chạy lại toàn bộ 110; số 19 là SUY RA từ danh sách con mục tiêu). Con không giết được sau 2 lần (324×3, 368, 400, 638 `&&→||`, và 319 `→false` tương đương) ghi ở `KNOWN_LIMITATIONS.md` mục "false negative tiềm ẩn". Dòng 562 ở D1(ii): ĐÃ có test.
