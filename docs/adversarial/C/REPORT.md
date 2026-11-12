# Nhóm C (số học và kiểm tra) — báo cáo đọc đối kháng

Công cụ: `/tmp/gritbin/gritc`; mọi chương trình tái hiện nằm trong `/tmp/eprompt/C/` (khung chạy 6 cấu hình: `run.sh`; so `--checks=all` với `proven`: `cmp.sh`). Không sửa mã, không cargo, không git.

## Danh sách THẬT (theo mức nghiêm trọng)

### C-1 (CAO) Kiểm biên lát `a[i..j]` / gán `a[i] ← v` dùng độ dài CŨ của biến lát bị gán lại trước khi truy cập (TOCTOU) ⇒ đọc/ghi ngoài biên, `--checks=all` mặc định
* Nguyên nhân (1): `crates/gt-mir/src/build/expr.rs:484` gán `len ≔ Len(p)` TRƯỚC khi đánh giá biểu thức cận cuối `end` (dòng 487 `self.as_operand(e)`); `Assert SliceRange` (dòng 547) so với `len` cũ, nhưng `PlaceElem::Subslice` (dòng 553) lại được hạ ở `lower.rs:1203-1230` bằng cách đọc LẠI `fat` của biến lát (`Deref` ở `lower.rs:1129-1141`) ⇒ lát mới dài `to - from` trên con trỏ của lát đã bị gán lại.
* Nguyên nhân (2): `expr.rs:332-336` (`StmtKind::Assign`): `as_place(target)` (kèm `Assert IndexOutOfBounds`) rồi mới `as_rvalue(value)`; nếu vế phải gán lại biến lát thì lệnh ghi qua `PlaceElem::Index` (`lower.rs:1172-1190`) đọc lát MỚI nhưng đã kiểm theo độ dài CŨ. DECISIONS.md:338 ghi "chỗ trước, giá trị sau" là thiết kế, nhưng nó chỉ chốt chỉ số `i`, không chốt con trỏ béo của lát.
* Hậu quả đo được: lát `t` dài 2000 trên mảng 2 byte; `t[i]` qua kiểm `i < t.len()` đọc 2000 byte ngoài mảng; ghi 60 byte vào mảng 2 byte. Kết quả khác nhau giữa dev/release-o2 (rác ngăn xếp) và thông dịch báo ICE (`MIR không hợp lệ: lát ngoài biên`, rc=2).
* Tái hiện: `/tmp/eprompt/C/ord1.gt`, `ord2_oob_read.gt` (llvm dev=2000000013989, release-o2=2000000013949, interpreter rc=2), `ord3_oob_write.gt`, `ord4_assign.gt` (ghi `s[50]` sau khi `s` trỏ mảng 2 phần tử; llvm rc=0, interpreter rc=2). Cùng hành vi với `--checks=proven`.
* Điều kiện: cần biểu thức cận/vế phải có tác dụng phụ gán lại CHÍNH biến lát (hiếm) — nhưng sai an toàn bộ nhớ nằm đúng chỗ TCB hứa. Cách sửa gợi ý: chụp `Len` + con trỏ lát vào temp trước khi đánh giá `end`/vế phải và dùng temp đó cho `Subslice`/`Index`.
* Không thấy bug tương tự ở: `a[ {s←…; 50} ]` (len đọc sau chỉ số; ord7 abort đúng), `x + {s←…}` và `f(s[50], {s←…})` (ord5/ord6: toán hạng được ổn định, ra 8 trên cả hai backend).

### C-2 (THẤP/TRUNG) Thông dịch sai với `u128 ≥ 2^127` (`--backend=interpreter`), LLVM đúng
* `interp.rs:41-65` biểu diễn mọi số nguyên bằng `i128`; `wrap` và `in_range` trả nguyên cho 128 bit ⇒ `u128` ≥ 2^127 thành số âm.
* Tái hiện: `a2_u128.gt` (`1u128 < 2^127+1`: interpreter=0, llvm=1 đúng); `b5_u128cast.gt` (`(2^127+1) as i128`: interpreter KHÔNG abort, llvm abort `cast_lossy` đúng).
* Ngoài TCB, nhưng oracle M7 và mọi so sánh "interp ≡ exe" mù với 128 bit.

