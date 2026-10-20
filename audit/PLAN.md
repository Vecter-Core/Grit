# KẾ HOẠCH CHẠY TOÀN BỘ DỰ ÁN GRIT (bộ kiểm toán — KHÔNG sửa dự án)

Phạm vi lệnh của chủ dự án: (1) đọc toàn bộ dự án, (2) có MỘT tệp chạy toàn bộ dự án, chạy lần lượt theo kế hoạch rồi ghi kết quả vào tệp,
(3) phân tích hướng chưa có / đang lỗi, (4) tự chạy tiếp tới hết, bị ngắt rồi quay lại thì **tự tiếp tục không cần nhắc**, và **không sửa
trực tiếp vào dự án — chỉ ghi vào tệp kết quả**.

## Các tệp (tất cả nằm trong `audit/`, tệp của dự án không bị chạm)
| Tệp | Vai trò |
|---|---|
| `run_all.sh` | **Tệp chạy toàn bộ dự án.** 58 giai đoạn, mỗi giai đoạn có timeout, log riêng, trạng thái riêng, tự kiểm `git status` trước/sau. |
| `state.tsv` | Điểm lưu: `ID  TRẠNG-THÁI  mã-thoát  bắt-đầu  giây  ghi-chú`. Quyết định giai đoạn nào còn phải chạy. |
| `RESULTS.md` | **Tệp kết quả** (tự sinh sau mỗi giai đoạn): bảng tổng hợp + chi tiết từng giai đoạn. |
| `ANALYSIS.md` | **Phân tích**: cái gì lỗi (có tái hiện + nguyên nhân gốc + hướng sửa), cái gì chưa có, rủi ro, thứ tự ưu tiên. |
| `results/<ID>.md` / `<ID>.note.md` | Đoạn kết quả tự sinh / ghi chú tay của người phân tích (được ghép vào RESULTS). |
| `summarize.py`, `report.py` | Trình tóm tắt log → `results/`, trình dựng `RESULTS.md`. |
| `corpus_cli.py` | Chạy `gritc check --json` trên cả 855 ca của `tests/{pass,fail,warn}` và so với `expect`. |
| `probes/*.gt` + `probes_run.py` | ~180 chương trình THỬ TẤN CÔNG tự viết (kỳ vọng ở dòng đầu `// probe:`); phát hiện âm tính giả. |
| `mutfuzz.py` | Fuzz đột biến token đen-hộp qua CLI (phát hiện ICE/crash/treo). |
| `diff_checks.py` | Vi sai độc lập: `--checks=all ≡ proven` (bộ giải M7) và thông dịch ≡ exe native (codegen). |
| `check_codes.py` | Mọi mã chẩn đoán trong `KNOWN` có tài liệu + ≥3 mẫu fail + ≥3 mẫu pass không (GRIT_PLAN §0.5). |
| `native_cases/*.gt` | Chương trình cho các giai đoạn tràn ngăn xếp / sập trình biên dịch. |
| `patches/driver_nollvm.patch` | Vá 17 dòng để `gritc` dựng không cần LLVM — CHỈ áp lên bản sao. |
| `patches/linux_port.patch` | Vá 5 tệp (+62/−26) để backend LLVM + gt-rt chạy trên Linux — CHỈ áp lên bản sao. |
| `logs/` (không vào git) | Log đầy đủ từng giai đoạn. |

## Giao thức tiếp tục (quan trọng)
Phiên mới (sau khi bị ngắt, hết lượt, container mới) chỉ cần:
1. Lấy nhánh làm việc để có `audit/state.tsv` + `audit/results/` mới nhất (`audit/logs/` không vào git — container mới không có, không sao).
2. `AUDIT_WORK=<thư mục ngoài dự án> bash audit/run_all.sh` — tự bỏ qua giai đoạn đã `PASS/FAIL/BLOCKED/SKIP`, chạy lại giai đoạn `RUNNING`
   (bị ngắt giữa chừng) và `PENDING`. Các giai đoạn dùng bản sao (nhóm G, L) **tự dựng lại bản sao** nếu mất (`make_copy`/`make_lcopy`).
3. Khi hết giai đoạn: cập nhật số liệu trong `ANALYSIS.md` (các dòng "xem RESULTS"), commit `audit/` lên nhánh làm việc.

