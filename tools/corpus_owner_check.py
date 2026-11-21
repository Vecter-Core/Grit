#!/usr/bin/env python3
"""GÓI 14 A2 — kiểm tra CƠ HỌC "mẫu corpus nào do crate nào chạy" (lỗi lặp lại của GÓI 7 và GÓI 13: tệp `tests/fail/E0203_private_field_in_pattern.gt` là lỗi của
GIAI ĐOẠN KIỂU (gt-types) nhưng corpus test của gt-resolve nhận MỌI mẫu mang tiền tố E0203 và đòi resolver phát đúng mã đó ⇒ cổng đỏ muộn, ở bước test nặng).

Quy tắc (đọc từ mã nguồn, không dựa trí nhớ):
 1. CLAIMS: mỗi `crates/<crate>/tests/*.rs` khai báo mã nó chọn — chuỗi `"Exxxx"` trong mảng `const OWNED*/…CODES*`, hoặc `starts_with("Exxxx_")` /
    `starts_with(&format!("{c}_"))` kèm OWNED. Mỗi mẫu `tests/{fail,pass,warn}/<mã>_*.gt` mang mã đó bị crate "chủ" chạy.
 2. PHÁT: crate nào có `E…` (không phải dòng chú thích) trong `src/`. Tập phát của X gồm X và các crate nó phụ thuộc (`[dependencies]` gt-*), vì X chạy các giai đoạn đó.
 3. RULE-1: crate chủ X của mã C mà C ∉ phát(X) ⇒ ĐỎ (X không thể phát mã đó).
 4. RULE-2 (đa giai đoạn): mã C do crate chủ X nhận mà một crate chủ KHÁC Y phụ thuộc X (Y ở hạ nguồn) cũng phát C ⇒ một mẫu mang mã C có thể là lỗi của giai đoạn Y,
    corpus của X sẽ đỏ. Mỗi mẫu như vậy PHẢI khai `tests/corpus_stage.tsv` (`<đường dẫn mẫu>\\t<crate>`) với crate == X. Chưa khai, hoặc khai crate khác ⇒ ĐỎ, nêu tên mẫu và crate.
    Mẫu lỗi của giai đoạn hạ nguồn KHÔNG vào corpus `fail/` mà là test cấu trúc (vd. crates/gt-types/tests/struct_pat_privacy.rs).
Chạy: python tools/corpus_owner_check.py [thư_mục_gốc]. Thoát 0 xanh, 1 đỏ. `--list` in bảng crate chủ / phát (để dán vào tài liệu)."""
import os
import re
import sys

CODE = re.compile(r"\bE\d{4}\b")


def cargo_deps(root, crate):
    p = os.path.join(root, "crates", crate, "Cargo.toml")
    deps, sec = set(), ""
    with open(p, encoding="utf-8") as f:
        for line in f:
            s = line.strip()
            if s.startswith("["):
                sec = s
                continue
            if sec == "[dependencies]":
                m = re.match(r"(gt-[a-z0-9-]+)\b", s)
                if m:
                    deps.add(m.group(1))
    return deps


def closure(root, crate, cache):
    if crate in cache:
        return cache[crate]
    cache[crate] = set()  # chống vòng
    out = {crate}
    for d in cargo_deps(root, crate):
        if os.path.isdir(os.path.join(root, "crates", d)):
            out |= closure(root, d, cache)
    cache[crate] = out
    return out


def emits(root, crate):
    codes = set()
    src = os.path.join(root, "crates", crate, "src")
    for dp, _, fs in os.walk(src):
        for fn in fs:
            if not fn.endswith(".rs"):
                continue
            with open(os.path.join(dp, fn), encoding="utf-8", errors="replace") as f:
                for line in f:
                    if line.lstrip().startswith("//"):
                        continue
                    line = line.split("//", 1)[0]
                    codes.update(CODE.findall(line))
    return codes


OWNED_ARR = re.compile(r"const\s+(?:OWNED\w*|\w*_CODES|OWNED)\s*:[^=]*=\s*&\[(.*?)\]\s*;", re.S)
FULL_PREFIX = re.compile(r'starts_with\("(E\d{4})_')


def claims(root):
    """crate ⇒ tập mã nó chọn (từ test corpus của chính nó)."""
    res = {}
    cd = os.path.join(root, "crates")
    for crate in sorted(os.listdir(cd)):
        td = os.path.join(cd, crate, "tests")
        if not os.path.isdir(td):
            continue
        got = set()
        for fn in sorted(os.listdir(td)):
            if not fn.endswith(".rs"):
                continue
            with open(os.path.join(td, fn), encoding="utf-8", errors="replace") as f:
                text = f.read()
            # chỉ tệp đọc corpus `tests/` chung (không phải test cấu trúc dùng chuỗi mã làm đầu vào)
            if "fail" not in text or "read_dir" not in text:
                continue
            for m in OWNED_ARR.finditer(text):
                got.update(CODE.findall(m.group(1)))
            got.update(FULL_PREFIX.findall(text))
        if got:
            res[crate] = got
    return res


