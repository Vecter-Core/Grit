#!/usr/bin/env python3
"""Tổng fuzz TÍCH LUỸ từ docs/FUZZ_LOG.jsonl (ADR-037). Dùng: python tools/fuzz_total.py [--json]
Chỉ tính các dòng status=ok (phiên chạy hết giờ, không lỗi) và status=historical (số liệu cũ ghi tay từ PROGRESS.md, gắn nhãn riêng).
Phiên `fail` KHÔNG được tính vào giờ nhưng được báo (và phải được điều tra); `skipped_busy` chỉ đếm số lần bỏ qua."""
import json, os, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
LOG = os.path.join(ROOT, "docs", "FUZZ_LOG.jsonl")
TARGET_HOURS = 24.0  # GRIT_PLAN §7 #2 (sửa bởi ADR-037): mỗi loại (lexer, parser)


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


def main():
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


if __name__ == "__main__":
    main()
