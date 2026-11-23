# PHASE2_PLAN — kế hoạch hợp nhất Giai đoạn 2 (ĐỀ XUẤT, CHƯA DUYỆT; GÓI 9, 2026-11-20)

> Trạng thái: **ĐỀ XUẤT của kiến trúc sư (Claude) cho Giang duyệt.** Không có mục nào ở đây là quyết định cho tới khi Giang ký. Không có dòng mã Giai đoạn 2 nào được viết trước khi điều kiện vào (mục 1) đạt. Không có mốc ngày. Nguồn: `docs/PHASE2_PLAN.md` (Opus 5.5, ĐỀ XUẤT) đối chiếu mã ở `docs/PHASE2_FACTCHECK.md`; `GRIT_PLAN.md` §8.

## 0. Tóm tắt cho Giang (≤ 1 trang)
1. **Việc gì:** gộp đề xuất của Opus vào MỘT kế hoạch có thứ tự, theo bốn dòng việc: **A. Bảo đảm**, **B. Độc lập backend**, **C. Dùng được cho AI**, **D. Hiệu năng**.
2. **Khác Opus ở đâu:** an toàn trước hiệu năng. Opus xếp "khép khoảng cách backend" và tối ưu bố cục lên đầu; kế hoạch này xếp **xoá Secret có bảo đảm (lỗ hổng thật: hiện KHÔNG có chỗ nào xoá Secret, FACTCHECK mục 3)** và các ADR thiết kế lên đầu, tối ưu hiệu năng sau cùng, sau khi có bằng chứng.
3. **Đối chiếu mã tìm ra:** 44 khẳng định: 29 đúng, 7 đúng một phần, 1 sai (số công cụ đo), 7 không kiểm được bằng đọc. Hai điều làm đổi kế hoạch: (i) niche cho con trỏ béo KHÔNG dùng được (chuỗi/lát rỗng có thể là `null+0`); (ii) mâu thuẫn `spawn`/`detach` (ADR-C12) chưa giải, mà kênh có kiểu (8.D) sẽ chạm đúng vào.
4. **Quy tắc cú pháp:** mọi thứ cần cú pháp mới (`old`, bất biến kiểu, kiểu khoảng, mã ma, `?`, nhãn đối số, tuyến tính) vào **MỘT đợt "edition" duy nhất**, sau khoá edition + `gritc fix`; lexer/parser khởi động lại đóng băng và fuzz 24 giờ MỘT lần sau đợt đó.
5. **Hai đường hạ (L1):** khi chuyển hạ LLVM sang GLIR, GIỮ CẢ HAI đường dưới cờ chỉ có trong test, so IR và kết quả trên toàn bộ corpus + mọi bộ sinh; chỉ bỏ đường cũ khi trùng.
6. **Backend x64 (nếu làm) là để ĐA DẠNG** (vi sai hai backend, bỏ phụ thuộc LLVM, dịch nhanh, không unwinding), KHÔNG vì "đáng tin hơn LLVM -O0". Mỗi backend giữ lớp kiểm sau cùng của riêng nó; COFF (Windows) xong TRƯỚC khi xét đổi mặc định dev.
7. **Giang cần duyệt:** 5 điểm ở `docs/STATUS.md` (cấu trúc, gom cú pháp, thứ tự, hai đường hạ, đổi mặc định sau này). Bốn ADR nháp ở `docs/PHASE2_ADR_DRAFTS.md`.

