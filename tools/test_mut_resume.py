"""Test --resume / --max-seconds của tools/mut.py (lệnh Giang 2026-10-17 mục 0a): kho git tạm với một crate giả, bộ test giả (bất kỳ thay đổi nào ⇒ ĐỎ).
Phiên bị ngắt giữa lô thì chạy tiếp bằng --resume, không chạy lại đột biến đã có dòng, không ghi trùng."""
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
LIB = "pub fn f(a: u32, b: u32) -> bool {\n    if a == b {\n        return true;\n    }\n    a < b && b > 0\n}\n"
CHECK = (
    "import os, sys, time\n"
    "time.sleep(float(os.environ.get('MUT_TEST_SLEEP', '0')))\n"
    "cur = open('crates/foo/src/lib.rs', encoding='utf-8', newline='').read()\n"
    "orig = open('tests/orig.txt', encoding='utf-8', newline='').read()\n"
    "sys.exit(0 if cur == orig else 1)\n"
)


class MutResume(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.mkdtemp(prefix="mut-resume-")
        self.work = tempfile.mkdtemp(prefix="mut-resume-work-")
        self.addCleanup(shutil.rmtree, self.d, True)
        self.addCleanup(shutil.rmtree, self.work, True)

        def w(rel, text):
            p = os.path.join(self.d, rel)
            os.makedirs(os.path.dirname(p), exist_ok=True)
            with open(p, "w", encoding="utf-8", newline="\n") as f:
                f.write(text)

        w("crates/foo/src/lib.rs", LIB)
        w("tests/orig.txt", LIB)
        w("tests/check.py", CHECK)
        w("Cargo.toml", "[workspace]\n")
        w("Cargo.lock", "")
        w(".gitignore", "target/\n")
        os.makedirs(os.path.join(self.d, "tools"))
        for n in ("mut.py", "track_guard.sh", "require_preflight.sh", "preflight.sh"):  # S2.c: mut.py run gọi require_preflight.sh
            shutil.copy(os.path.join(ROOT, "tools", n), os.path.join(self.d, "tools", n))

        def git(*a):
            subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@t", *a], cwd=self.d, check=True, capture_output=True)

        git("init", "-q")
        git("add", "-A")
        git("commit", "-q", "-m", "init")
        self.out = os.path.join(self.d, "target", "res.jsonl")
        os.makedirs(os.path.dirname(self.out), exist_ok=True)

    def run_mut(self, *args, sleep="0"):
        env = dict(os.environ, GRIT_MUT_DIR=self.work.replace("\\", "/"), GRIT_MUT_TEST=f"{sys.executable} tests/check.py",
                   GRIT_MUT_TIMEOUT="60", PYTHONIOENCODING="utf-8", MUT_TEST_SLEEP=sleep,
                   PREFLIGHT_SKIP_RESOURCES="1", PREFLIGHT_TRIES="1", PREFLIGHT_SLEEP="0")
        return subprocess.run([sys.executable, os.path.join(self.d, "tools", "mut.py"), *args], cwd=self.d, env=env, capture_output=True,
                              text=True, encoding="utf-8", errors="replace", stdin=subprocess.DEVNULL)

    def rows(self):
        if not os.path.exists(self.out):
            return []
        with open(self.out, encoding="utf-8") as f:
            return [json.loads(ln) for ln in f if ln.strip()]

    def listed(self):
        r = self.run_mut("list", "crates/foo/src/lib.rs")
        self.assertEqual(r.returncode, 0, r.stderr)
        return int(r.stdout.strip().splitlines()[-1].split()[1])

    def test_resume_skips_finished_mutants_and_never_duplicates(self):
        n = self.listed()
        self.assertGreaterEqual(n, 4, "cần ≥ 4 đột biến để thử ngắt giữa lô")
        r = self.run_mut("run", "--limit", "1", "--out", self.out, "crates/foo/src/lib.rs")
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        first = self.rows()
        self.assertEqual(len(first), 1)
        self.assertEqual(first[0]["status"], "killed")
        self.assertEqual(first[0]["source"], "tự động")
        # không --resume trên tệp đã có kết quả ⇒ TỪ CHỐI (không trộn hai lô)
        r = self.run_mut("run", "--out", self.out, "crates/foo/src/lib.rs")
        self.assertNotEqual(r.returncode, 0)
        self.assertIn("--resume", r.stdout + r.stderr)
        self.assertEqual(len(self.rows()), 1)
        # --resume ⇒ chạy phần còn lại, đúng n dòng, id duy nhất, dòng đầu không đổi
        r = self.run_mut("run", "--resume", "--out", self.out, "crates/foo/src/lib.rs")
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        rows = self.rows()
        self.assertEqual(len(rows), n)
        self.assertEqual(len({x["id"] for x in rows}), n)
        self.assertEqual(rows[0], first[0])
        self.assertTrue(all(x["status"] == "killed" for x in rows))
        # chạy tiếp lần nữa: không còn gì để chạy, không thêm dòng
        r = self.run_mut("run", "--resume", "--out", self.out, "crates/foo/src/lib.rs")
        self.assertIn("0 đột biến cần chạy", r.stdout)
        self.assertEqual(len(self.rows()), n)

    def test_max_seconds_stops_cleanly_between_mutants_and_resume_finishes(self):
        n = self.listed()
        r = self.run_mut("run", "--max-seconds", "1", "--out", self.out, "crates/foo/src/lib.rs", sleep="1.3")
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        part = self.rows()
        self.assertTrue(1 <= len(part) < n, f"{len(part)} / {n}")
        self.assertIn("--resume", r.stdout)
        r = self.run_mut("run", "--resume", "--out", self.out, "crates/foo/src/lib.rs")
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        rows = self.rows()
        self.assertEqual(len(rows), n)
        self.assertEqual(len({x["id"] for x in rows}), n)

    def test_torn_last_line_from_an_interrupted_session_is_ignored(self):
        n = self.listed()
        self.run_mut("run", "--limit", "2", "--out", self.out, "crates/foo/src/lib.rs")
        with open(self.out, "a", encoding="utf-8", newline="") as f:
            f.write('{"id": "dở dang')  # bị cắt giữa chừng, không xuống dòng
        r = self.run_mut("run", "--resume", "--out", self.out, "crates/foo/src/lib.rs")
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        with open(self.out, encoding="utf-8") as f:
            lines = [ln for ln in f.read().splitlines() if ln.strip()]
        good = []
        for ln in lines:
            try:
                good.append(json.loads(ln)["id"])
            except (ValueError, KeyError):
                pass  # đúng một dòng dở ở giữa tệp
        self.assertEqual(len(good), n, "mỗi đột biến đúng một dòng hợp lệ, không dòng nào bị dính vào dòng dở")
        self.assertEqual(len(set(good)), n)


if __name__ == "__main__":
    unittest.main()
