"""GÓI 4 S5.b: chế độ song song của tools/fuzz_loop.sh (kế hoạch theo RAM trống) và tools/fuzz_total.py (giờ trên băm hiện tại, danh sách tệp băm)."""
import os
import subprocess
import sys
import unittest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def plan(*args, free=None):
    env = dict(os.environ, PYTHONIOENCODING="utf-8")
    if free is not None:
        env["FUZZ_FAKE_FREE_GB"] = str(free)
    p = subprocess.run(["bash", "tools/fuzz_loop.sh", "1", *args, "--plan"], cwd=ROOT, env=env, capture_output=True, text=True, encoding="utf-8", errors="replace")
    return p.returncode, p.stdout


class FuzzLoopPlan(unittest.TestCase):
    def test_default_is_sequential(self):
        rc, out = plan(free=64)
        self.assertEqual(rc, 0, out)
        self.assertIn("MODE=sequential", out)

    def test_parallel_when_enough_free_memory(self):
        rc, out = plan("--parallel", free=16)
        self.assertEqual(rc, 0, out)
        self.assertIn("MODE=parallel", out)

    def test_parallel_falls_back_when_memory_is_short(self):
        rc, out = plan("--parallel", free=3)
        self.assertEqual(rc, 0, out)
        self.assertIn("MODE=sequential", out)
        self.assertIn("lùi về lần lượt", out)

    def test_threshold_is_configurable_and_exact(self):
        env = dict(os.environ, FUZZ_FAKE_FREE_GB="6", FUZZ_PAR_MIN_GB="6")
        p = subprocess.run(["bash", "tools/fuzz_loop.sh", "1", "--parallel", "--plan"], cwd=ROOT, env=env, capture_output=True, text=True, encoding="utf-8")
        self.assertIn("MODE=parallel", p.stdout)


class FuzzTotal(unittest.TestCase):
    def run_total(self, *a):
        return subprocess.run([sys.executable, "tools/fuzz_total.py", *a], cwd=ROOT, capture_output=True, text=True, encoding="utf-8", env=dict(os.environ, PYTHONIOENCODING="utf-8"))

    def test_prints_hours_on_current_hash_and_number_of_hashed_files(self):
        p = self.run_total()
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertIn("GIỜ TRÊN BĂM HIỆN TẠI", p.stdout)
        self.assertIn("tệp được băm", p.stdout)

    def test_list_files_names_the_lexer_and_parser_sources(self):
        p = self.run_total("--list-files")
        self.assertIn("crates/gt-lexer/src/", p.stdout)
        self.assertIn("crates/gt-parser/src/", p.stdout)

    def test_kind_done_is_false_below_24_hours(self):
        self.assertEqual(self.run_total("--kind-done", "lex").returncode, 1)
        self.assertEqual(self.run_total("--kind-done", "parse").returncode, 1)


if __name__ == "__main__":
    unittest.main()
