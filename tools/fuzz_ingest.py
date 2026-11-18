#!/usr/bin/env python3
"""GÓI 7 S4.b: nạp kết quả fuzz từ CI Linux vào docs/FUZZ_LOG.jsonl (nguồn "ci-linux"), CHỐNG GHI TRÙNG.
Dùng:  gh run download <run-id> -D DIR   rồi   python tools/fuzz_ingest.py DIR [--log docs/FUZZ_LOG.jsonl] [--dry-run]
Đọc mọi `fuzz-log.jsonl` dưới DIR. Mỗi dòng PHẢI có: target (lex|parse), status (ok|fail), seconds, runs, src, ci_run. Khoá chống trùng = (ci_run, target):
chạy lại lệnh với cùng artifact không ghi thêm. Dòng thiếu trường/sai kiểu bị TỪ CHỐI (báo, không ghi). Giờ chỉ tính khi status=ok (xem tools/fuzz_total.py).
Thoát 0 nếu không có dòng bị từ chối; 1 nếu có; 2 nếu dùng sai."""
import json
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
REQUIRED = {"target": str, "status": str, "seconds": int, "runs": int, "src": str, "ci_run": str}


def validate(d):
    for k, t in REQUIRED.items():
        if k not in d or not isinstance(d[k], t) or isinstance(d[k], bool):
            return f"thiếu/sai trường `{k}`"
    if d["target"] not in ("lex", "parse"):
        return "target không phải lex|parse"
    if d["status"] not in ("ok", "fail"):
        return "status không phải ok|fail"
    if d["seconds"] < 0 or d["runs"] < 0 or not d["ci_run"] or not d["src"]:
        return "giá trị không hợp lệ"
    return None


def existing_keys(log):
    keys = set()
    if os.path.exists(log):
        with open(log, encoding="utf-8") as f:
            for ln in f:
                ln = ln.strip()
                if ln.startswith("{"):
                    d = json.loads(ln)
                    if d.get("source") == "ci-linux":
                        keys.add((str(d.get("ci_run")), d.get("target")))
    return keys


def ingest(src_dir, log, dry=False):
    """Trả (đã_ghi, bỏ_trùng, bị_từ_chối: list[str])."""
    keys = existing_keys(log)
    new, dup, bad = [], 0, []
    for dp, _dn, fs in os.walk(src_dir):
        for fn in sorted(fs):
            if fn != "fuzz-log.jsonl":
                continue
            path = os.path.join(dp, fn)
            with open(path, encoding="utf-8") as f:
                lines = f.read().splitlines()
            for i, ln in enumerate(lines, 1):
                ln = ln.strip()
                if not ln:
                    continue
                try:
                    d = json.loads(ln)
                except ValueError:
                    bad.append(f"{path}:{i}: không phải JSON")
                    continue
                why = validate(d) if isinstance(d, dict) else "không phải đối tượng"
                if why:
                    bad.append(f"{path}:{i}: {why}")
                    continue
                key = (d["ci_run"], d["target"])
                if key in keys:
                    dup += 1
                    continue
                keys.add(key)
                d["source"] = "ci-linux"
                new.append(d)
    if new and not dry:
        with open(log, "a", encoding="utf-8", newline="\n") as f:
            for d in new:
                f.write(json.dumps(d, ensure_ascii=False, separators=(",", ":")) + "\n")
    return len(new), dup, bad


def main():
    a = sys.argv[1:]
    if not a or a[0].startswith("--"):
        print(__doc__)
        sys.exit(2)
    log = a[a.index("--log") + 1] if "--log" in a else os.path.join(ROOT, "docs", "FUZZ_LOG.jsonl")
    n, dup, bad = ingest(a[0], log, "--dry-run" in a)
    for b in bad:
        print("TỪ CHỐI:", b)
    print(f"ghi {n} dòng, bỏ trùng {dup}, từ chối {len(bad)}" + (" (dry-run)" if "--dry-run" in a else ""))
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
