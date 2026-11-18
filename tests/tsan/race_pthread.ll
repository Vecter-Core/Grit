; ĐỐI CHỨNG DƯƠNG (TSan, GÓI 4 S3.a): hai luồng pthread cùng ghi MỘT ô toàn cục không đồng bộ ⇒ TSan PHẢI báo "data race".
; Grit không cho viết đua dữ liệu nên đối chứng nằm ở mức IR. Không dùng gt-rt.
@g = global i64 0, align 8
declare i32 @pthread_create(ptr, ptr, ptr, ptr)
declare i32 @pthread_join(i64, ptr)
define ptr @worker(ptr %a) {
  store i64 1, ptr @g, align 8
  ret ptr null
}
define i32 @main() {
  %t1 = alloca i64, align 8
  %t2 = alloca i64, align 8
  %r1 = call i32 @pthread_create(ptr %t1, ptr null, ptr @worker, ptr null)
  %r2 = call i32 @pthread_create(ptr %t2, ptr null, ptr @worker, ptr null)
  %h1 = load i64, ptr %t1, align 8
  %j1 = call i32 @pthread_join(i64 %h1, ptr null)
  %h2 = load i64, ptr %t2, align 8
  %j2 = call i32 @pthread_join(i64 %h2, ptr null)
  ret i32 0
}
