#!/usr/bin/env python3
"""Kiểm tra corpus mẫu `tests/` khớp với `docs/spec/error_codes.md` (M0, plan §0.5/§9).

Chưa parse `.gt` (parser ở M2); chỉ kiểm tính nhất quán của corpus. Exit 0 = sạch, 1 = có lỗi.
Chạy từ gốc repo:  python tools/check_expectations.py
"""
import os
import re
import sys
import unicodedata

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TESTS = os.path.join(ROOT, "tests")
CODES_MD = os.path.join(ROOT, "docs", "spec", "error_codes.md")
EXPECT_RE = re.compile(r"^// expect: (pass|lexer|fail E\d{4}|warn S\d{3})\s*$")
DIR_KIND = {"pass": "pass", "fail": "fail", "warn": "warn", "lexer": "lexer"}
# bidi, zero-width/vô hình (D15). Chỉ các mẫu E1201/E1202 được phép chứa.
BIDI = set(range(0x202A, 0x202F)) | set(range(0x2066, 0x206A))
INVISIBLE = {0x200B, 0x200C, 0x200D, 0x2060, 0xFEFF}
# Mẫu cố ý vi phạm Unicode: tên file -> (cho phép bidi, cho phép vô hình, cho phép non-NFC)
UNICODE_EXEMPT = {
    "E1201_bidi.gt": (True, False, False),
    "E1202_zero_width.gt": (False, True, False),
    "E1203_homoglyph.gt": (False, False, False),
    "E1204_not_nfc.gt": (False, False, True),
}

errors = []


def err(msg):
    errors.append(msg)


def load_code_table():
    """Trả về (tập mã, danh sách (mã, mục tham chiếu test)) từ bảng markdown."""
    codes, refs = set(), []
    with open(CODES_MD, encoding="utf-8") as f:
        for line in f:
            cells = [c.strip() for c in line.strip().strip("|").split("|")]
            if len(cells) < 4 or not re.fullmatch(r"[ESN]\d{3,4}", cells[0]):
                continue
            codes.add(cells[0])
            for ref in cells[-1].split(","):
                ref = ref.strip()
                if ref and ref != "—":
                    refs.append((cells[0], ref))
    return codes, refs


def gt_files():
    for kind in DIR_KIND:
        d = os.path.join(TESTS, kind)
        if not os.path.isdir(d):
            continue
        for name in sorted(os.listdir(d)):
            if name.endswith(".gt"):
                yield kind, name, os.path.join(d, name)


def check_unicode(name, text, rel):
    allow_bidi, allow_inv, allow_non_nfc = UNICODE_EXEMPT.get(name, (False, False, False))
    for ch in text:
        cp = ord(ch)
        if cp in BIDI and not allow_bidi:
            err(f"{rel}: ký tự bidi U+{cp:04X} không được phép trong mẫu này")
        if cp in INVISIBLE and not allow_inv:
            err(f"{rel}: ký tự vô hình U+{cp:04X} không được phép trong mẫu này")
    if not allow_non_nfc and unicodedata.normalize("NFC", text) != text:
        err(f"{rel}: nội dung không phải NFC")
    if name in UNICODE_EXEMPT:
        # mẫu cố ý xấu phải THỰC SỰ chứa thứ nó nhắm tới, nếu không test vô nghĩa
        has_bidi = any(ord(c) in BIDI for c in text)
        has_inv = any(ord(c) in INVISIBLE for c in text)
        has_non_nfc = unicodedata.normalize("NFC", text) != text
        has_mixed = any("CYRILLIC" in unicodedata.name(c, "") for c in text)
        want = {"E1201_bidi.gt": has_bidi, "E1202_zero_width.gt": has_inv,
                "E1203_homoglyph.gt": has_mixed, "E1204_not_nfc.gt": has_non_nfc}[name]
        if not want:
            err(f"{rel}: mẫu Unicode độc hại nhưng không chứa ký tự mục tiêu")


