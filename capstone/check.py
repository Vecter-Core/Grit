#!/usr/bin/env python3
"""So một chương trình Grit (đã dựng thành tệp thực thi) với bản tham chiếu Rust trên bộ ca của `gen_cases.py`.
Dùng: check.py --k csvstat|calc|kvstore|grep-lite --cand <exe> [--ref <exe>] [--cases <thư-mục>] [--large] [--timeout 30] [--only <tên,tên>] [--json <tệp>]
So: mã thoát, stdout, stderr (nguyên văn) và nội dung mọi tệp trong thư mục ca sau khi chạy. Mỗi bên chạy trong bản sao riêng của ca (`@DIR@` → thư mục bản sao).
Thoát 0 = mọi ca khớp; 1 = có ca lệch/hết giờ; 3 = dùng sai."""
import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import gen_cases  # noqa: E402


def snapshot(d):
    out = {}
    for root, _, files in os.walk(d):
        for f in files:
            p = os.path.join(root, f)
            rel = os.path.relpath(p, d)
            if rel in ("args", "stdin"):
                continue
            with open(p, "rb") as fh:
                out[rel] = fh.read()
    return out


def run_one(exe, case_dir, work, timeout):
    d = os.path.join(work, "run")
    shutil.rmtree(d, ignore_errors=True)
    shutil.copytree(case_dir, d)
    args = [a.replace("@DIR@", d) for a in open(os.path.join(case_dir, "args"), encoding="utf-8").read().split("\n")[:-1]]
    stdin_path = os.path.join(case_dir, "stdin")
    stdin = open(stdin_path, "rb").read() if os.path.exists(stdin_path) else b""
    try:
        p = subprocess.run([exe, *args], input=stdin, capture_output=True, timeout=timeout)
        res = {"code": p.returncode, "stdout": p.stdout, "stderr": p.stderr.replace(d.encode(), b"@DIR@")}
    except subprocess.TimeoutExpired:
        res = {"code": "TIMEOUT", "stdout": b"", "stderr": b""}
    snap = snapshot(d)
    res["files"] = {k: v for k, v in snap.items()}
    # đường dẫn thư mục bản sao khác nhau giữa hai bên: chuẩn hoá trong stdout
    res["stdout"] = res["stdout"].replace(d.encode(), b"@DIR@")
    return res


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--k", required=True, choices=["csvstat", "calc", "kvstore", "grep-lite"])
    ap.add_argument("--cand", required=True)
    ap.add_argument("--ref")
    ap.add_argument("--cases")
    ap.add_argument("--large", action="store_true")
    ap.add_argument("--timeout", type=float, default=30)
    ap.add_argument("--only")
    ap.add_argument("--json")
    a = ap.parse_args()
    ref = a.ref or os.path.join(HERE, "reference", "target", "release", a.k)
    if not os.path.exists(ref):
        print(f"check: thiếu bản tham chiếu {ref} (cd capstone/reference && cargo build --release)")
        return 3
    tmp = tempfile.mkdtemp(prefix="capstone_")
    cases = a.cases or os.path.join(tmp, "cases")
    if not a.cases:
        gen_cases.main(["gen", cases] + (["--large"] if a.large else []))
    names = sorted(os.listdir(os.path.join(cases, a.k)))
    if a.only:
        names = [n for n in names if n in a.only.split(",")]
    bad = []
    rows = []
    for n in names:
        cdir = os.path.join(cases, a.k, n)
        r = run_one(ref, cdir, os.path.join(tmp, "r"), a.timeout)
        c = run_one(a.cand, cdir, os.path.join(tmp, "c"), a.timeout)
        same = r == c
        rows.append({"k": a.k, "case": n, "ok": same, "ref_code": r["code"], "cand_code": c["code"]})
        if not same:
            diff = []
            for key in ("code", "stdout", "stderr", "files"):
                if r[key] != c[key]:
                    if key in ("stdout", "stderr"):
                        diff.append(f"{key}: ref={r[key][:160]!r} cand={c[key][:160]!r}")
                    else:
                        diff.append(f"{key}: ref={str(r[key])[:160]} cand={str(c[key])[:160]}")
            bad.append((n, diff))
    for n, diff in bad[:20]:
        print(f"LỆCH {a.k}/{n}")
        for d in diff:
            print("   ", d)
    print(f"CAPSTONE {a.k}: {len(names)} ca; khớp {len(names) - len(bad)}; LỆCH {len(bad)}")
    if a.json:
        with open(a.json, "a", encoding="utf-8") as f:
            for r in rows:
                f.write(json.dumps(r, ensure_ascii=False) + "\n")
    shutil.rmtree(tmp, ignore_errors=True)
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
