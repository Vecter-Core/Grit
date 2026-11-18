"""GÓI 4 S2.b: số phiên bản tài liệu khớp mã; bản sao tạm với tài liệu LỆCH phải ĐỎ."""
import os
import shutil
import sys
import tempfile
import unittest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "tools"))
import version_docs as vd  # noqa: E402

FILES = ["CHANGELOG.md", "docs/spec/FREEZE.md", "docs/READINESS.md", "docs/PROJECT_STATUS.md", "README.md", "BUILDING.md", "crates/gt-driver/Cargo.toml"]


class VersionDocs(unittest.TestCase):
    def test_real_repo_docs_match_the_code(self):
        self.assertEqual(vd.check(ROOT), [])

    def copy(self):
        d = tempfile.mkdtemp(prefix="verdocs-")
        self.addCleanup(shutil.rmtree, d, True)
        for rel in FILES:
            src = os.path.join(ROOT, rel)
            if os.path.isfile(src):
                dst = os.path.join(d, rel)
                os.makedirs(os.path.dirname(dst), exist_ok=True)
                shutil.copy(src, dst)
        return d

    def sub(self, d, rel, a, b):
        p = os.path.join(d, rel)
        s = open(p, encoding="utf-8", newline="").read()
        self.assertIn(a, s, rel)
        open(p, "w", encoding="utf-8", newline="").write(s.replace(a, b, 1))

    def test_copy_alone_is_green(self):
        self.assertEqual(vd.check(self.copy(), run_binary=False), [])

    def test_stale_label_in_each_document_is_red(self):
        v = vd.cargo_version(ROOT)
        for rel in ("CHANGELOG.md", "docs/spec/FREEZE.md", "docs/READINESS.md", "docs/PROJECT_STATUS.md"):
            d = self.copy()
            p = os.path.join(d, rel)
            s = open(p, encoding="utf-8", newline="").read()
            self.assertIn(v, s, rel)
            anchor = {"CHANGELOG.md": "## ", "docs/spec/FREEZE.md": "`gritc --version` ⇒ `", "docs/READINESS.md": "Nhãn hiện hành: **`", "docs/PROJECT_STATUS.md": "Bản hiện tại là **`"}[rel]
            self.assertIn(anchor + v, s, rel)
            open(p, "w", encoding="utf-8", newline="").write(s.replace(anchor + v, anchor + "1.0.0-rc2", 1))
            e = vd.check(d, run_binary=False)
            self.assertTrue(any(rel in x for x in e), (rel, e))

    def test_code_bump_without_docs_is_red(self):
        d = self.copy()
        self.sub(d, "crates/gt-driver/Cargo.toml", 'version = "' + vd.cargo_version(ROOT) + '"', 'version = "1.0.0-rc9"')
        self.assertTrue(vd.check(d, run_binary=False))

    def test_readme_mentioning_an_old_version_is_red(self):
        d = self.copy()
        with open(os.path.join(d, "README.md"), "a", encoding="utf-8") as f:
            f.write("\nPhiên bản hiện tại: 1.0.0-rc2\n")
        e = vd.check(d, run_binary=False)
        self.assertTrue(any("README.md" in x for x in e), e)


if __name__ == "__main__":
    unittest.main()
