#!/usr/bin/env python3
"""ĐO nhu cầu stack THẬT của 6 công cụ dogfood và 4 capstone (GÓI 4 S2.a). THỦ CÔNG (không nằm trong cổng).

Cơ chế CHỈ để kiểm thử: runtime `gt-rt` dựng với `--features test-fault` cho phép `GRITRT_TEST_STACK_BYTES=<byte>` ghi đè cỡ stack của luồng Grit.
Bản sản xuất BỎ QUA biến này (có test). Cách dùng (Git Bash, đã đặt GRIT_LLVM_DIR/PATH, đã chạy preflight):
  CARGO_TARGET_DIR=target/tmp/rtstack cargo build --release --features test-fault --manifest-path runtime/gt-rt/Cargo.toml
  python tools/stack_probe.py [--out docs/STACK_PROBE.jsonl]
Mỗi chương trình dựng ở dev và release-o2 với runtime test-fault, chạy trên BỘ ĐẦU VÀO LỚN NHẤT với stack 4, 8, 16, 32, 64 MiB và 256 MiB (đối chiếu); đạt =
mã thoát 0 và đầu ra BẰNG đầu ra ở 256 MiB (capstone: mọi ca `check.py --large` khớp bản tham chiếu Rust). Ghi stack nhỏ nhất đạt."""
import hashlib
import json
import os
import random
import shutil
import subprocess
import sys
import tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
GRITC = os.environ.get("GRITC", os.path.join(ROOT, "target", "debug", "gritc.exe" if os.name == "nt" else "gritc"))
RT = os.environ.get("STACK_RT_LIB", os.path.join(ROOT, "target", "tmp", "rtstack", "release", "gt_rt.lib" if os.name == "nt" else "libgt_rt.a"))
SIZES_MIB = [4, 8, 16, 32, 64, 256]
MIB = 1 << 20


def build(pkg, profile, out):
    sys.path.insert(0, os.path.join(ROOT, "capstone"))
    d = tempfile.mkdtemp(prefix="stkbuild-")
    try:
        for srcdir in (os.path.join(ROOT, "lib", "std", "src"), os.path.join(pkg, "src")):
            for f in os.listdir(srcdir):
                if f.endswith(".gt"):
                    shutil.copy(os.path.join(srcdir, f), os.path.join(d, f))
        env = dict(os.environ, GRIT_RT_LIB=RT)
        r = subprocess.run([GRITC, "build", d, f"--profile={profile}", "-o", out], capture_output=True, text=True, env=env)
        if r.returncode != 0:
            raise SystemExit("dựng thất bại " + pkg + ": " + (r.stdout + r.stderr)[-800:])
    finally:
        shutil.rmtree(d, ignore_errors=True)


def run(exe, args, mib, stdin=b""):
    env = dict(os.environ, GRITRT_TEST_STACK_BYTES=str(mib * MIB))
    p = subprocess.run([exe, *args], input=stdin, capture_output=True, env=env, timeout=600)
    return p.returncode, hashlib.sha256(p.stdout + b"|" + p.stderr).hexdigest()


def gen_inputs(d):
    rnd = random.Random(7)
    words = ["alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta", "theta", "iota", "kappa"] + [("w%d" % i) for i in range(2000)]
    text = os.path.join(d, "text32.txt")
    with open(text, "wb") as f:
        n = 0
        while n < 32 * MIB:
            line = (" ".join(rnd.choice(words) for _ in range(rnd.randint(1, 12))) + "\n").encode()
            f.write(line)
            n += len(line)
    blob = os.path.join(d, "blob32.bin")
    with open(blob, "wb") as f:
        f.write(rnd.randbytes(32 * MIB))
    nums = os.path.join(d, "nums.txt")
    with open(nums, "w") as f:
        for _ in range(1_000_000):
            f.write("%d\n" % rnd.randint(0, 10**6))
    js = os.path.join(d, "big.json")
    deep = "[" * 99 + "1" + "]" * 99
    with open(js, "w") as f:
        f.write("[")
        n = 0
        first = True
        while n < 13 * MIB:
            item = '{"k":%d,"s":"abc","a":[1,2,3,{"x":null}],"d":%s}' % (rnd.randint(0, 10**6), deep)
            f.write(("" if first else ",") + item)
            n += len(item)
            first = False
        f.write("]")
    return {"wc": text, "wordfreq": text, "sortlines": text, "sha256": blob, "stats": nums, "json": js}


