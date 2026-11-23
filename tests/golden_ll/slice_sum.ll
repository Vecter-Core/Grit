; ModuleID = 'grit'
source_filename = "grit"
target datalayout = "e-m:w-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-windows-msvc"

@gt.attestation = constant [65 x i8] c"2c430b69636d943c3edbb3715563ded84f57743b666cb5fddf78650d24a9a543\00"

; Function Attrs: noreturn
declare void @gritrt_abort(i32) #0

define i64 @prog__main.06e433423c68530e() {
alloca:
  %l0 = alloca i64, align 8
  %l1 = alloca [3 x i64], align 8
  %l2 = alloca { ptr, i64 }, align 8
  %l3 = alloca i64, align 8
  %l4 = alloca i64, align 8
  %l5 = alloca i1, align 1
  %l6 = alloca i1, align 1
  %l7 = alloca i1, align 1
  br label %bb0

bb0:                                              ; preds = %alloca
  %gep = getelementptr inbounds [3 x i64], ptr %l1, i64 0, i64 0
  store i64 1, ptr %gep, align 8
  %gep1 = getelementptr inbounds [3 x i64], ptr %l1, i64 0, i64 1
  store i64 2, ptr %gep1, align 8
  %gep2 = getelementptr inbounds [3 x i64], ptr %l1, i64 0, i64 2
  store i64 3, ptr %gep2, align 8
  store i64 0, ptr %l4, align 8
  store i64 3, ptr %l3, align 8
  %ld = load i64, ptr %l4, align 8
  %ld3 = load i64, ptr %l3, align 8
  %cmp = icmp ule i64 %ld, %ld3
  store i1 %cmp, ptr %l5, align 1
  %ld4 = load i64, ptr %l3, align 8
  %ld5 = load i64, ptr %l3, align 8
  %cmp6 = icmp ule i64 %ld4, %ld5
  store i1 %cmp6, ptr %l6, align 1
  %ld7 = load i1, ptr %l5, align 1
  %ld8 = load i1, ptr %l6, align 1
  %and = and i1 %ld7, %ld8
  store i1 %and, ptr %l7, align 1
  %ld9 = load i1, ptr %l7, align 1
  br i1 %ld9, label %bb1, label %trap.0

bb1:                                              ; preds = %bb0
  %from = load i64, ptr %l4, align 8
  %gep10 = getelementptr inbounds i64, ptr %l1, i64 %from
  %sublen = sub i64 3, %from
  %f0 = insertvalue { ptr, i64 } undef, ptr %gep10, 0
  %f1 = insertvalue { ptr, i64 } %f0, i64 %sublen, 1
  store { ptr, i64 } %f1, ptr %l2, align 8
  %ld11 = load { ptr, i64 }, ptr %l2, align 8
  %call = call i64 @prog__sum.f4efe6fc43b0edbe({ ptr, i64 } %ld11)
  store i64 %call, ptr %l0, align 8
  br label %bb2

bb2:                                              ; preds = %bb1
  %ret = load i64, ptr %l0, align 8
  ret i64 %ret

trap.0:                                           ; preds = %bb0
  call void @gritrt_abort(i32 13) #0
  unreachable
}

