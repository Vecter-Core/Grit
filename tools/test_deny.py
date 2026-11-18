"""GÓI 5 S2.b: deny.toml (allow-wildcard-paths = true) chỉ miễn cho phụ thuộc PATH. Đối chứng dương: bản sao tạm có phụ thuộc crates.io phiên bản "*" thì `cargo deny check bans` PHẢI đỏ;
đối chứng âm: phiên bản cụ thể thì xanh; phụ thuộc path không phiên bản thì xanh. Cần cargo-deny + mạng (CI job `deny` đặt GRIT_REQUIRE_DENY=1 ⇒ thiếu cargo-deny là LỖI; máy cục bộ: bỏ qua có thông báo)."""
import os
import shutil
import subprocess
import tempfile
import unittest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
HAVE = subprocess.run(["cargo", "deny", "--version"], capture_output=True).returncode == 0 if shutil.which("cargo") else False


def make(deps):
    d = tempfile.mkdtemp(prefix="denyt-")
    os.makedirs(os.path.join(d, "src"))
    os.makedirs(os.path.join(d, "loc", "src"))
    with open(os.path.join(d, "loc", "Cargo.toml"), "w") as f:
        f.write('[package]\nname="loc"\nversion="0.1.0"\nedition="2021"\npublish=false\nlicense="MIT"\n')
    open(os.path.join(d, "loc", "src", "lib.rs"), "w").write("")
    with open(os.path.join(d, "Cargo.toml"), "w") as f:
        f.write('[package]\nname="t"\nversion="0.1.0"\nedition="2021"\npublish=false\nlicense="MIT"\n[dependencies]\n' + deps + "\n")
    open(os.path.join(d, "src", "lib.rs"), "w").write("")
    return d


def deny(d):
    r = subprocess.run(["cargo", "deny", "--manifest-path", os.path.join(d, "Cargo.toml"), "--config", os.path.join(ROOT, "deny.toml"), "check", "bans"],
                       capture_output=True, text=True, encoding="utf-8", errors="replace")
    return r.returncode, r.stdout + r.stderr


@unittest.skipUnless(HAVE or os.environ.get("GRIT_REQUIRE_DENY"), "không có cargo-deny (cài: cargo install cargo-deny --locked)")
class DenyWildcard(unittest.TestCase):
    def tmp(self, deps):
        d = make(deps)
        self.addCleanup(shutil.rmtree, d, True)
        return d

    def test_have_cargo_deny_when_required(self):
        if os.environ.get("GRIT_REQUIRE_DENY"):
            self.assertTrue(HAVE, "GRIT_REQUIRE_DENY=1 nhưng thiếu cargo-deny")

    def test_registry_wildcard_is_denied(self):
        rc, out = deny(self.tmp('itoa = "*"'))
        self.assertNotEqual(rc, 0, "phụ thuộc crates.io '*' PHẢI bị từ chối\n" + out[-600:])
        self.assertIn("wildcard", out.lower())

    def test_pinned_version_is_accepted(self):
        rc, out = deny(self.tmp('itoa = "1"'))
        self.assertEqual(rc, 0, out[-600:])

    def test_path_dependency_without_version_is_accepted(self):
        rc, out = deny(self.tmp('loc = { path = "loc" }'))
        self.assertEqual(rc, 0, out[-600:])


if __name__ == "__main__":
    unittest.main()