def samples(root):
    out = []
    for kind in ("fail", "pass", "warn"):
        d = os.path.join(root, "tests", kind)
        if not os.path.isdir(d):
            continue
        for fn in sorted(os.listdir(d)):
            m = re.match(r"([ESN]\d{4})_", fn)
            if m and (fn.endswith(".gt") or os.path.isdir(os.path.join(d, fn))):
                out.append((kind, fn, m.group(1)))
    return out


def stage_map(root):
    p = os.path.join(root, "tests", "corpus_stage.tsv")
    res = {}
    if os.path.exists(p):
        with open(p, encoding="utf-8") as f:
            for line in f:
                line = line.rstrip("\n")
                if not line or line.startswith("#"):
                    continue
                a, _, b = line.partition("\t")
                res[a.strip()] = b.strip()
    return res


def check(root):
    cl = claims(root)
    cache = {}
    emit = {c: emits(root, c) for c in cl}
    reach = {c: closure(root, c, cache) for c in cl}
    can_emit = {c: set().union(*[emits(root, d) for d in reach[c] if os.path.isdir(os.path.join(root, "crates", d, "src"))]) for c in cl}
    owner = {}
    for c, codes in cl.items():
        for k in codes:
            owner.setdefault(k, []).append(c)
    stages = stage_map(root)
    bad = []
    seen_codes = {code for (_, _, code) in samples(root)}
    for k, crates in sorted(owner.items()):
        for x in crates:
            if k in seen_codes and k not in can_emit[x]:
                bad.append(f"RULE-1: corpus của {x} chọn mã {k} nhưng {x} (và các crate nó phụ thuộc) không phát mã đó")
    ambiguous = {}
    for k, crates in owner.items():
        for x in crates:
            downstream = [y for y in cl if y != x and x in reach[y] and k in emit[y] and y not in crates]
            if downstream:
                ambiguous[k] = (x, downstream)
    for kind, fn, code in samples(root):
        if code not in ambiguous:
            continue
        x, downstream = ambiguous[code]
        rel = f"tests/{kind}/{fn}"
        got = stages.get(rel)
        if got is None:
            bad.append(f"RULE-2: {rel}: mã {code} được corpus của {x} chạy NHƯNG {', '.join(downstream)} cũng phát {code} ở hạ nguồn; khai `{rel}\\t{x}` vào tests/corpus_stage.tsv "
                       f"nếu đây là lỗi của {x}, còn lỗi của {downstream[0]} thì KHÔNG đặt vào corpus mà viết test cấu trúc ở {downstream[0]}/tests/")
        elif got != x:
            bad.append(f"RULE-2: {rel}: khai giai đoạn {got} nhưng corpus chạy mẫu mã {code} là của {x} (crate {got} không chạy mẫu này) — chuyển thành test cấu trúc của {got}")
    for rel, got in stages.items():
        if not os.path.exists(os.path.join(root, rel)):
            bad.append(f"corpus_stage.tsv: tệp không tồn tại: {rel}")
    return bad, cl, can_emit, ambiguous


def write_stage(root, ambiguous):
    nl = chr(10)
    rows = [f"tests/{kind}/{fn}" + chr(9) + ambiguous[code][0] for kind, fn, code in samples(root) if code in ambiguous]
    head = ["# GÓI 14 A2: mẫu của mã ĐA GIAI ĐOẠN (xem tools/corpus_owner_check.py RULE-2) ⇒ crate có corpus test chạy mẫu đó. Một dòng: <đường dẫn> TAB <crate>.",
            "# Lỗi của giai đoạn hạ nguồn KHÔNG khai ở đây: viết test cấu trúc trong crate đó (vd. crates/gt-types/tests/struct_pat_privacy.rs)."]
    with open(os.path.join(root, "tests", "corpus_stage.tsv"), "w", encoding="utf-8", newline=nl) as f:
        f.write(nl.join(head + rows) + nl)
    print(f"đã ghi {len(rows)} dòng")


def main(argv):
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8")
    args = [a for a in argv[1:] if not a.startswith("--")]
    root = os.path.abspath(args[0]) if args else os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    bad, cl, can_emit, ambiguous = check(root)
    if "--list" in argv:
        for c in sorted(cl):
            print(f"{c}: chọn {' '.join(sorted(cl[c]))}")
        for k, (x, d) in sorted(ambiguous.items()):
            print(f"đa giai đoạn {k}: chủ {x}, hạ nguồn phát: {', '.join(d)}")
    if "--bless" in argv:  # chỉ dùng MỘT LẦN khi lập sidecar: ghi mọi mẫu đa giai đoạn hiện có là của crate chủ (hợp lệ vì corpus test của crate chủ đang XANH trên cây này)
        write_stage(root, ambiguous)
        return 0
    for b in bad[:20]:
        print("CORPUS_OWNER_RED: " + b[:400])
    if len(bad) > 20:
        print(f"CORPUS_OWNER_RED: ... và {len(bad) - 20} lỗi nữa")
    n = len(samples(root))
    print(f"CORPUS_OWNER {'RED' if bad else 'OK'}: {len(cl)} crate chủ, {n} mẫu, {len(ambiguous)} mã đa giai đoạn")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
