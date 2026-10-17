#!/usr/bin/env python3
"""fail_open_scan: quét TẤT ĐỊNH các chỗ "im lặng chấp nhận" theo danh sách tìm kiếm của FAIL_OPEN_AUDIT và theo dõi chỗ nào đã RÀ bằng DẤU VÂN TAY (R3a).

Danh sách tìm kiếm: unwrap_or* (unwrap_or, unwrap_or_default, unwrap_or_else), `_ =>`, continue/return sớm sau `else {`, `.ok()`, `let _ =`, `.get(`, map_or, is_some_and, is_none_or.
Dấu vân tay = sha1(đường dẫn + dòng khớp + 2 dòng ngữ cảnh trước/sau đã chuẩn hoá khoảng trắng + số thứ tự trùng trong tệp). Sổ cái: docs/FAIL_OPEN_LEDGER.jsonl
(mỗi dòng: fp, file, line, text, verdict ∈ an_toan|da_sua|nghi_van|khong_ra_duoc, reason). Chỗ khớp nằm trong khối `#[cfg(test)]` (đến cuối tệp) bị bỏ qua.
Tệp bị sửa QUANH chỗ đã rà (dòng khớp hoặc 2 dòng ngữ cảnh đổi) ⇒ dấu vân tay đổi ⇒ chỗ đó quay lại CHƯA RÀ; sửa chỗ khác trong tệp không ảnh hưởng.

  python tools/fail_open_scan.py stats [--stamp]            in "đã rà / tổng" theo tệp (--stamp ghi dòng đếm vào đầu docs/FAIL_OPEN_AUDIT.md)
  python tools/fail_open_scan.py list [--file S] [--all]    in chỗ CHƯA RÀ (hoặc tất cả) kèm ngữ cảnh
  python tools/fail_open_scan.py mark --file S --verdict V --reason R [--lines a-b]   đánh dấu các chỗ CHƯA RÀ trong tệp (khớp S) và khoảng dòng
  python tools/fail_open_scan.py check                      thoát 1 nếu còn chỗ CHƯA RÀ (trừ khi đã ghi khong_ra_duoc)
"Đọc mã" khác "chứng minh": ledger ghi kết quả ĐỌC, không phải chứng minh."""
import argparse
import hashlib
import io
import json
import os
import re
import sys

ROOT = os.environ.get("FOS_ROOT") or os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
LEDGER = os.path.join(ROOT, "docs", "FAIL_OPEN_LEDGER.jsonl")
AUDIT = os.path.join(ROOT, "docs", "FAIL_OPEN_AUDIT.md")
SRC_ROOTS = ["crates", os.path.join("runtime", "gt-rt", "src")]
PATTERNS = [
    ("unwrap_or", re.compile(r"\bunwrap_or(_default|_else)?\b")),
    ("_ =>", re.compile(r"^\s*_\s*(if [^=]*)?=>")),
    (".ok()", re.compile(r"\.ok\(\)")),
    ("let _ =", re.compile(r"\blet\s+_\s*=")),
    (".get(", re.compile(r"\.get\(")),
    ("map_or", re.compile(r"\bmap_or(_else)?\b")),
    ("is_some_and", re.compile(r"\bis_some_and\b")),
    ("is_none_or", re.compile(r"\bis_none_or\b")),
    ("else-early", re.compile(r"\belse\s*\{\s*(continue|return)\b")),
]
EARLY = re.compile(r"^\s*(continue|return)\b")


def src_files():
    out = []
    for base in SRC_ROOTS:
        for dp, dn, fn in os.walk(os.path.join(ROOT, base)):
            dn[:] = [d for d in dn if d not in ("target", "tests", "fuzz", "benches", ".git")]
            if os.sep + "src" not in dp + os.sep and not dp.endswith("src"):
                continue
            for f in fn:
                if f.endswith(".rs"):
                    out.append(os.path.join(dp, f))
    return sorted(out)


def norm(s):
    return re.sub(r"\s+", " ", s.strip())


def scan():
    hits = []
    for path in src_files():
        rel = os.path.relpath(path, ROOT).replace("\\", "/")
        lines = io.open(path, encoding="utf-8", newline="").read().replace("\r\n", "\n").split("\n")
        end = len(lines)
        for i, l in enumerate(lines):
            if l.startswith("#[cfg(test)]"):
                end = i
                break
        seen = {}
        for i in range(end):
            l = lines[i]
            code = l.split("//")[0] if "//" in l else l
            if not code.strip():
                continue
            names = [n for n, rx in PATTERNS if rx.search(code)]
            # `continue`/`return` ở dòng ngay sau `else {` (rustfmt xuống dòng)
            if EARLY.match(code) and i > 0 and lines[i - 1].rstrip().endswith("else {"):
                names.append("else-early")
            if not names:
                continue
            ctx = [norm(lines[j]) if 0 <= j < len(lines) else "" for j in range(i - 2, i + 3)]
            key = rel + "|" + "|".join(ctx)
            n = seen.get(key, 0)
            seen[key] = n + 1
            fp = hashlib.sha1((key + "|" + str(n)).encode("utf-8")).hexdigest()[:14]
            hits.append({"fp": fp, "file": rel, "line": i + 1, "text": norm(l)[:160], "pat": ",".join(names), "ctx": lines[max(0, i - 2):i + 3]})
    return hits


