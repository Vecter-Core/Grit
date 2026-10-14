#!/usr/bin/env python3
"""Đo dev (-O0) và release (ADR-036) trên fib + 5 công cụ dogfood. Dùng: python tools/bench_profile.py [số lần=7] [--write].
PHÉP ĐO, không phải cổng. Mỗi exe chạy N lần xen kẽ dev/release (giảm trôi nhiệt/nền), bỏ 1 lần khởi động nóng; báo trung vị, min, max và độ
lệch chuẩn tổng thể (phương sai lớn trên máy này do quét tệp của Windows Defender). Đầu vào sinh tất định (LCG) ở thư mục tạm."""
import os, statistics, subprocess, sys, tempfile, time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
GRITC = os.environ.get("GRITC", os.path.join(ROOT, "target", "debug", "gritc.exe"))
LLVM = os.environ.get("GRIT_LLVM_DIR", "D:/Tools/llvm-20.1.8")
ENV = dict(os.environ, PATH=os.path.join(LLVM, "bin") + os.pathsep + os.environ["PATH"])
args = [a for a in sys.argv[1:] if not a.startswith("--")]
N = int(args[0]) if args and args[0].isdigit() else 7


def lcg(seed):
    s = seed
    while True:
        s = (s * 6364136223846793005 + 1442695040888963407) & 0xFFFFFFFFFFFFFFFF
        yield s >> 33


def gen_inputs(tmp):
    g = lcg(20261014)
    p = {}
    # wc: 32 MiB văn bản
    words = [b"alpha", b"beta", b"gamma", b"delta", b"epsilon", b"zeta", b"eta", b"theta"]
    buf = bytearray()
    while len(buf) < 32 << 20:
        line = b" ".join(words[next(g) % 8] for _ in range(1 + next(g) % 12))
        buf += line + b"\n"
    p["wc"] = os.path.join(tmp, "wc.txt")
    open(p["wc"], "wb").write(bytes(buf))
    # sha256: 32 MiB byte ngẫu nhiên
    p["sha256"] = os.path.join(tmp, "sha.bin")
    open(p["sha256"], "wb").write(bytes(next(g) & 255 for _ in range(32 << 20)))
    # sortlines: 200k dòng ngắn
    p["sortlines"] = os.path.join(tmp, "lines.txt")
    open(p["sortlines"], "w").write("".join("line%08d\n" % (next(g) % 100000000) for _ in range(200_000)))
    # json: mảng 20k đối tượng
    p["json"] = os.path.join(tmp, "data.json")
    objs = ",".join(
        '{"id":%d,"name":"item%d","tags":["a","b",%d],"ok":true,"v":%d}' % (i, i, next(g) % 100, next(g) % 100000) for i in range(20_000)
    )
    open(p["json"], "w").write("[" + objs + "]")
    # stats: 10^6 số
    p["stats"] = os.path.join(tmp, "nums.txt")
    open(p["stats"], "w").write("".join("%d\n" % (next(g) % 100000) for _ in range(1_000_000)))
    return p


PROF2 = os.environ.get("BENCH_PROFILE", "release")  # hồ sơ gắn nhãn cột "release" (release hoặc release-o2)


def build(src, out, profile):
    profile = PROF2 if profile == "release" else profile
    t = time.perf_counter()
    r = subprocess.run([GRITC, "build", src, f"--profile={profile}", "-o", out], env=ENV, capture_output=True, text=True, encoding="utf-8", errors="replace")
    if r.returncode != 0 or not os.path.exists(out):
        sys.exit(f"build lỗi {src} {profile}: {r.stderr[-400:]}")
    return time.perf_counter() - t


def run_once(exe, arg):
    t = time.perf_counter()
    r = subprocess.run([exe] + ([arg] if arg else []), env=ENV, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    return time.perf_counter() - t, r


def main():
    tmp = tempfile.mkdtemp(prefix="gtbp-")
    inputs = gen_inputs(tmp)
    items = [("fib(35)", os.path.join(ROOT, "tests", "bench", "fib.gt"), None)]
    for n in ["wc", "json", "sha256", "sortlines", "stats"]:
        items.append((n, os.path.join(ROOT, "examples", n, "src"), inputs[n]))
    rows = []
    for name, src, arg in items:
        exes = {}
        bt = {}
        for prof in ["dev", "release"]:
            exes[prof] = os.path.join(tmp, f"{name.split('(')[0]}_{prof}.exe")
            bt[prof] = build(src, exes[prof], prof)
        outs = {}
        for prof in exes:  # khởi động nóng + đối chiếu kết quả (stdout) giữa hai hồ sơ
            _, r = run_once(exes[prof], arg)
            outs[prof] = (r.returncode, r.stdout)
        same = outs["dev"] == outs["release"]
        ts = {"dev": [], "release": []}
        for _ in range(N):
            for prof in ["dev", "release"]:
                d, _ = run_once(exes[prof], arg)
                ts[prof].append(d)
        rows.append((name, bt, ts, same))
        print(f"{name}: dev {statistics.median(ts['dev']):.3f}s release {statistics.median(ts['release']):.3f}s kết quả giống nhau={same}", flush=True)
    lines = [
        "| bench | dựng dev / release (gritc debug) | dev: trung vị (min–max) ± σ | release: trung vị (min–max) ± σ | tăng tốc (trung vị) | tăng tốc (min) | đầu ra giống nhau |",
        "|---|---|---|---|---|---|---|",
    ]
    for name, bt, ts, same in rows:
        def f(v):
            return f"{statistics.median(v):.3f} s ({min(v):.3f}–{max(v):.3f}) ± {statistics.pstdev(v):.3f}"
        sp = statistics.median(ts["dev"]) / statistics.median(ts["release"])
        spm = min(ts["dev"]) / min(ts["release"])
        lines.append(f"| {name} | {bt['dev']:.2f} / {bt['release']:.2f} s | {f(ts['dev'])} | {f(ts['release'])} | ×{sp:.2f} | ×{spm:.2f} | {'có' if same else 'KHÔNG'} |")
    print("\n".join(lines))
    if "--write" in sys.argv:
        open(os.path.join(ROOT, "target", "tmp", "bench_profile_table.md"), "w", encoding="utf-8").write("\n".join(lines) + "\n")


main()
