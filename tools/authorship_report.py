#!/usr/bin/env python3
"""Count TCB source lines by the G2 authorship rule; emit Markdown to stdout."""
from __future__ import annotations

import re
import subprocess
from collections import Counter, defaultdict

BASE = "0a93310"
CLAUDE = "origin/claude/gallant-darwin-m251cq"
GROUPS = {
    "gt-rt": ["runtime/gt-rt"],
    "gt-codegen-llvm": ["crates/gt-codegen-llvm"],
    "gt-guard": ["crates/gt-guard"],
    "gt-borrowck": ["crates/gt-borrowck"],
    "gt-types": ["crates/gt-types"],
    "gt-oblig": ["crates/gt-oblig"],
    "gt-term": ["crates/gt-term"],
    "gt-effects": ["crates/gt-effects"],
    "gt-mir": ["crates/gt-mir"],
    "gt-driver": ["crates/gt-driver"],
}


def git(*args: str) -> str:
    return subprocess.check_output(["git", *args], text=True, encoding="utf-8").strip()


def main() -> int:
    head = git("rev-parse", "HEAD")
    base = git("rev-parse", BASE)
    claude_commits = set(git("rev-list", CLAUDE).splitlines())
    codex_commits = set(git("rev-list", f"{base}..{head}").splitlines()) - claude_commits
    totals: dict[str, Counter[str]] = defaultdict(Counter)
    for group, roots in GROUPS.items():
        files = git("ls-files", "--", *roots).splitlines()
        files = [f for f in files if f.endswith(".rs")]
        for path in files:
            blame = git("blame", "--line-porcelain", "HEAD", "--", path)
            for line in blame.splitlines():
                m = re.match(r"^([0-9a-f]{40}) \d+ \d+(?: \d+)?$", line)
                if not m:
                    continue
                commit = m.group(1)
                label = "Codex" if commit in codex_commits else "Claude"
                totals[group][label] += 1
    print(f"# TCB code authorship\n\nBase: `{base}`; HEAD: `{head}`; Claude ancestry: `{CLAUDE}`.\n")
    print("| Nhóm | Codex dòng | Claude dòng | Tổng | Codex % |")
    print("|---|---:|---:|---:|---:|")
    all_counts: Counter[str] = Counter()
    for group, counts in totals.items():
        codex, claude = counts["Codex"], counts["Claude"]
        total = codex + claude
        all_counts.update(counts)
        pct = 100 * codex / total if total else 0.0
        print(f"| `{group}` | {codex} | {claude} | {total} | {pct:.2f}% |")
    total = sum(all_counts.values())
    pct = 100 * all_counts["Codex"] / total if total else 0.0
    print(f"| **Tổng** | **{all_counts['Codex']}** | **{all_counts['Claude']}** | **{total}** | **{pct:.2f}%** |")
    print("\nĐếm dòng `git blame`, gồm merge/conflict-resolution commit theo quy tắc; không suy ra model cụ thể cho từng commit.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
