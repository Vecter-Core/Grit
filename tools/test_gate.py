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


NL = chr(10)
HDR = "| đường dẫn | commit | lý do |" + NL + "|---|---|---|" + NL


class DeletionsLedger(unittest.TestCase):
    """Giai đoạn `deletions`: mọi tệp từng bị xoá phải có dòng trong docs/DELETIONS.md (lý do không rỗng nếu dưới tests/, docs/errors/, lib/, crates/*/src/)."""

    def setUp(self):
        self.repo = tracked_repo()
        self.addCleanup(shutil.rmtree, self.repo, True)
        os.makedirs(os.path.join(self.repo, "tests"), exist_ok=True)
        os.makedirs(os.path.join(self.repo, "docs"), exist_ok=True)
        self.write("tests/mau.gt", "x" + NL)
        self.write("docs/DELETIONS.md", HDR)
        self.git("add", "-A")
        self.git("commit", "-q", "-m", "thêm")

    def git(self, *a):
        subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@t", *a], cwd=self.repo, check=True, capture_output=True)

    def write(self, rel, text):
        with open(os.path.join(self.repo, rel), "w", encoding="utf-8", newline=NL) as f:
            f.write(text)

    def run_gate(self):
        env = dict(os.environ, GATE_STAGES="deletions", PYTHONIOENCODING="utf-8")
        script = os.path.join(ROOT, "tools", "gate.sh").replace(chr(92), "/")
        p = subprocess.run(["bash", script, self.repo.replace(chr(92), "/")], cwd=self.repo, env=env, capture_output=True,
                           text=True, encoding="utf-8", errors="replace")
        return p.returncode, p.stdout

    def delete_and_commit(self, rel):
        os.remove(os.path.join(self.repo, rel))
        self.git("add", "-A")
        self.git("commit", "-q", "-m", "xoá " + rel)

    def test_green_when_nothing_was_ever_deleted(self):
        rc, out = self.run_gate()
        self.assertEqual(rc, 0, out)
        self.assertIn("DELETIONS_OK", out)

    def test_red_when_a_deleted_file_has_no_ledger_line(self):
        self.delete_and_commit("tests/mau.gt")
        rc, out = self.run_gate()
        self.assertNotEqual(rc, 0, out)
        self.assertIn("DELETIONS_MISSING", out)
        self.assertIn("tests/mau.gt", out)
        self.assertIn("GATE_RESULT=RED", out)

    def test_green_when_the_deletion_is_recorded_with_a_reason(self):
        self.delete_and_commit("tests/mau.gt")
        self.write("docs/DELETIONS.md", HDR + "| tests/mau.gt | abc1234 | đổi tên sang mau2 |" + NL)
        rc, out = self.run_gate()
        self.assertEqual(rc, 0, out)
        self.assertIn("DELETIONS_OK", out)

    def test_red_when_the_line_exists_but_the_reason_is_empty_in_a_critical_dir(self):
        self.delete_and_commit("tests/mau.gt")
        self.write("docs/DELETIONS.md", HDR + "| tests/mau.gt | abc1234 |  |" + NL)
        rc, out = self.run_gate()
        self.assertNotEqual(rc, 0, out)
        self.assertIn("DELETIONS_NO_REASON", out)

    def test_green_when_a_line_is_recorded_for_a_file_that_was_not_deleted(self):
        self.write("docs/DELETIONS.md", HDR + "| tests/khac.gt | abc1234 | dự phòng |" + NL)
        rc, out = self.run_gate()
        self.assertEqual(rc, 0, out)


class CommitCheck(unittest.TestCase):
    """tools/commit_check.sh: commit (đã stage) có tệp bị xoá phải cập nhật docs/DELETIONS.md cùng commit."""

    def setUp(self):
        self.repo = tracked_repo()
        self.addCleanup(shutil.rmtree, self.repo, True)
        os.makedirs(os.path.join(self.repo, "docs"), exist_ok=True)
        with open(os.path.join(self.repo, "docs", "DELETIONS.md"), "w", encoding="utf-8", newline=NL) as f:
            f.write(HDR)
        self.git("add", "-A")
        self.git("commit", "-q", "-m", "thêm")

    def git(self, *a):
        subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@t", *a], cwd=self.repo, check=True, capture_output=True)

    def check(self):
        script = os.path.join(ROOT, "tools", "commit_check.sh").replace(chr(92), "/")
        p = subprocess.run(["bash", script, self.repo.replace(chr(92), "/")], capture_output=True, text=True, encoding="utf-8", errors="replace")
        return p.returncode, p.stdout

    def test_staged_deletion_without_ledger_update_is_refused(self):
        os.remove(os.path.join(self.repo, "other.txt"))
        self.git("add", "-A")
        rc, out = self.check()
        self.assertNotEqual(rc, 0, out)
        self.assertIn("other.txt", out)

    def test_staged_deletion_with_ledger_line_in_the_same_commit_is_accepted(self):
        os.remove(os.path.join(self.repo, "other.txt"))
        with open(os.path.join(self.repo, "docs", "DELETIONS.md"), "a", encoding="utf-8", newline=NL) as f:
            f.write("| other.txt | abc1234 | thử |" + NL)
        self.git("add", "-A")
        rc, out = self.check()
        self.assertEqual(rc, 0, out)

    def test_commit_without_deletions_is_accepted(self):
        with open(os.path.join(self.repo, "other.txt"), "a", encoding="utf-8") as f:
            f.write("thêm" + NL)
        self.git("add", "-A")
        rc, out = self.check()
        self.assertEqual(rc, 0, out)


