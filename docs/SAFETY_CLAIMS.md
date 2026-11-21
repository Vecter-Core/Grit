# SAFETY_CLAIMS — Grit đảm bảo gì và KHÔNG đảm bảo gì

Tài liệu này nói đúng mức tin cậy mà dự án có bằng chứng. Nó không phải lời quảng cáo. Nền đã kiểm: **Linux x86_64** (LLVM 20.1.x, `cc`). **Windows** đã kiểm bằng cổng liền mạch `13e2df9` (2026-11-18, GREEN 1023/0/204); CI GitHub, capstone K1/K2/K4 trên Windows và VS Code thật vẫn CHƯA kiểm.

## 1. Bản chất của đảm bảo
- An toàn của Grit đến từ **KIỂM TRA TĨNH có thể sai** + **bằng chứng thực nghiệm**: corpus chương trình đúng/sai, oracle động (chạy chương trình thật rồi so với kết luận của bộ kiểm), đột biến (cố ý làm hỏng chính bộ kiểm và đếm xem test có bắt được không), so sánh ba nguồn (mô hình Rust độc lập ≡ thông dịch ≡ exe LLVM) và fuzz.
- Đây **KHÔNG phải chứng minh hình thức**. Một lỗi trong compiler (một "âm tính giả") có thể cho phép một chương trình sai chạy được. Dự án coi âm tính giả là lỗi nghiêm trọng nhất: mỗi cái tìm ra được ghi vào `docs/FIELD_BUGS.md` (tiền tố FB-C trong nhánh này), có test ĐỎ trước khi sửa, và vào corpus vĩnh viễn.
- Thiết kế **fail-closed**: nếu gritc không chứng minh được hoặc không biết ⇒ từ chối (ĐỎ/ICE) chứ không cho qua. Kiểm tra số học/chỉ số không chứng minh được thì được GIỮ lại và chạy lúc chạy (ghi chú `N06xx`), mặc định `--checks=all`.

## 2. Lớp lỗi Grit chặn (với mức bằng chứng)
| lớp lỗi | cơ chế | bằng chứng chính |
|---|---|---|
| lỗi kiểu, thiếu đường trả về, `match` không vét cạn, gán vào biến bất biến | M4/M5 | corpus fail/pass theo mã `E03xx`; đột biến |
| use-after-move, mượn chồng, tham chiếu sống lâu hơn dữ liệu, dùng sau `Arena::reset` | M6 (mượn + vùng đời) | corpus + **oracle động** (`gt-borrowck/tests`, bộ sinh chương trình) + đột biến có "tắt bản sửa" |
| tràn số, chia 0, chỉ số/lát ngoài biên, ép kiểu mất thông tin | M7 (nghĩa vụ) + kiểm tra lúc chạy giữ lại | corpus + so sánh ba nguồn; **69 đột biến nguy hiểm của M7 còn sống ở thời điểm viết** (xem `docs/STATUS.md`) |
| vòng lặp/đệ quy không chứng minh được dừng | M8 (`↓`, nhiên liệu) | corpus `E07xx`; đột biến |
| hiệu ứng không khai báo, I/O không có capability | M8 effects + E0310 | corpus; điều kiện cứng #12 (mô hình capability) |
| rò rỉ bí mật vào I/O, bí mật trong mã nguồn | M9 | corpus `S1xx/S2xx` |
| data race qua chia sẻ không `Send/Sync`, đảo thứ tự khoá | M8 conc | corpus `E05xx` + `threads_repeat` (lặp 300 lần) |
| IR LLVM chứa thao tác nhạy cảm thiếu nghĩa vụ, cờ UB | IR Guard (`gt-guard`) | bảng `RUNTIME` đóng; đột biến Guard |
| chương trình hợp lệ bị OS giết vì ngăn xếp nhỏ | `main` chạy trên ngăn xếp 256 MiB | `stack_depth.rs` (FB-C8) |

