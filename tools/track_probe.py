"""Tái hiện CÓ KIỂM SOÁT sự cố `lib/core/TASK.md` biến mất (lệnh Giang 2026-10-16 mục 0b): chạy từng bước công cụ đột biến, MỖI bước bọc bởi
`tools/track_guard.sh` (git status + băm `git diff HEAD` từng tệp) trước/sau; ghi nối docs/TRACK_PROBE.jsonl; DỪNG ngay khi tệp đã theo dõi đổi.
Dùng:  python tools/track_probe.py   (1 worker, tuần tự; cần máy rảnh; mất ~1,5 giờ)"""
import io
import json
import os
import subprocess
import sys
import tempfile
import time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__))).replace("\\", "/")
GUARD = ROOT + "/tools/track_guard.sh"
OUT = ROOT + "/docs/TRACK_PROBE.jsonl"
ENV = dict(os.environ, CARGO_BUILD_JOBS="2", PYTHONIOENCODING="utf-8", GRIT_LLVM_DIR="D:/Tools/llvm-20.1.8",
           GRIT_RT_LIB=ROOT + "/runtime/gt-rt/target/release/gt_rt.lib", STD_RANDOM="4")
ENV["PATH"] = "D:/Tools/llvm-20.1.8/bin;" + ENV["PATH"]


os.environ["TRACK_IGNORE"] = "docs/TRACK_PROBE.jsonl"  # tệp kết quả của chính probe (ghi nối có chủ ý)


def guard(mode, snap):
    r = subprocess.run(["bash", GUARD, mode, snap, ROOT], capture_output=True, text=True, encoding="utf-8", errors="replace")
    return r.returncode, r.stdout.strip()


def porcelain():
    r = subprocess.run(["git", "status", "--porcelain"], cwd=ROOT, capture_output=True, text=True, encoding="utf-8")
    return r.stdout.strip()


def step(name, argv, extra_env=None):
    snap = tempfile.mktemp(prefix="probe_snap_").replace("\\", "/")
    guard("snapshot", snap)
    p_before = porcelain()
    t = time.time()
    env = dict(ENV, **(extra_env or {}))
    r = subprocess.run(argv, cwd=ROOT, env=env, capture_output=True, text=True, encoding="utf-8", errors="replace")
    rc, msg = guard("verify", snap)
    rec = {"step": name, "exit": r.returncode, "seconds": round(time.time() - t, 1), "track_ok": rc == 0,
           "porcelain_before_equals_after": p_before == porcelain(), "note": msg[:300]}
    with io.open(OUT, "a", encoding="utf-8") as f:
        f.write(json.dumps(rec, ensure_ascii=False) + "\n")
    print(rec, flush=True)
    if rc != 0:
        print("!!!!! TỆP ĐÃ THEO DÕI ĐỔI SAU BƯỚC", name, "\n", msg, "\nstdout/stderr cuối:\n", (r.stdout + r.stderr)[-1500:])
        sys.exit(3)


def main():
    py = sys.executable
    for i in range(1, 20):
        if i == 15:
            continue  # mẫu không-đổi
        step(f"mut_std #{i}", [py, ROOT + "/tools/mut_std.py"], {"MUT_ONLY": str(i)})
    for i in (1, 2, 3):
        step(f"mut_q87 #{i}", [py, ROOT + "/tools/mut_q87.py"], {"MUT_ONLY": str(i)})
    out = ROOT + "/target/tmp/probe_mut_py.jsonl"
    if os.path.exists(out):
        os.remove(out)
    step("tools/mut.py (gt-span, --limit 6)", [py, ROOT + "/tools/mut.py", "run", "--workers", "1", "--limit", "6", "--out", out,
                                               ROOT + "/crates/gt-span/src/lib.rs"],
         {"GRIT_MUT_TEST": "cargo test -p gt-span -- --test-threads=2"})
    print("XONG: không bước nào làm đổi tệp đã theo dõi")


if __name__ == "__main__":
    main()
