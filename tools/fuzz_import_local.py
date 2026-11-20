#!/usr/bin/env python3
"""GÓI 12 P2.c: nối log fuzz của một CLONE chạy trong WSL (nguồn "wsl-linux") vào docs/FUZZ_LOG.jsonl của kho chính, CHỐNG TRÙNG.
Dùng:  python tools/fuzz_import_local.py <đường_dẫn_jsonl> [--log docs/FUZZ_LOG.jsonl] [--dry-run]
Với mỗi dòng của tệp nguồn:
  - không có trường `source` (lịch sử Windows nằm sẵn trong clone) hoặc status=skipped_busy ⇒ BỎ QUA (không ghi, không từ chối);
  - `source` khác "wsl-linux" ⇒ TỪ CHỐI (nguồn lạ);
  - thiếu/sai trường (ts, target lex|parse, status ok|fail, seconds, runs, src, os, kernel) ⇒ TỪ CHỐI;
  - `src` KHÁC băm nguồn hiện tại của kho chính (`fuzz_total.current_hash`) ⇒ TỪ CHỐI (mã đã đổi, giờ này không tính được);
  - trùng khoá (ts, target, source) với dòng đã có trong log ⇒ bỏ trùng.
Giờ chỉ được tính khi status=ok (xem tools/fuzz_total.py). Thoát 0 nếu không có dòng bị từ chối; 1 nếu có; 2 nếu dùng sai."""
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import fuzz_total  # noqa: E402

ROOT = os.path.dirname(HERE)
SOURCE = "wsl-linux"
REQUIRED = {"ts": str, "target": str, "status": str, "seconds": int, "runs": int, "src": str, "os": str, "kernel": str}


def validate(d, current):
    for k, t in REQUIRED.items():
        if k not in d or not isinstance(d[k], t) or isinstance(d[k], bool):
            return f"thiếu/sai trường `{k}`"
    if d["target"] not in ("lex", "parse"):
        return "target không phải lex|parse"
    if d["status"] not in ("ok", "fail"):
        return "status không phải ok|fail"
    if d["seconds"] < 0 or d["runs"] < 0 or not d["ts"] or not d["os"] or not d["kernel"]:
        return "giá trị không hợp lệ"
    if d["src"] != current(d["target"]):
        return f"băm nguồn {d['src']} khác băm hiện tại của kho chính ({current(d['target'])})"
    return None


def existing_keys(log):
    keys = set()
    if os.path.exists(log):
        with open(log, encoding="utf-8") as f:
            for ln in f:
                ln = ln.strip()
                if ln.startswith("{"):
                    d = json.loads(ln)
                    if d.get("source") == SOURCE:
                        keys.add((d.get("ts"), d.get("target")))
    return keys


def import_log(path, log, dry=False, current=fuzz_total.current_hash):
    """Trả (đã_ghi, bỏ_trùng, bỏ_qua, bị_từ_chối: list[str])."""
    keys = existing_keys(log)
    new, dup, skipped, bad = [], 0, 0, []
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
        if not isinstance(d, dict):
            bad.append(f"{path}:{i}: không phải đối tượng")
            continue
        if "source" not in d or d.get("status") == "skipped_busy":
            skipped += 1
            continue
        if d["source"] != SOURCE:
            bad.append(f"{path}:{i}: nguồn lạ {d['source']!r} (chỉ nhận {SOURCE!r})")
            continue
        why = validate(d, current)
        if why:
            bad.append(f"{path}:{i}: {why}")
            continue
        key = (d["ts"], d["target"])
        if key in keys:
            dup += 1
            continue
        keys.add(key)
        new.append(d)
    if new and not dry:
        with open(log, "a", encoding="utf-8", newline="\n") as f:
            for d in new:
                f.write(json.dumps(d, ensure_ascii=False, separators=(",", ":")) + "\n")
    return len(new), dup, skipped, bad


def main():
    a = sys.argv[1:]
    if not a or a[0].startswith("--"):
        print(__doc__)
        sys.exit(2)
    log = a[a.index("--log") + 1] if "--log" in a else os.path.join(ROOT, "docs", "FUZZ_LOG.jsonl")
    n, dup, skipped, bad = import_log(a[0], log, "--dry-run" in a)
    for b in bad:
        print("TỪ CHỐI:", b)
    print(f"ghi {n} dòng, bỏ trùng {dup}, bỏ qua {skipped}, từ chối {len(bad)}" + (" (dry-run)" if "--dry-run" in a else ""))
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