## 1. ĐIỀU KIỆN VÀO Giai đoạn 2 (ĐỀ XUẤT)
Không bắt đầu mã Giai đoạn 2 trước khi ĐỒNG THỜI có:
- nhãn `1.0.0-seed` đạt, hoặc đạt có điều kiện **do Giang ký** (`docs/STATUS.md`);
- các điều kiện còn lại của `GRIT_PLAN.md` §7 (cổng chuyển giai đoạn) đã được kiểm từng cái và ghi bằng chứng (hiện còn mở: cổng Linux, fuzz 24 giờ, mục chặn khác — xem `STATUS.md`);
- cổng đầy đủ rc6 xanh (Windows XANH ở `16c37f0`; Linux CHƯA: `docs/CLOUD_GATE_PROMPT.md`);
- Giang ra lệnh bằng văn bản mở Giai đoạn 2 (`STATUS.md` #14: "cấm khi chưa có lệnh").
Trước đó chỉ làm được TÀI LIỆU/ĐO (mục 4, giai đoạn 2.0).

## 2. BỐN DÒNG VIỆC (mỗi mục: nguồn, ADR, dòng TCB, cổng, rủi ro, phụ thuộc, cú pháp mới?)
Ký hiệu nguồn: "Opus §x" = `PHASE2_PLAN.md`; "GP 8.x" = `GRIT_PLAN.md`. Mọi mục ghi **ĐỀ XUẤT**.

### A. Bảo đảm (assurance)
| Mục | Nguồn | ADR cần | Dòng TCB | Bằng chứng cổng | Rủi ro / phụ thuộc | Cú pháp mới? |
|---|---|---|---|---|---|---|
| **A1 — 1b: xoá Secret có bảo đảm** (ĐÃ LÀM 2026-11-23, ADR-P2-03; TCB B6 CHỜ DUYỆT; nguồn `Secret::new` thêm vì trước đó không tạo được Secret) | Opus §2.1b; FACTCHECK §3 | ADR-P2-03 (nháp) | CÓ: một intrinsic `trusted` mới (xoá volatile); dòng TCB mới, trạng thái ban đầu "CHƯA DUYỆT" | test đỏ: exe mà Secret còn nguyên trong vùng Arena sau `reset`/thoát phạm vi; test xanh: byte đã xoá; đột biến tự động bản xoá (`tools/mut.py`); Guard cập nhật bảng `RUNTIME` nếu thêm hàm `gritrt_*`; golden IR | KHÔNG dựa vào `Drop` (chưa hạ: `KNOWN_LIMITATIONS.md:105,233`) ⇒ codegen chèn. Giới hạn CÔNG KHAI: chỉ ô nhớ Grit quản lý; thanh ghi tràn, swap, core dump KHÔNG được bảo vệ. Phụ thuộc: không | Không |
| **A2 — 8.A SMT** (BƯỚC 1 ĐÃ LÀM 2026-11-23: Farkas + kernel thay vì Z3 — ADR-P2-04; bước 2: sự kiện tuyến tính trong M7) | GP 8.A | ADR thiết kế SMT (làm ở 2.0) | bộ giải ngoài vào TCB hoặc kernel kiểm bằng chứng (8.B) | bộ so lệch hai bộ giải (hiện: P1 + vét cạn); oracle; đột biến M7 | rất lớn; nghiên cứu (GP §8). Phụ thuộc: ADR | Không |
| **A3 — 8.B kernel** (BƯỚC 1 ĐÃ LÀM: `gt-kernel`, 100% chứng minh corpus có chứng chỉ, gritc CHỈ TIN KERNEL) | GP 8.B | ADR kernel | kernel nhỏ MỚI trong TCB | kernel kiểm lại bằng chứng của A2; đột biến kernel | phụ thuộc A2 | Không |
| **A4 — N2 `old`, N3 bất biến kiểu, N4 kiểu khoảng, N5 mã ma** (ĐỀ XUẤT; đi cùng A2/A3) | Opus §6.1 | ADR riêng từng mục + ADR-P2-01 | N5: Guard phải khẳng định không còn mã ma trong IR (luật mới) | từng cái: test đỏ trước, oracle động, đột biến | **mỗi mục là cú pháp/ngữ nghĩa mới** ⇒ vào đợt edition (mục 3). N3/N4 chỉ có ý nghĩa SAU A2/A3 | **CÓ** (gom ở mục 3) |
| **A5 — N6 kiểu tuyến tính** (ĐỀ XUẤT; cuối) | Opus §6.1; GP 8.C | ADR riêng | `Drop` đã hạ + loại cấu trúc mới của oracle M6 | oracle M6 mở rộng; đột biến M6 | cần `Drop` hạ (mục chưa làm: FACTCHECK #11) và `Cap<T>` affine (FACTCHECK #25 CHƯA KIỂM) | có thể (thuộc tính/lang item) — CHƯA quyết |

### B. Độc lập backend (L0–L6 của Opus §8)
**Điều kiện cứng (ĐỀ XUẤT, Giang duyệt):**
- **(i)** L1 giữ **HAI đường hạ**: `MIR → LLVM` hiện có và `GLIR → LLVM` mới, dưới cờ CHỈ có trong test (không ở `gritc` thật); so **IR** (golden) và **kết quả chạy** trên TOÀN BỘ corpus và MỌI bộ sinh hiện có (`llvm_diff*`, `std_lib`, `std2_lib`, `fn_value_gen`, `hashmap_collide`, `release_policy`…); chỉ bỏ đường cũ khi hai đường trùng trên cổng liền mạch xanh.
- **(ii)** Mỗi backend VẪN có lớp kiểm sau cùng của riêng nó: lỗi trong bước hạ không được lọt qua phần độc lập backend. GLIR Guard chỉ kiểm GLIR; sau khi hạ xuống backend, phần riêng backend (LLVM verifier, `invoke`, `unreachable`, cờ UB, FMA — FACTCHECK §4) KHÔNG được bỏ.
- **(iii)** Backend COFF/Windows xong TRƯỚC khi xét đổi mặc định dev (L5); mặc định dev vẫn LLVM -O0 cho tới đó.
- **(iv)** Lập luận cho x64 là **ĐA DẠNG** (vi sai hai backend độc lập, bỏ phụ thuộc LLVM, dịch nhanh, không unwinding). KHÔNG viết "đáng tin hơn LLVM -O0": chưa có bằng chứng; backend mới cũng có lỗi; Guard + vi sai là thứ cho độ tin.
| Mục | Nguồn | ADR | Dòng TCB | Cổng | Rủi ro / phụ thuộc | Cú pháp? |
|---|---|---|---|---|---|---|
| **B0 (=L0)** khoanh phụ thuộc LLVM + đo gritc release | Opus §8.3 | ADR-P2-02 (nháp) | không | FACTCHECK mục 4 + 10 làm sẵn; đo Q1.h #1 | không đổi hành vi | Không |
| **B1 (=L1)** `gt-layout` + GLIR, hai đường hạ | Opus §8.2 | ADR-P2-02 | GLIR + `gt-layout` vào TCB (dòng mới) | điều kiện (i) ở trên; golden GLIR; cổng liền mạch | GLIR sai ⇒ sửa mọi backend ⇒ chốt bằng ADR + golden TRƯỚC backend thứ hai | Không |
| **B2 (=L2)** tách Guard: GLIR Guard + Guard-post từng backend | Opus §8.3 | ADR-P2-02 | Guard đổi cấu trúc (dòng TCB Guard) | bộ tiêm lỗi Guard (`mut_guard.py`, `test_mut_guard.py`), đột biến Guard ≥ 1/2 tự động | điều kiện (ii); rủi ro: luật chuyển nhầm sang phần độc lập mất kiểm LLVM | Không |
| **B3 (=L3)** `gt-codegen-x64` tập con, Linux ELF; `--backend=x64` thử nghiệm | Opus §7.3, §8.3 | ADR x64 | backend mới = TCB mới + bộ mã hoá lệnh | vi sai x64 ≡ LLVM ≡ thông dịch; bộ mã hoá kiểm bằng disassembler NGOÀI TCB; đột biến bộ mã hoá | công sức lớn; ước 8–15k dòng là ƯỚC LƯỢNG (FACTCHECK §9); phụ thuộc B1, B2 | Không |
| **B4 (=L4)** đủ tập con Phase 1; Windows COFF | Opus §8.3 | như B3 | như B3 | toàn bộ corpus + bộ sinh | điều kiện (iii) | Không |
| **B5 (=L5)** xét đổi mặc định `dev` | Opus §8.3 | ADR đổi mặc định | không | cổng liền mạch xanh CẢ HAI backend; **Giang duyệt riêng** | đổi hành vi người dùng thấy | Không |
| **B6 (=L6)** Wasm / Cranelift (phụ) | Opus §7.5 | ADR riêng | backend mới | như B3 | sau B4 | Không |

### C. Dùng được cho AI
| Mục | Nguồn | ADR | TCB | Cổng | Rủi ro | Cú pháp? |
|---|---|---|---|---|---|---|
| **C1 — N1 sinh test từ hợp đồng** (`rig test --auto`; NGOÀI TCB) | Opus §6.1 | ADR ngắn | KHÔNG (chỉ tìm lỗi, không cấp phép) | test của công cụ; oracle hợp đồng; không đổi phán quyết `gritc` | công cụ sai ≠ false negative (không quyết định); phụ thuộc: hợp đồng `⊳⊲` | Không |
| **C2 — N7 `?`** | Opus §6.1; `SEED_LIMITATIONS.md:14` | ADR-P2-01 | không (hạ thành `⌥`+return sớm) nếu desugar trước M4; CHƯA KIỂM chỗ desugar | corpus + test từ chối; round-trip `fmt` | cú pháp mới ⇒ đợt edition | **CÓ** |
| **C3 — N8 edition + `gritc fix`** | Opus §6.1; FACTCHECK #24 (khẳng định FREEZE chưa kiểm được) | ADR-P2-01 | không | `rig.toml edition`; golden `fix`; cổng | PHẢI trước mọi cú pháp mới | Không (khoá cấu hình; CHƯA xác minh FREEZE cho phép) |
| **C4 — N9 nhãn đối số** | Opus §6.1; FACTCHECK #26 | ADR riêng, **CHỈ nếu Q1.h #6 cho thấy tần suất đáng kể** | không | corpus; đột biến E03xx | không có số liệu hiện tại; có thể KHÔNG làm | **CÓ** (nếu duyệt) |
| (C4: HOÃN, GÓI 10) | | | | | N9 chỉ làm khi có ≥ 3 lỗi đảo đối số cùng kiểu XÁC NHẬN (đầu ra sai, qua mọi kiểm tra) trong ≥ 2 phiên capstone độc lập (Haiku 5.5 và Sonnet 5.5); đo GÓI 10 trên nhật ký hiện có: 0 ca |
| **C5 — N10 newtype trong suốt** | Opus §6.1 | ADR nhỏ (cùng B1) | `gt-layout` bảo đảm bố cục | golden bố cục | phụ thuộc B1 | Không |
| **C6 — N11 chứng nhận theo băm** | Opus §6.1 | ADR | không (lưu trữ) | tái lập; cache ≡ biên dịch lại | cache sai = false negative; D9 tất định | Không |

### D. Hiệu năng
| Mục | Nguồn | ADR | TCB | Cổng | Rủi ro / phụ thuộc |
|---|---|---|---|---|---|
| **D1** bằng chứng release-o2 hết mức (Q82) | Opus §3.1; `DECISIONS.md:1110` | không (đã có) | không | `RELEASE_DIFF_RESULTS.jsonl` ≥ 1500 chương trình; ca float/abort | gần như thuần đo |
| **D2 — 7a** niche cơ bản + tag nhỏ | Opus §2.7 | ADR (sau GLIR) | đổi bố cục enum ⇒ Guard/TCB cập nhật | differential dev/release/release-o2; golden; oracle bố cục | **KHÔNG áp niche lên con trỏ béo** (FACTCHECK #16: ptr có thể 0); tag đổi ở ≥ 6 cụm hạ (FACTCHECK #42); phụ thuộc B1 |
| **D3** khởi tạo tại chỗ (destination-passing) | Opus §2.1 | ADR | không đổi TCB nếu chỉ hạ | `bench_profile` trước/sau; oracle abort giữa chừng (Q1.h #7) | KHÔNG phải biện pháp bảo vệ Secret (Opus §2.1 đúng) |
| **D4** Arena đặt trước địa chỉ ảo — **CHƯA ĐỦ ĐIỀU KIỆN** (GÓI 10 S2): đề xuất Opus giả định `Vec` giữ `&!Arena` ĐỘC QUYỀN; bản hiện hành là `&'a Arena` CHIA SẺ (ADR-C03). Với Arena chia sẻ chỉ khối được cấp CUỐI CÙNG mới lớn tại chỗ được ⇒ lợi ích THU HẸP; chưa đủ điều kiện cho tới khi có số `memcpy` của `Vec::push` (Q1.h #3; GÓI 10 CHƯA đo vì preflight đỏ) | Opus §2.4 | ADR | `gritrt_arena_*` đổi hợp đồng TCB | `mut_arena.py`, `shared_arena_diff`, benchmark; test CẢ HAI đường (đặt trước thất bại ⇒ lùi về chunk) | **KHÔNG nới luật M6** ("`push` vô hiệu tham chiếu cũ"); commit charge Windows (Q1.h #9) |
| **D5** kênh có kiểu (8.D) | Opus §2.8 | ADR (gồm giải ADR-C12 spawn/detach) | intrinsic TCB mới + TSan | TSan như H7/H8 (`tsan_check.sh`) | mâu thuẫn spawn/detach (FACTCHECK §6) phải giải TRƯỚC |
| **D6** biên dịch tăng dần (8.E, `gritc serve`) | Opus §2.5 | ADR | cache trong đường quyết định | cổng "tăng dần ≡ từ đầu" trên bộ sinh | cache sai = false negative; làm SAU đo Q1.h #1 |

## 3. QUY TẮC GOM CÚ PHÁP (ĐỀ XUẤT)
- Mọi mục cần cú pháp mới (**N2, N3, N4, N5, N7, N9 nếu duyệt, N6 nếu cần**) vào **MỘT đợt "edition" duy nhất**.
- Đợt đó đi **sau N8** (khoá `edition` + `gritc fix`) để mã cũ vẫn dịch được.
- `gt-lexer`/`gt-parser` **khởi động lại đóng băng và fuzz 24 giờ MỘT lần** sau đợt này (hiện đóng băng: `docs/spec/FREEZE.md:30-31`); KHÔNG thêm cú pháp rời rạc (mỗi lần sửa làm mất giờ fuzz đã tích luỹ).
- Từng mục vẫn cần ADR riêng và test-trước; việc "gom" chỉ là gom LỊCH và gom lần đóng băng, không gom quyết định.
- Ngoài đợt: không từ khoá/cú pháp/ngữ nghĩa nào mới (AGENTS.md).

## 4. THỨ TỰ ĐỀ XUẤT (Giang duyệt)
- **2.0 (không đổi cú pháp, chủ yếu tài liệu/đo):** A1 (xoá Secret); ADR thiết kế SMT (A2); B0 (L0) và ADR-P2-02 (GLIR); các phép đo ở `PHASE2_FACTCHECK.md` mục 10.
- **2.1:** B1–B2 (L1–L2) với hai đường hạ; D2 (7a, sau B1); D1 (bằng chứng release-o2).
- **2.2 (đợt cú pháp):** C3 (N8) trước; rồi C2 (N7), A4 (mục cú pháp), C4 nếu duyệt; fuzz lexer/parser 24 giờ MỘT lần.
- **2.3:** A2 (8.A SMT) + N2–N5; C1 (N1).
- **2.4:** B3–B4 (x64, COFF), B6 (Wasm), A3 (8.B kernel), D5 (kênh), A5 (8.C, N6).
**Khác thứ tự của Opus (§3) và lý do — an toàn trước hiệu năng:**
1. Opus xếp "khép khoảng cách backend" (release-o2) số 1, rồi 7a, rồi 1b. Ở đây A1 (xoá Secret) lên đầu vì là lỗ hổng an toàn thật (FACTCHECK §3), không phải tối ưu.
2. Opus xếp 7a ngay sau backend. Ở đây 7a chờ B1 (GLIR) vì tag đổi chạm ≥ 6 cụm hạ (FACTCHECK #42) và niche con trỏ béo không dùng được (#16).
3. Opus xếp N7 (`?`) và N1 sớm. Ở đây `?` vào đợt edition duy nhất (mục 3); N1 sau SMT vì dùng hợp đồng đã được chứng minh làm oracle.
4. Opus để x64 song song 8.A sau L2; ở đây x64 sau khi có hai đường hạ và Guard tách (điều kiện (i)–(ii)).
5. Kênh có kiểu (D5) cuối vì phải giải mâu thuẫn spawn/detach trước.

## 5. KHÔNG LÀM (giữ lý do từ đề xuất Opus)
| Không làm | Lý do (từ Opus) |
|---|---|
| Enum cỡ biến thiên (§2.2) | mất truy cập chỉ số O(1); bố cục mới phải vào TCB/Guard; lợi ích chỉ có số tự báo (Goose) |
| Tham chiếu tương đối (§2.3) | Arena nhiều chunk không liên tục ⇒ M7 không có cận; "đọc thẳng từ đĩa" là `unsafe` thật |
| Nén theo miền giá trị của M7 (7b, §2.7) | M7 chứng minh theo điểm chương trình, không theo kiểu; lỗi M7 ⇒ hỏng bộ nhớ; ngược P6. Đường đúng (kiểu khoảng tường minh N4) sau A2/A3 |
| `comptime` (§6.2) | gần macro/eval (D8), phá ngân sách bước (D20) |
| Tham chiếu thế hệ (§6.2) | kiểm lúc chạy thay chứng minh tĩnh (P2/P4) |
| Effect handlers (§6.2) | cần closure + continuation |
| GC / ARC (§6.2) | trái mục 4.0(a) |
| Unwinding (§6.2) | trái D3 |
| Reference capabilities (§6.2) | trùng sở hữu + `Send`/`Sync`-like |
| AST/MIR phẳng (§2.6) | MIR đã phẳng; AST là ADR-013; chưa có số đo |

## 6. TIÊU CHÍ XONG (kiểm được bằng máy), RỦI RO, CHƯA KIỂM — theo mục
Chung cho MỌI mục: test ĐỎ trước khi sửa; ADR viết TRƯỚC mã; dòng TCB mới có hợp đồng trước; đột biến tự động ≥ một nửa (`tools/mut.py`; M6/M7/M8/Guard); differential codegen ≡ thông dịch ở dev/release/release-o2; cổng liền mạch `bash tools/gate.sh D:/Grit` xanh trên ĐÚNG mã đó; benchmark báo **min** và trung vị. Tên test cụ thể do ADR từng mục chốt (CHƯA đặt tên ở đây: tên chỉ có nghĩa khi có thiết kế).
| Mục | XONG khi (máy kiểm) | Rủi ro chính | CHƯA KIỂM |
|---|---|---|---|
| A1 | test `secret_wiped_*` (tên ĐỀ XUẤT) đỏ→xanh ở dev/release/release-o2; đột biến "bỏ ghi" bị bắt; `GATE_RESULT=GREEN` | giới hạn phạm vi bảo vệ bị hiểu thành "an toàn tuyệt đối" | codegen có chèn được tại mọi điểm kết thúc phạm vi (kể cả đường abort) |
| A2 | hai bộ giải đồng ý trên oracle; đột biến M7 | độ phức tạp | mọi thứ (nghiên cứu) |
| A3 | kernel kiểm lại mọi bằng chứng A2 | kernel lớn lên | mọi thứ |
| A4 | từng mục: corpus pass/fail + oracle + đột biến | cú pháp lẻ | tương tác N3/N4 với M7 |
| A5 | oracle M6 loại mới xanh; fix-off bị bắt | cần Drop hạ | `Cap` affine |
| B0 | `docs` ghi bảng phụ thuộc + số đo gritc release | — | đo |
| B1 | hai đường hạ trùng: golden IR + kết quả trên toàn corpus + bộ sinh; `GATE_RESULT=GREEN` | GLIR sai | mọi thứ |
| B2 | bộ tiêm lỗi Guard đỏ khi tắt từng luật; đột biến Guard ≥ 1/2 tự động | luật rơi mất | luật phụ thuộc ngầm |
| B3 | vi sai x64 ≡ LLVM ≡ thông dịch; disassembler oracle; đột biến bộ mã hoá | hai nơi có bug | cỡ thật, tốc độ |
| B4 | toàn corpus + bộ sinh xanh trên Windows và Linux | COFF/linker | mọi thứ |
| B5 | cổng xanh CẢ HAI backend + Giang ký | đổi mặc định | — |
| C1 | `rig test --auto` tìm lại các FB đã biết (≥ N, N do ADR chốt) | báo sai làm phiền | tỷ lệ phát hiện |
| C2 | corpus `?` pass/fail; round-trip `fmt`; không đổi phán quyết chương trình cũ | cú pháp | chỗ desugar |
| C3 | `gritc fix` chuyển mã cũ→mới, golden | FREEZE có cho phép? (#24) | #24 |
| C4 | chỉ khi Q1.h #6 > ngưỡng do Giang đặt | đo thấp ⇒ bỏ | tần suất |
| C5 | golden bố cục `repr(transparent)` | — | — |
| C6 | cache ≡ biên dịch lại trên bộ sinh | cache sai | mọi thứ |
| D1 | ≥ 1500 chương trình dev≡release-o2 (`RELEASE_DIFF_RESULTS.jsonl`) | — | tăng tốc thực |
| D2 | golden + diff xanh; cỡ enum giảm đo được | niche sai ⇒ hỏng bộ nhớ | lợi ích byte |
| D3 | `bench_profile` trước/sau; oracle abort xanh | thứ tự đánh giá | lợi ích |
| D4 | `mut_arena.py` bị bắt; hai đường xanh | commit charge | lợi ích |
| D5 | TSan sạch 8 mẫu × 50; ADR-C12 giải | đua dữ liệu | — |
| D6 | tăng dần ≡ từ đầu | cache | — |

## 7. SƠ ĐỒ PHỤ THUỘC (văn bản)
```text
ĐIỀU KIỆN VÀO (seed ký, §7, cổng rc6 xanh, lệnh của Giang)
   │
   ├── A1 xoá Secret ───────────────────────────────────────────────┐
   ├── B0 (L0 khảo sát + đo) ──► ADR-P2-02 (GLIR) ──► B1 (L1) ──► B2 (L2) ──► B3 (x64 ELF) ──► B4 (COFF) ──► B5 (xét dev) ──► B6
   │                                                      │                                      
   │                                                      └──► D2 (7a, niche mỏng) , C5 (newtype)
   ├── ADR SMT ──► A2 (8.A) ──► A3 (8.B) ──► A4 (N2,N3,N4,N5)         [N3,N4 cần A2/A3]
   │                  └────────► C1 (N1 sinh test)
   ├── C3 (N8 edition + fix) ──► ĐỢT CÚ PHÁP (C2 N7, A4-cú-pháp, C4?, A5?) ──► fuzz 24h lexer/parser MỘT lần
   ├── D1 (release-o2 bằng chứng)
   ├── ADR-C12 (spawn/detach) ──► D5 (kênh có kiểu + TSan)
   └── D3, D4 (chỉ sau đo Q1.h) ; D6 (sau đo gritc release)
A5 (N6) cần: Drop đã hạ + oracle M6 mở rộng.   A2/A3 cần ADR SMT trước.
```
KHÔNG hứa mốc ngày.
