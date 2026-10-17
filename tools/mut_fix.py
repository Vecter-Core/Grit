"""Đột biến TAY BỔ SUNG (mục 1 của lệnh 2026-10-17: S-A…S-E) cho chỗ `tools/mut.py` không sinh được. Luật: số đột biến tay ≤ số đột biến tự động của cùng lô.
Mỗi đột biến: sửa MỘT chỗ, chạy `cargo test -p <crate>`, luôn hoàn nguyên. Kết quả ghi nối vào target/tmp/mut_fix.jsonl (trường source = "tay")."""
import hashlib, io, json, os, subprocess

R = 'D:/Grit/'
OUT = R + 'target/tmp/mut_fix.jsonl'
M = [
    # (crate, tệp, hướng, tên, cũ, mới)
    ('gt-types', 'crates/gt-types/src/exhaust.rs', 'accept', 'S-B: bỏ miền vô hạn của u128', '            Ty::Int(IntTy::U128) => None,\n', ''),
    ('gt-types', 'crates/gt-types/src/check/pat.rs', 'accept', 'S-B: signed_value kẹp literal dương', '    } else {\n        i128::try_from(v).ok()\n    }', '    } else {\n        Some(i128::try_from(v).unwrap_or(i128::MAX))\n    }'),
    ('gt-types', 'crates/gt-types/src/check/pat.rs', 'accept', 'S-B: range_pat bỏ qua cận không đọc được (cận mở)', '            let Some(v) = Self::lit_value(l) else {\n                return XPat::Opaque;\n            };', '            let Some(v) = Self::lit_value(l) else {\n                continue;\n            };'),
    ('gt-types', 'crates/gt-types/src/check/pat.rs', 'accept', 'S-B: lit_pat kẹp literal không biểu diễn', '                    Some(v) => XPat::Range(v, v),\n                    None => XPat::Opaque,\n                },', '                    Some(v) => XPat::Range(v, v),\n                    None => XPat::Range(i128::MAX, i128::MAX),\n                },'),
    ('gt-types', 'crates/gt-types/src/send.rs', 'accept', 'S-A: ADT lạ ⇒ Ok', '                    return Err(internal("ADT không có trong cơ sở kiểu"));', '                    return Ok(());'),
    ('gt-types', 'crates/gt-types/src/send.rs', 'accept', 'thông điệp Send/Sync đảo nhau', '    let name = if which == Auto::Send { "Send" } else { "Sync" };', '    let name = if which == Auto::Send { "Sync" } else { "Send" };'),
    ('gt-types', 'crates/gt-types/src/send.rs', 'accept', 'S-A: Mutex thiếu đối số ⇒ Ok', '                            None => Err(internal("`Mutex` thiếu đối số kiểu")),', '                            None => Ok(()),'),
    ('gt-conc', 'crates/gt-conc/src/lib.rs', 'accept', 'S-E: không gọi check_atomic_fn_values', '        check_atomic_fn_values(t, b, bag);', ''),
    ('gt-conc', 'crates/gt-conc/src/lib.rs', 'accept', 'S-E: bỏ rà toán hạng Aggregate', '            Rvalue::Aggregate(_, ops) => out.extend(ops.iter()),', '            Rvalue::Aggregate(_, _ops) => {}'),
    ('gt-conc', 'crates/gt-conc/src/lib.rs', 'accept', 'S-E: bỏ rà đối số lời gọi', '                    ops.extend(args.iter().map(|o| (o, sp)));', ''),
    ('gt-mir', 'crates/gt-mir/src/build/expr.rs', 'reject', 'FB-13: bỏ nhánh Res::Assoc', '                    Some(Res::Assoc { .. }) => return Ok(Operand::Const(b.fn_item_const(e, &ty)?)),\n', ''),
    ('gt-mir', 'crates/gt-mir/src/validate.rs', 'accept', 'S-D: Infer không bị coi là chưa giải quyết', '        Ty::Err | Ty::Infer(_) => true,', '        Ty::Err => true,\n        Ty::Infer(_) => false,'),
    ('gt-mir', 'crates/gt-mir/src/validate.rs', 'accept', 'S-D: đối số ADT không được rà', '        Ty::Tuple(v) | Ty::Adt(_, v) => v.iter().any(unresolved),', '        Ty::Tuple(v) => v.iter().any(unresolved),\n        Ty::Adt(_, _) => false,'),
    ('gt-mir', 'crates/gt-mir/src/validate.rs', 'accept', 'S-D: kiểu trả về của Fn không được rà', '        Ty::Fn(p, r, _) => p.iter().any(unresolved) || unresolved(r),', '        Ty::Fn(p, _r, _) => p.iter().any(unresolved),'),
    ('gt-mir', 'crates/gt-mir/src/validate.rs', 'accept', 'S-D: generics của Callee::Fn không được rà', '                            Callee::Fn { generics, .. } => tys.extend(generics.iter()),', '                            Callee::Fn { .. } => {}'),
    ('gt-mir', 'crates/gt-mir/src/validate.rs', 'accept', 'S-D: kiểu trường của phép chiếu không được rà', '        if let PlaceElem::Field(_, t) = e {\n            out.push(t);\n        }', '        if let PlaceElem::Field(_, _t) = e {}'),
]


