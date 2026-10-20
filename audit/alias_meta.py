#!/usr/bin/env python3
"""Kiểm METAMORPHIC alias ASCII ↔ canonical (GRIT_PLAN §4.1, §9): đổi mọi ký hiệu canonical sang bí danh ASCII 1–1 (docs/spec/symbols.md) KHÔNG được đổi
phán quyết. Với mỗi mẫu đơn trong tests/{pass,fail,warn}: so (verdict, tập mã) của bản gốc với bản đã thay ký hiệu (mỗi bí danh được bao bằng dấu cách để
không dính vào định danh kề, vd `μs` → `\\mut  s`). Bỏ qua mẫu cố ý độc hại Unicode (E12xx). Dùng: alias_meta.py --gritc G"""
import argparse, glob, json, os, re, shutil, subprocess, sys, tempfile

MAP = {"ƒ": r"\fn", "≔": ":=", "μ": r"\mut", "←": "<-", "→": "->", "⟳": r"\loop", "↓": r"\dec", "⊳": r"\pre", "⊲": r"\post", "⋈": r"\inv",
       "⚠": r"\unsafe", "⊢": r"\proof", "∅": r"\nil", "⎇": r"\if", "⇝": r"\else", "⌥": r"\match", "↦": "=>", "⏎": r"\ret", "▢": r"\struct",
       "⊔": r"\enum", "▦": r"\mod", "⇲": r"\use", "◈": r"\pub", "⊏": r"\trait", "⊐": r"\impl", "⊠": r"\break", "⊡": r"\cont", "≡": r"\type",
       "□": r"\const", "■": r"\static", "∧": r"\land", "∨": r"\lor", "¬": r"\neg", "⇒": r"\implies", "∀": r"\forall", "∃": r"\exists",
       "≤": "<=", "≥": ">=", "≠": "!=", "∈": r"\in"}


def to_ascii(src):
    out = []
    for line in src.split("\n"):
        i = line.find("//")
        code, cm = (line, "") if i < 0 else (line[:i], line[i:])
        # không đụng vào literal chuỗi/ký tự: tách theo dấu nháy kép đơn giản
        parts = re.split(r'("(?:[^"\\]|\\.)*")', code)
        for k in range(0, len(parts), 2):
            parts[k] = "".join(f" {MAP[c]} " if c in MAP else c for c in parts[k])
        out.append("".join(parts) + cm)
    return "\n".join(out)


def check(g, path):
    r = subprocess.run([g, "check", path, "--json"], capture_output=True, text=True, timeout=120)
    try:
        j = json.loads(r.stdout)
    except Exception:
        return ("NOJSON", r.returncode)
    return (j.get("verdict"), tuple(sorted(d.get("code") for d in j.get("diagnostics", []))))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--gritc", required=True)
    ap.add_argument("--tests", default=os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "tests"))
    a = ap.parse_args()
    n = same = skipped = 0
    bad = []
    tmp = tempfile.mkdtemp(prefix="aliasmeta_")
    for kind in ("pass", "fail", "warn"):
        for f in sorted(glob.glob(os.path.join(a.tests, kind, "*.gt"))):
            base = os.path.basename(f)
            if base.startswith(("E12", "E0101", "E0102")):
                skipped += 1
                continue
            n += 1
            d = os.path.join(tmp, "c"); shutil.rmtree(d, ignore_errors=True); os.makedirs(d)
            p = os.path.join(d, "main.gt")
            src = open(f, encoding="utf-8").read()
            open(p, "w", encoding="utf-8").write(src)
            before = check(a.gritc, p)
            open(p, "w", encoding="utf-8").write(to_ascii(src))
            after = check(a.gritc, p)
            if before == after:
                same += 1
            else:
                bad.append((base, before, after))
    print(f"SUMMARY: alias ASCII ≡ canonical: {n} mẫu; giống {same}; KHÁC {len(bad)}; bỏ qua {skipped} (lỗi cú pháp/Unicode cố ý)")
    for b in bad[:30]:
        print("KHÁC", b)
    shutil.rmtree(tmp, ignore_errors=True)
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
