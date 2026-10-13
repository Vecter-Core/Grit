# KNOWN LIMITATIONS (cập nhật theo milestone)

## Sau M0
- Chưa có compiler: `docs/spec/` là đặc tả M0 FINAL (Q1–Q14 đã được Giang duyệt) nhưng chưa được compiler nào kiểm chứng.
- Token đo bằng proxy (tiktoken), không phải tokenizer Claude (ADR-002).
- `tools/check_expectations.py` chỉ kiểm tính nhất quán của corpus (header, tên file, mã lỗi, Unicode, `.tokens`); **chưa** chạy gritc. Các mẫu **chưa được parse bởi công cụ nào**, nên có thể lệch grammar ở chỗ nhỏ — lỗi lộ ở M2, khi đó phải sửa mẫu hoặc grammar và ghi lại.
- Đã chấp nhận hệ quả của Q5: mọi `μ` là token `KW_MUT` nên tên biến không được chứa `μ` ở bất kỳ vị trí nào (kể cả `aμb`, `xμ`); `aμb` lex thành `IDENT MUT IDENT` và bị parser từ chối.
- Tên token đã chốt chính thức ở M1 (ADR-010); `tests/lexer/*.tokens` đã cập nhật.
- Một số mẫu fail dựa vào hàm chưa định nghĩa trong core (`root_fs`, `raw_read`, `Arena::new`) vì lib/core viết ở M12; mẫu chỉ nhằm minh hoạ mã lỗi, cần rà lại ở milestone sở hữu mã đó.
- Chưa có mẫu đa file (E0204 vòng phụ thuộc module) — thuộc M3.
- Grammar M0 đã đóng hết Q1–Q14; không còn điểm mở. Tên token đã chốt (ADR-010).
- `'a'` vs `'a` (semantics §15), `*` prefix/infix (§14), literal/comment (§19) mới là đặc tả + mẫu; chưa có lexer/parser kiểm.

## Hạn chế chủ ý của Phase 1 (không phải thiếu sót)
- **Toán tử bit không có cú pháp trực tiếp ở Phase 1, dùng hàm trong `lib/core`:** `band`, `bor`, `bxor`, `bnot`, `shl`, `shr` (Q9). Không chặn M1–M11; hàm viết ở M12. Do đó `|`, `^`, `~` đứng riêng là E0101.
- **Không có toán tử gán kết hợp** (`+=`, `-=`, `*=`, …): chủ ý, vì `x ← x + 1` đã đủ và nhất quán với triết lý không dư thừa. `x += 1` là E0101.
- **Comment nhiều dòng `/* */` bị CẤM** (E0101 khi gặp `/*` hoặc `*/`); chỉ có `//` và `///`. Dành riêng cho Phase 2 nếu cần. Hệ quả: `a /*p` phải viết `a / *p`.
- **`■` (static) chỉ bất biến** (D11); không có `■ μ`; static chứa kiểu đồng bộ chờ M8.
- **Ràng buộc trait chỉ inline** `<T: A + B>`; không có where-clause (`\where` dành riêng cho Phase 2).
- **Dành riêng, chưa có nghĩa:** `\extern` (FFI, D14), `\where`. Mọi chỗ dùng → E0101.
- **Phase 2 hoặc chưa có:** closure (và `|x|`), hàm biến đổi tham số (variadic), `cfg`/biên dịch có điều kiện, nội suy chuỗi, chuỗi nhiều dòng,
  raw string, macro, `dyn`/trait object. Chưa cần từ khoá dành riêng thêm: các cú pháp này hiện không đụng token nào (`|` chưa dùng; `#cfg` đi qua cú pháp thuộc tính).
- **Con trỏ thô `*□ T` / `*μ T`** chỉ hợp lệ trong khối `⚠`; ngoài `⚠` → E1001 (trừ intrinsic trusted trong `lib/core`). Phase 1 gần như mọi `⚠` của người dùng bị ĐỎ (D21).
- **Range mở trong biểu thức độc lập (không phải index/pattern) chưa có ngữ nghĩa — chờ iterator Phase 2.** `a..`, `..b`, `..` chỉ dùng được làm chỉ số slice và trong pattern; đứng độc lập bị từ chối (E0301). Range đóng `a..b`/`a..=b` làm giá trị độc lập chỉ dùng làm đối tượng của `⟳ x in …`.
- **Pattern** chưa có: or-pattern, guard, `μ` trong pattern, destructure ở `let`. Literal struct chưa có `..base` và viết tắt `{ x, y }`.
- **Iterator tổng quát chưa có:** `⟳ x in …` chỉ nhận range đóng và slice/array (qua `&`); iterable khác → E0701 cho tới M12/Phase 2.

## Lexer (M1)
- **Homoglyph chưa phủ hết:** chỉ phát hiện (a) trộn hai bảng chữ trong một định danh/lifetime (E1203) và (b) `µ` (U+00B5) giả `μ`. **Không** phát hiện định danh thuần một bảng chữ nhưng trông như tên Latin (vd `х` Cyrillic thay `x`; `аррӏе` toàn Cyrillic) — cần bảng confusables UTS #39; chưa làm. Rủi ro: hai định danh khác nhau nhưng trông giống nhau.
- Quy tắc trộn bảng chữ cũng từ chối cả tiếng Nhật trộn Han + Hiragana/Katakana trong một định danh (fail-closed).
- Danh sách ký tự "vô hình" là danh sách hữu hạn (bidi, zero-width, định dạng, tag, variation selector…), không phải mọi ký tự Cf; ký tự mới của Unicode có thể lọt cho tới khi cập nhật bảng.
- `x<-1` lex thành `x <- 1` (gán) theo khớp-dài-nhất; muốn so sánh với số âm phải viết `x < -1`.
- Văn bản `STR_LIT`/`CHAR_LIT` trong token là văn bản nguồn nguyên văn (chưa giải mã escape); giải mã/kiểm giá trị ở tầng sau.
- Literal số chưa kiểm phạm vi (`128i8` → E0601 ở M7). `ƒ` và `μ` đều là ký hiệu dành riêng nên không có định danh nào chứa `ƒ` hay `μ`.
- Fuzz lexer: 6 phiên × 10 phút sạch; chưa có phiên liên tục ≥1 giờ (ASan Windows) và chưa đạt ≥24 giờ tích luỹ của cổng chuyển giai đoạn.
- **leak_check** (`crates/gt-lexer/tests/leak_check.rs`) đo bộ nhớ qua tiến trình con/`/proc` vì crate cấm `unsafe` (không gọi được `GetProcessMemoryInfo`): Windows dùng PowerShell (`PrivateMemorySize64`), Linux `/proc/self/status` (VmRSS), macOS `ps`. Môi trường khác, hoặc máy không có PowerShell/`ps`, **không đo được → test FAIL** (fail-closed). Không có phương án đếm allocation vì cần `unsafe impl GlobalAlloc`. Test chạy ~12–25 s (1 triệu lần lex) và không bị `#[ignore]`.
- **Fuzz trên Windows:** ASan hết bộ nhớ nội bộ khi quarantine mặc định 256 MB (đã gặp ở lần chạy 1 giờ). `tools/fuzz_lexer.sh` đặt `ASAN_OPTIONS=quarantine_size_mb=8` và chạy nhiều phiên ngắn, giữ corpus. Cần nightly, `cargo-fuzz`, và `clang_rt.asan_dynamic-x86_64.dll` trong PATH.

## gt-fmt và comment (ADR-012) — quy tắc gắn comment, ghi trước khi viết gt-fmt
- Comment `//` nằm trong bảng trivia của lexer, **không** nằm trong AST. Từ Q16b (ADR-016 v2) gt-fmt gắn **theo token gần nhất**, không theo node: comment dòng riêng → token KẾ TIẾP; comment cuối dòng → token LIỀN TRƯỚC; E1301 chỉ khi bước xác minh thất bại. (Quy tắc theo node 1–4 của v1 đã bị thay thế.)
- **Bất biến kiểm bằng test khi viết gt-fmt:** `fmt(x)` giữ nguyên **số comment và nội dung từng comment**; `fmt(fmt(x)) == fmt(x)`.
- Dòng trống giữa các node không nằm trong AST nên gt-fmt chưa bảo toàn (sẽ chuẩn hoá về tối đa một dòng trống); ghi nhận để Giang xem lại.
- `///` (doc comment) là token `DOC_COMMENT` gắn vào node AST nên được gt-fmt giữ theo node.
- gt-fmt ở M2 là **thư viện** (`gt_fmt::format`); chưa có lệnh `gritc fmt` — binary `gritc` (gt-driver) chỉ có từ M9, và chỉ ở đó gt-fmt mới được nối thành lệnh.

