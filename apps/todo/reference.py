#!/usr/bin/env python3
"""Bản tham chiếu ĐỘC LẬP của todo (đặc tả README.md). Dùng: python reference.py <tệp> add|done|remove|list [đối số]."""
import sys


def die(msg, code):
    sys.stderr.write("todo: " + msg + "\n")
    return code


def main():
    a = sys.argv
    if len(a) < 3 or len(a) > 4:
        return die("usage", 2)
    path, cmd = a[1], a[2]
    try:
        data = open(path, "rb").read()
    except FileNotFoundError:
        data = b""
    except OSError:
        return die("cannot read data file", 2)
    lines = []
    pos = 0
    while pos < len(data):
        e = data.find(b"\n", pos)
        if e < 0:
            return die("data file is corrupt", 4)
        ln = data[pos:e]
        if len(ln) < 5 or ln[0:1] != b"[" or ln[1:2] not in (b" ", b"x") or ln[2:3] != b"]" or ln[3:4] != b" ":
            return die("data file is corrupt", 4)
        lines.append(ln)
        pos = e + 1
    out = sys.stdout.buffer
    if cmd == "list":
        if len(a) != 3:
            return die("usage", 2)
        for i, ln in enumerate(lines):
            out.write(b"%d " % (i + 1) + ln + b"\n")
        return 0
    if len(a) != 4 or cmd not in ("add", "done", "remove"):
        return die("usage", 2)
    arg = a[3]
    if cmd == "add":
        ab = arg.encode("utf-8", "surrogateescape")
        if len(ab) == 0 or b"\n" in ab:
            return die("text must be non-empty and have no newline", 2)
        lines.append(b"[ ] " + ab)
        msg = b"added %d\n" % len(lines)
    else:
        if not arg.isascii() or not arg.isdigit():
            return die("bad item number", 2)
        num = int(arg)
        if num >= 1 << 64:
            return die("bad item number", 2)
        if num == 0 or num > len(lines):
            return die("item number out of range", 3)
        if cmd == "done":
            lines[num - 1] = b"[x] " + lines[num - 1][4:]
            msg = b"done " + arg.encode() + b"\n"
        else:
            del lines[num - 1]
            msg = b"removed " + arg.encode() + b"\n"
    try:
        open(path, "wb").write(b"".join(ln + b"\n" for ln in lines))
    except OSError:
        return die("cannot write data file", 2)
    out.write(msg)
    return 0


if __name__ == "__main__":
    sys.exit(main())
