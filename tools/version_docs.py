#!/usr/bin/env python3
"""Số phiên bản trong tài liệu phải khớp mã (GÓI 4 S2.b). Lần trước tài liệu ghi `1.0.0-rc2` trong khi `gritc --version` in `1.0.0-rc1`.
`check(root)` trả danh sách lỗi (rỗng = khớp). Nguồn sự thật: `version` của gói gt-driver trong `crates/gt-driver/Cargo.toml`; nếu có bản dựng
`target/debug/gritc[.exe]` thì `gritc --version` cũng phải in đúng số đó."""
from __future__ import annotations

import os
import re
import subprocess
import sys

VER = re.compile(r"1\.0\.0-(?:rc\d+|seed)")


def cargo_version(root: str) -> str:
    t = open(os.path.join(root, "crates", "gt-driver", "Cargo.toml"), encoding="utf-8").read()
    m = re.search(r'^version\s*=\s*"([^"]+)"', t, flags=re.M)
    if not m:
        raise SystemExit("không đọc được version của gt-driver")
    return m.group(1)


def first_match(text: str, pat: str):
    m = re.search(pat, text, flags=re.S)
    return m.group(1) if m else None


def check(root: str, run_binary: bool = True) -> list[str]:
    v = cargo_version(root)
    errs: list[str] = []

    def read(rel):
        p = os.path.join(root, rel)
        return open(p, encoding="utf-8").read() if os.path.isfile(p) else None

    spots = [
        ("CHANGELOG.md", r"^## (1\.0\.0-[a-z0-9]+)", re.M),
        ("docs/spec/FREEZE.md", r"`gritc --version` ⇒ `([^`]+)`", 0),
        ("docs/READINESS.md", r"Nhãn hiện hành: \*\*`([^`]+)`\*\*", 0),
        ("docs/PROJECT_STATUS.md", r"Bản hiện tại là \*\*`([^`]+)`\*\*", 0),
    ]
    for rel, pat, fl in spots:
        t = read(rel)
        if t is None:
            errs.append(f"{rel}: thiếu tệp")
            continue
        m = re.search(pat, t, flags=fl)
        if not m:
            errs.append(f"{rel}: không tìm thấy dòng phiên bản (mẫu {pat})")
        elif m.group(1) != v:
            errs.append(f"{rel}: ghi {m.group(1)} nhưng mã là {v}")
    for rel in ("README.md", "BUILDING.md"):
        t = read(rel)
        if t:
            for found in set(VER.findall(t)):
                if found != v:
                    errs.append(f"{rel}: nhắc {found} nhưng mã là {v}")
    if run_binary:
        for exe in ("gritc.exe", "gritc"):
            p = os.path.join(root, "target", "debug", exe)
            if os.path.isfile(p):
                out = subprocess.run([p, "--version"], capture_output=True, text=True, encoding="utf-8", errors="replace").stdout
                if v not in out:
                    errs.append(f"{exe} --version in {out.strip()!r}, mã là {v}")
                break
    return errs


if __name__ == "__main__":
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    e = check(os.path.abspath(sys.argv[1]) if len(sys.argv) > 1 else root)
    print("VERSION_DOCS_OK" if not e else "VERSION_DOCS_RED\n  " + "\n  ".join(e))
    sys.exit(1 if e else 0)
