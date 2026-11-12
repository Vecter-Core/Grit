# Thuật ngữ

- **Arena:** kiểu vùng nhớ của ngôn ngữ Grit trong `lib/core`; không phải công cụ hay AI bên ngoài.
- **Arena chia sẻ:** `Vec`/`Text` hiện giữ `&'a Arena` dùng chung, theo ADR-C03; `Arena::reset` vẫn cần `&!self` và bị borrow checker chặn khi còn khoản mượn. Không nhầm với các mở rộng vùng nhớ/ownership khác có thể được xem xét ở Phase 2.
