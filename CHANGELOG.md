# CHANGELOG

Định dạng: mới nhất ở trên. Số phiên bản ở `gritc --version` / `Cargo.toml` (gt-driver, rig). Nền đã kiểm: Linux x86_64 (dòng Seed) và Windows x86_64 (cổng 2026-11-18, xem `docs/RUN_REPORTS.md`).

## 1.0.0-rc8 — GÓI 12: FB-C36 (chu trình qua hằng □ được chấp nhận; gt-term thêm cạnh Const::Item), cổng Linux rc7, công cụ (2026-11-20)
- **Sửa soundness (FB-C36):** hằng `□` được hạ thành LỜI GỌI hàm ở mỗi chỗ dùng; `□ C ≔ C`, vòng hằng và `□ C ≔ f()` với `f` đọc `C` (kể cả qua `⊳` hoặc phương thức trait) từng `check` XANH và tràn ngăn xếp khi chạy. Nay E0703. `gt-term` `const_cycles`; test `const_cycle` (12; bộ sinh 160 chương trình: tắt sửa 160/160 chấp nhận sai, bật 0/160), 3 tệp `tests/fail` + 1 `tests/pass`.
- Công cụ: `tools/gate.sh` chia gói `gt-driver` theo mục tiêu (`GATE_SPLIT_PKGS`, in `secs=`); `tools/fuzz_import_local.py`; `FUZZ_TOOLCHAIN`; log fuzz ghi `source`/`os`/`kernel` trên Linux.

## 1.0.0-rc7 — GÓI 11: FB-C35 (đệ quy qua thân hợp đồng ⊳/⊲ được chấp nhận; gt-term gộp cạnh hợp đồng), ghim rustc 1.98.1, công cụ (2026-11-20)

## 1.0.0-rc6 — GÓI 8: FB-C34, ICE 33 hợp đồng, E1001 oan, công cụ audit/kit (2026-11-19)
- **Sửa soundness (FB-C34):** `gt-term`: callee không có thân, có tham số kiểu hàm, nhận đối số có thể mang hàm (vd `spawn(id(f), n)`) ⇒ cạnh gián tiếp tới mọi hàm bị thoát ⇒ E0703. Test đỏ trước, bộ sinh `spawn_family`.
- `gt-oblig`: ≥ 33 hợp đồng mỗi lời gọi ⇒ ICE E9901 (không panic); giới hạn độ sâu `has_raw_ptr` 32 → 256 (tuple lồng không còn E1001 oan).
- Công cụ/tài liệu: `make_attack_kit.sh`, `import_attack.py`, `adversarial_check.py`, `fixture_secret_check.py`, `reports_index.py`, `call_edge_probe.py`, `docs/CALL_EDGE_AUDIT.md`, ADR-C12/C13.

## 1.0.0-rc5 — GÓI 7: FB-C32 (vòng không dừng được chấp nhận khi hết 24 biến bóng), kho công khai, fuzz CI (2026-11-19)
- **Sửa soundness (FB-C32):** `gt-oblig` `judge_variant_edge`: vòng thiếu biến bóng riêng không còn được chứng minh bằng biến bóng cũ ⇒ E0702. Test đỏ trước (`ghost_limit_soundness`), oracle hàm rộng (`ghost_wide_oracle`), mẫu corpus `E0702_ghost_limit_stale_ghost`.
- Công cụ/tài liệu: `tools/secret_scan_history.py`, SECURITY.md, issue template, workflow `fuzz.yml` (chỉ dispatch), `tools/fuzz_ingest.py`, `docs/AI_PITFALLS.md`, `tools/limits_probe.py` + LIMITS_AUDIT cập nhật.

