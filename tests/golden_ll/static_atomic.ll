; ModuleID = 'grit'
source_filename = "grit"
target datalayout = "e-m:w-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-windows-msvc"

@"static.prog::COUNT" = global { i32 } zeroinitializer
@gt.attestation = constant [65 x i8] c"21a0fef0fb522f8fedd2cb55e9fcd5cd95932e869089ebbcc0e62793f6ec0bc6\00"

; Function Attrs: noreturn
declare void @gritrt_abort(i32) #0

define i32 @prog__main.06e433423c68530e() {
alloca:
  %l0 = alloca i32, align 4
  %l1 = alloca {}, align 8
  %l3 = alloca { i64, [0 x i64] }, align 8
  %l4 = alloca ptr, align 8
  br label %bb0

bb0:                                              ; preds = %alloca
  %call = call {} @prog__worker.c0e26e01e9729395(i32 1)
  store {} %call, ptr %l1, align 1
  br label %bb1

bb1:                                              ; preds = %bb0
  %tag = getelementptr inbounds nuw { i64, [0 x i64] }, ptr %l3, i32 0, i32 0
  store i64 4, ptr %tag, align 8
  %pay = getelementptr inbounds nuw { i64, [0 x i64] }, ptr %l3, i32 0, i32 1
  store ptr @"static.prog::COUNT", ptr %l4, align 8
  %ld = load ptr, ptr %l4, align 8
  %ld1 = load { i64, [0 x i64] }, ptr %l3, align 8
  %aload = load atomic i32, ptr %ld seq_cst, align 4, !gt.obl !2
  store i32 %aload, ptr %l0, align 4
  br label %bb2

bb2:                                              ; preds = %bb1
  %ret = load i32, ptr %l0, align 4
  ret i32 %ret
}

define {} @prog__worker.c0e26e01e9729395(i32 %0) {
alloca:
  %l0 = alloca {}, align 8
  %l1 = alloca i32, align 4
  %l2 = alloca i32, align 4
  %l4 = alloca { i64, [0 x i64] }, align 8
  %l5 = alloca ptr, align 8
  store i32 %0, ptr %l1, align 4
  br label %bb0

bb0:                                              ; preds = %alloca
  %tag = getelementptr inbounds nuw { i64, [0 x i64] }, ptr %l4, i32 0, i32 0
  store i64 0, ptr %tag, align 8
  %pay = getelementptr inbounds nuw { i64, [0 x i64] }, ptr %l4, i32 0, i32 1
  store ptr @"static.prog::COUNT", ptr %l5, align 8
  %ld = load ptr, ptr %l5, align 8
  %ld1 = load i32, ptr %l1, align 4
  %ld2 = load { i64, [0 x i64] }, ptr %l4, align 8
  %1 = atomicrmw add ptr %ld, i32 %ld1 monotonic, align 4, !gt.obl !2
  store i32 %1, ptr %l2, align 4
  br label %bb1

bb1:                                              ; preds = %bb0
  store {} zeroinitializer, ptr %l0, align 1
  %ret = load {}, ptr %l0, align 1
  ret {} %ret
}

define { i32 } @prog__COUNT.64a39aefa184457a() {
alloca:
  %l0 = alloca { i32 }, align 8
  br label %bb0

bb0:                                              ; preds = %alloca
  store { i32 } zeroinitializer, ptr %l0, align 4
  br label %bb1

bb1:                                              ; preds = %bb0
  %ret = load { i32 }, ptr %l0, align 4
  ret { i32 } %ret
}

define void @gt_entry() {
entry:
  call void @gritrt_dev_checks()
  %sinit = call { i32 } @prog__COUNT.64a39aefa184457a()
  store { i32 } %sinit, ptr @"static.prog::COUNT", align 4
  %main = call i32 @prog__main.06e433423c68530e()
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

!gt.attest = !{!0}
!gt.instances = !{!1}

!0 = !{!"21a0fef0fb522f8fedd2cb55e9fcd5cd95932e869089ebbcc0e62793f6ec0bc6"}
!1 = !{!"prog__COUNT.64a39aefa184457a=prog::COUNT", !"prog__main.06e433423c68530e=prog::main", !"prog__worker.c0e26e01e9729395=prog::worker"}
!2 = !{!"E0503"}
