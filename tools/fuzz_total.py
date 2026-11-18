#!/usr/bin/env python3
"""Tổng fuzz TÍCH LUỸ từ docs/FUZZ_LOG.jsonl (ADR-037). Dùng: python tools/fuzz_total.py [--json]
Chỉ tính các dòng status=ok (phiên chạy hết giờ, không lỗi) và status=historical (số liệu cũ ghi tay từ PROGRESS.md, gắn nhãn riêng).
Phiên `fail` KHÔNG được tính vào giờ nhưng được báo (và phải được điều tra); `skipped_busy` chỉ đếm số lần bỏ qua."""
import hashlib, json, os, subprocess, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
LOG = os.path.join(ROOT, "docs", "FUZZ_LOG.jsonl")
TARGET_HOURS = 24.0  # GRIT_PLAN §7 #2 (sửa bởi ADR-037): mỗi loại (lexer, parser)


SRC_DIRS = {
    "lex": ["crates/gt-lexer/src", "crates/gt-lexer/fuzz/fuzz_targets"],
    "parse": ["crates/gt-parser/src", "crates/gt-parser/fuzz/fuzz_targets", "crates/gt-lexer/src", "crates/gt-ast/src", "crates/gt-span/src", "crates/gt-diag/src"],
}


def current_hash(target):
    """Băm NGUỒN hiện tại (cùng công thức `tools/fuzz_session.sh`: sha1 của `git ls-files -s <thư mục>` , 12 ký tự đầu)."""
    out = subprocess.run(["git", "ls-files", "-s", *SRC_DIRS[target]], cwd=ROOT, capture_output=True).stdout
    return hashlib.sha1(out).hexdigest()[:12]


def seconds_on_hash(target, h, path=LOG):
    """Tổng giây phiên `ok` của `target` có `src == h` (dòng cũ không có `src` KHÔNG được tính)."""
    t = 0
    if os.path.exists(path):
        for ln in open(path, encoding="utf-8"):
            ln = ln.strip()
            if ln.startswith("{"):
                d = json.loads(ln)
                if d.get("target") == target and d.get("status") == "ok" and d.get("src") == h:
                    t += d["seconds"]
    return t


def totals(path=LOG):
    t = {}
    if not os.path.exists(path):
        return t
    for ln in open(path, encoding="utf-8"):
        ln = ln.strip()
        if not ln.startswith("{"):
            continue
        d = json.loads(ln)
        k = t.setdefault(d["target"], {"seconds": 0, "runs": 0, "sessions": 0, "historical_seconds": 0, "fail": 0, "skipped_busy": 0})
        s = d.get("status")
        if s == "ok":
            k["seconds"] += d["seconds"]
            k["runs"] += d["runs"]
            k["sessions"] += 1
        elif s == "historical":
            k["seconds"] += d["seconds"]
            k["historical_seconds"] += d["seconds"]
            k["runs"] += d["runs"]
            k["sessions"] += d.get("sessions", 1)
        elif s == "fail":
            k["fail"] += 1
        elif s == "skipped_busy":
            k["skipped_busy"] += 1
    return t


def hashed_files(target):
    """Danh sách tệp (đường dẫn trong chỉ mục git) mà băm nguồn của `target` bao phủ: đổi bất kỳ tệp nào ⇒ băm đổi ⇒ giờ cũ KHÔNG còn tính."""
    out = subprocess.run(["git", "ls-files", *SRC_DIRS[target]], cwd=ROOT, capture_output=True, text=True, encoding="utf-8").stdout
    return [x for x in out.splitlines() if x]


def main():
    sys.stdout.reconfigure(encoding="utf-8")
    if "--kind-done" in sys.argv:
        k = sys.argv[sys.argv.index("--kind-done") + 1]
        sys.exit(0 if seconds_on_hash(k, current_hash(k)) >= TARGET_HOURS * 3600 else 1)
    if "--list-files" in sys.argv:
        for target in ("lex", "parse"):
            print(f"{target}: {len(hashed_files(target))} tệp được băm (băm {current_hash(target)}):")
            for f in hashed_files(target):
                print("  " + f)
        return
    if "--done-on-current-hash" in sys.argv:
        ok = all(seconds_on_hash(k, current_hash(k)) >= TARGET_HOURS * 3600 for k in ("lex", "parse"))
        sys.exit(0 if ok else 1)
    t = totals()
    if "--json" in sys.argv:
        print(json.dumps(t, sort_keys=True))
        return
    for target in ("lex", "parse"):
        k = t.get(target, {"seconds": 0, "runs": 0, "sessions": 0, "historical_seconds": 0, "fail": 0, "skipped_busy": 0})
        h = k["seconds"] / 3600
        print(
            f"{target}: {h:.2f} giờ tích luỹ / {TARGET_HOURS:.0f} giờ ({100 * h / TARGET_HOURS:.1f}%), {k['sessions']} phiên "
            f"(trong đó {k['historical_seconds'] / 3600:.2f} giờ là số liệu cũ ghi tay), {k['runs']} lượt, phiên lỗi {k['fail']}, bỏ qua do máy bận {k['skipped_busy']}"
        )
    for target in ("lex", "parse"):
        h = current_hash(target)
        print(f"  {target}: GIỜ TRÊN BĂM HIỆN TẠI {h}: {seconds_on_hash(target, h) / 3600:.2f} giờ / {TARGET_HOURS:.0f}  ({len(hashed_files(target))} tệp được băm; `python tools/fuzz_total.py --list-files` để xem)")


if __name__ == "__main__":
    main()