def sh(cmd):
    env = dict(os.environ, CARGO_BUILD_JOBS='2', GRIT_LLVM_DIR='D:/Tools/llvm-20.1.8', GRIT_RT_LIB=R + 'runtime/gt-rt/target/release/gt_rt.lib')
    env['PATH'] = 'D:/Tools/llvm-20.1.8/bin;' + env['PATH']
    for _ in range(4):
        r = subprocess.run(cmd, shell=True, cwd=R, env=env, capture_output=True, text=True, encoding='utf-8', errors='replace', stdin=subprocess.DEVNULL)
        t = r.stdout + r.stderr
        if 'LNK1104' not in t:
            break
    return r.returncode, t


only = os.environ.get('MUT_ONLY')
done = set()
if os.path.exists(OUT):
    with io.open(OUT, encoding='utf-8') as f:
        for ln in f:
            try:
                done.add(json.loads(ln)['id'])
            except (ValueError, KeyError):
                pass
    if os.path.getsize(OUT) > 0:
        with io.open(OUT, 'rb') as f:
            f.seek(-1, os.SEEK_END)
            if f.read(1) != b'\n':
                with io.open(OUT, 'a', encoding='utf-8', newline='') as g:
                    g.write('\n')
out = io.open(OUT, 'a', encoding='utf-8')
for i, (crate, path, dirn, name, a, b) in enumerate(M):
    if only and str(i) not in only.split(','):
        continue
    P = R + path
    orig = io.open(P, encoding='utf-8', newline='').read()
    base = orig.replace('\r\n', '\n')
    crlf = '\r\n' in orig
    rid = hashlib.sha1(f'{path}:{name}'.encode()).hexdigest()[:12]
    if rid in done:
        continue
    row = {'id': rid, 'file': path, 'line': 0, 'rule': 'TAY: ' + name, 'old': a.strip()[:200],
           'new': b.strip()[:200] or '(xoá)', 'direction': dirn, 'source': 'tay'}
    if base.count(a) != 1:
        row.update(status='invalid', note=f'KHÔNG KHỚP đúng 1 lần ({base.count(a)})')
    else:
        try:
            t = base.replace(a, b, 1)
            io.open(P, 'w', encoding='utf-8', newline='').write(t.replace('\n', '\r\n') if crlf else t)
            rc, txt = sh(f'cargo test -p {crate} -- --test-threads=2')
            if rc != 0 and ('could not compile' in txt or 'error[E' in txt) and 'test result: FAILED' not in txt:
                row.update(status='invalid', note='không biên dịch: ' + txt[-300:].replace('\n', ' '))
            elif rc != 0:
                names = [ln.split(' ')[1] for ln in txt.splitlines() if ln.startswith('test ') and ln.endswith('FAILED')]
                row.update(status='killed', note=f'{crate}: ' + ', '.join(names[:4]) if names else f'{crate}: tiến trình test chết (vd tràn ngăn xếp)')
            else:
                row.update(status='survived', note='')
        finally:
            io.open(P, 'w', encoding='utf-8', newline='').write(orig)
    out.write(json.dumps(row, ensure_ascii=False) + '\n')
    out.flush()
out.close()
