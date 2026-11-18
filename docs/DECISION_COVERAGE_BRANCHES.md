# DECISION_COVERAGE_BRANCHES — bảng TỪNG nhánh chưa chạy (GÓI 6 S1.a)

Sinh bằng `python tools/coverage_table.py cov.json.gz`. Tổng nhánh = 734; chưa chạy = 90. ĐẾM THẬT từ bảng (không ước lượng theo tên hàm):

| nhóm | số nhánh |
|---|---:|
| A | 4 |
| B | 6 |
| C | 57 |
| D | 23 |

| nguồn | số nhánh |
|---|---:|
| ĐỌC MÃ | 57 |
| CHƯA | 14 |
| TN-KHÔNG-CHẠM | 10 |
| TN | 6 |
| TN-NGOÀI-CORPUS | 2 |
| ĐỌC MÃ+lexer | 1 |

Nhóm: **A** hook chỉ-kiểm-thử/hằng cấu hình · **B** giới hạn tài nguyên · **C** nhánh phòng thủ (ICE/let-else) cho trạng thái do giai đoạn trước đảm bảo · **D** đạt tới được từ chương trình người dùng.
Nguồn: **TN** = có chương trình thật; **TN-KHÔNG-CHẠM** = đã thử, chương trình chạy nhưng KHÔNG chạm nhánh; **TN-NGOÀI-CORPUS** = chạy trực tiếp nhưng quá chậm/hai mã nên không vào corpus; **ĐỌC MÃ** = chỉ đọc mã (không có thí nghiệm); **CHƯA** = chưa làm.

