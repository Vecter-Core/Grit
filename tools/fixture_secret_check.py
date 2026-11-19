#!/usr/bin/env python3
"""GÓI 8 S0f: phân loại NGOẠI TUYẾN các chuỗi giống bí mật trong tests/ và crates/gt-secrets/tests (mẫu của chính bộ quét S101).

Loại:
  (i)   CẤU TRÚC HỢP LỆ, có thể là thật — token GitHub `gh[pousr]_`/`github_pat_` có 6 ký tự cuối = checksum đúng theo cách tính
        ĐÃ NÊU (CRC32 của 30 ký tự ngẫu nhiên, mã hoá base62, đệm trái tới 6). CHƯA đối chiếu với tài liệu nhà cung cấp (không gọi mạng):
        vì vậy ghi rõ "theo cách tính đã nêu".
  (ii)  VÍ DỤ CÔNG KHAI — chuỗi ví dụ nổi tiếng trong tài liệu nhà cung cấp (vd khoá AWS `AKIAIOSFODNN7EXAMPLE`).
  (iii) GIẢ HIỂN NHIÊN — ký tự lặp, hoặc chứa example/test/fake/dummy/xxxx/123456/placeholder.
  (iv)  KHÔNG PHÂN LOẠI ĐƯỢC — còn lại (và mọi trường hợp không chắc cách tính: ghi KHÔNG KIỂM ĐƯỢC thay vì đoán).

KHÔNG in giá trị: chỉ 4 ký tự đầu. KHÔNG gọi mạng. Ghi vào docs/SECRET_FIXTURES.md; mục (i) và (iv) được đưa vào đầu docs/PENDING_GIANG.md
(tệp, dòng, loại, việc Giang cần làm) bằng `--pending`.
Dùng: python tools/fixture_secret_check.py [--out docs/SECRET_FIXTURES.md] [--pending]
"""
import os
import re
import sys
import zlib

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SCAN_DIRS = ("tests", os.path.join("crates", "gt-secrets", "tests"))
B62 = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"
GH = re.compile(r"\b((?:gh[pousr])_([A-Za-z0-9]{30})([A-Za-z0-9]{6})|github_pat_[A-Za-z0-9_]{20,})")
AWS = re.compile(r"\b((?:AKIA|ASIA)[0-9A-Z]{16})\b")
GOOGLE = re.compile(r"\bAIza[0-9A-Za-z_\-]{35}\b")
SLACK = re.compile(r"\bxox[baprs]-[A-Za-z0-9\-]{10,}")
PEM = re.compile(r"-----BEGIN (?:RSA |EC |OPENSSH |PGP |DSA )?PRIVATE KEY")
OTHER = re.compile(r"\b(sk_(?:live|test)_[A-Za-z0-9]{16,}|sk-ant-[A-Za-z0-9_\-]{20,})")
PUBLIC_EXAMPLES = {"AKIAIOSFODNN7EXAMPLE", "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"}
FAKE_WORDS = re.compile(r"(example|test|fake|dummy|xxxx|123456|placeholder|sample)", re.I)


def b62(n, width=6):
    s = ""
    while n:
        n, r = divmod(n, 62)
        s = B62[r] + s
    return s.rjust(width, "0")


def github_checksum_ok(payload, check):
    """Cách tính đã nêu: CRC32(payload) → base62 đệm trái 6 ký tự; so với 6 ký tự cuối."""
    return b62(zlib.crc32(payload.encode("ascii")) & 0xFFFFFFFF) == check


def is_repeated(tok):
    body = re.sub(r"^[a-z]+_", "", tok)
    return len(set(body)) <= 3 or re.search(r"(.)\1{5,}", body) is not None


def classify(kind, tok, m=None):
    """Trả (loại, ghi_chú). loại ∈ {i, ii, iii, iv}."""
    if kind == "github":
        if m is None or m.group(2) is None:
            return "iv", "dạng github_pat_ (checksum chưa biết cách tính đã nêu)"
        if github_checksum_ok(m.group(2), m.group(3)):
            return "i", "CẤU TRÚC HỢP LỆ theo cách tính đã nêu (CRC32/base62); chưa đối chiếu tài liệu nhà cung cấp"
        if FAKE_WORDS.search(tok) or is_repeated(tok):
            return "iii", "checksum sai và có dấu hiệu giả"
        return "iv", "checksum không khớp cách tính đã nêu: KHÔNG KIỂM ĐƯỢC chắc chắn"
    if tok in PUBLIC_EXAMPLES:
        return "ii", "ví dụ công khai của nhà cung cấp"
    if FAKE_WORDS.search(tok) or is_repeated(tok):
        return "iii", "dấu hiệu giả (từ khoá giả hoặc ký tự lặp)"
    return "iv", "không phân loại được"


def candidates(text):
    """Trả (kind, tok, match|None) cho mọi chuỗi giống bí mật trong text."""
    for m in GH.finditer(text):
        yield "github", m.group(1), m
    for m in AWS.finditer(text):
        yield "aws", m.group(1), None
    for m in GOOGLE.finditer(text):
        yield "google", m.group(0), None
    for m in SLACK.finditer(text):
        yield "slack", m.group(0), None
    for m in OTHER.finditer(text):
        yield "other", m.group(1), None
    for m in PEM.finditer(text):
        yield "pem", m.group(0), None


