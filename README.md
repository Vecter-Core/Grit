# Grit

**Trạng thái: `1.0.0-rc8` (bản ứng viên, CHƯA phải Seed).** Chưa đạt các điều kiện cứng còn mở trong `docs/READINESS.md`; đọc `docs/KNOWN_LIMITATIONS.md` trước khi dùng.

**Nhãn `main` trên GitHub (`Vecter-Core/Grit`):** `main` = `e8a8ae5` (rc7): cổng Windows XANH, cổng Linux XANH (WSL2 cục bộ, không phải CI) trên `4ef044e`; cây `main` khác `4ef044e` đúng một dòng `license` (`docs/REMOTE_STATE.md`). rc8 (bản cục bộ ở trên) CHƯA lên `main`.

**Nói thẳng về mức tin cậy:** Grit an toàn *bằng kiểm tra* (trình biên dịch fail-closed: không chứng minh được thì từ chối), corpus có oracle, đột biến và đọc mã cùng họ model. **Chưa có kiểm toán độc lập**, chưa có bằng chứng hình thức đầy đủ, và còn điều kiện cứng mở. Điều được và không được khẳng định: [`docs/SAFETY_CLAIMS.md`](docs/SAFETY_CLAIMS.md); giới hạn đã biết: [`docs/SEED_LIMITATIONS.md`](docs/SEED_LIMITATIONS.md). Báo lỗi an toàn (chấp nhận sai): xem [`SECURITY.md`](SECURITY.md). Chưa có giấy phép (LICENSE): mặc định mọi quyền được bảo lưu.

**Nhánh:** `main` = commit có CI xanh, hoặc commit CHỈ-TÀI-LIỆU (hay công cụ/test/CI, không đổi hành vi sản phẩm) ngay sau đó — kiểm bằng `bash tools/main_check.sh <commit CI xanh> <đầu main>`; chỉ fast-forward (không force, không squash). `windows-work` = nhánh làm việc (mặc định trên GitHub; đề xuất đổi sang `main` khi sẵn sàng).

Grit là ngôn ngữ lập trình an toàn theo thiết kế (Phase 1 — Seed): trình biên dịch **`gritc`** kiểm tra sở hữu/mượn, nghĩa vụ chứng minh (tràn số, chia 0, chỉ số mảng), hiệu ứng, capability, tính dừng của đệ quy/vòng lặp và rò rỉ bí mật **trước khi** sinh mã; trình quản lý gói **`rig`**. Chương trình bị ĐỎ thì không có output.

Mã nguồn Grit là tệp `.gt`. Mỗi ký hiệu Unicode có đúng một bí danh ASCII (`ƒ` = `\fn`, `≔` = `:=`, `→` = `->`, `∅` = `\nil`, …; bảng đầy đủ ở `docs/spec/symbols.md`) — viết bằng bí danh rồi chạy `gritc fmt` để ra dạng chuẩn.

## Dựng

| Nền tảng | Cần | Ghi chú |
|---|---|---|
| Windows x86_64 (MSVC) — nền tảng chính | Rust ≥ 1.98, LLVM 20.1.8 bản dựng sẵn (`GRIT_LLVM_DIR`) | xem `docs/ENVIRONMENT.md` |
| Linux x86_64 | Rust ≥ 1.97, `libLLVM-20.so` (gói `llvm-20-dev`/`libllvm20`), `cc` | `GRIT_LLVM_LIB_DIR` nếu không ở `/usr/lib/x86_64-linux-gnu` |

```sh
cargo build --release --manifest-path runtime/gt-rt/Cargo.toml   # runtime (staticlib, dựng riêng vì panic = abort)
cargo build --release -p gt-driver -p rig                        # gritc, rig
```

Không có LLVM: `cargo build --release -p gt-driver --no-default-features` dựng `gritc` chỉ có backend thông dịch (`--backend=interpreter`).

## Dùng

```sh
gritc check chuongtrinh.gt        # kiểm tra (0 xanh · 1 đỏ · 2 lỗi nội bộ · 3 sai cách dùng)
gritc run   chuongtrinh.gt        # kiểm tra + biên dịch native (LLVM) + chạy
gritc build chuongtrinh.gt -o ra  # sinh tệp thực thi
gritc fmt   chuongtrinh.gt        # định dạng chuẩn (không đổi ngữ nghĩa)
gritc explain E0401               # giải thích một mã lỗi
```

Tên tệp là tên module nên phải là định danh hợp lệ (`main.gt`, không phải `01_add.gt`).

```
ƒ fib(n: u32) → u32 ! ∅
  ⊳ n <= 40            // tiền điều kiện (được bộ giải chứng minh ở chỗ gọi)
  ↓ n                  // biến thể giảm: chứng minh đệ quy dừng
{ ⎇ n < 2 { n } ⇝ { fib(n - 1) + fib(n - 2) } }
ƒ main() → u32 ! ∅ { fib(20) }
```

## Kiểm thử

```sh
cargo test --workspace -- --test-threads=2     # ~40 phút; test vét cạn/oracle rất nặng, đừng tăng số luồng
python3 tools/check_expectations.py            # corpus tests/{pass,fail,warn} khớp docs/spec/error_codes.md
```

## Tài liệu

**Muốn nắm toàn cảnh (dự án là gì, có mấy phần, đã tới đâu, cái gì cần duyệt): đọc `docs/PROJECT_STATUS.md`.**

`GRIT_PLAN.md` (kế hoạch), `docs/spec/` (ngữ pháp, ngữ nghĩa, bảng ký hiệu, mã lỗi), `docs/errors/<mã>.md` (từng mã lỗi), `docs/DECISIONS.md` (ADR), `docs/KNOWN_LIMITATIONS.md`, `docs/READINESS.md`, `examples/` (6 công cụ viết bằng Grit: wc, sha256, sortlines, stats, wordfreq, json).
