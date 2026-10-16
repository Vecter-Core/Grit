"""Đột biến lô nhỏ, 1 worker, cho kiểm động Arena của gt-rt (`arena_verify`, `finish_alloc`, `reset`): sửa runtime/gt-rt/src/lib.rs, dựng lại gt_rt.lib (release),
chạy gt-rt unit test + gt-driver/tests/arena_dynamic.rs. Luôn hoàn nguyên (finally) và dựng lại thư viện gốc. Kết quả: target/tmp/mut_arena.txt."""
import io
import os
import subprocess

R = 'D:/Grit/'
P = R + 'runtime/gt-rt/src/lib.rs'
MUTS = [
    ('bỏ kiểm chồng với vùng đứng TRƯỚC', '        && *e > p\n', '        && false\n'),
    ('bỏ kiểm chồng với vùng đứng SAU', '        && *s < end\n', '        && false\n'),
    ('bỏ kiểm căn lề', '    if !p.is_multiple_of(align) {', '    if false {'),
    ('bỏ kiểm nằm trong chunk', 'end <= c.0 + c.1', 'true'),
    ('không ghi nhận vùng đã cấp', '    st.live.insert(p, end);\n', '    let _ = (p, end);\n'),
    ('reset không xoá thế hệ', '    st.live.clear(); // thế hệ mới\n', ''),
    ('kiểm dùng < thay <= (biên chồng)', '        && *s < end\n', '        && *s + 1 < end\n'),
    ('chèn lỗi overlap bị bỏ qua', 'Some("arena_overlap") if size > 0 => {', 'Some("arena_overlap") if size > 1_000_000_000 => {'),
]
env = dict(os.environ, CARGO_BUILD_JOBS='2', GRIT_LLVM_DIR='D:/Tools/llvm-20.1.8', GRIT_RT_LIB=R + 'runtime/gt-rt/target/release/gt_rt.lib')
env['PATH'] = 'D:/Tools/llvm-20.1.8/bin;' + env['PATH']
only = os.environ.get('MUT_ONLY')
out = io.open(R + 'target/tmp/mut_arena.txt', 'a', encoding='utf-8')


def sh(cmd, cwd):
    for _ in range(4):
        r = subprocess.run(cmd, shell=True, cwd=cwd, env=env, capture_output=True, text=True, encoding='utf-8', errors='replace')
        t = r.stdout + r.stderr
        if 'LNK1104' not in t:
            break
    return r.returncode, t


for i, (name, a, b) in enumerate(MUTS):
    if only and str(i) not in only.split(','):
        continue
    orig = io.open(P, encoding='utf-8', newline='').read()
    base = orig.replace('\r\n', '\n')
    if a not in base:
        out.write(f'{i}: KHÔNG KHỚP | {name}\n'); out.flush(); continue
    try:
        io.open(P, 'w', encoding='utf-8', newline='').write(base.replace(a, b, 1))
        rc, t = sh('cargo build --release', R + 'runtime/gt-rt')
        if rc != 0:
            st = 'KHÔNG BIÊN DỊCH'
        else:
            rc1, t1 = sh('cargo test', R + 'runtime/gt-rt')
            rc2, t2 = sh('cargo test -p gt-driver --test arena_dynamic -- --test-threads=2', R)
            st = 'SỐNG SÓT' if rc1 == 0 and rc2 == 0 else ('BỊ BẮT bởi ' + ('gt-rt unit' if rc1 != 0 else '') + (' arena_dynamic' if rc2 != 0 else ''))
    finally:
        io.open(P, 'w', encoding='utf-8', newline='').write(orig)
        sh('cargo build --release', R + 'runtime/gt-rt')
    out.write(f'{i}: {st} | {name}\n'); out.flush()
out.write('XONG\n'); out.close()
