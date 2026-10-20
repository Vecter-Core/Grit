### G10_mutfuzz — [bản sao] fuzz đột biến token qua CLI gritc (ICE/crash/treo)

- **Trạng thái:** FAIL (mã thoát 1, 122s, bắt đầu 2026-10-20T11:57:02Z)
- fuzz đột biến token qua CLI gritc: 8000 mẫu, seed 11; ICE=1; OK_GREEN=2068; OK_RED=5931
- ICE x1: lỗi nội bộ của compiler (ICE): MIR của `m_NaNfNfaN::pass_through` không hợp lệ:

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
SUMMARY: fuzz đột biến token qua CLI gritc: 8000 mẫu, seed 11; ICE=1; OK_GREEN=2068; OK_RED=5931
SUMMARY: ICE x1: lỗi nội bộ của compiler (ICE): MIR của `m_NaNfNfaN::pass_through` không hợp lệ: 
```
</details>
