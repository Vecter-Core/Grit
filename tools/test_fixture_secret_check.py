"""GÓI 8 S0f: test tools/fixture_secret_check.py bằng chuỗi TỔNG HỢP (không dùng bí mật thật)."""
import os
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import fixture_secret_check as F

PAYLOAD = "abcdefghijklmnopqrstuvwxyzABCD"  # 30 ký tự tổng hợp
GOOD = "gh" + "p_" + PAYLOAD + F.b62(__import__("zlib").crc32(PAYLOAD.encode()) & 0xFFFFFFFF)
BAD = "gh" + "p_" + PAYLOAD + "AAAAAA"


class Classify(unittest.TestCase):
    def test_github_checksum_valid_is_structural(self):
        c = list(F.candidates('x = "' + GOOD + '"'))
        self.assertEqual(len(c), 1)
        cls, note = F.classify(c[0][0], c[0][1], c[0][2])
        self.assertEqual(cls, "i")
        self.assertIn("theo cách tính đã nêu", note)

    def test_github_checksum_wrong_is_not_structural(self):
        c = list(F.candidates('"' + BAD + '"'))
        cls, _ = F.classify(c[0][0], c[0][1], c[0][2])
        self.assertIn(cls, ("iii", "iv"))
        self.assertNotEqual(cls, "i")

    def test_public_aws_example(self):
        cls, _ = F.classify("aws", "AKIAIOSFODNN7EXAMPLE")
        self.assertEqual(cls, "ii")

    def test_obvious_fake(self):
        cls, _ = F.classify("aws", "AKIA" + "X" * 16)
        self.assertEqual(cls, "iii")
        cls, _ = F.classify("google", "AIza" + "example_key_for_docs_123456789012345")
        self.assertEqual(cls, "iii")

    def test_unclassifiable(self):
        cls, _ = F.classify("slack", "xoxb-" + "q8Zk2mWn7Lp4Rt9Vb3Xc")
        self.assertEqual(cls, "iv")

    def test_no_value_printed_in_render(self):
        rows = [{"file": "t/a.rs", "line": 1, "kind": "github", "masked": GOOD[:4] + "****", "cls": "i", "note": "n"}]
        out = F.render(rows)
        self.assertNotIn(GOOD, out)
        self.assertIn(GOOD[:4] + "****", out)

    def test_scan_temp_tree(self):
        d = tempfile.mkdtemp()
        os.makedirs(os.path.join(d, "tests"))
        with open(os.path.join(d, "tests", "a.gt"), "w", encoding="utf-8", newline="\n") as f:
            f.write('x := "' + GOOD + '"\n')
        rows = F.scan(d)
        self.assertEqual([r["cls"] for r in rows], ["i"])
        self.assertEqual(rows[0]["line"], 1)

    def test_pending_block_lists_only_i_and_iv(self):
        rows = [{"file": "a", "line": 1, "kind": "g", "masked": "ghp_****", "cls": "i", "note": "x"},
                {"file": "b", "line": 2, "kind": "g", "masked": "ghp_****", "cls": "iii", "note": "y"}]
        out = F.pending_block(rows)
        self.assertIn("`a:1`", out)
        self.assertNotIn("`b:2`", out)


if __name__ == "__main__":
    unittest.main()
