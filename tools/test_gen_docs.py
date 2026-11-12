"""Mọi hàm (`ƒ tên`) ở prelude/std phải có MỘT mục trong docs/STD_REFERENCE.md (đếm độc lập với bộ phân tích: tránh bỏ sót impl viết gọn một dòng)."""
import os
import re
import subprocess
import sys
import unittest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
STRING_RE = re.compile(r'"(?:\\.|[^"\\])*"')


def count_fns(path):
    n = 0
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = STRING_RE.sub('""', line)
            i = line.find("//")
            if i >= 0:
                line = line[:i]
            n += len(re.findall(r"ƒ\s+[A-Za-z_]", line))
    return n


class GenDocs(unittest.TestCase):
    def test_every_function_has_an_entry(self):
        doc = open(os.path.join(ROOT, "docs", "STD_REFERENCE.md"), encoding="utf-8").read()
        entries = len(re.findall(r"^- `ƒ ", doc, re.M))
        internal = len(re.findall(r"`(?:parse_digits|first_non_ws|end_after_last_non_ws|swap_at|sift_down|next_cap)`", doc.split("### Nội bộ")[-1])) if "### Nội bộ" in doc else 0
        total = count_fns(os.path.join(ROOT, "lib/core/prelude.gt")) + count_fns(os.path.join(ROOT, "lib/std/src/std.gt"))
        # trait method khai báo trong `⊏ … { ƒ cmp(...); }` nằm trong mục Kiểu (không phải mục `ƒ` riêng): trừ chúng
        trait_decls = 0
        for p in ("lib/core/prelude.gt", "lib/std/src/std.gt"):
            for line in open(os.path.join(ROOT, p), encoding="utf-8"):
                if re.match(r"\s*(◈ )?⊏ ", line):
                    trait_decls += len(re.findall(r"ƒ\s+[A-Za-z_]", line))
        self.assertEqual(entries + internal + trait_decls, total, f"mục={entries} nội bộ={internal} trait={trait_decls} nguồn={total}: bộ sinh bỏ sót hàm")


if __name__ == "__main__":
    unittest.main()
