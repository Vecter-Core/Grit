"""Test tools/nul_check.sh (R1b): kho git tạm; tệp theo dõi có đuôi văn bản chứa NUL ⇒ ĐỎ nêu tên; tệp nhị phân (.png) chứa NUL ⇒ không tính; sạch ⇒ OK."""
import os
import shutil
import subprocess
import shutil
BASH = shutil.which("bash") or "bash"  # Windows: CreateProcess ưu tiên System32\bash.exe (WSL); which() theo PATH (Git Bash đứng đầu)
import tempfile
import unittest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SCRIPT = os.path.join(ROOT, "tools", "nul_check.sh").replace("\\", "/")


class NulCheck(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.mkdtemp(prefix="nulchk-")
        self.addCleanup(shutil.rmtree, self.d, True)

    def git(self, *a):
        subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@t", *a], cwd=self.d, check=True, capture_output=True)

    def put(self, rel, data):
        p = os.path.join(self.d, rel)
        os.makedirs(os.path.dirname(p), exist_ok=True)
        with open(p, "wb") as f:
            f.write(data)

    def run_check(self):
        return subprocess.run([BASH, SCRIPT, self.d.replace("\\", "/")], capture_output=True, text=True, encoding="utf-8", errors="replace",
                              stdin=subprocess.DEVNULL)

    def test_clean_tree_is_ok_and_binary_files_are_ignored(self):
        self.put("a.rs", "fn main() {}\n".encode())
        self.put("docs/b.md", "Tiếng Việt\n".encode())
        self.put("img/x.png", b"\x89PNG\x00\x00\x00")  # nhị phân có NUL hợp lệ
        self.git("init", "-q")
        self.git("add", "-A")
        self.git("commit", "-q", "-m", "i")
        r = self.run_check()
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertIn("NUL_OK", r.stdout)

    def test_each_text_extension_with_nul_is_reported_by_name(self):
        names = ["a.rs", "b.gt", "c.md", "d.toml", "e.py", "f.sh", "g.json", "h.jsonl", "i.txt", "j.yml", "k.lock"]
        for n in names:
            self.put("src/" + n, b"abc\x00\x00def")
        self.put("ok.rs", b"fn main() {}\n")
        self.git("init", "-q")
        self.git("add", "-A")
        self.git("commit", "-q", "-m", "i")
        r = self.run_check()
        self.assertEqual(r.returncode, 1, r.stdout + r.stderr)
        for n in names:
            self.assertIn("src/" + n, r.stdout)
        self.assertNotIn("ok.rs", r.stdout)

    def test_fuzz_regression_inputs_may_contain_nul_on_purpose(self):
        self.put("crates/gt-parser/tests/fuzz_regressions/crash_x.gt", b"a" + bytes([0]) + b"b")
        self.git("init", "-q")
        self.git("add", "-A")
        self.git("commit", "-q", "-m", "i")
        r = self.run_check()
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)

    def test_file_filled_with_nul_like_the_real_incident_is_caught(self):
        self.put("crates/x/src/lib.rs", b"\x00" * 4096)
        self.git("init", "-q")
        self.git("add", "-A")
        self.git("commit", "-q", "-m", "i")
        r = self.run_check()
        self.assertEqual(r.returncode, 1, r.stdout + r.stderr)
        self.assertIn("crates/x/src/lib.rs", r.stdout)


if __name__ == "__main__":
    unittest.main()