## gt-ast (ADR-013)
- **`NodeId` không ổn định qua chỉnh sửa:** ổn định cho cùng một nguồn (cấp tuần tự tất định), nhưng sửa nguồn rồi parse lại sẽ cấp lại. LSP tăng dần (M13) phải ánh xạ lại bảng kết quả theo span, không giữ NodeId cũ.
- **`NodeId` chỉ duy nhất trong một tệp:** M3 (nhiều tệp/module) cần khoá `(FileId, NodeId)`; `gt-span` chưa có `FileId`.
- Ngoặc nhóm không vào AST; số literal giữ văn bản nguyên (giá trị/phạm vi kiểm ở tầng sau).

## gt-fmt: dòng trống (Giang duyệt 2026-10-05)
- Grit muốn **đúng một dạng chuẩn**: gt-fmt tự quy định dòng trống — **một dòng trống giữa hai item cấp cao nhất liền kề, không dòng trống thừa** (không dòng trống đầu/cuối tệp, không hai dòng trống liên tiếp). Dòng trống người dùng viết không được bảo toàn (không nằm trong AST).

## gt-parser (ADR-014)
- **Mọi pass đệ quy trên AST PHẢI chạy qua `gt_stack::run_with_big_stack`** (ADR-015; `MAX_DEPTH` chỉ bảo đảm cây không sâu hơn 256, không bảo đảm stack của luồng gọi). Types/borrowck/effects/… (M4+) chưa tồn tại nên chưa có gì để vi phạm; mỗi pass mới phải có test độ sâu tương tự `depth_law`.
- Khôi phục lỗi chỉ ở mức câu lệnh / item / nhánh match: lỗi trong danh sách tham số, đối số, trường vẫn làm mất cả câu lệnh/item chứa nó (một lỗi, bỏ cả câu lệnh).
- Sau lỗi `E0102` (quá sâu) parser dừng hẳn: không báo thêm lỗi nào ở phần còn lại của tệp.
- Biểu thức phẳng quá 256 toán hạng/postfix bị từ chối (`E0102`) — hệ quả của Q15e.
- AST không có node lỗi: chỉ dùng AST khi không có ĐỎ (fail-closed).
- Parser đang theo các đề xuất Q15a/b/c/h/i chưa được Giang duyệt (OPEN_QUESTIONS); nếu bị bác, phải sửa parser và các mẫu liên quan.
- Số mức lồng tối đa theo từng loại khác nhau (127 cho khối `{ }`/`⎇`/`⟳`/`⚠`, 253–255 cho biểu thức/kiểu/pattern) vì mỗi loại tốn độ sâu khác nhau cho mỗi mức; giới hạn chung vẫn là 256.

## gt-fmt: giới hạn đã biết (M2, ADR-016)
- **Comment được đặt theo token (v2); hệ quả bố cục:** comment cuối dòng ép xuống dòng nên một danh sách tham số/đối số ngắn bị ngắt tại comment (không tự chuyển thành mỗi phần tử một dòng); nhiều lần ép dòng liên tiếp tạo bậc thang thụt +2; comment đứng trước dấu phẩy nguồn đẩy dấu phẩy xuống dòng riêng. Hợp lệ và idempotent nhưng không đẹp; cải thiện bố cục (vd tham số mỗi dòng khi có comment) là việc có chủ đích sau này, cần ADR.
- **E1301 chỉ khi xác minh thất bại** (phương án cuối). Hiện chưa có đầu vào hợp lệ nào kích hoạt (bài đo chèn comment sau từng token của 33 file corpus: 0 từ chối); nếu gặp là lỗi của gt-fmt.
- **Một dạng chuẩn duy nhất, không bảo toàn bố cục người viết:** xuống dòng/thụt lề/khoảng trắng bị chuẩn hoá; ngoặc thừa bị bỏ, ngoặc cần thiết được thêm (theo độ ưu tiên của parser); `INLINE_MAX = 60` ký tự quyết định khối một dòng hay nhiều dòng; không ngắt dòng giữa các toán tử của một biểu thức dài (chỉ khối/match/danh sách mới nhiều dòng).
- **Từ chối khi nguồn có bất kỳ chẩn đoán nào của parser** (fail-closed): gt-fmt không định dạng tệp hỏng và không có chế độ "cố gắng".
- **`x as T < y`:** quy tắc NGÔN NGỮ (như Rust, `semantics.md`), không phải workaround của fmt: `T <` mở tham số kiểu; parser báo lỗi kèm gợi ý máy áp dụng được bọc `(x as T)`; fmt in `(x as T) < y`.
- **gt-fmt chỉ là thư viện** (`gt_fmt::format`); lệnh `gritc fmt` có từ M9 (gt-driver).

## gt-resolve (M3, ADR-017)
- **Chưa kiểm ở M3 (đẩy sang M4+, KHÔNG có nghĩa là đã đúng):** `Kiểu::tên` với `tên` không phải biến thể (mục kết hợp trong `⊐`/`⊏`; M3 chỉ ghi `Res::Assoc`), tên trường và truy cập trường private, phương thức, struct literal có trường lạ/thiếu, tên hiệu ứng (`io`, `lock(L)`), attribute, lifetime `'a`, kiểu của tên (hàm dùng ở vị trí kiểu…). Hệ quả: một chương trình dùng `E::tenSai` hay `x.truongSai` qua được M3 và PHẢI bị M4 chặn.
- **Pattern chỉ có ident**: gõ nhầm tên biến thể (`Nonee`) lặng lẽ thành binding mới; bắt phần lớn bởi match-vét-cạn (M5). Biến thể là tên trong không gian tên của module chứa enum (OPEN_QUESTIONS Q19): hai enum cùng module không được có biến thể trùng tên (E0202).
- **Tên module-tệp thấy được ở mọi tệp** và bị D7 bảo vệ (biến cục bộ không được trùng tên một tệp); module `▦` lồng không thấy tên của module cha (phải dùng đường dẫn tuyệt đối `tệp::…`). Không có `super`/`self::`/đổi tên khi nhập/glob.
- **E0204** theo định nghĩa cạnh của ADR-017 mục 6: cha→con và con→cha cùng tồn tại không phải vòng; vòng giữa tệp/anh em/lên cấp mới bị chặn.
- **Độc lập thứ tự (tính chất bắt buộc của plan):** xáo thứ tự TỆP cho kết quả giống từng byte; xáo thứ tự ITEM trong tệp giữ nguyên dạng chuẩn và tập chẩn đoán logic nhưng offset (và thứ tự D19 theo offset) đổi theo văn bản — không thể khác (D19 sắp theo offset).
- **Prelude là đầu vào:** fixture trong test; lib/core thật ở M12. Các hàm corpus cần (`root_fs`, `raw_read`, `core::abort`, kiểu `Cap/Fs/Secret/Arena`) hiện chỉ có trong fixture của test, không có trong sản phẩm.
- **`result` trong `⊲`** là đề xuất chưa duyệt (Q18).
- **Thông báo lỗi:** dòng nguồn rất dài (vd mẫu E0102_too_deep_*) bị vẽ nguyên dòng trong dạng người đọc (chưa cắt/ghi chú cột). Một số thông báo parser còn thô: `self ≔ …` báo "mong đợi `;` hoặc `}`" thay vì nói `self` là từ khoá; struct literal trần trong hợp đồng báo "mong đợi kiểu, gặp số". Snapshot khoá hiện trạng để mọi cải thiện là thay đổi có chủ đích.
- **Hạ tầng test (Windows):** build/test/fuzz có thể hỏng ngẫu nhiên khi máy hết commit bộ nhớ (os error 1455, `only metadata stub found`, ASan `error code: 1455`) — không phải lỗi mã; chạy lại với `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0` hoặc khi máy rảnh. Các artifact fuzz do nguyên nhân này đã được kiểm lại (chạy lại sạch) và giữ ở `crates/gt-parser/tests/fuzz_regressions/`.

## gt-types (M4, ADR-018)
- **Literal ngoài phạm vi kiểu (`x: u8 ≔ 300`) QUA M4** — theo spec đó là E0601 của M7 (ADR-018 mục 6); tới khi M7 xong chưa có gì chặn. Literal vượt `u128` thì E0301.
- **Thiếu đường trả về** (`ƒ f() → i32 { x; }`, `⎇` không `⇝` ở đuôi) QUA M4 — E0307 thuộc M5. Hàm `→ !`: M4 chỉ yêu cầu thân có kiểu `!`; "mọi đường đều phân kỳ" do M5.
- **Chưa bắt buộc "trait phải thấy được trong phạm vi"** để gọi phương thức: mọi `⊐` đều thấy. **Generics của trait** (`⊏ T<U>`) chưa dùng được; không kiểu liên kết, không `dyn`, không specialization.
- **Thân hàm trong prelude KHÔNG được kiểm** (tin chữ ký; prelude là mã tin cậy, lib/core thật ở M12). `#lang` chỉ nhận option/copy/range/secret/cap.
- **Hiệu ứng:** M4 ghi hiệu ứng vào chữ ký (kiểu hàm so sánh cả tập hiệu ứng) nhưng KHÔNG kiểm gọi hàm có hiệu ứng trong hàm thuần — việc của M5.
- **Cú pháp:** `∈` cần miền là range có ngoặc (`x ∈ (0..10)`); không có pattern `..=b` (chỉ `a..=b`, `a..`, `..b`); `⌥` trên `&T` là lỗi kiểu ở Phase 1 (không auto-deref scrutinee).
- **Mã chết sau nhánh bắt-tất-cả** không phải lỗi M4 (ghi chú N của M5).
- **Instance:** bảng instance lazy chỉ từ các gốc không generic; hàm generic không được gọi từ gốc nào không có instance (vẫn được kiểm một lần, ở dạng đa hình). Ngưỡng 64 → E0312.
- **Đột biến:** KHÔNG đột biến toàn bộ gt-types (chính sách 2026-10-06); chỉ ~48 đột biến có mục tiêu cho D1/D4/D5/`!` (kết quả ở PROGRESS.md).

