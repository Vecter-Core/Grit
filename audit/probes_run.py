#!/usr/bin/env python3
"""Chạy các chương trình THỬ TẤN CÔNG tự viết (`audit/probes/*.gt`) qua `gritc check --json` và đối chiếu kỳ vọng ở dòng đầu.

Dòng đầu mỗi tệp:  // probe: reject [E0xxx|E0yyy] | accept | warn Sxxx
  reject        phải bị chặn (verdict red/ice); nếu có mã thì mã đó phải nằm trong tập đã nêu (dấu | = một trong các mã)
  accept        phải qua (green/yellow) — đối chứng dương để chắc rằng probe không bị chặn vì lỗi cú pháp
  warn Sxxx     phải VÀNG đúng mã
Phân loại kết quả:
  OK                  khớp kỳ vọng
  FALSE_NEGATIVE      probe `reject` nhưng gritc CHO QUA  ⇒ lỗ hổng an toàn tiềm tàng (mức nghiêm trọng nhất)
  WRONG_CODE          bị chặn nhưng mã khác kỳ vọng       ⇒ xem lại probe hoặc thông điệp lỗi
  FALSE_POSITIVE      probe `accept` nhưng bị chặn         ⇒ chặn nhầm hoặc probe viết sai
  KNOWN_GAP           probe `reject` được chấp nhận NHƯNG dự án đã ghi giới hạn này (dòng `// known: <nơi ghi>` trong probe)
  ICE                 gritc báo lỗi nội bộ (exit 2)
Dùng: probes_run.py --gritc <gritc> [--dir audit/probes] [-v]
"""
import argparse
import glob
import json
import os
import re
import subprocess
import sys


def run(gritc, path):
    r = subprocess.run([gritc, "check", path, "--json"], capture_output=True, text=True, timeout=120)
    try:
        j = json.loads(r.stdout)
    except Exception:
        return None, [], r.returncode, r.stderr[-300:]
    return j.get("verdict"), [d.get("code") for d in j.get("diagnostics", [])], r.returncode, ""


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--gritc", required=True)
    ap.add_argument("--dir", default=os.path.join(os.path.dirname(os.path.abspath(__file__)), "probes"))
    ap.add_argument("-v", action="store_true")
    a = ap.parse_args()
    rows = []
    for p in sorted(glob.glob(os.path.join(a.dir, "*.gt"))):
        name = os.path.basename(p)[:-3]
        text = open(p, encoding="utf-8").read()
        m = re.match(r"// probe: (.*)", text)
        exp = m.group(1).strip() if m else "?"
        kn = re.search(r"^// known: (.+)$", text, re.M)
        known = kn.group(1).strip() if kn else ""
        verdict, codes, rc, err = run(a.gritc, p)
        ecodes = sorted({c for c in codes if c and c[0] == "E"})
        scodes = sorted({c for c in codes if c and c[0] == "S"})
        if verdict is None:
            st = "ICE" if rc == 2 else f"NOJSON({rc}) {err}"
        elif verdict == "ice":
            st = "ICE"
        elif exp.startswith("reject"):
            if verdict in ("green", "yellow"):
                st = "KNOWN_GAP" if known else "FALSE_NEGATIVE"
            else:
                want = exp[len("reject"):].strip()
                if not want or any(w in ecodes for w in want.split("|")):
                    st = "OK"
                else:
                    st = "WRONG_CODE"
        elif exp == "accept":
            st = "OK" if verdict in ("green", "yellow") else "FALSE_POSITIVE"
        elif exp.startswith("warn"):
            want = exp.split()[1]
            st = "OK" if (verdict == "yellow" and want in scodes) else ("KNOWN_GAP" if known else "WRONG_CODE")
        else:
            st = "BAD_PROBE_HEADER"
        rows.append((name, exp, verdict, ecodes, scodes, st))
    bad = [r for r in rows if r[5] != "OK"]
    cnt = {}
    for r in rows:
        cnt[r[5]] = cnt.get(r[5], 0) + 1
    print(f"SUMMARY: {len(rows)} probe tự viết; " + "; ".join(f"{k}={v}" for k, v in sorted(cnt.items())))
    for r in rows:
        if a.v or r[5] != "OK":
            print(f"{r[5]:15s} {r[0]:45s} kỳ vọng={r[1]:28s} verdict={r[2]} E={r[3]} S={r[4]}")
    # RC: 1 nếu có lỗ hổng MỚI (FALSE_NEGATIVE) hoặc ICE; KNOWN_GAP (đã có trong KNOWN_LIMITATIONS) không làm đỏ
    return 1 if any(r[5] in ("FALSE_NEGATIVE", "ICE") for r in rows) else 0


if __name__ == "__main__":
    sys.exit(main())
