"""Lưới tệp-đã-theo-dõi của tools/mut.py (lệnh Giang 2026-10-16 mục 0e): mô phỏng bằng kho git tạm.
`track_snapshot` chụp kho, `track_verify` sau MỖI đột biến phải ĐỎ khi kho thật bị sửa/xoá hoặc có tệp mới không ignore; không đỏ khi chỉ ghi vào target/."""
import importlib.util
import os
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
spec = importlib.util.spec_from_file_location("mut", os.path.join(ROOT, "tools", "mut.py"))
mut = importlib.util.module_from_spec(spec)
sys.modules["mut"] = mut
spec.loader.exec_module(mut)


class MutTrackGuard(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.mkdtemp(prefix="mut-track-")
        self.addCleanup(shutil.rmtree, self.d, True)

        def git(*a):
            subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@t", *a], cwd=self.d, check=True, capture_output=True)

        git("init", "-q")
        for n, t in [("a.rs", "gốc\n"), (".gitignore", "target/\n")]:
            with open(os.path.join(self.d, n), "w", encoding="utf-8", newline="\n") as f:
                f.write(t)
        git("add", "-A")
        git("commit", "-q", "-m", "init")
        self.old = (mut.ROOT, mut.TRACK_SNAP)
        mut.ROOT = self.d
        mut.TRACK_SNAP = os.path.join(self.d, "target_snap.txt")  # nằm NGOÀI .gitignore ⇒ xoá sau
        self.addCleanup(lambda: setattr(mut, "ROOT", self.old[0]))
        self.addCleanup(lambda: setattr(mut, "TRACK_SNAP", self.old[1]))
        os.makedirs(os.path.join(self.d, "target"), exist_ok=True)
        mut.TRACK_SNAP = os.path.join(self.d, "target", "snap.txt")
        mut.track_snapshot()

    def test_unchanged_and_target_writes_are_fine(self):
        with open(os.path.join(self.d, "target", "x.txt"), "w") as f:
            f.write("x")
        ok, msg = mut.track_verify()
        self.assertTrue(ok, msg)

    def test_deleting_a_tracked_file_is_caught_and_named(self):
        os.remove(os.path.join(self.d, "a.rs"))
        ok, msg = mut.track_verify()
        self.assertFalse(ok)
        self.assertIn("a.rs", msg)

    def test_modifying_a_tracked_file_is_caught(self):
        with open(os.path.join(self.d, "a.rs"), "a", encoding="utf-8") as f:
            f.write("sửa\n")
        ok, msg = mut.track_verify()
        self.assertFalse(ok)
        self.assertIn("a.rs", msg)

    def test_new_unignored_file_is_caught(self):
        with open(os.path.join(self.d, "moi.rs"), "w") as f:
            f.write("m")
        ok, msg = mut.track_verify()
        self.assertFalse(ok)
        self.assertIn("moi.rs", msg)


if __name__ == "__main__":
    unittest.main()
