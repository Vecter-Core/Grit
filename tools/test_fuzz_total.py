"""GÓI 14 E1: `tools/fuzz_total.py --summary` bằng log GIẢ LẬP (không cần fuzz thật)."""
import datetime
import json
import os
import subprocess
import sys
import tempfile
import unittest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "tools"))
import fuzz_total  # noqa: E402

NOW = datetime.datetime(2026, 11, 21, 12, 0, tzinfo=datetime.timezone.utc)
H = {"lex": "aaaaaaaaaaaa", "parse": "bbbbbbbbbbbb"}


def write_log(rows):
    f = tempfile.NamedTemporaryFile("w", suffix=".jsonl", delete=False, encoding="utf-8", newline="\n")
    for r in rows:
        f.write(json.dumps(r) + "\n")
    f.close()
    return f.name


def ok(target, src, sec, ts="2026-11-20T16:31:47Z", source="wsl-linux"):
    return {"ts": ts, "target": target, "status": "ok", "seconds": sec, "runs": 10, "exit": 0, "src": src, "source": source}


class Summary(unittest.TestCase):
    def test_hours_missing_by_source_last_session_and_estimates(self):
        p = write_log([
            ok("lex", "oldoldoldold", 99999),  # băm cũ: KHÔNG tính
            ok("lex", H["lex"], 3600 * 2, source="windows"),
            ok("lex", H["lex"], 3600, ts="2026-11-21T01:00:00Z"),
            ok("parse", H["parse"], 1800),
        ])
        self.addCleanup(os.unlink, p)
        t = fuzz_total.summary_text(H, p, NOW)
        self.assertIn("lex   băm aaaaaaaaaaaa: có 3.00 h / 24 h — CÒN THIẾU 21.00 h", t)
        self.assertIn("windows 2.00 h", t)
        self.assertIn("wsl-linux 1.00 h", t)
        self.assertIn("parse băm bbbbbbbbbbbb: có 0.50 h / 24 h — CÒN THIẾU 23.50 h", t)
        self.assertIn("phiên cuối lex: 2026-11-21T01:00:00Z status=ok", t)
        self.assertIn("tuần tự còn 44.5 h", t)  # 21 + 23.5
        self.assertIn("song song", t)
        self.assertIn("còn 23.5 h", t)  # max
        self.assertIn("SUY RA", t)
        self.assertNotIn("LỖI", t)

    def test_failed_session_is_reported_and_not_counted(self):
        bad = ok("parse", H["parse"], 600)
        bad["status"] = "fail"
        bad["exit"] = 77
        p = write_log([bad])
        self.addCleanup(os.unlink, p)
        t = fuzz_total.summary_text(H, p, NOW)
        self.assertIn("1 phiên LỖI", t)
        self.assertIn("exit=77", t)
        self.assertIn("có 0.00 h", t)

    def test_full_24_hours_on_both_says_enough(self):
        p = write_log([ok("lex", H["lex"], 24 * 3600), ok("parse", H["parse"], 25 * 3600)])
        self.addCleanup(os.unlink, p)
        t = fuzz_total.summary_text(H, p, NOW)
        self.assertIn("ĐỦ 24 giờ", t)

    def test_empty_log(self):
        t = fuzz_total.summary_text(H, os.path.join(tempfile.gettempdir(), "khong_co_log_xyz.jsonl"), NOW)
        self.assertIn("CÒN THIẾU 24.00 h", t)
        self.assertIn("phiên cuối lex: chưa có", t)

    def test_cli_summary_runs_on_the_real_repo_log(self):
        o = subprocess.run([sys.executable, os.path.join(ROOT, "tools", "fuzz_total.py"), "--summary"], capture_output=True, text=True, encoding="utf-8")
        self.assertEqual(o.returncode, 0, o.stderr)
        self.assertIn("FUZZ_SUMMARY", o.stdout)
        self.assertIn("CÒN THIẾU", o.stdout)
        self.assertLessEqual(len(o.stdout.splitlines()), 20)  # một màn hình


if __name__ == "__main__":
    unittest.main()
