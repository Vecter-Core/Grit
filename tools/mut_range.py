#!/usr/bin/env python3
"""Bao `tools/mut.py` để CHỈ sinh đột biến trong khoảng dòng (1-based, gồm hai đầu): GRIT_MUT_RANGE=lo-hi. Cách dùng như mut.py:
  GRIT_MUT_RANGE=293-690 python tools/mut_range.py list|run|report ...   (GÓI 10 S3.b: hàm quyết định của gt-term sửa ở FB-C32/33/34)"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mut  # noqa: E402

lo, hi = (int(x) for x in os.environ.get("GRIT_MUT_RANGE", "1-10**9".replace("10**9", "1000000000")).split("-"))
_orig = mut.gen_file


def gen_file(path):
    ms, crlf = _orig(path)
    return [m for m in ms if lo <= m[1] + 1 <= hi], crlf


mut.gen_file = gen_file
if __name__ == "__main__":
    mut.main()