def scan(root=ROOT):
    rows = []
    for d in SCAN_DIRS:
        base = os.path.join(root, d)
        for dp, dn, fs in os.walk(base):
            dn[:] = [x for x in dn if x not in ("target", ".git")]
            for fn in fs:
                if not fn.endswith((".rs", ".gt", ".md", ".txt", ".py", ".toml", ".json", ".ll")):
                    continue
                path = os.path.join(dp, fn)
                try:
                    with open(path, encoding="utf-8") as fh:
                        lines = fh.read().splitlines()
                except (UnicodeDecodeError, OSError):
                    continue
                for ln_no, line in enumerate(lines, 1):
                    for kind, tok, m in candidates(line):
                        cls, note = classify(kind, tok, m)
                        rows.append({"file": os.path.relpath(path, root).replace(os.sep, "/"), "line": ln_no,
                                     "kind": kind, "masked": tok[:4] + "****", "cls": cls, "note": note})
    return rows


def render(rows):
    out = ["# SECRET_FIXTURES — phân loại NGOẠI TUYẾN các fixture giống bí mật (GÓI 8 S0f)", "",
           "Sinh bởi `tools/fixture_secret_check.py`. KHÔNG in giá trị (chỉ 4 ký tự đầu), KHÔNG gọi mạng.",
           "Loại: **(i)** cấu trúc hợp lệ theo cách tính đã nêu (có thể là thật — cần Giang kiểm); **(ii)** ví dụ công khai; **(iii)** giả hiển nhiên; **(iv)** không phân loại được.",
           "Cách tính checksum (i) là CRC32 + base62 đệm 6 ký tự, theo mô tả của phiên này; CHƯA đối chiếu tài liệu nhà cung cấp (không có mạng).", "",
           "| tệp | dòng | loại bí mật | 4 ký tự đầu | phân loại | ghi chú |", "|---|---:|---|---|---|---|"]
    for r in sorted(rows, key=lambda x: (x["cls"], x["file"], x["line"])):
        out.append("| %s | %d | %s | %s | (%s) | %s |" % (r["file"], r["line"], r["kind"], r["masked"], r["cls"], r["note"]))
    counts = {}
    for r in rows:
        counts[r["cls"]] = counts.get(r["cls"], 0) + 1
    out += ["", "Tổng: %d mục — %s." % (len(rows), ", ".join("(%s) %d" % (k, counts.get(k, 0)) for k in ("i", "ii", "iii", "iv")))]
    return "\n".join(out) + "\n"


def pending_block(rows):
    need = [r for r in rows if r["cls"] in ("i", "iv")]
    lines = ["## KIỂM FIXTURE BÍ MẬT (GÓI 8 S0f, ngoại tuyến)",
             "Mục loại (i) và (iv) — việc Giang: mở từng tệp, dòng dưới đây; nếu chuỗi giống token/khoá THẬT của bạn thì THU HỒI ngay. Giá trị không được in ở đây (chỉ 4 ký tự đầu)."]
    for r in need:
        lines.append("* (%s) `%s:%d` — loại %s, bắt đầu `%s` — %s" % (r["cls"], r["file"], r["line"], r["kind"], r["masked"], r["note"]))
    if not need:
        lines.append("* không có mục loại (i) hoặc (iv).")
    return "\n".join(lines) + "\n"


def main(argv):
    sys.stdout.reconfigure(encoding="utf-8")
    out = os.path.join(ROOT, "docs", "SECRET_FIXTURES.md")
    if "--out" in argv:
        out = argv[argv.index("--out") + 1]
    rows = scan()
    with open(out, "w", encoding="utf-8", newline="\n") as f:
        f.write(render(rows))
    print("ghi", out, "—", len(rows), "mục")
    if "--pending" in argv:
        put_pending(os.path.join(ROOT, "docs", "PENDING_GIANG.md"), pending_block(rows))
    return 0


def put_pending(path, block):
    """Đặt khối KIỂM FIXTURE lên ĐẦU tệp (sau dòng tiêu đề `# …`). Nếu đã có khối cũ (bắt đầu bằng marker) thì thay nó (đến `## ` kế tiếp)."""
    marker = "## KIỂM FIXTURE BÍ MẬT"
    with open(path, encoding="utf-8") as f:
        s = f.read()
    nl = "\r\n" if "\r\n" in s else "\n"
    body = block.replace("\n", nl)
    lines = s.split(nl)
    if any(x.startswith(marker) for x in lines):
        start = next(i for i, x in enumerate(lines) if x.startswith(marker))
        end = next((i for i in range(start + 1, len(lines)) if lines[i].startswith("## ")), len(lines))
        new = lines[:start] + body.split(nl) + lines[end:]
    else:
        # ĐẦU tệp: ngay sau dòng tiêu đề `# …` (dòng 1)
        first = 1 if lines and lines[0].startswith("# ") else 0
        new = lines[:first] + [""] + body.split(nl) + lines[first:]
    with open(path, "w", encoding="utf-8", newline="") as f:
        f.write(nl.join(new))


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
