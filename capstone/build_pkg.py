#!/usr/bin/env python3
"""Dựng một gói Rig của capstone ở MỘT hồ sơ bất kỳ (rig chỉ dựng hồ sơ dev): gom src/*.gt của gói + lib/std/src/*.gt vào thư mục tạm rồi `gritc build --profile=…`.
Dùng: build_pkg.py <thư-mục-gói> <hồ-sơ: dev|release|release-o2> <ra.exe>   (GRITC=đường-dẫn-gritc, mặc định gritc trong PATH)"""
import os
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
STD = os.path.join(os.path.dirname(HERE), "lib", "std", "src")


def main():
    if len(sys.argv) != 4:
        print(__doc__)
        return 3
    pkg, prof, out = sys.argv[1:]
    d = tempfile.mkdtemp(prefix="capbuild-")
    try:
        for srcdir in (STD, os.path.join(pkg, "src")):
            for f in os.listdir(srcdir):
                if f.endswith(".gt"):
                    shutil.copy(os.path.join(srcdir, f), os.path.join(d, f))
        g = os.environ.get("GRITC", "gritc")
        r = subprocess.run([g, "build", d, f"--profile={prof}", "-o", out], capture_output=True, text=True)
        if r.returncode != 0:
            sys.stderr.write(r.stdout[-3000:] + r.stderr[-3000:])
        return r.returncode
    finally:
        shutil.rmtree(d, ignore_errors=True)


if __name__ == "__main__":
    sys.exit(main())
