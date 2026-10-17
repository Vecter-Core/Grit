# RUN_REPORTS — báo cáo ngắn sau MỖI mục của RUNLIST (số THẬT; cái gì chưa kiểm)

Quy ước: mỗi mục một tiểu mục `## R<n> — <trạng thái> (ngày)`; nêu việc đã làm, số liệu thật, lỗi thật tìm được (FB-n), cái KHÔNG làm/chưa kiểm, chỗ lệnh sai kỹ thuật + phương án fail-closed đã chọn.

## Ghi chú kỹ thuật về lệnh (không phải mục)
- **Cổng liền mạch vs "không tiến trình nền":** giai đoạn `test` của `tools/gate.sh` chạy `cargo test --workspace` mất ~30–45 phút, vượt giới hạn 10 phút của một lệnh công cụ; không thể chạy "đồng bộ ≤ 15 phút" mà vẫn là MỘT cổng liền mạch. Phương án fail-closed: cổng là NGOẠI LỆ DUY NHẤT chạy tách (WMI, stdin từ /dev/null); phiên chết giữa chừng ⇒ cổng không xanh ⇒ chạy lại từ đầu. Mọi việc nặng khác theo lô đồng bộ ≤ 8 phút (`--max-seconds 480`).
