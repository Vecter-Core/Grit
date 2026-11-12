; ModuleID = 'grit'
source_filename = "grit"
target datalayout = "e-m:w-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-windows-msvc"

@gt.attestation = constant [65 x i8] c"589654e810e65a11f63867ff8dc9f14dc8b809ce1d97e280a9d8cc36c7dc22cf\00"

; Function Attrs: noreturn
declare void @gritrt_abort(i32) #0

define i8 @prog__main.06e433423c68530e() {
alloca:
  %l0 = alloca i8, align 1
  br label %bb0

bb0:                                              ; preds = %alloca
  %call = call i8 @prog__add.f016c5565df880da(i8 -56, i8 100)
  store i8 %call, ptr %l0, align 1
  br label %bb1

bb1:                                              ; preds = %bb0
  %ret = load i8, ptr %l0, align 1
  ret i8 %ret
}

define i8 @prog__add.f016c5565df880da(i8 %0, i8 %1) {
alloca:
  %l0 = alloca i8, align 1
  %l1 = alloca i8, align 1
  %l2 = alloca i8, align 1
  %l3 = alloca { i8, i1 }, align 8
  store i8 %0, ptr %l1, align 1
  store i8 %1, ptr %l2, align 1
  br label %bb0

bb0:                                              ; preds = %alloca
  %ld = load i8, ptr %l1, align 1
  %ld1 = load i8, ptr %l2, align 1
  %i = call { i8, i1 } @llvm.uadd.with.overflow.i8(i8 %ld, i8 %ld1)
  store { i8, i1 } %i, ptr %l3, align 1
  %fld = getelementptr inbounds nuw { i8, i1 }, ptr %l3, i32 0, i32 1
  %ld2 = load i1, ptr %fld, align 1
  br i1 %ld2, label %trap.0, label %bb1

bb1:                                              ; preds = %bb0
  %fld3 = getelementptr inbounds nuw { i8, i1 }, ptr %l3, i32 0, i32 0
  %ld4 = load i8, ptr %fld3, align 1
  store i8 %ld4, ptr %l0, align 1
  %ret = load i8, ptr %l0, align 1
  ret i8 %ret

trap.0:                                           ; preds = %bb0
  call void @gritrt_abort(i32 1) #0
  unreachable
}

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare { i8, i1 } @llvm.uadd.with.overflow.i8(i8, i8) #1

define void @gt_entry() {
entry:
  call void @gritrt_dev_checks()
  %main = call i8 @prog__main.06e433423c68530e()
  ret void
}

declare void @gritrt_dev_checks()

declare i32 @gritrt_start(ptr)

define i32 @main() {
entry:
  %r = call i32 @gritrt_start(ptr @gt_entry)
  ret i32 %r
}

attributes #0 = { noreturn }
attributes #1 = { nocallback nofree nosync nounwind speculatable willreturn memory(none) }

!gt.attest = !{!0}
!gt.instances = !{!1}

!0 = !{!"589654e810e65a11f63867ff8dc9f14dc8b809ce1d97e280a9d8cc36c7dc22cf"}
!1 = !{!"prog__add.f016c5565df880da=prog::add", !"prog__main.06e433423c68530e=prog::main"}
