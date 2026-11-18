#!/usr/bin/env python3
"""GÓI 6 S3: so exe của một app (apps/<tên>) với bản tham chiếu độc lập (apps/<tên>/reference.py) trên bộ ca (apps/<tên>/cases.py), từng byte, ở dev và release-o2.
  python apps/check_app.py <tên> [--profiles dev,release-o2] [--gritc đường/dẫn/gritc]
cases.py định nghĩa `cases()` → list[(tên_ca, {tên_tệp: bytes}, [đối số; ký tự {d} thay bằng thư mục tạm])]. Hai bên chạy CÙNG thư mục tạm (bản sao riêng cho từng bên nếu app ghi tệp).
So: mã thoát + stdout từng byte + (nếu cases.py có `final_files(dir)`) nội dung các tệp sau khi chạy. Thoát 0 nếu mọi ca khớp."""
import importlib.util
import os
import shutil
import subprocess
import sys
import tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def build(app, profile, out, gritc):
    d = tempfile.mkdtemp(prefix="appbuild-")
    try:
        for srcdir in (os.path.join(ROOT, "lib", "std", "src"), os.path.join(ROOT, "apps", app, "src")):
            for f in os.listdir(srcdir):
                if f.endswith(".gt"):
                    shutil.copy(os.path.join(srcdir, f), os.path.join(d, f))
        r = subprocess.run([gritc, "build", d, f"--profile={profile}", "-o", out], capture_output=True, text=True, encoding="utf-8", errors="replace")
        if r.returncode != 0:
            raise SystemExit("dựng thất bại: " + (r.stdout + r.stderr)[-1500:])
    finally:
        shutil.rmtree(d, ignore_errors=True)


def materialize(files, base):
    os.makedirs(base, exist_ok=True)
    for name, data in files.items():
        p = os.path.join(base, name)
        os.makedirs(os.path.dirname(p), exist_ok=True)
        open(p, "wb").write(data)


def run(cmd, cwd, args, d, tmo=120):
    a = [x.replace("{d}", d) for x in args]
    p = subprocess.run(cmd + a, cwd=cwd, capture_output=True, timeout=tmo)
    return p.returncode, p.stdout


def main():
    app = sys.argv[1]
    profiles = (sys.argv[sys.argv.index("--profiles") + 1] if "--profiles" in sys.argv else "dev,release-o2").split(",")
    gritc = sys.argv[sys.argv.index("--gritc") + 1] if "--gritc" in sys.argv else os.path.join(ROOT, "target", "debug", "gritc.exe" if os.name == "nt" else "gritc")
    spec = importlib.util.spec_from_file_location("cases_" + app, os.path.join(ROOT, "apps", app, "cases.py"))
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    ref = [sys.executable, os.path.join(ROOT, "apps", app, "reference.py")]
    work = tempfile.mkdtemp(prefix="appcheck-")
    bad = 0
    total = 0
    try:
        for prof in profiles:
            exe = os.path.join(work, f"{app}-{prof}.exe")
            build(app, prof, exe, gritc)
            for name, files, args in mod.cases():
                dr = os.path.join(work, "ref"); dc = os.path.join(work, "cand")
                for d in (dr, dc):
                    shutil.rmtree(d, ignore_errors=True)
                    materialize(files, d)
                seqs = args if (args and isinstance(args[0], list)) else [args]   # một ca có thể là DÃY lệnh (trạng thái tệp được giữ giữa các lệnh)
                a = [run(ref, dr, x, dr) for x in seqs]
                b = [run([exe], dc, x, dc) for x in seqs]
                ok = a == b
                if ok and hasattr(mod, "final_files"):
                    ok = mod.final_files(dr) == mod.final_files(dc)
                total += 1
                if not ok:
                    bad += 1
                    print(f"MISMATCH {prof} {name}")
                    for k, (x, y) in enumerate(zip(a, b)):
                        if x != y:
                            print(f"  lệnh {k}: ref=(exit {x[0]}, {x[1][:160]!r}) cand=(exit {y[0]}, {y[1][:160]!r})")
                            break
                    else:
                        print("  tệp cuối khác nhau")
        print(f"APPCHECK {app}: {total - bad}/{total} ca khớp ({','.join(profiles)})")
    finally:
        shutil.rmtree(work, ignore_errors=True)
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
