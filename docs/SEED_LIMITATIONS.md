# SEED_LIMITATIONS — Seed 1.0 chưa làm gì, và cách né

Tài liệu cho NGƯỜI DÙNG (và AI viết Grit). Chi tiết kỹ thuật, bằng chứng và lịch sử: `docs/KNOWN_LIMITATIONS.md`. Cái Grit đảm bảo/không đảm bảo: `docs/SAFETY_CLAIMS.md`.

## Nền tảng và môi trường
- **Linux x86_64 (dòng Seed, 2026-10-23) và Windows x86_64 MSVC (cổng `89438fd`, rc3, 2026-11-18: GREEN 1027/0/206) đã kiểm.** Chưa kiểm: CI GitHub (chưa chạy), capstone K1/K2/K4 trên Windows, VS Code thật. `gritc` nhớ đường dẫn tuyệt đối của kho (runtime `libgt_rt.a`): đừng di chuyển kho sau khi dựng.
- Mỗi lần sửa mã gói Rig phải `rig lock`. `rig new` ghi đường dẫn tuyệt đối `lib/std` vào `rig.toml`.
- Hồ sơ `release`/`release-o2` có bằng chứng THỰC NGHIỆM tương đương `dev` (so ba nguồn, capstone ở dev và release-o2), không phải chứng minh.

## Ngôn ngữ (chưa có; cách né)
| chưa có | cách né |
|---|---|
| closure, macro, `dyn`, `async`, `unsafe` cho người dùng, `f32`, tham số lifetime lồng | hàm có tên + con trỏ hàm (không bắt biến); tách hàm; `f64`; vùng đời tường minh một cấp |
| toán tử `?`, phân rã tuple `(a, b) ≔ f()` | `⌥ r { Ok(v) ↦ …, Err(e) ↦ … }`; `t ≔ f(); a ≔ t.0;` (cả bốn capstone viết được không cần chúng) |
| `Option::unwrap`, `is_some`, `Cap<Fs>::sub` chạy được | `⌥`, `unwrap_or`, `is_none` (std). `check` XANH nhưng backend từ chối (mã 3) |
| `==` trên tuple/`&T`/Option/struct/lát | so từng trường; `*a == *b`; `cmp(..).is_eq()`; `[u8]::equals` |
| bộ giải không chứng minh được bước PHI TUYẾN (`x ← x * 2` tiến tới cận); `x / k`, `a % b` thì ĐƯỢC từ GÓI 12 (ADR-C16) | nhiên liệu (`core::fuel_left`) hoặc ngân sách độ sâu |
| mảng `.len()` | `&a[0usize..N]` hoặc truyền `&a` vào hàm nhận `&[T]` (ép unsize có từ GÓI 12) |
| `u128` trong `⌥` thiếu nhánh `_` | luôn thêm `_` |

## Thư viện
- `Vec`/`HashMap`: phần tử phải `Copy`/không cần drop; không free từng phần (chỉ `Arena::reset`). `HashMap` thứ tự duyệt không xác định. `[T]::sort` là heapsort, KHÔNG ổn định.
- stdout không đệm (mỗi `write_*` là một lời gọi hệ thống): đầu ra lớn nên gom vào `Vec<u8>`. Ca lớn chậm hơn bản Rust ~×5 ở `dev` (xem `docs/CAPSTONE_LOG.md`).
- Không có: chuỗi định dạng, số thực in/phân tích, BTreeMap, đọc stdin theo dòng (đọc toàn bộ rồi tách), thời gian ngoài `Cap<Clock>::now`.

## An toàn (xem SAFETY_CLAIMS)
- Âm tính giả của các bộ kiểm có thể còn: đợt đọc mã đối kháng + quét fail-open của S7 vẫn tìm thêm 13 lỗ hổng thật (FB-C15…C27) mà corpus/oracle/đột biến trước đó KHÔNG bắt — ngưỡng tin cậy là "đã qua đọc mã đối kháng bằng subagent cùng họ model", KHÔNG phải "đã duyệt" hay chứng minh; fuzz (lexer/parser/chuỗi kiểm) mới ở mức "đang tích luỹ" (≪ 24 giờ); đột biến M7 nhóm nguy hiểm: 30/60 giết bằng test + 30 tương đương theo lập luận đọc mã (`docs/M7_DANGER_MUTANTS.md`). CHƯA có người thứ hai là con người.
- Abort có kiểm soát (mã 70) là dừng chương trình: an toàn bộ nhớ nhưng có thể là từ chối dịch vụ.
- Data race: không có oracle động đầy đủ; chỉ corpus + lặp.
- `spawn` và `main` đều dùng ngăn xếp 256 MiB (G1.5, FB-C29). Mảng cục bộ/đệ quy vượt cỡ này vẫn có thể bị HĐH giết; 256 MiB được reserve cho mỗi worker nên tạo nhiều luồng có thể cạn tài nguyên. Dùng `Arena`/`Vec` cho vùng lớn.
- Ghi vào stdout đã đóng (`>&-`) thành công im lặng; tên tệp nguồn `gritrt_*.gt` bị IR Guard từ chối (E1101): đổi tên tệp.
- `check` xanh nhưng `build/run` đỏ (mã 3) cho vài hàm prelude chưa có lowering (`Host::fs`, `Cap<Fs>::sub`, `Cap<Net>::connect`, `Cap<Env>::var`, `Cap<Clock>::now`) và cho chương trình có hai `main`.
- Hợp đồng `⊳`/`⊲` ở thành viên trait/impl được gọi qua tham số kiểu (`t.f(x)` với `T: Tr`) bị E0901 (không có điểm gọi cụ thể để kiểm): gọi trên kiểu cụ thể hoặc để thân tự kiểm. Vòng phụ thuộc khởi tạo giữa các `■` (kể cả qua hàm) bị từ chối (mã 3).

## Công cụ
- `gritc lsp`: chẩn đoán + định dạng toàn tài liệu; chưa có hover/đi tới định nghĩa/hoàn thành mã. `editors/vscode`: khung chưa thử trong VS Code thật.
- `gritc check` văn bản ẩn ghi chú `N…` (xem bằng `GRIT_NOTES=1` hoặc `--json`).
