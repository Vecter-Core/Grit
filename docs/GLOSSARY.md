# Thuật ngữ

- **Arena:** kiểu vùng nhớ của ngôn ngữ Grit, được khai báo trong `lib/core`; không phải công cụ hay AI bên ngoài.
- **Arena chia sẻ:** ĐÃ CÀI (ADR-C03, nhánh Linux, đã hợp nhất): `Vec<'a, T>`/`Text<'a>` giữ `&'a Arena` dùng chung (xem `lib/core/prelude.gt`), nên nhiều `Vec`/`Text` có thể cùng một Arena. `Arena::reset` vẫn cần `&!self` nên borrow checker chặn khi còn `Vec`/lát/khoản mượn sống. Quyết định cũ "hoãn Arena chia sẻ" ĐÃ BỊ THAY THẾ bởi ADR-C03; việc còn lại là kiểm bằng chứng soundness (điều kiện cứng #9, mutability bên trong). Không nhầm với các mở rộng vùng nhớ/ownership khác có thể xem xét ở Phase 2.