def record(out, row):
    print(json.dumps(row, ensure_ascii=False))
    out.write(json.dumps(row, ensure_ascii=False) + "\n")
    out.flush()


def main():
    outp = "docs/STACK_PROBE.jsonl"
    if "--out" in sys.argv:
        outp = sys.argv[sys.argv.index("--out") + 1]
    if not os.path.isfile(RT):
        raise SystemExit("thiếu runtime test-fault: " + RT)
    work = tempfile.mkdtemp(prefix="stkprobe-")
    done = set()
    if os.path.isfile(os.path.join(ROOT, outp)):   # --resume ngầm định: bỏ qua (chương trình, hồ sơ) đã có dòng
        for ln in open(os.path.join(ROOT, outp), encoding="utf-8"):
            if ln.strip():
                r0 = json.loads(ln)
                done.add((r0["program"], r0["profile"]))
    only = sys.argv[sys.argv.index("--only") + 1].split(",") if "--only" in sys.argv else None
    out = open(os.path.join(ROOT, outp), "a", encoding="utf-8")
    try:
        inputs = gen_inputs(work) if (not only or any(n in only for n in ("wc", "json", "sha256", "sortlines", "wordfreq", "stats"))) else {}
        for name in ("wc", "json", "sha256", "sortlines", "wordfreq", "stats"):
            for prof in ("dev", "release-o2"):
                if (name, prof) in done or (only and name not in only):
                    continue
                exe = os.path.join(work, f"{name}-{prof}.exe")
                build(os.path.join(ROOT, "examples", name), prof, exe)
                base = run(exe, [inputs[name]], 256)
                minimal = None
                res = {}
                for mib in SIZES_MIB:
                    r = run(exe, [inputs[name]], mib)
                    ok = r == base and base[0] == 0
                    res[mib] = ok
                    if ok and minimal is None:
                        minimal = mib
                record(out, {"program": name, "kind": "dogfood", "profile": prof, "baseline_exit": base[0], "ok_by_mib": res, "min_mib": minimal})
        # capstone: check.py --large với ngăn xếp ghi đè qua biến môi trường kế thừa
        ref_dir = os.path.join(ROOT, "capstone", "reference")
        if only and not any(n in only for n in ("csvstat", "calc", "kvstore", "grep-lite")):
            return
        subprocess.run(["cargo", "build", "--release", "--manifest-path", os.path.join(ref_dir, "Cargo.toml")], check=True, capture_output=True)
        cases = os.path.join(work, "cases")
        subprocess.run([sys.executable, os.path.join(ROOT, "capstone", "gen_cases.py"), cases, "--large"], check=True, capture_output=True)
        for sol, kind in (("csvstat", "csvstat"), ("calc", "calc"), ("kvstore", "kvstore"), ("grep_lite", "grep-lite")):
            refexe = os.path.join(ref_dir, "target", "release", kind + (".exe" if os.name == "nt" else ""))
            for prof in ("dev", "release-o2"):
                if (kind, prof) in done or (only and kind not in only):
                    continue
                exe = os.path.join(work, f"{kind}-{prof}.exe")
                build(os.path.join(ROOT, "capstone", "solutions", sol), prof, exe)
                res = {}
                minimal = None
                for mib in SIZES_MIB:
                    env = dict(os.environ, GRITRT_TEST_STACK_BYTES=str(mib * MIB), PYTHONIOENCODING="utf-8")
                    r = subprocess.run([sys.executable, os.path.join(ROOT, "capstone", "check.py"), "--k", kind, "--large", "--cand", exe, "--ref", refexe, "--cases", cases],
                                       capture_output=True, text=True, encoding="utf-8", errors="replace", env=env)
                    ok = r.returncode == 0
                    res[mib] = ok
                    if ok and minimal is None:
                        minimal = mib
                record(out, {"program": kind, "kind": "capstone", "profile": prof, "ok_by_mib": res, "min_mib": minimal})
    finally:
        out.close()
        shutil.rmtree(work, ignore_errors=True)


if __name__ == "__main__":
    main()
