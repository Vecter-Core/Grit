#!/usr/bin/env python3
"""Đột biến mã nguồn Rust theo dòng (M6, ADR-023 §10a): sinh đột biến, áp từng cái trên MỘT bản sao làm việc, chạy bộ test,
ghi kết quả. Mỗi đột biến cần >=1 test ĐỎ thật (không tính lỗi biên dịch/liên kết); treo = bị bắt.

Dùng:
  python tools/mut.py list  <tệp.rs>...                      # in các đột biến sẽ sinh (không chạy)
  python tools/mut.py run   --workers 3 --out kq.jsonl <tệp.rs>...
  python tools/mut.py report kq.jsonl

Mỗi worker có thư mục riêng `D:/Grit_mut/wN` (sao chép Cargo.*, crates/, tests/) và `target/` riêng; kết quả ghi nối vào
tệp .jsonl nên chạy lại sẽ tiếp tục (bỏ qua đột biến đã có kết quả). Luôn hoàn nguyên tệp sau mỗi đột biến.
Cần PYTHONIOENCODING=utf-8.
"""
import argparse
import concurrent.futures as cf
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import threading

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
WORK = os.environ.get("GRIT_MUT_DIR", "D:/Grit_mut")
TEST_CMD = os.environ.get("GRIT_MUT_TEST", "cargo test -p gt-borrowck").split()
TIMEOUT = int(os.environ.get("GRIT_MUT_TIMEOUT", "900"))

# (regex, thay thế, tên) — áp lên MỖI lần xuất hiện trong dòng mã (không phải chú thích)
RULES = [
    (r"(?<![=!<>])==(?!=)", "!=", "== → !="),
    (r"!=(?!=)", "==", "!= → =="),
    (r" <= ", " < ", "<= → <"),
    (r" < ", " <= ", "< → <="),
    (r" >= ", " > ", ">= → >"),
    (r" > ", " >= ", "> → >="),
    (r" && ", " || ", "&& → ||"),
    (r" \|\| ", " && ", "|| → &&"),
    (r"\btrue\b", "false", "true → false"),
    (r"\bfalse\b", "true", "false → true"),
    (r"(if|while|&&|\|\|) !", r"\1 ", "bỏ phủ định"),
    (r"\.any\(", ".all(", "any → all"),
    (r"\.all\(", ".any(", "all → any"),
    (r"Kind::UniqueBorrow", "Kind::SharedBorrow", "UniqueBorrow → SharedBorrow"),
    (r"Kind::SharedBorrow", "Kind::UniqueBorrow", "SharedBorrow → UniqueBorrow"),
    (r"Kind::Read\b", "Kind::Write", "Read → Write"),
    (r"Kind::Write\b", "Kind::Read", "Write → Read"),
    (r"Kind::Move\b", "Kind::Read", "Move → Read"),
    (r"Depth::Deep", "Depth::Shallow", "Deep → Shallow"),
    (r"Depth::Shallow", "Depth::Deep", "Shallow → Deep"),
    (r"BorrowKind::Unique\b", "BorrowKind::Shared", "Unique → Shared"),
    (r"BorrowKind::Shared\b", "BorrowKind::Unique", "Shared → Unique"),
    (r"return false;", "return true;", "return false → true"),
    (r"return true;", "return false;", "return true → false"),
    (r"\bcontinue;", "break;", "continue → break"),
    (r" \+ 1\b", " - 1", "+1 → -1"),
    (r" - 1\b", " + 1", "-1 → +1"),
    (r"\bis_some\(\)", "is_none()", "is_some → is_none"),
    (r"\bis_none\(\)", "is_some()", "is_none → is_some"),
    (r"\.max\(", ".min(", "max → min"),
    (r"\.min\(", ".max(", "min → max"),
    (r"checked_add\(", "checked_sub(", "checked_add → checked_sub"),
    (r"checked_sub\(", "checked_add(", "checked_sub → checked_add"),
    (r"Tri::Yes\b", "Tri::Maybe", "Yes → Maybe"),
    (r"Tri::No\b", "Tri::Maybe", "No → Maybe"),
    (r"\bis_empty\(\)", "!{X}.is_empty()", None),  # xử lý riêng bên dưới (bỏ qua)
]
RULES = [r for r in RULES if r[2] is not None]

