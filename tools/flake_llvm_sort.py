#!/usr/bin/env python3
"""R5: điều tra flake `llvm_sort::random_arrays_with_duplicates_match_stable_sort` (lệnh Giang 2026-10-17).

Chạy trực tiếp tệp test đã dựng (KHÔNG gọi cargo) và ghi MỖI lần chạy một dòng vào `docs/FLAKE_LLVM_SORT.jsonl`:
thời điểm, seed, kết quả (xanh/đỏ), thời gian, bộ nhớ vật lý còn trống, parallel=1, và TOÀN BỘ thông điệp khi đỏ.

  python tools/flake_llvm_sort.py --runs 12 --seed-start 1 --max-seconds 500

Mỗi lần chạy dùng một seed khác nhau (`seed-start` trở đi), TUẦN TỰ; cấm tiến trình song song. Dừng nhận lần mới khi quá `--max-seconds`.
Không phải bằng chứng "đã sửa": kết luận thống kê ở docs/RUN_REPORTS.md (0/N ⇒ giới hạn trên ≈ 3/N ở tin cậy 95%).
"""
import argparse
import ctypes
from datetime import datetime
import glob
import json
import os
import subprocess
import sys
import time

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
LEDGER = os.path.join(ROOT, "docs", "FLAKE_LLVM_SORT.jsonl")
TEST = "random_arrays_with_duplicates_match_stable_sort"


class MemStatus(ctypes.Structure):
    _fields_ = [
        ("dwLength", ctypes.c_ulong),
        ("dwMemoryLoad", ctypes.c_ulong),
        ("ullTotalPhys", ctypes.c_ulonglong),
        ("ullAvailPhys", ctypes.c_ulonglong),
        ("ullTotalPageFile", ctypes.c_ulonglong),
        ("ullAvailPageFile", ctypes.c_ulonglong),
        ("ullTotalVirtual", ctypes.c_ulonglong),
        ("ullAvailVirtual", ctypes.c_ulonglong),
        ("sullAvailExtendedVirtual", ctypes.c_ulonglong),
    ]


def free_mem_mb():
    m = MemStatus()
    m.dwLength = ctypes.sizeof(MemStatus)
    ctypes.windll.kernel32.GlobalMemoryStatusEx(ctypes.byref(m))
    return int(m.ullAvailPhys // (1024 * 1024))


def newest_exe():
    c = glob.glob(os.path.join(ROOT, "target", "debug", "deps", "llvm_sort-*.exe"))
    c = [p for p in c if os.path.isfile(p)]
    if not c:
        sys.exit("không thấy llvm_sort-*.exe: dựng bằng `cargo test -p gt-driver --test llvm_sort --no-run`")
    return max(c, key=os.path.getmtime)


def main():
    sys.stdout.reconfigure(encoding="utf-8")
    ap = argparse.ArgumentParser()
    ap.add_argument("--runs", type=int, default=8)
    ap.add_argument("--seed-start", type=int, default=1)
    ap.add_argument("--max-seconds", type=int, default=470)
    ap.add_argument("--timeout", type=int, default=420)
    a = ap.parse_args()
    if a.runs < 1:
        ap.error("--runs phải ≥ 1")
    if a.seed_start < 0 or a.seed_start + a.runs - 1 > (1 << 64) - 1:
        ap.error("dải seed phải nằm trong 0..2^64-1")
    exe = newest_exe()
    env = dict(os.environ)
    env["PATH"] = "D:/Tools/llvm-20.1.8/bin;" + env.get("PATH", "")
    start = time.time()
    done = 0
    red = 0
    while done < a.runs and time.time() - start < a.max_seconds:
        seed = a.seed_start + done
        mem = free_mem_mb()
        t0 = time.time()
        env["GT_SORT_SEED"] = str(seed)
        try:
            p = subprocess.Popen(
                [exe, TEST, "--exact", "--test-threads=2"],
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                env=env,
                cwd=os.path.join(ROOT, "crates", "gt-driver"),
            )
            out, _ = p.communicate(timeout=a.timeout)
            ok = p.returncode == 0
            text = out.decode("utf-8", "replace")
        except subprocess.TimeoutExpired:
            p.kill()
            out, _ = p.communicate()
            ok = False
            text = "TIMEOUT sau %d s\n" % a.timeout + out.decode("utf-8", "replace")
        except OSError as e:
            ok = False
            text = f"SPAWN ERROR: {type(e).__name__}: {e}"
        rec = {
            "time": datetime.now().astimezone().isoformat(timespec="seconds"),
            "method": "serial_unique_seed_v1",
            "seed": seed,
            "result": "xanh" if ok else "do",
            "seconds": round(time.time() - t0, 1),
            "free_mem_mb": mem,
            "parallel": 1,
            "exe": os.path.basename(exe),
        }
        if not ok:
            rec["message"] = text
            red += 1
        with open(LEDGER, "a", encoding="utf-8", newline="\n") as f:
            f.write(json.dumps(rec, ensure_ascii=False) + "\n")
        done += 1
        if not ok:
            break
    print(f"đã chạy {done} lần, đỏ {red}, {round(time.time() - start)} s")
    sys.exit(0 if red == 0 else 1)


if __name__ == "__main__":
    main()
