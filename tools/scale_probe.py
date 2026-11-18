#!/usr/bin/env python3
"""GÓI 6 S2.a: đo thời gian `gritc check` theo cỡ n của một chương trình sinh tự động. THỦ CÔNG (không nằm trong cổng); mỗi lần chạy có `timeout`, kết quả ghi NGAY vào tệp jsonl sau từng lần.
  python tools/scale_probe.py [--gritc ĐƯỜNG_DẪN_TUYỆT_ĐỐI/gritc] [--out docs/SCALE_PROBE.jsonl] [--sizes 50,100,...] [--families if,loop,match,fns,loopfns] [--timeout 120]
Họ chương trình (n = số cấu trúc): if = n `⎇` liên tiếp trong một hàm; loop = n vòng lặp liên tiếp trong MỘT hàm (vượt 24 ⇒ E0702, xem docs/LIMITS_AUDIT.md);
match = một `⌥` có n nhánh literal; fns = n hàm nhỏ gọi nhau theo chuỗi; loopfns = n hàm, mỗi hàm một vòng lặp, main gọi lần lượt.
Nếu gritc dựng với GRITC_TIMINGS (bản đo tạm), dòng `TIMING <pass> <ms>` trên stderr được cộng dồn vào `stages_ms` (selftest/prelude chạy cùng pass vài lần: đường cơ sở là cỡ nhỏ).
Tiếp tục (resume): bỏ qua (họ, n) đã có trong tệp; sau lần quá hạn đầu tiên của một họ, bỏ cỡ lớn hơn."""
import json
import os
import re
import subprocess
import sys
import tempfile
import time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
NL = chr(10)


def gen(fam, n):
    if fam == "if":
        body = "".join("  ⎇ x > %du32 { x ← x + 1u32; } ⇝ { }" % (i % 1000) + NL for i in range(n))
        return "ƒ main() → u32 ! ∅ {" + NL + "  μx ≔ 0u32;" + NL + body + "  x" + NL + "}" + NL
    if fam == "loop":
        body = "".join(("  μi%d ≔ 0u32;" + NL + "  ⟳ i%d < 3 ↓ 3 - i%d { i%d ← i%d + 1; }" + NL + "  x ← x +%% i%d;" + NL) % ((k,) * 6) for k in range(n))
        return "ƒ main() → u32 ! ∅ {" + NL + "  μx ≔ 0u32;" + NL + body + "  x" + NL + "}" + NL
    if fam == "match":
        arms = "".join("    %du32 ↦ %du32," % (i, i + 1) + NL for i in range(n))
        return "ƒ main() → u32 ! ∅ {" + NL + "  v ≔ 7u32;" + NL + "  ⌥ v {" + NL + arms + "    _ ↦ 0u32" + NL + "  }" + NL + "}" + NL
    if fam == "fns":
        fs = "ƒ f0(x: u32) → u32 ! ∅ { x }" + NL + "".join("ƒ f%d(x: u32) → u32 ! ∅ { f%d(x) +%% 1u32 }" % (i, i - 1) + NL for i in range(1, n))
        return fs + "ƒ main() → u32 ! ∅ { f%d(1u32) }" % (n - 1) + NL
    if fam == "loopfns":
        fs = "".join(("ƒ g%d(x: u32) → u32 ! ∅ {" + NL + "  μi ≔ 0u32;" + NL + "  ⟳ i < 3 ↓ 3 - i { i ← i + 1; }" + NL + "  x +%% i" + NL + "}" + NL) % k for k in range(n))
        calls = "".join("  x ← g%d(x);" % k + NL for k in range(n))
        return fs + "ƒ main() → u32 ! ∅ {" + NL + "  μx ≔ 0u32;" + NL + calls + "  x" + NL + "}" + NL
    raise SystemExit("họ không biết: " + fam)


def main():
    a = sys.argv
    opt = lambda k, d: a[a.index(k) + 1] if k in a else d
    gritc = opt("--gritc", os.path.join(ROOT, "target", "release", "gritc.exe" if os.name == "nt" else "gritc"))
    out = opt("--out", "docs/SCALE_PROBE.jsonl")
    sizes = [int(x) for x in opt("--sizes", "50,100,200,400,800,1600,3200").split(",")]
    fams = opt("--families", "if,loop,match,fns").split(",")
    tmo = int(opt("--timeout", "120"))
    done = set()
    path = os.path.join(ROOT, out)
    if os.path.isfile(path):
        for l in open(path, encoding="utf-8"):
            if l.strip():
                r = json.loads(l)
                done.add((r["family"], r["n"]))
    f = open(path, "a", encoding="utf-8")
    d = tempfile.mkdtemp(prefix="scale-")
    env = dict(os.environ, GRITC_TIMINGS="1")
    for fam in fams:
        for n in sizes:
            if (fam, n) in done:
                continue
            src = os.path.join(d, f"{fam}_{n}.gt")
            open(src, "w", encoding="utf-8").write(gen(fam, n))
            t0 = time.time()
            try:
                p = subprocess.run([gritc, "check", src, "--backend=interpreter"], capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=tmo, env=env)
                rc, err, st = p.returncode, p.stdout + p.stderr, "done"
            except subprocess.TimeoutExpired as e:
                dec = lambda b: b.decode("utf-8", "replace") if isinstance(b, bytes) else (b or "")
                rc, err, st = None, dec(e.stdout) + dec(e.stderr), "timeout"
            dt = time.time() - t0
            stages = {}
            for m in re.finditer(r"TIMING (\S+) (\d+)", err):
                stages[m.group(1)] = stages.get(m.group(1), 0) + int(m.group(2))
            verdict = re.search(r"phán quyết (\w+)", err)
            row = {"family": fam, "n": n, "seconds": round(dt, 2), "status": st, "exit": rc, "verdict": verdict.group(1) if verdict else None, "stages_ms": stages}
            print(json.dumps(row, ensure_ascii=False), flush=True)
            f.write(json.dumps(row, ensure_ascii=False) + NL)
            f.flush()
            if st == "timeout":
                break   # cỡ lớn hơn của cùng họ chắc chắn cũng quá hạn
    f.close()


if __name__ == "__main__":
    main()
