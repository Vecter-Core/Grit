# CROSS_REVIEW_COVERAGE (tạo bằng `python tools/cross_review_coverage.py`)

> **CHỈ DÙNG CLAUDE (GÓI 4, 2026-11-19):** Giang chỉ dùng Claude; không dùng GPT/Codex nữa. Đọc chéo KHÁC HỌ model chỉ tồn tại cho đúng hai vùng: `runtime/gt-rt` và `crates/gt-guard` (≈ 2 650 dòng) do GPT-6 đọc ở G1.5 tại hash `c5aa45b`; MỌI thay đổi sau hash đó (theo `git blame`) quay về "CHƯA đọc chéo". Toàn bộ phần còn lại (≈ 63 000 dòng: checker M6/M7/M8, kiểu, MIR, hiệu ứng, driver, codegen) chỉ có corpus, oracle, đột biến, bộ sinh và đọc mã CÙNG HỌ (độc lập yếu). Không dùng chữ "kiểm toán" và không dùng "khác họ" cho vùng nào ngoài hai vùng trên.

HEAD đo: `3572787`; BASE `0a93310`; G1.5 (GPT-6 đọc) tại `c5aa45b`; R11b (Claude đọc) tại `f9f0f78`.

Tác giả theo trailer `Author-Model` nếu có, không thì theo tổ tiên (xem đầu `tools/cross_review_coverage.py`).

| crate | tổng dòng | Claude (nhãn) | Claude (không nhãn) | GPT-6 (nhãn) | Windows không nhãn | GPT-6 đã đọc | Claude đã đọc | CHƯA: đổi sau lần đọc | CHƯA: ngoài phạm vi | CHƯA: khác |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `gt-rt` | 959 | 23 | 772 | 59 | 105 | 772 | 164 | 0 | 0 | 23 |
| `gt-guard` | 1689 | 96 | 1525 | 0 | 68 | 1525 | 68 | 96 | 0 | 0 |
| `gt-codegen-llvm` | 3711 | 0 | 3711 | 0 | 0 | 0 | 0 | 0 | 3711 | 0 |
| `gt-borrowck` | 7506 | 108 | 7391 | 0 | 7 | 0 | 0 | 0 | 7506 | 0 |
| `gt-types` | 11341 | 4 | 11305 | 0 | 32 | 0 | 0 | 0 | 11341 | 0 |
| `gt-oblig` | 12658 | 0 | 12658 | 0 | 0 | 0 | 0 | 0 | 12658 | 0 |
| `gt-term` | 2224 | 0 | 2215 | 9 | 0 | 0 | 0 | 0 | 2224 | 0 |
| `gt-effects` | 800 | 0 | 800 | 0 | 0 | 0 | 0 | 0 | 800 | 0 |
| `gt-mir` | 7952 | 0 | 7952 | 0 | 0 | 0 | 0 | 0 | 7952 | 0 |
| `gt-driver` | 17016 | 880 | 15905 | 11 | 220 | 0 | 0 | 0 | 17016 | 0 |
| **Tổng** | **65856** | 1111 | 64234 | 79 | 432 | **2297** | **232** | 96 | 63208 | 23 |

**Đọc chéo theo phạm vi tài liệu:** 2529/65856 dòng (3.8%). **Chưa đọc chéo:** 63327 dòng, trong đó NGOÀI phạm vi G1.5/R11b (checker M6/M7/M8/kiểu/hiệu ứng/MIR/driver) = 63208.

## Cách đọc bảng và HẠN CHẾ (nói thẳng)
1. **"Đọc chéo" = người đọc khác họ model với tác giả dòng.** Tác giả: trailer `Author-Model` (chỉ 40/471 commit có; 30 Sonnet 5.5, 10 GPT-6), không thì tổ tiên nhánh Linux (Claude), còn lại trong `BASE..HEAD` là "Windows không nhãn" (Codex GPT-6 **hoặc** Claude phiên Windows trước; git không lưu model).
2. **G1.5** (GPT-6, `docs/TCB_CODE_AUDIT.md`, tài liệu ghi lúc `c5aa45b`) đọc H1–H10, IR Guard, `gt-rt` và "phần hạ LLVM liên quan". Chỉ `gt-rt` và `gt-guard` được tính là đọc; `gt-codegen-llvm` tài liệu không nêu tệp nên **không tính** (3711 dòng, `CHƯA KIỂM`). Người đọc G1.5 là GPT-6 và không chạy test/build; phát hiện nào cũng được kiểm lại tại chỗ.
3. **R11b** (Claude Sonnet 5.5, `docs/TCB_CODE_AUDIT_CLAUDE.md`, `f9f0f78`) đọc diff Codex ở `gt-rt`/`gt-guard` + vùng giải xung đột merge. Tính theo phạm vi tài liệu, không xác nhận từng dòng.
4. **Checker M6/M7/M8/kiểu/hiệu ứng/MIR/driver = 63 208 dòng CHƯA TỪNG được đọc chéo** (chỉ có corpus, oracle, đột biến, fuzz). Đây là phần lớn TCB theo số dòng.
5. Dòng nhóm "đổi sau lần đọc" (`gt-guard` 96, `gt-rt` ít) gồm mã do chính Claude Sonnet 5.5 viết ở phiên này (test allowlist, FB-C31): chưa ai khác đọc.
6. `git blame` gán dòng cho commit chạm cuối; định dạng lại làm dòng cũ mang commit mới. Số liệu là cận gần đúng, không phải đánh giá an toàn toàn diện.
7. **Hệ quả cho B3 (đã siết ở GÓI 4):** "đạt có điều kiện cho intrinsic/runtime/Guard" chỉ đúng với `gt-rt` + `gt-guard` (~2 650 dòng); KHÔNG đúng cho phần còn lại của TCB. Phiếu "Giang quyết" ước "G1.5 phủ gần hết số dòng" là **không đúng** (phủ ≈ 3,8 % theo bảng này).

## Kế hoạch khi chỉ có Claude (GÓI 4 S1)
- B3 "đạt có điều kiện" CHỈ cho `gt-rt` + `gt-guard` tại `c5aa45b`; dòng đổi sau đó (cột "CHƯA: đổi sau lần đọc") KHÔNG được tính.
- Phần còn lại: bù bằng (1) phiên Claude Opus MỚI chỉ đọc (`docs/E_PROMPT_OPUS.md`, R11c CHỜ PHIÊN OPUS, ghi "CÙNG HỌ, ĐỘC LẬP YẾU"), (2) đo độ phủ nhánh quyết định (`docs/DECISION_COVERAGE.md`, tuỳ chọn), (3) ThreadSanitizer cho đua dữ liệu (`docs/TSAN_RESULTS.md`).
- Đọc cùng họ KHÔNG nâng trạng thái dòng TCB nào.
