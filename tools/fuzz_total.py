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


def seconds_on_hash_by_source(target, h, path=LOG):
    """Như `seconds_on_hash` nhưng tách theo nguồn (`source`; dòng không có trường này = "windows"). Trả dict nguồn -> giây ĐỒNG HỒ."""
    t = {}
    if os.path.exists(path):
        for ln in open(path, encoding="utf-8"):
            ln = ln.strip()
            if ln.startswith("{"):
                d = json.loads(ln)
                if d.get("target") == target and d.get("status") == "ok" and d.get("src") == h:
                    k = d.get("source", "windows")
                    t[k] = t.get(k, 0) + d["seconds"]
    return t


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


def last_session(target, path=LOG):
    """Dòng log CUỐI của `target` (mọi trạng thái trừ `historical`), hoặc None."""
    last = None
    if os.path.exists(path):
        for ln in open(path, encoding="utf-8"):
            ln = ln.strip()
            if ln.startswith("{"):
                d = json.loads(ln)
                if d.get("target") == target and d.get("status") != "historical":
                    last = d
    return last


def failures(target, path=LOG):
    """Các phiên `fail` của `target`."""
    out = []
    if os.path.exists(path):
        for ln in open(path, encoding="utf-8"):
            ln = ln.strip()
            if ln.startswith("{"):
                d = json.loads(ln)
                if d.get("target") == target and d.get("status") == "fail":
                    out.append(d)
    return out


def summary_text(hashes, path=LOG, now=None):
    """GÓI 14 E1: MỘT màn hình cho Giang dán lại. `hashes` = {mục tiêu: băm nguồn hiện tại}. Ước tính đồng hồ treo tường, giả định mỗi mục tiêu chạy với tốc độ thật 1 giờ log/giờ đồng hồ
    (tuần tự: hai phần xen kẽ nên tổng = thiếu_lex + thiếu_parse; song song: hai đồng hồ riêng nên = max). Đây là SUY RA, không phải đo."""
    import datetime as _dt
    now = now or _dt.datetime.now(_dt.timezone.utc)
    lines = ["FUZZ_SUMMARY (băm nguồn HIỆN TẠI; chỉ phiên status=ok trên băm này mới tính)"]
    missing = {}
    for tg in ("lex", "parse"):
        h = hashes[tg]
        sec = seconds_on_hash(tg, h, path)
        by = seconds_on_hash_by_source(tg, h, path)
        miss = max(0.0, TARGET_HOURS - sec / 3600)
        missing[tg] = miss
        src = ", ".join(f"{k} {v / 3600:.2f} h" for k, v in sorted(by.items())) or "chưa có"
        lines.append(f"  {tg:<5} băm {h}: có {sec / 3600:.2f} h / {TARGET_HOURS:.0f} h — CÒN THIẾU {miss:.2f} h   (theo nguồn: {src})")
    for tg in ("lex", "parse"):
        ls = last_session(tg, path)
        if ls is None:
            lines.append(f"  phiên cuối {tg}: chưa có")
        else:
            lines.append(f"  phiên cuối {tg}: {ls.get('ts')} status={ls.get('status')} {ls.get('seconds')} s exit={ls.get('exit')} nguồn={ls.get('source', 'windows')}")
        fl = failures(tg, path)
        if fl:
            lines.append(f"  !! {tg}: {len(fl)} phiên LỖI (cuối: {fl[-1].get('ts')} exit={fl[-1].get('exit')}) — phải điều tra trước khi tin giờ tích luỹ")
    seq, par = missing["lex"] + missing["parse"], max(missing.values())
    fmt = lambda h: (now + _dt.timedelta(hours=h)).strftime("%Y-%m-%d %H:%M UTC")
    if seq <= 0:
        lines.append("  ĐỦ 24 giờ cả hai trên băm hiện tại (điều kiện #1 mới chỉ ĐỦ GIỜ; Giang tự đánh dấu)")
    else:
        lines.append(f"  ước tính (SUY RA): tuần tự còn {seq:.1f} h ⇒ xong ≈ {fmt(seq)}; song song (`--parallel`, cần RAM trống ≥ 10 GB, WSL: FUZZ_PAR_MIN_GB=6) còn {par:.1f} h ⇒ xong ≈ {fmt(par)}")
    return chr(10).join(lines)


def main():
    sys.stdout.reconfigure(encoding="utf-8")
    if "--kind-done" in sys.argv:
        k = sys.argv[sys.argv.index("--kind-done") + 1]
        sys.exit(0 if seconds_on_hash(k, current_hash(k)) >= TARGET_HOURS * 3600 else 1)
    if "--summary" in sys.argv:
        print(summary_text({k: current_hash(k) for k in ("lex", "parse")}))
        return
    if "--hash" in sys.argv:
        print(current_hash(sys.argv[sys.argv.index("--hash") + 1]))
        return
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
        by = seconds_on_hash_by_source(target, h)
        print(f"  {target}: theo nguồn trên băm {h}: " + (", ".join(f"{k} {v / 3600:.2f} giờ" for k, v in sorted(by.items())) or "chưa có") + " (đồng hồ treo tường)")
        print(f"  {target}: GIỜ TRÊN BĂM HIỆN TẠI {h}: {seconds_on_hash(target, h) / 3600:.2f} giờ / {TARGET_HOURS:.0f}  ({len(hashed_files(target))} tệp được băm; `python tools/fuzz_total.py --list-files` để xem)")


if __name__ == "__main__":
    main()
