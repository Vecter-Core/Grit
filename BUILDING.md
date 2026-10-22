# BUILDING — dựng Grit từ nguồn

Nền **đã kiểm**: Linux x86_64 (Ubuntu 24.04, `libllvm20` = LLVM 20.1.2, `cc` = gcc 13). **Windows: chưa kiểm** sau các thay đổi của nhánh Seed 1.0.

## Linux (một lệnh)
```text
bash tools/setup_cloud.sh      # Rust + libllvm20 + dựng runtime/gritc/rig (release) + gritc doctor; idempotent
export PATH="$PWD/target/release:$PATH"
gritc doctor                   # mọi dòng OK, cuối: doctor: XANH
```
Hoặc `bash tools/install.sh` (như trên rồi liên kết `gritc`, `rig` vào `~/.local/bin`). Dựng thủ công:
```text
export GRIT_LLVM_LIB_DIR=/usr/lib/x86_64-linux-gnu         # thư mục chứa libLLVM-20.so (gói libllvm20)
cargo build --release --manifest-path runtime/gt-rt/Cargo.toml   # runtime (workspace riêng, panic = abort)
cargo build --release -p gt-driver -p rig --bin gritc --bin rig
```
- `gritc` NHỚ đường dẫn `runtime/gt-rt/target/release/libgt_rt.a` theo thư mục kho lúc dựng (đặt `GRIT_RT_LIB=<tệp>` để ghi đè): đừng di chuyển kho sau khi dựng.
- Không có `libllvm20` trong apt? `tools/setup_cloud.sh` thử tải LLVM 20.1.8 dựng sẵn (đường dự phòng CHƯA kiểm); hoặc tự đặt `GRIT_LLVM_LIB_DIR`.
- Dựng không backend llvm (chỉ kiểm + thông dịch): `cargo build -p gt-driver --no-default-features`.

## Windows (CHƯA KIỂM)
Cần: Rust msvc, Visual Studio Build Tools (link.exe), LLVM **20.1.8** dựng sẵn (`clang+llvm-20.1.8-x86_64-pc-windows-msvc.tar.xz`) giải nén và đặt `GRIT_LLVM_DIR` (xem `docs/ENVIRONMENT.md`), rồi `pwsh tools/install.ps1`. `LLVM-C.dll` phải nằm trong `PATH` lúc chạy `gritc`.

## Kiểm tra
```text
bash tools/gate.sh .                 # cổng đầy đủ (≈ 50 phút trên 4 lõi; điểm lưu: chạy lại cùng lệnh để tiếp; GATE_BUDGET_SEC=540 chia nhỏ)
python3 tools/doc_test.py            # mọi khối ```grit trong tài liệu
(cd capstone/reference && cargo build --release) && python3 capstone/verify.py   # K1–K4 ở dev và release-o2
```

## Dựng tái lập (cùng mã ⇒ cùng SHA-256)
```text
cargo build --locked --release -p gt-driver --bin gritc; a=$(sha256sum target/release/gritc | cut -d' ' -f1)
cargo clean;  cargo build --locked --release -p gt-driver --bin gritc; b=$(sha256sum target/release/gritc | cut -d' ' -f1)
test "$a" = "$b" && echo TAI_LAP
```
Với chương trình Grit: `rig build` hai lần cho cùng nhị phân (`rig.rs::builds_are_reproducible_byte_for_byte`). Ghi chú: `gritc` nhúng đường dẫn tuyệt đối của kho (CARGO_MANIFEST_DIR) nên tái lập đòi CÙNG đường dẫn.

## Sự cố thường gặp
| triệu chứng | cách sửa |
|---|---|
| `không thấy libLLVM-20.so` khi dựng | cài `libllvm20` hoặc đặt `GRIT_LLVM_LIB_DIR` |
| `gritc doctor`: LỖI runtime | `cargo build --release --manifest-path runtime/gt-rt/Cargo.toml` hoặc `GRIT_RT_LIB` |
| `gritc doctor`: LỖI trình liên kết | `sudo apt-get install build-essential` |
| `rig` mã 10 "rig.lock không khớp" | `rig lock` (sau MỌI lần sửa mã gói) |
| hết dung lượng đĩa | xoá `target/`, `target/tmp`; môi trường đám mây: dung lượng ghi cố định theo phiên |
