#!/usr/bin/env python3
"""Kiểm cả bốn capstone: dựng capstone/solutions/<k> ở các hồ sơ (mặc định dev và release-o2) rồi so với bản tham chiếu Rust trên toàn bộ bộ ca (+ ca lớn).
Dùng: verify.py [--profiles dev,release-o2] [--k csvstat,calc,kvstore,grep-lite]   (cần: `cargo build --release` trong capstone/reference; GRITC=đường-dẫn-gritc)
Mã thoát: 0 = mọi (chương trình × hồ sơ) khớp; 1 = có lệch; 3 = thiếu công cụ."""
import json
import os
import subprocess
import sys
import tempfile
import time

HERE = os.path.dirname(os.path.abspath(__file__))
PKG = {"csvstat": "csvstat", "calc": "calc", "kvstore": "kvstore", "grep-lite": "grep_lite"}


def main(argv):
    profiles = ["dev", "release-o2"]
    ks = list(PKG)
    for i, a in enumerate(argv):
        if a == "--profiles":
            profiles = argv[i + 1].split(",")
        if a == "--k":
            ks = argv[i + 1].split(",")
    exe = "csvstat"
    bad = 0
    out = tempfile.mkdtemp(prefix="capverify-")
    rows = []
    for k in ks:
        ref = os.path.join(HERE, "reference", "target", "release", k)
        if not os.path.isfile(ref) and os.path.isfile(ref + ".exe"):
            ref += ".exe"  # Windows
        if not os.path.isfile(ref):
            print(f"thiếu bản tham chiếu {ref}: chạy `cargo build --release` trong capstone/reference")
            return 3
        for prof in profiles:
            cand = os.path.join(out, f"{k}-{prof}.exe")
            t0 = time.time()
            r = subprocess.run([sys.executable, os.path.join(HERE, "build_pkg.py"), os.path.join(HERE, "solutions", PKG[k]), prof, cand], capture_output=True, text=True)
            if r.returncode != 0:
                print(f"DỰNG LỖI {k} {prof}: {r.stderr[-500:]}")
                bad += 1
                continue
            bt = time.time() - t0
            r = subprocess.run([sys.executable, os.path.join(HERE, "check.py"), "--k", k, "--large", "--cand", cand, "--ref", ref], capture_output=True, text=True)
            last = r.stdout.strip().splitlines()[-1] if r.stdout.strip() else ""
            ok = r.returncode == 0
            print(f"{'OK ' if ok else 'LỆCH'} {k:10s} {prof:11s} dựng {bt:4.1f}s  {last}")
            rows.append({"k": k, "profile": prof, "ok": ok, "line": last})
            if not ok:
                bad += 1
                print(r.stdout[-1500:])
    print("CAPSTONE_VERIFY", "XANH" if bad == 0 else f"ĐỎ({bad})")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
