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


def sh(*a, cwd=None):
    return subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@t", *a], cwd=cwd, capture_output=True, text=True, encoding="utf-8", check=True).stdout


class SyncClone(unittest.TestCase):
    """tools/fuzz_sync_clone.sh bằng kho nguồn giả: dòng log chỉ có ở clone được GIỮ, HEAD tiến lên, chạy lại không nhân đôi."""

    def test_keeps_local_rows_advances_head_and_is_idempotent(self):
        import tempfile
        tmp = tempfile.mkdtemp(prefix="fuzz-sync-")
        self.addCleanup(shutil.rmtree, tmp, True)
        src, clone = os.path.join(tmp, "src").replace(chr(92), "/"), os.path.join(tmp, "clone").replace(chr(92), "/")
        os.makedirs(os.path.join(src, "docs"))
        sh("init", "-q", "-b", "integration", cwd=src)
        log = os.path.join(src, "docs", "FUZZ_LOG.jsonl")
        with open(log, "w", encoding="utf-8", newline=chr(10)) as f:
            f.write('{"ts":"A"}'+chr(10)+'{"ts":"B"}'+chr(10))
        sh("add", "-A", cwd=src)
        sh("commit", "-q", "-m", "c1", cwd=src)
        script = os.path.join(ROOT, "tools", "fuzz_sync_clone.sh").replace(chr(92), "/")

        def sync():
            p = subprocess.run([BASH, script, clone, src, "integration"], capture_output=True, text=True, encoding="utf-8")
            self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
            return p.stdout

        sync()
        with open(os.path.join(clone, "docs", "FUZZ_LOG.jsonl"), "a", encoding="utf-8", newline=chr(10)) as f:
            f.write('{"ts":"LOCAL1"}'+chr(10)+'{"ts":"LOCAL2"}'+chr(10))
        with open(log, "a", encoding="utf-8", newline=chr(10)) as f:  # kho chính tiến lên (nhập thêm một dòng)
            f.write('{"ts":"C"}'+chr(10))
        sh("commit", "-q", "-am", "c2", cwd=src)
        out = sync()
        with open(os.path.join(clone, "docs", "FUZZ_LOG.jsonl"), encoding="utf-8") as f:
            rows = f.read().split()
        self.assertEqual(rows, ['{"ts":"A"}', '{"ts":"B"}', '{"ts":"C"}', '{"ts":"LOCAL1"}', '{"ts":"LOCAL2"}'], out)
        sync()  # lần hai: không nhân đôi
        with open(os.path.join(clone, "docs", "FUZZ_LOG.jsonl"), encoding="utf-8") as f:
            self.assertEqual(len(f.read().split()), 5)


class Night(unittest.TestCase):
    def test_start_defaults_to_parallel_26_hours_pinned_nightly_and_syncs_the_clone(self):
        rc, out = run("start")
        self.assertEqual(rc, 0, out)
        self.assertIn("fuzz_loop.sh 26 --parallel", out)
        self.assertIn("FUZZ_TOOLCHAIN=nightly-2026-10-04", out)
        self.assertIn("fuzz_sync_clone.sh $HOME/grit_fuzz /mnt/d/Grit integration", out)
        self.assertIn("tee -a", out)
        self.assertIn("WSL[Ubuntu-24.04]", out)

    def test_start_syncs_through_the_script_file_not_a_long_inline_string(self):
        rc, out = run("start", FUZZ_REPO_WSL="/mnt/q/Kho")
        self.assertEqual(rc, 0, out)
        self.assertIn("bash /mnt/q/Kho/tools/fuzz_sync_clone.sh", out)
        self.assertIn("/mnt/q/Kho integration", out)

    def test_default_repo_path_is_converted_from_windows_to_wsl(self):
        rc, out = run("start")
        self.assertEqual(rc, 0, out)
        self.assertIn("bash /mnt/", out)  # D:/Grit ⇒ /mnt/d/Grit

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
