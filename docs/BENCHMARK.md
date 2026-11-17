# Benchmark nền của backend llvm (M10)

Đo bằng `python tools/bench.py 9` ngày 2026-10-13, máy i5-12450H / 15,6 GB, Windows 11, gritc **debug** (chỉ ảnh hưởng cột "dựng"), exe sinh ra bởi LLVM 20.1.8 **-O0** (hồ sơ `dev`, mọi `Assert` giữ, `--checks=all`). Tệp: `tests/bench/*.gt`; tham chiếu Rust: `tools/bench_ref/*.rs` (cùng thuật toán, cùng kiểm tràn/biên).

| bench | công việc | dựng (gritc debug) | exe LLVM -O0 trung vị / min (9 lần) | Mops/s (theo min) | thông dịch (thu nhỏ) | thông dịch Mops/s | Rust -C opt-level=0 (trung vị / min) | Rust -O (trung vị / min) |
|---|---|---|---|---|---|---|---|---|
| loop_sum | 400,000,000 | 0.20 s | 0.52 s / 0.50 s | 804.1 | 2.41 s (1,000,000) | 0.42 | 0.92 / 0.83 s | 0.03 / 0.02 s |
| fib | 29,860,703 | 0.21 s | 1.10 s / 1.08 s | 27.6 | 0.22 s (57,313) | 0.26 | 0.17 / 0.16 s | 1.05 / 0.05 s |
| slice_sum | 200,000,000 | 0.19 s | 0.52 s / 0.49 s | 409.7 | 0.30 s (1,000) | 0.00 | 0.60 / 0.51 s | 0.05 / 0.04 s |
| threads | 4,000,000 | 0.22 s | 1.27 s / 0.28 s | 14.5 | — | — | — | — |

## Đọc số cho đúng (đừng suy diễn quá)
- **Nhiễu lớn trên máy này.** Lần chạy đầu của một exe mới thường chậm thêm ~1 s (quét tệp của Windows Defender); script bỏ một lần khởi động nóng nhưng trung vị vẫn bị kéo (vd `Rust -O fib`: 1.05 s trung vị vs 0.05 s min; `threads`: 1.27 s vs 0.28 s). **Dùng cột `min` làm mốc tốt nhất**, trung vị là trải nghiệm điển hình trên máy này.
- **Đây là mốc NỀN, không phải so sánh ngôn ngữ.** `-O0` của Grit vẫn là mã "mọi local là alloca", tuple kiểm tràn lưu/đọc qua bộ nhớ; chậm hơn Rust `-O` cỡ 10–50× ở vòng lặp số học và ~20× ở `fib` (1.08 s vs 0.05 s). So với Rust `-C opt-level=0`: tương đương ở `loop_sum`/`slice_sum`, chậm ~6,7× ở `fib` (0.16 s) vì Grit trả tuple `{T, i1}` qua bộ nhớ cho mỗi phép kiểm tràn và gọi hàm qua alloca.
- **Thông dịch tham chiếu** chỉ ~0.2–0.4 triệu thao tác/s (chậm ~2000× so với exe -O0 ở `loop_sum`); `slice_sum` thu nhỏ còn 1 000 phần tử vì thông dịch sao chép mảng khi `Use` (không đại diện cho backend). Thông dịch KHÔNG hỗ trợ `static`/`spawn` (bỏ qua ở `threads`).
- **`threads`**: 4 luồng HĐH thật, 2 triệu `Mutex::with` + 2 triệu `fetch_add`; 14,5 triệu thao tác khoá+nguyên tử/s (theo min). Kết quả luôn 4 000 000.
- **Chưa có hồ sơ `release` (-O2).** ADR-029 §7 chỉ cho phép khi `dev` ≡ `release` trên bộ đối chiếu; IR Guard phải chạy SAU tối ưu (LLVM có thể sinh `nsw`/`unreachable`). Việc để sau, cần Giang duyệt hướng.
- Thời gian dựng ~0.2 s là của `gritc` debug (cổng M4–M9 + codegen + liên kết); `gritc` release sẽ nhanh hơn, chưa đo.

