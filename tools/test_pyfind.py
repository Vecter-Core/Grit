"""GÓI 11 S0.b: tools/pyfind.sh chọn python3 khi KHÔNG có `python` (PATH giả), ưu tiên biến PYTHON, và báo lỗi rõ khi không có Python."""
import os
import shutil
import subprocess
import tempfile
import unittest

BASH = shutil.which("bash") or "bash"
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PYFIND = os.path.join(ROOT, "tools", "pyfind.sh").replace("\\", "/")


def fake_bin(d, names):
    for n in names:
        p = os.path.join(d, n)
        with open(p, "w", newline="\n") as f:
            # giả lập một Python 3: thoát 0 với lệnh -c (kiểm phiên bản) và in tên
            f.write('#!/bin/sh\nexit 0\n')
        os.chmod(p, 0o755)


class PyFind(unittest.TestCase):
    def run_find(self, names, extra_env=None):
        d = tempfile.mkdtemp(prefix="pyfind-")
        self.addCleanup(shutil.rmtree, d, True)
        fake_bin(d, names)
        env = {"PATH": d.replace("\\", "/") + ":/usr/bin:/bin", "HOME": d}
        if extra_env:
            env.update(extra_env)
        # PATH CHỈ có thư mục giả (không có python thật): dùng đường dẫn tuyệt đối tới bash
        env["PATH"] = d.replace("\\", "/")
        p = subprocess.run([BASH, "-c", f'. "{PYFIND}"; echo "PY=[$PY]"'], capture_output=True, text=True, encoding="utf-8", errors="replace", env=env)
        return p.stdout + p.stderr

    def test_python3_only_is_found(self):
        self.assertIn("PY=[python3]", self.run_find(["python3"]))

    def test_python_preferred_when_both_exist(self):
        self.assertIn("PY=[python]", self.run_find(["python", "python3"]))

    def test_env_python_wins(self):
        self.assertIn("PY=[mypy]", self.run_find(["python", "mypy"], {"PYTHON": "mypy"}))

    def test_none_found_gives_clear_error(self):
        out = self.run_find([])
        self.assertIn("PY=[]", out)
        self.assertIn("không tìm thấy Python 3", out)


if __name__ == "__main__":
    unittest.main()