## ĐIỀU KIỆN CỨNG: lỗ hổng tạm thời của M4/M5 (Giang, 2026-10-06; cũng ở GRIT_PLAN.md mục 7, điểm 9)
- Trước M7, literal ngoài phạm vi (`x: u8 ≔ 300`) lọt qua M4; E0307 (thiếu đường trả về) phụ thuộc M5.
- **M9 không được coi là xong, và `gritc` KHÔNG ĐƯỢC sinh binary, nếu E0601 (M7) chưa được triển khai và có test; tương tự E0307 (M5).** Cổng này phải được kiểm tường minh khi làm gt-driver (M9): driver từ chối sinh đầu ra nếu các kiểm tra này chưa có.

## gt-types: capability và region (M4.5, ADR-019)
- **Capability:** luật nguồn gốc theo kiểu (chỉ `main` cấp cao gọi được hàm trả `Cap<…>` mà không nhận `Cap<…>`). Không có phân tích dòng chảy: không phát hiện "capability cũ bị dùng sau khi đã chuyển đi" (đó là use-after-move, M6) và không kiểm hiệu ứng `{io}` của thao tác (M5). Khai báo (không gọi) nguồn gốc trong chương trình người dùng không bị báo. Danh sách loại capability do lib/core quyết.
- **Region:** chỉ kiểm CHỮ KÝ (khai báo, trùng, elision). Region KHÔNG nằm trong `Ty`; thân hàm chưa kiểm gì về region — `ƒ f() → &i32 { &x }` trả tham chiếu local, hay Arena thoát, đều QUA M4/M4.5 và chỉ bị chặn ở M6 (E0404/E0405). Cho tới khi M6 xong, đây là lỗ hổng đã biết.
- **Ngữ pháp:** đã có đối số lifetime trong kiểu (`Holder<'a>`, ADR-021). **Chưa có ràng buộc `'a: 'b`: một số chữ ký hợp lệ chưa viết được; chỉ thêm khi chương trình thật đòi** (Giang, 2026-10-06). Elision chặt hơn Rust: không có luật `&self` thắng; không tham chiếu vào ⇒ `'static` (M6 kiểm thân hàm thật sự trả dữ liệu `'static`).

## gt-mir / gt-dataflow (M5, ADR-020, ADR-022)
- **Chưa hạ (ICE E9901, không bỏ qua):** lượng từ `∀ ∃` ngoài hợp đồng; hợp đồng `⊳ ⊲ ↓ ⋈` và chứng cứ của `⚠` chỉ được GHI NHẬN (`Body.contracts`), hạ ở M7/M10; hàm tạo biến thể có trường dùng như giá trị; phương thức trait dùng như giá trị; `⟳` trên range ký tự.
- **Không có crate gt-hir** (hoãn, Q27): bước hạ MIR đọc thẳng AST + bảng của gt-types.
- **Không elaboration drop** (cờ drop) — `Drop` vô điều kiện ở cuối phạm vi, kể cả biến đã move; M10 làm elaboration.
- **Không tối ưu MIR:** `⌥` kiểm từng nhánh tuần tự (đọc lại discriminant mỗi nhánh); nhiều biến tạm/`StorageLive` hơn cần thiết. Đúng trước, nhanh sau.
- **Biến tạm của đuôi khối chết ở cuối khối** (chặt hơn Rust, ADR-022 §4); người nhận theo giá trị bị move sau đối số (ADR-022 §5).
- **Thứ tự đánh giá của phép gán khác Rust:** chỗ trước, giá trị sau (semantics §3a).
- **Thông dịch MIR (test) chỉ phủ** số nguyên, bool, rẽ nhánh, vòng lặp, gọi hàm, struct/tuple/enum, tham chiếu; không số thực, không lời gọi qua trait, không prelude ngoài `core::abort`. Nó và bộ sinh chương trình là công cụ test, không thuộc TCB.
- **`→ !` thật sự phân kỳ** chỉ kiểm được trên CFG dựng tay (M4 đã chặn mọi nguồn có thể kích hoạt).


## Môi trường build trên Windows (để phiên sau không gặp lại)
- **Ổ C: có thể hết chỗ** (đã gặp: 0 byte trống) ⇒ `link.exe` lỗi `LNK1104`/`LNK1108`, và cả công cụ shell báo `No space left on device`. `link.exe` và rustc ghi tệp tạm vào `%TEMP%`. Cách làm đã chốt: đặt `TEMP`/`TMP`/`TMPDIR` về `D:\Grit\target\tmp` (thư mục này nằm trong `target/`, đã bị `.gitignore`). `t.sh` (chạy `cargo test` có thử lại khi `LNK1104/LNK1108/1455`) và `tools/gate.sh` (cổng đầy đủ) đều làm việc đó sẵn — luôn chạy cargo qua hai script này.
- Kèm `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_BUILD_JOBS=1` (Windows hết commit bộ nhớ khi build song song). Không chạy hai `cargo` cùng lúc (tranh chấp khoá `target/` và bộ nhớ).
- Khi C: đầy mà shell không chạy được: giải phóng chỗ ở `%TEMP%` (thư mục cài đặt .NET/VS cũ). 2026-10-06 đã xoá `%TEMP%\meaphhtb` (4,2 GB, 16 ngày tuổi, là tệp tạm của cài đặt workload .NET — sinh lại được).
- Python in tiếng Việt ra ống/tệp cần `PYTHONIOENCODING=utf-8` (mặc định cp1252 làm script chết giữa chừng); không dùng `\` trong chuỗi Python gửi qua heredoc shell (xem `env-gotchas`).

- **Quy tắc môi trường (Giang, 2026-10-08; áp dụng cho MỌI build/test/đột biến):** không chạy hai tiến trình `cargo` cùng lúc; `CARGO_BUILD_JOBS=2` và test với `--test-threads=2` (`t.sh`, `tools/gate.sh` đã đặt sẵn); đột biến chạy **1 worker** (`tools/mut.py` mặc định 1), chia lô nhỏ và kết quả được ghi nối vào `.jsonl` NGAY sau từng đột biến (máy chết giữa chừng không mất tiến độ; chạy lại sẽ tiếp tục); không bao giờ giữ quá 1 bản sao làm việc đột biến (`D:/Grit_mut/w0`). Lý do: 4 worker + biên dịch song song làm hết commit bộ nhớ (`os error 1455`) và giết cả đợt đột biến M7 ở mẫu cuối.

## gt-borrowck (M6, ADR-023/024)
- **NLL 2018, sound nhưng không complete (ADR-023 §1):** chặn nhầm (dương tính giả) gồm: `RegionSig` mô-đun hoá (kết quả `pick(&x,&y)` buộc với CẢ HAI đối số dù thân chỉ trả một); liveness theo BIẾN (vật chứa giữ nhiều tham chiếu: mọi region trong kiểu cùng sống); chỉ số mảng coi mọi chỉ số chồng lấn; "problem case #3". Đo trên 20 000 chương trình ngẫu nhiên: 0,92% chương trình oracle-sạch bị từ chối, 100% thuộc hai nguyên nhân đầu.
- **Dựng ADT bất biến kéo dài khoản vay (sound, chặt hơn):** `Sl { s: &!p }` với `Sl<'a> { s: &!'a &'a i32 }` giữ `p` bị mượn suốt `'a` (Rust cũng vậy); xem ADR-024 §7.
- **Oracle động chỉ là bằng chứng kiểm thử:** thấy vi phạm trên đường đã chạy, và chỉ với cấu trúc bộ sinh sinh ra; quy tắc tĩnh Drop (E0407 cho kiểu có Drop) không được mô hình hoá; xem ADR-024 §6.
- **Trạng thái bằng chứng của borrowck (nói thẳng):** CHƯA được chứng minh hình thức. Bằng chứng gồm: corpus 172 xấu + 187 đúng (theo 8 nhóm ADR-023 §7) và 77 mẫu pass cũ; đột biến toàn crate (347 đột biến, 284 bị bắt, 50 sống sót đã phân loại); oracle động trên 20 000 chương trình ngẫu nhiên × 8 tổ hợp nhánh với độ phủ theo 10 loại cấu trúc; 1 609 cơ sở × tấn công hai phía; soundness hunting. **Hai lỗ hổng thật đã tìm thấy và vá (ghi ở ADR-024 §7), cả hai KHÔNG phải do bộ sinh ngẫu nhiên tìm ra lần đầu** (#1 do hunting viết tay, #2 do đột biến); sau đó bộ sinh được mở rộng để TỰ tìm được cả hai (tắt bản sửa ⇒ phát hiện). Hệ quả: tin cậy vào borrowck bị chặn bởi những cấu trúc mà bộ sinh/corpus chưa có; coi mỗi lần phát hiện lỗ hổng mới bằng cách khác là bằng chứng bộ sinh còn thiếu.

### Đột biến chưa giải quyết, coi như false negative tiềm ẩn (M6 "đạt có điều kiện")
Bốn đột biến cuối của `gt-borrowck/src/regions.rs` sống sót qua mọi test; KHÔNG được chứng minh tương đương, nên coi như false negative tiềm ẩn cho tới khi Giang chấp nhận lập luận hoặc có chương trình phân biệt:
- **254** (`ty_invariant`: `Ty::Ref(false, i) | Array | Slice ⇒ ty_invariant(i)` bị đổi nên `&T`/mảng/lát coi là hiệp biến): lập luận — đường duy nhất làm thu hẹp một vùng bất biến là GHI qua nó, mà `&T` không cho ghi (D5/E0308), nên ADT có trường `&'x (&!'y &'z T)` không khai thác được độ lỏng; mảng/lát có `&!` bên trong vẫn bất biến qua nhánh `Array|Slice` còn lại (test `adts_are_invariant_exactly_when_a_field_type_is`). Rủi ro nếu sau này có mutability bên trong (ô `Cell`-like trong lib/core, M12).
- **361** (`Repeat`: quan hệ hai chiều thay vì một chiều): lập luận — `[x; n]` có đúng MỘT nguồn nên chiều ngược chỉ thêm các điểm của nguồn vào đích. Đột biến này làm checker CHẶT hơn, không lỏng hơn.
- **591** (kết quả lời gọi: quan hệ hai chiều): lập luận — biến region của callee được tạo mới và cận dưới duy nhất của nó là chính kết quả, nên chiều ngược không thêm ràng buộc nào có thể quan sát. CHẶT hơn.
- **554** (`&self` của phương thức khai báo như `&!self`): hướng CHẶT hơn (thêm bất biến cho phần trỏ tới); tôi không viết được chương trình hợp lệ bị từ chối nhầm trong giới hạn nỗ lực hợp lý.
Ba trong bốn là hướng chặt hơn (không phải lỗ hổng an toàn nếu sai); 254 là hướng lỏng hơn và phụ thuộc vào việc ngôn ngữ chưa có mutability bên trong.

