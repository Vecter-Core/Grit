"""S2.c (GÓI 3): công cụ chạy nặng TỰ TỪ CHỐI khi preflight FAIL (không dựa vào kỷ luật của agent).
Kho git tạm có .git/index hỏng ⇒ preflight thật báo FAIL ⇒ run_guarded.sh / mut_batch.sh / mut.py run phải thoát khác 0 mà KHÔNG chạy lệnh.
GRIT_PREFLIGHT_DIR chỉ để kiểm thử (trỏ preflight tới kho tạm); PREFLIGHT_SKIP_RESOURCES=1 bỏ kiểm tài nguyên máy (kho hỏng vẫn FAIL)."""
import os
import shutil
import subprocess
import tempfile
import unittest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
P = lambda s: s.replace("\\", "/")


class RefuseOnPreflightFail(unittest.TestCase):
    def mkrepo(self, corrupt):
        d = tempfile.mkdtemp(prefix="reqpf-")
        self.addCleanup(shutil.rmtree, d, True)
        g = lambda *a: subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@t", *a], cwd=d, check=True, capture_output=True)
        g("init", "-q")
        open(os.path.join(d, "a.txt"), "w").write("x\n")
        g("add", "-A"); g("commit", "-q", "-m", "i")
        if corrupt:
            n = os.path.getsize(os.path.join(d, ".git", "index"))
            open(os.path.join(d, ".git", "index"), "wb").write(b"\0" * n)
        return d

    def env(self, d):
        return dict(os.environ, GRIT_PREFLIGHT_DIR=P(d), PREFLIGHT_SKIP_RESOURCES="1", PREFLIGHT_TRIES="1", PREFLIGHT_SLEEP="0", PYTHONIOENCODING="utf-8")

    def run_cmd(self, argv, d):
        return subprocess.run(argv, cwd=ROOT, env=self.env(d), capture_output=True, text=True, encoding="utf-8", errors="replace", stdin=subprocess.DEVNULL)

    def test_run_guarded_refuses_and_does_not_run_the_command(self):
        d = self.mkrepo(True)
        marker = os.path.join(d, "ran.txt")
        r = self.run_cmd(["bash", "tools/run_guarded.sh", "bash", "-c", "echo x > " + P(marker)], d)
        self.assertNotEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertIn("PREFLIGHT", r.stdout + r.stderr)
        self.assertFalse(os.path.exists(marker))

    def test_run_guarded_runs_when_preflight_ok(self):
        d = self.mkrepo(False)
        r = self.run_cmd(["bash", "tools/run_guarded.sh", "true"], d)
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)

    def test_mut_batch_refuses(self):
        d = self.mkrepo(True)
        r = self.run_cmd(["bash", "tools/mut_batch.sh", "gt-term", "crates/gt-term/src/lib.rs", "1", "2", P(os.path.join(d, "o.jsonl")), "5"], d)
        self.assertNotEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertIn("PREFLIGHT", r.stdout + r.stderr)
        self.assertFalse(os.path.exists(os.path.join(d, "o.jsonl")))

    def test_mut_py_run_refuses_but_list_does_not(self):
        d = self.mkrepo(True)
        out = os.path.join(d, "o.jsonl")
        r = self.run_cmd(["python", "tools/mut.py", "run", "--out", out, "crates/gt-term/src/lib.rs"], d)
        self.assertNotEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertIn("PREFLIGHT", r.stdout + r.stderr)
        self.assertFalse(os.path.exists(out))
        r = self.run_cmd(["python", "tools/mut.py", "list", "crates/gt-term/src/lib.rs"], d)
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)

    def test_full_gate_refuses(self):
        d = self.mkrepo(True)
        r = self.run_cmd(["bash", P(os.path.join(ROOT, "tools", "gate.sh")), P(d)], d)
        self.assertNotEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertIn("PREFLIGHT", r.stdout + r.stderr)
        self.assertNotIn("GATE_RESULT=GREEN", r.stdout)


if __name__ == "__main__":
    unittest.main()
