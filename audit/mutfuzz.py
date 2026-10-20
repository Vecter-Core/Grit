#!/usr/bin/env python3
"""Fuzz đột biến mức TOKEN, đen-hộp qua dòng lệnh `gritc check` (không cần nightly/cargo-fuzz).

Lấy chương trình từ tests/pass, áp một đột biến ngẫu nhiên có hạt giống cố định (xoá/nhân đôi/hoán đổi token, thay định danh, thay literal số
bằng giá trị biên, thay tên kiểu), chạy `gritc check --json` với giới hạn thời gian. Phân loại:
  OK_GREEN / OK_RED   mã thoát 0/1 (mong đợi)
  ICE                 mã thoát 2 hoặc verdict ice hoặc E9901 — lỗi nội bộ trên đầu vào lạ (fail-closed nhưng là BUG theo GRIT_PLAN §9)
  CRASH               tiến trình chết do tín hiệu / có 'panicked' trong stderr / mã thoát lạ — NGHIÊM TRỌNG
  TIMEOUT             quá thời gian — treo hoặc quá chậm
Ghi các đầu vào ICE/CRASH/TIMEOUT (tối đa 12 mỗi loại) vào --out để tái hiện. Dùng: mutfuzz.py --gritc G [--n 3000] [--seed 1] [--jobs 2] [--out DIR]
"""
import argparse
import glob
import hashlib
import json
import os
import random
import re
import subprocess
import sys
import tempfile
from concurrent.futures import ThreadPoolExecutor

TOK = re.compile(r"\s+|[A-Za-z_][A-Za-z_0-9]*|[0-9][0-9_]*|[^\sA-Za-z_0-9]")
INTS = ["0", "1", "2", "7", "127", "128", "255", "256", "65535", "65536", "2147483647", "2147483648", "4294967295", "4294967296",
        "9223372036854775807", "9223372036854775808", "18446744073709551615", "18446744073709551616",
        "170141183460469231731687303715884105727", "340282366920938463463374607431768211455", "340282366920938463463374607431768211456"]
TYPES = ["i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize", "bool", "char", "f64", "str"]


def mutate(src, rng):
    toks = TOK.findall(src)
    idx = [i for i, t in enumerate(toks) if not t.isspace()]
    if len(idx) < 3:
        return src
    ids = [t for t in toks if re.fullmatch(r"[A-Za-z_][A-Za-z_0-9]*", t)]
    kind = rng.choice(["del", "dup", "swap", "ident", "int", "type", "delrun", "ins"])
    i = rng.choice(idx)
    if kind == "del":
        toks[i] = ""
    elif kind == "dup":
        toks[i] = toks[i] + " " + toks[i]
    elif kind == "swap":
        j = min(len(toks) - 1, i + 2)
        toks[i], toks[j] = toks[j], toks[i]
    elif kind == "ident" and ids:
        if re.fullmatch(r"[A-Za-z_][A-Za-z_0-9]*", toks[i]):
            toks[i] = rng.choice(ids)
    elif kind == "int":
        nums = [k for k in idx if re.fullmatch(r"[0-9][0-9_]*", toks[k])]
        if nums:
            toks[rng.choice(nums)] = rng.choice(INTS)
    elif kind == "type":
        tys = [k for k in idx if toks[k] in TYPES]
        if tys:
            toks[rng.choice(tys)] = rng.choice(TYPES)
    elif kind == "delrun":
        for k in range(i, min(len(toks), i + rng.randint(2, 8))):
            toks[k] = ""
    elif kind == "ins":
        toks.insert(i, rng.choice(["{", "}", "(", ")", ";", "↦", "⎇", "⟳", "&!", "&", "*", "←", "≔", "μ", "::", "<", ">", ",", "'a", "!", "∅"]))
    return "".join(toks)


def run(gritc, src, tmp):
    h = hashlib.sha1(src.encode()).hexdigest()[:12]
    p = os.path.join(tmp, "m_" + h + ".gt")
    with open(p, "w", encoding="utf-8") as f:
        f.write(src)
    try:
        r = subprocess.run([gritc, "check", p, "--json"], capture_output=True, text=True, timeout=30)
    except subprocess.TimeoutExpired:
        return "TIMEOUT", src, ""
    finally:
        pass
    err = r.stderr or ""
    if r.returncode < 0 or "panicked" in err or r.returncode not in (0, 1, 2, 3):
        return "CRASH", src, f"rc={r.returncode} {err[-200:]}"
    try:
        j = json.loads(r.stdout)
    except Exception:
        return ("ICE" if r.returncode == 2 else "NOJSON"), src, f"rc={r.returncode} {err[-200:]}"
    codes = [d.get("code") for d in j.get("diagnostics", [])]
    if j.get("verdict") == "ice" or "E9901" in codes or r.returncode == 2:
        msg = next((d.get("message", "") for d in j.get("diagnostics", []) if d.get("code") == "E9901"), "")
        return "ICE", src, msg[:200]
    return ("OK_GREEN" if r.returncode == 0 else "OK_RED"), src, ""


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--gritc", required=True)
    ap.add_argument("--n", type=int, default=3000)
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--jobs", type=int, default=2)
    ap.add_argument("--tests", default=os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "tests", "pass"))
    ap.add_argument("--out", default=None)
    a = ap.parse_args()
    rng = random.Random(a.seed)
    files = sorted(glob.glob(os.path.join(a.tests, "*.gt")))
    seeds = [open(f, encoding="utf-8").read() for f in files]
    jobs = []
    for _ in range(a.n):
        s = rng.choice(seeds)
        for _ in range(rng.choice([1, 1, 1, 2, 3])):
            s = mutate(s, rng)
        jobs.append(s)
    tmp = tempfile.mkdtemp(prefix="mutfuzz_")
    cnt, keep, sigs = {}, {}, {}
    with ThreadPoolExecutor(a.jobs) as ex:
        for st, src, info in ex.map(lambda s: run(a.gritc, s, tmp), jobs):
            cnt[st] = cnt.get(st, 0) + 1
            if st not in ("OK_GREEN", "OK_RED"):
                sig = re.sub(r"[0-9]+", "N", info)[:80]
                sigs.setdefault((st, sig), 0)
                sigs[(st, sig)] += 1
                lst = keep.setdefault(st, [])
                if len(lst) < 12 and sigs[(st, sig)] <= 2:
                    lst.append((src, info))
    print("SUMMARY: fuzz đột biến token qua CLI gritc: " + f"{a.n} mẫu, seed {a.seed}; " + "; ".join(f"{k}={v}" for k, v in sorted(cnt.items())))
    for (st, sig), n in sorted(sigs.items(), key=lambda x: -x[1]):
        print(f"SUMMARY: {st} x{n}: {sig}")
    if a.out:
        os.makedirs(a.out, exist_ok=True)
        for st, lst in keep.items():
            for k, (src, info) in enumerate(lst):
                with open(os.path.join(a.out, f"{st}_{k}.gt"), "w", encoding="utf-8") as f:
                    f.write(f"// {st}: {info}\n{src}")
    # ICE trên đầu vào lạ cũng là BUG theo GRIT_PLAN §9 (dù fail-closed) ⇒ đỏ
    return 1 if any(cnt.get(k) for k in ("CRASH", "TIMEOUT", "ICE")) else 0


if __name__ == "__main__":
    sys.exit(main())