### ĐIỀU KIỆN CỨNG: mutability bên trong (Giang, 2026-10-07; cũng ở GRIT_PLAN.md mục 7, điểm 10)
Đột biến **238 (`regions.rs:254`)** chỉ an toàn chừng nào ngôn ngữ chưa có mutability bên trong, nhưng M8 đưa `Mutex`/atomics vào trước M12. **Trước khi bất kỳ kiểu nào có mutability bên trong được đưa vào fixture hoặc lib**, mở lại 238, 361, 591, 554: mỗi cái hoặc bị giết bằng chương trình tấn công dùng mutability bên trong (thêm kịch bản `interior_mut` vào bộ sinh, tắt bản sửa để kiểm), hoặc có lập luận tương đương mới tính cả mutability bên trong. **Chưa xong thì M8 không được bắt đầu phần Mutex/atomics.**

## gt-oblig (M7, ADR-025)
- **Sound, không complete — chỉ khoảng + zone (ADR-025 §2):** không chứng minh được điều cần quan hệ phi tuyến (`x*y`), đẳng thức giữa ba biến trở lên, hay điều kiện chỉ suy ra từ nội dung bộ nhớ (trừ độ dài lát). Mọi thứ không chứng minh được là check lúc chạy + ghi chú N06xx/N09xx, KHÔNG là lỗi. Tỉ lệ xoá trên corpus mẫu: xem PROGRESS.
- **"Chắc chắn sai" dựa trên khả năng tới được CỦA PHÂN TÍCH, không phải của thực thi:** nếu trạng thái trừu tượng ≠ ⊥ nhưng điểm đó thực tế không bao giờ tới (vd điều kiện mâu thuẫn mà miền không phát hiện), E060x/E0901 vẫn ĐỎ — hướng chặt hơn (từ chối chương trình hợp lệ), không phải lỗ hổng an toàn. Đã quan sát: 3/6000 ca ngẫu nhiên có hệ giả thiết mâu thuẫn mà bộ giải chính không phát hiện (`tests/provers.rs`).
- **Hợp đồng — các ràng buộc cố ý (fail-closed, E0902 thay vì bỏ qua):** (1) `⊳` chưa chứng minh mà đối số không `Copy` ⇒ E0902 (không sao chép được để gọi thân hợp đồng lúc chạy); (2) `⊲` nhắc tham số bị gán lại/mượn `&!` ⇒ E0902 (nghĩa "giá trị lúc vào"); (3) lượng từ chỉ hạ được khi miền là khoảng nguyên LITERAL `a..b`/`a..=b`; miền là một giá trị `Range` ⇒ E0902; (4) lượng từ NGOÀI hợp đồng `⊳ ⊲ ↓ ⋈` là lỗi ĐỎ E0316 ở M4 (ICE ở M5 chỉ còn là chốt phòng thủ không với tới được); (5) `⊳` chỉ là GIẢ THIẾT của thân khi hàm CHỈ bị gọi trực tiếp: không thoát qua giá trị hàm (`Const::FnItem`), không là thành viên của `⊐ Trait for`; hàm khác vẫn bị kiểm ở điểm gọi trực tiếp nhưng thân không được hưởng giả thiết.
- **Vị từ thuần được mô hình hoá như nguyên tử không diễn giải** (`is_some(&x)`, mọi hàm một đối số trả `bool`, hiệu ứng `∅`, không nhận `&!`), khoá theo local `x`; nguyên tử bị quên khi `x` bị gán lại. Đúng chừng nào giá trị của vị từ chỉ phụ thuộc nội dung `x` mà không có mutability bên trong — **ràng buộc thêm vào ĐIỀU KIỆN CỨNG mutability bên trong ở trên**: trước khi `Mutex`/atomics vào fixture, vị từ trên các kiểu đó phải bị loại khỏi `pure_preds` (hoặc có lập luận mới).
- **Điều kiện ghép `∧ ∨ ⇒ ¬`:** từ M5-fix (2026-10-08) điều kiện của `⎇` và `⟳` được hạ THẲNG thành nhánh nên bộ giải biết cả hai vế ở mỗi nhánh. Điều kiện dùng làm GIÁ TRỊ (gán vào biến bool, đối số) vẫn hạ như cũ (hợp hai đường), nên sau `c ≔ a ∧ b; ⎇ c { … }` quan hệ của cả `a` lẫn `b` chưa được dùng (sound, chưa đầy đủ).
- **Vị từ thuần, bí danh lát, hợp đồng:** xem các mục trên; lỗ hổng #3 (chuỗi bí danh qua tham số lát) và #4 (`⊲` chứng minh bằng giá trị hiện tại của tham số bị gán lại) đã sửa trong M7 và có test (`tests/decisions*.rs`).
- **Local bị mượn `&!` ở BẤT KỲ đâu trong thân không được theo dõi** (luôn ⊤): mất chính xác (vd biến đếm bị truyền `&!` một lần), không mất an toàn.
- **Cận chi phí (ADR-025 §12):** `ZONE_MAX_VARS = 24` biến cho quan hệ (N0605 liệt kê biến bị bỏ); `MAX_BLOCK_STEPS = 400 000` lần xử lý khối, vượt ⇒ chạy lại không quan hệ, vẫn vượt ⇒ giữ mọi check (N0605). Kết quả tất định.
- **Narrowing có thể bị bỏ** (`BodyReport.narrowing_rejected`) nếu không qua kiểm hậu-điểm-cố-định: chỉ kém chính xác hơn.
- **Bộ giải vét cạn chỉ ở độ rộng nhỏ; 8-bit đầy đủ chỉ chạy khi `GT_FULL8=1`** (tay, một lần); độ rộng 16..128 dựa vào công thức đã kiểm ở 2..8 (không được vét cạn trực tiếp). Đây là bằng chứng kiểm thử, KHÔNG phải chứng minh hình thức.
- **Oracle mức chương trình (bằng chứng, không phải chứng minh):** chỉ phủ chương trình một kiểu số nguyên, hàm phụ ≤ 3, mảng ≤ 2, không struct/enum/số thực; chỉ thấy vi phạm trên đầu vào đã thử (biên + ngẫu nhiên).
- **Không chèn kiểm cho `Drop`/unwinding:** abort của check lúc chạy kết thúc tiến trình (D3), không chạy `Drop`.

