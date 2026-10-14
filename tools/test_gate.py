"""Kiểm thử CHÍNH cổng `tools/gate.sh` (lệnh Giang 2026-10-14, mục 0b): giai đoạn test không biên dịch được / không có dòng `test result` /
có test đỏ / lệnh thoát khác 0 ⇒ cổng ĐỎ (mã thoát ≠ 0, in GATE_RESULT=RED); chỉ khi có kết quả hợp lệ và 0 đỏ mới XANH (mã 0).
Chạy được không cần cargo: GATE_STAGES=test và GATE_TEST_CMD giả lập đầu ra của `cargo test`."""
import os
import subprocess
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


if __name__ == "__main__":
    unittest.main()
