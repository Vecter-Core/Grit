### P01_proposed_fix_f01 — [bản sao 3] ĐỀ XUẤT sửa F-01 (9 dòng ở gt-borrowck/check.rs): 14 probe có bị chặn không, corpus/test có hồi quy không

- **Trạng thái:** PASS (mã thoát 0, 240s, bắt đầu 2026-10-20T12:05:34Z)
- 181 probe tự viết; FALSE_NEGATIVE=9; KNOWN_GAP=3; OK=169
- corpus qua dòng lệnh gritc: 855 ca; khớp 855; lệch 0
- fail:OK = 438
- fail:OK_EXTRA = 2
- đề xuất sửa F-01: ĐẠT — 14 probe bị chặn, corpus 855/855, gt-borrowck xanh
- **cargo test:** 57 đạt / 0 lỗi / 0 ignore trong 11 bộ test

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
SUMMARY: 181 probe tự viết; FALSE_NEGATIVE=9; KNOWN_GAP=3; OK=169
== corpus qua CLI (kỳ vọng 855/855)
SUMMARY: corpus qua dòng lệnh gritc: 855 ca; khớp 855; lệch 0
SUMMARY: fail:OK = 438
SUMMARY: fail:OK_EXTRA = 2
== cargo test -p gt-borrowck (kỳ vọng không hồi quy)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.98s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.72s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.35s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 205.82s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.34s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
SUMMARY: đề xuất sửa F-01: ĐẠT — 14 probe bị chặn, corpus 855/855, gt-borrowck xanh
```
</details>