### C-3 (THẤP, fail-closed) Hạ LLVM lỗi/không hỗ trợ ở 128 bit và `*^`
* `i128/u128 as f64` (hạ `cast_fits` giả định i64: `lower.rs:1865-1887`): IR sai kiểu (`add i128 %ld, i64 …`), IR Guard bắt ⇒ `error[E1101]` thay vì mã chương trình (`a4_i128f64.gt`; interpreter abort `cast_lossy`).
* `*^` (`MulSat`, `lower.rs:1731`) và nhân/chia/`%` 128 bit (`lower.rs:1622-1666`, `1766`): `backend llvm chưa hỗ trợ` (rc=3) trong khi interpreter chạy (`a5_u128div.gt`; ma trận `*^` mọi kiểu). Không phải UB; backend chưa phủ.

## Bảng phép | kết luận | bằng chứng | tái hiện

| phép | kết luận | bằng chứng tệp:dòng | chương trình tái hiện |
|---|---|---|---|
| `+ - *` kiểm tràn (i8..i64, u8..u64, isize, usize) | LOẠI (đúng): 3 hồ sơ + proven khớp mô hình Python trên ~14 giá trị biên/kiểu | `lower.rs:1756-1783` (`llvm.{s,u}{add,sub,mul}.with.overflow`), `expr.rs:1022-1045` | `run_mx.py` (mx_out.txt: 176 OK) |
| `+ - *` 128 bit | add/sub đúng (a1,a3,b1-b4); `*` không hỗ trợ (C-3) | `lower.rs:1766` | `a1_i128.gt`,`b4_i128min.gt` |
| `/ %` kể cả chia 0 và `MIN / -1`, `MIN % -1` | LOẠI (đúng); lớp phòng thủ thứ hai ở codegen | `lower.rs:1664-1700`, `expr.rs:1059-1112` | `run_mx.py`, `t3.gt` |
| `-x` (OverflowNeg) | LOẠI (đúng, abort `overflow_neg`) | `expr.rs:955-970`, `lower.rs:1530` | `run_mx.py` neg |
| `+% -% *%`, `+^ -^`, `+? -? *?` | LOẠI (đúng); `*^` không hỗ trợ ở LLVM (C-3) | `lower.rs:1651-1655,1707-1716` | `run_mx.py` |
| so sánh nguyên (6 phép) | LOẠI | `lower.rs:1717-1733` | `run_mx.py` |
| `as` int→int (100 cặp, giá trị biên) | LOẠI (đúng): `CastFits` đúng ở dev/release-o2/proven | `lower.rs:1852-1917`, `expr.rs:76-93` | `run_cast.py` (cast_out1/2.txt) |
| `as` u128/i128 | LLVM đúng; interpreter sai (C-2); `as f64` 128 bit (C-3) | `interp.rs:1109-1135` | `a2`, `b5`, `a4`, `a6` |
| `as f64` / `to_i64` / NaN / `¬(x<y)` | LOẠI: cận ±2^53 khớp interpreter/LLVM; NaN khớp | `lower.rs:1809-1829,1864-1887`; `analysis.rs:833-843` | `f_*.gt` (9 giá trị), `g1.gt`, `g2.gt` (474 ở cả 6 cấu hình) |
| chỉ số `a[i]` (mảng/lát) | LOẠI | `expr.rs:555-577` | `u03,u04,u06..u13` |
| lát `a[i..j]`, `..=`, lát-của-lát (cận 0..2^64-1) | ma trận 5 dạng × 81 cặp × 3 cấu hình khớp mô hình, NHƯNG xem C-1 | `expr.rs:463-553` | `sl.py` (0 lệch) + `ord1..ord4` |
| `Assert` Fuel (`core::fuel_left`) | LOẠI (đỏ E0901 khi chắc chắn cạn; abort `fuel_exhausted` ở runtime) | `lower.rs:1999` | `p/u01`, `p/u02` |
| `shl shr rotr rotl` | LOẠI: n=0,1,31,32,2^32-1 khớp ở 6 cấu hình; n≥32 ⇒ abort `precondition` (cả proven) | `lower.rs:2920-2974` | `s_{shl,shr,rotr,rotl}_*.gt` |
| `--checks=all` bỏ qua Assert? | Không có đường nào thấy (driver:303-312 chỉ xoá khi `Proven`); `cli.rs:121` cấm proven + release | `driver.rs:303`, `pipeline.rs:86` | — |
| `--checks=proven` xoá nhầm | KHÔNG tìm được âm tính giả: 23 chương trình tấn công (gán lại, wrap, bí danh, `&!`, trường/ô mảng, hậu điều kiện, vòng/continue, quan hệ n-m, ép dấu, `∨/∧`) cho kết quả y hệt `all` | `analysis.rs:743-875`, `interval.rs:434-470` | `p/t*.gt`, `p/u*.gt`, `cmp.sh` |
| thông dịch ≡ exe (số học) | LOẠI ngoài 128 bit: 7 767 ca (i8,u8,i32,u32,i64,u64,usize × 21 phép × biên) khớp mô hình | `interp.rs:961-1135` | `run_interp.py` (interp_out.txt: bad 0) |
| literal ngoài phạm vi | LOẠI (E0601 đúng ở biên i8/u8/u64/i128) | `expr.rs:~395` | `lit.gt` (vòng lặp bash) |
| id nguyên tử trùng vùng (`UNREP_BASE + bb*1024+idx` đè `PRED_BASE`) | NGHI: số học id cho phép trùng khi khối ≥ 1024; thử 33 kích thước hàm không tái hiện được (không chứng minh) | `analysis.rs:36-44,1044-1046` | `u/gen_u.py` |

