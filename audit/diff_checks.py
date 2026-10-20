#!/usr/bin/env python3
"""Vi sai độc lập: (mặc định) cho bộ giải M7 — chạy cùng chương trình số học ngẫu nhiên với `--checks=all` (giữ mọi check) và `--checks=proven`
(xoá check đã chứng minh) trên thông dịch MIR. Hai chế độ PHẢI cho cùng kết quả (cùng giá trị hoặc cùng loại abort). Khác ⇒ bộ giải đã xoá
một check không chứng minh được (nghiêm trọng nhất: GRIT_PLAN §7 điểm 10, 69 đột biến "xoá thêm check" còn mở).
Bộ sinh này viết riêng cho kiểm toán (không dùng chung mã với oracle của dự án). Dùng: diff_checks.py --gritc G [--n 300] [--seed 1] [--left "…" --right "…"]
Vi sai backend (cần bản gritc có LLVM): --left "--backend=interpreter" --right "--backend=llvm" (so thông dịch MIR với exe native).
"""
import argparse
import json
import os
import random
import subprocess
import sys
import tempfile
from concurrent.futures import ThreadPoolExecutor

TYPES = {"u8": (0, 255), "i8": (-128, 127), "u16": (0, 65535), "i16": (-32768, 32767), "u32": (0, 2**32 - 1), "i32": (-2**31, 2**31 - 1),
         "u64": (0, 2**64 - 1), "i64": (-2**63, 2**63 - 1)}
OPS = ["+", "-", "*", "/", "%"]
CMP = ["<", "≤", ">", "≥", "==", "≠"]


