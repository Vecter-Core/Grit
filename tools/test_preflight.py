"""Test tools/preflight.sh (R1a): kho git tạm; tệp .git/index chứa NUL, ref rỗng, HEAD NUL ⇒ ĐỎ (thoát 2); kho sạch ⇒ OK."""
import os
import shutil
import subprocess
import shutil
BASH = shutil.which("bash") or "bash"  # Windows: CreateProcess ưu tiên System32\bash.exe (WSL); which() theo PATH (Git Bash đứng đầu)
import tempfile
import unittest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SCRIPT = os.path.join(ROOT, "tools", "preflight.sh").replace("\\", "/")


class Preflight(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.mkdtemp(prefix="preflight-")
        self.addCleanup(shutil.rmtree, self.d, True)

        def git(*a):
            subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@t", *a], cwd=self.d, check=True, capture_output=True)

        git("init", "-q")
        with open(os.path.join(self.d, "a.txt"), "w", encoding="utf-8", newline="\n") as f:
            f.write("gốc\n")
        git("add", "-A")
        git("commit", "-q", "-m", "init")

    def run_pf(self):
        env = dict(os.environ, PREFLIGHT_SKIP_RESOURCES="1", PREFLIGHT_TRIES="2", PREFLIGHT_SLEEP="0")
        return subprocess.run([BASH, SCRIPT, self.d.replace("\\", "/")], capture_output=True, text=True, encoding="utf-8", errors="replace",
                              env=env, stdin=subprocess.DEVNULL)

    def corrupt(self, rel, data):
        with open(os.path.join(self.d, ".git", rel), "wb") as f:
            f.write(data)

    def test_clean_repo_is_ready(self):
        r = self.run_pf()
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertIn("PREFLIGHT_OK", r.stdout)

    def test_nul_filled_index_fails(self):
        size = os.path.getsize(os.path.join(self.d, ".git", "index"))
        self.corrupt("index", b"\x00" * size)
        r = self.run_pf()
        self.assertEqual(r.returncode, 2, r.stdout + r.stderr)
        self.assertIn("index", r.stdout)

    def test_empty_or_missing_branch_ref_fails(self):
        refs = os.path.join(self.d, ".git", "refs", "heads")
        for n in os.listdir(refs):
            open(os.path.join(refs, n), "wb").close()  # ref rỗng
        r = self.run_pf()
        self.assertEqual(r.returncode, 2, r.stdout + r.stderr)
        self.assertIn("rỗng", r.stdout)

    def test_nul_in_ref_and_head_fail(self):
        refs = os.path.join(self.d, ".git", "refs", "heads")
        for n in os.listdir(refs):
            with open(os.path.join(refs, n), "wb") as f:
                f.write(b"\x00" * 41)
        r = self.run_pf()
        self.assertEqual(r.returncode, 2, r.stdout + r.stderr)
        self.corrupt("HEAD", b"\x00" * 21)
        r = self.run_pf()
        self.assertEqual(r.returncode, 2, r.stdout + r.stderr)
        self.assertIn("HEAD", r.stdout)

    def test_corrupt_object_database_fails_fsck(self):
        # làm hỏng một object rời: fsck --full phải đỏ
        objs = os.path.join(self.d, ".git", "objects")
        victim = None
        for sub in os.listdir(objs):
            p = os.path.join(objs, sub)
            if len(sub) == 2 and os.path.isdir(p):
                victim = os.path.join(p, os.listdir(p)[0])
                break
        self.assertIsNotNone(victim)
        os.chmod(victim, 0o666)
        with open(victim, "wb") as f:
            f.write(b"khong phai zlib")
        r = self.run_pf()
        self.assertEqual(r.returncode, 2, r.stdout + r.stderr)

    def test_low_disk_threshold_is_enforced(self):
        # Windows kiểm ổ D:/C: (PREFLIGHT_MIN_D_GB); Linux kiểm ổ chứa kho (PREFLIGHT_MIN_DISK_KB) — GÓI 4: test chạy được trên cả hai nền
        env = dict(os.environ, PREFLIGHT_TRIES="1", PREFLIGHT_SLEEP="0", PREFLIGHT_MIN_D_GB="99999", PREFLIGHT_MIN_DISK_KB="99999999999999")
        r = subprocess.run([BASH, SCRIPT, self.d.replace("\\", "/")], capture_output=True, text=True, encoding="utf-8", errors="replace",
                           env=env, stdin=subprocess.DEVNULL)
        self.assertEqual(r.returncode, 2, r.stdout + r.stderr)
        self.assertTrue("D:" in r.stdout or "ổ chứa kho" in r.stdout, r.stdout)


class PreflightToolchain(Preflight):
    """GÓI 11 S0.a: tools/TOOLCHAIN ghim rustc; lệch ⇒ ĐỎ kèm cách sửa; khớp ⇒ OK (giả lập qua PREFLIGHT_RUSTC_VERSION)."""

    def run_pf_with(self, have):
        os.makedirs(os.path.join(self.d, "tools"), exist_ok=True)
        with open(os.path.join(self.d, "tools", "TOOLCHAIN"), "w", newline="\n") as f:
            f.write("rustc=1.98.1\n")
        env = dict(os.environ, PREFLIGHT_SKIP_RESOURCES="1", PREFLIGHT_TRIES="1", PREFLIGHT_SLEEP="0", PREFLIGHT_RUSTC_VERSION=have)
        return subprocess.run([BASH, SCRIPT, self.d.replace("\\", "/")], capture_output=True, text=True, encoding="utf-8", errors="replace",
                              env=env, stdin=subprocess.DEVNULL)

    def test_matching_rustc_is_ok(self):
        r = self.run_pf_with("1.98.1")
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)

    def test_newer_rustc_is_red_with_fix_hint(self):
        r = self.run_pf_with("1.100.0")
        self.assertEqual(r.returncode, 2, r.stdout + r.stderr)
        self.assertIn("1.100.0", r.stdout)
        self.assertIn("rustup toolchain install 1.98.1", r.stdout)


if __name__ == "__main__":
    unittest.main()
