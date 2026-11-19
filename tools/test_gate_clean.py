"""Kiểm thử tools/gate_clean.sh trên kho git tạm (GÓI 10, S1.a): clone sạch, không đụng kho gốc, từ chối đích sai."""
import os
import shutil
import subprocess
import tempfile
import unittest

BASH = shutil.which("bash") or "bash"
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SCRIPT = os.path.join(ROOT, "tools", "gate_clean.sh").replace("\\", "/")


def run(cmd, cwd):
    return subprocess.run(cmd, cwd=cwd, capture_output=True, text=True, encoding="utf-8", errors="replace",
                          env={**os.environ, "GATE_CLEAN_SKIP_PREFLIGHT": "1"})


class GateClean(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix="gc_")
        self.repo = os.path.join(self.tmp, "repo")
        os.makedirs(os.path.join(self.repo, "tools"))
        g = lambda *a: subprocess.run(["git", *a], cwd=self.repo, capture_output=True, text=True, check=True)
        g("init", "-q")
        g("config", "user.email", "t@t"); g("config", "user.name", "t")
        # cổng giả: in một dòng GATE_RESULT và thoát theo nội dung tệp mark
        with open(os.path.join(self.repo, "tools", "gate.sh"), "w", newline="\n") as f:
            f.write('#!/bin/bash\ncd "$1" || exit 1\nls -A | grep -q stray && echo STRAY_SEEN\necho GATE_RESULT=GREEN\n')
        with open(os.path.join(self.repo, "a.txt"), "w") as f:
            f.write("x")
        g("add", "-A"); g("commit", "-q", "-m", "c1")
        self.head = g("rev-parse", "HEAD").stdout.strip()
        self.g = g

    def test_clone_runs_gate_and_ignores_stray_untracked_file_in_origin(self):
        with open(os.path.join(self.repo, "stray.txt"), "w") as f:
            f.write("stray")  # tệp chưa theo dõi trong kho gốc
        dest = os.path.join(self.tmp, "clone").replace("\\", "/")
        p = run([BASH, SCRIPT, self.head, dest, self.repo.replace("\\", "/")], self.tmp)
        self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
        self.assertIn("GATE_RESULT=GREEN", p.stdout)
        self.assertNotIn("STRAY_SEEN", p.stdout)  # bản clone không có tệp lạ
        self.assertTrue(os.path.exists(os.path.join(self.repo, "stray.txt")))  # kho gốc nguyên vẹn
        self.assertIn(self.head, p.stdout)

    def test_refuses_nonempty_destination_and_unknown_commit(self):
        dest = os.path.join(self.tmp, "full"); os.makedirs(dest)
        open(os.path.join(dest, "keep"), "w").close()
        p = run([BASH, SCRIPT, self.head, dest.replace("\\", "/"), self.repo.replace("\\", "/")], self.tmp)
        self.assertNotEqual(p.returncode, 0)
        self.assertIn("GATE_CLEAN_FAIL", p.stdout)
        self.assertTrue(os.path.exists(os.path.join(dest, "keep")))
        p = run([BASH, SCRIPT, "deadbeef", os.path.join(self.tmp, "c2").replace("\\", "/"), self.repo.replace("\\", "/")], self.tmp)
        self.assertNotEqual(p.returncode, 0)
        self.assertIn("GATE_CLEAN_FAIL", p.stdout)

    def test_refuses_destination_inside_repo(self):
        p = run([BASH, SCRIPT, self.head, (self.repo + "/inside").replace("\\", "/"), self.repo.replace("\\", "/")], self.tmp)
        self.assertNotEqual(p.returncode, 0)
        self.assertIn("TRONG kho", p.stdout)


if __name__ == "__main__":
    unittest.main()
