import io, subprocess, os
R = 'D:/Grit/'
S = 'lib/std/src/'
MUTS = [
 ('std_ord.gt', 'None ↦ ⌥ o { Some(_) ↦ Ordering::Less, None ↦ Ordering::Equal }', 'None ↦ ⌥ o { Some(_) ↦ Ordering::Greater, None ↦ Ordering::Equal }', 'Option cmp: None vs Some đảo'),
 ('std_ord.gt', 'Some(b) ↦ a.cmp(b), None ↦ Ordering::Greater }', 'Some(b) ↦ a.cmp(b), None ↦ Ordering::Less }', 'Option cmp: Some vs None đảo'),
 ('std_ord.gt', 'Err(_) ↦ Ordering::Less }', 'Err(_) ↦ Ordering::Greater }', 'Result cmp: Ok vs Err đảo'),
 ('std_ord.gt', 'Ordering::Equal ↦ b.cmp(d), r ↦ r', 'Ordering::Equal ↦ Ordering::Equal, r ↦ r', 'tuple2 bỏ thành phần thứ hai'),
 ('std_ord.gt', 'Ordering::Equal ↦ c.cmp(f), r ↦ r }', 'Ordering::Equal ↦ Ordering::Equal, r ↦ r }', 'tuple3 bỏ thành phần thứ ba'),
 ('std_ord.gt', 'Ordering::Equal ↦ na.cmp(&nb),', 'Ordering::Equal ↦ Ordering::Equal,', 'lát: bỏ so độ dài'),
 ('std_ord.gt', 'm ≔ ⎇ na < nb { na } ⇝ { nb };', 'm ≔ ⎇ na < nb { nb } ⇝ { na };', 'lát: max thay min độ dài'),
 ('std_ord.gt', 'Ordering::Equal ↦ { i ← i + 1; },', 'Ordering::Equal ↦ { i ← m; },', 'lát: dừng sau phần tử đầu'),
 ('std_ord.gt', 'Ordering::Greater ↦ 2u8 }', 'Ordering::Greater ↦ 1u8 }', 'ord_rank: Greater = Equal'),
 ('std_ord.gt', 'self.as_bytes().cmp(o.as_bytes())', 'o.as_bytes().cmp(self.as_bytes())', 'Text: đảo vế'),
 ('std.gt', 'Ordering::Equal ↦ o, Ordering::Less ↦ Ordering::Less', 'Ordering::Equal ↦ Ordering::Equal, Ordering::Less ↦ Ordering::Less', 'Ordering::then bỏ vế sau'),
 ('std.gt', 'Ordering::Less ↦ Ordering::Greater, Ordering::Equal ↦ Ordering::Equal', 'Ordering::Less ↦ Ordering::Less, Ordering::Equal ↦ Ordering::Equal', 'Ordering::reverse: Less không đảo'),
 ('std.gt', '⌥ b.cmp(&a) { Ordering::Less ↦ b, _ ↦ a }', '⌥ b.cmp(&a) { Ordering::Less ↦ a, _ ↦ b }', 'min: thành max'),
 ('std.gt', 'Some(v) ↦ Some(v), None ↦ o }', 'Some(v) ↦ Some(v), None ↦ None }', 'Option::or: bỏ vế phải'),
 ('std.gt', 'ƒ is_ok(&self) → bool ! ∅ { ⌥ self { Ok(_) ↦ true, Err(_) ↦ false } }', 'ƒ is_ok(&self) → bool ! ∅ { ⌥ self { Ok(_) ↦ false, Err(_) ↦ true } }', 'Result::is_ok đảo'),
 ('std.gt', 'Some(v) ↦ Some(f(v)), None ↦ None } }\n  ƒ and_then<U>', 'Some(v) ↦ Some(f(v)), None ↦ None } }\n  ƒ and_then<U>', '(bỏ qua: không đổi)'),
 ('std_ord_int.gt', '⊐ Ord for u8 {\n  ƒ cmp(&self, o: &Self) → Ordering ! ∅ {\n    ⎇ *self < *o { Ordering::Less } ⇝ { ⎇ *self > *o', '⊐ Ord for u8 {\n  ƒ cmp(&self, o: &Self) → Ordering ! ∅ {\n    ⎇ *self < *o { Ordering::Less } ⇝ { ⎇ *self >= *o', 'Ord u8: Equal thành Greater'),
 ('std_ord_int.gt', '⊐ Ord for char {\n  ƒ cmp(&self, o: &Self) → Ordering ! ∅ {\n    ⎇ *self < *o', '⊐ Ord for char {\n  ƒ cmp(&self, o: &Self) → Ordering ! ∅ {\n    ⎇ *self > *o', 'Ord char: đảo'),
 ('std_ord_int.gt', '⎇ *self { ⎇ *o { Ordering::Equal } ⇝ { Ordering::Greater } }', '⎇ *self { ⎇ *o { Ordering::Less } ⇝ { Ordering::Greater } }', 'Ord bool: true≠true'),
 ('std_ord_int.gt', '⊐ Ord for i8 {\n  ƒ cmp(&self, o: &Self) → Ordering ! ∅ {\n    ⎇ *self < *o { Ordering::Less } ⇝ { ⎇ *self > *o { Ordering::Greater } ⇝ { Ordering::Equal } }', '⊐ Ord for i8 {\n  ƒ cmp(&self, o: &Self) → Ordering ! ∅ {\n    ⎇ *self < *o { Ordering::Less } ⇝ { ⎇ *self > *o { Ordering::Equal } ⇝ { Ordering::Equal } }', 'Ord i8: Greater thành Equal'),
]
env = dict(os.environ, CARGO_BUILD_JOBS='2', STD_RANDOM='4', GRIT_LLVM_DIR='D:/Tools/llvm-20.1.8', GRIT_RT_LIB=R + 'runtime/gt-rt/target/release/gt_rt.lib')
env['PATH'] = 'D:/Tools/llvm-20.1.8/bin;' + env['PATH']
ONLY = os.environ.get('MUT_ONLY')
out = io.open(R + 'target/tmp/mut_std.txt', 'a', encoding='utf-8')
for i, (f, a, b, name) in enumerate(MUTS):
    if ONLY and str(i) not in ONLY.split(','):
        continue
    if a == b:
        continue
    p = R + S + f
    orig = io.open(p, encoding='utf-8', newline='').read()
    if a not in orig:
        out.write(f'{i}: KHÔNG KHỚP MẪU | {name}\n'); out.flush(); continue
    try:
        io.open(p, 'w', encoding='utf-8', newline='').write(orig.replace(a, b, 1))
        for _ in range(4):
            r = subprocess.run('cargo test -p gt-driver --test std_lib every_std -- --nocapture --test-threads=2', shell=True, cwd=R, env=env, capture_output=True, text=True, encoding='utf-8', errors='replace')
            t = r.stdout + r.stderr
            if 'LNK1104' not in t:
                break
        if 'could not compile' in t:
            st = 'KHÔNG BIÊN DỊCH'
        elif r.returncode == 0:
            st = 'SỐNG SÓT'
        elif ('mô hình' in t and '≠' in t) or 'cổng ĐỎ' in t or 'bất thường' in t or 'LỆCH' in t:
            st = 'BỊ BẮT'
        else:
            st = 'KHÁC (đỏ nhưng không rõ lý do)'
    finally:
        io.open(p, 'w', encoding='utf-8', newline='').write(orig)
    out.write(f'{i}: {st} | {f} | {name}\n'); out.flush()
out.write('XONG\n'); out.close()