## 1.0.0-rc4 — GÓI 5/6: kiểm chứng CI, độ phủ nhánh, giới hạn kích cỡ (nhánh `integration`, 2026-11-19)
Nhãn vẫn là **RC**, không phải `1.0.0-seed` (A2 chưa đạt; `docs/SEED_FINISH_LINE.md`).
- **Thay đổi mã sản phẩm duy nhất:** `gt-term` — thông báo E0702 của vòng lặp nêu nguyên nhân và gợi ý tách hàm khi hàm vượt giới hạn 24 biến/vòng lặp của bộ chứng minh (test ĐỎ trước `crates/gt-term/tests/limit_hint.rs`). KHÔNG đổi phán quyết nào.
- **Golden:** `tests/golden_ll/{checked_add,slice_sum,static_atomic}.ll` chỉ đổi BĂM CHỨNG THỰC (6 dòng) do chuỗi phiên bản nằm trong băm.
- Công cụ/tài liệu: `tools/{main_check.sh,coverage_branches.py,scale_probe.py,platform_tests.py,...}`, `apps/{csv2json,todo,difflite}`, `docs/{LIMITS_AUDIT,APPS_LOG,SEED_FINISH_LINE,PLATFORM_TESTS,DECISION_COVERAGE}.md`.

## 1.0.0-rc3 — hợp nhất hai dòng, Arena chia sẻ, kiểm chứng Windows (nhánh `integration`, 2026-11-18)
Nhãn vẫn là **RC**, không phải `1.0.0-seed` (A2 an toàn chưa đạt: `docs/READINESS.md` mục "Seed 1.0"). **Sửa nhãn:** các tài liệu cũ ghi `gritc --version` là `1.0.0-rc2`, nhưng `crates/gt-driver/Cargo.toml` thực tế ở `1.0.0-rc1` (bản đổi rc2 không vào mã); rc3 là lần đầu số phiên bản KHỚP với mã. Thẻ cục bộ `seed-1.0-rc2` đã đổi tên `seed-1.0-rc2-old` (chưa từng đẩy).
- **Thay đổi hành vi:** `spawn` thất bại ⇒ abort `spawn_failed` (mã 70) thay vì `abort: unknown` (hợp đồng H8); runtime `gt-rt` có feature `test-fault` CHỈ cho kiểm thử.
- **Công cụ/cổng:** điểm lưu của `tools/gate.sh` gắn BĂM CÂY (nội dung tệp mới), in `GATE_TREE`/`GATE_STAGE_TREE`; công cụ nặng (`run_guarded`, `mut_batch`, `mut.py run`, cổng đầy đủ) tự từ chối khi preflight FAIL; `tools/unsafe_inventory.py` + `docs/unsafe_baseline.json` trong cổng; `tools/stress_spawn.sh` (thủ công).
- **Golden:** `tests/golden_ll/{checked_add,slice_sum,static_atomic}.ll` chỉ đổi BĂM CHỨNG THỰC (6 dòng) do chuỗi phiên bản nằm trong băm.
- Chi tiết các giai đoạn: `docs/RUN_REPORTS.md` (P1.e–P4, GÓI 3).

## 1.0.0-rc2 — Seed 1.0 (nhánh `claude/gallant-darwin-m251cq`, 2026-10-23)
Nhãn là **RC**, không phải `1.0.0-seed`: danh sách điều kiện còn thiếu ở `docs/READINESS.md` (mục "Seed 1.0") và `docs/RUN_REPORTS.md` (S8).

