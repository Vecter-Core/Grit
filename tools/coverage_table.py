#!/usr/bin/env python3
"""GÓI 6 S1.a: BẢNG TỪNG NHÁNH chưa chạy (đếm thật từ cov.json) kèm nhóm A/B/C/D, thí nghiệm hoặc lý do, và cột "nguồn" (TN = có thí nghiệm thật; ĐỌC MÃ = chỉ đọc mã; CHƯA = chưa làm).
  python tools/coverage_table.py cov.json.gz > docs/DECISION_COVERAGE_BRANCHES.md
Chú giải nhóm: A = hook chỉ-kiểm-thử/hằng cấu hình; B = giới hạn tài nguyên; C = nhánh phòng thủ (ICE/let-else) cho trạng thái do giai đoạn trước đảm bảo; D = đạt tới được từ chương trình người dùng.
Khoá chú thích: `<crate>/<tệp>:<dòng>:<hướng>` (hướng = true/false; khi trùng dòng thì có thêm cột trong khoá `@cột`)."""
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "tools"))
import coverage_branches as cb  # noqa: E402

R = "ĐỌC MÃ"
# khoá -> (nhóm, nguồn, thí nghiệm/lý do)
ANN = {
    "gt-oblig/analysis.rs:978:false": ("D", "TN-KHÔNG-CHẠM", "cần biến có cận dưới vô hạn qua `hyps_for`; thử `tests/pass/loop_downward_i64_cov.gt` (xanh) nhưng KHÔNG chạm; hướng sai chỉ BỚT giả thiết ⇒ ít chứng minh hơn ⇒ giữ check (lập luận)"),
    "gt-oblig/analysis.rs:983:false": ("D", "TN-KHÔNG-CHẠM", "như dòng 978 với cận trên vô hạn"),
    "gt-oblig/analysis.rs:907:34:false": ("D", "TN-KHÔNG-CHẠM", "biến không theo dõi nhận khoảng; thử `pass/many_locals_beyond_zone_cov.gt` (40 biến, xanh + ghi chú) KHÔNG chạm"),
    "gt-oblig/analysis.rs:2010:true": ("B", "TN-NGOÀI-CORPUS", "`UNREP_MAX_BLOCKS`: hàm 16 000 `⎇` ⇒ xanh + N0605 \"hết ngân sách bước\" (LIMITS_AUDIT #3); quá chậm để vào corpus (37 s release, > 15 phút debug)"),
    "gt-oblig/analysis.rs:2025:true": ("B", R, "`MAX_BLOCK_STEPS` = 400 000 bước/khối: chưa dựng được chương trình chạm riêng giới hạn này (LIMITS_AUDIT #4)"),
    "gt-oblig/analysis.rs:2035:false": ("C", R, "`back_edge && visits[t] >= 1` sai khi cạnh lùi tới khối CHƯA thăm: thứ tự duyệt hậu thứ tự luôn thăm đích cạnh lùi trước (bất biến của thuật toán)"),
    "gt-oblig/analysis.rs:292:true": ("B", "TN-KHÔNG-CHẠM", "`widen` với môi trường chết: thử `pass/dead_loop_constant_false_cov.gt`, `pass/dead_inner_loop_cov.gt` (xanh) KHÔNG chạm: vòng trong nhánh chết bị bỏ trước khi tới widen"),
    "gt-oblig/analysis.rs:295:true": ("B", "TN-KHÔNG-CHẠM", "như dòng 292 (môi trường mới chết)"),
    "gt-oblig/driver.rs:144:true": ("C", R, "`analyze_program_with` chỉ gọi khi M5 không có chẩn đoán (pipeline bỏ qua M7 nếu M5 đỏ); nhánh phòng thủ trả rỗng"),
    "gt-oblig/driver.rs:179:false": ("C", R, "khối MIR không có terminator: MIR đã qua `validate` nên không có"),
    "gt-oblig/driver.rs:190:false": ("C", R, "không tìm thấy chữ ký của thân hàm: mọi thân hàm có chữ ký (do M4)"),
    "gt-oblig/driver.rs:201:false": ("C", R, "tra thành viên impl theo tên (phòng thủ)"),
    "gt-oblig/driver.rs:203:true": ("D", "CHƯA", "thân thuộc hàm có hợp đồng/precondition hoặc thành viên impl; hướng `true` phụ thuộc cách duyệt `info.pre`; chưa thí nghiệm"),
    "gt-oblig/driver.rs:242:27:true": ("D", "TN", "`tests/fail/E0601_literal_negative_out_of_range_cov.gt` (-129i8 ⇒ E0601); phủ đo ở lần chạy sau"),
    "gt-oblig/driver.rs:268:true": ("C", R, "`rep.ice`: ICE của phân tích một thân; chỉ có khi lỗi nội bộ (chèn lỗi test `fault_dropped_obligations`); không đạt tới từ chương trình Grit đúng"),
    "gt-oblig/driver.rs:291:true": ("B", "TN-NGOÀI-CORPUS", "`budget_exhausted`: hàm 16 000 `⎇` in ghi chú N0605 \"hết ngân sách bước phân tích `main`\" (đã thấy), nhưng quá chậm để vào corpus"),
    "gt-oblig/driver.rs:352:false": ("C", R, "tìm Assert terminator đích để cắt sau khi chứng minh (chỉ khi `--checks=proven`); khối không có terminator: không có sau validate"),
    "gt-oblig/driver.rs:353:false": ("C", R, "như dòng 352 (terminator không phải Assert)"),
    "gt-oblig/driver.rs:359:true": ("C", R, "`gt_mir::validate` lỗi sau khi xoá check ⇒ E9901: chỉ khi thuật toán xoá sinh MIR hỏng (đột biến); không đạt tới từ chương trình"),
    "gt-oblig/driver.rs:469:false": ("C", R, "khối không có terminator (như 179)"),
    "gt-borrowck/check.rs:424:false": ("D", "CHƯA", "`exact && us.contains(node)` sai: điểm dùng không chính xác; cần chương trình có vùng không-chính-xác; chưa thí nghiệm"),
    "gt-borrowck/regions.rs:326:true": ("C", R, "số vị trí region nguồn ≠ đích: kiểu đã qua unify nên khớp cấu trúc; nhánh `defend` ghi nhận (RELATE_MISMATCHES)"),
    "gt-borrowck/regions.rs:335:true": ("C", R, "tham chiếu không có vị trí region: mọi `&T` có region (do M4); `defend`"),
    "gt-borrowck/regions.rs:370:false": ("C", R, "đối số kiểu vượt số vị trí region: `defend`"),
    "gt-borrowck/solve.rs:55:false": ("C", R, "khối không có terminator (như driver 179)"),
    "gt-types/send.rs:64:true": ("B", "TN", "kiểu mảng lồng 70 tầng truyền vào `spawn` ⇒ ĐỎ (E0312 + E0501 \"lồng quá sâu\"); chạy trực tiếp, không trong corpus (hai mã)"),
    "gt-types/send.rs:122:true": ("D", "CHƯA", "kiểu đệ quy khi kiểm Send (`seen`): cần ADT đệ quy qua tham chiếu/Arena; chưa thí nghiệm"),
    "gt-types/send.rs:124:52:true": ("D", "CHƯA", "như dòng 122"),
    "gt-types/db.rs:126:35:true": ("A", R, "`select_impl_fault()`: hook kiểm thử (không bật được từ chương trình)"),
    "gt-types/db.rs:126:53:true": ("C", R, "`ty.has_param() || has_infer()` khi chọn impl: kiểu đã giải trước khi chọn impl"),
    "gt-types/db.rs:129:true": ("A", R, "`select_impl_first()`: hook kiểm thử"),
    "gt-types/pat.rs:15:false": ("C", "TN", "binding tên resolve tới Def không phải biến thể: bị E0205 (che khuất tên cấp module) ở resolve trước khi tới checker: thử `■ K` làm pattern ⇒ E0205 (không phải E0301)"),
    "gt-types/pat.rs:64:false": ("C", "TN", "như dòng 15 (E0205 chặn)"),
    "gt-types/pat.rs:79:false": ("C", "TN", "như dòng 15"),
    "gt-types/pat.rs:137:false": ("D", "TN", "`tests/fail/E0301_pattern_variant_on_int_cov.gt` (`Some(x)` trên u32 ⇒ E0301)"),
    "gt-types/pat.rs:222:49:false": ("D", "CHƯA", "pattern khoảng trên biến suy kiểu không phải Int: cần kiểu Infer khác Int; chưa thí nghiệm"),
    "gt-types/pat.rs:268:false": ("C", R, "`pat_db_fault`: \"db không nhất quán\" (ADT cha không có) ⇒ ICE theo thiết kế R3; cần db hỏng"),
    "gt-types/pat.rs:271:false": ("C", R, "`pat_db_fault`: cha biến thể không phải enum"),
    "gt-types/pat.rs:274:false": ("C", R, "`pat_db_fault`: biến thể không có trong enum cha"),
    "gt-types/pat.rs:347:29:true": ("D", "CHƯA", "pattern tuple trên kiểu chưa suy ra (Infer) không có `..`; chưa dựng được"),
    "gt-types/pat.rs:347:29:false": ("D", "CHƯA", "như dòng 347 true"),
    "gt-types/pat.rs:421:false": ("C", R, "db thiếu ADT cho struct pattern (ICE phòng thủ)"),
    "gt-types/pat.rs:431:false": ("C", R, "ADT không phải struct trong struct pattern (ICE phòng thủ)"),
    "gt-types/pat.rs:473:false": ("D", "CHƯA", "trường riêng tư của struct ở module khác trong pattern; cần gói nhiều tệp (apps hỗ trợ); chưa thí nghiệm"),
    "gt-types/pat.rs:545:false": ("C", "ĐỌC MÃ+lexer", "literal char nhiều ký tự bị lexer từ chối (`tests/fail/E0101_char_multi.gt`) nên không tới `decode_char`"),
    "gt-types/call.rs:9:false": ("D", "CHƯA", "đối số hàm không phải Path cho tham số `{poly}`: cần biểu thức hàm trả giá trị hàm; chưa dựng được"),
    "gt-types/call.rs:13:false": ("D", "CHƯA", "như dòng 9 (Path tới thứ không phải hàm/biến cục bộ)"),
    "gt-types/call.rs:33:false": ("D", "CHƯA", "như dòng 9"),
    "gt-types/call.rs:50:false": ("C", R, "Path không có kết quả resolve: resolver đã báo lỗi và dừng"),
    "gt-types/call.rs:75:false": ("C", R, "Def Fn không có chữ ký trong db (ICE phòng thủ: trả Ty::Err)"),
    "gt-types/call.rs:116:false": ("C", R, "biến thể không có ADT cha trong db (phòng thủ)"),
    "gt-types/call.rs:119:false": ("C", R, "cha không phải enum (phòng thủ)"),
    "gt-types/call.rs:122:false": ("C", R, "biến thể không có trong enum cha (phòng thủ)"),
    "gt-types/call.rs:168:false": ("D", "TN-KHÔNG-CHẠM", "`X::tên` với X là fn: bị E0201 \"là fn, không phải module hoặc kiểu\" ở resolve (thử `g::foo(1u32)`), không tới checker"),
    "gt-types/call.rs:272:true": ("A", R, "`ALLOW_INTRINSIC_VALUES` là hằng cấu hình (false); có test riêng khi bật"),
    "gt-types/call.rs:287:false": ("C", R, "`sig_by_node` None cho callee prelude: mọi hàm prelude có chữ ký"),
    "gt-types/call.rs:289:false": ("D", "ĐỌC MÃ", "KHÔNG đạt tới: cần hàm prelude TRẢ `Cap` và có tham số `Cap` KHÔNG tính `self`; prelude hiện không có hàm nào như vậy (`Cap::sub` chỉ có `&str` ngoài self); hàm người dùng không phải prelude nên thoát sớm ở `is_prelude`"),
    "gt-types/call.rs:310:false": ("C", R, "impl theo `impl_node` không có thành viên (phòng thủ)"),
    "gt-types/call.rs:367:false": ("D", "CHƯA", "`pick_effects` khác hiệu ứng ứng viên; chưa thí nghiệm"),
    "gt-types/call.rs:430:false": ("C", R, "`record_pick_site` không có callee (phòng thủ)"),
    "gt-types/call.rs:512:false": ("D", "CHƯA", "đối số suy kiểu mà `probe(unify)` thất bại; chưa thí nghiệm"),
    "gt-types/call.rs:553:true": ("D", "TN-KHÔNG-CHẠM", "gọi phương thức trên giá trị chưa suy ra kiểu: `x ≔ None; x.foo()` ⇒ HAI lỗi E0301 + E0309 (không dùng được làm mẫu một mã); lời nhắn \"chưa đủ thông tin\" xuất hiện ở E0301 trước"),
    "gt-types/validate.rs:25:false": ("C", R, "impl trỏ tới trait không có trong db (ICE phòng thủ)"),
    "gt-types/validate.rs:112:false": ("C", R, "thiếu thông tin region của chữ ký trait/impl (phòng thủ)"),
    "gt-types/validate.rs:140:12:true": ("D", "TN-KHÔNG-CHẠM", "số vị trí region đầu vào/đầu ra khác nhau giữa trait và impl: thử impl có `&u32` thay tham số tường minh ⇒ E0315 ở elision (không tới đây)"),
    "gt-types/validate.rs:140:36:true": ("D", "TN-KHÔNG-CHẠM", "như dòng 140:12"),
    "gt-types/validate.rs:202:false": ("C", "TN", "`⊐ Copy for Nope` (self_ty Err): E0201 chặn ở resolve trước (thử `v4`)"),
    "gt-types/validate.rs:212:false": ("C", R, "ADT không có trong db khi kiểm Copy (phòng thủ)"),
    "gt-types/validate.rs:274:true": ("C", "TN", "Copy impl cho struct chứa chính nó theo giá trị: bị E0301 \"kích thước vô hạn\" ở `collect` trước (thử `▢ N { a: N }`)"),
    "gt-term/lib.rs:643:false": ("C", R, "thân không có chữ ký hàm khi gom biến thể: mọi thân hàm có chữ ký"),
    "gt-term/lib.rs:748:true": ("C", R, "ICE `thân có vòng lặp nhưng không có chữ ký hàm`: cần thân (hằng/static) có vòng lặp; hằng/static chỉ nhận biểu thức không có vòng lặp (bị parser/types chặn): ĐỌC MÃ, chưa kiểm bằng thí nghiệm riêng"),
    "gt-term/lib.rs:750:16:true": ("C", R, "như dòng 748 (`!b.loops.is_empty()` bên trong)"),
    "gt-term/lib.rs:750:16:false": ("C", R, "như dòng 748"),
    "gt-term/lib.rs:323:false": ("C", R, "instance của thân không có trong `by_node`: instance sinh từ thân có thật"),
    "gt-term/lib.rs:339:false": ("C", R, "như dòng 323"),
    "gt-term/lib.rs:368:false": ("C", R, "khối không có terminator"),
    "gt-term/lib.rs:404:24:true": ("C", R, "`self_ty` của lời gọi trait còn tham số/biến suy: instance của mono luôn cụ thể (đã thay thế)"),
    "gt-term/lib.rs:404:42:true": ("C", R, "như dòng 404:24"),
    "gt-term/lib.rs:409:false": ("C", R, "đích trait method không có thân (intrinsic/không MIR)"),
    "gt-term/lib.rs:411:false": ("A", R, "`DROP_TRAIT_EDGES`: chốt toàn cục chỉ để kiểm thử (test `conservative_fallback.rs`)"),
    "gt-term/lib.rs:421:true": ("C", R, "bù cạnh cho instance chưa xử lý: mọi instance đều đã xử lý trong vòng trước"),
    "gt-term/lib.rs:461:true": ("D", "TN-KHÔNG-CHẠM", "nén đường của union-find ở `find`: thử `tests/pass/mutual_recursion_four_cycle_cov.gt` (đệ quy tương hỗ 4 vòng, xanh) KHÔNG chạm (đo lần 2); cần chuỗi cha dài ≥ 2 trong union-find; chưa biết dạng chương trình"),
    "gt-term/lib.rs:472:false": ("C", R, "như dòng 323"),
    "gt-term/lib.rs:221:false": ("C", R, "`stack.pop()` None trong Tarjan: gốc SCC luôn còn trong ngăn xếp (bất biến thuật toán)"),
    "gt-effects/lib.rs:91:false": ("C", R, "`sig_by_node` None cho hàm có thân: mọi hàm có chữ ký"),
    "gt-effects/lib.rs:117:false": ("C", R, "như dòng 91"),
    "gt-effects/lib.rs:176:false": ("C", R, "impl trỏ trait không có trong bảng ⇒ E9901 (nhánh ICE có chủ ý)"),
    "gt-effects/lib.rs:177:20:true": ("C", R, "như dòng 176 (có thành viên đầu tiên để gắn lỗi)"),
    "gt-effects/lib.rs:177:20:false": ("C", R, "như dòng 176 (impl rỗng)"),
    "gt-effects/lib.rs:190:false": ("C", R, "thành viên impl không có trong trait ⇒ E9901: validate (E0311) đã chặn trước"),
    "gt-term/lib.rs:908:61:true": ("D", "CHƯA", "nhánh MỚI do bản sửa của gói này: `zone_dropped` không rỗng làm điều kiện lời nhắc đúng; difflite bản đầu (apps, `main` 33 biến) cho thấy tồn tại chương trình đúng bị E0702 vì hơn 24 biến, nhưng CHƯA rút gọn thành mẫu nhỏ (thử 30 biến + một vòng đơn giản: KHÔNG bị từ chối, 0 chẩn đoán)"),
    "gt-guard/lib.rs:191:21:false": ("C", R, "so tên callee với tên hàm runtime: hướng \"khác tên\" của bộ so; cần đọc mã để xác định (CHƯA chứng minh không đạt tới)"),
}