| tệp:dòng:cột | hàm | hướng | điều kiện | nhóm | nguồn | thí nghiệm / lý do |
|---|---|---|---|---|---|---|
| gt-oblig/src/analysis.rs:978:20 | hyps_for | false | `if let B::Fin(lo) = iv.lo` | D | TN-KHÔNG-CHẠM | cần biến có cận dưới vô hạn qua `hyps_for`; thử `tests/pass/loop_downward_i64_cov.gt` (xanh) nhưng KHÔNG chạm; hướng sai chỉ BỚT giả thiết ⇒ ít chứng minh hơn ⇒ giữ check (lập luận) |
| gt-oblig/src/analysis.rs:983:20 | hyps_for | false | `if let B::Fin(hi) = iv.hi` | D | TN-KHÔNG-CHẠM | như dòng 978 với cận trên vô hạn |
| gt-oblig/src/analysis.rs:907:34 | assign_local | false | `Val::Interval(iv) if tracked => st.env.assign_interval(dst as VarId, *iv),` | D | TN-KHÔNG-CHẠM | biến không theo dõi nhận khoảng; thử `pass/many_locals_beyond_zone_cov.gt` (40 biến, xanh + ghi chú) KHÔNG chạm |
| gt-oblig/src/analysis.rs:2010:8 | fixpoint | true | `if n > UNREP_MAX_BLOCKS {` | B | TN-NGOÀI-CORPUS | `UNREP_MAX_BLOCKS`: hàm 16 000 `⎇` ⇒ xanh + N0605 "hết ngân sách bước" (LIMITS_AUDIT #3); quá chậm để vào corpus (37 s release, > 15 phút debug) |
| gt-oblig/src/analysis.rs:2025:16 | fixpoint | true | `if *steps > MAX_BLOCK_STEPS {` | B | ĐỌC MÃ | `MAX_BLOCK_STEPS` = 400 000 bước/khối: chưa dựng được chương trình chạm riêng giới hạn này (LIMITS_AUDIT #4) |
| gt-oblig/src/analysis.rs:2035:41 | fixpoint | false | `if back_edge && visits[t] >= 1 {` | C | ĐỌC MÃ | `back_edge && visits[t] >= 1` sai khi cạnh lùi tới khối CHƯA thăm: thứ tự duyệt hậu thứ tự luôn thăm đích cạnh lùi trước (bất biến của thuật toán) |
| gt-oblig/src/analysis.rs:292:12 | widen | true | `if self.env.is_dead() {` | B | TN-KHÔNG-CHẠM | `widen` với môi trường chết: thử `pass/dead_loop_constant_false_cov.gt`, `pass/dead_inner_loop_cov.gt` (xanh) KHÔNG chạm: vòng trong nhánh chết bị bỏ trước khi tới widen |
| gt-oblig/src/analysis.rs:295:12 | widen | true | `if new.env.is_dead() {` | B | TN-KHÔNG-CHẠM | như dòng 292 (môi trường mới chết) |
| gt-oblig/src/driver.rs:144:8 | analyze_program_with | true | `if !m.report.diagnostics.is_empty() {` | C | ĐỌC MÃ | `analyze_program_with` chỉ gọi khi M5 không có chẩn đoán (pipeline bỏ qua M7 nếu M5 đỏ); nhánh phòng thủ trả rỗng |
| gt-oblig/src/driver.rs:179:17 | analyze_program_with | false | `let Some(term) = &bb.terminator else { continue };` | C | ĐỌC MÃ | khối MIR không có terminator: MIR đã qua `validate` nên không có |
| gt-oblig/src/driver.rs:190:17 | analyze_program_with | false | `let Some(sig) =` | C | ĐỌC MÃ | không tìm thấy chữ ký của thân hàm: mọi thân hàm có chữ ký (do M4) |
| gt-oblig/src/driver.rs:201:35 | analyze_program_with | false | `.any(\|mm\| mm.name == sig.name && info.pre.contains_key(&mm.node))` | C | ĐỌC MÃ | tra thành viên impl theo tên (phòng thủ) |
| gt-oblig/src/driver.rs:203:37 | analyze_program_with | true | `if sig.has_contracts \|\| info.pre.contains_key(&sig.node) \|\| in_impls {` | D | CHƯA | thân thuộc hàm có hợp đồng/precondition hoặc thành viên impl; hướng `true` phụ thuộc cách duyệt `info.pre`; chưa thí nghiệm |
| gt-oblig/src/driver.rs:268:16 | analyze_program_with | true | `if let Some(e) = &rep.ice {` | C | ĐỌC MÃ | `rep.ice`: ICE của phân tích một thân; chỉ có khi lỗi nội bộ (chèn lỗi test `fault_dropped_obligations`); không đạt tới từ chương trình Grit đúng |
| gt-oblig/src/driver.rs:291:12 | analyze_program_with | true | `if rep.budget_exhausted {` | B | TN-NGOÀI-CORPUS | `budget_exhausted`: hàm 16 000 `⎇` in ghi chú N0605 "hết ngân sách bước phân tích `main`" (đã thấy), nhưng quá chậm để vào corpus |
| gt-oblig/src/driver.rs:352:24 | analyze_program_with | false | `if let Some(term) = ob.blocks[*blk as usize].terminator.as_mut()` | C | ĐỌC MÃ | tìm Assert terminator đích để cắt sau khi chứng minh (chỉ khi `--checks=proven`); khối không có terminator: không có sau validate |
| gt-oblig/src/driver.rs:353:28 | analyze_program_with | false | `&& let TerminatorKind::Assert { target, .. } = &term.kind` | C | ĐỌC MÃ | như dòng 352 (terminator không phải Assert) |
| gt-oblig/src/driver.rs:359:16 | analyze_program_with | true | `if let Err(errs) = gt_mir::validate::validate(&ob, &t.db) {` | C | ĐỌC MÃ | `gt_mir::validate` lỗi sau khi xoá check ⇒ E9901: chỉ khi thuật toán xoá sinh MIR hỏng (đột biến); không đạt tới từ chương trình |
| gt-oblig/src/driver.rs:469:20 | escaped_sites | false | `if let Some(t) = &bb.terminator {` | C | ĐỌC MÃ | khối không có terminator (như 179) |
| gt-borrowck/src/check.rs:424:24 | check_uses | false | `if exact && us.contains(node) {` | D | CHƯA | `exact && us.contains(node)` sai: điểm dùng không chính xác; cần chương trình có vùng không-chính-xác; chưa thí nghiệm |
| gt-borrowck/src/regions.rs:326:12 | relate | true | `if src.len() != dst.len() {` | C | ĐỌC MÃ | số vị trí region nguồn ≠ đích: kiểu đã qua unify nên khớp cấu trúc; nhánh `defend` ghi nhận (RELATE_MISMATCHES) |
| gt-borrowck/src/regions.rs:335:20 | relate | true | `if src.is_empty() {` | C | ĐỌC MÃ | tham chiếu không có vị trí region: mọi `&T` có region (do M4); `defend` |
| gt-borrowck/src/regions.rs:370:24 | relate | false | `if o + n <= src.len() {` | C | ĐỌC MÃ | đối số kiểu vượt số vị trí region: `defend` |
| gt-borrowck/src/solve.rs:55:16 | live_before | false | `if let Some(t) = &bb.terminator {` | C | ĐỌC MÃ | khối không có terminator (như driver 179) |
| gt-types/src/send.rs:64:12 | auto_rec | true | `if depth > 64 {` | B | TN | kiểu mảng lồng 70 tầng truyền vào `spawn` ⇒ ĐỎ (E0312 + E0501 "lồng quá sâu"); chạy trực tiếp, không trong corpus (hai mã) |
| gt-types/src/send.rs:122:20 | auto_rec | true | `if seen` | D | CHƯA | kiểu đệ quy khi kiểm Send (`seen`): cần ADT đệ quy qua tham chiếu/Arena; chưa thí nghiệm |
| gt-types/src/send.rs:124:52 | auto_rec | true | `.any(\|(sd, sa, sw)\| sd == d && sa == args && *sw == which)` | D | CHƯA | như dòng 122 |
| gt-types/src/db.rs:126:35 | select_impl | true | `if select_impl_fault() \|\| ty.has_param() \|\| ty.has_infer() {` | A | ĐỌC MÃ | `select_impl_fault()`: hook kiểm thử (không bật được từ chương trình) |
| gt-types/src/db.rs:126:53 | select_impl | true | `if select_impl_fault() \|\| ty.has_param() \|\| ty.has_infer() {` | C | ĐỌC MÃ | `ty.has_param() || has_infer()` khi chọn impl: kiểu đã giải trước khi chọn impl |
| gt-types/src/db.rs:129:12 | select_impl | true | `if select_impl_first() {` | A | ĐỌC MÃ | `select_impl_first()`: hook kiểm thử |
| gt-types/src/check/pat.rs:15:39 | destructures | false | `Some(Res::Def(vd)) if self.env.defs()[vd.index()].kind == DefKind::Variant` | C | TN | binding tên resolve tới Def không phải biến thể: bị E0205 (che khuất tên cấp module) ở resolve trước khi tới checker: thử `■ K` làm pattern ⇒ E0205 (không phải E0301) |
| gt-types/src/check/pat.rs:64:39 | pat_inner | false | `Some(Res::Def(vd)) if self.env.defs()[vd.index()].kind == DefKind::Variant => {` | C | TN | như dòng 15 (E0205 chặn) |
| gt-types/src/check/pat.rs:79:39 | pat_inner | false | `Some(Res::Def(vd)) if self.env.defs()[vd.index()].kind == DefKind::Variant => {` | C | TN | như dòng 15 |
| gt-types/src/check/pat.rs:222:49 | range_pat | false | `\|\| matches!(rt, Ty::Infer(i) if self.infer.kind_of(i) == crate::infer::VarKind::Int);` | D | CHƯA | pattern khoảng trên biến suy kiểu không phải Int: cần kiểu Infer khác Int; chưa thí nghiệm |
| gt-types/src/check/pat.rs:268:13 | variant_pat | false | `let Some(adt) = parent.and_then(\|x\| self.env.db.adts.get(&x)).cloned() else {` | C | ĐỌC MÃ | `pat_db_fault`: "db không nhất quán" (ADT cha không có) ⇒ ICE theo thiết kế R3; cần db hỏng |
| gt-types/src/check/pat.rs:271:13 | variant_pat | false | `let AdtKind::Enum(vs) = &adt.kind else {` | C | ĐỌC MÃ | `pat_db_fault`: cha biến thể không phải enum |
| gt-types/src/check/pat.rs:274:13 | variant_pat | false | `let Some(vi) = vs.iter().position(\|v\| v.def == vd) else {` | C | ĐỌC MÃ | `pat_db_fault`: biến thể không có trong enum cha |
| gt-types/src/check/pat.rs:347:29 | tuple_pat | true | `Ty::Infer(_) if !has_rest => {` | D | CHƯA | pattern tuple trên kiểu chưa suy ra (Infer) không có `..`; chưa dựng được |
| gt-types/src/check/pat.rs:347:29 | tuple_pat | false | `Ty::Infer(_) if !has_rest => {` | D | CHƯA | như dòng 347 true |
| gt-types/src/check/pat.rs:421:13 | struct_pat | false | `let Some(adt) = self.env.db.adts.get(&d).cloned() else {` | C | ĐỌC MÃ | db thiếu ADT cho struct pattern (ICE phòng thủ) |
| gt-types/src/check/pat.rs:431:13 | struct_pat | false | `let AdtKind::Struct(defs) = &adt.kind else {` | C | ĐỌC MÃ | ADT không phải struct trong struct pattern (ICE phòng thủ) |
| gt-types/src/check/pat.rs:473:16 | struct_pat | false | `if !defs[i].public && !self.is_within(self.module, adt.module) {` | D | CHƯA | trường riêng tư của struct ở module khác trong pattern; cần gói nhiều tệp (apps hỗ trợ); chưa thí nghiệm |
| gt-types/src/check/pat.rs:545:19 | decode_char | false | `return if it.next().is_none() {` | C | ĐỌC MÃ+lexer | literal char nhiều ký tự bị lexer từ chối (`tests/fail/E0101_char_multi.gt`) nên không tới `decode_char` |
| gt-types/src/check/call.rs:9:13 | fn_arg_effects | false | `let ExprKind::Path(p) = &a.kind else {` | D | CHƯA | đối số hàm không phải Path cho tham số `{poly}`: cần biểu thức hàm trả giá trị hàm; chưa dựng được |
| gt-types/src/check/call.rs:13:28 | fn_arg_effects | false | `Res::Def(d) if self.env.defs()[d.index()].kind == DefKind::Fn => {` | D | CHƯA | như dòng 9 (Path tới thứ không phải hàm/biến cục bộ) |
| gt-types/src/check/call.rs:33:20 | poly_param | false | `&& let Some(e) = self.fn_arg_effects(a)` | D | CHƯA | như dòng 9 |
| gt-types/src/check/call.rs:50:13 | path_value | false | `let Some(res) = self.env.res.uses.get(&p.id).cloned() else {` | C | ĐỌC MÃ | Path không có kết quả resolve: resolver đã báo lỗi và dừng |
| gt-types/src/check/call.rs:75:21 | def_value | false | `let Some((sig, _)) = self.env.db.fns.get(&d).cloned() else {` | C | ĐỌC MÃ | Def Fn không có chữ ký trong db (ICE phòng thủ: trả Ty::Err) |
| gt-types/src/check/call.rs:116:21 | def_value | false | `let Some(adt) = parent.and_then(\|p\| self.env.db.adts.get(&p)).cloned() else {` | C | ĐỌC MÃ | biến thể không có ADT cha trong db (phòng thủ) |
| gt-types/src/check/call.rs:119:21 | def_value | false | `let AdtKind::Enum(vs) = &adt.kind else {` | C | ĐỌC MÃ | cha không phải enum (phòng thủ) |
| gt-types/src/check/call.rs:122:21 | def_value | false | `let Some(v) = vs.iter().find(\|v\| v.def == d) else {` | C | ĐỌC MÃ | biến thể không có trong enum cha (phòng thủ) |
| gt-types/src/check/call.rs:168:28 | assoc_value | false | `Res::Def(d) if self.env.db.traits.contains_key(d) => {` | D | TN-KHÔNG-CHẠM | `X::tên` với X là fn: bị E0201 "là fn, không phải module hoặc kiểu" ở resolve (thử `g::foo(1u32)`), không tới checker |
| gt-types/src/check/call.rs:272:12 | reject_intrinsic_value | true | `if crate::ALLOW_INTRINSIC_VALUES` | A | ĐỌC MÃ | `ALLOW_INTRINSIC_VALUES` là hằng cấu hình (false); có test riêng khi bật |
| gt-types/src/check/call.rs:287:20 | reject_intrinsic_value | false | `&& let Some(sig) = self.env.db.sig_by_node(callee_node)` | C | ĐỌC MÃ | `sig_by_node` None cho callee prelude: mọi hàm prelude có chữ ký |
| gt-types/src/check/call.rs:289:16 | reject_intrinsic_value | false | `&& !sig.params.iter().any(\|p\| self.has_cap(p))` | D | ĐỌC MÃ | KHÔNG đạt tới: cần hàm prelude TRẢ `Cap` và có tham số `Cap` KHÔNG tính `self`; prelude hiện không có hàm nào như vậy (`Cap::sub` chỉ có `&str` ngoài self); hàm người dùng không phải prelude nên thoát sớm ở `is_prelude` |
| gt-types/src/check/call.rs:310:20 | apply_pick_types | false | `&& let Some(node) = self` | C | ĐỌC MÃ | impl theo `impl_node` không có thành viên (phòng thủ) |
| gt-types/src/check/call.rs:367:30 | pick_effects | false | `Some(effects) if effects == pick.effects => pick.effects.clone(),` | D | CHƯA | `pick_effects` khác hiệu ứng ứng viên; chưa thí nghiệm |
| gt-types/src/check/call.rs:430:21 | record_pick_site | false | `let Some(callee) = callee else {` | C | ĐỌC MÃ | `record_pick_site` không có callee (phòng thủ) |
| gt-types/src/check/call.rs:512:41 | call | false | `if r.has_infer() && self.infer.probe(\|inf\| inf.unify(&r, x)) {` | D | CHƯA | đối số suy kiểu mà `probe(unify)` thất bại; chưa thí nghiệm |
| gt-types/src/check/call.rs:553:31 | method_call | true | `let code = if msg.contains("chưa đủ thông tin") {` | D | TN-KHÔNG-CHẠM | gọi phương thức trên giá trị chưa suy ra kiểu: `x ≔ None; x.foo()` ⇒ HAI lỗi E0301 + E0309 (không dùng được làm mẫu một mã); lời nhắn "chưa đủ thông tin" xuất hiện ở E0301 trước |
| gt-types/src/validate.rs:25:13 | validate_trait_impl | false | `let Some(td) = self.db.traits.get(&tr.trait_def).cloned() else {` | C | ĐỌC MÃ | impl trỏ tới trait không có trong db (ICE phòng thủ) |
| gt-types/src/validate.rs:112:13 | regions_compatible | false | `let (Some(ts), Some(is)) = (` | C | ĐỌC MÃ | thiếu thông tin region của chữ ký trait/impl (phòng thủ) |
| gt-types/src/validate.rs:140:12 | regions_compatible | true | `if ti.len() != ii.len() \|\| to.len() != io.len() {` | D | TN-KHÔNG-CHẠM | số vị trí region đầu vào/đầu ra khác nhau giữa trait và impl: thử impl có `&u32` thay tham số tường minh ⇒ E0315 ở elision (không tới đây) |
| gt-types/src/validate.rs:140:36 | regions_compatible | true | `if ti.len() != ii.len() \|\| to.len() != io.len() {` | D | TN-KHÔNG-CHẠM | như dòng 140:12 |
| gt-types/src/validate.rs:202:16 | validate_copy_impl | false | `if !matches!(imp.self_ty, Ty::Err) {` | C | TN | `⊐ Copy for Nope` (self_ty Err): E0201 chặn ở resolve trước (thử `v4`) |
| gt-types/src/validate.rs:212:13 | validate_copy_impl | false | `let Some(adt) = self.db.adts.get(d).cloned() else {` | C | ĐỌC MÃ | ADT không có trong db khi kiểm Copy (phòng thủ) |
| gt-types/src/validate.rs:274:42 | is_copy_ignoring_self | true | `if matches!(ty, Ty::Adt(d, _) if *d == own) {` | C | TN | Copy impl cho struct chứa chính nó theo giá trị: bị E0301 "kích thước vô hạn" ở `collect` trước (thử `▢ N { a: N }`) |
| gt-term/src/lib.rs:643:13 | check_termination | false | `let Some(f) = fns.get(&b.node) else { continue };` | C | ĐỌC MÃ | thân không có chữ ký hàm khi gom biến thể: mọi thân hàm có chữ ký |
| gt-term/src/lib.rs:748:12 | check_termination | true | `if !fns.contains_key(&b.node) {` | C | ĐỌC MÃ | ICE `thân có vòng lặp nhưng không có chữ ký hàm`: cần thân (hằng/static) có vòng lặp; hằng/static chỉ nhận biểu thức không có vòng lặp (bị parser/types chặn): ĐỌC MÃ, chưa kiểm bằng thí nghiệm riêng |
| gt-term/src/lib.rs:750:16 | check_termination | true | `if !b.loops.is_empty() {` | C | ĐỌC MÃ | như dòng 748 (`!b.loops.is_empty()` bên trong) |
| gt-term/src/lib.rs:750:16 | check_termination | false | `if !b.loops.is_empty() {` | C | ĐỌC MÃ | như dòng 748 |
| gt-term/src/lib.rs:908:61 | check_termination | true | `if need > gt_oblig::analysis::GHOST_MAX_VARS \|\| dropped {` | D | CHƯA | nhánh MỚI do bản sửa của gói này: `zone_dropped` không rỗng làm điều kiện lời nhắc đúng; difflite bản đầu (apps, `main` 33 biến) cho thấy tồn tại chương trình đúng bị E0702 vì hơn 24 biến, nhưng CHƯA rút gọn thành mẫu nhỏ (thử 30 biến + một vòng đơn giản: KHÔNG bị từ chối, 0 chẩn đoán) |
| gt-term/src/lib.rs:323:12 | instance_graph | false | `if by_node.contains_key(&i.node) {` | C | ĐỌC MÃ | instance của thân không có trong `by_node`: instance sinh từ thân có thật |
| gt-term/src/lib.rs:339:13 | instance_graph | false | `let Some(&bi) = by_node.get(&node) else {` | C | ĐỌC MÃ | như dòng 323 |
| gt-term/src/lib.rs:368:17 | instance_graph | false | `let Some(term) = bb.terminator.as_ref() else {` | C | ĐỌC MÃ | khối không có terminator |
| gt-term/src/lib.rs:404:24 | instance_graph | true | `if st.has_param() \|\| st.has_infer() {` | C | ĐỌC MÃ | `self_ty` của lời gọi trait còn tham số/biến suy: instance của mono luôn cụ thể (đã thay thế) |
| gt-term/src/lib.rs:404:42 | instance_graph | true | `if st.has_param() \|\| st.has_infer() {` | C | ĐỌC MÃ | như dòng 404:24 |
| gt-term/src/lib.rs:409:24 | instance_graph | false | `if by_node.contains_key(&n2) {` | C | ĐỌC MÃ | đích trait method không có thân (intrinsic/không MIR) |
| gt-term/src/lib.rs:411:28 | instance_graph | false | `if !DROP_TRAIT_EDGES.load(std::sync::atomic::Ordering::SeqCst) {` | A | ĐỌC MÃ | `DROP_TRAIT_EDGES`: chốt toàn cục chỉ để kiểm thử (test `conservative_fallback.rs`) |
| gt-term/src/lib.rs:421:11 | instance_graph | true | `while edges.len() < n {` | C | ĐỌC MÃ | bù cạnh cho instance chưa xử lý: mọi instance đều đã xử lý trong vòng trước |
| gt-term/src/lib.rs:461:15 | find | true | `while p[y] != r {` | D | TN-KHÔNG-CHẠM | nén đường của union-find ở `find`: thử `tests/pass/mutual_recursion_four_cycle_cov.gt` (đệ quy tương hỗ 4 vòng, xanh) KHÔNG chạm (đo lần 2); cần chuỗi cha dài ≥ 2 trong union-find; chưa biết dạng chương trình |
| gt-term/src/lib.rs:472:13 | find | false | `let Some(&bi) = by_node.get(&list[k].0) else {` | C | ĐỌC MÃ | như dòng 323 |
| gt-term/src/lib.rs:221:31 | sccs | false | `while let Some(w) = stack.pop() {` | C | ĐỌC MÃ | `stack.pop()` None trong Tarjan: gốc SCC luôn còn trong ngăn xếp (bất biến thuật toán) |
| gt-effects/src/lib.rs:91:25 | check_effects | false | `let Some(cs) = t.db.sig_by_node(*node) else {` | C | ĐỌC MÃ | `sig_by_node` None cho hàm có thân: mọi hàm có chữ ký |
| gt-effects/src/lib.rs:117:25 | check_effects | false | `let Some(cs) = t` | C | ĐỌC MÃ | như dòng 91 |
| gt-effects/src/lib.rs:176:13 | check_effects | false | `let Some(td) = t.db.traits.get(&tr.trait_def) else {` | C | ĐỌC MÃ | impl trỏ trait không có trong bảng ⇒ E9901 (nhánh ICE có chủ ý) |
| gt-effects/src/lib.rs:177:20 | check_effects | true | `if let Some(m0) = imp.members.first() {` | C | ĐỌC MÃ | như dòng 176 (có thành viên đầu tiên để gắn lỗi) |
| gt-effects/src/lib.rs:177:20 | check_effects | false | `if let Some(m0) = imp.members.first() {` | C | ĐỌC MÃ | như dòng 176 (impl rỗng) |
| gt-effects/src/lib.rs:190:17 | check_effects | false | `let Some(tm) = td.members.iter().find(\|x\| x.name == mem.name) else {` | C | ĐỌC MÃ | thành viên impl không có trong trait ⇒ E9901: validate (E0311) đã chặn trước |
| gt-guard/src/lib.rs:191:21 | runtime_address_violations | false | `callee_name(&i).as_deref() == Some(name.as_str())` | C | ĐỌC MÃ | so tên callee với tên hàm runtime: hướng "khác tên" của bộ so; cần đọc mã để xác định (CHƯA chứng minh không đạt tới) |
