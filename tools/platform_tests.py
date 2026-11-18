#!/usr/bin/env python3
"""GÓI 5 S2.a: kiểm kê test CHỈ-MỘT-NỀN và test bị #[ignore]. Quét tĩnh các hàm #[test] có thuộc tính cfg(windows|unix|target_os|not(windows|unix)) hoặc #[ignore] ngay phía trên.
Mỗi mục PHẢI có trong docs/PLATFORM_TESTS.md (dòng bảng bắt đầu bằng `| `path::hàm` |`) và ngược lại; #[ignore] PHẢI có lý do (`#[ignore = "..."]`).
Dùng: python tools/platform_tests.py [--list]   (thoát 1 nếu lệch)."""
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DOC = os.path.join(ROOT, "docs", "PLATFORM_TESTS.md")
CFG = re.compile(r"#\[cfg\((?:not\()?(?:windows|unix|target_os)")
IGN = re.compile(r"#\[ignore(?P<rest>[^\]]*)\]")
FN = re.compile(r"\s*(?:pub\s+)?(?:async\s+)?fn\s+(\w+)")


def scan(root=ROOT):
    found, bad = {}, []
    for base in ("crates", "runtime", "tools"):
        for dp, dn, fs in os.walk(os.path.join(root, base)):
            dn[:] = [d for d in dn if d not in ("target", ".git")]
            for fn in fs:
                if not fn.endswith(".rs"):
                    continue
                path = os.path.join(dp, fn)
                rel = os.path.relpath(path, root).replace(os.sep, "/")
                lines = open(path, encoding="utf-8", errors="replace").read().split("\n")
                for i, l in enumerate(lines):
                    m = FN.match(l)
                    if not m:
                        continue
                    attrs = []
                    j = i - 1
                    while j >= 0 and (lines[j].strip().startswith("#[") or lines[j].strip().startswith("//")):
                        attrs.append(lines[j])
                        j -= 1
                    text = "\n".join(attrs)
                    if "#[test]" not in text:
                        continue
                    kinds = []
                    if CFG.search(text):
                        kinds.append("cfg")
                    g = IGN.search(text)
                    if g:
                        kinds.append("ignore")
                        if "=" not in g.group("rest"):
                            bad.append(f"{rel}::{m.group(1)}: #[ignore] KHÔNG có lý do")
                    if kinds:
                        found[f"{rel}::{m.group(1)}"] = "+".join(kinds)
    return found, bad


def documented():
    out = set()
    if os.path.isfile(DOC):
        for l in open(DOC, encoding="utf-8"):
            m = re.match(r"\|\s*`([^`|]+::\w+)`\s*\|", l)
            if m:
                out.add(m.group(1))
    return out


def main():
    found, bad = scan()
    doc = documented()
    if "--list" in sys.argv:
        for k, v in sorted(found.items()):
            print(f"{k}\t{v}")
        return 0
    errs = list(bad)
    errs += [f"chưa ghi trong docs/PLATFORM_TESTS.md: {k}" for k in sorted(set(found) - doc)]
    errs += [f"PLATFORM_TESTS.md ghi nhưng không còn trong mã: {k}" for k in sorted(doc - set(found))]
    if errs:
        print("PLATFORM_TESTS_FAIL")
        print("\n".join(errs))
        return 1
    print(f"PLATFORM_TESTS_OK ({len(found)} test một-nền/ignore đều có lý do và có trong tài liệu)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