### Thêm
- **Ngôn ngữ / thư viện:** đủ để viết bốn capstone (`wc`, `json`, `kvstore`, `sortlines`…) mà KHÔNG cần `?`/phân rã tuple/closure (vẫn chưa có): `Vec`/`Text`/`HashMap` mở rộng, `[T]::sort`, băm, `core::exit`, đọc `Cap<In>::read_all`, ghi `Cap<Fs>::write`, `&str ==` bằng byte (`gritrt_bytes_eq`), `[u8]::equals`, `find_byte_or_end`… (xem `docs/STD_REFERENCE.md`, sinh tự động).
- **Công cụ:** `gritc doctor`, `gritc lsp` (chẩn đoán UTF-16 + định dạng; `editors/vscode` khung), `rig new` tự thêm phụ thuộc std, `tools/setup_cloud.sh`, `tools/install.sh/.ps1`, CI Linux/Windows (`.github/workflows/ci.yml`; Windows chỉ viết, CHƯA chạy).
- **Tài liệu máy kiểm:** QUICKSTART, QUICKSTART_CLOUD, GRIT_BOOK, LANGUAGE_REFERENCE, AI_GUIDE (≤ 12 000 token proxy), STD_REFERENCE (sinh), SAFETY_CLAIMS; mọi khối ```` ```grit ```` được `tools/doc_test.py` biên dịch/chạy trong cổng.
- **Cổng:** giai đoạn `failopen` (quét mẫu im lặng-chấp-nhận, baseline), `tools/fuzz_loop.sh` + fuzz chuỗi kiểm đầy đủ (`crates/gt-driver/fuzz`), `docs/FUZZ_LOG.jsonl` tích luỹ.
- **Bằng chứng:** `docs/M7_DANGER_MUTANTS.md` (đột biến nhóm nguy hiểm của bộ giải M7), `docs/FAIL_OPEN_AUDIT.md` §8–9 (471 vị trí đã đọc; 5 nhóm TCB đọc đối kháng), `docs/archive/adversarial/`.

### Sửa (lỗ hổng thật, mỗi cái có test ĐỎ trước ở commit riêng; chi tiết `docs/FIELD_BUGS.md`)
- FB-C1…C9 (kiểm toán ngoài 2026-10-20/21), FB-C10 (`Copy` generic dưới `K: Copy`), FB-C12 (thông dịch ghi qua lát), FB-C13 (`==` trên `&str`/tuple), FB-C14 (`as` số thực suy diễn).
- **S7 (đọc mã đối kháng + quét fail-open):** FB-C15 (đệ quy qua `[f; N]`), FB-C16/C17 (`Copy` cho `W<T>` thiếu `T: Copy`; `Copy` cho Arena/Mutex/Atomic/Cell; `Cell::get` đòi `T: Copy`), FB-C18 (`main` có tham số miễn E0310; `Cap` bọc trong struct), FB-C19 (`==` trên biến suy diễn), **FB-C20 (kiểm biên lát dùng độ dài cũ ⇒ đọc/ghi ngoài mảng)**, **FB-C21 (thứ tự khởi tạo `■` qua hàm ⇒ gọi con trỏ NULL)**, FB-C22 (ordering Atomic bị `&!`), FB-C23 (`alloca` trong vòng lặp), FB-C24 (`imut` ADT lạ), FB-C25 (`⊳` trait qua generic), FB-C26 (nguyên tử `unrep`), FB-C27 (Guard: địa chỉ khai báo ngoài).

### Đổi (ghi rõ vì thu hẹp bề mặt, fail-closed)
- Thêm E0901 cho `⊳`/`⊲` của thành viên trait gọi qua tham số kiểu (ADR-C05); `⊳` trên hàm dùng làm giá trị ⇒ E0901 (ADR-C02); `Vec`/`Text` chia sẻ `&Arena` (ADR-C03); `==` chỉ cho số nguyên/bool/char/`&str` (FB-C13); `Cell::get` đòi `T: Copy` (FB-C17).
- Phiên bản gritc `1.0.0-rc1` → `1.0.0-rc2` (băm attestation đổi ⇒ golden IR bless lại).

### Không đổi / chưa có
- Windows chưa kiểm; Rig chưa có kho gói; chưa có closure/`?`/phân rã tuple/hover LSP (xem `docs/SEED_LIMITATIONS.md`).

## 1.0.0-rc1 — mốc S5 (2026-10-22, tag cục bộ `seed-1.0-rc1`; đẩy tag lên origin bị proxy chặn HTTP 403)
- Cổng xanh 53286a7, build tái lập (hai lần cùng SHA-256), capstone 8/8, `rig` + `setup_cloud.sh` + `gritc doctor`.

## trước đó
- Lịch sử chi tiết: `docs/PROGRESS.md`, `docs/HANDOFF_ARCHIVE.md`, `git log`.
