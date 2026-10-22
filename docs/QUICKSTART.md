# QUICKSTART — chương trình Grit đầu tiên (10 dòng)

Linux x86_64 là nền đã kiểm. Windows: **chưa kiểm** (xem `BUILDING.md`).

```text
1. git clone <địa-chỉ-repo> Grit && cd Grit
2. bash tools/setup_cloud.sh              # Linux: cài Rust + LLVM 20.1.8, dựng gritc/rig/gt-rt (Windows: pwsh tools/install.ps1)
3. export PATH="$PWD/target/release:$PATH"   # Windows: $env:PATH = "$PWD\target\release;$env:PATH"
4. gritc doctor                           # kiểm môi trường; mọi dòng phải OK
5. rig new hello && cd hello              # gói mới đã có phụ thuộc `std`
6. sửa src/main.gt (mẫu bên dưới)
7. rig lock                               # BẮT BUỘC sau mỗi lần sửa mã
8. rig run                                # kiểm + dựng + chạy → in dòng chào
9. gritc check --json src/main.gt         # (tuỳ chọn) đầu ra máy cho công cụ/AI
10. đọc docs/AI_GUIDE.md rồi viết chương trình của bạn
```

`src/main.gt`:
```grit std run
ƒ main() ! {io} {
  out ≔ root_out();
  out.write_str("xin chào Grit\n");
}
```
```output
xin chào Grit
```

Gặp lỗi `E…`? Đọc `docs/AI_GUIDE.md` mục 9, hoặc `docs/errors/<mã>.md`. Bạn đang dùng AI để viết mã? Xem `docs/QUICKSTART_CLOUD.md`.
