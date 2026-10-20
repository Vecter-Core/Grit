### I02_fuzz_lex — [bản sao] cargo-fuzz lexer 300s (ASan, libFuzzer)

- **Trạng thái:** PASS (mã thoát 0, 317s, bắt đầu 2026-10-20T11:51:06Z)
- fuzz lex: 300s, thoát 0, 0 tệp artifact (crash/timeout/oom) — chép vào audit/results/fuzz_artifacts/lex nếu có

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
"\000\000\000\001" # Uses: 50
".+" # Uses: 57
"\000\021\000\000" # Uses: 52
"\022\000\000\000\000\000\000\000" # Uses: 30
"\007\000\000\000\000\000\000\000" # Uses: 31
"X\000\000\000" # Uses: 21
"e\000\000\000" # Uses: 26
"\000\000\000<" # Uses: 23
"\005\375" # Uses: 24
"ze\314\256" # Uses: 15
"\000&" # Uses: 10
"\001w" # Uses: 7
"\000\000\000\000\000\000\000f" # Uses: 2
"\377\377\377\377\377\377\377\012" # Uses: 4
"pu3" # Uses: 0
###### End of recommended dictionary. ######
Done 2667202 runs in 301 second(s)
SUMMARY: fuzz lex: 300s, thoát 0, 0 tệp artifact (crash/timeout/oom) — chép vào audit/results/fuzz_artifacts/lex nếu có
```
</details>
