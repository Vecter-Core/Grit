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

IDENT_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")
CASE_EXPECT_RE = re.compile(r"^expect: (pass|fail E\d{4}|warn S\d{3})\s*$")
# Mã đã triển khai ở milestone đã đóng: PHẢI có docs/errors/<mã>.md với >=3 test fail + >=3 test pass (plan §0.5).
# Mở rộng dần theo milestone (M3: E01xx và E02xx).
DOC_REQUIRED = [
    "E0101", "E0102", "E0201", "E0202", "E0203", "E0204", "E0205",
    "E0301", "E0302", "E0303", "E0304", "E0305", "E0307", "E0308", "E0309",
    "E0310", "E0311", "E0312", "E0313", "E0314", "E0315", "E0316", "E0317", "E0318", "E1002",
    "E0401", "E0402", "E0403", "E0404", "E0405", "E0406", "E0407", "E0501", "E0502", "E0503",
    "E0601", "E0602", "E0603", "E0604", "E0901", "E0902", "E1001", "E1401",
    "E0701", "E0702", "E0703", "E0801", "E0802",
]
# Ghi chú (N) của M7: docs/errors/<mã>.md với >=3 mẫu `pass` (mỗi mẫu PHẢI phát đúng ghi chú đó: kiểm bằng gt-oblig/tests/corpus.rs).
NOTE_DOC_REQUIRED = [
    "N0601", "N0602", "N0603", "N0604", "N0605", "N0901", "N0902", "N0903", "N0904", "N0905", "N1401",
]
# Cảnh báo VÀNG (S) của M9: docs/errors/<mã>.md với >=3 mẫu `warn` (đúng "warn <mã>") và >=3 mẫu `pass`.
S_DOC_REQUIRED = ["S101", "S102", "S201", "S202"]
DOC_LINE_RE = re.compile(r"^- (fail|warn|pass): (\S+)\s*$")

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


def case_dirs():
    """Ca nhiều tệp (ADR-017b): thư mục `tests/{pass,fail,warn}/<ca>/` có các `.gt` và một tệp `expect`."""
    for kind in ("pass", "fail", "warn"):
        d = os.path.join(TESTS, kind)
        if not os.path.isdir(d):
            continue
        for name in sorted(os.listdir(d)):
            if os.path.isdir(os.path.join(d, name)):
                yield kind, name, os.path.join(d, name)


def check_case_dir(kind, name, path, codes):
    """Trả về số tệp .gt đã kiểm."""
    rel = f"tests/{kind}/{name}/"
    exp_path = os.path.join(path, "expect")
    if not os.path.isfile(exp_path):
        err(f"{rel}: thiếu tệp 'expect'")
        return 0
    with open(exp_path, encoding="utf-8") as f:
        lines = [ln for ln in f.read().split("\n") if ln.strip()]
    exp_lines = [ln for ln in lines if ln.startswith("expect:")]
    others = [ln for ln in lines if not ln.startswith("expect:") and not ln.startswith("note:")]
    if len(exp_lines) != 1 or others:
        err(f"{rel}expect: cần đúng 1 dòng 'expect:' (và tuỳ chọn dòng 'note:'), không có dòng khác")
        return 0
    m = CASE_EXPECT_RE.match(exp_lines[0])
    if not m:
        err(f"{rel}expect: sai định dạng: {exp_lines[0]!r}")
        return 0
    exp = m.group(1)
    word = exp.split()[0]
    if word != DIR_KIND[kind]:
        err(f"{rel}: expect '{word}' nhưng nằm trong thư mục '{kind}'")
    if word in ("fail", "warn"):
        code = exp.split()[1]
        if code not in codes:
            err(f"{rel}: mã {code} không có trong docs/spec/error_codes.md")
        if not name.startswith(code + "_"):
            err(f"{rel}: tên thư mục phải bắt đầu bằng '{code}_'")
    gts = sorted(n for n in os.listdir(path) if n.endswith(".gt"))
    extra = sorted(n for n in os.listdir(path) if not n.endswith(".gt") and n != "expect")
    if extra:
        err(f"{rel}: tệp lạ {extra} (chỉ cho phép *.gt và expect)")
    if len(gts) < 2:
        err(f"{rel}: ca nhiều tệp cần >=2 tệp .gt, có {len(gts)}")
    folded = {}
    for g in gts:
        stem = g[:-3]
        if not IDENT_RE.match(stem):
            err(f"{rel}{g}: tên tệp (tên module) phải là ident hợp lệ")
        if stem.casefold() in folded:
            err(f"{rel}: '{g}' trùng '{folded[stem.casefold()]}' khi bỏ phân biệt hoa/thường")
        folded[stem.casefold()] = g
        try:
            with open(os.path.join(path, g), "rb") as f:
                text = f.read().decode("utf-8")
        except UnicodeDecodeError:
            err(f"{rel}{g}: không phải UTF-8 hợp lệ")
            continue
        if any(ln.startswith("// expect:") for ln in text.split("\n")):
            err(f"{rel}{g}: tệp trong ca nhiều tệp không được có dòng '// expect:' (kỳ vọng nằm ở tệp 'expect')")
        check_unicode(g, text, rel + g)
    return len(gts)


