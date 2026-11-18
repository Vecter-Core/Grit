; ĐỐI CHỨNG DƯƠNG qua đường `gritrt_spawn` của gt-rt (GÓI 4 S3.a): hai worker do `gritrt_spawn` tạo cùng ghi một ô toàn cục không đồng bộ ⇒ TSan PHẢI báo "data race"
; (chứng minh luồng do runtime tạo cũng được theo dõi). main chờ bằng usleep (không tạo cạnh happens-before giữa hai worker).
@g = global i64 0, align 8
declare void @gritrt_spawn(ptr, ptr)
declare i32 @usleep(i32)
define void @worker(ptr %a) {
  store i64 1, ptr @g, align 8
  ret void
}
define i32 @main() {
  call void @gritrt_spawn(ptr @worker, ptr null)
  call void @gritrt_spawn(ptr @worker, ptr null)
  %u = call i32 @usleep(i32 300000)
  ret i32 0
}
