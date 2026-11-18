#!/usr/bin/env python3
"""GÓI 6 S1.a: liệt kê TỪNG nhánh chưa chạy (tệp, dòng, cột, hàm bao quanh thật, hướng, mã nguồn) từ cov.json(.gz) và tools/coverage_targets.txt.
  python tools/coverage_branches.py cov.json.gz > nhanh.tsv   (đếm thật; không ước lượng)
Hàm bao quanh = `fn` gần nhất phía trên dòng nhánh (trong tệp), không phải hàm trong coverage_targets (có thể là `*` cả tệp)."""
import json
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "tools"))
import coverage_report as cr  # noqa: E402

FN = re.compile(r"\s*(?:pub(?:\([a-z]+\))?\s+)?(?:const\s+)?(?:unsafe\s+)?fn\s+(\w+)")


def collect(cov_path):
    cov = cr.load(cov_path)
    bf = cr.branches_by_file(cov)
    targets = [l.strip() for l in open(os.path.join(ROOT, "tools", "coverage_targets.txt"), encoding="utf-8") if l.strip() and not l.startswith("#")]
    rows, seen, total = [], set(), 0
    cache = {}
    for t in targets:
        rel, fn = t.rsplit(":", 1)
        full = next((k for k in bf if k.replace(chr(92), "/").endswith("/" + rel)), None)
        if full is None:
            continue
        if rel not in cache:
            L = open(os.path.join(ROOT, rel), encoding="utf-8").read().split("\n")
            cache[rel] = (L, [(i + 1, FN.match(l).group(1)) for i, l in enumerate(L) if FN.match(l)])
        L, fl = cache[rel]
        sp = cr.span(L, fn)
        for (l1, c1, l2, c2), (a, b) in sorted(bf[full].items()):
            if sp[0] <= l1 <= sp[1]:
                enc = [n for ln, n in fl if ln <= l1]
                for lab, n in (("true", a), ("false", b)):
                    key = (rel, l1, c1, lab)
                    if key in seen:
                        continue
                    seen.add(key)
                    total += 1
                    if n == 0:
                        rows.append((rel, l1, c1, enc[-1] if enc else "?", lab, L[l1 - 1].strip()[:120]))
    return rows, total


if __name__ == "__main__":
    rows, total = collect(sys.argv[1])
    sys.stdout.reconfigure(encoding="utf-8")
    print(f"# tổng nhánh (true+false, mỗi (tệp,dòng,cột,hướng) một lần) = {total}; chưa chạy = {len(rows)}")
    for r in rows:
        print("%s:%d:%d\t%s\t%s\t%s" % (r[0], r[1], r[2], r[3], r[4], r[5]))
