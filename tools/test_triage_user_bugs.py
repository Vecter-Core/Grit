"""GÓI 5 S4.b: tools/triage_user_bugs.py đếm đúng; mục mẫu không tính; 'chấp nhận sai' mở ⇒ thoát 1."""
import os
import sys
import unittest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "tools"))
import triage_user_bugs as t  # noqa: E402

DOC = """### UB-0: mẫu
- Loại: chấp nhận sai (nghiêm trọng)
<!-- MỤC MỚI Ở DƯỚI -->
### UB-1: a
- Loại: từ chối oan
- Mức phiền (chỉ từ chối oan): có cách né
- Trạng thái: mở
### UB-2: b
- Loại: chấp nhận sai (nghiêm trọng)
- Trạng thái: đóng (commit abc)
### UB-3: c
- Loại: chấp nhận sai (nghiêm trọng)
- Trạng thái: mở
"""


class Triage(unittest.TestCase):
    def test_counts(self):
        it = t.parse(DOC)
        self.assertEqual([i["id"] for i in it], ["UB-1", "UB-2", "UB-3"])
        self.assertEqual(sum(1 for i in it if i["kind"] == "chap_nhan_sai" and not i["closed"]), 1)
        self.assertEqual(sum(1 for i in it if i["kind"] == "tu_choi_oan"), 1)

    def test_empty_when_no_marker(self):
        self.assertEqual(t.parse("### UB-1: x\n- Loại: từ chối oan\n"), [])


if __name__ == "__main__":
    unittest.main()
