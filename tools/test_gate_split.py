"""GÓI 12 P4: gói trong GATE_SPLIT_PKGS chạy MỖI mục tiêu (`--lib`, `--test X`, `--doc`) như một đơn vị có điểm lưu riêng (`test:<gói>:<mục tiêu>`) và in
`secs=`; đổi cây ⇒ làm lại; một mục tiêu đỏ ⇒ lần sau chỉ chạy lại mục tiêu đó và phần chưa chạy. Dùng `cargo` giả trên PATH (không cần cargo thật)."""
import os
import shutil
import subprocess
import tempfile
import unittest

BASH = shutil.which("bash") or "bash"
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
META = (
    '{"packages":[{"name":"pkgA","targets":[{"kind":["lib"],"name":"pkga"},{"kind":["test"],"name":"t1"},{"kind":["test"],"name":"t2"}]},'
    '{"name":"pkgB","targets":[{"kind":["lib"],"name":"pkgb"}]}]}'
)
FAKE_CARGO = """#!/bin/sh
case "$1" in
  metadata) cat <<'EOF'
@@META@@
EOF
  ;;
  test) echo "$*" >> "$FAKE_LOG"
    case "$*" in *"--test $FAKE_FAIL "*) echo 'test result: FAILED. 1 passed; 1 failed; 0 ignored'; exit 101;; esac
    echo 'test result: ok. 3 passed; 0 failed; 0 ignored' ;;
esac
"""


def fwd(p):
    return p.replace(chr(92), "/")


def tracked_repo():
    d = tempfile.mkdtemp(prefix="gate-split-")

    def git(*a):
        subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@t", *a], cwd=d, check=True, capture_output=True)

    git("init", "-q")
    with open(os.path.join(d, "tracked.txt"), "w", encoding="utf-8", newline="\n") as f:
        f.write("gốc\n")
    with open(os.path.join(d, ".gitignore"), "w", encoding="utf-8", newline="\n") as f:
        f.write("target/\n")
    git("add", "-A")
    git("commit", "-q", "-m", "init")
    return d


class SplitTestUnits(unittest.TestCase):
    def setUp(self):
        self.repo = tracked_repo()
        self.bin = tempfile.mkdtemp(prefix="fakecargo-")
        self.log = os.path.join(self.bin, "calls.log")
        self.addCleanup(shutil.rmtree, self.repo, True)
        self.addCleanup(shutil.rmtree, self.bin, True)
        with open(os.path.join(self.bin, "cargo"), "w", encoding="utf-8", newline="\n") as f:
            f.write(FAKE_CARGO.replace("@@META@@", META))
        os.chmod(os.path.join(self.bin, "cargo"), 0o755)  # Linux: PATH chỉ chạy tệp có bit thực thi (nếu thiếu, gate gọi cargo THẬT)

    def run_gate(self, fail="-"):
        env = {k: v for k, v in os.environ.items() if k not in ("GATE_RESET", "GATE_BUDGET_SEC", "GATE_FORCE_PREFLIGHT", "GATE_TEST_CMD")}
        sep = ";" if os.name == "nt" else ":"
        env.update(GATE_STAGES="test", GATE_SPLIT_PKGS="pkgA", PYTHONIOENCODING="utf-8", FAKE_LOG=fwd(self.log), FAKE_FAIL=fail,
                   PATH=fwd(self.bin) + sep + env.get("PATH", ""))
        script = fwd(os.path.join(ROOT, "tools", "gate.sh"))
        p = subprocess.run([BASH, script, fwd(self.repo)], cwd=self.repo, env=env, capture_output=True, text=True, encoding="utf-8", errors="replace")
        return p.returncode, p.stdout

    def calls(self):
        if not os.path.exists(self.log):
            return []
        with open(self.log, encoding="utf-8") as f:
            return [x.strip() for x in f if x.strip()]

    def test_each_target_is_its_own_unit_with_timing(self):
        rc, out = self.run_gate()
        self.assertEqual(rc, 0, out)
        c = self.calls()
        self.assertEqual(len(c), 5, c)  # pkgA: lib, t1, t2, doc; pkgB: một đơn vị
        self.assertTrue(any("--test t1" in x for x in c), c)
        self.assertTrue(any("--doc" in x for x in c), c)
        self.assertEqual(sum("-p pkgB" in x for x in c), 1, c)
        for u in ("pkgA:lib", "pkgA:t1", "pkgA:t2", "pkgA:doc", "pkgB"):
            self.assertIn("TEST_UNIT " + u + " ", out)
        self.assertIn("secs=", out)
        self.assertIn("TEST_PASSED=15 FAILED=0 SUITES=5", out)

    def test_unchanged_tree_reuses_every_target_and_changed_tree_reruns_all(self):
        self.run_gate()
        rc, out2 = self.run_gate()
        self.assertEqual(rc, 0, out2)
        self.assertEqual(len(self.calls()), 5)
        self.assertIn("test:pkgA:t1: ĐÃ ĐẠT", out2)
        with open(os.path.join(self.repo, "tracked.txt"), "w", encoding="utf-8", newline="\n") as f:
            f.write("đã sửa\n")
        rc, out3 = self.run_gate()
        self.assertEqual(rc, 0, out3)
        self.assertIn("làm lại từ đầu", out3)
        self.assertEqual(len(self.calls()), 10)

    def test_a_red_target_is_the_only_one_rerun_afterwards(self):
        rc, out1 = self.run_gate(fail="t2")
        self.assertNotEqual(rc, 0, out1)
        self.assertIn("GATE_RESULT=RED", out1)
        before = len(self.calls())
        rc, out2 = self.run_gate()
        self.assertEqual(rc, 0, out2)
        self.assertIn("test:pkgA:lib: ĐÃ ĐẠT", out2)
        self.assertIn("test:pkgA:t1: ĐÃ ĐẠT", out2)
        later = self.calls()[before:]
        self.assertFalse(any("--lib" in x or "--test t1" in x for x in later), later)
        self.assertTrue(any("--test t2" in x for x in later), later)


if __name__ == "__main__":
    unittest.main()