Lệnh hữu ích: `--list` (bảng trạng thái) · `--only ID,ID` (chạy ép) · `--group C` (một nhóm) · `--retry-failed` · `--next N` · `--report`.
Mẹo: giai đoạn FAIL có thể là **kết quả đúng** (đã tìm thấy lỗi) — đừng `--retry-failed` mù quáng; đọc `results/<ID>.md` và `.note.md`.

## Vì sao có giai đoạn BLOCKED, và bản sao có vá
Dự án chạy ở **Windows x86_64 MSVC + LLVM 20.1.8** (docs/ENVIRONMENT.md). Máy kiểm toán là Linux (LLVM hệ thống 18 + gói `libllvm20`). Bằng chứng:
`gt-llvm/build.rs` đòi `LLVM-C.lib`; `gt-driver/src/native.rs` gọi `link.exe`; `gt-rt/src/lib.rs` có `compile_error!`; và `gt-driver --no-default-features`
không biên dịch. Vì vậy: D01/E01 ghi **BLOCKED** (đúng sự thật cho mã nguyên trạng). Để vẫn kiểm được phần còn lại, có **hai bản sao ngoài dự án**:
- *Bản sao 1* (`$AUDIT_WORK/grit`): vá 17 dòng ⇒ `gritc` backend thông dịch ⇒ nhóm G.
- *Bản sao 2* (`$AUDIT_WORK/grit_linux`): vá Linux ⇒ backend LLVM thật, exe ELF ⇒ nhóm L (cần `libllvm20`).
Kết quả trên bản sao KHÔNG phải kết quả trên Windows: chúng xác nhận logic, không xác nhận tính chất riêng Windows.

## Danh sách giai đoạn
| Nhóm | Giai đoạn | Mục đích |
|---|---|---|
| A | `A00_env` `A01_fetch` `A02_inventory` | môi trường, phụ thuộc, kiểm kê (crate, LOC, tệp nhị phân bị theo dõi, CI/deny) |
| B | `B01_fmt` `B02_clippy` `B03_audit` `B04_ci_deny` `B05_codes` | cổng chất lượng GRIT_PLAN §0.5/§0.7 |
| C | `C00_build`, `C_<crate>` ×17 | dựng và test từng crate thuần theo thứ tự phụ thuộc, `--test-threads=2` (quy tắc dự án) |
| D/E | `D01_native_probe` `E01_gtrt` | chứng minh bằng lệnh thật rằng mã nguyên trạng không dựng được ngoài Windows |
| F | `F01_expect` `F02_tool_tests` | công cụ Python của dự án |
| G | `G00`…`G14` | bản sao 1: test gt-driver/rig, corpus qua CLI, chạy chương trình, fmt lũy đẳng, tất định, explain, **probe tấn công**, vi sai M7 (số học `G09`, chỉ số mảng `G14`), fuzz token, **3 kiểm metamorphic** (`G11` fmt, `G12` alias ASCII↔canonical, `G13` thứ tự khai báo) |
| L | `L01`…`L08` | bản sao 2: port Linux, test native gt-driver/rig/guard/codegen/rt, vi sai thông dịch≡native, **demo dùng-sau-reset**, tràn ngăn xếp, sập trình biên dịch |
| P | `P01_proposed_fix_f01` | bản sao 3: áp **đề xuất** sửa F-01 (`patches/proposed_F01_check_uses.patch`) rồi chạy lại probe, corpus, test gt-borrowck — chứng minh bản sửa chặn được mà không hồi quy (KHÔNG áp vào dự án) |
| H/I | `H01_llvm_probe` `I01`–`I03` | khả năng có LLVM 20 cho Linux; cài nightly+cargo-fuzz; fuzz lexer/parser thật (libFuzzer+ASan, mặc định 300 s mỗi cái — `FUZZ_SECONDS`) |

Không chạy (và lý do): đột biến (`tools/mut*.py`: nhiều giờ, ghi vào bản sao riêng của dự án), `probe_diff` bản đủ (~29 phút), `GT_FULL8=1` (~11 phút),
benchmark (cần Windows để so được với số của dự án), fuzz ≥ 24 giờ. Đây là việc của chủ dự án khi có máy Windows; xem `ANALYSIS.md` §5.