define i64 @prog__sum.f4efe6fc43b0edbe({ ptr, i64 } %0) {
alloca:
  %l0 = alloca i64, align 8
  %l1 = alloca { ptr, i64 }, align 8
  %l2 = alloca i64, align 8
  %l3 = alloca i64, align 8
  %l4 = alloca {}, align 8
  %l5 = alloca {}, align 8
  %l6 = alloca i1, align 1
  %l7 = alloca i1, align 1
  %l8 = alloca i64, align 8
  %l9 = alloca i64, align 8
  %l10 = alloca { ptr, i64 }, align 8
  %l11 = alloca i64, align 8
  %l12 = alloca i64, align 8
  %l13 = alloca i1, align 1
  %l14 = alloca { i64, i1 }, align 8
  %l15 = alloca { i64, i1 }, align 8
  store { ptr, i64 } %0, ptr %l1, align 8
  br label %bb0

bb0:                                              ; preds = %alloca
  store i64 0, ptr %l2, align 8
  store i64 0, ptr %l3, align 8
  br label %bb1

bb1:                                              ; preds = %bb7, %bb0
  %ld = load i64, ptr %l3, align 8
  store i64 %ld, ptr %l8, align 8
  %fat = load { ptr, i64 }, ptr %l1, align 8
  %p = extractvalue { ptr, i64 } %fat, 0
  %n = extractvalue { ptr, i64 } %fat, 1
  %f0 = insertvalue { ptr, i64 } undef, ptr %p, 0
  %f1 = insertvalue { ptr, i64 } %f0, i64 %n, 1
  store { ptr, i64 } %f1, ptr %l10, align 8
  %ld1 = load { ptr, i64 }, ptr %l10, align 8
  %len = extractvalue { ptr, i64 } %ld1, 1
  store i64 %len, ptr %l9, align 8
  br label %bb4

bb2:                                              ; preds = %bb4
  %ld2 = load i64, ptr %l3, align 8
  store i64 %ld2, ptr %l12, align 8
  %fat3 = load { ptr, i64 }, ptr %l1, align 8
  %p4 = extractvalue { ptr, i64 } %fat3, 0
  %n5 = extractvalue { ptr, i64 } %fat3, 1
  store i64 %n5, ptr %l11, align 8
  %ld6 = load i64, ptr %l12, align 8
  %ld7 = load i64, ptr %l11, align 8
  %cmp = icmp ult i64 %ld6, %ld7
  store i1 %cmp, ptr %l13, align 1
  %ld8 = load i1, ptr %l13, align 1
  br i1 %ld8, label %bb5, label %trap.0

bb3:                                              ; preds = %bb4
  store {} zeroinitializer, ptr %l4, align 1
  %ld9 = load i64, ptr %l2, align 8
  store i64 %ld9, ptr %l0, align 8
  %ret = load i64, ptr %l0, align 8
  ret i64 %ret

bb4:                                              ; preds = %bb1
  %ld10 = load i64, ptr %l8, align 8
  %ld11 = load i64, ptr %l9, align 8
  %cmp12 = icmp ult i64 %ld10, %ld11
  store i1 %cmp12, ptr %l7, align 1
  %ld13 = load i1, ptr %l7, align 1
  store i1 %ld13, ptr %l6, align 1
  %ld14 = load i1, ptr %l6, align 1
  switch i1 %ld14, label %bb2 [
    i1 false, label %bb3
  ]

bb5:                                              ; preds = %bb2
  %ld15 = load i64, ptr %l2, align 8
  %fat16 = load { ptr, i64 }, ptr %l1, align 8
  %p17 = extractvalue { ptr, i64 } %fat16, 0
  %n18 = extractvalue { ptr, i64 } %fat16, 1
  %idx = load i64, ptr %l12, align 8
  %gep = getelementptr inbounds i64, ptr %p17, i64 %idx
  %ld19 = load i64, ptr %gep, align 8
  %i = call { i64, i1 } @llvm.sadd.with.overflow.i64(i64 %ld15, i64 %ld19)
  store { i64, i1 } %i, ptr %l14, align 8
  %fld = getelementptr inbounds nuw { i64, i1 }, ptr %l14, i32 0, i32 1
  %ld20 = load i1, ptr %fld, align 1
  br i1 %ld20, label %trap.1, label %bb6

bb6:                                              ; preds = %bb5
  %fld21 = getelementptr inbounds nuw { i64, i1 }, ptr %l14, i32 0, i32 0
  %ld22 = load i64, ptr %fld21, align 8
  store i64 %ld22, ptr %l2, align 8
  %ld23 = load i64, ptr %l3, align 8
  %i24 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %ld23, i64 1)
  store { i64, i1 } %i24, ptr %l15, align 8
  %fld25 = getelementptr inbounds nuw { i64, i1 }, ptr %l15, i32 0, i32 1
  %ld26 = load i1, ptr %fld25, align 1
  br i1 %ld26, label %trap.2, label %bb7

bb7:                                              ; preds = %bb6
  %fld27 = getelementptr inbounds nuw { i64, i1 }, ptr %l15, i32 0, i32 0
  %ld28 = load i64, ptr %fld27, align 8
  store i64 %ld28, ptr %l3, align 8
  store {} zeroinitializer, ptr %l5, align 1
  br label %bb1

trap.0:                                           ; preds = %bb2
  call void @gritrt_abort(i32 2) #0
  unreachable

trap.1:                                           ; preds = %bb5
  call void @gritrt_abort(i32 1) #0
  unreachable

trap.2:                                           ; preds = %bb6
  call void @gritrt_abort(i32 1) #0
  unreachable
}

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare { i64, i1 } @llvm.sadd.with.overflow.i64(i64, i64) #1

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare { i64, i1 } @llvm.uadd.with.overflow.i64(i64, i64) #1

define void @gt_entry() {
entry:
  call void @gritrt_dev_checks()
  %main = call i64 @prog__main.06e433423c68530e()
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

!0 = !{!"2c430b69636d943c3edbb3715563ded84f57743b666cb5fddf78650d24a9a543"}
!1 = !{!"prog__main.06e433423c68530e=prog::main", !"prog__sum.f4efe6fc43b0edbe=prog::sum"}
