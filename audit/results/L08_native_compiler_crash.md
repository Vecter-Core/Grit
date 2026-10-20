### L08_native_compiler_crash — [bản sao 2] đầu vào được M4–M9 cho qua nhưng làm CHÍNH gritc sập (kiểu kích thước vô hạn…)

- **Trạng thái:** FAIL (mã thoát 1, 0s, bắt đầu 2026-10-20T11:49:57Z)
- mã thoát hợp lệ của gritc là 0/1/2/3 (D17); 134/139 = tiến trình gritc tự sập (SIGABRT/SIGSEGV)

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
/home/user/Grit/audit/run_all.sh: line 397: 19095 Aborted                 timeout 120 "$g" run "$f" > "$WORK/crash.out" 2>&1
crash_recursive_type.gt: exit=134 :: thread 'main' (19101) has overflowed its stack fatal runtime error: stack overflow, aborting 
SUMMARY: mã thoát hợp lệ của gritc là 0/1/2/3 (D17); 134/139 = tiến trình gritc tự sập (SIGABRT/SIGSEGV)
```
</details>
