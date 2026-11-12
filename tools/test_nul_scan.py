"""Kiểm thử cơ chế quét NUL (R1) bằng kho git tạm: tệp văn bản chứa NUL ⇒ nhận ra; tệp nhị phân có đuôi đã biết ⇒ bỏ qua; kho sạch ⇒ 0."""
import os
import shutil
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import nul_scan  # noqa: E402


def repo(files):
    d = tempfile.mkdtemp(prefix="nulscan-")
    subprocess.run(["git", "init", "-q"], cwd=d, check=True)
    for name, data in files.items():
        os.makedirs(os.path.dirname(os.path.join(d, name)), exist_ok=True)
        with open(os.path.join(d, name), "wb") as f:
            f.write(data)
    subprocess.run(["git", "add", "-A"], cwd=d, check=True)
    return d


class NulScan(unittest.TestCase):
    def tearDown(self):
        for d in getattr(self, "dirs", []):
            shutil.rmtree(d, ignore_errors=True)

    def mk(self, files):
        d = repo(files)
        self.dirs = getattr(self, "dirs", []) + [d]
        return d

    def test_clean_repo_is_clean(self):
        d = self.mk({"a.rs": b"fn main() {}\n", "b.md": "tiếng Việt\n".encode("utf-8")})
        self.assertEqual(nul_scan.scan(d), [])
        self.assertEqual(nul_scan.main(["x", d]), 0)

    def test_text_file_with_nul_is_found(self):
        d = self.mk({"a.rs": b"fn main() {}\n", "bad.rs": b"fn\0\0\0main\n", "allnul.md": b"\0" * 64})
        bad = {f: n for f, n, _ in nul_scan.scan(d)}
        self.assertEqual(bad, {"bad.rs": 3, "allnul.md": 64})
        self.assertEqual(nul_scan.main(["x", d]), 1)

    def test_known_binary_extensions_are_skipped(self):
        d = self.mk({"x.exe": b"MZ\0\0\0", "ok.txt": b"ok\n"})
        self.assertEqual(nul_scan.scan(d), [])

    def test_fuzz_regression_inputs_are_exempt(self):
        d = self.mk({"crates/p/tests/fuzz_regressions/crash_1.gt": b"a\0b", "src/bad.rs": b"x\0"})
        self.assertEqual([f for f, _, _ in nul_scan.scan(d)], ["src/bad.rs"])

    def test_not_a_repo_is_an_error_not_clean(self):
        d = tempfile.mkdtemp(prefix="nulscan-norepo-")
        self.dirs = [d]
        self.assertEqual(nul_scan.main(["x", d]), 2)


if __name__ == "__main__":
    unittest.main()
