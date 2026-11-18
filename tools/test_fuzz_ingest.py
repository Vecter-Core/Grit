"""GÓI 7 S4.b: test tools/fuzz_ingest.py và phần theo-nguồn của tools/fuzz_total.py (log GIẢ LẬP, không dùng fuzz thật)."""
import json
import os
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import fuzz_ingest as I
import fuzz_total as T


def row(**kw):
    d = {"target": "lex", "status": "ok", "seconds": 600, "runs": 1000, "src": "abc123abc123", "ci_run": "111"}
    d.update(kw)
    return d


def write(path, rows):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        for r in rows:
            f.write((json.dumps(r) if isinstance(r, dict) else r) + "\n")


def count(path):
    with open(path, encoding="utf-8") as f:
        return len([x for x in f.read().splitlines() if x.strip()])


class Ingest(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.mkdtemp()
        self.log = os.path.join(self.d, "FUZZ_LOG.jsonl")
        write(self.log, [{"ts": "x", "target": "lex", "status": "ok", "seconds": 60, "runs": 5, "exit": 0, "src": "abc123abc123"}])

    def test_appends_with_source_and_dedups(self):
        write(os.path.join(self.d, "a", "fuzz-lex", "fuzz-log.jsonl"), [row()])
        write(os.path.join(self.d, "a", "fuzz-parse", "fuzz-log.jsonl"), [row(target="parse")])
        self.assertEqual(I.ingest(os.path.join(self.d, "a"), self.log)[:2], (2, 0))
        self.assertEqual(I.ingest(os.path.join(self.d, "a"), self.log)[:2], (0, 2))
        with open(self.log, encoding="utf-8") as f:
            lines = [json.loads(x) for x in f.read().splitlines() if x.strip()]
        self.assertEqual(len(lines), 3)
        self.assertEqual([x.get("source") for x in lines[1:]], ["ci-linux", "ci-linux"])

    def test_new_run_same_target_is_not_dup(self):
        write(os.path.join(self.d, "a", "fuzz-log.jsonl"), [row(ci_run="1")])
        write(os.path.join(self.d, "b", "fuzz-log.jsonl"), [row(ci_run="2")])
        self.assertEqual(I.ingest(os.path.join(self.d, "a"), self.log)[0], 1)
        self.assertEqual(I.ingest(os.path.join(self.d, "b"), self.log)[0], 1)

    def test_rejects_bad_rows(self):
        write(os.path.join(self.d, "a", "fuzz-log.jsonl"),
              [row(target="zzz"), row(seconds="x"), "không phải json", row(status="weird"), json.dumps({"target": "lex"})])
        n, dup, bad = I.ingest(os.path.join(self.d, "a"), self.log)
        self.assertEqual((n, dup, len(bad)), (0, 0, 5))
        self.assertEqual(count(self.log), 1)

    def test_dry_run_writes_nothing(self):
        write(os.path.join(self.d, "a", "fuzz-log.jsonl"), [row()])
        self.assertEqual(I.ingest(os.path.join(self.d, "a"), self.log, dry=True)[0], 1)
        self.assertEqual(count(self.log), 1)

    def test_total_by_source_wall_clock_only_ok_on_hash(self):
        write(self.log, [
            {"target": "lex", "status": "ok", "seconds": 3600, "runs": 1, "src": "H"},
            row(src="H", seconds=7200, source="ci-linux"),
            row(src="H", seconds=999, source="ci-linux", status="fail"),
            row(src="OLD", seconds=5000, source="ci-linux"),
            row(target="parse", src="H", seconds=1800, source="ci-linux"),
        ])
        self.assertEqual(T.seconds_on_hash_by_source("lex", "H", self.log), {"windows": 3600, "ci-linux": 7200})
        self.assertEqual(T.seconds_on_hash_by_source("parse", "H", self.log), {"ci-linux": 1800})
        self.assertEqual(T.seconds_on_hash("lex", "H", self.log), 10800)


if __name__ == "__main__":
    unittest.main()
