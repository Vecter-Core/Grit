#!/usr/bin/env python3
"""Bản tham chiếu ĐỘC LẬP của difflite (đặc tả README.md). Dùng: python reference.py <tệp_a> <tệp_b>."""
import sys


def lines(data):
    out = []
    pos = 0
    while pos < len(data):
        e = data.find(b"\n", pos)
        if e < 0:
            e = len(data)
        out.append(data[pos:e])
        pos = e + 1
    return out


def main():
    if len(sys.argv) != 3:
        sys.stderr.write("usage: difflite <file_a> <file_b>\n")
        return 2
    try:
        a = lines(open(sys.argv[1], "rb").read())
        b = lines(open(sys.argv[2], "rb").read())
    except OSError:
        sys.stderr.write("difflite: cannot read input file\n")
        return 2
    na, nb = len(a), len(b)
    if na > 2000 or nb > 2000 or (na + 1) * (nb + 1) > 4000000:
        sys.stderr.write("difflite: input too large\n")
        return 3
    # L[i][j] = LCS(a[i:], b[j:])
    L = [[0] * (nb + 1) for _ in range(na + 1)]
    for i in range(na - 1, -1, -1):
        for j in range(nb - 1, -1, -1):
            if a[i] == b[j]:
                L[i][j] = L[i + 1][j + 1] + 1
            else:
                L[i][j] = max(L[i + 1][j], L[i][j + 1])
    res = []
    diffs = 0
    i = j = 0
    while i < na or j < nb:
        if i < na and j < nb and a[i] == b[j]:
            res.append(b"  " + a[i])
            i += 1
            j += 1
        elif j >= nb or (i < na and L[i + 1][j] >= L[i][j + 1]):
            res.append(b"- " + a[i])
            i += 1
            diffs += 1
        else:
            res.append(b"+ " + b[j])
            j += 1
            diffs += 1
    if diffs == 0:
        return 0
    sys.stdout.buffer.write(b"".join(x + b"\n" for x in res))
    return 1


if __name__ == "__main__":
    sys.exit(main())
