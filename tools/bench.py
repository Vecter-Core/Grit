#!/usr/bin/env python3
"""Benchmark nền của backend llvm (M10). Dùng: python tools/bench.py [số lần] — ghi bảng ra stdout (và docs/BENCHMARK.md qua --write).
Đo thật: thời gian dựng (gritc build), thời gian chạy exe (trung vị/min của N lần), thời gian thông dịch ở kích thước thu nhỏ (quy ra tốc độ),
và tham chiếu Rust (rustc -C opt-level=0 và -O) cho cùng thuật toán. KHÔNG phải benchmark so sánh ngôn ngữ: chỉ là mốc nền để theo dõi."""
import os, re, statistics, subprocess, sys, tempfile, time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
GRITC = os.environ.get("GRITC", os.path.join(ROOT, "target", "debug", "gritc.exe"))
LLVM = os.environ.get("GRIT_LLVM_DIR", "D:/Tools/llvm-20.1.8")
ENV = dict(os.environ, PATH=os.path.join(LLVM, "bin") + os.pathsep + os.environ["PATH"])
N = int(sys.argv[1]) if len(sys.argv) > 1 and sys.argv[1].isdigit() else 5

# (tên, số đơn vị công việc ở kích thước đầy đủ, (regex, thay) thu nhỏ cho thông dịch, số đơn vị ở kích thước thu nhỏ, tham chiếu rust)
BENCH = [
    ("loop_sum", 400_000_000, (r"sum_to\(400000000\)", "sum_to(1000000)"), 1_000_000, "loop_sum"),
    ("fib", 29_860_703, (r"fib\(35\)", "fib(22)"), 57_313, "fib"),  # số lời gọi: 2*fib(n+1)-1
    ("slice_sum", 200_000_000, (r"200000", "1"), 1_000, "slice_sum"),
    ("threads", 4_000_000, None, 0, None),
]


def timed(cmd, env=ENV):
    t = time.perf_counter()
    r = subprocess.run(cmd, env=env, capture_output=True, text=True, encoding="utf-8", errors="replace")
    return time.perf_counter() - t, r


def main():
    tmp = tempfile.mkdtemp(prefix="gtbench-")
    rows = []
    for name, work, shrink, small_work, ref in BENCH:
        src = os.path.join(ROOT, "tests", "bench", name + ".gt")
        exe = os.path.join(tmp, name + ".exe")
        bt, r = timed([GRITC, "build", "--backend=llvm", "-o", exe, src])
        if r.returncode != 0:
            print(f"{name}: build thất bại: {r.stderr}")
            sys.exit(1)
        timed([exe])  # khởi động nóng: bỏ lần đầu (quét tệp mới của Windows Defender làm lần chạy đầu chậm ~1 s)
        runs = []
        for _ in range(N):
            t, rr = timed([exe])
            if rr.returncode != 0:
                print(f"{name}: exe thoát {rr.returncode}")
                sys.exit(1)
            runs.append(t)
        row = {"name": name, "work": work, "build": bt, "med": statistics.median(runs), "min": min(runs)}
        if shrink:
            text = open(src, encoding="utf-8").read()
            small = re.sub(shrink[0], shrink[1], text)
            sp = os.path.join(tmp, name + "_small.gt")
            open(sp, "w", encoding="utf-8").write(small)
            it, r2 = timed([GRITC, "run", "--backend=interpreter", sp])
            if r2.returncode != 0:
                print(f"{name}: thông dịch thoát {r2.returncode}: {r2.stdout} {r2.stderr}")
                sys.exit(1)
            row["interp_s"] = it
            row["interp_work"] = small_work
        if ref:
            rs = os.path.join(ROOT, "tools", "bench_ref", ref + ".rs")
            for tag, flags in (("rust_O0", ["-C", "opt-level=0"]), ("rust_O2", ["-O"])):
                rexe = os.path.join(tmp, f"{ref}_{tag}.exe")
                subprocess.run(["rustc", *flags, "-o", rexe, rs], capture_output=True, check=True)
                timed([rexe])
                rr = []
                for _ in range(N):
                    t, x = timed([rexe])
                    rr.append(t)
                row[tag] = (statistics.median(rr), min(rr))
        rows.append(row)
    out = [f"| bench | công việc | dựng (gritc debug) | exe LLVM -O0 trung vị / min ({N} lần) | Mops/s (theo min) | thông dịch (thu nhỏ) | thông dịch Mops/s | Rust -C opt-level=0 (trung vị / min) | Rust -O (trung vị / min) |", "|---|---|---|---|---|---|---|---|---|"]
    for r in rows:
        mops = r["work"] / r["min"] / 1e6
        ip = f'{r["interp_s"]:.2f} s ({r["interp_work"]:,})' if "interp_s" in r else "—"
        imops = f'{r["interp_work"] / r["interp_s"] / 1e6:.2f}' if "interp_s" in r else "—"
        o0 = f'{r["rust_O0"][0]:.2f} / {r["rust_O0"][1]:.2f} s' if "rust_O0" in r else "—"
        o2 = f'{r["rust_O2"][0]:.2f} / {r["rust_O2"][1]:.2f} s' if "rust_O2" in r else "—"
        out.append(f'| {r["name"]} | {r["work"]:,} | {r["build"]:.2f} s | {r["med"]:.2f} s / {r["min"]:.2f} s | {mops:.1f} | {ip} | {imops} | {o0} | {o2} |')
    print("\n".join(out))


main()
