# CHANGELOG

Định dạng: mới nhất ở trên. Số phiên bản ở `gritc --version` / `Cargo.toml` (gt-driver, rig). Nền đã kiểm: Linux x86_64; Windows CHƯA kiểm.

## 1.0.0-rc2 — Seed 1.0 (nhánh `claude/gallant-darwin-m251cq`, 2026-10-23)
Nhãn là **RC**, không phải `1.0.0-seed`: danh sách điều kiện còn thiếu ở `docs/READINESS.md` (mục "Seed 1.0") và `docs/RUN_REPORTS.md` (S8).

### Thêm
- **Ngôn ngữ / thư viện:** đủ để viết bốn capstone (`wc`, `json`, `kvstore`, `sortlines`…) mà KHÔNG cần `?`/phân rã tuple/closure (vẫn chưa có): `Vec`/`Text`/`HashMap` mở rộng, `[T]::sort`, băm, `core::exit`, đọc `Cap<In>::read_all`, ghi `Cap<Fs>::write`, `&str ==` bằng byte (`gritrt_bytes_eq`), `[u8]::equals`, `find_byte_or_end`… (xem `docs/STD_REFERENCE.md`, sinh tự động).
- **Công cụ:** `gritc doctor`, `gritc lsp` (chẩn đoán UTF-16 + định dạng; `editors/vscode` khung), `rig new` tự thêm phụ thuộc std, `tools/setup_cloud.sh`, `tools/install.sh/.ps1`, CI Linux/Windows (`.github/workflows/ci.yml`; Windows chỉ viết, CHƯA chạy).
- **Tài liệu máy kiểm:** QUICKSTART, QUICKSTART_CLOUD, GRIT_BOOK, LANGUAGE_REFERENCE, AI_GUIDE (≤ 12 000 token proxy), STD_REFERENCE (sinh), SAFETY_CLAIMS; mọi khối ```` ```grit ```` được `tools/doc_test.py` biên dịch/chạy trong cổng.
- **Cổng:** giai đoạn `failopen` (quét mẫu im lặng-chấp-nhận, baseline), `tools/fuzz_loop.sh` + fuzz chuỗi kiểm đầy đủ (`crates/gt-driver/fuzz`), `docs/FUZZ_LOG.jsonl` tích luỹ.
- **Bằng chứng:** `docs/M7_DANGER_MUTANTS.md` (đột biến nhóm nguy hiểm của bộ giải M7), `docs/FAIL_OPEN_AUDIT.md` §8–9 (471 vị trí đã đọc; 5 nhóm TCB đọc đối kháng), `docs/adversarial/`.

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