### Những gì oracle không quan sát được (M7; luật grit-contract §3.3, Giang 2026-10-08)
Oracle mức chương trình (`gt-oblig/tests/oracle.rs`) so MIR giữ-mọi-check với MIR đã tỉa trên cùng đầu vào. Nó MÙ với:
- **Chương trình bị từ chối bởi kiểm tra khác:** mọi chương trình có E0902 (vd `⊲` nhắc tham số bị gán lại) không có hành vi lúc chạy nào để so. Lỗ hổng #4 (chứng minh `⊲` bằng giá trị hiện tại của tham số bị gán lại) nằm đúng chỗ này; chỉ `tests/decisions2.rs` (golden chính xác) giết được nó.
- **Hợp đồng `⊲` trên tham số bị gán lại, nghĩa "giá trị lúc vào":** thân hợp đồng nhận giá trị HIỆN TẠI nên ngay cả MIR tham chiếu cũng không có nghĩa đúng để so.
- **Nhánh `SwitchInt` theo số nguyên:** M5 không sinh ra, nên 10 đột biến của nhóm `edges` không bị oracle chạm (xem bảng đột biến).
- **Chương trình ngoài bộ sinh:** một kiểu số nguyên, ≤ 3 hàm phụ, mảng ≤ 2, không struct/enum/số thực/closure; không `Callee::Value`/trait; không mutability bên trong.
- **Đường chạy sau lần abort đầu tiên:** ~70% lần chạy abort sớm (đo ở M7-sau-oracle: 14 367/48 000 kết thúc bình thường), nên code nằm sâu sau nhiều nghĩa vụ ít được chạy tới; ngưỡng "≥ 100 check bị xoá mà đoạn sau chạy tới cuối hàm" mỗi loại (ADR-025 §13) giảm nhưng không loại bỏ điểm mù này.
- **Giá trị đầu vào hiếm:** đầu vào là biên của khoảng `⊳` của `entry` + ngẫu nhiên đều; không có đầu vào "thông minh" (concolic).

### Đột biến chưa giải quyết của gt-oblig, coi như false negative tiềm ẩn (M7 "đạt có điều kiện")
Đột biến theo dòng trên 7 tệp của `gt-oblig/src` (1 148 đột biến): **826 bị bắt, 232 sống sót, 90 không biên dịch**. Lệnh Giang 2026-10-08 (mục 4): KHÔNG đuổi từng mẫu ở `analysis.rs`/`env.rs`/`driver.rs`; nhóm theo HÀM, viết chương trình golden cho hàm quyết định xoá/giữ (`tests/decisions.rs`, `decisions2.rs`, `env_ref.rs`; đã giết 100+ mẫu và tìm ra 2 lỗ hổng soundness thật), rồi dừng theo giới hạn nỗ lực (hai vòng golden + một vòng sửa).

| Nhóm | Tệp | Số sống sót | Hướng |
|---|---|---|---|
| **Quyết định xoá/giữ, chưa giải quyết** | `analysis.rs` 62, `driver.rs` 7 | **69** | **NGUY HIỂM: có thể làm bộ giải xoá THÊM check** (hoặc chèn sai) nếu đột biến là bug thật |
| Chèn kiểm lúc chạy (kiểu Copy, `apply_insert`) | `analysis.rs` 6, `driver.rs` 1 | 7 | chèn sai/thừa/E0902 sai; không xoá check |
| Tương đương/dư thừa theo lập luận cụ thể, hoặc chỉ mất chính xác/chi phí/báo cáo | `env.rs` 44, `analysis.rs` 74, `driver.rs` 16 | 134 | giữ thừa check hoặc không đổi hành vi quan sát được |
| Lõi số học & zone (`int.rs`, `interval.rs`, `zone.rs`, `prover.rs`) | | 22 | tương đương (lập luận dưới) |

Hàm của nhóm NGUY HIỂM (số mẫu sống): `assign_local` 11 (hàm chuyển: `Neg`/bool→số/định nghĩa ref), `judge_posts` 11 (phát hiện tham số được ⊲ nhắc), `edges` 10, `build_prog_info` 7 (điều kiện `pure_preds`, thành viên trait), `walk_paths` 7, `assume_posts` 6, `canon_term` 4, `canon_prop` 4, `subst_prop_k` 4 (ranh giới miền id nguyên tử), `pred_key` 3, `extract_contract` 2.
Lập luận từng nhóm (KHÔNG phải chứng minh):
- `edges` (10): nhánh `SwitchInt` theo số nguyên không được M5 sinh ra (`⌥` trên số nguyên hạ thành so sánh + rẽ bool), nên không với tới được từ MIR hiện tại; NẾU M5 đổi cách hạ thì nhóm này thành nguy hiểm thật — ghi điều kiện cứng bên dưới.
- `assign_local` (11): phần lớn là nhánh `UnOp::Neg` (luôn đi sau `Assert(OverflowNeg)` nên `fits` luôn `Yes`, hai nhánh tương đương) và ranh giới id của nguyên tử (`>= → >` trên `PRED_BASE`, vô hại vì id không bao giờ bằng đúng `PRED_BASE`).
- `judge_posts` (11), `canon_*` (8), `subst_prop_k` (4), `pred_key` (3): ranh giới của miền id nguyên tử (`<`/`<=`/`>=`) và phát hiện "tham số được nhắc" bằng nhiều dấu hiệu dư nhau (một chương trình luôn có thêm biến `result`, nên đột biến `== → !=` vẫn cho `used`); KHÔNG kết luận được tương đương từng mẫu.
- `build_prog_info` (7): tổ hợp `&&`/`||` của điều kiện vị từ thuần (`n_args == 1`, kiểu `bool`, hiệu ứng `∅`, không `&!`): đột biến nới điều kiện có thể mô hình hoá hàm KHÔNG thuần như vị từ — nguy hiểm thật nếu có chương trình tương ứng; chưa viết được.
- Hàm "an toàn": `reduce` (15, cờ `changed` chỉ ảnh hưởng số vòng lặp tới điểm cố định; phát hiện rỗng thiếu thì chỉ mất chứng minh rỗng), `kill` (6), `assume_le` (9, điều kiện thừa với `tri_le` phía trước; hệ số khác 0 nên `> 0 ≡ >= 0`), `refine_*` (4, gán lại cùng giá trị), `new` (12, ngưỡng widening: `min/max` và `±1` hoán đổi nhau vẫn chèn đủ tập; cận số biến zone; `record_pc` chỉ tốn bộ nhớ), `forget_local` (6, mọi local được gán trước khi đọc nên quên ở `StorageLive/Dead` dư thừa), `join/widen/step_stmt` (7, `Env` tự xử lý trạng thái chết), `hyps_for` (6, giả thiết thừa/ít hơn: chỉ yếu hơn), `fixpoint/narrow/check_post_fixpoint/analyze_body` (15, trần bước và cờ báo cáo; narrowing bị bỏ nếu kiểm hỏng), `analyze_program` (11) và `remaining_checks_json` (5: chẩn đoán/báo cáo), `var_name/stamp_local/site_atom/...` (nhãn).
- Lõi (22): `int.rs` 4 (`make` chuẩn hoá số 0 âm; nhánh `to_decimal` xoá số 0 đầu dư thừa với vòng cắt số 0 cuối), `interval.rs` 6 (`rem_trunc_nz`: các nhánh `>= → >` đều cho cùng kết quả ở biên 0; hai nhánh phòng thủ `lo == +∞`/`hi == −∞` không với tới), `zone.rs` 13 (cờ `empty` đã chặn mọi thao tác; ma trận rỗng luôn có chéo âm nên `leq`/`meet_with` vẫn đúng; `shift` ghi chéo hai lần triệt tiêu), `prover.rs` 2 (đã giết 2 mẫu, còn 2 là `(ZoneProver::new(0))` tương đương).

**Điều kiện cứng mới (Giang, 2026-10-08; GRIT_PLAN.md mục 7, điểm 11):** M9 không được coi là xong nếu còn đột biến theo hướng "xoá thêm check" trong đường quyết định xoá/giữ của `analysis.rs` (69 mẫu trên là danh sách đó); và M9 PHẢI có chế độ **giữ mọi check** hoạt động đúng làm mặc định khi dùng thử Grit thật cho tới khi điều kiện được giải quyết (đề xuất của Giang, để đánh giá trong ADR của M9).
**Điều kiện kèm theo (của tôi):** nếu M5 đổi cách hạ `⌥`/điều kiện `⎇` thì chạy lại đột biến nhóm `edges`.