## Hồ sơ `release` (ADR-036) so với `dev` — đo ngày 2026-10-14 (bước iv)
Đo bằng `python tools/bench_profile.py 7`: fib(35) và 5 công cụ dogfood với đầu vào sinh tất định (wc 32 MiB văn bản; sha256 32 MiB byte; sortlines 200 000 dòng; json mảng 20 000 đối tượng; stats 10^6 số), mỗi exe chạy 7 lần XEN KẼ dev/release sau 1 lần khởi động nóng; máy i5-12450H, Windows 11, gritc debug. Đầu ra (stdout + mã thoát) của hai hồ sơ GIỐNG NHAU ở mọi bench. **Đây là PHÉP ĐO, không phải cổng.**

### A. Cấu hình ĐÃ CHỐT (commit hiện tại): release = `mem2reg,sroa,early-cse` ở mức IR, backend vẫn O0
| bench | dựng dev / release (gritc debug) | dev: trung vị (min–max) ± σ | release: trung vị (min–max) ± σ | tăng tốc (trung vị) | tăng tốc (min) | đầu ra giống nhau |
|---|---|---|---|---|---|---|
| fib(35) | 0.22 / 0.21 s | 0.082 s (0.080–0.092) ± 0.004 | 0.096 s (0.093–0.102) ± 0.003 | ×0.86 | ×0.85 | có |
| wc | 0.23 / 0.24 s | 0.284 s (0.274–0.301) ± 0.011 | 0.280 s (0.276–0.314) ± 0.014 | ×1.01 | ×0.99 | có |
| json | 1.09 / 1.14 s | 0.051 s (0.049–0.057) ± 0.003 | 0.049 s (0.048–0.059) ± 0.004 | ×1.04 | ×1.02 | có |
| sha256 | 0.55 / 0.57 s | 1.409 s (1.357–1.494) ± 0.050 | 2.156 s (2.076–2.258) ± 0.073 | ×0.65 | ×0.65 | có |
| sortlines | 0.48 / 0.47 s | 0.240 s (0.237–0.279) ± 0.014 | 0.201 s (0.196–0.233) ± 0.014 | ×1.19 | ×1.21 | có |
| stats | 0.36 / 0.37 s | 0.149 s (0.146–0.151) ± 0.002 | 0.148 s (0.145–0.154) ± 0.004 | ×1.00 | ×1.00 | có |

Đọc: **release chưa nhanh hơn dev một cách đáng kể** — wc, json, stats ≈ ×1,0; sortlines ×1,2; fib ×0,86 và **sha256 ×0,65 (CHẬM HƠN dev, lặp lại ở cả hai lần đo)**. Nguyên nhân (giả thuyết, chưa kiểm riêng): sau `mem2reg` các giá trị sống qua ranh giới khối nằm trong thanh ghi ảo, nhưng bộ cấp phát thanh ghi của backend O0 (`fast`) làm tràn mọi giá trị ở cuối mỗi khối + chép phi ⇒ không lợi so với alloca và đôi khi tệ hơn. Danh sách pass đã chốt vì Guard-post (xem ADR-036 §7) nên tối ưu IR bị giới hạn; lợi tốc độ thật nằm ở backend.

### B. THÍ NGHIỆM (KHÔNG commit): cùng danh sách pass nhưng `TargetMachine` ở `OptimizationLevel::Default` (backend O2)
| bench | dựng dev / release (gritc debug) | dev: trung vị (min–max) ± σ | release: trung vị (min–max) ± σ | tăng tốc (trung vị) | tăng tốc (min) | đầu ra giống nhau |
|---|---|---|---|---|---|---|
| fib(35) | 0.21 / 0.21 s | 0.090 s (0.083–0.101) ± 0.006 | 0.072 s (0.065–1.075) ± 0.496 | ×1.25 | ×1.27 | có |
| wc | 0.26 / 0.28 s | 0.299 s (0.283–0.339) ± 0.022 | 1.126 s (0.130–1.157) ± 0.453 | ×0.27 | ×2.17 | có |
| json | 1.17 / 1.17 s | 0.057 s (0.048–0.068) ± 0.007 | 0.038 s (0.037–1.030) ± 0.347 | ×1.49 | ×1.28 | có |
| sha256 | 0.54 / 0.56 s | 1.462 s (1.348–1.641) ± 0.097 | 0.427 s (0.401–1.485) ± 0.477 | ×3.42 | ×3.36 | có |
| sortlines | 0.46 / 0.50 s | 0.248 s (0.244–0.254) ± 0.004 | 0.097 s (0.090–0.100) ± 0.003 | ×2.55 | ×2.70 | có |
| stats | 0.36 / 0.41 s | 0.181 s (0.163–0.245) ± 0.026 | 1.062 s (1.056–1.069) ± 0.004 | ×0.17 | ×0.15 | có |

