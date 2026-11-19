"""GÓI 8 S0d: test tools/adversarial_check.py và tools/import_attack.py (kho tạm; gritc thật ở target/release)."""
import os
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, HERE)
import adversarial_check as A
import import_attack as I

GRITC = os.path.join(ROOT, "target", "release", "gritc.exe")
PASS_SRC = "// expect: pass\nƒ main() → i32 ! ∅ { 0 }\n"
FAIL_SRC = "// expect: fail E0702\nƒ f() → () ! ∅ { μi ≔ 0; ⟳ i < 10 ↓ i { i ← i +% 1; } }\nƒ main() → i32 ! ∅ { 0 }\n"
WRONG_CODE = "// expect: fail E0401\nƒ f() → () ! ∅ { μi ≔ 0; ⟳ i < 10 ↓ i { i ← i +% 1; } }\nƒ main() → i32 ! ∅ { 0 }\n"


class Judge(unittest.TestCase):
    def test_pass_green_ok_and_red_wrong(self):
        self.assertIsNone(A.judge("pass", None, "green", []))
        self.assertIsNotNone(A.judge("pass", None, "red", ["E0702"]))

    def test_fail_needs_red_and_matching_code(self):
        self.assertIsNone(A.judge("fail", "E0702", "red", ["E0702"]))
        self.assertIsNotNone(A.judge("fail", "E0702", "red", ["E0401"]))
        self.assertIsNotNone(A.judge("fail", "E0702", "green", []))

    def test_ice_always_wrong(self):
        self.assertIsNotNone(A.judge("fail", "E0702", "red", ["E9901", "E0702"]))
        self.assertIsNotNone(A.judge("pass", None, "green", ["E9901"]))


@unittest.skipUnless(os.path.isfile(GRITC), "cần target/release/gritc.exe")
class Gate(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.mkdtemp()
        os.makedirs(os.path.join(self.d, "pass"))
        os.makedirs(os.path.join(self.d, "fail"))

    def write(self, sub, name, src):
        with open(os.path.join(self.d, sub, name), "w", encoding="utf-8", newline="\n") as f:
            f.write(src)

    def run_tool(self):
        return subprocess.run([sys.executable, os.path.join(HERE, "adversarial_check.py"), "--gritc", GRITC, "--dir", self.d],
                              capture_output=True, text=True, encoding="utf-8")

    def test_all_correct_exit_zero(self):
        self.write("pass", "p.gt", PASS_SRC)
        self.write("fail", "f.gt", FAIL_SRC)
        r = self.run_tool()
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertIn("0 sai", r.stdout)

    def test_wrong_code_fails_gate(self):
        self.write("fail", "w.gt", WRONG_CODE)
        r = self.run_tool()
        self.assertEqual(r.returncode, 1, r.stdout)
        self.assertIn("mong đợi E0401", r.stdout)

    def test_missing_expect_fails_gate(self):
        self.write("pass", "n.gt", "ƒ main() → i32 ! ∅ { 0 }\n")
        self.assertEqual(self.run_tool().returncode, 1)


@unittest.skipUnless(os.path.isfile(GRITC), "cần target/release/gritc.exe")
class Import(unittest.TestCase):
    def test_dry_run_classifies_without_writing(self):
        src = tempfile.mkdtemp()
        with open(os.path.join(src, "a.gt"), "w", encoding="utf-8", newline="\n") as f:
            f.write(PASS_SRC)
        with open(os.path.join(src, "b.gt"), "w", encoding="utf-8", newline="\n") as f:
            f.write(WRONG_CODE)
        with open(os.path.join(src, "c.gt"), "w", encoding="utf-8", newline="\n") as f:
            f.write("ƒ main() → i32 ! ∅ { 0 }\n")
        r = subprocess.run([sys.executable, os.path.join(HERE, "import_attack.py"), src, "--gritc", GRITC, "--dry-run"],
                           capture_output=True, text=True, encoding="utf-8")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("NHẬP pass", r.stdout)
        self.assertIn("NGHI VẤN", r.stdout)   # ý định E0401, ra E0702
        self.assertIn("BỎ", r.stdout)         # không có // expect:


if __name__ == "__main__":
    unittest.main()