## gt-oblig M8 nửa 1: điều kiện dừng và hiệu ứng (ADR-026)
- **Fail-closed, mất chính xác (hợp lệ nhưng bị bác):** biến thể `↓` chỉ gồm tổng/hiệu của tham số/biến nguyên và `xs.len()` với hệ số ±1; phép nhân (kể cả với hằng) cho E0702 vì `lin` không hỗ trợ `Mul`; tối đa 4 biến mỗi cặp zone. Đệ quy qua giá trị hàm/phương thức trait luôn E0703 (đồ thị gọi bảo thủ: mọi hàm có thể bị gọi gián tiếp). Điều kiện `∨`/`!=` không suy được biến thể. `μi ≔ n` không cho quan hệ zone `i = n` ở đầu vòng nên `DomainQuery` không thấy `i − n ≤ 0` ở đó (chứng minh biến thể dùng biến bóng nên không bị ảnh hưởng).
- **Oracle dừng (term_oracle, 1500 chương trình):** 318 chấp nhận (đều dừng), 627 bị bác mà thật sự phân kỳ, 555 bị bác nhưng thật ra dừng (mất chính xác); E0701 608, E0702 1109, E0703 112. Chèn lỗi `Lt → Le` vào chứng minh biến thể: oracle bắt ở chương trình #78. Điểm mù: "phân kỳ" = hết 300 000 bước hoặc đệ quy quá sâu; vòng dừng sau hơn 300 000 bước bị coi là phân kỳ (không gây âm tính giả cho chương trình được chấp nhận).
- **Đột biến M8 nửa 1 chưa giải quyết (false negative tiềm ẩn):** `effects.rs` 19 → 6 sống sót (4 nhánh ICE không tới được từ chương trình hợp lệ: dòng 49/60/73/95; 2 nhánh `continue → break` bỏ qua impl không trait / thành viên không khớp: 142/146). `termination.rs` 137 → 14 sống sót: tương đương hoặc chỉ khác chữ thông báo (189 cờ Tarjan, 507 ×4 chữ, 528 `specs` rỗng, 594 span); giới hạn chính xác (84 `is_int_local`, 114 `Mul`, 129 tên `.len`); chưa bắt được: 412, 471, 477, 511 (`continue → break` ở chỗ bỏ qua và cờ `given` của đặc tả vòng). `analysis.rs` (chứng minh biến thể/đệ quy, 17 + chạy lại): các đột biến theo hướng nguy hiểm (`all → any` của tham số nguyên vẹn, so sánh SCC, `&& → ||` của điều kiện gán lại/mượn) đều đã bị bắt; còn 7 sống sót không ảnh hưởng tính đúng (nhãn loại nghĩa vụ ở dòng 1408, cờ `is_assert` ở 1416/1473, biên chỉ số local ở 1449/1454).

## gt-types `spawn` / `Send` (M8 nửa 2, ADR-026 §2; Q51–Q52 chờ duyệt)
- **Q51 đã duyệt (Giang, 2026-10-08) cho Phase 1:** tham chiếu, con trỏ thô, `Arena` không `Send`. Luồng có phạm vi và chia sẻ tham chiếu giữa các luồng thuộc **8.D (Phase 2)**. **Q52 chỉ duyệt cho `spawn` thuần:** khi Mutex vào, worker phải được phép có `lock(L)` và `mem` (và `io`: ADR Mutex nêu quyết định), cập nhật E0501/Q52 cho khớp.
- **Bảo thủ:** mọi tham chiếu, con trỏ thô và `Arena` không `Send`; worker của `spawn` phải `! ∅`; `spawn` không trả về tay cầm (không `join`). Thông dịch MIR chạy `spawn` tuần tự — KHÔNG phản ánh ngữ nghĩa luồng thật, không phải bằng chứng cho codegen (M10).
- **Không quan sát được:** không có oracle động cho đua dữ liệu (thông dịch tuần tự không tạo ra đua); độ tin cậy của E0501 dựa vào corpus (5 fail + 4 pass), test cấu trúc (generic, đệ quy, đệ quy đa hình, ràng buộc theo tham số) và đột biến.
- **Đột biến `send.rs` (22):** sau khi thêm test còn 1 sống sót tương đương: `depth > 64` → `>= 64` (chỉ dịch biên chốt độ sâu). Đột biến theo hướng nguy hiểm (ràng buộc của tham số khác, `seen` che trường bẩn) đều bị bắt.

### Điều kiện cứng mutability bên trong: TRẠNG THÁI (2026-10-08, ADR-027 §0)
- **Đã làm trước khi Mutex/Atomic* vào fixture/lib:** lang item `cell` bất biến trong `regions.rs`; **238 đã giết** (test tấn công `interior_mut.rs` + kịch bản `interior_mut` của bộ sinh với oracle động hiểu `Cell`; tắt bản sửa ⇒ oracle bắt 53 âm tính giả/3 000 chương trình); vị từ trên kiểu có thể chứa ô bị loại khỏi `pure_preds`.
- **Chờ Giang chấp nhận lập luận đơn điệu** cho **361, 591, 554** (dòng 366, 596, 559 hiện tại): cả ba THÊM ràng buộc ⇒ chỉ có thể từ chối thêm, không chấp nhận thêm. Vẫn sống sau lần chạy lại với bộ test mới.
- Đột biến `TypeDb::may_have_interior_mut` (db.rs) và điều kiện loại vị từ (driver.rs): còn 2 tương đương (`Ty::Infer` không còn sau suy kiểu; ADT thiếu trong `adts` không xảy ra với chương trình hợp lệ) và 1 có sẵn từ M7 (driver.rs `mut_arg`).
- Sai khác bộ sinh `interior_mut`: dương tính giả "ghi đè muộn" (kiểu `Cell<&'a T>` buộc mọi tham chiếu lưu vào sống tới lần dùng cuối của ô) đã được phân loại là nguyên nhân đã biết (Rust cũng từ chối).

## gt-types / gt-oblig: Mutex, Atomic, Sync, thứ tự khoá (M8 nửa 2, ADR-027)
- **Phạm vi bảo thủ (fail-closed):** khoá chỉ qua `Mutex::with(&self, f, arg)` có phạm vi (không guard, không giữ khoá qua nhiều hàm); `Arg: Send`, `R: Send` (cấm tham chiếu); chia sẻ giữa luồng chỉ qua `■` static kiểu `Sync` bằng TÊN; tham chiếu giữa luồng và luồng có phạm vi thuộc 8.D. Kiểu cấp khoá phải là kiểu CỤ THỂ có `#level("n")` (tham số kiểu hay thiếu `#level` ⇒ E0502); trùng tên ngắn giữa hai kiểu cấp khoá khác cấp ⇒ E0502. Atomic chỉ cho `bool`/`i32`/`u32`/`i64`/`u64`/`usize`; ordering phải truy ngược được về biến thể viết thẳng (E0503), biến gán hai lần hay truyền qua tham số ⇒ E0503.
- **Hiệu ứng đa hình `poly`/`detach`:** chỉ nhận hàm là tên hàm tự do hoặc biến cục bộ kiểu hàm (không gọi/biểu thức trả về hàm); hiệu ứng của `spawn` = hiệu ứng worker TRỪ `lock(*)` (worker không lồng trong khoá người gọi). Hiệu ứng `ffi` bị cấm ở Phase 1 (D14).
- **Không quan sát được:** KHÔNG có oracle động cho đua dữ liệu/deadlock/memory ordering (thông dịch MIR tuần tự, tất định); độ tin cậy dựa vào corpus (E0501/E0502/E0503), test cấu trúc và đột biến. **Thông dịch của `Cell`/`Mutex`/`Atomic`/`spawn` là công cụ test, NGOÀI TCB; không phải bằng chứng cho codegen — M10 phải cài bằng atomics/khoá thật và kiểm thử luồng thật (ThreadSanitizer/loom-like) trước khi tuyên bố đa luồng.**
- **Đột biến M8 nửa 2 chưa giải quyết (false negative tiềm ẩn / tương đương):** `send.rs` (21): 2 tương đương (biên chốt độ sâu; chữ thông báo). `ordering.rs` (63 → 8 sau lần chạy lại): chốt độ sâu (26 ×2), truy vết qua phép chiếu (32) và `&&` thay `||` (57) mà kiểu đã loại trừ, nhánh bỏ qua khi chữ ký thiếu (94), cờ `bad` chỉ tránh báo trùng (108/120/121). `effects.rs` phần khoá (23 → 6): nhánh ICE có sẵn (61/74/105/152/156) và `cs.name == "with"` (279, chỉ `with` mang `lock`). Các đột biến theo hướng nguy hiểm (truy vết ordering, lan truyền hiệu ứng của đối số hàm, thứ tự khoá, trùng tên cấp) đều đã bị bắt sau khi thêm test.

### M8 "ĐẠT CÓ ĐIỀU KIỆN" (2026-10-09, theo yêu cầu của Giang)
- **Quy tắc bất biến cho `Mutex`/`Atomic` chưa được chứng minh là cần thiết bằng kiểm tắt bản sửa** (tắt chỉ Mutex/Atomic ⇒ 0 âm tính giả), vì chưa có đường nhét tham chiếu sống ngắn vào chúng (`Arg: Send`; Atomic chỉ chứa số). Nó là phòng thủ chiều sâu; chỉ `Cell` thể hiện quy tắc qua oracle (tắt ⇒ 24 âm tính giả/3 000). **Điều kiện mở khoá:** khi 8.D cho phép tham chiếu trong `Arg`/luồng có phạm vi, thêm tấn công riêng cho Mutex vào kịch bản `interior_mut` và chạy lại kiểm tắt bản sửa trước khi bật.
- **Đột biến 361/591/554 của `regions.rs` còn sống**, cả ba theo hướng CHẶT hơn (chi tiết bảng ở ADR-027 §0); lập luận đơn điệu + đo 0 âm tính giả chờ Giang chấp nhận. Đo trên 6 000 chương trình: bản gốc và cả ba bản đột biến cho kết quả giống hệt (0 âm tính giả, 127 dương tính giả).
- Còn nguyên các đột biến M7 chưa giải quyết (điều kiện cứng điểm 10) và Q48–Q52 (xem DECISIONS).

