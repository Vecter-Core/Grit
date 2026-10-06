#!/usr/bin/env python3
"""Test cho tools/check_expectations.py (đột biến corpus: script phải THẤT BẠI).

Mỗi ca sao chép script + tests/ + docs/ vào thư mục tạm, áp MỘT đột biến, rồi chạy script: baseline phải sạch (exit 0),
đột biến phải làm exit 1 và nêu đúng lý do. Chạy:  python -m unittest tools/test_check_expectations.py
"""
import os
import re
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


class Sandbox:
    def __enter__(self):
        self.dir = tempfile.mkdtemp(prefix="chk_exp_", dir=os.environ.get("GRIT_TMP"))
        os.makedirs(os.path.join(self.dir, "tools"))
        shutil.copy(os.path.join(ROOT, "tools", "check_expectations.py"), os.path.join(self.dir, "tools"))
        shutil.copytree(os.path.join(ROOT, "tests"), os.path.join(self.dir, "tests"))
        os.makedirs(os.path.join(self.dir, "docs"))
        shutil.copytree(os.path.join(ROOT, "docs", "spec"), os.path.join(self.dir, "docs", "spec"))
        shutil.copytree(os.path.join(ROOT, "docs", "errors"), os.path.join(self.dir, "docs", "errors"))
        return self

    def __exit__(self, *a):
        shutil.rmtree(self.dir, ignore_errors=True)

    def p(self, *parts):
        return os.path.join(self.dir, *parts)

    def read(self, rel):
        with open(self.p(rel), encoding="utf-8", newline="") as f:
            return f.read()

    def write(self, rel, text):
        with open(self.p(rel), "w", encoding="utf-8", newline="") as f:
            f.write(text)

    def run(self):
        env = dict(os.environ, PYTHONIOENCODING="utf-8")
        r = subprocess.run([sys.executable, self.p("tools", "check_expectations.py")], capture_output=True, text=True,
                           encoding="utf-8", env=env)
        return r.returncode, r.stdout


def listed(code):
    """(fail_refs, pass_refs) của docs/errors/<code>.md."""
    with open(os.path.join(ROOT, "docs", "errors", code + ".md"), encoding="utf-8") as f:
        lines = f.read().split("\n")
    f_ = [m.group(1) for ln in lines if (m := re.match(r"^- fail: (\S+)$", ln))]
    p_ = [m.group(1) for ln in lines if (m := re.match(r"^- pass: (\S+)$", ln))]
    return f_, p_


class CheckExpectations(unittest.TestCase):
    def expect_failure(self, sb, needle):
        code, out = sb.run()
        self.assertEqual(code, 1, "đột biến phải làm script thất bại:\n" + out)
        self.assertIn(needle, out)

    def test_baseline_is_clean(self):
        with Sandbox() as sb:
            code, out = sb.run()
            self.assertEqual(code, 0, out)

    def test_orphan_fail_sample_is_caught(self):
        with Sandbox() as sb:
            sb.write("tests/fail/E0401_zz_orphan.gt", "// expect: fail E0401\nƒ f() ! ∅ { }\n")
            self.expect_failure(sb, "E0401_zz_orphan")

    def test_changed_header_code_of_listed_sample_is_caught(self):
        with Sandbox() as sb:
            ref = listed("E0402")[0][3]  # tests/fail/<ref>.gt
            rel = f"tests/{ref}.gt"
            sb.write(rel, sb.read(rel).replace("fail E0402", "fail E0403", 1))
            self.expect_failure(sb, ref.split("/")[1])

    def test_pass_sample_turned_into_fail_is_caught(self):
        with Sandbox() as sb:
            ref = listed("E0401")[1][0]
            rel = f"tests/{ref}.gt"
            sb.write(rel, sb.read(rel).replace("// expect: pass", "// expect: fail E0401", 1))
            self.expect_failure(sb, ref)

    def test_deleted_listed_sample_is_caught(self):
        with Sandbox() as sb:
            ref = listed("E0405")[0][2]
            os.remove(sb.p("tests", ref + ".gt"))
            self.expect_failure(sb, ref)

    def test_doc_line_pointing_to_nothing_is_caught(self):
        with Sandbox() as sb:
            doc = "docs/errors/E0406.md"
            sb.write(doc, sb.read(doc).rstrip("\n") + "\n- fail: fail/E0406_does_not_exist\n")
            self.expect_failure(sb, "E0406_does_not_exist")

    def test_fewer_than_three_fail_refs_is_caught(self):
        with Sandbox() as sb:
            doc = "docs/errors/E0407.md"
            keep = 0
            out = []
            for ln in sb.read(doc).split("\n"):
                if ln.startswith("- fail:"):
                    keep += 1
                    if keep > 2:
                        continue
                out.append(ln)
            sb.write(doc, "\n".join(out))
            # các mẫu bị bỏ khỏi docs thành mồ côi, và mã chỉ còn <3 mẫu fail
            code, text = sb.run()
            self.assertEqual(code, 1, text)
            self.assertIn("cần >=3 dòng '- fail:", text)

    def test_fewer_than_three_pass_refs_is_caught(self):
        with Sandbox() as sb:
            doc = "docs/errors/E0403.md"
            out = [ln for ln in sb.read(doc).split("\n") if not ln.startswith("- pass:")]
            sb.write(doc, "\n".join(out) + "\n- pass: pass/78_bc_nll_shared_then_write\n")
            self.expect_failure(sb, "cần >=3 dòng '- pass:")

    def test_listing_in_wrong_codes_doc_is_caught(self):
        with Sandbox() as sb:
            ref = listed("E0402")[0][0]
            doc = "docs/errors/E0401.md"
            sb.write(doc, sb.read(doc).rstrip("\n") + f"\n- fail: {ref}\n")
            self.expect_failure(sb, ref)


if __name__ == "__main__":
    unittest.main()
