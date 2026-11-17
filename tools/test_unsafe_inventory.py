"""UNSAFE-01 (R11b, GÓI 3 S2.a): kiểm kê `unsafe` TỰ ĐỘNG. (1) trên kho thật: số `unsafe` theo tệp KHÔNG vượt docs/unsafe_baseline.json và crate không nằm trong danh sách được phép
phải cấm unsafe (forbid(unsafe_code) hoặc [lints] workspace = true); (2) công cụ BẮT một `unsafe` mới (thư mục tạm) và một crate được phép/không bị cấm."""
import json
import os
import shutil
import sys
import tempfile
import unittest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "tools"))
import unsafe_inventory as inv  # noqa: E402


def mk(root, rel, text):
    p = os.path.join(root, rel)
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, "w", encoding="utf-8", newline="\n") as f:
        f.write(text)


class RealRepo(unittest.TestCase):
    def test_real_repo_matches_baseline(self):
        problems = inv.check(ROOT)
        self.assertEqual(problems, [], "\n".join(problems))


class Mechanism(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.mkdtemp(prefix="unsafe-inv-")
        self.addCleanup(shutil.rmtree, self.d, True)
        mk(self.d, "crates/a/Cargo.toml", "[package]\nname='a'\n[lints]\nworkspace = true\n")
        mk(self.d, "crates/a/src/lib.rs", "#![forbid(unsafe_code)]\npub fn f() {}\n")
        mk(self.d, "docs/unsafe_baseline.json", json.dumps({"allowed_crates": [], "files": {}}))

    def test_clean_tree_passes(self):
        self.assertEqual(inv.check(self.d), [])

    def test_new_unsafe_block_in_a_forbidden_crate_is_reported(self):
        mk(self.d, "crates/a/src/lib.rs", "pub fn f() { unsafe { std::hint::unreachable_unchecked() } }\n")
        p = inv.check(self.d)
        self.assertTrue(any("crates/a/src/lib.rs" in x for x in p), p)

    def test_unsafe_fn_impl_extern_and_attribute_are_counted(self):
        mk(self.d, "crates/a/src/lib.rs", "pub unsafe fn g() {}\nunsafe impl Send for X {}\n#[unsafe(no_mangle)]\npub extern \"C\" fn h() {}\nunsafe extern \"C\" { fn k(); }\n")
        self.assertEqual(inv.count_file(os.path.join(self.d, "crates/a/src/lib.rs")), 4)

    def test_word_in_comment_or_string_is_not_counted(self):
        mk(self.d, "crates/a/src/lib.rs", "// unsafe { x }\n/// `unsafe fn` là gì\npub const S: &str = \"unsafe { }\";\n")
        self.assertEqual(inv.count_file(os.path.join(self.d, "crates/a/src/lib.rs")), 0)

    def test_allowed_crate_may_exceed_nothing_but_baseline(self):
        mk(self.d, "crates/b/Cargo.toml", "[package]\nname='b'\n")
        mk(self.d, "crates/b/src/lib.rs", "pub fn f() { unsafe { g() } }\n")
        mk(self.d, "docs/unsafe_baseline.json", json.dumps({"allowed_crates": ["b"], "files": {"crates/b/src/lib.rs": 1}}))
        self.assertEqual(inv.check(self.d), [])
        mk(self.d, "crates/b/src/lib.rs", "pub fn f() { unsafe { g() } unsafe { g() } }\n")
        self.assertTrue(inv.check(self.d))

    def test_crate_without_forbid_and_not_allowed_is_reported(self):
        mk(self.d, "crates/c/Cargo.toml", "[package]\nname='c'\n")
        mk(self.d, "crates/c/src/lib.rs", "pub fn f() {}\n")
        p = inv.check(self.d)
        self.assertTrue(any("crates/c" in x for x in p), p)


if __name__ == "__main__":
    unittest.main()
