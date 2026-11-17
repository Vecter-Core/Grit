#!/usr/bin/env python3
"""Đo ĐỌC CHÉO theo vùng TCB (GÓI 3 S3.a). In Markdown ra stdout.

Phân loại tác giả từng dòng (git blame tại HEAD), theo thứ tự ưu tiên:
  1. trailer `Author-Model:` của commit (GPT-6 => GPT-6; Claude ... => Claude);
  2. commit thuộc tổ tiên nhánh Linux `origin/claude/gallant-darwin-m251cq` hoặc có từ trước BASE => Claude (không nhãn);
  3. commit còn lại trong BASE..HEAD KHÔNG nhãn => "Windows không nhãn" (có thể Codex GPT-6 HOẶC Claude phiên Windows; git không lưu model).
Đọc chéo: người đọc phải KHÁC họ với tác giả dòng.
  - G1.5 (GPT-6, đọc tại G15_HASH): phạm vi tính = gt-rt, gt-guard (gt-codegen-llvm: tài liệu chỉ ghi "phần hạ liên quan", không xác nhận được => không tính). Dòng tác giả Claude có commit <= G15_HASH => "GPT-6 đã đọc".
  - R11b (Claude Sonnet 5.5, đọc tại R11B_HASH): phạm vi = diff của Codex ở gt-rt/gt-guard. Dòng "GPT-6"/"Windows không nhãn" ở hai crate đó
    có commit <= R11B_HASH => "Claude đã đọc" (chỉ theo phạm vi TÀI LIỆU, không xác nhận từng dòng).
  - Mọi dòng khác => "CHƯA đọc chéo" (kèm cờ: ngoài phạm vi, hoặc đổi SAU lần đọc).
HẠN CHẾ: blame theo dòng cuối cùng chạm; dòng bị di chuyển/format lại mang commit mới (đếm vào "đổi sau"); trailer chỉ có ở 40/471 commit.
"""
from __future__ import annotations

import re
import subprocess
import sys
from collections import Counter, defaultdict

BASE = "0a93310"
LINUX = "origin/claude/gallant-darwin-m251cq"
G15_HASH = "c5aa45b"
R11B_HASH = "f9f0f78"
GROUPS = {
    "gt-rt": ["runtime/gt-rt"],
    "gt-guard": ["crates/gt-guard"],
    "gt-codegen-llvm": ["crates/gt-codegen-llvm"],
    "gt-borrowck": ["crates/gt-borrowck"],
    "gt-types": ["crates/gt-types"],
    "gt-oblig": ["crates/gt-oblig"],
    "gt-term": ["crates/gt-term"],
    "gt-effects": ["crates/gt-effects"],
    "gt-mir": ["crates/gt-mir"],
    "gt-driver": ["crates/gt-driver"],
}
G15_SCOPE = {"gt-rt", "gt-guard"}   # gt-codegen-llvm: G1.5 chỉ ghi "phần hạ LLVM liên quan" (không rõ tệp) => KHÔNG tính là đã đọc
R11B_SCOPE = {"gt-rt", "gt-guard"}


def git(*args: str) -> str:
    return subprocess.check_output(["git", *args], text=True, encoding="utf-8", errors="replace").strip()


def main() -> int:
    head = git("rev-parse", "HEAD")
    base = git("rev-parse", BASE)
    linux = set(git("rev-list", LINUX).splitlines())
    windows = set(git("rev-list", f"{base}..{head}").splitlines()) - linux
    anc_g15 = set(git("rev-list", G15_HASH).splitlines())
    anc_r11b = set(git("rev-list", R11B_HASH).splitlines())
    trailer: dict[str, str] = {}
    for rec in git("log", "--format=%H\t%(trailers:key=Author-Model,valueonly,separator=;)").splitlines():
        h, _, v = rec.partition("\t")
        if v.strip():
            trailer[h] = v.strip()

    def author(c: str) -> str:
        t = trailer.get(c)
        if t:
            return "GPT-6" if re.search(r"gpt|codex", t, re.I) else "Claude"
        if c in windows:
            return "Windows-không-nhãn"
        return "Claude-không-nhãn"

    rows: dict[str, Counter] = defaultdict(Counter)
    for group, roots in GROUPS.items():
        files = [f for f in git("ls-files", "--", *roots).splitlines() if f.endswith(".rs")]
        for path in files:
            blame = git("blame", "--line-porcelain", "HEAD", "--", path)
            for line in blame.splitlines():
                m = re.match(r"^([0-9a-f]{40}) \d+ \d+(?: \d+)?$", line)
                if not m:
                    continue
                c = m.group(1)
                a = author(c)
                rows[group]["total"] += 1
                rows[group]["a:" + a] += 1
                claude_side = a.startswith("Claude")
                if group in G15_SCOPE and claude_side and c in anc_g15:
                    rows[group]["read_gpt"] += 1
                elif group in R11B_SCOPE and not claude_side and c in anc_r11b:
                    rows[group]["read_claude"] += 1
                else:
                    if group not in G15_SCOPE | R11B_SCOPE:
                        rows[group]["unread_outofscope"] += 1
                    elif c not in anc_g15 and c not in anc_r11b:
                        rows[group]["unread_changed_after"] += 1
                    else:
                        rows[group]["unread_other"] += 1
    print("# CROSS_REVIEW_COVERAGE (tạo bằng `python tools/cross_review_coverage.py`)\n")
    print(f"HEAD đo: `{head[:7]}`; BASE `{base[:7]}`; G1.5 (GPT-6 đọc) tại `{G15_HASH}`; R11b (Claude đọc) tại `{R11B_HASH}`.\n")
    print("Tác giả theo trailer `Author-Model` nếu có, không thì theo tổ tiên (xem đầu `tools/cross_review_coverage.py`).")
    print("\n| crate | tổng dòng | Claude (nhãn) | Claude (không nhãn) | GPT-6 (nhãn) | Windows không nhãn | GPT-6 đã đọc | Claude đã đọc | CHƯA: đổi sau lần đọc | CHƯA: ngoài phạm vi | CHƯA: khác |")
    print("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|")
    tot: Counter = Counter()
    for g, c in rows.items():
        tot.update(c)
        print(f"| `{g}` | {c['total']} | {c['a:Claude']} | {c['a:Claude-không-nhãn']} | {c['a:GPT-6']} | {c['a:Windows-không-nhãn']} | "
              f"{c['read_gpt']} | {c['read_claude']} | {c['unread_changed_after']} | {c['unread_outofscope']} | {c['unread_other']} |")
    print(f"| **Tổng** | **{tot['total']}** | {tot['a:Claude']} | {tot['a:Claude-không-nhãn']} | {tot['a:GPT-6']} | {tot['a:Windows-không-nhãn']} | "
          f"**{tot['read_gpt']}** | **{tot['read_claude']}** | {tot['unread_changed_after']} | {tot['unread_outofscope']} | {tot['unread_other']} |")
    read = tot["read_gpt"] + tot["read_claude"]
    print(f"\n**Đọc chéo theo phạm vi tài liệu:** {read}/{tot['total']} dòng ({100 * read / max(1, tot['total']):.1f}%). "
          f"**Chưa đọc chéo:** {tot['total'] - read} dòng, trong đó NGOÀI phạm vi G1.5/R11b (checker M6/M7/M8/kiểu/hiệu ứng/MIR/driver) = {tot['unread_outofscope']}.")
    sys.stdout.flush()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