## gritc / M9 (ADR-028)
- **Backend duy nhất là thông dịch MIR tuần tự** (`"backend":"interpreter"`, mọi đầu ra ghi rõ). `spawn` chạy worker NGAY, tuần tự: KHÔNG thực thi đua dữ liệu, KHÔNG khoá thật, KHÔNG memory ordering thật, không có luồng nào chạy song song; ngữ nghĩa của thông dịch KHÔNG phải bằng chứng cho codegen M10 (M10 phải cài bằng luồng/atomics/khoá thật và kiểm thử luồng thật). `gritc build` ở M9 ghi gói `*.gtmir` (MIR + attestation), không phải binary.
- **Mặc định `--checks=all`** (giữ mọi check) cho tới khi các đột biến "xoá thêm check" của `analysis.rs`/`driver.rs` (69, xem "gt-oblig") được giải quyết và Giang duyệt (GRIT_PLAN §7 điểm 10). `--checks=proven` chỉ để đo/thử.
- **E1001 bảo thủ (D21):** MỌI `⚠` ở mã người dùng bị từ chối (Phase 1 chưa có nghĩa vụ `⊢` nào giải được), kể cả khối rỗng; lib/core miễn (D22). Kiểm trên cờ `unsafe_ctx` của MIR (mã người dùng) và chữ ký/trường/hằng.
- **Bộ quét secret (S101/S102/S201/S202) không đầy đủ:** mẫu nhận dạng và ngưỡng entropy (≥ 3,3 bit/ký tự, ≥ 12 ký tự; mật khẩu ≥ 8 ký tự, ≥ 2,5, có chữ số) là kinh nghiệm — có dương tính giả (khoá mẫu trong tài liệu) và âm tính giả (secret ghép lúc chạy, mã hoá lồng, tên không gợi ý). S201/S202 theo dõi taint theo THÂN HÀM, không theo luồng điều khiển, không xuyên hàm; sink = lời gọi có hiệu ứng `io`.
- **`gritc fmt`:** E1301 (không gắn được comment) là phương án cuối của gt-fmt và không có đầu vào đã biết nào sinh ra nó; đường từ chối của `fmt` (tệp KHÔNG bị sửa) được kiểm bằng lỗi cú pháp. Định dạng tại chỗ ghi nguyên tử (tệp tạm rồi đổi tên).
- **`GRITC_FAULT`** (chèn lỗi kiểm cổng) nằm trong binary phát hành; chỉ làm gritc từ chối thêm. Prelude của gritc là `lib/core/prelude.gt` (giả lập lib/core; M12 thay) — có test khoá bằng fixture của gt-types.
- **Đầu vào:** `gritc` nhận một tệp hoặc một thư mục (mọi `*.gt`, sắp theo tên); chưa có manifest/phụ thuộc (Rig, M11). `run` đòi đúng một `ƒ main()` không tham số ở cấp tệp.

