#!/usr/bin/env python3
"""Đồng bộ `docs/fail_open_classification.tsv` với kết quả quét hiện tại (Seed 1.0 S7; xem docs/FAIL_OPEN_AUDIT.md mục 8).
Khoá khớp: (tệp, mẫu, dòng mã chuẩn hoá) — KHÔNG số dòng. Cập nhật số dòng của mục còn khớp; in mục MỚI (chưa phân loại) và mục đã biến mất.
Dùng: python3 tools/fail_open_sync.py [--write]   (không --write: chỉ in). Mã thoát 1 nếu còn mục chưa phân loại."""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import fail_open_scan as fo

TSV = os.path.join(fo.ROOT, "docs", "fail_open_classification.tsv")


def main(argv):
    hits = fo.scan_baseline()
    rows = []
    header = []
    with open(TSV, encoding="utf-8") as f:
        for ln in f:
            ln = ln.rstrip("\n")
            if ln.startswith("#"):
                header.append(ln)
            elif ln:
                rows.append(ln.split("\t"))
    by = {}
    for r in rows:
        by.setdefault((r[0], r[2], r[6]), []).append(r)
    out, new = [], []
    for h in hits:
        k = (h["file"], h["pattern"], h["code"])
        if by.get(k):
            r = by[k].pop(0)
            r[1] = str(h["line"])
            out.append(r)
        else:
            new.append(h)
    gone = [r for rs in by.values() for r in rs]
    for h in new:
        print(f"MỚI (chưa phân loại): {h['file']}:{h['line']} | {h['pattern']} | {h['code']}")
    for r in gone:
        print(f"ĐÃ BIẾN MẤT: {r[0]}:{r[1]} | {r[2]} | {r[3]}")
    if "--write" in argv:
        out.sort(key=lambda r: (r[0], int(r[1])))
        with open(TSV, "w", encoding="utf-8", newline="\n") as f:
            f.write("\n".join(header) + "\n")
            for r in out:
                f.write("\t".join(r) + "\n")
        print(f"đã ghi {len(out)} dòng -> {TSV}")
    return 1 if new else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
