# CALL_EDGE_AUDIT — đệ quy qua cạnh gọi gián tiếp (GÓI 8 S1a, 2026-11-19)

Mục tiêu: kiểm mọi nơi một hàm có thể được GIỮ rồi gọi lại (container) mà đệ quy không có `↓` chứng minh được hoặc `div` (D12) phải bị ĐỎ.
Phương pháp: `tools/call_edge_probe.py` sinh chương trình, chạy `gritc check --json` (CHỈ kiểm; KHÔNG chạy chương trình, nên không có rủi ro cạn tài nguyên). Bản dựng: `target/debug/gritc.exe` sau sửa FB-C34 (commit của GÓI 8). Số liệu thô: `docs/CALL_EDGE_RESULTS.jsonl`.

## 1. Bảng tổ hợp (mỗi dòng một lần chạy thật)
| container | hình dạng | phán quyết | mã | ghi chú |
|---|---|---|---|---|
| biến cục bộ `μh ≔ f; h(n)` | trực tiếp (f đưa chính f) | đỏ | E0703 | |
| biến cục bộ | tương hỗ f→g→f qua container | đỏ | E0703 | |
| tham số `app(h, n)` | trực tiếp | đỏ | E0703 | |
| tham số | tương hỗ | đỏ | E0703 | |
| trường struct `S { h: f }` rồi `μk ≔ s.h; k(n)` | trực tiếp | đỏ | E0703 | `(s.h)(n)` không viết được: E0309 |
| trường struct | tương hỗ | đỏ | E0703 | |
| `Option<ƒ>` + `⌥ Some(h) ↦ h(n)` | trực tiếp | đỏ | E0703 | |
| `Option<ƒ>` | tương hỗ | đỏ | E0703 | |
| tuple `(f, 1u32).0(n)` | trực tiếp | đỏ | E0703 | |
| tuple | tương hỗ | đỏ | E0703 | |
| hàm trả hàm `pick() → (ƒ) ! ∅ { f }` | trực tiếp | đỏ | E0703 | cần `! ∅` sau kiểu trả về |
| hàm trả hàm | tương hỗ | đỏ | E0703 | |
| generic `id<T>(x: T)`, `μh ≔ id(f); h(n)` | trực tiếp | đỏ | E0703 | (thí nghiệm riêng) |
| static `■ H: ƒ… ≔ f;` gọi `H(n)` | trực tiếp | đỏ | E0703 | (thí nghiệm riêng) |
| `spawn(w, n)` với w gọi lại (qua hàm trung gian) | tương hỗ | đỏ | E0703 | bộ sinh `spawn_family`, 5 hình dạng × 4 độ sâu |
| `spawn(id(f), n)` qua generic | trực tiếp | **đỏ sau sửa** (trước sửa: XANH = FB-C34) | E0703 | xem FB-C34 |
| `{detach}` do người dùng viết | — | không viết được | E1002 | chỉ prelude được dùng hiệu ứng này |

**Tổ hợp không viết được:** `Vec<ƒ>` (cần `Vec::new(&arena)` + hiệu ứng `{mem}`; đọc phần tử qua `get` và `⌥` cho E0305/E0301 khi thử; CHƯA viết được chương trình hợp lệ) — ghi nhận, CHƯA KIỂM.
**Chưa thử:** `Cell`/`Atomic` chứa hàm, trait method trả hàm, `HashMap` giá trị hàm, `Text`.

## 2. Phát hiện
- **FB-C34 (NGHIÊM TRỌNG, đã sửa trong GÓI 8):** `spawn(id(f), n)` với `f` gọi lại `spawn` qua generic được CHẤP NHẬN. Đã sửa: callee không có thân có tham số kiểu hàm và nhận đối số có thể mang hàm ⇒ cạnh gián tiếp tới mọi hàm bị thoát (xem `docs/FIELD_BUGS.md`).
- Sửa đầu tiên quá rộng (mọi đối số không phải hằng ⇒ đồ thị bảo thủ) làm `E0502_ok_*` (chương trình ĐÚNG) bị E0703 oan. Đã thu hẹp: chỉ đối số kiểu hàm / tham số chưa biết / có phép chiếu, và chỉ callee có tham số kiểu hàm. Kiểm: `m8_corpus` xanh, toàn bộ `gt-term` xanh (13 bộ, exit 0).

## 3. CHƯA KIỂM / giới hạn
- S1c đã chạy (mục 4). Chưa làm: 2 dòng không viết được (static khả biến, Text/HashMap len).
- Bộ sinh spawn không chạy thông dịch (oracle với `spawn` chưa có); "tự bắt khi tắt bản sửa" CHƯA kiểm bằng tay.
- Intrinsic không thân nhận hàm NẰM TRONG cấu trúc (tuple/struct) không được phát hiện qua chữ ký (giới hạn đã ghi trong `gt-term`).
- Cổng đầy đủ GREEN trên `7fa8647` (rc5 + FB-C34); xem RUN_REPORTS GÓI 8.

## 4. S1c — biến thể phụ thuộc trạng thái (GÓI 8; mỗi dòng một lần `gritc check`, bản dựng debug sau FB-C34)
| # | chương trình | kỳ vọng | kết quả |
|---|---|---|---|
| 1 | `⟳ c.get() > 0 ↓ c.get() { c.set(c.get()+1) }` (Cell) | ĐỎ | ĐỎ E0702 (+N0601) |
| 2 | `⟳ A.load(SeqCst) > 0 ↓ A.load(SeqCst) { A.store(9, SeqCst) }` (Atomic) | ĐỎ | ĐỎ E0702 |
| 3 | static khả biến | — | không viết được (static phải `Sync`, không `μ`; E0501/E0308 của audit 01 a08) |
| 4 | `⟳ v.len() < 10 ↓ v.len() { v.push(1) }` (Vec) | ĐỎ | ĐỎ E0702 |
| 5 | Text/HashMap len khi thân thêm | ĐỎ | không viết được trong thí nghiệm này (CHƯA KIỂM) |
| 6 | `⟳ *r > 0 ↓ *r { *r ← *r + 1 }` (qua `&!`) | ĐỎ | ĐỎ E0702 (+N0601) |
| 7a | `↓ m { m ← m -% 1 }` không bảo vệ | ĐỎ | ĐỎ E0702 |
| 7b | `↓ m { m ← m -^ 1 }` (bão hoà) không bảo vệ | ĐỎ | ĐỎ E0702 |
| 7c | `⟳ m > 0 ↓ m { m ← m -% 1 }` có bảo vệ | XANH | XANH (đúng) |
| 8 | biến thể/điều kiện là kết quả hàm `g(i)` | ĐỎ | ĐỎ E0702 (+N0601) |
| 9 | `↓ i * m` (m có thể 0) | ĐỎ | ĐỎ E0702 |
**Kết luận:** 0 chương trình XANH sai trong 9 dòng; không có FB-C mới từ S1c.
