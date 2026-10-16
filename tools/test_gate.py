"""Kiểm thử CHÍNH cổng `tools/gate.sh` (lệnh Giang 2026-10-14, mục 0b): giai đoạn test không biên dịch được / không có dòng `test result` /
có test đỏ / lệnh thoát khác 0 ⇒ cổng ĐỎ (mã thoát ≠ 0, in GATE_RESULT=RED); chỉ khi có kết quả hợp lệ và 0 đỏ mới XANH (mã 0).
Chạy được không cần cargo: GATE_STAGES=test và GATE_TEST_CMD giả lập đầu ra của `cargo test`.
Lệnh Giang 2026-10-16 (mục 0b/0c): giai đoạn `track` — tệp đã theo dõi KHÔNG được đổi trong lúc cổng chạy (kho git tạm)."""
import os
import shutil
import subprocess
import tempfile
import unittest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def gate(test_cmd):
    env = dict(os.environ, GATE_STAGES="test", GATE_TEST_CMD=test_cmd, PYTHONIOENCODING="utf-8")
    p = subprocess.run(["bash", "tools/gate.sh", ROOT.replace("\\", "/")], cwd=ROOT, env=env,
                       capture_output=True, text=True, encoding="utf-8", errors="replace")
    return p.returncode, p.stdout


class GateExitCodes(unittest.TestCase):
    def test_green_when_results_are_valid_and_nothing_failed(self):
        rc, out = gate("echo 'test result: ok. 5 passed; 0 failed; 0 ignored'; echo 'test result: ok. 2 passed; 0 failed; 1 ignored'")
        self.assertEqual(rc, 0, out)
        self.assertIn("TEST_PASSED=7 FAILED=0 SUITES=2", out)
        self.assertIn("GATE_RESULT=GREEN", out)

    def test_red_when_test_stage_does_not_compile(self):
        # mô phỏng `link.exe: Access is denied`: cargo lỗi, không có dòng `test result`
        rc, out = gate("echo 'error: could not exec the linker link.exe'; exit 101")
        self.assertNotEqual(rc, 0, out)
        self.assertIn("TEST_STAGE_INVALID", out)
        self.assertIn("GATE_RESULT=RED", out)

    def test_red_when_there_is_no_test_result_line_even_if_command_exits_zero(self):
        rc, out = gate("echo 'chẳng có gì'")
        self.assertNotEqual(rc, 0, out)
        self.assertIn("TEST_STAGE_INVALID", out)

    def test_red_when_a_test_failed(self):
        rc, out = gate("echo 'test result: FAILED. 3 passed; 1 failed; 0 ignored'; exit 101")
        self.assertNotEqual(rc, 0, out)
        self.assertIn("GATE_RESULT=RED", out)

    def test_red_when_failed_count_is_nonzero_even_if_command_exits_zero(self):
        rc, out = gate("echo 'test result: FAILED. 3 passed; 1 failed; 0 ignored'")
        self.assertNotEqual(rc, 0, out)


def tracked_repo():
    """Kho git tạm: tracked.txt và other.txt đã commit, .gitignore bỏ qua target/. Trả về thư mục."""
    d = tempfile.mkdtemp(prefix="gate-track-")

    def git(*a):
        subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@t", *a], cwd=d, check=True,
                       capture_output=True)

    git("init", "-q")
    for name, text in [("tracked.txt", "gốc\n"), (".gitignore", "target/\n"), ("other.txt", "khác\n")]:
        with open(os.path.join(d, name), "w", encoding="utf-8", newline="\n") as f:
            f.write(text)
    git("add", "-A")
    git("commit", "-q", "-m", "init")
    return d


def gate_in(repo, test_cmd, pre=None):
    if pre:
        pre(repo)
    env = dict(os.environ, GATE_STAGES="test track", GATE_TEST_CMD=test_cmd, PYTHONIOENCODING="utf-8")
    script = os.path.join(ROOT, "tools", "gate.sh").replace("\\", "/")
    p = subprocess.run(["bash", script, repo.replace("\\", "/")], cwd=repo, env=env,
                       capture_output=True, text=True, encoding="utf-8", errors="replace")
    return p.returncode, p.stdout


OK_LINE = "echo 'test result: ok. 1 passed; 0 failed; 0 ignored'; "


class TrackedFilesMustNotChange(unittest.TestCase):
    """Giai đoạn `track` của cổng: giai đoạn nào xoá/sửa tệp đã theo dõi hoặc tạo tệp mới không ignore ⇒ ĐỎ nêu tên tệp."""

    def setUp(self):
        self.repo = tracked_repo()
        self.addCleanup(shutil.rmtree, self.repo, True)

    def test_red_when_a_stage_deletes_a_tracked_file(self):
        rc, out = gate_in(self.repo, OK_LINE + "rm tracked.txt")
        self.assertNotEqual(rc, 0, out)
        self.assertIn("TRACK_CHANGED", out)
        self.assertIn("tracked.txt", out)
        self.assertIn("GATE_RESULT=RED", out)

    def test_red_when_a_stage_modifies_a_tracked_file(self):
        rc, out = gate_in(self.repo, OK_LINE + "echo sua >> tracked.txt")
        self.assertNotEqual(rc, 0, out)
        self.assertIn("TRACK_CHANGED", out)
        self.assertIn("tracked.txt", out)

    def test_red_when_a_stage_creates_a_new_unignored_file(self):
        rc, out = gate_in(self.repo, OK_LINE + "echo moi > moi.txt")
        self.assertNotEqual(rc, 0, out)
        self.assertIn("TRACK_CHANGED", out)
        self.assertIn("moi.txt", out)

    def test_not_red_when_a_stage_only_writes_under_target(self):
        rc, out = gate_in(self.repo, OK_LINE + "mkdir -p target && echo x > target/ra.txt")
        self.assertEqual(rc, 0, out)
        self.assertIn("TRACK_OK", out)

    def test_dirty_tree_at_start_is_fine_but_a_further_change_is_red(self):
        def dirty(repo):
            with open(os.path.join(repo, "other.txt"), "a", encoding="utf-8") as f:
                f.write("dang lam do\n")

        rc, out = gate_in(self.repo, OK_LINE + "true", pre=dirty)
        self.assertEqual(rc, 0, out)
        self.assertIn("TRACK_OK", out)
        rc, out = gate_in(self.repo, OK_LINE + "echo them >> other.txt")
        self.assertNotEqual(rc, 0, out)
        self.assertIn("other.txt", out)


if __name__ == "__main__":
    unittest.main()