class Gen:
    def __init__(self, rng, ty):
        self.r, self.ty = rng, ty
        lo, hi = TYPES[ty]
        self.lo, self.hi = lo, hi
        self.vars = ["a", "b"]
        self.mut = []
        self.n = 0

    def lit(self):
        lo, hi = self.lo, self.hi
        c = self.r.choice([0, 1, 2, 3, 7, 10, 100, hi, hi - 1, lo, lo + 1, hi // 2, hi // 2 + 1, 127, 128, 255, 256])
        c = max(lo, min(hi, c))
        return f"{c}{self.ty}" if c >= 0 else f"({c}{self.ty})"

    def expr(self, d=0):
        k = self.r.random()
        if d >= 2 or k < 0.35:
            return self.r.choice(self.vars + self.mut) if self.r.random() < 0.7 else self.lit()
        if k < 0.9:
            return f"({self.expr(d + 1)} {self.r.choice(OPS)} {self.expr(d + 1)})"
        return f"({self.expr(d + 1)} {self.r.choice(['+%', '-%', '*%', '+^', '-^'])} {self.expr(d + 1)})"

    def cond(self):
        return f"{self.expr(1)} {self.r.choice(CMP)} {self.expr(1)}"

    def stmts(self, depth=0):
        out = []
        for _ in range(self.r.randint(1, 4)):
            k = self.r.random()
            if k < 0.30 or not self.mut:
                self.n += 1
                v = f"m{self.n}"
                out.append(f"μ{v} ≔ {self.expr()};")
                self.mut.append(v)
            elif k < 0.65:
                out.append(f"{self.r.choice(self.mut)} ← {self.expr()};")
            elif k < 0.85 and depth < 2:
                out.append(f"⎇ {self.cond()} {{ {' '.join(self.stmts(depth + 1))} }} ⇝ {{ {' '.join(self.stmts(depth + 1))} }}")
            elif depth < 1:
                self.n += 1
                i = f"i{self.n}"
                bound = self.r.choice([3, 5, 10])
                self.mut.append(i)
                body = " ".join(self.stmts(depth + 1))
                self.mut.remove(i)
                out.append(f"μ{i} ≔ 0{self.ty}; ⟳ {i} < {bound}{self.ty} ↓ {bound}{self.ty} - {i} {{ {body} {i} ← {i} + 1{self.ty}; }}")
        return out

    def program(self):
        body = " ".join(self.stmts())
        res = self.r.choice(self.mut)
        return f"ƒ f(a: {self.ty}, b: {self.ty}) → {self.ty} ! ∅ {{ {body} {res} }}\n"


class GenArr:
    """Bộ sinh thứ hai: chỉ số mảng/lát (usize) — nơi M7 xoá kiểm tra biên. Tham số a, b: usize."""

    def __init__(self, rng):
        self.r = rng
        self.n = self.r.randint(2, 6)
        self.ty = "usize"
        self.lo, self.hi = 0, 2**64 - 1

    def idx(self, i=None):
        c = self.r.choice(["a", "b", "a % 5", "(a + b) % 4", "a / 2", "(a +% b) % 3", "a - b", "a + 1", "b * 2", "0usize", "1usize", f"{self.n - 1}usize", f"{self.n}usize"])
        if i and self.r.random() < 0.5:
            c = self.r.choice([i, f"{i} + 1", f"{i} / 2", f"{i} * 2", f"({i} + a) % 5"])
        return c

    def cond(self, i=None):
        return f"{self.idx(i)} {self.r.choice(['<', '≤', '>', '≥'])} {self.r.choice(['xs.len()', 'a', 'b', str(self.n) + 'usize', str(self.n - 1) + 'usize'])}"

    def program(self):
        elems = ", ".join(f"{k + 1}usize" for k in range(self.n))
        body = [f"xs ≔ [{elems}];", "μr ≔ 0usize;"]
        for _ in range(self.r.randint(1, 3)):
            k = self.r.random()
            if k < 0.35:
                body.append(f"⎇ {self.cond()} {{ r ← r + xs[{self.idx()}]; }}")
            elif k < 0.6:
                body.append(f"⎇ {self.cond()} {{ r ← r + xs[{self.idx()}]; }} ⇝ {{ r ← r + 1usize; }}")
            elif k < 0.8:
                lo, hi = self.idx(), self.idx()
                body.append(f"⎇ {lo} ≤ {hi} ∧ {hi} ≤ xs.len() {{ s ≔ &xs[{lo}..{hi}]; r ← r + s.len(); ⎇ s.len() > 0 {{ r ← r + s[0]; }} }}")
            else:
                b = self.r.choice([self.n, self.n + 1, 3])
                body.append(f"μi ≔ 0usize; ⟳ i < {b}usize ↓ {b}usize - i {{ r ← r + xs[{self.idx('i')}]; i ← i + 1usize; }}")
        return f"ƒ f(a: usize, b: usize) → usize ! ∅ {{ {' '.join(body)} r }}\n"


def norm_value(v):
    """Thông dịch in `Int(45, I32)`, exe native báo `45`: chuẩn hoá về số nguyên."""
    import re
    if v is None:
        return None
    m = re.search(r"-?\d+", str(v))
    return m.group(0) if m else str(v)


def run(gritc, src, mode, tmp, a, b):
    p = os.path.join(tmp, "main.gt")
    with open(p, "w", encoding="utf-8") as f:
        f.write(src + f"ƒ main() → {src.split('→ ')[1].split(' ')[0]} ! ∅ {{ f({a}, {b}) }}\n")
    try:
        r = subprocess.run([gritc, "run", p, "--json", *mode.split()], capture_output=True, text=True, timeout=120)
    except subprocess.TimeoutExpired:
        return ("TIMEOUT",)
    try:
        j = json.loads(r.stdout)
    except Exception:
        return ("NOJSON", r.returncode)
    if j.get("verdict") not in ("green", "yellow"):
        return ("REJECT", j.get("verdict"), tuple(d.get("code") for d in j.get("diagnostics", []) if str(d.get("code", "")).startswith("E")))
    run_ = j.get("run", {})
    return ("RUN", run_.get("exit_status"), run_.get("abort_kind"), norm_value(run_.get("value")))


def one(args):
    gritc, idx, seed, left, right, kind = args
    rng = random.Random(seed * 1000003 + idx)
    if kind == "arrays":
        g = GenArr(rng)
        ty = "usize"
        src = g.program()
        lo, hi = g.lo, g.hi
        cands = [0, 1, 2, 3, 4, 5, 6, 7, 100, hi, hi - 1]
    else:
        ty = rng.choice(list(TYPES))
        g = Gen(rng, ty)
        src = g.program()
        lo, hi = g.lo, g.hi
        cands = [lo, lo + 1, -1 if lo < 0 else 0, 0, 1, 2, 5, 99, 100, 127, 128, hi // 2, hi - 1, hi]
    out = {"ty": ty, "src": src, "cases": []}
    tmp = tempfile.mkdtemp(prefix="dc_")
    for _ in range(6):
        a = max(lo, min(hi, rng.choice(cands)))
        b = max(lo, min(hi, rng.choice(cands)))
        lit = lambda x: f"{x}{ty}" if x >= 0 else f"({x}{ty})"
        ra = run(gritc, src, left, tmp, lit(a), lit(b))
        if ra[0] == "REJECT":
            out["rejected"] = ra
            break
        rp = run(gritc, src, right, tmp, lit(a), lit(b))
        out["cases"].append((a, b, ra, rp))
        if ra != rp and not (ra[0] == "RUN" and rp[0] == "RUN" and ra[1] == rp[1] and ra[3] == rp[3]):
            out["diff"] = (a, b, ra, rp)
            break
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--gritc", required=True)
    ap.add_argument("--n", type=int, default=300)
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--jobs", type=int, default=2)
    ap.add_argument("--gen", default="arith", choices=["arith", "arrays"], help="bộ sinh: số học hoặc chỉ số mảng/lát")
    ap.add_argument("--left", default="--backend=interpreter --checks=all", help="đối số của bên trái (mặc định giữ mọi check)")
    ap.add_argument("--right", default="--backend=interpreter --checks=proven", help="đối số của bên phải")
    a = ap.parse_args()
    stats = {"programs": 0, "rejected": 0, "runs": 0, "aborts": 0, "diffs": 0, "ice": 0}
    diffs = []
    with ThreadPoolExecutor(a.jobs) as ex:
        for res in ex.map(one, [(a.gritc, i, a.seed, a.left, a.right, a.gen) for i in range(a.n)]):
            stats["programs"] += 1
            if "rejected" in res:
                stats["rejected"] += 1
                continue
            for _, _, ra, rp in res["cases"]:
                stats["runs"] += 1
                if ra[0] == "RUN" and ra[2]:
                    stats["aborts"] += 1
                if ra[0] == "RUN" and ra[1] == 2:
                    stats["ice"] += 1
            if "diff" in res:
                stats["diffs"] += 1
                diffs.append(res)
    print(f"SUMMARY: vi sai [{a.left}] ≡ [{a.right}]: " + "; ".join(f"{k}={v}" for k, v in stats.items()) + f"; seed={a.seed}")
    for d in diffs[:10]:
        print("DIFF", d["ty"], d["diff"])
        print(d["src"])
    return 1 if diffs else 0


if __name__ == "__main__":
    sys.exit(main())