# `if COND {` (kể cả `} else if COND {`) trên một dòng
IF_RE = re.compile(r"^(\s*(?:\} else )?)if (?!let )(.+?) \{\s*$")

# xoá nguyên một câu lệnh (một dòng) có tác dụng phụ lên bảng/tập
DELETE_RE = re.compile(
    r"^(\s*)((self|out|v|acc|res|work|ls|us|stack)\b[\w\.\[\]]*\.(push|insert|remove|extend|union_with|subtract|clear|add_region|constrain|relate)\(.*\)\??;)\s*$"
)


def code_part(line):
    """Phần mã của một dòng (bỏ chú thích `//`; không xử lý chuỗi chứa `//`, chấp nhận)."""
    i = line.find("//")
    return line if i < 0 else line[:i]


def skip_line(line, in_test):
    t = line.strip()
    return (
        in_test
        or not t
        or t.startswith("//")
        or t.startswith("#[")
        or t.startswith("#![")
        or t.startswith("///")
        or t.startswith("use ")
    )


def gen_file(path):
    with open(path, encoding="utf-8", newline="") as f:
        text = f.read()
    crlf = "\r\n" in text
    lines = text.replace("\r\n", "\n").split("\n")
    out = []
    in_test = False
    for ln, line in enumerate(lines):
        if "#[cfg(test)]" in line:
            in_test = True
        if skip_line(line, in_test):
            continue
        code = code_part(line)
        for pat, rep, name in RULES:
            for m in re.finditer(pat, code):
                new = code[: m.start()] + m.expand(rep) + code[m.end():] + line[len(code):]
                if new != line:
                    out.append((path, ln, name, line, new))
        m = IF_RE.match(code)
        if m:
            pre, cond = m.group(1), m.group(2)
            for rep, name in (("true", "condition → true"), ("false", "condition → false"), (f"!({cond})", "condition → phủ định")):
                out.append((path, ln, name, line, f"{pre}if {rep} {{" + line[len(code):]))
        m = DELETE_RE.match(code)
        if m:
            out.append((path, ln, "xoá câu lệnh", line, m.group(1) + "{}; // đã xoá"))
    uniq, seen = [], set()
    for p, ln, name, old, new in out:
        k = (p, ln, new)
        if k not in seen:
            seen.add(k)
            uniq.append((p, ln, name, old, new))
    return uniq, crlf


def mid(path, ln, new):
    return hashlib.sha1(f"{path}:{ln}:{new}".encode("utf-8")).hexdigest()[:12]


