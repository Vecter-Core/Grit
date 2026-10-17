"""Đột biến TAY cho gt-guard (luật G1 và sửa FB-9) — những chỗ `tools/mut.py` không sinh được (arm `match`, `matches!`, `.replace`, vòng lặp danh sách dùng).
Mỗi đột biến: sửa MỘT chỗ trong crates/gt-guard/src/lib.rs, chạy `cargo test -p gt-guard`; nếu sống sót thì chạy thêm `release_static_mutex` của gt-driver (bắt đột biến làm
Guard NGHIÊM hơn, hỏng chương trình thật); luôn hoàn nguyên (finally). 1 worker. Kết quả ghi nối vào target/tmp/mut_guard_manual.jsonl (định dạng như docs/MUTATION_GT_GUARD.jsonl).
Hướng: `accept` = đột biến làm Guard CHẤP NHẬN thêm IR (nguy hiểm nếu sống sót), `reject` = nghiêm hơn."""
import hashlib
import io
import json
import os
import subprocess

R = 'D:/Grit/'
P = R + 'crates/gt-guard/src/lib.rs'
OUT = R + 'target/tmp/mut_guard_manual.jsonl'
MUTS = [
    # (hướng, tên, cũ, mới)
    ('reject', 'inst_of: bỏ nhánh IntValue', '        AnyValueEnum::IntValue(x) => x.as_instruction(),\n', ''),
    ('reject', 'inst_of: bỏ nhánh PointerValue', '        AnyValueEnum::PointerValue(x) => x.as_instruction(),\n', ''),
    ('reject', 'inst_of: bỏ nhánh InstructionValue', '        AnyValueEnum::InstructionValue(i) => Some(i),\n', ''),
    ('accept', 'chỉ xét lần dùng ĐẦU TIÊN', '            u = use_.get_next_use();\n', '            u = None;\n'),
    ('accept', 'không kiểm đối số (0..0)', '(0..n.saturating_sub(1)).all(', '(0..0).all('),
    ('reject', 'kiểm cả toán hạng đích (0..n)', '(0..n.saturating_sub(1)).all(', '(0..n).all('),
    ('accept', 'bỏ điều kiện đích gọi == tên', 'callee_name(&i).as_deref() == Some(name.as_str())', 'true'),
    ('accept', 'luật 3c chỉ Invoke', 'InstructionOpcode::Invoke | InstructionOpcode::CallBr', 'InstructionOpcode::Invoke'),
    ('accept', 'luật 3c chỉ CallBr', 'InstructionOpcode::Invoke | InstructionOpcode::CallBr', 'InstructionOpcode::CallBr'),
    ('accept', 'không rà địa chỉ runtime (bỏ gọi)', 'runtime_address_violations(m)', 'Vec::<String>::new()'),
    ('accept', 'chỉ rà hàm `gritrt_` đầu bảng (starts_with → ==)', 'name.starts_with("gritrt_")', '(name == "gritrt_abort")'),
    ('reject', 'FB-9: bỏ miễn `getelementptr inbounds nuw`', '.replace("getelementptr inbounds nuw", "getelementptr")', ''),
    ('reject', 'FB-9: bỏ miễn `getelementptr nuw`', '.replace("getelementptr nuw", "getelementptr")', ''),
    ('accept', 'FB-9: miễn quá rộng (mọi " nuw")', '.replace("getelementptr inbounds nuw", "getelementptr")', '.replace(" nuw", "")'),
    ('accept', 'FB-9: miễn quá rộng (mọi " nsw")', '.replace("getelementptr nuw", "getelementptr")', '.replace(" nsw", "")'),
]
env = dict(os.environ, CARGO_BUILD_JOBS='2', GRIT_LLVM_DIR='D:/Tools/llvm-20.1.8', GRIT_RT_LIB=R + 'runtime/gt-rt/target/release/gt_rt.lib')
env['PATH'] = 'D:/Tools/llvm-20.1.8/bin;' + env['PATH']


def sh(cmd):
    for _ in range(4):
        r = subprocess.run(cmd, shell=True, cwd=R, env=env, capture_output=True, text=True, encoding='utf-8', errors='replace', stdin=subprocess.DEVNULL)
        t = r.stdout + r.stderr
        if 'LNK1104' not in t:
            break
    return r.returncode, t


def main():
    only = os.environ.get('MUT_ONLY')
    out = io.open(OUT, 'a', encoding='utf-8')
    orig = io.open(P, encoding='utf-8', newline='').read()
    base = orig.replace('\r\n', '\n')
    crlf = '\r\n' in orig
    for i, (dirn, name, a, b) in enumerate(MUTS):
        if only and str(i) not in only.split(','):
            continue
        row = {'id': hashlib.sha1(f'{P}:{name}'.encode()).hexdigest()[:12], 'file': 'crates/gt-guard/src/lib.rs', 'line': 0, 'rule': 'TAY: ' + name,
               'old': a.strip(), 'new': b.strip() or '(xoá)', 'direction': dirn}
        if base.count(a) != 1:
            row.update(status='invalid', note=f'KHÔNG KHỚP đúng 1 lần ({base.count(a)})')
        else:
            try:
                t = base.replace(a, b, 1)
                io.open(P, 'w', encoding='utf-8', newline='').write(t.replace('\n', '\r\n') if crlf else t)
                rc, txt = sh('cargo test -p gt-guard -- --test-threads=2')
                if rc != 0 and ('could not compile' in txt or 'error[E' in txt):
                    row.update(status='invalid', note='không biên dịch: ' + txt[-200:].replace('\n', ' '))
                elif rc != 0:
                    names = [ln.split(' ')[1] for ln in txt.splitlines() if ln.startswith('test ') and ln.endswith('FAILED')]
                    row.update(status='killed', note='gt-guard: ' + ', '.join(names[:4]))
                else:
                    rc2, txt2 = sh('cargo test -p gt-driver --test release_static_mutex -- --test-threads=2')
                    if rc2 != 0:
                        row.update(status='killed', note='gt-driver release_static_mutex')
                    else:
                        row.update(status='survived', note='')
            finally:
                io.open(P, 'w', encoding='utf-8', newline='').write(orig)
        out.write(json.dumps(row, ensure_ascii=False) + '\n')
        out.flush()
    out.close()
    sh('cargo test -p gt-guard --no-run')


main()
