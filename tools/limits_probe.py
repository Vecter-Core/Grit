#!/usr/bin/env python3
"""GÓI 7 S2.b: thí nghiệm "hết ngân sách/giới hạn" cho từng pass. THỦ CÔNG (không nằm trong cổng); mỗi lần chạy có `timeout`; kết quả ghi NGAY vào docs/LIMITS_EXPERIMENTS.jsonl.
  python tools/limits_probe.py --gritc D:/Grit/target/release/gritc.exe [--exp tên,...] [--n 14000] [--timeout 300] [--find-boundary]
Nguyên tắc cần khẳng định: vượt giới hạn thì nghĩa vụ KHÔNG kiểm được lúc chạy (dừng, mượn, hiệu ứng, Send) PHẢI cho ĐỎ hoặc ICE, không bao giờ XANH.
Mỗi thí nghiệm: một hàm lớn (n câu `⎇` liên tiếp ≈ 3 khối MIR mỗi câu) + một đoạn CUỐI chứa lỗi thật của loại nghĩa vụ đó. `ok_*` là đối chứng: cùng hàm lớn nhưng đoạn cuối ĐÚNG."""
import json
import os
import re
import subprocess
import sys
import tempfile
import time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
NL = chr(10)


def big(n, tail, pre="", params=""):
    """hàm f có n câu `⎇` rồi `tail`; `main` gọi f."""
    body = "".join("  ⎇ x > %du32 { x ← x + 1u32; } ⇝ { }" % (i % 1000) + NL for i in range(n))
    return pre + "ƒ f(" + params + ") → u32 ! ∅ {" + NL + "  μx ≔ 0u32;" + NL + body + tail + NL + "  x" + NL + "}" + NL


EXPS = {
    # nghĩa vụ KHÔNG kiểm được lúc chạy: dừng
    "term_bad": lambda n: big(n, "  μz ≔ 0u32; ⟳ z < 5 ↓ 5 - z { }") + "ƒ main() → i32 ! ∅ { f(); 0 }" + NL,
    "term_ok": lambda n: big(n, "  μz ≔ 0u32; ⟳ z < 5 ↓ 5 - z { z ← z + 1; }") + "ƒ main() → i32 ! ∅ { f(); 0 }" + NL,
    # mượn/alias (M6)
    "borrow_bad": lambda n: big(n, "  μq ≔ 1; a ≔ &!q; b ≔ &!q; *a ← 2; *b ← 3;") + "ƒ main() → i32 ! ∅ { f(); 0 }" + NL,
    "borrow_ok": lambda n: big(n, "  μq ≔ 1; a ≔ &!q; *a ← 2;") + "ƒ main() → i32 ! ∅ { f(); 0 }" + NL,
    # hiệu ứng: hàm ! ∅ gọi hàm có io
    "effect_bad": lambda n: big(n, "  io1();", pre="ƒ io1() ! {io} { }" + NL) + "ƒ main() → i32 ! ∅ { f(); 0 }" + NL,
    "effect_ok": lambda n: big(n, "", pre="ƒ io1() ! {io} { }" + NL) + "ƒ main() → i32 ! {io} { f(); io1(); 0 }" + NL,
    # Send (E0501): spawn của Arena
    "send_bad": lambda n: big(n, "  a ≔ Arena::new(); spawn(w, a);", pre="ƒ w(a: Arena) ! ∅ { }" + NL).replace("! ∅ {" + NL + "  μx", "! {mem} {" + NL + "  μx", 1) + "ƒ main() → i32 ! {mem} { f(); 0 }" + NL,
    # nghĩa vụ KIỂM được lúc chạy: chia 0 (giữ check ⇒ chạy lên là bị chặn, không UB). Biến chia KHÔNG chứng minh được vì x phụ thuộc n nhánh.
    "div_checked": lambda n: big(n, "  μd ≔ x - x; x ← x / (d + 0u32);") + "ƒ main() → i32 ! ∅ { f(); 0 }" + NL,
}


def run(gritc, src, tmo, extra=()):
    t0 = time.time()
    env = dict(os.environ, GRIT_NOTES="1")
    try:
        p = subprocess.run([gritc, "check", src, "--backend=interpreter", *extra], capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=tmo, env=env)
        rc, out, st = p.returncode, p.stdout + p.stderr, "done"
    except subprocess.TimeoutExpired:
        rc, out, st = None, "", "timeout"
    return rc, out, st, time.time() - t0


def summarize(out):
    codes = sorted(set(re.findall(r"(?:error|warning|note)\[(\w+)\]", out)))
    v = re.search(r"phán quyết (\w+)", out)
    return v.group(1) if v else None, codes, ("hết ngân sách" in out or "vượt ngân sách" in out)


def main():
    a = sys.argv
    opt = lambda k, d: a[a.index(k) + 1] if k in a else d
    gritc = opt("--gritc", None)
    if not gritc or not os.path.isabs(gritc):
        raise SystemExit("cần --gritc ĐƯỜNG_DẪN_TUYỆT_ĐỐI")
    n = int(opt("--n", "14000"))
    tmo = int(opt("--timeout", "300"))
    names = opt("--exp", ",".join(EXPS)).split(",")
    outp = os.path.join(ROOT, opt("--out", "docs/LIMITS_EXPERIMENTS.jsonl"))
    d = tempfile.mkdtemp(prefix="limits-")
    f = open(outp, "a", encoding="utf-8")
    for name in names:
        src = os.path.join(d, name + ".gt")
        open(src, "w", encoding="utf-8", newline=NL).write(EXPS[name](n))
        rc, out, st, dt = run(gritc, src, tmo)
        verdict, codes, budget = summarize(out)
        row = {"exp": name, "n": n, "status": st, "exit": rc, "verdict": verdict, "codes": codes, "budget_note": budget, "seconds": round(dt, 1)}
        print(json.dumps(row, ensure_ascii=False), flush=True)
        f.write(json.dumps(row, ensure_ascii=False) + NL)
        f.flush()


if __name__ == "__main__":
    main()
