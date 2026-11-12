#!/usr/bin/env python3
"""Kiểm METAMORPHIC cho `gritc fmt` (GRIT_PLAN §9): định dạng KHÔNG được đổi phán quyết.
Với mỗi mẫu đơn trong tests/{pass,fail,warn}: chép sang thư mục tạm dưới tên `main.gt`, ghi nhận (verdict, tập mã) của `gritc check`, chạy `gritc fmt`
(ghi tại chỗ trên BẢN CHÉP), rồi `check` lại. Phán quyết hoặc tập mã khác nhau ⇒ fmt đã đổi ngữ nghĩa (hoặc làm mất/đổi chẩn đoán).
Mẫu bị fmt từ chối (nguồn hỏng cú pháp) được đếm riêng. Không ghi gì vào tests/. Dùng: fmt_meta.py --gritc G [--tests DIR]
"""
import argparse
import glob
import json
import os
import shutil
import subprocess
import sys
import tempfile


def check(g, path):
    r = subprocess.run([g, "check", path, "--json"], capture_output=True, text=True, timeout=120)
    try:
        j = json.loads(r.stdout)
    except Exception:
        return ("NOJSON", r.returncode)
    return (j.get("verdict"), tuple(sorted(d.get("code") for d in j.get("diagnostics", []))))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--gritc", required=True)
    ap.add_argument("--tests", default=os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "tests"))
    a = ap.parse_args()
    n = refused = same = 0
    bad = []
    tmp = tempfile.mkdtemp(prefix="fmtmeta_")
    for kind in ("pass", "fail", "warn"):
        for f in sorted(glob.glob(os.path.join(a.tests, kind, "*.gt"))):
            n += 1
            d = os.path.join(tmp, "c")
            shutil.rmtree(d, ignore_errors=True)
            os.makedirs(d)
            p = os.path.join(d, "main.gt")
            shutil.copyfile(f, p)
            before = check(a.gritc, p)
            r = subprocess.run([a.gritc, "fmt", p], capture_output=True, text=True, timeout=120)
            if r.returncode != 0:
                refused += 1
                continue
            after = check(a.gritc, p)
            if before == after:
                same += 1
            else:
                bad.append((os.path.basename(f), before, after))
    print(f"SUMMARY: fmt metamorphic: {n} mẫu; phán quyết không đổi {same}; fmt từ chối {refused}; ĐỔI {len(bad)}")
    for b in bad[:30]:
        print("ĐỔI", b)
    shutil.rmtree(tmp, ignore_errors=True)
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