class Worker:
    def __init__(self, n):
        self.n = n
        self.dir = f"{WORK}/w{n}"
        self.lock = threading.Lock()

    def prepare(self):
        os.makedirs(self.dir, exist_ok=True)
        for item in ("Cargo.toml", "Cargo.lock"):
            shutil.copy(os.path.join(ROOT, item), self.dir)
        for d in ("crates", "tests", "lib"):
            src, dst = os.path.join(ROOT, d), os.path.join(self.dir, d)
            if os.path.isdir(src):
                shutil.rmtree(dst, ignore_errors=True)
                shutil.copytree(src, dst, ignore=shutil.ignore_patterns("target", "fuzz", "mutants.out"))
        os.makedirs(f"{self.dir}/tmp", exist_ok=True)
        if os.environ.get("GRIT_MUT_NO_OPT") == "1":
            # bản sao làm việc biên dịch gt-oblig ở opt-level 0 (nhanh hơn nhiều; chỉ dùng với bộ test nhỏ, KHÔNG với vét cạn/oracle)
            cp = os.path.join(self.dir, "Cargo.toml")
            with open(cp, encoding="utf-8", newline="") as f:
                t = f.read()
            t = t.replace("opt-level = 2", "opt-level = 0")
            with open(cp, "w", encoding="utf-8", newline="") as f:
                f.write(t)
        for root, _, files in os.walk(f"{self.dir}/crates"):
            for fn in files:
                if fn.endswith(".rs"):
                    os.utime(os.path.join(root, fn), None)

    def env(self):
        e = dict(os.environ)
        e.update(
            CARGO_TARGET_DIR=f"{self.dir}/target",
            TEMP=f"{self.dir}/tmp".replace("/", "\\"),
            TMP=f"{self.dir}/tmp".replace("/", "\\"),
            CARGO_INCREMENTAL="1",
            CARGO_PROFILE_DEV_DEBUG="0",
            CARGO_BUILD_JOBS="2",
            GT_ORACLE_PROGRAMS=os.environ.get("GT_ORACLE_PROGRAMS", "3000"),
            PYTHONIOENCODING="utf-8",
        )
        return e

    def run_tests(self, timeout=None):
        for attempt in range(4):
            proc = subprocess.Popen(TEST_CMD, cwd=self.dir, env=self.env(), stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                    text=True, encoding="utf-8", errors="replace")
            try:
                so, se = proc.communicate(timeout=timeout or TIMEOUT)
            except subprocess.TimeoutExpired:
                # treo = bị bắt; giết CẢ CÂY (tệp thực thi test mồ côi vẫn chạy và khoá `target/`)
                subprocess.run(["taskkill", "/F", "/T", "/PID", str(proc.pid)], capture_output=True)
                proc.communicate()
                return "killed", "timeout"
            out = so + se

            class r:  # noqa: N801 — giữ tên cũ của kết quả
                returncode = proc.returncode
            if re.search(r"LNK1104|LNK1108|os error 1455|os error 32\b", out):
                continue
            if r.returncode == 0:
                return "survived", ""
            if re.search(r"error(\[E\d+\])?:|could not compile", out) and "test result: FAILED" not in out and "panicked" not in out:
                return "invalid", out[-300:]
            return "killed", ""
        return "invalid", "liên kết lỗi 4 lần"

    def apply(self, path, ln, new, crlf):
        rel = os.path.relpath(path, ROOT)
        target = os.path.join(self.dir, rel)
        with open(target, encoding="utf-8", newline="") as f:
            orig = f.read()
        lines = orig.replace("\r\n", "\n").split("\n")
        lines[ln] = new
        mutated = "\n".join(lines)
        if crlf:
            mutated = mutated.replace("\n", "\r\n")
        with open(target, "w", encoding="utf-8", newline="") as f:
            f.write(mutated)
        return target, orig

    def evaluate(self, m, crlf):
        path, ln, name, old, new = m
        with self.lock:
            target, orig = self.apply(path, ln, new, crlf)
            try:
                status, note = self.run_tests()
            finally:
                with open(target, "w", encoding="utf-8", newline="") as f:
                    f.write(orig)
                os.utime(target, None)  # mtime MỚI để cargo biên dịch lại bản gốc (tránh artifact cũ của đột biến)
        return {"id": mid(path, ln, new), "file": os.path.relpath(path, ROOT).replace("\\", "/"), "line": ln + 1,
                "rule": name, "old": old.strip(), "new": new.strip(), "status": status, "note": note}


def cmd_list(args):
    n = 0
    for p in args.files:
        ms, _ = gen_file(p)
        for _, ln, name, old, new in ms:
            print(f"{os.path.relpath(p, ROOT)}:{ln + 1}: [{name}] {old.strip()}  ⇒  {new.strip()}")
            n += 1
    print(f"-- {n} đột biến")


