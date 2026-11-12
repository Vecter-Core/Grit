#!/usr/bin/env python3
"""Chạy `gritc check --json` (bản dựng THẬT, qua dòng lệnh) trên toàn bộ corpus `tests/{pass,fail,warn}` và đối chiếu `expect`.

Khác với `tools/check_expectations.py` (chỉ kiểm tính nhất quán của corpus) và với test trong crate (gọi API Rust): đây là bằng chứng
đen-hộp qua đúng lệnh người dùng gõ. Không ghi gì vào `tests/`.

Dùng: corpus_cli.py --gritc <đường dẫn gritc> [--tests <thư mục tests>] [--jobs 4]
Mã thoát: 0 = mọi ca khớp; 1 = có ca lệch.
"""
import argparse
import json
import os
import re
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor

HDR = re.compile(r"^// expect: (pass|fail E\d{4}|warn S\d{3})\s*$", re.M)
CASE = re.compile(r"^expect: (pass|fail E\d{4}|warn S\d{3})\s*$", re.M)


def collect(tests):
    out = []
    for kind in ("pass", "fail", "warn"):
        d = os.path.join(tests, kind)
        if not os.path.isdir(d):
            continue
        for name in sorted(os.listdir(d)):
            p = os.path.join(d, name)
            if os.path.isfile(p) and name.endswith(".gt"):
                m = HDR.search(open(p, encoding="utf-8").read())
                out.append((kind, name, p, m.group(1) if m else None))
            elif os.path.isdir(p):
                ef = os.path.join(p, "expect")
                m = CASE.search(open(ef, encoding="utf-8").read()) if os.path.exists(ef) else None
                out.append((kind, name + "/", p, m.group(1) if m else None))
    return out


def run_one(gritc, case):
    kind, name, path, expect = case
    import re as _re, shutil, tempfile
    tmpdir = None
    if os.path.isfile(path):
        # tên tệp là tên module (phải là định danh hợp lệ): mẫu như `01_add.gt` bị `BadModuleName` nếu đưa thẳng ⇒ chép sang tên hợp lệ
        tmpdir = tempfile.mkdtemp(prefix="corp_")
        # test trong crate nạp mẫu đơn dưới tên module `main` (mẫu E02xx dùng `main::...`); tên tệp thật như `01_add.gt` còn bị `BadModuleName`
        safe = "main.gt"
        dst = os.path.join(tmpdir, safe)
        shutil.copyfile(path, dst)
        path = dst
    try:
        r = subprocess.run(
            [gritc, "check", path, "--json"],
            capture_output=True, text=True, timeout=120,
        )
    except subprocess.TimeoutExpired:
        return case, "TIMEOUT", [], None
    finally:
        if tmpdir:
            shutil.rmtree(tmpdir, ignore_errors=True)
    try:
        j = json.loads(r.stdout)
    except Exception:
        return case, f"NOJSON(exit={r.returncode})", [], r.stderr[-200:]
    codes = [d.get("code", "?") for d in j.get("diagnostics", [])]
    return case, j.get("verdict", "?"), codes, r.returncode


def judge(expect, verdict, codes, rc):
    errs = sorted({c for c in codes if c.startswith("E")})
    warns = sorted({c for c in codes if c.startswith("S")})
    if expect is None:
        return "NO_EXPECT", ""
    if expect == "pass":
        if verdict == "green" and rc == 0:
            return "OK", ""
        return "MISMATCH", f"muốn pass, được verdict={verdict} exit={rc} lỗi={errs} vàng={warns}"
    k, code = expect.split()
    if k == "fail":
        if code in errs and rc in (1, 2):
            extra = [e for e in errs if e != code]
            return ("OK_EXTRA", f"có thêm mã {extra}") if extra else ("OK", "")
        return "MISMATCH", f"muốn fail {code}, được verdict={verdict} exit={rc} lỗi={errs}"
    if k == "warn":
        if code in warns and verdict == "yellow" and rc == 0:
            return "OK", ""
        return "MISMATCH", f"muốn warn {code}, được verdict={verdict} exit={rc} vàng={warns} lỗi={errs}"
    return "NO_EXPECT", expect


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--gritc", required=True)
    ap.add_argument("--tests", default=os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "tests"))
    ap.add_argument("--jobs", type=int, default=4)
    a = ap.parse_args()

    cs = collect(a.tests)
    stats = {}
    mism = []
    extra = []
    with ThreadPoolExecutor(a.jobs) as ex:
        for case, verdict, codes, rc in ex.map(lambda c: run_one(a.gritc, c), cs):
            kind, name, path, expect = case
            st, why = judge(expect, verdict, codes, rc if isinstance(rc, int) else -1)
            if verdict.startswith(("NOJSON", "TIMEOUT")):
                st, why = "MISMATCH", f"{verdict} {rc if rc else ''}"
            key = f"{kind}:{st}"
            stats[key] = stats.get(key, 0) + 1
            if st == "MISMATCH" or st == "NO_EXPECT":
                mism.append(f"{kind}/{name}  muốn={expect}  {why}")
            elif st == "OK_EXTRA":
                extra.append(f"{kind}/{name}  muốn={expect}  {why}")
    total = len(cs)
    ok = sum(v for k, v in stats.items() if k.endswith(":OK") or k.endswith(":OK_EXTRA"))
    print(f"SUMMARY: corpus qua dòng lệnh gritc: {total} ca; khớp {ok}; lệch {len(mism)}")
    for k in sorted(stats):
        print(f"SUMMARY: {k} = {stats[k]}")
    if extra:
        print(f"SUMMARY: {len(extra)} ca fail phát thêm mã E ngoài mã kỳ vọng (thông tin)")
        for e in extra[:40]:
            print("EXTRA", e)
    for m in mism:
        print("MISMATCH", m)
    return 0 if not mism else 1


if __name__ == "__main__":
    sys.exit(main())