Đọc theo **min** (trung vị bị nhiễu, xem dưới): fib ×1,27; wc ×2,17; json ×1,28; **sha256 ×3,36**; sortlines ×2,70; stats: cả 7 lần chạy của release trong script đều ≈ 1,06 s (hiệu ứng quét tệp ảnh hưởng TOÀN BỘ lần chạy của exe này, nên cột này vô nghĩa); đo lại riêng (3 vòng × 5 lần, cùng tệp 10^6 số): dev 0,15–0,18 s, release 0,06–0,08 s ⇒ ≈ ×2,3. Các dòng `wc`, `stats`, `json`, `fib`, `sha256` có lần chạy ≈ 1,0–1,1 s (với `stats`: cả 7 lần): đó là hiệu ứng quét tệp của Windows Defender lên exe mới (đã ghi ở mục đầu tệp này), KHÔNG phải tốc độ chương trình — vì vậy dùng **min** làm số đại diện và đừng tin trung vị/σ ở các dòng đó. (Lần đo đầu của cấu hình A cũng dính hiệu ứng này: fib release 1,11 s đều đặn 7 lần; đo lại và đo tay cho 0,096 s.)
Bằng chứng an toàn của thí nghiệm B (mẫu nhỏ): `release_traps` (14 loại abort + số thực từng bit IEEE gồm bẫy FMA) 4/4, `release_guard` 4/4, `tiny_*` 3/3 + 4/4, 60 chương trình f64 và 30 chương trình abort-index đều 0 bất đồng. CHƯA chạy 1820 chương trình ở cấu hình B.
**Vì sao KHÔNG tự bật:** backend O2 đưa TỐI ƯU MÁY (DAG-combine, cấp phát thanh ghi greedy, gộp nhánh) vào TCB ở tầng SAU Guard-post — Guard-post chỉ nhìn IR nên không thấy tầng này; chỉ còn bằng chứng hành vi dev≡release. ADR-036 chỉ duyệt danh sách pass IR. Q82 đã được Giang duyệt có điều kiện (2026-10-14; xem `OPEN_QUESTIONS.md`): release/release-o2 là hỗ trợ bằng chứng thực nghiệm, mặc định vẫn dev (nhãn "chờ" sửa 2026-11-18).

## G2.3 — tái đo sortlines với Rust O0 và Rust `-O` (2026-11-12)

Lịch sử trong `git log -- docs/BENCHMARK.md` cho thấy harness M10 đã biên dịch Rust ở cả `opt-level=0` và `-O`; hai mức phải được báo riêng. Harness tái lập mới là `tools/bench_sortlines_g2.py` (commit `5256a75`), dùng `gritc 1.0.0-rc1`, Rust 1.98.1, LLVM 20.1.8, sort ổn định `sort_by`, input tuần tự một lượt làm nóng rồi năm lượt đo. Raw có lệnh/profile, hash input/output ở `docs/SORTLINES_BENCHMARK_G2.jsonl`.

| Cấu hình | G2 run A median / min | G2 run B median / min | So khớp output |
|---|---:|---:|---|
| Grit dev (`-O0`, Arena check động bật) | 2,8402 / 2,6747 s | 3,6890 / 3,3729 s | SHA-256 `45459d…cef51` |
| Grit release-o2 | 1,2007 / 1,1386 s | 1,4924 / 1,2580 s | cùng SHA-256 |
| Rust `-C opt-level=0` | 1,9651 / 1,9495 s | 2,1585 / 1,9668 s | cùng SHA-256 |
| Rust `-O` (`opt-level=3`, cột chính) | 0,8834 / 0,8297 s | 0,8017 / 0,7773 s | cùng SHA-256 |

Input là 1.636.921 dòng, 33.554.438 byte, SHA-256 `562030eb…c093aa77`; so sánh byte đầy đủ qua hash đầu ra và độ dài. Run A dùng harness commit `5256a75`, run B và probe scaling dùng `ef17563`; raw của cả hai ở `docs/SORTLINES_BENCHMARK_G2.jsonl`.

