"""GÓI 14 A2: tools/corpus_owner_check.py — cây giả (không cần cargo) tái hiện lỗi lặp lại của GÓI 7/13 (mẫu E0203 của giai đoạn kiểu rơi vào corpus của resolver)
và kiểm công cụ ĐỎ/XANH đúng; cuối cùng kiểm cây thật của kho."""
import os
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TOOL = os.path.join(ROOT, "tools", "corpus_owner_check.py")


def w(path, text):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write(text)


CORPUS = 'const OWNED: &[&str] = &[{codes}];\nfn x() {{ let _ = std::fs::read_dir("fail"); }}\n'


def build(tmp):
    w(os.path.join(tmp, "crates/gt-res/Cargo.toml"), '[package]\nname = "gt-res"\n[dependencies]\n')
    w(os.path.join(tmp, "crates/gt-res/src/lib.rs"), 'fn a() { let _ = ErrorCode::E0203; let _ = ErrorCode::E0201; }\n')
    w(os.path.join(tmp, "crates/gt-res/tests/corpus.rs"), CORPUS.format(codes='"E0201", "E0203"'))
    w(os.path.join(tmp, "crates/gt-typ/Cargo.toml"), '[package]\nname = "gt-typ"\n[dependencies]\ngt-res = { path = "../gt-res" }\n')
    w(os.path.join(tmp, "crates/gt-typ/src/lib.rs"), 'fn b() { let _ = ErrorCode::E0203; let _ = ErrorCode::E0301; }\n// E0999 chỉ trong chú thích\n')
    w(os.path.join(tmp, "crates/gt-typ/tests/corpus.rs"), CORPUS.format(codes='"E0301"'))
    w(os.path.join(tmp, "tests/fail/E0201_a.gt"), "// expect: fail E0201\n")
    w(os.path.join(tmp, "tests/fail/E0301_a.gt"), "// expect: fail E0301\n")
    w(os.path.join(tmp, "tests/fail/E0203_private_item.gt"), "// expect: fail E0203\n")
    w(os.path.join(tmp, "tests/corpus_stage.tsv"), "# c\ntests/fail/E0203_private_item.gt\tgt-res\n")


def run(tmp):
    env = dict(os.environ, PYTHONIOENCODING="utf-8")
    p = subprocess.run([sys.executable, "-I", TOOL, tmp], capture_output=True, text=True, encoding="utf-8", env=env)
    return p.returncode, p.stdout + p.stderr


class CorpusOwner(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix="corpus-owner-")
        self.addCleanup(shutil.rmtree, self.tmp, True)
        build(self.tmp)

    def test_consistent_tree_is_green(self):
        rc, out = run(self.tmp)
        self.assertEqual(rc, 0, out)
        self.assertIn("CORPUS_OWNER OK", out)

    def test_new_sample_of_a_multi_stage_code_without_stage_entry_is_red_and_names_sample_and_crate(self):
        # đúng lỗi GÓI 7/13: mẫu E0203 của GIAI ĐOẠN KIỂU thêm vào corpus mà không ai nói crate nào chạy nó
        w(os.path.join(self.tmp, "tests/fail/E0203_private_field_in_pattern.gt"), "// expect: fail E0203\n")
        rc, out = run(self.tmp)
        self.assertEqual(rc, 1, out)
        self.assertIn("CORPUS_OWNER_RED", out)
        self.assertIn("E0203_private_field_in_pattern.gt", out)
        self.assertIn("gt-res", out)
        self.assertIn("gt-typ", out)

    def test_sample_declared_as_a_downstream_stage_is_red(self):
        w(os.path.join(self.tmp, "tests/fail/E0203_private_field_in_pattern.gt"), "// expect: fail E0203\n")
        with open(os.path.join(self.tmp, "tests/corpus_stage.tsv"), "a", encoding="utf-8", newline="\n") as f:
            f.write("tests/fail/E0203_private_field_in_pattern.gt\tgt-typ\n")
        rc, out = run(self.tmp)
        self.assertEqual(rc, 1, out)
        self.assertIn("gt-typ", out)
        self.assertIn("không chạy mẫu này", out)

    def test_claimed_code_the_crate_cannot_emit_is_red(self):
        w(os.path.join(self.tmp, "crates/gt-res/tests/corpus.rs"), CORPUS.format(codes='"E0201", "E0203", "E0777"'))
        w(os.path.join(self.tmp, "tests/fail/E0777_x.gt"), "// expect: fail E0777\n")
        rc, out = run(self.tmp)
        self.assertEqual(rc, 1, out)
        self.assertIn("RULE-1", out)
        self.assertIn("E0777", out)

    def test_stage_entry_for_a_missing_file_is_red(self):
        with open(os.path.join(self.tmp, "tests/corpus_stage.tsv"), "a", encoding="utf-8", newline="\n") as f:
            f.write("tests/fail/E0203_gone.gt\tgt-res\n")
        rc, out = run(self.tmp)
        self.assertEqual(rc, 1, out)
        self.assertIn("E0203_gone.gt", out)

    def test_comment_mentions_do_not_count_as_emission(self):
        w(os.path.join(self.tmp, "crates/gt-res/tests/corpus.rs"), CORPUS.format(codes='"E0201", "E0203", "E0999"'))
        w(os.path.join(self.tmp, "tests/fail/E0999_x.gt"), "// expect: fail E0999\n")
        rc, out = run(self.tmp)
        self.assertEqual(rc, 1, out)  # E0999 chỉ nằm trong chú thích của gt-typ, gt-res không phát

    def test_real_repository_is_green(self):
        rc, out = run(ROOT)
        self.assertEqual(rc, 0, out)


if __name__ == "__main__":
    unittest.main()