### Đột biến M9 (1 worker, lô nhỏ; theo lệnh: chỉ PHẦN PHÁN QUYẾT của gt-driver + phần mới của gt-oblig; `gt-secrets` làm thêm)
- **`gt-driver/src/verdict.rs`** (hàm quyết định chạy/không chạy, mã thoát): 25 đột biến ⇒ 23 bị bắt, 2 sống sót ("pass hỏng ⇒ ICE": test cũ không phân biệt) ⇒ thêm test pass phụ hỏng ⇒ **cả hai bị bắt**. Còn lại: 0.
- **`gt-driver/src/pipeline.rs`** (cổng `open_gate`/`take_gated`, chèn lỗi, pass bắt buộc, canary): 17 đột biến ⇒ 13 bắt, 1 không hợp lệ, 3 sống sót ⇒ thêm test trạng thái từng nhãn phụ ⇒ 2 bị bắt. **Còn 1 tương đương:** `selftest`: `.any(…) ⇒ .all(…)` (canary chỉ phát đúng một mã).
- **`gt-driver/src/cli.rs`** (logic cổng ở `build`/`run`, 20 đột biến trên dòng 144–262): 13 bắt, **7 sống sót đều là phần hiển thị / tương đương, không phải quyết định cổng:** thông điệp "đã ghi gói" (dòng 171, 4 biến thể), in cảnh báo người đọc trước khi chạy (dòng 202–203, màu), kiểm hợp lệ mã của `explain` (244: mã sai vẫn ra mã thoát 3 qua bảng tra).
- **Nhánh `--checks=all` trong `gt-oblig/src/driver.rs`:** 5 đột biến ⇒ 4 bắt, 1 không hợp lệ, **0 sống sót**.
- **`gt-oblig/src/unsafe_check.rs` (E1001):** lần chạy đầu 84 đột biến ⇒ 43 bắt, 39 sống sót, 2 không hợp lệ. Đột biến PHÁT HIỆN LỖ HỔNG THẬT: chữ ký thành viên TRAIT chứa con trỏ thô không bị kiểm (vòng kiểm chỉ duyệt hàm/impl vốn đều có thân) ⇒ đã thêm vòng duyệt thành viên trait + test; đã bỏ hai vòng kiểm chữ ký hàm/impl thừa. **Chưa chạy lại toàn bộ sau khi sửa** (lần chạy lại một phần trước khi bỏ mã thừa: 50 đột biến ⇒ 15 bắt, 35 sống sót, phần lớn là chọn span của chẩn đoán (`first_span`, không ảnh hưởng việc từ chối), đường `raw_assign` KHÔNG tới được (con trỏ thô không tạo được từ mã an toàn: M4 bác ép `&`→`*`), kiểm `is_prelude` (prelude không có con trỏ thô) và các chốt độ sâu. **Nói thẳng:** số liệu cuối cho `unsafe_check.rs` chưa có; các nhánh quyết định (có ⚠, có con trỏ thô trong chữ ký/trường/trait/kiểu hàm) đều có test chính xác.
- **`gt-secrets/src/lib.rs` (cảnh báo VÀNG, làm thêm):** 200 đột biến ⇒ 131 bắt, 67 sống sót, 2 không hợp lệ ⇒ thêm test biên chính xác cho từng mẫu, taint nhiều đối số/ADT/vòng lặp ⇒ lần chạy lại 106 ⇒ 73 bắt, 33 sống sót ⇒ thêm test ⇒ lần chạy cuối DỪNG sớm ở 57/75 (một số đột biến làm vòng quét treo, mỗi cái chiếm 15 phút timeout): 33 bắt, **24 sống sót**, phần còn lại chưa chạy. Sống sót còn lại chủ yếu là biên số học tương đương (ngưỡng entropy so với số thực, `steps ≤ 8` của cửa sổ nhìn trước, chốt độ sâu), `continue ⇒ break` trong vòng quét tên, và vài nhánh phòng thủ khó tới. Đây là bộ quét kinh nghiệm (false negative tiềm ẩn đã nêu ở mục "gritc / M9"), không phải kiểm tra an toàn.
- **Cổng phán quyết cuối cùng** còn được bảo vệ ở hai lớp độc lập ngoài đột biến: canary `selftest` (E0601/E0307 phải còn bị bắt trước mọi phán quyết sạch) và danh sách pass bắt buộc.

## M10 — backend LLVM (ghi 2026-10-13)
- **Nghĩa vụ nhạy cảm theo LỚP, không theo điểm:** IR Guard đòi mỗi atomic/spawn/mutex mang `!gt.obl` đúng lớp (E0503/E0501/E0502) nhưng chưa trỏ tới từng nghĩa vụ đã giải cụ thể (M7/M8 chưa có ID nghĩa vụ ổn định theo điểm). Hệ quả: một thao tác nhạy cảm do bug codegen thêm vào mà vẫn được gắn đúng lớp sẽ lọt Guard; phát hiện nhờ test đối chiếu, không nhờ Guard.
- **Chưa hạ được (⇒ `Unsupported`, gritc exit 3, không exe):** số thực; gọi qua trait (`Callee::TraitMethod`); nhân/chia/`*?` 128 bit (cần `__multi3`/`__divti3`); `MulSat`; `Atomic<bool>`; ordering không phải hằng lúc biên dịch; sao chép giá trị `static` có Atomic/Mutex/Cell; thân hợp đồng `Pre/Post` như hàm.
- **`Drop` người dùng bị bỏ qua** (cùng ngữ nghĩa thông dịch; Phase 1 chỉ có `Drop` giả trong prelude).
- **`spawn` không có `join`:** chương trình kết thúc khi `main` trả về, luồng còn chạy bị dừng cùng tiến trình; hộp tham số của `spawn` không giải phóng (rò rỉ có chủ đích, nhỏ).
- **Thứ tự khởi tạo `static`** chỉ theo phụ thuộc TRỰC TIẾP trong thân khởi tạo; static đọc static khác QUA hàm gọi trong initializer có thể sai thứ tự (chưa có test phản ví dụ; ghi để biết).
- **TSan không dùng được** với toolchain MSVC hiện tại: đa luồng được kiểm bằng chạy lặp (4 luồng × 1000 lần, 68/68 vòng đúng) chứ không bằng bộ phát hiện đua dữ liệu. Không thay được chứng minh tuyệt đối.
- **Chỉ x86_64-pc-windows-msvc**, LLVM 20.1.8 liên kết động (`LLVM-C.dll` trên PATH khi chạy gritc backend llvm; exe sinh ra KHÔNG cần).
- **M10 "đạt có điều kiện" (duyệt 2026-10-13):** `gt-guard` và `gt-codegen-llvm` CHƯA qua đột biến; chỉ có hồ sơ `dev` (-O0, mọi Assert giữ). Chứng cứ thay thế: test đối chiếu thông dịch ≡ llvm, IR dựng tay cho Guard, golden IR, mẫu đa luồng. Hồ sơ `release` (-O2) KHÔNG có: sau tối ưu IR khác IR Guard đã kiểm ⇒ bộ tối ưu LLVM nằm trong TCB; cần ADR riêng trước khi làm.

## gt-guard — đột biến (2026-10-13, 1 worker, `tools/mut.py`, test: `cargo test -p gt-guard`)
- 40 đột biến sinh được trên `gt-guard/src/lib.rs` (hàm quyết định: 5 luật + từ chối): lần đầu 38 giết / 2 sống. Hai sống sót phân tích theo hướng:
  - **lib.rs:158 `llvm.assume` (`condition → false`)**: hướng CHẤP NHẬN THÊM IR ⇒ viết test `rejects_llvm_assume` ⇒ GIẾT (kết quả cuối 39/40 giết).
  - **lib.rs:29 `callee_name` chỉ nhận `call` (`condition → false`)**: TƯƠNG ĐƯƠNG ở hướng chấp nhận: dạng chữ chỉ sinh `@tên(` cho `call`/`invoke`; `invoke` là terminator nên không đứng ngay trước `unreachable` trong cùng khối (luật 4 không bị qua mặt); với luật 3 đột biến chỉ làm Guard NGHIÊM hơn (đòi `!gt.obl` cả trên `invoke`). Còn sống có chủ đích, không có test giết được.
- **Phạm vi chưa phủ bởi đột biến:** các nhánh `None => …` / mũi tên `"gritrt_spawn" => Some("E0501")` bị đột biến dòng-theo-dòng không sinh (tool chỉ đột biến toán tử/điều kiện/hằng); đã kiểm tay bằng test `runtime_calls_need_their_class` (xoá một mũi tên thì test đỏ). `gt-codegen-llvm` KHÔNG đột biến (điều kiện duyệt M10; thay bằng so sánh ngẫu nhiên, xem PROGRESS).

## I/O tối thiểu của dogfood (2026-10-13; xem docs/DOGFOOD.md)
- `Cap<Fs>::read` đọc NGUYÊN tệp vào bộ nhớ (không giải phóng; tệp lớn ⇒ tốn RAM tương ứng); `File::bytes` trả lát trỏ vào bộ nhớ đó. Mọi lỗi I/O/đối số ⇒ abort (mã 70, kind `io_error`/`arg_out_of_range`) kèm dòng chi tiết ở stderr; CHƯA có `Result`/`IoError`.
- `root_*()` là hàm nguồn capability của prelude (giả lập M12); **E0310 (D28/D29) chỉ cho `main` cấp cao của tệp gọi hoặc nhắc tới chúng** — đã kiểm 2026-10-13: hàm thường gọi `root_fs()`/`root_out()` bị ĐỎ E0310. (Câu cũ "gọi ở đâu cũng được" trong báo cáo bước 5 là SAI.) Còn lại: capability gốc vẫn do hàm prelude trả (chưa do runtime cấp cho `main`) — M12.
- Bộ giải M7 không chứng minh `b[i]` cho `b` cục bộ lấy từ `f.bytes()` dù có `i < b.len()` (check runtime còn nguyên, đúng nhưng chậm hơn).
- IR Guard không có lớp nghĩa vụ cho lời gọi runtime I/O.
- `Cap<Env>::arg` chuyển đối số không-UTF-8 thành lossy.
- **gt-rt `unsafe`:** danh sách ở ADR-031; chưa có test tự động đếm `unsafe` của gt-rt so với danh sách đó. Cột "người duyệt" của `docs/TCB_INTRINSICS.md` đang `CHỜ GIANG` cho mọi intrinsic.
- **gt-guard — đột biến sau khi thêm luật lớp I/O + bảng RUNTIME đóng (2026-10-13):** 57 đột biến (40 cũ + 17 mới cho luật 3b/lớp E0310), 1 worker: **56 giết / 1 sống**; mutant sống là đúng mutant TƯƠNG ĐƯƠNG cũ (`callee_name` chỉ nhận `call`, dòng 32 sau khi chèn tài liệu): lập luận tương đương ở mục "gt-guard — đột biến" phía trên vẫn đúng và áp cho luật 3b (`invoke` là terminator, không phải đích hợp lệ nào do codegen sinh; nếu có `invoke` tới `gritrt_*` thì Guard hiện từ chối vì `gritrt_*` lạ KHÔNG được xét như call ⇒ chỉ nghiêm hơn). Kết quả: `docs/MUTATION_GT_GUARD.jsonl`. Bản chụp được đột biến là commit "IR Guard: lớp nghĩa vụ E0310…"; thêm 4 dòng bảng RUNTIME cho arena/`fs_open` sau đó là dữ liệu, không đổi logic.
- **Từ dogfood công cụ 2 (JSON, 2026-10-13):** (a) tên biến thể enum người dùng che `Ok`/`Err`/`Some`/`None` của prelude trong cùng tệp (đặt tên khác); (b) M8 đòi MỌI cạnh của chu trình đệ quy tương hỗ giảm nghiêm ngặt (không chấp nhận giảm trên một cạnh/trọng số) và không chứng minh vòng với bước tiến `i ← i + f(..)` (dùng nhiên liệu); (c) không có phân rã tuple trong `≔`; (d) tool JSON không giải mã escape/không có giá trị số (không có `String`/`Vec`/`f64` ở backend); (e) lồng tối đa 100 tầng trong `examples/json`.
- **gt-guard — đột biến SAU FB-2 (hàm `callee_name` đã viết lại bằng API; 2026-10-13, 1 worker):** 57 đột biến ⇒ **56 giết / 1 sống** (dòng 33, `condition → false` của kiểm `opcode == Call`). Tương đương theo hướng chấp nhận: bỏ kiểm opcode thì `CallSiteValue::try_from` vẫn trả `Err` cho mọi lệnh không phải call/invoke ⇒ `None`; chỉ `invoke` lọt qua và khi đó bị xét như call (Guard NGHIÊM hơn); `invoke` là terminator nên không thể đứng ngay trước `unreachable` ⇒ luật 4 không bị qua mặt. Kết quả: `docs/MUTATION_GT_GUARD.jsonl` (thay bản trước FB-2, vốn là của hàm cũ).
- **Phân rã tuple trong `≔` (`(a, b) ≔ f()`) — HOÃN (2026-10-13):** dùng `t.0`, `t.1`. Cần cú pháp mới ⇒ cần Giang duyệt; xét lại khi chương trình thật cho thấy vướng (đã gặp ở examples/json: dùng `s.0/s.1`, đọc được nhưng dài).
- **Tên biến thể người dùng che `Ok`/`Err`/`Some`/`None` của prelude (giữ, Q17/D7):** lỗi hiện ra là E0301 (pattern kiểu `R` ≠ `Result`), KHÔNG phải E0205; nay kèm gợi ý sửa `Result::Ok` (MaybeIncorrect) và chỉ rõ enum bị che.
- **`core::fuel_left` + mẫu nhiên liệu:** chương trình tự lo nhiên liệu đủ (≥ số vòng tối đa); M8 chứng minh dừng nhờ `fuel`; cạn ⇒ abort `fuel_exhausted` ở runtime (check còn lại ghi N0905).
- **M7: hàm có > `ZONE_MAX_VARS` = 24 biến nguyên mất quan hệ giữa biến (N0605) ⇒ vòng `⟳ j < rem ↓ rem - j` có thể bị E0702 dù đúng** (gặp ở sha256 `digest`, 2026-10-13). Cách tránh: tách hàm. Không đổi bộ giải.
- **Phép bit chỉ có cho `u32`** (`band bor bxor bnot shl shr rotr rotl`); chưa có u8/u64. Mọi hàm tương ứng là TCB (docs/TCB_INTRINSICS.md).
