# CLOUD_GATE_PROMPT — lời nhắc cổng Linux cho một phiên Claude Code ĐÁM MÂY (GÓI 8 S4.d)

Dùng khi GitHub Actions còn bị khoá (HTTP 422). **Giang** mở một phiên Claude Code chạy trên máy Linux đám mây, dán nguyên văn khối dưới đây, rồi dán KẾT QUẢ vào `docs/CLOUD_GATE_RESULT.md` (agent của kho này KHÔNG tự điền tệp đó). Thời gian kỳ vọng: cổng Linux đầy đủ trên CI trước đây ≈ 46–74 phút (+ dựng ban đầu ≈ 15–25 phút).

## Khối để dán (tự chứa)
```text
Bạn là phiên Claude Code chạy trên Linux đám mây. Nhiệm vụ DUY NHẤT: chạy cổng đầy đủ của Grit và báo kết quả. KHÔNG sửa mã. KHÔNG sửa tệp theo dõi. KHÔNG push. KHÔNG tạo cron/lịch.
1. git clone https://github.com/gn3305970-lang/Grit.git grit && cd grit && git checkout windows-work   # (hoặc commit Giang chỉ định: git checkout <hash>)
   In ra: git rev-parse HEAD ; git status -s  (phải sạch).
2. bash tools/setup_cloud.sh          # cài toolchain/LLVM 20 theo tệp (đọc nó trước khi chạy; nếu nó cần quyền root và bạn không có, dừng và báo)
3. bash tools/preflight.sh .          # phải in PREFLIGHT=OK; không OK thì DỪNG và dán nguyên văn đầu ra
4. export CARGO_BUILD_JOBS=2 ; bash tools/gate.sh .   # MỘT lần liền mạch, đầy đủ; đừng chạy hai cargo cùng lúc
5. Dán NGUYÊN VĂN các dòng: GATE_RESULT=..., GATE_TREE=..., mọi dòng `GATE_RED:`, mọi dòng `TEST_UNIT ...`, và `FAIL_OPEN_SCAN`, `DOC_TEST`, `ADVERSARIAL`, `NUL_OK`, `DELETIONS_OK`, `TRACK_OK`.
6. Tính tổng: số đơn vị TEST_UNIT, tổng passed, tổng failed, tổng suites.
7. Nếu GATE_RESULT=RED: KHÔNG sửa; dán 40 dòng log quanh dòng đỏ đầu tiên và dừng.
Không in bí mật nếu gặp. Không gửi dữ liệu ra ngoài ngoài khung trò chuyện này.
```

## Cách đọc kết quả (cho Giang và cho agent ở phiên sau)
- Hợp lệ khi: `GATE_RESULT=GREEN`, `GATE_TREE` do phiên đám mây in ra **bằng** `GATE_TREE` của cùng commit mã (so với `docs/RUN_REPORTS.md`; băm gồm HEAD, `git diff HEAD`, tệp chưa theo dõi nên PHẢI bằng nhau nếu cây sạch cùng commit), và số test Linux = số test Windows trừ 3 test chỉ-Windows (`docs/PLATFORM_TESTS.md`).
- Điều kiện đẩy `main` (AGENTS.md): CẢ cổng Windows xanh VÀ cổng Linux xanh trên cùng cây mã. Kiểm phần chỉ-tài-liệu sau đó bằng `bash tools/main_check.sh <commit xanh> <đầu>`.
