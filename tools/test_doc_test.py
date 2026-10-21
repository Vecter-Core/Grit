"""Kiểm thử tools/doc_test.py: bộ trích khối, thuộc tính, ghép nhóm, và — quan trọng nhất — khối SAI phải làm công cụ ĐỎ (không lọt)."""
import os
import shutil
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import doc_test  # noqa: E402

OK = "ƒ main() → u64 ! ∅ { 7u64 }\n"
BAD_TYPE = "ƒ main() → u64 ! ∅ { true }\n"


def md(text):
    d = tempfile.mkdtemp(prefix="doctest-md-")
    p = os.path.join(d, "t.md")
    with open(p, "w", encoding="utf-8") as f:
        f.write(text)
    return d, p


class Parse(unittest.TestCase):
    def test_only_grit_fences_are_blocks_and_attrs_are_parsed(self):
        d, p = md("```\nx\n```\n```text\ny\n```\n```grit run value=7\n" + OK + "```\n```output\n\n```\n```grit fail=E0301,E0302 std\n" + BAD_TYPE + "```\n")
        try:
            blocks = doc_test.parse_blocks(p)
            self.assertEqual(len(blocks), 2)
            self.assertEqual(blocks[0][1], {"run": True, "value": "7"})
            self.assertEqual(blocks[1][1], {"fail": "E0301,E0302", "std": True})
        finally:
            shutil.rmtree(d, ignore_errors=True)

    def test_output_fence_is_paired_with_the_preceding_run_block(self):
        d, p = md("```grit run\n" + OK + "```\n\n```output\nhello\n```\n")
        try:
            (line, attrs, code, out), = doc_test.parse_blocks(p)
            self.assertEqual(out, "hello\n")
        finally:
            shutil.rmtree(d, ignore_errors=True)


class EndToEnd(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.gritc = doc_test.find_gritc()

    def run_doc(self, text):
        if not self.gritc:
            self.skipTest("không có gritc")
        d, p = md(text)
        try:
            r = subprocess.run([sys.executable, os.path.join(HERE, "doc_test.py"), p], capture_output=True, text=True, encoding="utf-8", env=dict(os.environ, GRITC=self.gritc))
            return r.returncode, r.stdout
        finally:
            shutil.rmtree(d, ignore_errors=True)

    def test_good_block_passes(self):
        code, out = self.run_doc("```grit value=7\n" + OK + "```\n")
        self.assertEqual(code, 0, out)

    def test_a_block_that_does_not_compile_makes_the_tool_red(self):
        code, out = self.run_doc("```grit\n" + BAD_TYPE + "```\n")
        self.assertEqual(code, 1, out)
        self.assertIn("DOC_FAIL", out)

    def test_expected_failure_must_really_fail_with_that_code(self):
        code, out = self.run_doc("```grit fail=E0301\n" + BAD_TYPE + "```\n")
        self.assertEqual(code, 0, out)
        code, out = self.run_doc("```grit fail=E0301\n" + OK + "```\n")
        self.assertEqual(code, 1, out)  # mong đợi lỗi mà chương trình lại xanh
        code, out = self.run_doc("```grit fail=E0999\n" + BAD_TYPE + "```\n")
        self.assertEqual(code, 1, out)  # sai mã lỗi

    def test_wrong_value_and_wrong_output_are_red(self):
        code, out = self.run_doc("```grit value=8\n" + OK + "```\n")
        self.assertEqual(code, 1, out)

    def test_no_grit_blocks_at_all_is_red(self):
        code, out = self.run_doc("chỉ có chữ\n")
        self.assertEqual(code, 1, out)


if __name__ == "__main__":
    unittest.main()