Đối chiếu với G1.4: input khi đó khác (hash `0d812e…cac38`), Grit 3,0055 s median và Rust `sort_by` 0,7741 s; G2.3 đo lại 2,8402 s và 0,8834 s trên input mới. Tái dựng đúng số `~2,05 s` lịch sử không thể xác nhận vì không còn hash/input/harness cũ trong Git. Dữ liệu phù hợp với giả thuyết thay baseline từ Rust debug/O0 (~2 s) sang Rust `-O` (~0,8 s), chứ không cho thấy Grit tăng 12% ổn định; Grit dev trên hai phép đo median là 3,0055 và 2,8402 s, trong khi input và harness khác. Không lấy hai tập đó để tính một benchmark paired.

### `arena_overlap` và độ phức tạp theo số dòng

Đọc `examples/sortlines/src/main.gt`: Vec dòng và Vec scratch đều `reserve(nl)` một lần; Vec đầu ra `reserve(total)` một lần. Mỗi `reserve` cần một lần cấp Arena, nên số lời gọi Arena của ứng dụng gần như cố định ở ba, không một lần mỗi dòng. Trong `runtime/gt-rt/src/lib.rs`, `arena_verify` kiểm chunk và tra hai láng giềng trong `BTreeMap` (`O(log a)` theo số vùng đã cấp); với ba lần cấp, nó không tạo vòng lặp O(n) trên số dòng.

Đo Grit dev ba lượt/input tăng dần (run B): 100.000 dòng (2.049.851 byte) median 0,1255 s; 400.000 (8.199.403 byte) 0,5323 s; 1.636.921 (33.554.438 byte) 2,6790 s. Thời gian mỗi dòng lần lượt 1,255 / 1,331 / 1,636 µs. Tăng nhanh hơn tuyến tính, phù hợp với sort O(n log n) cộng chi phí cấp phát/copy; không có bằng chứng Arena overlap đang cấp phát O(n) lần. Không sửa runtime vì giả thuyết O(n) allocation không được xác nhận.

Giới hạn: chỉ một máy Windows, một họ dữ liệu ASCII, không có profiler tách chi phí sort/đọc/ghi. Hai run G2 khá nhiễu (Grit dev median 2,84–3,69 s); không ghép input khác nhau giữa G1.4/G2 và không dùng một lần đo để khẳng định tăng tốc tổng quát.


### C. CẤU HÌNH ỨNG VIÊN ĐÃ CÓ BẰNG CHỨNG: `--profile=release-o2` (đo sạch 2026-10-15, 9 lần xen kẽ, không dính hiệu ứng quét tệp)
Cùng đầu vào như mục A/B; `BENCH_PROFILE=release-o2 python tools/bench_profile.py 9`.
| bench | dựng dev / release (gritc debug) | dev: trung vị (min–max) ± σ | release: trung vị (min–max) ± σ | tăng tốc (trung vị) | tăng tốc (min) | đầu ra giống nhau |
|---|---|---|---|---|---|---|
| fib(35) | 0.18 / 0.18 s | 0.082 s (0.079–0.091) ± 0.003 | 0.063 s (0.059–0.069) ± 0.003 | ×1.31 | ×1.33 | có |
| wc | 0.23 / 0.25 s | 0.283 s (0.267–0.298) ± 0.008 | 0.127 s (0.122–0.144) ± 0.007 | ×2.22 | ×2.18 | có |
| json | 1.09 / 1.13 s | 0.055 s (0.051–0.059) ± 0.003 | 0.039 s (0.035–0.049) ± 0.004 | ×1.40 | ×1.48 | có |
| sha256 | 0.52 / 0.53 s | 1.404 s (1.375–1.588) ± 0.061 | 0.421 s (0.405–0.487) ± 0.024 | ×3.33 | ×3.39 | có |
| sortlines | 0.50 / 0.52 s | 0.269 s (0.249–0.302) ± 0.017 | 0.111 s (0.101–0.136) ± 0.010 | ×2.41 | ×2.47 | có |
| stats | 0.40 / 0.42 s | 0.153 s (0.147–0.163) ± 0.004 | 0.066 s (0.063–0.092) ± 0.009 | ×2.31 | ×2.33 | có |
Đầu ra (stdout + mã thoát) giống nhau ở mọi bench. Số này thay thế các ô nhiễu của mục B (stats ×2,3 là đo riêng cũ; nay đo trong cùng script). Bằng chứng an toàn: ADR-036 §9.
