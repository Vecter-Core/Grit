"""GÓI 5 S2.a: kiểm kê test một-nền/ignore khớp docs/PLATFORM_TESTS.md; ignore không lý do bị bắt."""
import os
import sys
import tempfile
import unittest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "tools"))
import platform_tests as pt  # noqa: E402


class PlatformTests(unittest.TestCase):
    def test_repo_matches_doc(self):
        found, bad = pt.scan()
        self.assertEqual(bad, [], "ignore không lý do")
        self.assertEqual(sorted(set(found) - pt.documented()), [])
        self.assertEqual(sorted(pt.documented() - set(found)), [])

    def test_ignore_without_reason_is_caught(self):
        d = tempfile.mkdtemp(prefix="plat-")
        os.makedirs(os.path.join(d, "crates", "x", "tests"))
        with open(os.path.join(d, "crates", "x", "tests", "a.rs"), "w", encoding="utf-8") as f:
            f.write("#[test]\n#[ignore]\nfn t1() {}\n#[test]\n#[ignore = \"có lý do\"]\nfn t2() {}\n#[cfg(windows)]\n#[test]\nfn t3() {}\n")
        found, bad = pt.scan(d)
        self.assertEqual(len(bad), 1)
        self.assertIn("t1", bad[0])
        self.assertEqual(sorted(k.split("::")[1] for k in found), ["t1", "t2", "t3"])


if __name__ == "__main__":
    unittest.main()
