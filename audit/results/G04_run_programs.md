### G04_run_programs — [bản sao] gritc run: mẫu, bench, 6 công cụ examples/

- **Trạng thái:** PASS (mã thoát 0, 670s, bắt đầu 2026-10-20T11:33:31Z)
- mẫu/bench chạy bằng thông dịch: 8 chương trình; exit0=2 abort70=0 hết giới hạn71=2 khác=4

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
== tests/samples + tests/bench (chạy bằng thông dịch MIR)
arrays_slices.gt: exit=0 kỳ vọng=79 nhận=Int(79, I64) green
numeric.gt: exit=0 kỳ vọng=11630 nhận=Int(11630, U64) green
threads.gt: exit=2 kỳ vọng=8000 nhận=None green
two_locks.gt: exit=2 kỳ vọng=4000 nhận=None green
fib.gt: exit=71 kỳ vọng=? nhận=None green
loop_sum.gt: exit=71 kỳ vọng=? nhận=None green
slice_sum.gt: exit=124 kỳ vọng=? nhận=NOJSON
threads.gt: exit=2 kỳ vọng=? nhận=None green
== 6 công cụ dogfood (examples/*/src) — cần Cap<Fs>/args/IO của runtime native
examples/json: exit=2 :: gritc: lỗi nội bộ khi chạy: bộ chạy phát hiện MIR không hợp lệ: không có thân MIR cho $prelude::root_env
examples/sha256: exit=2 :: gritc: lỗi nội bộ khi chạy: bộ chạy phát hiện MIR không hợp lệ: không có thân MIR cho $prelude::root_env
examples/sortlines: exit=1 :: gritc: phán quyết red; 1 lỗi, 0 cảnh báo, 0 ghi chú; backend: interpreter
examples/stats: exit=2 :: gritc: lỗi nội bộ khi chạy: bộ chạy phát hiện MIR không hợp lệ: không có thân MIR cho $prelude::root_env
examples/wc: exit=2 :: gritc: lỗi nội bộ khi chạy: bộ chạy phát hiện MIR không hợp lệ: không có thân MIR cho $prelude::root_env
examples/wordfreq: exit=2 :: gritc: lỗi nội bộ khi chạy: bộ chạy phát hiện MIR không hợp lệ: không có thân MIR cho $prelude::root_env
SUMMARY: mẫu/bench chạy bằng thông dịch: 8 chương trình; exit0=2 abort70=0 hết giới hạn71=2 khác=4
```
</details>