def key(rel, line, col, d, dup):
    base = rel.replace("crates/", "").replace("/src/", "/").replace("check/", "")
    return f"{base}:{line}:{col}:{d}" if (base, line, d) in dup else f"{base}:{line}:{d}"


def main():
    rows, total = cb.collect(sys.argv[1])
    cnt = {}
    for r in rows:
        k = (r[0].replace("crates/", "").replace("/src/", "/").replace("check/", ""), r[1], r[4])
        cnt[k] = cnt.get(k, 0) + 1
    dup = {k for k, v in cnt.items() if v > 1}
    out = []
    groups, srcs, miss = {}, {}, []
    for rel, line, col, fn, d, cond in rows:
        k = key(rel, line, col, d, dup)
        base = rel.replace("crates/", "").replace("/src/", "/").replace("check/", "")
        a = ANN.get(k) or ANN.get(f"{base}:{line}:{col}:{d}")
        if a is None:
            miss.append(k)
            a = ("?", "CHƯA PHÂN LOẠI", "")
        groups[a[0]] = groups.get(a[0], 0) + 1
        srcs[a[1]] = srcs.get(a[1], 0) + 1
        out.append((rel.replace("crates/", ""), line, col, fn, d, cond, a))
    sys.stdout.reconfigure(encoding="utf-8")
    print("# DECISION_COVERAGE_BRANCHES — bảng TỪNG nhánh chưa chạy (GÓI 6 S1.a)\n")
    print(f"Sinh bằng `python tools/coverage_table.py cov.json.gz`. Tổng nhánh = {total}; chưa chạy = {len(rows)}. ĐẾM THẬT từ bảng (không ước lượng theo tên hàm):\n")
    print("| nhóm | số nhánh |\n|---|---:|")
    for g in "ABCD?":
        if g in groups:
            print(f"| {g} | {groups[g]} |")
    print("\n| nguồn | số nhánh |\n|---|---:|")
    for s, n in sorted(srcs.items(), key=lambda x: -x[1]):
        print(f"| {s} | {n} |")
    print("\nNhóm: **A** hook chỉ-kiểm-thử/hằng cấu hình · **B** giới hạn tài nguyên · **C** nhánh phòng thủ (ICE/let-else) cho trạng thái do giai đoạn trước đảm bảo · **D** đạt tới được từ chương trình người dùng.")
    print("Nguồn: **TN** = có chương trình thật; **TN-KHÔNG-CHẠM** = đã thử, chương trình chạy nhưng KHÔNG chạm nhánh; **TN-NGOÀI-CORPUS** = chạy trực tiếp nhưng quá chậm/hai mã nên không vào corpus; **ĐỌC MÃ** = chỉ đọc mã (không có thí nghiệm); **CHƯA** = chưa làm.\n")
    print("| tệp:dòng:cột | hàm | hướng | điều kiện | nhóm | nguồn | thí nghiệm / lý do |\n|---|---|---|---|---|---|---|")
    for rel, line, col, fn, d, cond, a in out:
        c = cond.replace("|", "\\|")
        print(f"| {rel}:{line}:{col} | {fn} | {d} | `{c}` | {a[0]} | {a[1]} | {a[2]} |")
    if miss:
        sys.stderr.write("CHƯA PHÂN LOẠI: " + ", ".join(miss) + "\n")
        sys.exit(1)


if __name__ == "__main__":
    main()
