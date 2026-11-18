"""GÓI 5 S1.a: tools/tsan_inject.py. Mặc định chỉ thêm sanitize_thread; --drop-locks (CHỈ đối chứng) xoá lệnh gọi lock/unlock; gritc/gt-rt phát hành không có cơ chế bỏ khoá."""
import os
import subprocess
import sys
import tempfile
import unittest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "tools"))
import tsan_inject as ti  # noqa: E402

IR = """declare void @gritrt_mutex_lock(ptr)
declare void @gritrt_mutex_unlock(ptr)
define void @f(ptr %lock) {
  call void @gritrt_mutex_lock(ptr %lock), !gt.obl !3
  store i64 1, ptr %lock
  call void @gritrt_mutex_unlock(ptr %lock), !gt.obl !3
  ret void
}
"""


class Inject(unittest.TestCase):
    def test_default_keeps_locks_and_adds_attr(self):
        o = ti.inject(IR)
        self.assertIn("call void @gritrt_mutex_lock", o)
        self.assertIn("call void @gritrt_mutex_unlock", o)
        self.assertIn("define void @f(ptr %lock) sanitize_thread {", o)

    def test_drop_locks_removes_only_the_calls(self):
        o = ti.inject(IR, drop_locks=True)
        self.assertNotIn("call void @gritrt_mutex", o)
        self.assertIn("declare void @gritrt_mutex_lock", o)
        self.assertIn("store i64 1", o)

    def test_cli_flag(self):
        with tempfile.NamedTemporaryFile("w", suffix=".ll", delete=False, encoding="utf-8") as f:
            f.write(IR)
        self.addCleanup(os.unlink, f.name)
        r = subprocess.run([sys.executable, os.path.join(ROOT, "tools", "tsan_inject.py"), "--drop-locks", f.name], capture_output=True, text=True, encoding="utf-8")
        self.assertEqual(r.returncode, 0)
        self.assertNotIn("call void @gritrt_mutex", r.stdout)

    def test_release_runtime_and_driver_have_no_lock_bypass(self):
        """Cơ chế bỏ khoá không được xuất hiện trong mã sản phẩm (gt-rt, gt-driver, gt-codegen-llvm)."""
        needles = ("drop_locks", "drop-locks", "SKIP_LOCK", "NO_LOCK")
        for base in ("runtime/gt-rt/src", "crates/gt-driver/src", "crates/gt-codegen-llvm/src"):
            for dp, _, fs in os.walk(os.path.join(ROOT, base)):
                for fn in fs:
                    if fn.endswith(".rs"):
                        t = open(os.path.join(dp, fn), encoding="utf-8", errors="replace").read()
                        for n in needles:
                            self.assertNotIn(n, t, f"{fn} chứa {n}")


if __name__ == "__main__":
    unittest.main()