class NulStage(unittest.TestCase):
    """Giai đoạn `nul` của cổng: tệp văn bản được theo dõi chứa byte NUL ⇒ ĐỎ nêu tên tệp (sự cố 2026-10-17)."""

    def run_gate(self, repo):
        env = dict(os.environ, GATE_STAGES="nul", PYTHONIOENCODING="utf-8")
        script = os.path.join(ROOT, "tools", "gate.sh").replace(chr(92), "/")
        p = subprocess.run(["bash", script, repo.replace(chr(92), "/")], cwd=repo, env=env, capture_output=True, text=True,
                           encoding="utf-8", errors="replace")
        return p.returncode, p.stdout

    def test_clean_repo_is_green_and_nul_file_is_red(self):
        repo = tracked_repo()
        self.addCleanup(shutil.rmtree, repo, True)
        rc, out = self.run_gate(repo)
        self.assertEqual(rc, 0, out)
        self.assertIn("NUL_OK", out)
        with open(os.path.join(repo, "tracked.txt"), "wb") as f:
            f.write(bytes(64))
        subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "-am", "nul"], cwd=repo, check=True, capture_output=True)
        rc, out = self.run_gate(repo)
        self.assertNotEqual(rc, 0, out)
        self.assertIn("tracked.txt", out)
        self.assertIn("GATE_RESULT=RED", out)


class SavepointTreeHash(unittest.TestCase):
    """GÓI 3 S2.d: điểm lưu của cổng gắn với BĂM CÂY (HEAD + diff theo dõi + NỘI DUNG tệp mới không ignore). Đổi nội dung (kể cả tệp chưa theo dõi
    giữ nguyên tên) giữa hai lần chạy ⇒ làm lại từ đầu; GATE_TREE và băm từng giai đoạn được in."""

    def setUp(self):
        self.repo = tracked_repo()
        self.addCleanup(shutil.rmtree, self.repo, True)

    def run_gate(self):
        env = dict(os.environ, GATE_STAGES="nul", PYTHONIOENCODING="utf-8")
        script = os.path.join(ROOT, "tools", "gate.sh").replace(chr(92), "/")
        p = subprocess.run(["bash", script, self.repo.replace(chr(92), "/")], cwd=self.repo, env=env, capture_output=True,
                           text=True, encoding="utf-8", errors="replace")
        return p.returncode, p.stdout

    def write(self, rel, text):
        with open(os.path.join(self.repo, rel), "w", encoding="utf-8", newline=NL) as f:
            f.write(text)

    @staticmethod
    def tree_of(out):
        for line in out.splitlines():
            if line.startswith("GATE_TREE="):
                return line.split("=", 1)[1].strip()
        return None

    def test_prints_tree_hash_and_stage_hash_equal_to_it(self):
        rc, out = self.run_gate()
        self.assertEqual(rc, 0, out)
        tree = self.tree_of(out)
        self.assertTrue(tree and len(tree) == 64, out)
        self.assertIn("GATE_STAGE_TREE nul " + tree, out)

    def test_unchanged_tree_reuses_savepoint_and_keeps_hash(self):
        rc, out1 = self.run_gate()
        rc, out2 = self.run_gate()
        self.assertEqual(rc, 0, out2)
        self.assertIn("ĐÃ ĐẠT", out2)
        self.assertEqual(self.tree_of(out1), self.tree_of(out2))

    def test_changed_untracked_content_with_same_name_forces_full_rerun(self):
        self.write("moi.txt", "một" + NL)
        rc, out1 = self.run_gate()
        self.write("moi.txt", "hai" + NL)
        rc, out2 = self.run_gate()
        self.assertEqual(rc, 0, out2)
        self.assertIn("làm lại từ đầu", out2)
        self.assertNotIn("ĐÃ ĐẠT", out2)
        self.assertNotEqual(self.tree_of(out1), self.tree_of(out2))

    def test_changed_tracked_file_forces_full_rerun(self):
        rc, out1 = self.run_gate()
        self.write("tracked.txt", "đã sửa" + NL)
        rc, out2 = self.run_gate()
        self.assertIn("làm lại từ đầu", out2)
        self.assertNotIn("ĐÃ ĐẠT", out2)


if __name__ == "__main__":
    unittest.main()