## NGHI
* `analysis.rs:1044`: `unrep_atom = UNREP_BASE + bb*1024 + idx` với `UNREP_BASE = 6<<20`, `PRED_BASE = 7<<20`: khi `bb ≥ 1024`, nguyên tử "nghĩa vụ không biểu diễn được" trùng nguyên tử vị từ thuần `PRED_BASE + p*1024 + local`; nếu vị từ đó đang đúng trong môi trường thì nghĩa vụ `Pre` không biểu diễn được có thể bị coi là đã chứng minh (chỉ ảnh hưởng `--checks=proven`). Tương tự `site_atom = SITE_BASE + bb*contract_atoms + …` có thể tràn vào vùng kế (≥ 2^20). Quét K=1008..1040 phép cộng đệm trước điểm gọi (hợp đồng thứ hai là gọi hàm hai đối số) vẫn abort `precondition` đúng ⇒ chưa có PoC; cần dựng đúng dạng hợp đồng `prop=None` để chắc.
* Hàm nguyên ≥ 2^20 local hoặc nhiều hơn 1024 local trong `pred_key` đã có chặn `key < 1024` (OK).

## Phần KHÔNG kịp đọc / chưa đào
* `gt-oblig/src/env.rs`, `prover.rs`, `zone.rs`, phần lớn `analysis.rs` (fixpoint/widening/narrowing `fixpoint`, `narrow`, `walk_paths`, điều kiện `cond_branch`): chỉ đọc `assign_local`, hyps, `extract_contract` và đầu tệp; đánh giá "không âm tính giả" của proven chỉ dựa trên 23 chương trình đối kháng, không phải đọc đầy đủ.
* `gt-guard/src/lib.rs`: chỉ đọc danh sách cờ cấm (nsw/nuw/exact/fast-math…); không kiểm tra Guard có đòi `Assert` biên còn nguyên.
* `runtime/gt-rt` (số học arena/Vec, `text`, `str.slice`, HashMap), `Vec::push` overflow, `Text`: không chạy; `bit_op` và `vec_grow_to` chỉ đọc.
* i128/u128 chỉ thử vài phép (add/sub/compare/cast); không có ma trận đầy đủ vì nhân/chia/`%` 128 bit không hỗ trợ.
* Không thử đa luồng, `Cell`, atomics (nhóm khác); không thử chương trình lớn > 1 000 khối cho các lỗi vùng id.
* Ma trận số học dùng giá trị biên cố định (~14/kiểu), không ngẫu nhiên.