def read_expect_of(ref):
    """Kỳ vọng của một tham chiếu test (đường dẫn tương đối `tests/`): tệp .gt hoặc thư mục ca. None nếu không có."""
    base = os.path.join(TESTS, *ref.split("/"))
    if os.path.isdir(base):
        f = os.path.join(base, "expect")
        if not os.path.isfile(f):
            return None
        with open(f, encoding="utf-8") as fh:
            for ln in fh.read().split("\n"):
                if ln.startswith("expect:"):
                    return ln[len("expect:"):].strip()
        return None
    if os.path.isfile(base + ".gt"):
        with open(base + ".gt", encoding="utf-8") as fh:
            first = fh.readline().strip()
        if first.startswith("// expect:"):
            return first[len("// expect:"):].strip()
    return None


def check_error_docs():
    """docs/errors/<mã>.md cho các mã đã triển khai: có tệp, >=3 `- fail:` và >=3 `- pass:`, mọi tham chiếu tồn tại và đúng kỳ vọng."""
    for code in DOC_REQUIRED + S_DOC_REQUIRED:
        path = os.path.join(ROOT, "docs", "errors", code + ".md")
        rel = f"docs/errors/{code}.md"
        if not os.path.isfile(path):
            err(f"{rel}: thiếu tài liệu mã lỗi")
            continue
        with open(path, encoding="utf-8") as f:
            lines = f.read().split("\n")
        found = {"fail": [], "warn": [], "pass": []}
        for ln in lines:
            m = DOC_LINE_RE.match(ln)
            if m:
                found[m.group(1)].append(m.group(2))
        kinds = ("warn", "pass") if code.startswith("S") else ("fail", "pass")
        for kind in kinds:
            if len(found[kind]) < 3:
                err(f"{rel}: cần >=3 dòng '- {kind}: <tham chiếu>' (plan §0.5), có {len(found[kind])}")
            for ref in found[kind]:
                exp = read_expect_of(ref)
                want = f"{kind} {code}" if kind in ("fail", "warn") else "pass"
                if exp is None:
                    err(f"{rel}: tham chiếu '{ref}' không tồn tại hoặc thiếu dòng expect")
                elif exp != want:
                    err(f"{rel}: '{ref}' có expect '{exp}', cần '{want}'")


def check_note_docs():
    """docs/errors/<N-mã>.md: có tệp, >=3 `- pass:` tồn tại và đúng kỳ vọng `pass`."""
    for code in NOTE_DOC_REQUIRED:
        path = os.path.join(ROOT, "docs", "errors", code + ".md")
        rel = f"docs/errors/{code}.md"
        if not os.path.isfile(path):
            err(f"{rel}: thiếu tài liệu ghi chú")
            continue
        with open(path, encoding="utf-8") as f:
            refs = [m.group(2) for m in (DOC_LINE_RE.match(ln) for ln in f.read().split("\n")) if m and m.group(1) == "pass"]
        if len(refs) < 3:
            err(f"{rel}: cần >=3 dòng '- pass: <tham chiếu>', có {len(refs)}")
        for ref in refs:
            if read_expect_of(ref) != "pass":
                err(f"{rel}: tham chiếu '{ref}' không tồn tại hoặc không phải 'expect: pass'")


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

    multi_files = 0
    for kind, name, path in case_dirs():
        seen_files[kind].add(name)
        total += 0
        multi_files += check_case_dir(kind, name, path, codes)
    total += multi_files
    check_error_docs()
    check_note_docs()

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
    # file fail/warn không được mồ côi: liệt kê trong bảng, hoặc (cho mã có docs/errors, corpus lớn như M6) trong
    # docs/errors/<mã>.md với dòng `- fail: fail/<tên>` — bảng không chứa nổi hàng trăm mẫu
    documented = set()
    for code in DOC_REQUIRED:
        path = os.path.join(ROOT, "docs", "errors", code + ".md")
        if os.path.isfile(path):
            with open(path, encoding="utf-8") as f:
                for ln in f.read().split("\n"):
                    m = DOC_LINE_RE.match(ln)
                    if m and m.group(1) == "fail" and m.group(2).startswith("fail/"):
                        documented.add(("fail", m.group(2)[5:]))
    for sub in ("fail", "warn"):
        for base in sorted(seen_files[sub]):
            if (sub, base) not in cited and (sub, base) not in documented:
                err(f"tests/{sub}/{base}.gt không được liệt kê trong error_codes.md (hay docs/errors/<mã>.md)")

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