## 3. Lớp lỗi Grit KHÔNG chặn (đừng dựa vào Grit cho những điều này)
- **Logic nghiệp vụ**: chương trình chạy đúng theo mã nhưng mã sai đề bài. (Vì vậy dự án so với bản tham chiếu Rust độc lập ở `capstone/`.)
- **Abort có kiểm soát vẫn là dừng chương trình**: tràn số hoặc chỉ số ngoài biên không chứng minh được sẽ abort (mã 70) — an toàn về bộ nhớ nhưng là từ chối dịch vụ nếu đầu vào do kẻ tấn công điều khiển.
- **Cạn tài nguyên**: bộ nhớ Arena, thời gian chạy, đĩa. Có giới hạn `GRITRT_ARENA_LIMIT` nhưng không có hạn mức thời gian.
- **`! ∅` không có nghĩa "không tạo luồng"** (ADR-C12, GÓI 10): hiệu ứng của `spawn` bằng hiệu ứng của worker; chú thích `{detach}` trong prelude không được thi hành, nên hàm `! ∅` gọi `spawn` với worker `! ∅` vẫn được chấp nhận. Đừng dùng `! ∅` để suy ra "đơn luồng"; đường siết cần ADR riêng (xem `docs/NEXT_PRODUCT_BATCH.md`).
- **Data race không có oracle động**: mô hình đa luồng mới chỉ được kiểm bằng corpus + lặp; không có bộ sinh/oracle động đầy đủ cho mọi mẫu truy cập.
- **Hạ tầng bên ngoài TCB**: bug của LLVM, trình liên kết, hệ điều hành, phần cứng, thư viện C chuẩn; kênh phụ (thời gian, bộ nhớ đệm); tệp/đường dẫn bị đổi giữa lúc kiểm và lúc dùng; tính nguyên tử của ghi tệp (`Cap<Fs>::write` có thể để lại tệp cụt).
- **Đặc tả sai / thiếu**: kiểm tra chỉ so với những gì được nêu; thông dịch MIR là công cụ đối chiếu NGOÀI TCB.
- **Windows**: Windows x86_64 (MSVC) đã kiểm bằng cổng liền mạch `89438fd` (rc3, 2026-11-18): GREEN 1027/0/206. **Vẫn CHƯA kiểm:** CI GitHub (chưa chạy), capstone K1/K2/K4 trên Windows, VS Code thật, `u128` nhân/chia ở backend LLVM (bị từ chối có chủ ý). Linux: số liệu dòng Seed 2026-10-22/23 (cổng 958/0).

## 4. Cơ sở tin cậy (TCB): phần nếu sai thì mọi đảm bảo trên mất giá trị
1. Chính các bộ kiểm: `gt-types`, `gt-mir`, `gt-borrowck`, `gt-oblig`, `gt-term`/effects/conc, `gt-secrets`, bộ xác minh verdict.
2. Backend `gt-codegen-llvm` và **IR Guard** (`gt-guard`) — chỗ duy nhất có thể biến chương trình đã kiểm thành mã máy không an toàn.
3. **Intrinsic và runtime** (`lib/core/prelude.gt` chỉ có chữ ký; thân do backend/`runtime/gt-rt` hạ): danh sách đầy đủ và hợp đồng từng cái ở `docs/TCB_INTRINSICS.md` — nhóm **CAO** (chạm bộ nhớ): `Arena`, `Vec`, `Text`, lát/`&str`, `Atomic`, `Mutex`, `spawn`, `Cell`, I/O (`read_all`, `write`, `File`), `&str == &str` (`gritrt_bytes_eq`).
4. LLVM 20.x, `cc`/`ld`, libc, nhân hệ điều hành.
- **Trạng thái duyệt TCB:** hợp đồng từng intrinsic do tác giả viết và kiến trúc sư (subagent/ủy quyền) duyệt; **chưa có người thứ hai đọc MÃ độc lập** cho nhóm CAO (`docs/TCB_REVIEW_GUIDE.md`). Khi S7 hoàn tất, trạng thái sẽ ghi "đã qua đọc mã đối kháng bằng subagent cùng họ model" — đó không phải kiểm toán độc lập của con người.
- Thư viện chuẩn `lib/std` viết bằng Grit THƯỜNG và đi qua cùng cổng: nó KHÔNG thuộc TCB (một lỗi ở đó là lỗi chương trình, được bắt hoặc không bởi cổng như mã người dùng).

## 5. Cách xác minh bạn tự làm được
```text
bash tools/gate.sh .            # cổng đầy đủ (fmt, clippy -D warnings, audit, mọi test, tài liệu, NUL, xoá tệp, theo dõi)
python3 tools/doc_test.py       # mọi khối mã trong tài liệu phải biên dịch đúng như chú thích
gritc doctor                    # môi trường (LLVM, runtime, trình liên kết, quyền ghi)
```
Tái lập: hai lần dựng cùng mã cho cùng SHA-256 (xem `BUILDING.md`).

## 6. Ngôn ngữ chúng tôi cam kết không dùng
Dự án không dùng các cụm "bất tử", "vĩnh cửu", "an toàn tuyệt đối" hay "đã chứng minh hình thức" để mô tả Grit. Nếu bạn thấy cụm như vậy ở đâu đó trong repo, đó là lỗi tài liệu: hãy báo.

## Mức bằng chứng về đọc mã (GÓI 4 S1.a, 2026-11-19)
**TSan (Linux, build phân tích): không phát hiện đua dữ liệu trong 8 mẫu × 50 lần (không phải "không có đua dữ liệu"; bộ nhớ yếu và Windows chưa kiểm). An toàn được kiểm bằng corpus, oracle, đột biến và đọc mã cùng họ model; chưa có đọc mã độc lập khác họ ngoài hai vùng nhỏ (`runtime/gt-rt` và `crates/gt-guard`, ≈ 2 650 dòng, tại hash G1.5 `c5aa45b`).** Không có đánh giá an toàn độc lập của con người; không dùng chữ "kiểm toán".
