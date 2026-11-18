#!/usr/bin/env python3
"""Quét BÍ MẬT và THÔNG TIN CÁ NHÂN trong toàn bộ lịch sử git (GÓI 7 S0).

Dùng:  python tools/secret_scan_history.py [--repo DIR] [--json OUT] [--rev ...]
Chỉ quét dòng THÊM VÀO (`+`) của `git log --all -p`. KHÔNG in giá trị: chỉ
tệp, commit, loại, 4 ký tự đầu che (ví dụ `ghp_****`). Thoát 0 dù có phát
hiện (phát hiện là dữ liệu); 2 nếu lỗi chạy git.
Không phải bằng chứng "không có": chỉ là danh sách mẫu đã quét (PATTERNS).
"""
import argparse, json, math, re, subprocess, sys, collections

PATTERNS = [
    ("github-token", re.compile(r"\b(gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{40,})")),
    ("aws-key", re.compile(r"\b(AKIA|ASIA)[0-9A-Z]{16}\b")),
    ("google-api-key", re.compile(r"\bAIza[0-9A-Za-z_\-]{35}\b")),
    ("anthropic/openai-key", re.compile(r"\bsk-(ant-)?[A-Za-z0-9_\-]{20,}")),
    ("slack-token", re.compile(r"\bxox[baprs]-[A-Za-z0-9\-]{10,}")),
    ("private-key", re.compile(r"-----BEGIN (RSA |EC |OPENSSH |PGP |DSA |ENCRYPTED )?PRIVATE KEY")),
    ("private-key", re.compile(r"-----BEGIN (OPENSSH|PGP) ")),
    ("url-credentials", re.compile(r"[a-z][a-z0-9+.\-]*://[^/\s:@]+:[^/\s@]+@[^/\s]+", re.I)),
    ("bearer", re.compile(r"authorization\s*[:=]\s*[\"']?bearer\s+[A-Za-z0-9._\-]{16,}", re.I)),
    ("assigned-secret", re.compile(
        r"(password|passwd|secret|token|api[_-]?key)[\"']?\s*[:=]\s*[\"'][^\"'\s]{8,}[\"']", re.I)),
]
PII = [
    ("windows-user-path", re.compile(r"[A-Za-z]:\\Users\\[A-Za-z0-9._ -]+", re.I)),
    ("windows-user-path", re.compile(r"/c/Users/[A-Za-z0-9._-]+")),
    ("email", re.compile(r"\b[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}\b")),
    ("phone", re.compile(r"(?<![\w.])(\+?84|0)[ .]?\d{2,3}[ .]?\d{3}[ .]?\d{3,4}(?![\w.])")),
    ("unrelated-keyword", re.compile(r"\b(TCBS|cổ phiếu|chứng khoán|tài khoản ngân hàng|mã số thuế)\b", re.I)),
]
EMAIL_OK = re.compile(r"(noreply|example\.(com|org)|users\.noreply\.github\.com|@local$|@anthropic\.com$|"
                      r"@(foo|bar|test|x|y)\.|\.invalid$|\.test$|\.local$)", re.I)
DANGEROUS_NAMES = re.compile(
    r"(^|/)(\.env(\..*)?|.*\.(pem|key|pfx|p12|bundle|sqlite|sqlite3|mbox|eml)|id_rsa.*|\.git-credentials|credentials.*)$", re.I)
HEX_RUN = re.compile(r"[A-Za-z0-9+/=_\-]{40,}")
SKIP_ENTROPY_PATH = re.compile(r"(\.lock$|Cargo\.lock|\.ll$|\.jsonl$|golden|\.sha|hash|corpus|\.gt$|\.json$)", re.I)


def mask(s):
    s = s.strip()
    return s[:4] + "****" if len(s) > 4 else "****"


def entropy(s):
    c = collections.Counter(s)
    n = len(s)
    return -sum(v / n * math.log2(v / n) for v in c.values())


def scan_line(path, line):
    """Trả danh sách (loại, che) cho một dòng thêm vào."""
    out = []
    for kind, rx in PATTERNS:
        m = rx.search(line)
        if m:
            out.append((kind, mask(m.group(0))))
    for kind, rx in PII:
        for m in rx.finditer(line):
            v = m.group(0)
            if kind == "email" and EMAIL_OK.search(v):
                continue
            out.append(("pii:" + kind, mask(v)))
    if not out and not SKIP_ENTROPY_PATH.search(path or ""):
        for m in HEX_RUN.finditer(line):
            tok = m.group(0)
            if len(tok) >= 40 and entropy(tok) > 4.5 and not re.fullmatch(r"[0-9a-f]+", tok) \
                    and not tok.startswith(("http", "crates/", "tests/")):
                out.append(("high-entropy", mask(tok)))
                break
    return out


def parse_log(text):
    """Sinh (commit, path, line_no_in_hunk, line) cho dòng `+`."""
    commit = path = None
    for raw in text.splitlines():
        if raw.startswith("\x00C "):
            commit = raw[3:].strip()
        elif raw.startswith("+++ "):
            p = raw[4:].strip()
            path = p[2:] if p.startswith("b/") else p
        elif raw.startswith("+") and not raw.startswith("+++"):
            yield commit, path, raw[1:]


def run_git(repo, args):
    r = subprocess.run(["git", "-C", repo] + args, capture_output=True, check=False)
    if r.returncode != 0:
        sys.stderr.write(r.stderr.decode("utf-8", "replace")[:300])
        sys.exit(2)
    return r.stdout.decode("utf-8", "replace")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--repo", default=".")
    ap.add_argument("--json")
    a = ap.parse_args()
    ncommits = len(run_git(a.repo, ["rev-list", "--all"]).split())
    log = run_git(a.repo, ["log", "--all", "--no-color", "-p", "--format=%x00C %H", "--no-renames"])
    findings = collections.OrderedDict()
    files = set()
    for commit, path, line in parse_log(log):
        files.add(path)
        for kind, m in scan_line(path, line):
            key = (kind, path, commit[:10], m)
            findings[key] = findings.get(key, 0) + 1
    names = run_git(a.repo, ["log", "--all", "--diff-filter=A", "--name-only", "--format="])
    bad_names = sorted({n for n in names.splitlines() if n and DANGEROUS_NAMES.match(n)})
    idents = run_git(a.repo, ["log", "--all", "--format=%an|%ae|%cn|%ce"]).splitlines()
    ident_count = collections.Counter(idents)
    res = {
        "commits": ncommits,
        "files_seen": len(files),
        "findings": [{"kind": k[0], "file": k[1], "commit": k[2], "masked": k[3], "lines": n}
                     for k, n in findings.items()],
        "dangerous_filenames": bad_names,
        "identities": [{"ident": mask_ident(i), "commits": n} for i, n in ident_count.items()],
    }
    if a.json:
        with open(a.json, "w", encoding="utf-8") as f:
            json.dump(res, f, ensure_ascii=False, indent=1)
    by = collections.Counter(f["kind"] for f in res["findings"])
    print(f"commits={ncommits} files={len(files)} findings={len(res['findings'])} by_kind={dict(by)}")
    print(f"dangerous_filenames={len(bad_names)}")


def mask_ident(i):
    parts = i.split("|")
    return "|".join(p if "@" not in p else p[:3] + "****@" + p.split("@", 1)[1] for p in parts)


if __name__ == "__main__":
    main()
