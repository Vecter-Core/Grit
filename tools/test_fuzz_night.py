"""GÓI 14 E2: tools/fuzz_night.sh — chế độ FUZZ_NIGHT_DRY=1 (không cần WSL) kiểm lệnh sẽ chạy trong WSL."""
import os
import shutil
import subprocess
import unittest

BASH = shutil.which("bash") or "bash"
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SCRIPT = os.path.join(ROOT, "tools", "fuzz_night.sh").replace(chr(92), "/")


def run(*args, **env):
    e = dict(os.environ, FUZZ_NIGHT_DRY="1", PYTHONIOENCODING="utf-8")
    e.update(env)
    p = subprocess.run([BASH, SCRIPT, *args], capture_output=True, text=True, encoding="utf-8", env=e, cwd=ROOT)
    return p.returncode, p.stdout + p.stderr


class Night(unittest.TestCase):
    def test_start_defaults_to_parallel_26_hours_pinned_nightly_and_syncs_the_clone(self):
        rc, out = run("start")
        self.assertEqual(rc, 0, out)
        self.assertIn("fuzz_loop.sh 26 --parallel", out)
        self.assertIn("FUZZ_TOOLCHAIN=nightly-2026-10-04", out)
        self.assertIn("git fetch -q /mnt/d/Grit integration", out)
        self.assertIn("core.autocrlf false", out)
        self.assertIn("tee -a", out)
        self.assertIn("WSL[Ubuntu-24.04]", out)
        self.assertIn("(pwd)", out)  # phải thoát ký tự để chạy TRONG WSL, không phải ở Git Bash

    def test_resync_keeps_the_clone_local_log_rows_instead_of_aborting_on_a_dirty_log(self):
        """Lỗi tìm thấy khi kiểm tay (GÓI 14): `git checkout FETCH_HEAD` từ chối vì docs/FUZZ_LOG.jsonl của clone khác HEAD ⇒ lệnh qua đêm không chạy được lần thứ hai."""
        rc, out = run("start")
        self.assertEqual(rc, 0, out)
        self.assertIn("git checkout -q -f FETCH_HEAD", out)
        self.assertIn(".fuzz_log.keep", out)
        self.assertIn(">> docs/FUZZ_LOG.jsonl", out)

    def test_hours_and_sequential_flag(self):
        rc, out = run("start", "0.05", "--sequential")
        self.assertEqual(rc, 0, out)
        self.assertIn("fuzz_loop.sh 0.05 ", out)
        self.assertNotIn("--parallel", out)

    def test_status_and_import(self):
        rc, out = run("status")
        self.assertEqual(rc, 0, out)
        self.assertIn("fuzz_total.py --summary", out)
        rc, out = run("import")
        self.assertEqual(rc, 0, out)
        self.assertIn("fuzz_clone_log.jsonl", out)

    def test_unknown_command_is_usage_error(self):
        rc, out = run("bogus")
        self.assertEqual(rc, 2, out)


if __name__ == "__main__":
    unittest.main()
