#!/usr/bin/env python3
"""GÓI 8 S0d: cổng HỒI QUY cho chương trình tấn công đã nhập (tests/adversarial/{pass,fail}).

Mỗi `.gt` có đầu `// expect: pass` hoặc `// expect: fail Exxxx`. `gritc check` phải cho đúng: pass ⇒ xanh/vàng (không đỏ, không ICE);
fail Exxxx ⇒ đỏ và có mã Exxxx. Sai ⇒ thoát 1. Thiếu đầu expect ⇒ lỗi.
Dùng: python tools/adversarial_check.py [--gritc ĐƯỜNG_DẪN_TUYỆT_ĐỐI] [--dir THƯ_MỤC] [--timeout 30]
Kiểm tự thân: tools/test_adversarial_check.py (kho tạm, có ca dương và âm).
"""
import json
import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
EXPECT = re.compile(r"^//\s*expect:\s*(pass|fail)(?:\s+(E\d{4}))?", re.M)


def run_check(gritc, path, tmo):
    try:
        p = subprocess.run([gritc, "check", "--json", path], capture_output=True, text=True,
                           encoding="utf-8", errors="replace", timeout=tmo)
    except subprocess.TimeoutExpired:
        return None, [], "timeout"
    try:
        d = json.loads(p.stdout)
    except ValueError:
        return None, [], "stdout không phải JSON"
    codes = sorted({x.get("code", "") for x in d.get("diagnostics", []) if x.get("code")})
    return d.get("verdict"), codes, ""


def judge(want, code, verdict, codes):
    """Trả None nếu đúng, chuỗi lý do nếu sai."""
    if verdict is None:
        return "không chạy được gritc"
    if any(c.startswith("E9") for c in codes):
        return "ICE " + ",".join(c for c in codes if c.startswith("E9"))
    if want == "pass":
        return None if verdict in ("green", "yellow") else "mong đợi chấp nhận, ra %s (%s)" % (verdict, ",".join(codes))
    if verdict != "red":
        return "mong đợi %s (từ chối), ra %s" % (code, verdict)
    if code and code not in codes:
        return "mong đợi %s, ra %s" % (code, ",".join(codes))
    return None


def _default_gritc():
    """bản dựng debug (cổng `cargo test` dựng gritc debug) nếu có; không thì release."""
    for rel in (("debug", "gritc.exe"), ("debug", "gritc"), ("release", "gritc.exe"), ("release", "gritc")):
        p = os.path.join(ROOT, "target", *rel)
        if os.path.isfile(p):
            return p
    return os.path.join(ROOT, "target", "debug", "gritc.exe")


def main(argv):
    gritc = argv[argv.index("--gritc") + 1] if "--gritc" in argv else _default_gritc()
    d = argv[argv.index("--dir") + 1] if "--dir" in argv else os.path.join(ROOT, "tests", "adversarial")
    tmo = int(argv[argv.index("--timeout") + 1]) if "--timeout" in argv else 30
    bad, total = [], 0
    for sub in ("pass", "fail"):
        base = os.path.join(d, sub)
        if not os.path.isdir(base):
            continue
        for name in sorted(os.listdir(base)):
            if not name.endswith(".gt"):
                continue
            total += 1
            path = os.path.join(base, name)
            with open(path, encoding="utf-8") as f:
                m = EXPECT.search(f.read())
            if not m:
                bad.append("%s/%s: thiếu // expect:" % (sub, name))
                continue
            verdict, codes, note = run_check(gritc, path, tmo)
            why = note or judge(m.group(1), m.group(2), verdict, codes)
            if why:
                bad.append("%s/%s: %s" % (sub, name, why))
    print("ADVERSARIAL: %d chương trình, %d sai" % (total, len(bad)))
    for b in bad:
        print("  SAI", b)
    return 1 if bad else 0


if __name__ == "__main__":
    sys.stdout.reconfigure(encoding="utf-8")
    sys.exit(main(sys.argv[1:]))
