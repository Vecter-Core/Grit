### L06_native_unsound_demo — [bản sao 2] chạy THẬT chương trình XANH nhưng dùng-sau-reset Arena (nhân đôi tay cầm): có đọc nhầm không

- **Trạng thái:** FAIL (mã thoát 1, 0s, bắt đầu 2026-10-20T11:45:33Z)
- kỳ vọng đúng ngữ nghĩa: main trả 1234 (giá trị đã cấp); nhận giá trị khác = bộ nhớ đã bị tái dùng sau reset (UB) trong chương trình được gritc cho qua

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
gritc check: exit=0 (gritc: phán quyết green; 0 lỗi, 0 cảnh báo, 0 ghi chú; backend: interpreter)
profile=dev: chạy xong (backend: llvm); main trả về 99
profile=release: chạy xong (backend: llvm); main trả về 99
profile=release-o2: chạy xong (backend: llvm); main trả về 99
SUMMARY: kỳ vọng đúng ngữ nghĩa: main trả 1234 (giá trị đã cấp); nhận giá trị khác = bộ nhớ đã bị tái dùng sau reset (UB) trong chương trình được gritc cho qua
```
</details>
