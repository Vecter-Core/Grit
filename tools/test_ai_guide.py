"""AI_GUIDE.md phải ngắn (≤ 12000 token theo PROXY = số byte UTF-8 / 3; không có tiktoken trong môi trường; proxy bảo thủ cho tiếng Việt + mã)
và dòng đầu `<!-- TOKEN_PROXY: ~N … -->` phải khớp số đo (sai lệch ≤ 2%)."""
import os
import re
import unittest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
LIMIT = 12000


class AiGuide(unittest.TestCase):
    def test_size_and_header(self):
        with open(os.path.join(ROOT, "docs", "AI_GUIDE.md"), "rb") as f:
            raw = f.read()
        tokens = len(raw) // 3
        self.assertLessEqual(tokens, LIMIT, f"AI_GUIDE ~{tokens} token > {LIMIT}: cắt bớt, đưa phần còn lại sang STD_REFERENCE/LANGUAGE_REFERENCE")
        first = raw.decode("utf-8").split("\n", 1)[0]
        m = re.search(r"TOKEN_PROXY: ~(\d+)", first)
        self.assertIsNotNone(m, "thiếu dòng đầu TOKEN_PROXY")
        self.assertLessEqual(abs(int(m.group(1)) - tokens), max(1, tokens // 50), f"ghi {m.group(1)} nhưng đo {tokens}")

    def test_forbidden_words_absent(self):
        for name in ("AI_GUIDE.md", "SAFETY_CLAIMS.md", "QUICKSTART.md", "GRIT_BOOK.md", "LANGUAGE_REFERENCE.md"):
            p = os.path.join(ROOT, "docs", name)
            if os.path.exists(p):
                t = open(p, encoding="utf-8").read().lower()
                for w in ("bất tử", "vĩnh cửu", "an toàn tuyệt đối"):
                    if name == "SAFETY_CLAIMS.md":
                        continue  # SAFETY_CLAIMS nêu các từ bị cấm để phủ nhận chúng
                    self.assertNotIn(w, t, f"{name}: từ cấm `{w}`")


if __name__ == "__main__":
    unittest.main()
