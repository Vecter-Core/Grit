#!/usr/bin/env python3
"""Chèn thuộc tính `sanitize_thread` vào MỌI hàm định nghĩa của một tệp LLVM IR văn bản (GÓI 4 S3). Dùng: python tools/tsan_inject.py [--drop-locks] in.ll > out.ll
Đây là BUILD PHÂN TÍCH (không phải binary đã qua Guard cuối): clang sẽ chèn lệnh theo dõi của ThreadSanitizer vào các hàm mang thuộc tính này."""
import sys


def inject(text: str, drop_locks: bool = False) -> str:
    """drop_locks=True: CHỈ để làm ĐỐI CHỨNG DƯƠNG (GÓI 5 S1.a): xoá mọi lệnh gọi gritrt_mutex_lock/unlock khỏi IR do gritc sinh (tương đương Mutex::with bỏ khoá).
    Tuyệt đối không dùng cho bản chạy thật; gritc/gt-rt phát hành KHÔNG có cơ chế này (nó nằm ở công cụ kiểm thử này)."""
    out = []
    for line in text.split("\n"):
        if drop_locks and line.lstrip().startswith("call void @gritrt_mutex_") and ("_lock(" in line or "_unlock(" in line):
            continue
        if line.startswith("define ") and line.rstrip().endswith("{") and "sanitize_thread" not in line:
            body = line.rstrip()
            line = body[:-1].rstrip() + " sanitize_thread {"
        out.append(line)
    return "\n".join(out)


if __name__ == "__main__":
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    sys.stdout.write(inject(open(args[0], encoding="utf-8").read(), "--drop-locks" in sys.argv))
