#!/usr/bin/env python3
"""Đối chiếu MỌI mã chẩn đoán đã biết (crates/gt-diag/src/code.rs KNOWN) với quy tắc GRIT_PLAN §0.5/§7#4:
mỗi mã phải có `docs/errors/<mã>.md`, ≥3 mẫu fail (hoặc warn) và ≥3 mẫu pass được liệt kê trong tài liệu đó.

Khác `tools/check_expectations.py`: script của dự án chỉ kiểm danh sách `DOC_REQUIRED` do chính dự án chọn; ở đây kiểm TOÀN BỘ mã trong KNOWN.
Chỉ đọc. Mã thoát 0 nếu mọi mã đạt, 1 nếu có mã thiếu.
"""
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def main():
    src = open(os.path.join(ROOT, "crates/gt-diag/src/code.rs"), encoding="utf-8").read()
    m = re.search(r"const KNOWN: &\[&str\] = &\[(.*?)\];", src, re.S)
    codes = re.findall(r'"([ENS]\d{3,4})"', m.group(1))
    bad = []
    for c in sorted(codes):
        p = os.path.join(ROOT, "docs/errors", c + ".md")
        if not os.path.exists(p):
            bad.append((c, "không có docs/errors/%s.md" % c))
            continue
        t = open(p, encoding="utf-8").read()
        nf = len(re.findall(r"^- (?:fail|warn): ", t, re.M))
        npass = len(re.findall(r"^- pass: ", t, re.M))
        is_note = c.startswith("N")
        if (not is_note and nf < 3) or npass < 3:
            bad.append((c, f"tài liệu có {nf} mẫu fail/warn và {npass} mẫu pass (cần ≥3 và ≥3)"))
    print(f"SUMMARY: {len(codes)} mã trong KNOWN; {len(bad)} mã chưa đủ tài liệu/mẫu theo GRIT_PLAN §0.5")
    for c, why in bad:
        print(f"THIẾU {c}: {why}")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
