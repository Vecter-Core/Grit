# KẾ HOẠCH CHẠY TOÀN BỘ DỰ ÁN GRIT (bộ kiểm toán — KHÔNG sửa dự án)

Phạm vi lệnh của chủ dự án: (1) đọc toàn bộ dự án, (2) có MỘT tệp chạy toàn bộ dự án, chạy lần lượt theo kế hoạch rồi ghi kết quả vào tệp,
(3) phân tích hướng chưa có / đang lỗi, (4) tự chạy tiếp tới hết, bị ngắt rồi quay lại thì **tự tiếp tục không cần nhắc**, và **không sửa
trực tiếp vào dự án — chỉ ghi vào tệp kết quả**.

## Các tệp (tất cả nằm trong `audit/`, tệp của dự án không bị chạm)
| Tệp | Vai trò |
|---|---|
| `run_all.sh` | **Tệp chạy toàn bộ dự án.** 37 giai đoạn, mỗi giai đoạn có timeout, log riêng, trạng thái riêng. |
| `state.tsv` | Điểm lưu: `ID  TRẠNG-THÁI  mã-thoát  bắt-đầu  giây  ghi-chú`. Quyết định giai đoạn nào còn phải chạy. |
| `RESULTS.md` | **Tệp kết quả** (tự sinh sau mỗi giai đoạn): bảng tổng hợp + chi tiết từng giai đoạn. |
| `ANALYSIS.md` | Phân tích: cái gì đang lỗi, cái gì chưa có, rủi ro, đề xuất (viết tay, dựa trên RESULTS + đọc mã). |
| `results/<ID>.md` | Đoạn kết quả của một giai đoạn; `results/<ID>.note.md` = ghi chú tay của người phân tích (được ghép vào RESULTS). |
| `summarize.py`, `report.py`, `corpus_cli.py` | Trình tóm tắt log, trình dựng RESULTS.md, bộ kiểm corpus qua dòng lệnh `gritc`. |
| `patches/driver_nollvm.patch` | Vá 17 dòng, CHỈ áp lên **bản sao** ngoài dự án (xem G00). |
| `logs/` (không vào git) | Log đầy đủ từng giai đoạn. |

## Giao thức tiếp tục (quan trọng)
Phiên mới (sau khi bị ngắt, hết lượt, container mới) chỉ cần:
1. `git pull`/`git checkout` nhánh làm việc để lấy `audit/state.tsv` + `audit/results/` mới nhất (container mới không có `audit/logs/`; không sao).
2. `bash audit/run_all.sh` — tự bỏ qua giai đoạn đã `PASS/FAIL/BLOCKED/SKIP`, chạy lại giai đoạn `RUNNING` (bị ngắt giữa chừng) và `PENDING`.
   Đặt `AUDIT_WORK=<thư mục ngoài dự án>` (mặc định `/tmp/grit_audit_work`). Giai đoạn nhóm G cần bản sao: nếu mất, chạy lại `--only G00_copy_patch`.
3. Khi hết giai đoạn: viết/cập nhật `ANALYSIS.md`, commit `audit/` lên nhánh.
`--list` xem bảng trạng thái; `--retry-failed` thử lại giai đoạn FAIL; `--only ID,ID` chạy ép; `--group C` chạy một nhóm; `--next N` chạy N giai đoạn.

## Vì sao có giai đoạn BLOCKED
Dự án chạy ở **Windows x86_64 MSVC + LLVM 20.1.8** (docs/ENVIRONMENT.md). Máy này là Linux (LLVM hệ thống 18). Bằng chứng, không suy đoán:
`crates/gt-llvm/build.rs` đòi `LLVM-C.lib`; `crates/gt-driver/src/native.rs` gọi `link.exe` của MSVC; `runtime/gt-rt/src/lib.rs` có
`compile_error!` cho mọi target khác Windows x86_64. Vì vậy backend LLVM / IR Guard / gt-rt / `rig build` **không chạy được ở đây** — ghi rõ
BLOCKED kèm log, KHÔNG ghi PASS. Phần chạy được nguyên trạng: 17 crate "thuần" (lexer → … → secrets) và, qua bản sao có vá, `gritc` với backend
thông dịch MIR.

## Danh sách giai đoạn và lý do
| Nhóm | Giai đoạn | Mục đích |
|---|---|---|
| A | `A00_env` `A01_fetch` `A02_inventory` | môi trường, phụ thuộc, kiểm kê (crate, LOC, tệp nhị phân bị theo dõi, CI/deny) |
| B | `B01_fmt` `B02_clippy` `B03_audit` `B04_ci_deny` | cổng chất lượng mà GRIT_PLAN §0.7 yêu cầu |
| C | `C00_build`, `C_<crate>` ×17 | dựng và chạy test từng crate theo thứ tự phụ thuộc (span → lexer → … → secrets), `--test-threads=2` theo quy tắc dự án |
| D | `D01_native_probe` | chứng minh bằng lệnh thật rằng gt-driver/rig/gt-llvm KHÔNG dựng được nguyên trạng ở đây |
| E | `E01_gtrt` | runtime native |
| F | `F01_expect` `F02_tool_tests` | công cụ Python của dự án: nhất quán corpus ↔ bảng mã lỗi; unittest công cụ |
| G | `G00`…`G07` | trên **bản sao có vá**: test gt-driver/rig, corpus qua CLI, chạy chương trình, fmt lũy đẳng, tất định, explain |
| H | `H01_llvm_probe` | có lấy được LLVM 20 cho Linux không (để biết port khả thi tới đâu) |
| I | `I01_fuzz_probe` | fuzz lexer/parser cần nightly + cargo-fuzz |

Không chạy (và lý do): đột biến (`tools/mut*.py`: nhiều giờ, ghi vào bản sao riêng), `probe_diff` bản đủ (~29 phút), `GT_FULL8=1` (~11 phút),
benchmark (cần exe native), fuzz ≥ 24 giờ. Đây là việc của chủ dự án khi có máy Windows; ghi vào ANALYSIS.md.
