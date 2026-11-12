#!/usr/bin/env python3
"""Reproduce the G2.3 sortlines comparison. Run after tools/preflight.sh.

Builds current Grit source at dev and release-o2, then a matching byte-line
Rust reference at opt-level 0 and 3. Records input/executable-output hashes,
tool versions, profiles, harness commit, and serial warmed timing samples.
"""
from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import shutil
import statistics
import subprocess
import tempfile
import time
from datetime import datetime, timezone

ROOT = Path(__file__).resolve().parent.parent
N_LINES = 1_636_921
EXTRA_LONG_LINES = 816_018
EXPECTED_BYTES = 33_554_438
RUNS = int(os.environ.get("GT_BENCH_RUNS", "5"))
GRITC = Path(os.environ.get("GRITC", ROOT / "target/debug/gritc.exe"))

RUST_REF = r'''use std::{env, fs, io::{self, Write}};
fn main() {
    let b = fs::read(env::args_os().nth(1).unwrap()).unwrap();
    let mut v: Vec<&[u8]> = b.split(|x| *x == b'\n').collect();
    if b.last() == Some(&b'\n') { v.pop(); }
    if b.is_empty() { v.clear(); }
    v.sort_by(|a, b| a.cmp(b));
    let mut o = io::BufWriter::new(io::stdout().lock());
    for x in v { o.write_all(x).unwrap(); o.write_all(b"\n").unwrap(); }
}'''


def run(cmd: list[str], *, cwd: Path | None = None, env: dict[str, str] | None = None) -> subprocess.CompletedProcess[bytes]:
    p = subprocess.run(cmd, cwd=cwd, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if p.returncode:
        raise RuntimeError(f"{cmd!r} exited {p.returncode}: {p.stderr[-2000:].decode(errors='replace')}")
    return p


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def write_input(path: Path, n_lines: int, extra_long_lines: int) -> None:
    state = 20261112
    with path.open("wb") as f:
        for i in range(n_lines):
            state = (state * 6364136223846793005 + 1442695040888963407) & ((1 << 48) - 1)
            line = f"{state:012x}{i:07d}" + ("x" if i < extra_long_lines else "")
            f.write(line.encode("ascii") + b"\n")


def main() -> None:
    llvm = Path(os.environ.get("GRIT_LLVM_DIR", "D:/Tools/llvm-20.1.8"))
    env = dict(os.environ)
    env["PATH"] = str(llvm / "bin") + os.pathsep + env.get("PATH", "")
    with tempfile.TemporaryDirectory(prefix="grit-sortlines-g2-") as td:
        tmp = Path(td)
        inp = tmp / "lines.txt"
        write_input(inp, N_LINES, EXTRA_LONG_LINES)
        data = inp.read_bytes()
        if len(data) != EXPECTED_BYTES or data.count(b"\n") != N_LINES:
            raise AssertionError("input shape changed")

        srcdir = tmp / "src"
        srcdir.mkdir()
        for source_dir in (ROOT / "examples/sortlines/src", ROOT / "lib/std/src"):
            for p in source_dir.glob("*.gt"):
                shutil.copy2(p, srcdir / p.name)

        bins: list[tuple[str, Path]] = []
        for profile in ("dev", "release-o2"):
            exe = tmp / f"grit-{profile}.exe"
            run([str(GRITC), "build", "--backend=llvm", f"--profile={profile}", "-o", str(exe), str(srcdir)], cwd=ROOT, env=env)
            bins.append((f"grit-{profile}", exe))
        ref = tmp / "ref.rs"
        ref.write_text(RUST_REF, encoding="utf-8")
        for label, flags in (("rust-O0", ["-C", "opt-level=0"]), ("rust-O", ["-C", "opt-level=3"])):
            exe = tmp / f"{label}.exe"
            run(["rustc", *flags, "-o", str(exe), str(ref)])
            bins.append((label, exe))

        expected_hash = None
        result = {
            "date": datetime.now(timezone.utc).astimezone().isoformat(),
            "method": "serial_warmup_plus_runs_same_input",
            "harness_commit": run(["git", "rev-parse", "HEAD"], cwd=ROOT).stdout.decode().strip(),
            "gritc": str(GRITC),
            "gritc_version": run([str(GRITC), "--version"], cwd=ROOT, env=env).stdout.decode(errors="replace").strip(),
            "rustc_version": run(["rustc", "--version"]).stdout.decode().strip(),
            "llvm_version": "LLVM 20.1.8",
            "input_bytes": len(data),
            "input_lines": N_LINES,
            "input_sha256": sha(data),
            "runs_each_after_warmup": RUNS,
            "profiles": {},
        }
        for label, exe in bins:
            samples = []
            output_hash = None
            for i in range(RUNS + 1):
                start = time.perf_counter()
                p = run([str(exe), str(inp)])
                elapsed = time.perf_counter() - start
                h = sha(p.stdout)
                if output_hash is None:
                    output_hash = h
                elif output_hash != h:
                    raise AssertionError(f"{label}: output varied between runs")
                if i:
                    samples.append(round(elapsed, 4))
            if expected_hash is None:
                expected_hash = output_hash
            elif expected_hash != output_hash:
                raise AssertionError(f"{label}: byte output hash differs")
            result["profiles"][label] = {
                "runs_s": samples,
                "median_s": round(statistics.median(samples), 4),
                "min_s": min(samples),
                "output_bytes": EXPECTED_BYTES,
                "output_sha256": output_hash,
            }
            print(label, result["profiles"][label], flush=True)

        # Scaling probe for dev Arena checks: same deterministic line family,
        # increasing n; this is a measurement, while reserve/allocator source
        # inspection determines how many Arena allocations occur.
        scale = []
        dev_exe = tmp / "grit-dev.exe"
        for n in (100_000, 400_000, N_LINES):
            extra = round(n * EXTRA_LONG_LINES / N_LINES)
            scale_in = tmp / f"scale-{n}.txt"
            write_input(scale_in, n, extra)
            samples = []
            for _ in range(3):
                start = time.perf_counter()
                p = run([str(dev_exe), str(scale_in)])
                samples.append(round(time.perf_counter() - start, 4))
            scale.append({
                "lines": n,
                "input_bytes": scale_in.stat().st_size,
                "input_sha256": sha(scale_in.read_bytes()),
                "runs_s": samples,
                "median_s": round(statistics.median(samples), 4),
                "exit": 0,
            })
            print("arena_scaling", scale[-1], flush=True)
        result["arena_scaling_dev"] = scale

        line = json.dumps(result, ensure_ascii=False, separators=(",", ":"))
        out = ROOT / "docs/SORTLINES_BENCHMARK_G2.jsonl"
        with out.open("a", encoding="utf-8", newline="\n") as f:
            f.write(line + "\n")
        print(f"recorded {out}", flush=True)


if __name__ == "__main__":
    main()
