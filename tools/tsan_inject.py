#!/usr/bin/env python3
"""Chèn thuộc tính `sanitize_thread` vào MỌI hàm định nghĩa của một tệp LLVM IR văn bản (GÓI 4 S3). Dùng: python tools/tsan_inject.py in.ll > out.ll
Đây là BUILD PHÂN TÍCH (không phải binary đã qua Guard cuối): clang sẽ chèn lệnh theo dõi của ThreadSanitizer vào các hàm mang thuộc tính này."""
import sys


def inject(text: str) -> str:
    out = []
    for line in text.split("\n"):
        if line.startswith("define ") and line.rstrip().endswith("{") and "sanitize_thread" not in line:
            body = line.rstrip()
            line = body[:-1].rstrip() + " sanitize_thread {"
        out.append(line)
    return "\n".join(out)


if __name__ == "__main__":
    sys.stdout.write(inject(open(sys.argv[1], encoding="utf-8").read()))