def cmd_run(args):
    done = set()
    if os.path.exists(args.out):
        with open(args.out, encoding="utf-8") as f:
            for ln in f:
                if ln.strip():
                    done.add(json.loads(ln)["id"])
    only = None
    if args.only_survivors:
        # khoá theo (tệp, văn bản đã đột biến): số dòng/ID đổi khi mã nguồn dịch chuyển
        only = {(r["file"], r["new"]) for r in map(json.loads, open(args.only_survivors, encoding="utf-8"))
                if r["status"] == "survived"}
    todo = []
    for p in args.files:
        ms, crlf = gen_file(p)
        for m in ms:
            i = mid(m[0], m[1], m[4])
            key = (os.path.relpath(m[0], ROOT).replace("\\", "/"), m[4].strip())
            lines_env = os.environ.get("GRIT_MUT_LINES")  # vd: prover.rs:226,env.rs:88 (chỉ các dòng này)
            if lines_env and f"{os.path.basename(m[0])}:{m[1] + 1}" not in lines_env.split(","):
                continue
            if i not in done and (only is None or key in only):
                todo.append((m, crlf))
    if args.limit:
        todo = todo[: args.limit]
    print(f"{len(todo)} đột biến cần chạy ({len(done)} đã có kết quả)", flush=True)
    workers = [Worker(i) for i in range(args.workers)]
    for w in workers:
        w.prepare()
    # BASELINE: bộ test phải XANH trên mã chưa đột biến, nếu không mọi đột biến đều "bị bắt" một cách giả tạo
    with cf.ThreadPoolExecutor(max_workers=len(workers)) as ex:
        base = list(ex.map(lambda w: w.run_tests(timeout=3600), workers))
    for w, (st, note) in zip(workers, base):
        if st != "survived":
            sys.exit(f"BASELINE ĐỎ ở worker {w.n} ({st}): {note} — không chạy đột biến.")
    print("baseline xanh ở mọi worker", flush=True)
    q = list(reversed(todo))
    qlock = threading.Lock()
    wlock = threading.Lock()
    counts = {"killed": 0, "survived": 0, "invalid": 0}

    def loop(w):
        while True:
            with qlock:
                if not q:
                    return
                m, crlf = q.pop()
            res = w.evaluate(m, crlf)
            with wlock:
                counts[res["status"]] += 1
                with open(args.out, "a", encoding="utf-8") as f:
                    f.write(json.dumps(res, ensure_ascii=False) + "\n")
                print(f"[{sum(counts.values())}/{len(todo)}] {res['status']:8} {res['file']}:{res['line']} {res['rule']}", flush=True)

    with cf.ThreadPoolExecutor(max_workers=len(workers)) as ex:
        list(ex.map(loop, workers))
    print(counts)


def cmd_report(args):
    rows = [json.loads(ln) for ln in open(args.file, encoding="utf-8") if ln.strip()]
    c = {}
    for r in rows:
        c[r["status"]] = c.get(r["status"], 0) + 1
    print("tổng:", len(rows), c)
    for r in rows:
        if r["status"] == "survived":
            print(f"SỐNG SÓT {r['file']}:{r['line']} [{r['rule']}] {r['old']}  ⇒  {r['new']}")


def main():
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="cmd", required=True)
    a = sub.add_parser("list")
    a.add_argument("files", nargs="+")
    b = sub.add_parser("run")
    b.add_argument("--workers", type=int, default=1)
    b.add_argument("--out", required=True)
    b.add_argument("--limit", type=int, default=0)
    b.add_argument("--only-survivors", help="chỉ chạy lại các đột biến đang SỐNG SÓT trong tệp .jsonl này (đã sửa mã/test từ lần trước)")
    b.add_argument("files", nargs="+")
    c = sub.add_parser("report")
    c.add_argument("file")
    args = ap.parse_args()
    if args.cmd == "list":
        args.files = [os.path.abspath(f) for f in args.files]
        cmd_list(args)
    elif args.cmd == "run":
        args.files = [os.path.abspath(f) for f in args.files]
        cmd_run(args)
    else:
        cmd_report(args)


if __name__ == "__main__":
    main()