def main():
    codes, refs = load_code_table()
    if not codes:
        err("không đọc được bảng mã lỗi trong error_codes.md")
    seen_files = {"fail": set(), "warn": set(), "pass": set(), "lexer": set()}
    total = 0
    for kind, name, path in gt_files():
        total += 1
        rel = f"tests/{kind}/{name}"
        seen_files[kind].add(name[:-3])
        try:
            with open(path, "rb") as f:
                raw = f.read()
            text = raw.decode("utf-8")
        except UnicodeDecodeError:
            err(f"{rel}: không phải UTF-8 hợp lệ")
            continue
        lines = text.split("\n")
        expect_lines = [ln for ln in lines if ln.startswith("// expect:")]
        if len(expect_lines) != 1:
            err(f"{rel}: cần đúng 1 dòng '// expect:', có {len(expect_lines)}")
            continue
        if lines[0] != expect_lines[0]:
            err(f"{rel}: dòng expect phải là dòng đầu tiên")
        m = EXPECT_RE.match(expect_lines[0])
        if not m:
            err(f"{rel}: dòng expect sai định dạng: {expect_lines[0]!r}")
            continue
        exp = m.group(1)
        word = exp.split()[0]
        if word != DIR_KIND[kind]:
            err(f"{rel}: expect '{word}' nhưng nằm trong thư mục '{kind}'")
        if word in ("fail", "warn"):
            code = exp.split()[1]
            if code not in codes:
                err(f"{rel}: mã {code} không có trong docs/spec/error_codes.md")
            if not name.startswith(code + "_"):
                err(f"{rel}: tên file phải bắt đầu bằng '{code}_'")
        if word == "lexer":
            tok = path[:-3] + ".tokens"
            if not os.path.isfile(tok):
                err(f"{rel}: thiếu file {os.path.basename(tok)}")
            else:
                with open(tok, encoding="utf-8") as f:
                    body = [ln for ln in f.read().split("\n") if ln.strip() and not ln.startswith("#")]
                if not body:
                    err(f"{rel}: file .tokens rỗng")
        check_unicode(name, text, rel)

    # mỗi mục tham chiếu trong bảng mã lỗi phải trỏ tới file có thật; mỗi mã đó khớp tiền tố
    cited = set()
    for code, ref in refs:
        if ref.startswith("pass/"):
            sub, base = "pass", ref[5:]
        else:
            sub, base = ("warn" if code.startswith("S") else "fail"), ref
        cited.add((sub, base))
        if base not in seen_files[sub]:
            err(f"error_codes.md: {code} tham chiếu '{ref}' nhưng tests/{sub}/{base}.gt không tồn tại")
        elif sub != "pass" and not base.startswith(code + "_"):
            err(f"error_codes.md: {code} tham chiếu '{ref}' không khớp tiền tố mã")
    # file fail/warn không được mồ côi (mọi mẫu phải được liệt kê trong bảng)
    for sub in ("fail", "warn"):
        for base in sorted(seen_files[sub]):
            if (sub, base) not in cited:
                err(f"tests/{sub}/{base}.gt không được liệt kê trong error_codes.md")

    # tên file không được trùng khi bỏ phân biệt hoa/thường (Windows/macOS sẽ ghi đè lẫn nhau)
    for kind in DIR_KIND:
        d = os.path.join(TESTS, kind)
        if os.path.isdir(d):
            folded = {}
            for name in os.listdir(d):
                key = name.casefold()
                if key in folded:
                    err(f"tests/{kind}: '{name}' trùng '{folded[key]}' khi bỏ phân biệt hoa/thường")
                folded[key] = name

    counts = {k: len(v) for k, v in seen_files.items()}
    print(f"Đã kiểm {total} file .gt: " + ", ".join(f"{k}={v}" for k, v in counts.items()))
    print(f"Bảng mã lỗi: {len(codes)} mã, {len(refs)} tham chiếu test")
    if errors:
        print(f"\nLỖI ({len(errors)}):")
        for e in errors:
            print("  - " + e)
        return 1
    print("OK: corpus nhất quán với error_codes.md")
    return 0


if __name__ == "__main__":
    sys.exit(main())
