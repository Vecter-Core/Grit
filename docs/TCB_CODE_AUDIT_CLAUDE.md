# R11b — đọc mã đối kháng phần TCB do Codex viết (reviewer: Claude Sonnet 5.5, 2026-11-17)

**Reviewer:** Claude Sonnet 5.5 (model họ Claude). **HEAD đọc:** `f9f0f78` (nhánh `integration`), diff gốc `c3ba7de`. Đây là đọc mã + thí nghiệm có mục tiêu, **không phải chứng minh**. Lưu ý trung thực: cùng *họ* với phần lớn mã Claude cũ nhưng KHÁC họ với Codex/GPT-6 (tác giả diff được duyệt); tôi cũng là tác giả của vài commit trong phiên này (FB-C31, test Guard) — các commit đó CHƯA được ai độc lập đọc.

## Tóm tắt
- Nghi vấn trong phạm vi: 4 mục FB (C28, C29, C30, C6) + vùng giải xung đột merge + 13 nghi vấn của sổ G2.2 = **17 mục**.
- **Xác nhận (có phản ví dụ / lỗi): 1** — FB-C31 (mới, nhẹ, không đạt tới từ Grit). **Bác bỏ trong phạm vi thí nghiệm: 9. Chưa kiểm / giới hạn còn mở: 7.**
- **Không có lỗi buộc dừng phát hành.** Không tìm thấy đường hỏng bộ nhớ đạt tới từ Grit.

## Bảng FB-C28 / C29 / C30 / C6
| Mục | Đã đọc | Thí nghiệm | Kết luận |
|---|---|---|---|
| FB-C28 `1c7deef` (`ffi_bytes`, fs_*) | toàn bộ diff + `grep from_raw_parts` toàn runtime | unit test `oversized_ffi_lengths_are_rejected_before_from_raw_parts` ĐỎ trước (bản debug dừng "unsafe precondition violated") rồi xanh | **XÁC NHẬN thiếu sót nhất quán → FB-C31**: bốn đường khác (`write_str`, `ewrite_str`, `utf8_valid`, `bytes_eq`) không dùng `ffi_bytes`; nay đã dùng. Không đạt tới từ Grit (độ dài lát bị chặn bởi cấp phát). `ffi_bytes` không kiểm `p != null` khi `n>0` (hợp đồng ghi ở `# Safety`; codegen bảo đảm) — giới hạn |
| FB-C29 `9da739e`, `7080925` (stack 256 MiB, main fail-closed) | diff + `gritrt_start/spawn` | (1) 300 luồng cùng sống: chạy đúng, trả 300; (2) **3000 luồng cùng sống làm cạn tài nguyên máy** (bash không `fork` được, bộ nhớ trống còn ~0,7 GB) — kết quả chương trình không thu được | **Bác bỏ lỗi logic; XÁC NHẬN giới hạn tài nguyên**: 256 MiB × nhiều luồng nguy hiểm (đã ghi `SEED_LIMITATIONS`). `spawn` thất bại ⇒ `abort(0)` không kèm thông báo (đóng nhưng khó chẩn đoán). Tràn quá 256 MiB ⇒ HĐH giết tiến trình (không phải abort mã 70) — chưa kiểm |
| FB-C30 `8eb17ba` (allowlist intrinsic Guard) | `allowed_llvm_intrinsic`, nhánh 3b, `runtime_address_violations` | test mới `every_unlisted_llvm_intrinsic_family_is_rejected`: 16 họ nguy hiểm (`ctlz/cttz` cờ poison, `abs`, `bswap`, `ctpop`, `bitreverse`, `fabs`, `minnum`, `memset`, `memmove`, `expect`, `trap`, `ptrmask`, `is.constant`, `umul.fix`, `smul.fix.sat`) — guard 36/36 | **Bác bỏ trong phạm vi**: danh sách đóng, đúng. Giới hạn: allowlist theo TÊN; `llvm.memcpy` cho phép nhưng Guard không kiểm chồng lấn vùng (verifier cũng không) |
| FB-C6 `a5c4295`, `f000546`, `cd9a0cf` (⊳ qua giá trị ⇒ E0901) | `escaped_sites`, driver | 15 cách đưa hàm có `⊳` thành giá trị: trực tiếp, biến, tham số, mảng, tuple, struct, `Option`, generic, trả về từ hàm, `■` tĩnh, `Kiểu::hàm`, ép kiểu `as`, nhánh `⎇`, `[g; n]`, thành viên trait qua ràng buộc generic | **Bác bỏ trong phạm vi**: 15/15 bị E0901 |

