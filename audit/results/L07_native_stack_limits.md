### L07_native_stack_limits — [bản sao 2] chương trình HỢP LỆ gây tràn ngăn xếp native: có abort có kiểm soát (mã 70) không

- **Trạng thái:** FAIL (mã thoát 1, 1s, bắt đầu 2026-10-20T11:47:48Z)
- D3 đòi abort có kiểm soát (70) khi lỗi không khôi phục; nhận mã khác 0/70/71 = tiến trình bị HĐH giết (tràn ngăn xếp), gt-rt không có bảo vệ

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
gritc: phán quyết green; 0 lỗi, 0 cảnh báo, 0 ghi chú; backend: interpreter
stack_array: exit=2 :: gritc: lỗi nội bộ khi chạy: chương trình bị hệ điều hành dừng
gritc: phán quyết green; 0 lỗi, 0 cảnh báo, 1 ghi chú; backend: interpreter
stack_recursion: exit=2 :: gritc: lỗi nội bộ khi chạy: chương trình bị hệ điều hành dừng
SUMMARY: D3 đòi abort có kiểm soát (70) khi lỗi không khôi phục; nhận mã khác 0/70/71 = tiến trình bị HĐH giết (tràn ngăn xếp), gt-rt không có bảo vệ
```
</details>
