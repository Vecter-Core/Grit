#!/usr/bin/env python3
"""GÓI 8 S0e: sinh docs/RUN_REPORTS_INDEX.md từ docs/RUN_REPORTS.md (không sửa nội dung gốc, KHÔNG sắp xếp lại).
Bảng: tiêu đề mục (dòng `## …`), ngày ghi trong tiêu đề (YYYY-MM-DD đầu tiên, hoặc "—"), dòng bắt đầu, số dòng của mục.
Dùng: python tools/reports_index.py [--src docs/RUN_REPORTS.md] [--out docs/RUN_REPORTS_INDEX.md] [--check]
--check: thoát 1 nếu tệp chỉ mục hiện có khác với nội dung sinh ra (cho cổng/test). Tệp gốc không theo thứ tự thời gian: chỉ liệt kê, không đổi.
"""
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DATE = re.compile(r"\d{4}-\d{2}-\d{2}")


def sections(text):
    """Trả danh sách (tiêu_đề, ngày|None, dòng_bắt_đầu, số_dòng) cho mọi mục `## …` (bỏ qua dòng trong khối mã ```)."""
    lines = text.split("\n")
    heads = []
    in_code = False
    for i, ln in enumerate(lines, 1):
        if ln.lstrip().startswith("```"):
            in_code = not in_code
            continue
        if not in_code and ln.startswith("## "):
            heads.append((i, ln[3:].strip()))
    out = []
    for k, (start, title) in enumerate(heads):
        end = heads[k + 1][0] - 1 if k + 1 < len(heads) else len(lines)
        m = DATE.search(title)
        out.append((title, m.group(0) if m else None, start, end - start + 1))
    return out


def render(src_text, src_name):
    rows = sections(src_text)
    out = ["# RUN_REPORTS_INDEX — mục lục tự sinh (đừng sửa tay)", "",
           "Sinh bởi `tools/reports_index.py` từ `%s`. Mục KHÔNG theo thứ tự thời gian trong tệp gốc (hai dòng làm việc đan xen); bảng này chỉ liệt kê, không sắp xếp lại." % src_name,
           "", "| # | tiêu đề mục | ngày trong tiêu đề | dòng bắt đầu | số dòng |", "|---:|---|---|---:|---:|"]
    for n, (title, date, start, count) in enumerate(rows, 1):
        t = title.replace("|", "\\|")
        out.append("| %d | %s | %s | %d | %d |" % (n, t, date or "—", start, count))
    out.append("")
    out.append("Tổng: %d mục." % len(rows))
    return "\n".join(out) + "\n"


def main(argv):
    src = os.path.join(ROOT, "docs", "RUN_REPORTS.md")
    out = os.path.join(ROOT, "docs", "RUN_REPORTS_INDEX.md")
    if "--src" in argv:
        src = argv[argv.index("--src") + 1]
    if "--out" in argv:
        out = argv[argv.index("--out") + 1]
    with open(src, encoding="utf-8") as f:
        text = f.read()
    rendered = render(text, os.path.basename(src))
    if "--check" in argv:
        if not os.path.isfile(out):
            print("RUN_REPORTS_INDEX: thiếu", out)
            return 1
        with open(out, encoding="utf-8") as f:
            cur = f.read()
        if cur.replace("\r\n", "\n") != rendered:
            print("RUN_REPORTS_INDEX: lệch — chạy lại tools/reports_index.py")
            return 1
        print("RUN_REPORTS_INDEX_OK")
        return 0
    with open(out, "w", encoding="utf-8", newline="\n") as f:
        f.write(rendered)
    print("ghi", out, "—", rendered.count("\n| ") - 1, "mục")
    return 0


if __name__ == "__main__":
    sys.stdout.reconfigure(encoding="utf-8")
    sys.exit(main(sys.argv[1:]))