## Vùng giải xung đột `c3ba7de` (`git diff-tree --cc`)
- `gt-types/check/call.rs`, `check/mod.rs`: hợp nhất `callee_pos` (E0313) với `CapUse` qua giá trị hàm (F-05) và `eq_deferred` (FB-C19): hai trường độc lập, không chồng chéo. **Bác bỏ** (đọc + corpus `E0313_*`, `E0310_*`).
- `gt-types/validate.rs` (Copy bounds): hai nhánh độc lập tìm CÙNG một lỗi (FB-17 của Windows ≡ FB-C16 của Linux); bản hợp nhất giữ phiên bản dùng `scope_bounds` của impl — `copy_impl_bounds` 4/4. **Bác bỏ**.
- `gt-borrowck/regions.rs`, `gt-types/db.rs`, `gt-oblig/driver.rs`: không có hunk xung đột thật ngoài hai mục trên (`diff-tree --cc` rỗng); luật vùng mới (WF) đã được xử lý riêng ở G2.1.c (ADR-C08, đối chứng dương `MNoWf`).
- `pick_effects` (`83a3e0e`): snapshot lệch ⇒ `?unknown` (đóng). **Bác bỏ**.

## Sổ 13 nghi vấn G2.2
| ID | Kết quả của reviewer |
|---|---|
| H10-01 argv lossy | Đọc `encode_args`: `to_string_lossy` không đơn ánh (hai argv không hợp lệ khác nhau cùng ra một chuỗi U+FFFD) nhưng luôn là UTF-8 hợp lệ ⇒ **bác bỏ lỗi bộ nhớ/UTF-8**; hành vi lossy là giới hạn có chủ ý. Chưa kiểm end-to-end qua shell |
| GUARD-01 | **Bác bỏ trong phạm vi** (bảng FB-C30) |
| RT-01 | **Xác nhận một phần** → FB-C31 |
| THREAD-01 | Bác bỏ logic; giới hạn tài nguyên (bảng FB-C29) |
| ARENA-01 | **Chưa kiểm** tổng quát mọi kích thước/căn lề; bằng chứng thêm: `shared_arena_diff` 320/320 (dev có `arena_overlap` BẬT) |
| VEC-01 | **Chưa kiểm** (không ép được tràn với kiểu hiện hỗ trợ) |
| SLICE-01 | Bác bỏ trong phạm vi (FB-C20 `slice_stale_len`, 902 ca) |
| UTF8-01 | Bác bỏ trong phạm vi (100 000 chuỗi so `from_utf8` của Rust) |
| ATOMIC-01 | **Chưa kiểm**: không TSan/loom |
| CELL-01 | Bác bỏ trong phạm vi (`interior_mut`, `fb_c17`) |
| GUARD-02 | **Chưa kiểm** (đếm abort theo (hàm, loại), không theo vị trí) |
| ATTEST-01 | **Chưa kiểm** (attestation là chuỗi metadata, không ràng buộc binary cuối) |
| UNSAFE-01 | **Chưa kiểm** (chưa có kiểm kê `unsafe` tự động) |

## Trạng thái dòng TCB
Không có dòng nào được ghi "ĐÃ QUA ĐỌC MÃ ĐỐI KHÁNG BẰNG MODEL KHÁC": các dòng có phần do Codex viết (`gt-rt` FFI/stack, `gt-guard` allowlist) đã được tôi đọc nhưng các sửa của CHÍNH tôi trong phiên này (FB-C31, test guard) và 7 mục "chưa kiểm" ở trên khiến chưa nên nâng trạng thái. Không đổi `TCB_INTRINSICS.md` ở mục này.

## Lỗi/giới hạn mới
- **FB-C31** (xem `docs/FIELD_BUGS.md`, commit `262008e` đỏ → `f9f0f78` xanh).
- Máy: chạy 3000 luồng 256 MiB làm cạn tài nguyên hệ thống — nên có trần số luồng/thông báo rõ khi `spawn` thất bại (đề xuất, chưa làm).
