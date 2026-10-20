#!/usr/bin/env python3
"""Kiểm METAMORPHIC thứ tự khai báo (GRIT_PLAN §9, M3 "độc lập thứ tự"): xáo thứ tự các item cấp cao của một tệp KHÔNG được đổi tập mã chẩn đoán.
Tách tệp thành khối bắt đầu ở dòng cột 0 có tiền tố item (ƒ ▢ ⊔ ⊐ ⊏ ≡ □ ■ ▦ ⇲ ◈ # ///), giữ dòng `// …` đầu tệp ở nguyên chỗ, xáo khối bằng hạt giống cố định
(2 hoán vị mỗi mẫu), so (verdict, tập mã). Mẫu có `⇲` (use) hoặc nhiều tệp/mô-đun bị bỏ qua vì thứ tự `⇲` có thể là quy ước. Dùng: order_meta.py --gritc G"""
import argparse, glob, json, os, random, shutil, subprocess, sys, tempfile

STARTS = ("ƒ", "▢", "⊔", "⊐", "⊏", "≡", "□", "■", "▦", "◈", "#", "///")


def blocks(src):
    lines = src.split("\n")
    head, i = [], 0
    while i < len(lines) and (lines[i].startswith("//") and not lines[i].startswith("///")):
        head.append(lines[i]); i += 1
    chunks, cur = [], []
    for l in lines[i:]:
        if l.startswith(STARTS) and cur and not cur[-1].startswith(("#", "///")):
            chunks.append(cur); cur = []
        cur.append(l)
    if cur:
        chunks.append(cur)
    return head, chunks


def check(g, path):
    r = subprocess.run([g, "check", path, "--json"], capture_output=True, text=True, timeout=120)
    try:
        j = json.loads(r.stdout)
    except Exception:
        return ("NOJSON", r.returncode)
    return (j.get("verdict"), tuple(sorted(set(d.get("code") for d in j.get("diagnostics", [])))))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--gritc", required=True)
    ap.add_argument("--tests", default=os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "tests"))
    a = ap.parse_args()
    rng = random.Random(42)
    n = same = skipped = 0
    bad = []
    tmp = tempfile.mkdtemp(prefix="ordermeta_")
    for kind in ("pass", "fail", "warn"):
        for f in sorted(glob.glob(os.path.join(a.tests, kind, "*.gt"))):
            base = os.path.basename(f)
            src = open(f, encoding="utf-8").read()
            head, ch = blocks(src)
            if len(ch) < 3 or "⇲" in src or base.startswith(("E12", "E0101", "E0102")):
                skipped += 1
                continue
            d = os.path.join(tmp, "c"); shutil.rmtree(d, ignore_errors=True); os.makedirs(d)
            p = os.path.join(d, "main.gt")
            open(p, "w", encoding="utf-8").write(src)
            before = check(a.gritc, p)
            n += 1
            ok = True
            for _ in range(2):
                perm = ch[:]
                rng.shuffle(perm)
                open(p, "w", encoding="utf-8").write("\n".join(head + [l for c in perm for l in c]))
                after = check(a.gritc, p)
                if after != before:
                    ok = False
                    bad.append((base, before, after))
                    break
            same += ok
    print(f"SUMMARY: order metamorphic: {n} mẫu (2 hoán vị mỗi mẫu); giống {same}; KHÁC {len(bad)}; bỏ qua {skipped}")
    for b in bad[:30]:
        print("KHÁC", b)
    shutil.rmtree(tmp, ignore_errors=True)
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
