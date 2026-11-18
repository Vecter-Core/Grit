#!/usr/bin/env python3
"""Bản tham chiếu ĐỘC LẬP của csv2json (cùng đặc tả README.md). Dùng: python reference.py <tệp.csv>; mã thoát và stdout là đối tượng so sánh."""
import sys

BS = bytes([0x5C])   # dấu gạch chéo ngược


def esc(b):
    out = bytearray(b'"')
    for c in b:
        if c == 0x22:
            out += BS + b'"'
        elif c == 0x5C:
            out += BS + BS
        elif c == 0x0A:
            out += BS + b"n"
        elif c == 0x0D:
            out += BS + b"r"
        elif c == 0x09:
            out += BS + b"t"
        elif c < 0x20:
            out += BS + b"u00" + b"%02x" % c
        else:
            out.append(c)
    out += b'"'
    return bytes(out)


def parse(data):
    """trả list bản ghi (list bytes) hoặc None nếu lỗi nháy"""
    n = len(data)
    i = 0
    recs = []
    while i < n:
        if data[i] == 0x0A:
            i += 1
            continue
        if data[i] == 0x0D and i + 1 < n and data[i + 1] == 0x0A:
            i += 2
            continue
        fields = []
        while True:
            if i < n and data[i] == 0x22:
                i += 1
                buf = bytearray()
                while True:
                    if i >= n:
                        return None
                    if data[i] == 0x22:
                        if i + 1 < n and data[i + 1] == 0x22:
                            buf.append(0x22)
                            i += 2
                            continue
                        i += 1
                        break
                    buf.append(data[i])
                    i += 1
                fields.append(bytes(buf))
            else:
                j = i
                while j < n and data[j] not in (0x2C, 0x0A, 0x22) and not (data[j] == 0x0D and j + 1 < n and data[j + 1] == 0x0A):
                    j += 1
                if j < n and data[j] == 0x22:
                    return None
                fields.append(data[i:j])
                i = j
            # sau trường
            if i >= n:
                break
            if data[i] == 0x2C:
                i += 1
                continue
            if data[i] == 0x0A:
                i += 1
                break
            if data[i] == 0x0D and i + 1 < n and data[i + 1] == 0x0A:
                i += 2
                break
            return None
        recs.append(fields)
    return recs


def main():
    if len(sys.argv) != 2:
        sys.stderr.write("usage: csv2json <file.csv>\n")
        return 2
    try:
        data = open(sys.argv[1], "rb").read()
    except OSError:
        sys.stderr.write("csv2json: cannot read\n")
        return 2
    try:
        data.decode("utf-8")
    except UnicodeDecodeError:
        sys.stderr.write("csv2json: input is not valid UTF-8\n")
        return 2
    recs = parse(data)
    if recs is None:
        sys.stderr.write("csv2json: malformed quoting\n")
        return 3
    if len(recs) <= 1:
        sys.stdout.buffer.write(b"[]\n")
        return 0
    hdr = recs[0]
    for r in recs[1:]:
        if len(r) != len(hdr):
            sys.stderr.write("csv2json: row has a different number of fields than the header\n")
            return 3
    rows = []
    for r in recs[1:]:
        rows.append(b"{" + b",".join(esc(h) + b":" + esc(v) for h, v in zip(hdr, r)) + b"}")
    sys.stdout.buffer.write(b"[\n" + b",\n".join(rows) + b"\n]\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
