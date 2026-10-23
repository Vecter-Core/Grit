#!/usr/bin/env python3
"""Quét mẫu "im lặng chấp nhận" (fail-open) ở các crate QUYẾT ĐỊNH phán quyết (Seed 1.0 S7; phương pháp của docs/FAIL_OPEN_AUDIT.md).
Mẫu: unwrap_or(, unwrap_or_default, .ok()?, let _ =, `_ =>` (nhánh mặc định), map_or(true|false, is_none_or, is_some_and, `continue`/`return` sớm sau `None`.
Đầu ra: danh sách `tệp:dòng | mẫu | dòng mã`; với --baseline <tệp.json> chỉ in mục MỚI so với baseline (khoá = tệp + mẫu + dòng mã đã chuẩn hoá, KHÔNG số dòng);
--write-baseline ghi baseline. Mỗi mục mới phải được ĐỌC và phân loại (AN TOÀN/SỬA/GHI NHẬN) trong docs/FAIL_OPEN_AUDIT.md rồi mới vào baseline.
Dùng: python3 tools/fail_open_scan.py [--baseline docs/fail_open_baseline.json] [--write-baseline] [--summary]
Mã thoát: 0 (không mục mới hoặc chỉ in) | 1 (có mục mới so với baseline)."""
import json
import os
import re
import sys

ROOT = os.environ.get("GRIT_ROOT") or os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CRATES = ["gt-types", "gt-mir", "gt-borrowck", "gt-oblig", "gt-term", "gt-effects", "gt-conc", "gt-secrets", "gt-guard", "gt-driver", "gt-codegen-llvm"]
PATTERNS = [
    ("unwrap_or", re.compile(r"\.unwrap_or\(")),
    ("unwrap_or_default", re.compile(r"\.unwrap_or_default\(")),
    ("ok()?", re.compile(r"\.ok\(\)\s*\?")),
    ("let _ =", re.compile(r"\blet\s+_\s*=")),
    ("_ =>", re.compile(r"^\s*_\s*=>")),
    ("map_or(true", re.compile(r"\.map_or\(\s*true\b")),
    ("map_or(false", re.compile(r"\.map_or\(\s*false\b")),
    ("is_none_or", re.compile(r"\.is_none_or\(")),
    ("is_some_and", re.compile(r"\.is_some_and\(")),
    ("if let Some … else {}", re.compile(r"\belse\s*\{\s*\}")),
]


def norm(line):
    return re.sub(r"\s+", " ", line.strip())


def scan():
    hits = []
    for c in CRATES:
        base = os.path.join(ROOT, "crates", c, "src")
        if not os.path.isdir(base):
            continue
        for dp, _, fs in os.walk(base):
            for f in sorted(fs):
                if not f.endswith(".rs"):
                    continue
                p = os.path.join(dp, f)
                rel = os.path.relpath(p, ROOT).replace(os.sep, "/")
                in_test = False
                with open(p, encoding="utf-8") as fh:
                    for i, line in enumerate(fh, 1):
                        if re.match(r"\s*#\[cfg\(test\)\]", line):
                            in_test = True  # phần còn lại của tệp là test (quy ước của dự án: mod tests ở cuối)
                        if in_test or line.lstrip().startswith("//"):
                            continue
                        for name, rx in PATTERNS:
                            if rx.search(line):
                                hits.append({"file": rel, "line": i, "pattern": name, "code": norm(line)})
    return hits


def key(h):
    return f"{h['file']}|{h['pattern']}|{h['code']}"


def main(argv):
    hits = scan()
    if "--summary" in argv:
        by = {}
        for h in hits:
            by.setdefault(h["file"].split("/")[1], {}).setdefault(h["pattern"], 0)
            by[h["file"].split("/")[1]][h["pattern"]] += 1
        for c in sorted(by):
            print(c, sum(by[c].values()), dict(sorted(by[c].items())))
        print("TỔNG", len(hits))
        return 0
    bpath = None
    if "--baseline" in argv:
        bpath = argv[argv.index("--baseline") + 1]
    if "--write-baseline" in argv and bpath:
        with open(bpath, "w", encoding="utf-8", newline="\n") as f:
            json.dump(sorted({key(h) for h in hits}), f, ensure_ascii=False, indent=0)
            f.write("\n")
        print(f"đã ghi baseline {len(hits)} mục -> {bpath}")
        return 0
    known = set()
    if bpath and os.path.exists(bpath):
        with open(bpath, encoding="utf-8") as f:
            known = set(json.load(f))
    new = [h for h in hits if key(h) not in known]
    for h in new:
        print(f"{h['file']}:{h['line']} | {h['pattern']} | {h['code']}")
    print(f"FAIL_OPEN_SCAN: {len(hits)} mục, {len(new)} MỚI so với baseline")
    return 1 if (bpath and new) else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
