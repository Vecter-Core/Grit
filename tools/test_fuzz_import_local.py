"""GÓI 12 P2.c: test tools/fuzz_import_local.py và dòng log Linux của tools/fuzz_session.sh (log GIẢ LẬP, không chạy fuzz thật)."""
import json
import os
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import fuzz_import_local as L
import fuzz_total as T

H = "abc123abc123"


def cur(_t):
    return H


def row(**kw):
    d = {"ts": "2026-11-20T10:00:00Z", "target": "lex", "status": "ok", "seconds": 300, "runs": 1000, "exit": 0, "src": H,
         "source": "wsl-linux", "os": "Ubuntu 24.04.5 LTS", "kernel": "6.18.33.2-microsoft-standard-WSL2"}
    d.update(kw)
    return d


def write(path, rows):
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        for r in rows:
            f.write((json.dumps(r) if isinstance(r, dict) else r) + "\n")


def lines(path):
    with open(path, encoding="utf-8") as f:
        return [x for x in f.read().splitlines() if x.strip()]


class Import(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.mkdtemp(prefix="fuzzimp-")
        self.src = os.path.join(self.d, "clone.jsonl")
        self.log = os.path.join(self.d, "FUZZ_LOG.jsonl")
        write(self.log, [{"ts": "2026-11-17T10:44:56Z", "target": "lex", "status": "ok", "seconds": 60, "runs": 1, "src": H}])

    def run_import(self, dry=False):
        return L.import_log(self.src, self.log, dry, current=cur)

    def test_valid_rows_are_appended_once_and_not_duplicated(self):
        write(self.src, [row(), row(ts="2026-11-20T10:05:00Z", target="parse", seconds=300)])
        n, dup, skipped, bad = self.run_import()
        self.assertEqual((n, dup, skipped, bad), (2, 0, 0, []))
        self.assertEqual(len(lines(self.log)), 3)
        n, dup, skipped, bad = self.run_import()
        self.assertEqual((n, dup, bad), (0, 2, []))
        self.assertEqual(len(lines(self.log)), 3)

    def test_rows_without_source_and_busy_rows_are_skipped_not_rejected(self):
        write(self.src, [{"ts": "x", "target": "lex", "status": "ok", "seconds": 60, "runs": 1, "src": H},
                         row(status="skipped_busy", seconds=0, runs=0)])
        n, dup, skipped, bad = self.run_import()
        self.assertEqual((n, skipped, bad), (0, 2, []))
        self.assertEqual(len(lines(self.log)), 1)

    def test_foreign_source_is_rejected(self):
        write(self.src, [row(source="windows"), row(ts="t2", source="ci-linux")])
        n, dup, skipped, bad = self.run_import()
        self.assertEqual(n, 0)
        self.assertEqual(len(bad), 2, bad)
        self.assertEqual(len(lines(self.log)), 1)

    def test_mismatching_source_hash_is_rejected(self):
        write(self.src, [row(src="deadbeef0000")])
        n, _dup, _s, bad = self.run_import()
        self.assertEqual(n, 0)
        self.assertEqual(len(bad), 1)
        self.assertIn("băm nguồn", bad[0])

    def test_missing_or_wrong_fields_are_rejected(self):
        bad_rows = [row(os=None), row(kernel=3), row(seconds=True), row(target="check"), row(status="weird"), "không phải json"]
        bad_rows[0].pop("os")
        write(self.src, bad_rows)
        n, _dup, _s, bad = self.run_import()
        self.assertEqual(n, 0)
        self.assertEqual(len(bad), len(bad_rows), bad)

    def test_dry_run_writes_nothing(self):
        write(self.src, [row()])
        n, _d, _s, bad = self.run_import(dry=True)
        self.assertEqual((n, bad), (1, []))
        self.assertEqual(len(lines(self.log)), 1)

    def test_imported_seconds_count_under_wsl_source_in_totals(self):
        write(self.src, [row(seconds=600)])
        self.run_import()
        by = T.seconds_on_hash_by_source("lex", H, self.log)
        self.assertEqual(by.get("wsl-linux"), 600)


if __name__ == "__main__":
    unittest.main()
