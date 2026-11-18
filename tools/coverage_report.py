#!/usr/bin/env python3
"""GÓI 5 S3: từ cov.json(.gz) do `cargo llvm-cov report --branch --json` tạo bảng NHÁNH cho các HÀM QUYẾT ĐỊNH và liệt kê nhánh chưa chạy.
  python tools/coverage_report.py cov.json.gz [--md docs/DECISION_COVERAGE_DATA.md] [--targets tools/coverage_targets.txt]
targets: mỗi dòng `đường/dẫn/tệp.rs:tên_hàm` (hàm tìm theo `fn tên` trong tệp; khoảng dòng = đến dấu `}` cân bằng). Nhánh = một điều kiện/đích rẽ của llvm-cov (true/false); 'chưa chạy' = số lần true hoặc false bằng 0
(cộng gộp qua mọi bản đơn hình hoá của hàm). Đây là độ phủ nhánh của TRÌNH BIÊN DỊCH (rustc nightly), KHÔNG phải của chương trình Grit."""
import gzip
import json
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def load(p):
    op = gzip.open if p.endswith(".gz") else open
    with op(p, "rt", encoding="utf-8") as f:
        return json.load(f)


def span(src_lines, name):
    if name == "*":
        return 1, len(src_lines)
    for i, l in enumerate(src_lines):
        if re.search(r"\bfn " + re.escape(name) + r"\b", l):
            depth, seen = 0, False
            for j in range(i, len(src_lines)):
                for ch in src_lines[j]:
                    if ch == "{":
                        depth += 1
                        seen = True
                    elif ch == "}":
                        depth -= 1
                if seen and depth == 0:
                    return i + 1, j + 1
    return None


def branches_by_file(data):
    out = {}
    for fn in data["data"][0]["functions"]:
        fns = fn.get("filenames", [])
        for b in fn.get("branches", []):
            l1, c1, l2, c2, t, f, fid = b[:7]
            if fid >= len(fns):
                continue
            key = (l1, c1, l2, c2)
            d = out.setdefault(fns[fid], {})
            cur = d.get(key, [0, 0])
            d[key] = [cur[0] + t, cur[1] + f]
    return out


def regions_by_file(data):
    out = {}
    for fn in data["data"][0]["functions"]:
        fns = fn.get("filenames", [])
        for r in fn.get("regions", []):
            l1, c1, l2, c2, cnt, fid, efid, kind = r[:8]
            if kind != 0 or fid >= len(fns) or fid != efid:
                continue
            d = out.setdefault(fns[fid], {})
            d[(l1, c1, l2, c2)] = d.get((l1, c1, l2, c2), 0) + cnt
    return out


def main():
    a = [x for x in sys.argv[1:] if not x.startswith("--")]
    cov = load(a[0])
    tpath = sys.argv[sys.argv.index("--targets") + 1] if "--targets" in sys.argv else os.path.join(ROOT, "tools", "coverage_targets.txt")
    md = sys.argv[sys.argv.index("--md") + 1] if "--md" in sys.argv else None
    bf = branches_by_file(cov)
    rf = regions_by_file(cov)
    unc_r = []
    rows, unc = [], []
    for line in open(tpath, encoding="utf-8"):
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        rel, fn = line.rsplit(":", 1)
        full = next((k for k in bf if k.replace("\\", "/").endswith("/" + rel)), None)
        if full is None:
            rows.append((rel, fn, None, None, None, "KHÔNG có dữ liệu nhánh cho tệp"))
            continue
        src = open(os.path.join(ROOT, rel), encoding="utf-8").read().split("\n")
        sp = span(src, fn)
        if sp is None:
            rows.append((rel, fn, None, None, None, "không tìm thấy hàm trong nguồn"))
            continue
        tot = cov_n = 0
        for (l1, c1, l2, c2), (t, f) in sorted(bf[full].items()):
            if sp[0] <= l1 <= sp[1]:
                for lab, n in (("true", t), ("false", f)):
                    tot += 1
                    if n > 0:
                        cov_n += 1
                    else:
                        unc.append((rel, fn, l1, c1, lab + " `" + src[l1 - 1].strip()[:90] + "`"))
        nreg = ureg = 0
        for (l1, c1, l2, c2), cnt in sorted(rf.get(full, {}).items()):
            if sp[0] <= l1 <= sp[1]:
                nreg += 1
                if cnt == 0:
                    ureg += 1
                    unc_r.append((rel, fn, l1, c1, l2, c2, src[l1 - 1].strip()[:90]))
        rows.append((rel, fn, tot, cov_n, sp, "vùng mã %d, chưa chạy %d" % (nreg, ureg)))
    out = ["| tệp | hàm | dòng | nhánh (true+false) | đã chạy | chưa chạy | ghi chú |", "|---|---|---|---:|---:|---:|---|"]
    for rel, fn, tot, c, sp, note in rows:
        out.append(f"| {rel} | {fn} | {'%d-%d' % sp if sp else '-'} | {tot if tot is not None else '-'} | {c if c is not None else '-'} | {(tot - c) if tot is not None else '-'} | {note} |")
    out += ["", "## Nhánh chưa chạy (tệp, hàm, dòng:cột, hướng)", ""]
    out += [f"- {r}::{f} {l}:{c} **{lab}**" for r, f, l, c, lab in unc]
    out += ["", "## Vùng mã (region, gồm nhánh match) chưa chạy: tệp, hàm, dòng:cột-dòng:cột, mã", ""]
    out += [f"- {r}::{f} {a}:{b}-{c}:{d} `{t}`" for r, f, a, b, c, d, t in unc_r]
    text = "\n".join(out) + "\n"
    if md:
        open(os.path.join(ROOT, md), "w", encoding="utf-8").write(text)
    else:
        sys.stdout.reconfigure(encoding="utf-8")
        print(text)


if __name__ == "__main__":
    main()
