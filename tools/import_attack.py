#!/usr/bin/env python3
"""GÓI 8 S0d: nhập chương trình của phiên tấn công vào tests/adversarial/{pass,fail} (hồi quy vĩnh viễn).

Dùng:  python tools/import_attack.py <thư_mục_tấn_công> [--gritc ĐƯỜNG_DẪN_TUYỆT_ĐỐI] [--dry-run]

Với mỗi `.gt` trong thư mục (không đệ quy): chạy `gritc check --json` (timeout 30 s) và ghi phán quyết + mã lỗi.
  * Đầu tệp có `// expect: fail Exxxx` hoặc `// expect: pass` là ý định của kẻ tấn công.
  * Phán quyết KHỚP ý định ⇒ nhập: fail ⇒ tests/adversarial/fail/<M>_<tên>.gt (có `// expect: fail M`);
    pass ⇒ tests/adversarial/pass/<tên>.gt.
  * Phán quyết KHÔNG khớp ⇒ KHÔNG nhập tự động. Liệt kê để phân tích tay (FALSE NEGATIVE nếu được chấp nhận mà phải từ chối).
  * Lỗi cú pháp (mã E0101/E0102 hoặc không có đầu expect) ⇒ bỏ, liệt kê.
Mục đích: đưa phát hiện vào cổng (tools/adversarial_check.py). KHÔNG sửa tệp gốc; KHÔNG ghi ngoài tests/adversarial.
Thoát 0 luôn khi chạy xong (đây là công cụ nhập, không phải cổng); số liệu in ra để con người xem.
"""
import json
import os
import re
import shutil
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ADV = os.path.join(ROOT, "tests", "adversarial")
EXPECT = re.compile(r"^//\s*expect:\s*(pass|fail)(?:\s+(E\d{4}))?", re.M)
SYNTAX = {"E0101", "E0102"}


def intent(src):
    m = EXPECT.search(src)
    if not m:
        return None, None
    return m.group(1), m.group(2)


def check(gritc, path, tmo=30):
    """Trả (verdict, [mã lỗi], ghi_chú). verdict = green|red|timeout|error."""
    try:
        p = subprocess.run([gritc, "check", "--json", path], capture_output=True, text=True,
                           encoding="utf-8", errors="replace", timeout=tmo)
    except subprocess.TimeoutExpired:
        return "timeout", [], "quá 30 s"
    try:
        d = json.loads(p.stdout)
    except ValueError:
        return "error", [], "stdout không phải JSON (thoát %s)" % p.returncode
    codes = sorted({x.get("code", "") for x in d.get("diagnostics", []) if x.get("severity") == "error" or x.get("code", "").startswith("E")})
    codes = [c for c in codes if c]
    return d.get("verdict", "?"), codes, ""


def main():
    a = sys.argv[1:]
    if not a or a[0].startswith("--"):
        print(__doc__)
        return 2
    src_dir = os.path.abspath(a[0])
    gritc = a[a.index("--gritc") + 1] if "--gritc" in a else os.path.join(ROOT, "target", "release", "gritc.exe")
    dry = "--dry-run" in a
    rows = []
    for name in sorted(os.listdir(src_dir)):
        if not name.endswith(".gt"):
            continue
        path = os.path.join(src_dir, name)
        with open(path, encoding="utf-8") as f:
            src = f.read()
        want, code = intent(src)
        verdict, codes, note = check(gritc, path)
        if want is None:
            rows.append((name, "BỎ", "không có đầu // expect:"))
            continue
        if verdict == "error" or any(c in SYNTAX for c in codes):
            rows.append((name, "BỎ", "lỗi cú pháp của tấn công (%s)" % (",".join(codes) or note)))
            continue
        if want == "fail" and verdict == "red" and (code is None or code in codes):
            dst_dir = os.path.join(ADV, "fail")
            mcode = codes[0] if codes else code
            dst = os.path.join(dst_dir, "%s_%s" % (mcode, name))
            rows.append((name, "NHẬP fail", "%s; ra %s" % (mcode, ",".join(codes))))
        elif want == "pass" and verdict == "green":
            dst = os.path.join(ADV, "pass", name)
            rows.append((name, "NHẬP pass", "green"))
        else:
            rows.append((name, "NGHI VẤN (phân tích tay)", "ý định %s%s, ra %s (%s)" % (want, "" if code is None else " " + code, verdict, ",".join(codes) or note)))
            continue
        if not dry:
            os.makedirs(os.path.dirname(dst), exist_ok=True)
            shutil.copyfile(path, dst)
    kinds = {}
    for _n, k, _ in rows:
        kinds[k] = kinds.get(k, 0) + 1
    for n, k, why in rows:
        print("%-34s %-28s %s" % (n, k, why))
    print("TÓM TẮT:", json.dumps(kinds, ensure_ascii=False), "(dry-run)" if dry else "")
    return 0


if __name__ == "__main__":
    sys.stdout.reconfigure(encoding="utf-8")
    sys.exit(main())
