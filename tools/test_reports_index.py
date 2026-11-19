"""GÓI 8 S0e: test tools/reports_index.py (chuỗi tổng hợp, không đọc docs thật)."""
import os
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import reports_index as R

SAMPLE = "\n".join([
    "# Báo cáo",
    "## GÓI 3 (2026-11-18; Claude)",
    "nội dung a",
    "```",
    "## không phải tiêu đề (trong khối mã)",
    "```",
    "## GÓI 1 (2026-11-10)",
    "nội dung b",
    "nội dung c",
    "## Ghi chú không ngày",
    "",
])


class Index(unittest.TestCase):
    def test_sections_keep_file_order_and_dates(self):
        s = R.sections(SAMPLE)
        self.assertEqual([x[0] for x in s], ["GÓI 3 (2026-11-18; Claude)", "GÓI 1 (2026-11-10)", "Ghi chú không ngày"])
        self.assertEqual([x[1] for x in s], ["2026-11-18", "2026-11-10", None])

    def test_line_numbers_and_counts(self):
        s = R.sections(SAMPLE)
        self.assertEqual(s[0][2], 2)           # dòng bắt đầu GÓI 3
        self.assertEqual(s[1][2], 7)           # GÓI 1 (dòng 6 là đóng khối mã, bỏ qua dòng ``` 4-6)
        self.assertEqual(s[1][3], 3)           # GÓI 1: tiêu đề + 2 dòng

    def test_render_has_table_and_count(self):
        out = R.render(SAMPLE, "X.md")
        self.assertIn("| 1 | GÓI 3 (2026-11-18; Claude) | 2026-11-18 | 2 |", out)
        self.assertIn("| 3 | Ghi chú không ngày | — |", out)
        self.assertIn("Tổng: 3 mục.", out)

    def test_pipe_in_title_is_escaped(self):
        out = R.render("## A | B (2026-01-01)\nx\n", "X.md")
        self.assertIn("A \\| B", out)

    def test_repo_index_is_current(self):
        # tệp chỉ mục thật phải khớp nguồn (cổng: reports_index.py --check)
        import subprocess
        r = subprocess.run([sys.executable, os.path.join(HERE, "reports_index.py"), "--check"], capture_output=True, text=True, encoding="utf-8")
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)


if __name__ == "__main__":
    unittest.main()
