#!/usr/bin/env python3
"""Test cho tools/secret_scan_history.py (ca dương tính giả lập + âm tính + repo thật tạm)."""
import os, subprocess, sys, tempfile, unittest, json
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import secret_scan_history as S

# Giá trị GIẢ LẬP lắp từ mảnh để chính tệp này không giống bí mật thật.
GH = "gh" + "p_" + "A1b2C3d4E5f6G7h8I9j0K1l2M3n4O5p6Q7r8"
AWS = "AK" + "IA" + "ABCDEFGHIJKLMNOP"
PEM = "-----BEGIN " + "RSA PRIVATE KEY-----"


class T(unittest.TestCase):
    def kinds(self, line, path="a.txt"):
        return {k for k, _ in S.scan_line(path, line)}

    def test_positive(self):
        self.assertIn("github-token", self.kinds("x = " + GH))
        self.assertIn("aws-key", self.kinds(AWS))
        self.assertIn("private-key", self.kinds(PEM))
        self.assertIn("url-credentials", self.kinds("git clone https://user:hunter2xx@host.example/r.git"))
        self.assertIn("assigned-secret", self.kinds('password = "correcthorse1"'))
        self.assertIn("pii:email", self.kinds("mail: someone.real@gmail.com"))
        self.assertIn("pii:windows-user-path", self.kinds(r"C:\Users\Someone\x"))
        self.assertIn("pii:unrelated-keyword", self.kinds("kế hoạch TCBS"))

    def test_negative(self):
        self.assertEqual(self.kinds("let x = 5; // thường"), set())
        self.assertEqual(self.kinds("noreply@anthropic.com"), set())
        self.assertEqual(self.kinds("token = next_token()"), set())
        self.assertEqual(self.kinds("sha 0123456789abcdef0123456789abcdef01234567"), set())

    def test_mask_has_no_secret(self):
        for k, m in S.scan_line("a", GH):
            self.assertTrue(m.endswith("****") and len(m) == 8)
            self.assertNotIn(GH[4:], m)

    def test_real_repo(self):
        with tempfile.TemporaryDirectory() as d:
            def g(*a):
                subprocess.run(["git", "-C", d, "-c", "user.name=t", "-c", "user.email=t@example.com"] + list(a),
                               check=True, capture_output=True)
            g("init", "-q")
            open(os.path.join(d, "k.env"), "w").write("A=1\n")
            open(os.path.join(d, ".env"), "w").write("X=" + GH + "\n")
            g("add", "."); g("commit", "-qm", "c1")
            out = os.path.join(d, "o.json")
            r = subprocess.run([sys.executable, os.path.join(HERE, "secret_scan_history.py"), "--repo", d, "--json", out],
                               capture_output=True, text=True)
            self.assertEqual(r.returncode, 0, r.stderr)
            self.assertNotIn(GH, r.stdout)
            j = json.load(open(out))
            self.assertEqual(j["commits"], 1)
            self.assertTrue(any(f["kind"] == "github-token" for f in j["findings"]))
            self.assertIn(".env", j["dangerous_filenames"])
            self.assertNotIn(GH, json.dumps(j))


if __name__ == "__main__":
    unittest.main()
