#!/usr/bin/env python3
"""Kiểm MỌI khối mã Grit trong tài liệu (Seed 1.0 S3g): tài liệu mà mã trong đó không biên dịch được là tài liệu SAI ⇒ cổng ĐỎ.

Chỉ các hàng rào có ngôn ngữ `grit` được kiểm (hàng rào trần ``` hoặc `text`/`sh` thì KHÔNG: dùng cho đoạn trích không phải chương trình đầy đủ).
Cú pháp dòng mở: ```grit [thuộc-tính ...]
  (không thuộc tính)   chương trình đầy đủ (tệp `main.gt`): `gritc check` phải XANH (VÀNG/ghi chú chấp nhận được; ĐỎ/ICE thì ĐỎ)
  std                  thêm gói `std` (lib/std/src/*.gt) vào đầu vào
  fail=E0xxx[,E0yyy]   chương trình PHẢI bị từ chối với (mọi) mã đó (ví dụ minh hoạ lỗi)
  run                  sau khi check, `gritc run` phải thoát 0; nếu ngay sau khối là hàng rào ```output thì stdout phải KHỚP đúng từng byte
  value=N              `gritc run` phải cho `main` trả đúng N (số nguyên)
  exit=N               chương trình phải thoát bằng `core::exit(N)` (gritc run báo mã 100+N); mặc định 0
  stdin=<tệp-tuỳ-chọn> (không dùng)
  file=ten.gt group=G  nhiều khối cùng `group` ghép thành MỘT chương trình nhiều tệp (mỗi khối một tệp `file=`; khối thiếu `file=` là main.gt)
Dùng: python3 tools/doc_test.py [--list] [đường-dẫn.md ...]   (mặc định: mọi *.md dưới docs/ và README.md)
Mã thoát: 0 = mọi khối đạt; 1 = có khối sai; 2 = thiếu gritc / dùng sai."""
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
FENCE = re.compile(r"^(\s*)```(\S*)\s*(.*)$")


def find_gritc():
    env = os.environ.get("GRITC")
    if env:
        return env
    exe = "gritc.exe" if os.name == "nt" else "gritc"
    cands = []
    td = os.environ.get("CARGO_TARGET_DIR")
    if td:
        cands += [os.path.join(td, "release", exe), os.path.join(td, "debug", exe)]
    cands += [os.path.join(ROOT, "target", "release", exe), os.path.join(ROOT, "target", "debug", exe)]
    for c in cands:
        if os.path.isfile(c):
            return c
    r = subprocess.run(["cargo", "build", "-q", "-p", "gt-driver", "--bin", "gritc"], cwd=ROOT)
    if r.returncode != 0:
        return None
    for c in cands:
        if os.path.isfile(c):
            return c
    return None


def parse_blocks(path):
    """[(dòng-bắt-đầu, thuộc-tính dict, mã, đầu ra mong đợi hoặc None)]"""
    with open(path, encoding="utf-8") as fh:
        text = fh.read().replace("\r\n", "\n")
    lines = text.split("\n")
    blocks = []
    i = 0
    while i < len(lines):
        m = FENCE.match(lines[i])
        if not m:
            i += 1
            continue
        lang, rest = m.group(2), m.group(3)
        j = i + 1
        body = []
        while j < len(lines) and not FENCE.match(lines[j]):
            body.append(lines[j])
            j += 1
        if lang == "grit":
            attrs = {}
            for tok in rest.split():
                if "=" in tok:
                    k, v = tok.split("=", 1)
                    attrs[k] = v
                else:
                    attrs[tok] = True
            out = None
            k = j + 1
            while k < len(lines) and lines[k].strip() == "":
                k += 1
            if k < len(lines):
                mo = FENCE.match(lines[k])
                if mo and mo.group(2) == "output":
                    o = []
                    k += 1
                    while k < len(lines) and not FENCE.match(lines[k]):
                        o.append(lines[k])
                        k += 1
                    out = "\n".join(o) + ("\n" if o else "")
            blocks.append((i + 1, attrs, "\n".join(body) + "\n", out))
        i = j + 1
    return blocks


def std_sources():
    d = os.path.join(ROOT, "lib", "std", "src")
    out = {}
    for f in sorted(os.listdir(d)):
        if f.endswith(".gt"):
            with open(os.path.join(d, f), encoding="utf-8") as fh:
                out[f] = fh.read()
    return out


def run_gritc(gritc, args, cwd):
    env = dict(os.environ)
    return subprocess.run([gritc] + args, cwd=cwd, capture_output=True, text=True, encoding="utf-8", env=env, timeout=300, stdin=subprocess.DEVNULL)


def diag_codes(stdout):
    for line in reversed(stdout.splitlines()):
        if line.startswith("{"):
            try:
                d = json.loads(line)
            except ValueError:
                continue
            return d, [x.get("code") for x in d.get("diagnostics", [])]
    return None, []


