; ĐỐI CHỨNG HẸP (GÓI 5 S1.b), qua gritrt_spawn: hai worker lặp 200000 vòng; CHỈ ở vòng cuối mới ghi ô toàn cục không đồng bộ. TSan PHẢI báo "data race" (cửa sổ hẹp: một va chạm duy nhất).
@g = global i64 0, align 8
declare void @gritrt_spawn(ptr, ptr)
declare i32 @usleep(i32)
define void @worker(ptr %a) {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %n, %cont ]
  %n = add i64 %i, 1
  %last = icmp eq i64 %i, 199999
  br i1 %last, label %w, label %cont
w:
  store i64 1, ptr @g, align 8
  br label %cont
cont:
  %done = icmp eq i64 %n, 200000
  br i1 %done, label %exit, label %loop
exit:
  ret void
}
define i32 @main() {
  call void @gritrt_spawn(ptr @worker, ptr null)
  call void @gritrt_spawn(ptr @worker, ptr null)
  %u = call i32 @usleep(i32 300000)
  ret i32 0
}