def load_ledger():
    d = {}
    if os.path.exists(LEDGER):
        for ln in io.open(LEDGER, encoding="utf-8"):
            if ln.strip():
                try:
                    r = json.loads(ln)
                    d[r["fp"]] = r
                except (ValueError, KeyError):
                    pass
    return d


def counts(hits, led):
    per = {}
    for h in hits:
        c = per.setdefault(h["file"], [0, 0])
        c[1] += 1
        if h["fp"] in led:
            c[0] += 1
    return per


def cmd_stats(a):
    hits, led = scan(), load_ledger()
    per = counts(hits, led)
    tot = sum(c[1] for c in per.values())
    done = sum(c[0] for c in per.values())
    stale = len([k for k in led if k not in {h["fp"] for h in hits}])
    print(f"đã rà / tổng = {done} / {tot}  (CHƯA RÀ {tot - done}; dòng sổ cái lỗi thời {stale})")
    for f, (d, t) in sorted(per.items(), key=lambda kv: (kv[1][0] - kv[1][1], kv[0])):
        if d < t or a.verbose:
            print(f"  {f}: {d}/{t}")
    if a.stamp:
        line = f"<!-- FOS:BEGIN -->**Đã rà / tổng chỗ khớp = {done} / {tot}** (CHƯA RÀ {tot - done}; `python tools/fail_open_scan.py stats`; dấu vân tay ở `docs/FAIL_OPEN_LEDGER.jsonl`). \"Đọc mã\" KHÁC \"chứng minh\".<!-- FOS:END -->"
        s = io.open(AUDIT, encoding="utf-8", newline="").read()
        crlf = "\r\n" in s
        t = s.replace("\r\n", "\n")
        if "<!-- FOS:BEGIN -->" in t:
            t = re.sub(r"<!-- FOS:BEGIN -->.*?<!-- FOS:END -->", lambda m: line, t, flags=re.S)
        else:
            i = t.index("\n") + 1
            t = t[:i] + "\n" + line + "\n" + t[i:]
        io.open(AUDIT, "w", encoding="utf-8", newline="").write(t.replace("\n", "\r\n") if crlf else t)
        print("đã ghi dòng đếm vào docs/FAIL_OPEN_AUDIT.md")
    return 0


def cmd_list(a):
    hits, led = scan(), load_ledger()
    n = 0
    for h in hits:
        if a.file and a.file not in h["file"]:
            continue
        if h["fp"] in led and not a.all:
            continue
        if a.limit and n >= a.limit:
            n += 1
            continue
        n += 1
        print(f"--- {h['file']}:{h['line']} [{h['pat']}] fp={h['fp']}" + (f"  (đã rà: {led[h['fp']]['verdict']})" if h["fp"] in led else ""))
        c = h["ctx"]
        mid = 2 if h["line"] > 2 else h["line"] - 1
        for k in range(max(0, mid - a.before), min(len(c), mid + a.after + 1)):
            print((" >" if k == mid else "  ") + c[k].rstrip()[:150])
    print(f"== {n} chỗ")
    return 0


def parse_lines(s):
    if not s:
        return None
    out = []
    for part in s.split(","):
        if "-" in part:
            x, y = part.split("-")
            out.append((int(x), int(y)))
        else:
            out.append((int(part), int(part)))
    return out


def cmd_mark(a):
    hits, led = scan(), load_ledger()
    rng = parse_lines(a.lines)
    n = 0
    with io.open(LEDGER, "a", encoding="utf-8", newline="\n") as f:
        for h in hits:
            if a.file not in h["file"] or h["fp"] in led:
                continue
            if rng and not any(x <= h["line"] <= y for x, y in rng):
                continue
            f.write(json.dumps({"fp": h["fp"], "file": h["file"], "line": h["line"], "text": h["text"], "pat": h["pat"], "verdict": a.verdict, "reason": a.reason}, ensure_ascii=False) + "\n")
            n += 1
    print(f"đã đánh dấu {n} chỗ ({a.verdict})")
    return 0


def cmd_check(a):
    hits, led = scan(), load_ledger()
    un = [h for h in hits if h["fp"] not in led]
    nv = [h for h in hits if h["fp"] in led and led[h["fp"]]["verdict"] == "nghi_van"]
    print(f"CHƯA RÀ {len(un)}; nghi vấn chưa xử lý {len(nv)}")
    return 1 if un or nv else 0


def main():
    sys.stdout.reconfigure(encoding="utf-8")
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="cmd", required=True)
    s = sub.add_parser("stats")
    s.add_argument("--stamp", action="store_true")
    s.add_argument("--verbose", action="store_true")
    l = sub.add_parser("list")
    l.add_argument("--file")
    l.add_argument("--all", action="store_true")
    l.add_argument("--before", type=int, default=1)
    l.add_argument("--after", type=int, default=1)
    l.add_argument("--limit", type=int, default=0)
    m = sub.add_parser("mark")
    m.add_argument("--file", required=True)
    m.add_argument("--verdict", required=True, choices=["an_toan", "da_sua", "nghi_van", "khong_ra_duoc"])
    m.add_argument("--reason", required=True)
    m.add_argument("--lines")
    sub.add_parser("check")
    a = ap.parse_args()
    sys.exit({"stats": cmd_stats, "list": cmd_list, "mark": cmd_mark, "check": cmd_check}[a.cmd](a))


if __name__ == "__main__":
    main()