def check_program(gritc, files, attrs, expect_out):
    """files: {tên: nội dung}; trả danh sách lỗi (chuỗi); rỗng = đạt."""
    errs = []
    d = tempfile.mkdtemp(prefix="doctest-")
    try:
        for name, text in files.items():
            with open(os.path.join(d, name), "w", encoding="utf-8", newline="\n") as f:
                f.write(text)
        r = run_gritc(gritc, ["check", "--json", d], d)
        doc, codes = diag_codes(r.stdout)
        want_fail = attrs.get("fail")
        if want_fail:
            wanted = [c for c in str(want_fail).split(",") if c]
            if r.returncode == 0:
                errs.append(f"mong đợi lỗi {wanted} nhưng gritc check XANH")
            for c in wanted:
                if c not in codes:
                    errs.append(f"thiếu mã lỗi {c} (có: {sorted(set(x for x in codes if x))})")
            return errs
        if r.returncode != 0:
            shown = [f"{x.get('code')}: {x.get('message')}" for x in (doc or {}).get("diagnostics", []) if str(x.get("code", "")).startswith("E")][:4]
            errs.append(f"gritc check thoát {r.returncode}: {shown or (r.stderr or r.stdout)[-300:]}")
            return errs
        if "run" in attrs or "value" in attrs:
            r2 = run_gritc(gritc, ["run", "--json", d], d)
            want_rc = 0 if str(attrs.get("exit", "0")) == "0" else 100 + int(attrs["exit"])
            if r2.returncode != want_rc:
                errs.append(f"gritc run thoát {r2.returncode} (mong đợi {want_rc}): {(r2.stderr or r2.stdout)[-300:]}")
                return errs
            doc2, _ = diag_codes(r2.stdout)
            if "value" in attrs:
                raw = ((doc2 or {}).get("run") or {}).get("value", "")
                m = re.match(r"Int\((-?\d+)", raw or "")
                got = m.group(1) if m else raw
                if got != str(attrs["value"]):
                    errs.append(f"value={attrs['value']} nhưng main trả {got!r}")
            if expect_out is not None:
                # stdout của chương trình = phần trước dòng JSON cuối
                lines = r2.stdout.splitlines(keepends=True)
                idx = max((i for i, l in enumerate(lines) if l.startswith("{")), default=len(lines))
                got = "".join(lines[:idx])
                if got != expect_out:
                    errs.append(f"stdout lệch: mong đợi {expect_out!r}, có {got!r}")
    finally:
        shutil.rmtree(d, ignore_errors=True)
    return errs


def collect(paths):
    out = []
    for p in paths:
        if os.path.isdir(p):
            for root, _, files in os.walk(p):
                for f in sorted(files):
                    if f.endswith(".md"):
                        out.append(os.path.join(root, f))
        else:
            out.append(p)
    return sorted(out)


def main(argv):
    listing = "--list" in argv
    paths = [a for a in argv if not a.startswith("--")]
    if not paths:
        paths = [os.path.join(ROOT, "docs"), os.path.join(ROOT, "README.md")]
    mds = collect(paths)
    gritc = None if listing else find_gritc()
    if not listing and gritc is None:
        print("DOC_TEST: không tìm thấy gritc (đặt GRITC hoặc dựng `cargo build -p gt-driver`)")
        return 2
    std = std_sources()
    total = bad = 0
    groups = {}
    for md in mds:
        rel = os.path.relpath(md, ROOT)
        for line, attrs, code, out in parse_blocks(md):
            total += 1
            label = f"{rel}:{line}"
            if listing:
                print(label, attrs)
                continue
            g = attrs.get("group")
            if g:
                groups.setdefault((rel, g), []).append((line, attrs, code, out))
                continue
            files = {attrs.get("file", "main.gt"): code}
            if attrs.get("std"):
                files.update({k: v for k, v in std.items() if k not in files})
            errs = check_program(gritc, files, attrs, out)
            if errs:
                bad += 1
                print(f"DOC_FAIL {label}: " + "; ".join(errs))
    for (rel, g), blks in sorted(groups.items()):
        files = {}
        attrs = {}
        out = None
        for line, a, code, o in blks:
            files[a.get("file", "main.gt")] = code
            attrs.update({k: v for k, v in a.items() if k not in ("file", "group")})
            out = o if o is not None else out
        if attrs.get("std"):
            files.update({k: v for k, v in std.items() if k not in files})
        errs = check_program(gritc, files, attrs, out)
        if errs:
            bad += 1
            print(f"DOC_FAIL {rel} group={g} (dòng {blks[0][0]}): " + "; ".join(errs))
    if listing:
        print(f"DOC_TEST: {total} khối grit")
        return 0
    print(f"DOC_TEST: {total} khối grit, {bad} sai")
    if total == 0:
        print("DOC_TEST: KHÔNG có khối grit nào trong tài liệu — tài liệu chưa được kiểm (đỏ)")
        return 1
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
