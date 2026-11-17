#!/usr/bin/env python3
"""Kiểm kê `unsafe` TỰ ĐỘNG (UNSAFE-01, GÓI 3 S2.a).

Dùng: python tools/unsafe_inventory.py [--write] [thư_mục_gốc]
- Đếm các chỗ `unsafe {`, `unsafe fn/impl/trait/extern` và thuộc tính `#[unsafe(...)]` trong `crates/*/src/**/*.rs` và `runtime/gt-rt/src/**/*.rs`
  (bỏ qua chú thích, chuỗi, thư mục tests/fuzz/target).
- So với `docs/unsafe_baseline.json` {"allowed_crates": [...], "files": {đường dẫn: số}}. LỖI nếu: một tệp có SỐ LƯỢNG VƯỢT baseline (kể cả tệp mới);
  hoặc một crate KHÔNG nằm trong allowed_crates mà không cấm unsafe (`#![forbid(unsafe_code)]` hoặc `[lints] workspace = true` với workspace `unsafe_code = "forbid"`).
- `--write` ghi baseline từ trạng thái hiện tại (chỉ dùng sau khi ĐỌC từng chỗ unsafe mới và ghi lý do ở docs/KNOWN_LIMITATIONS hoặc chú thích SAFETY).
Đây là kiểm kê chữ nghĩa (không phải phân tích an toàn): không chứng minh các chỗ unsafe đúng.
"""
from __future__ import annotations

import json
import os
import re
import sys

BASELINE = os.path.join("docs", "unsafe_baseline.json")
PATTERN = re.compile(r"\bunsafe\b\s*(?:\{|fn\b|impl\b|trait\b|extern\b)|#\[unsafe\(")
STRING = re.compile(r'"(?:\\.|[^"\\])*"')


def strip(text: str) -> str:
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.S)
    out = []
    for line in text.splitlines():
        line = STRING.sub('""', line)
        i = line.find("//")
        out.append(line if i < 0 else line[:i])
    return "\n".join(out)


def count_file(path: str) -> int:
    with open(path, encoding="utf-8", errors="replace") as f:
        return len(PATTERN.findall(strip(f.read())))


def rust_files(src_dir: str):
    for base, dirs, files in os.walk(src_dir):
        dirs[:] = [d for d in dirs if d not in ("tests", "fuzz", "target", "benches")]
        for n in files:
            if n.endswith(".rs"):
                yield os.path.join(base, n)


def scan(root: str) -> tuple[dict[str, int], list[str]]:
    """Trả ({tệp: số unsafe} chỉ tệp có > 0, danh sách crate thiếu cấm unsafe)."""
    counts: dict[str, int] = {}
    unforbidden: list[str] = []
    crates_dir = os.path.join(root, "crates")
    dirs = []
    if os.path.isdir(crates_dir):
        dirs += [os.path.join(crates_dir, c) for c in sorted(os.listdir(crates_dir)) if os.path.isdir(os.path.join(crates_dir, c))]
    rt = os.path.join(root, "runtime", "gt-rt")
    if os.path.isdir(rt):
        dirs.append(rt)
    for c in dirs:
        rel_c = os.path.relpath(c, root).replace("\\", "/")
        src = os.path.join(c, "src")
        if not os.path.isdir(src):
            continue
        for p in rust_files(src):
            n = count_file(p)
            if n:
                counts[os.path.relpath(p, root).replace("\\", "/")] = n
        cargo = os.path.join(c, "Cargo.toml")
        ctext = open(cargo, encoding="utf-8").read() if os.path.isfile(cargo) else ""
        ws_lints = re.search(r"^\[lints\]\s*\n(?:[^\[]*\n)*?\s*workspace\s*=\s*true", ctext, flags=re.M) is not None
        attr = any(
            os.path.isfile(os.path.join(src, f)) and "forbid(unsafe_code)" in open(os.path.join(src, f), encoding="utf-8", errors="replace").read()
            for f in ("lib.rs", "main.rs")
        )
        if not (ws_lints or attr):
            unforbidden.append(rel_c)
    return counts, unforbidden


def check(root: str) -> list[str]:
    bp = os.path.join(root, BASELINE)
    base = json.load(open(bp, encoding="utf-8")) if os.path.isfile(bp) else {"allowed_crates": [], "files": {}}
    allowed = set(base.get("allowed_crates", []))
    files = base.get("files", {})
    counts, unforbidden = scan(root)
    problems = []
    for f, n in sorted(counts.items()):
        b = files.get(f, 0)
        if n > b:
            problems.append(f"{f}: {n} chỗ unsafe > baseline {b} (đọc chỗ mới, ghi lý do/SAFETY rồi `--write`)")
    for c in unforbidden:
        name = c.split("/")[-1]
        if name not in allowed and c not in allowed:
            problems.append(f"{c}: crate không cấm unsafe (thiếu forbid(unsafe_code) / [lints] workspace) và không nằm trong allowed_crates")
    return problems


def main(argv: list[str]) -> int:
    write = "--write" in argv
    rest = [a for a in argv if not a.startswith("--")]
    root = os.path.abspath(rest[0]) if rest else os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    if write:
        counts, unforbidden = scan(root)
        allowed = sorted({c.split("/")[-1] for c in unforbidden})
        os.makedirs(os.path.join(root, "docs"), exist_ok=True)
        with open(os.path.join(root, BASELINE), "w", encoding="utf-8", newline="\n") as f:
            json.dump({"allowed_crates": allowed, "files": dict(sorted(counts.items()))}, f, ensure_ascii=False, indent=2)
            f.write("\n")
        print(f"UNSAFE_BASELINE_WRITTEN {len(counts)} tệp, crate được phép: {allowed}")
        return 0
    problems = check(root)
    if problems:
        print("UNSAFE_INVENTORY_RED")
        for p in problems:
            print("  " + p)
        return 1
    counts, _ = scan(root)
    print(f"UNSAFE_INVENTORY_OK {sum(counts.values())} chỗ trong {len(counts)} tệp")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
