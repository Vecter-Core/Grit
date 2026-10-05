# Việc cần làm: tài liệu mã lỗi (đơn giản, lặp lại — giao cho AI khác)
Với MỖI mã trong `docs/spec/error_codes.md` tạo `docs/errors/<mã>.md`: tiêu đề, mức (ĐỎ/VÀNG/GHI CHÚ),
giải thích 2–3 câu, ví dụ sai (copy từ tests/fail hoặc tests/warn), cách sửa. Không bịa hành vi ngoài `semantics.md`.
Cuối mỗi file ghi rõ: cần ≥3 test fail + ≥3 test pass khi mã được triển khai (plan §0.5).
