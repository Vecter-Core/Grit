# TCB code authorship

Base: `0a93310159bd8a157f017be020e651a74ca0791d`; HEAD: `3572787ecbb71599ebe01f9dacdb329746ec3ce5`; Claude ancestry: `origin/claude/gallant-darwin-m251cq`.

| Nhóm | Codex dòng | Claude dòng | Tổng | Codex % |
|---|---:|---:|---:|---:|
| `gt-rt` | 187 | 772 | 959 | 19.50% |
| `gt-codegen-llvm` | 0 | 3711 | 3711 | 0.00% |
| `gt-guard` | 164 | 1525 | 1689 | 9.71% |
| `gt-borrowck` | 115 | 7391 | 7506 | 1.53% |
| `gt-types` | 36 | 11305 | 11341 | 0.32% |
| `gt-oblig` | 0 | 12658 | 12658 | 0.00% |
| `gt-term` | 9 | 2215 | 2224 | 0.40% |
| `gt-effects` | 0 | 800 | 800 | 0.00% |
| `gt-mir` | 0 | 7952 | 7952 | 0.00% |
| `gt-driver` | 1111 | 15905 | 17016 | 6.53% |
| **Tổng** | **1622** | **64234** | **65856** | **2.46%** |

Đếm dòng `git blame`, gồm merge/conflict-resolution commit theo quy tắc; không suy ra model cụ thể cho từng commit.

**Đo lại 2026-11-18 (GÓI 3 S3.a):** bảng trên dùng quy tắc cũ: "Codex" = mọi commit trong `BASE..HEAD` không thuộc nhánh Linux. Quy tắc đó **tính cả commit của Claude Sonnet 5.5 ở phiên Windows không có nhãn** là "Codex", nên cột Codex là **cận trên**. Phân loại chi tiết theo trailer `Author-Model` và phạm vi đã đọc chéo: `docs/CROSS_REVIEW_COVERAGE.md` (`python tools/cross_review_coverage.py`). Số HEAD cũ `9e9b54c`: Codex 503/64 768 (0,78 %).
