#!/usr/bin/env python3
"""Quét byte NUL trong tệp VĂN BẢN đã theo dõi (R1; sự cố 2026-10-16: tệp bị ghi đè bằng NUL không bị phát hiện cho tới khi biên dịch).
Dùng: nul_scan.py <thư-mục-kho>. Thoát 0 = sạch, 1 = có tệp chứa NUL (in tên + số byte NUL), 2 = không đọc được kho.
Tệp nhị phân đã biết (đuôi trong BINARY) bị bỏ qua; mọi đuôi khác (kể cả không đuôi) coi là văn bản.
Ngoại lệ có chủ ý: thư mục `fuzz_regressions/` (đầu vào đã làm sập lexer/parser, CỐ Ý chứa NUL)."""
import os
import subprocess
import sys

BINARY = {".exe", ".dll", ".lib", ".a", ".o", ".obj", ".png", ".jpg", ".jpeg", ".gif", ".ico", ".pdf", ".zip", ".gz", ".xz", ".bundle", ".pdb", ".so", ".vsix", ".woff", ".woff2"}


EXEMPT_DIR = "fuzz_regressions"


def tracked(root):
    # `root` phải CHÍNH LÀ gốc một kho git (không phải thư mục con của kho khác: khi TMPDIR nằm trong dự án, thư mục tạm sẽ "thuộc" kho cha)
    top = subprocess.run(["git", "-C", root, "rev-parse", "--show-toplevel"], capture_output=True, text=True)
    if top.returncode != 0 or os.path.realpath(top.stdout.strip()) != os.path.realpath(root):
        return None
    p = subprocess.run(["git", "-C", root, "ls-files", "-z"], capture_output=True)
    if p.returncode != 0:
        return None
    return [x.decode("utf-8", "surrogateescape") for x in p.stdout.split(b"\0") if x]


def scan(root):
    files = tracked(root)
    if files is None:
        return None
    bad = []
    for f in files:
        if os.path.splitext(f)[1].lower() in BINARY or EXEMPT_DIR in f.replace("\\", "/").split("/"):
            continue
        path = os.path.join(root, f)
        try:
            with open(path, "rb") as fh:
                data = fh.read()
        except OSError:
            continue  # tệp đã bị xoá khỏi cây làm việc: việc của giai đoạn `deletions`/`track`
        n = data.count(b"\0")
        if n:
            bad.append((f, n, len(data)))
    return bad


def main(argv):
    root = argv[1] if len(argv) > 1 else "."
    bad = scan(root)
    if bad is None:
        print("NUL_SCAN: không đọc được kho git ở", root)
        return 2
    for f, n, size in bad:
        print(f"NUL_FOUND {f}: {n} byte NUL / {size} byte")
    print(f"NUL_SCAN: {len(bad)} tệp văn bản đã theo dõi chứa NUL")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
