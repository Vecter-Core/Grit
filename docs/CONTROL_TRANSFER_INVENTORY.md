# CONTROL_TRANSFER_INVENTORY — kiểm kê phía MIR: mọi nơi có thể chạy mã người dùng (GÓI 11 S1.f; 2026-11-20)

Mục tiêu: liệt kê mọi cấu trúc MIR/giai đoạn có thể THAM CHIẾU một hàm hoặc CHẠY mã người dùng, và `gt-term` (kiểm tra dừng, D12) xử lý nó thế nào. Mọi hàng KHÔNG XỬ LÝ mà đạt tới được từ chương trình Grit đều phải có thí nghiệm. Nguồn: đọc `crates/gt-term/src/lib.rs` (HEAD sau FB-C35) và `crates/gt-mir/src/mir.rs`; thí nghiệm chạy bằng `gritc` (Windows, bản dựng dev từ cây có bản sửa FB-C35). Nhãn: ĐÃ XỬ LÝ / BẢO THỦ / KHÔNG XỬ LÝ / KHÔNG ÁP DỤNG.

## Bảng
```text
#  cấu trúc                                        gt-term                  tệp:dòng (HEAD)                    thí nghiệm / test
1  Terminator Call + Callee::Fn (có thân)          ĐÃ XỬ LÝ (cạnh trực tiếp) lib.rs instance_graph ~400, conservative ~600   instance_graph.rs, termination.rs
2  Callee::Fn không thân (intrinsic: spawn,        ĐÃ XỬ LÝ: cạnh gián tiếp tới  lib.rs ~410-430 (FB-C33/34)       spawn_recursion.rs (bộ sinh 5 hình dạng);
   Mutex::with) nhận hàm làm đối số                  mọi Const::FnItem đối số;                                      mutants_goi11 (spawn)
                                                    đối số động ⇒ lùi đồ thị bảo thủ
3  Callee::TraitMethod                             ĐÃ XỬ LÝ (instance: cạnh    lib.rs ~432-448; conservative ~650   instance_graph.rs; mutants_goi11
                                                    gián tiếp; không giải được ⇒ bảo thủ)
4  Callee::Value (gọi hàm giá trị)                 ĐÃ XỬ LÝ (cạnh gián tiếp tới lib.rs ~450-460                      recursion_through_each_edge_kind (value, tham số)
                                                    mọi hàm "bị thoát")
5  Const::FnItem trong Use / Cast / Repeat /       ĐÃ XỬ LÝ (tập `escaped` +    lib.rs check_termination ~753-785    instance_graph.rs (FB-C15 Repeat); termination.rs
   Aggregate                                         discover instance)
6  Const::FnItem trong BinaryOp/Unary/Overflow/    KHÔNG ÁP DỤNG: kiểu hàm      (type-check M4)                      —
   Ref/Discriminant/Len                              không thể là toán hạng
7  Terminator SwitchInt/Goto/Return/Unreachable/   KHÔNG ÁP DỤNG (không chạy    mir.rs TerminatorKind               —
   Assert                                            mã người dùng)
8  Terminator Drop (glue)                          KHÔNG ÁP DỤNG ở Phase 1:     `⊐ Drop for` chỉ viết trong lib/core  z/x1: E0313; x5: E0313
                                                    mã người dùng không viết được Drop
9  THÂN HỢP ĐỒNG ⊳ / ⊲ (BodyKind::Pre/Post)        **KHÔNG XỬ LÝ trước GÓI 11 ⇒ FB-C35**;  lib.rs ~700-720 (`contracts`)    contract_recursion.rs (6 test; bộ sinh
                                                    nay ĐÃ XỬ LÝ (cạnh gộp vào nút                                 160 chương trình); tests/fail/E0703_recursion_through_{pre,post}condition.gt
                                                    của hàm sở hữu)
10 Biến thể ↓ trong chữ ký                         KHÔNG ÁP DỤNG: chỉ dạng    lib.rs Tx::term                      E0702 (không phải tuyến tính)
                                                    tuyến tính theo tham số/len, không gọi hàm
11 Bất biến vòng ⋈ (trong thân hàm)                ĐÃ XỬ LÝ: là một phần thân  (MIR hàm)                            z5.gt: đệ quy f→(⋈ g)→f ⇒ E0703 (ĐO)
                                                    MIR của hàm
12 Khối `⚠` (unsafe) với `⊢`                        KHÔNG ÁP DỤNG ở người dùng   grammar: `⚠` chỉ lib/core           —
13 Khởi tạo static/const (BodyKind::Static/Const)  KHÔNG XỬ LÝ ở gt-term (không  lib.rs `bodies` chỉ BodyKind::Fn    z1.gt: `■ X ≔ f()` với f đọc X ⇒ check XANH;
                                                    nằm trong đồ thị gọi)                                          LLVM: "chưa hỗ trợ: vòng phụ thuộc khởi tạo giữa
                                                    nhưng backend TỪ CHỐI                                          các static" (fail-closed, KHÔNG sinh binary);
                                                                                                                   thông dịch: lỗi nội bộ. Hàm gọi trong initializer
                                                                                                                   mà đệ quy tự nó vẫn bị E0703 (x2.gt)
14 Phương thức mặc định trong ⊏ (thân trait)        KHÔNG ÁP DỤNG: không viết được   z3.gt: E0101                      —
15 Toán tử người dùng, closure, iterator tuỳ chỉnh  KHÔNG ÁP DỤNG ở Phase 1       spec: không có                       —
16 Đồ thị instance (đơn hình hoá)                  ĐÃ XỬ LÝ (+ cận nút, lùi    lib.rs ~293-540                      graph_limit_is_exact_to_the_node;
                                                    bảo thủ)                                                          conservative_fallback.rs
17 Hàm nằm TRONG cấu trúc (tuple/struct/Vec)       KHÔNG XỬ LÝ (giới hạn đã     lib.rs may_hold_fn                   docs/KNOWN_LIMITATIONS.md; CALL_EDGE_AUDIT.md
   rồi đưa cho intrinsic không thân                  ghi): chỉ chữ ký kiểu hàm
18 Thân intrinsic (gritrt_*), runtime             NGOÀI gt-term (TCB runtime)  —                                    TCB_INTRINSICS.md
```

## Phát hiện
- **FB-C35 (hàng 9):** nghiêm trọng, đã sửa trong GÓI 11 (commit `7c6334c`); xem `docs/FIELD_BUGS.md`.
- **Hàng 13 (static/const):** `gritc check` XANH nhưng backend LLVM từ chối (không sinh binary) và thông dịch báo lỗi nội bộ ⇒ không có chương trình không dừng chạy được; nhưng `check` XANH cho một vòng khởi tạo là không nhất quán với D12; ghi `docs/KNOWN_LIMITATIONS.md` và `docs/NEXT_PRODUCT_BATCH.md` (kiểm đồ thị khởi tạo static ở M8/M4). KHÔNG sửa trong gói (không phải âm tính giả chạy được).
- **Số hàng KHÔNG XỬ LÝ còn lại:** 3 (hàng 13, 17; hàng 9 đã xử lý). Hàng 17 là giới hạn đã biết từ GÓI 8.

## CHƯA KIỂM
- Hàng 11 (bất biến ⋈) mới thử MỘT chương trình (z5); hàng 13 chưa thử static khả biến (không viết được theo audit 01 a08); thí nghiệm hàng 8 chỉ qua E0313 (cấm viết). Nền: Windows; Linux CHƯA chạy thí nghiệm riêng (cổng Linux rc6 xanh không chứa các test mới).
