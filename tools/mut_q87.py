"""Đột biến lô nhỏ, 1 worker, cho hàm quyết định mới của Q87 (gt-types pat.rs/expr.rs, hạ MIR matching.rs). Luôn hoàn nguyên tệp."""
import io, subprocess, os, sys

R = 'D:/Grit/'
T = 'crates/gt-types/src/check/pat.rs'
E = 'crates/gt-types/src/check/expr.rs'
M = 'crates/gt-mir/src/build/matching.rs'
TYPES = 'cargo test -p gt-types -- --test-threads=2'
DRV = 'cargo test -p gt-driver --test gate --test ref_pattern_real --test ref_pattern_fault --test llvm_diff_refpat -- --test-threads=2'
BC = 'cargo test -p gt-borrowck --test corpus --test diag_snapshot -- --test-threads=2'
MUTS = [
    (T, 'if self.pat_ref.is_some() {', 'if false && self.pat_ref.is_some() {', 0, 'chấp nhận tham chiếu lồng (bỏ kiểm một tầng)'),
    (T, 'PatternKind::Variant { .. } | PatternKind::Struct { .. } | PatternKind::Tuple(_) => {', 'PatternKind::Variant { .. } | PatternKind::Struct { .. } => {', 0, 'destructures bỏ Tuple (gt-types)'),
    (T, 'PatternKind::Variant { .. } | PatternKind::Struct { .. } | PatternKind::Tuple(_) => {', 'PatternKind::Variant { .. } | PatternKind::Struct { .. } | PatternKind::Tuple(_) | PatternKind::Lit(_) => {', 0, 'destructures thêm Lit (chấp nhận hằng trên &U)'),
    (T, 'Some(m) => Ty::Ref(m, Box::new(ty)),', 'Some(_) => ty,', 0, 'biến bind có kiểu V thay vì &V'),
    (T, 'Some(m) => Ty::Ref(m, Box::new(ty)),', 'Some(_) => Ty::Ref(false, Box::new(ty)),', 0, 'biến bind luôn &V (mất &!)'),
    (T, '_ if self.pat_poison => Ty::Err,', '_ if false => Ty::Err,', 0, 'bỏ kiểu Err cho biến của mẫu lỗi'),
    (T, 'self.pat_ref = Some(m);', 'self.pat_ref = Some(!m);', 0, 'chế độ đảo (kiểu biến)'),
    (T, 'self.pat_modes.push((p.id, m));', 'self.pat_modes.push((p.id, !m));', 0, 'ghi pat_modes đảo'),
    (T, 'self.pat_ref = None;', '', 0, 'không thoát chế độ sau pattern gốc'),
    (E, 'Ty::Ref(_, inner) if arms.iter().any(|a| self.destructures(&a.pattern)) => *inner,', 'Ty::Ref(_, inner) if false && arms.iter().any(|a| self.destructures(&a.pattern)) => *inner,', 0, 'vét cạn không bỏ & của scrutinee'),
    (M, 'let rv = b.new_borrow(k, place);', 'let rv = b.new_borrow(BorrowKind::Shared, place);', 0, 'MIR: khoản vay bind luôn chung'),
    (M, 'let deref = place.project(PlaceElem::Deref);', 'let deref = place.clone();', 0, 'MIR: bỏ Deref'),
    (M, 'Some(derived), fail, binds);', 'None, fail, binds);', 0, 'MIR: không bật chế độ cho con'),
    (M, 'self.test_pat(p, &pl, t, mode, fail, binds)?;', 'self.test_pat(p, &pl, t, None, fail, binds)?;', 0, 'MIR: mất chế độ ở mẫu con tuple/biến thể'),
    (M, 'Some(sp) => self.test_pat(sp, &sub, &fts[k], mode, fail, binds)?,', 'Some(sp) => self.test_pat(sp, &sub, &fts[k], None, fail, binds)?,', 0, 'MIR: mất chế độ ở mẫu con struct'),
    (M, 'Self::bind_kind(mode),', 'None,', 0, 'MIR: bind tên trần không phải khoản vay'),
    (M, 'Self::bind_kind(mode),', 'None,', 1, 'MIR: bind trường viết tắt không phải khoản vay'),
    (M, 'PatternKind::Variant { .. } | PatternKind::Struct { .. } | PatternKind::Tuple(_) => {', 'PatternKind::Variant { .. } | PatternKind::Struct { .. } => {', 0, 'MIR: destructures bỏ Tuple'),
    (M, 'BorrowKind::Unique\n', 'BorrowKind::Shared\n', 0, 'MIR: bind_kind chung thay duy nhất'),
    (M, 'if self.t.pat_modes.get(&p.id) != Some(&derived) {', 'if false {', 0, 'MIR: bỏ khẳng định chế độ MIR ≡ gt-types'),
    (M, '&& self.t.pat_modes.contains_key(&p.id)', '&& false', 0, 'MIR: bỏ ICE "gt-types ghi chế độ mà MIR không thấy tham chiếu"'),
]

ONLY = os.environ.get('MUT_ONLY')
OUT = io.open(R + 'target/tmp/mut_q87b.txt', 'a', encoding='utf-8')
ENV = dict(os.environ, CARGO_BUILD_JOBS='2')
for i, (f, a, b, nth, name) in enumerate(MUTS):
    if ONLY and str(i) not in ONLY.split(','):
        continue
    p = R + f
    orig = io.open(p, encoding='utf-8', newline='').read()
    crlf = '\r\n' in orig
    base = orig.replace('\r\n', '\n')
    idx = -1
    for _ in range(nth + 1):
        idx = base.index(a, idx + 1)
    new = base[:idx] + b + base[idx + len(a):]
    st = None
    try:
        io.open(p, 'w', encoding='utf-8', newline='').write(new.replace('\n', '\r\n') if crlf else new)
        for cmd in (TYPES, DRV, BC):
            for _ in range(4):
                r = subprocess.run(cmd, shell=True, cwd=R, env=ENV, capture_output=True, text=True, encoding='utf-8', errors='replace')
                txt = r.stdout + r.stderr
                if 'LNK1104' not in txt:
                    break
            if 'could not compile' in txt:
                st = 'KHÔNG BIÊN DỊCH'
                break
            if r.returncode != 0:
                st = 'BỊ BẮT bởi ' + cmd.split('-p ')[1].split(' ')[0] + ' ' + ' '.join(x for x in cmd.split() if x.startswith('--test') or x.startswith('ref_') or x in ('gate',))
                break
        if st is None:
            st = 'SỐNG SÓT'
    finally:
        io.open(p, 'w', encoding='utf-8', newline='').write(orig)
    OUT.write(f'{i}: {st} | {name}\n')
    OUT.flush()
OUT.write('XONG\n')
OUT.close()
