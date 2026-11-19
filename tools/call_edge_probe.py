#!/usr/bin/env python3
"""GÓI 8 S1a: thí nghiệm CẠNH GỌI GIÁN TIẾP. Sinh chương trình đệ quy QUA container và kiểm bằng `gritc check --json`
(CHỈ kiểm, không chạy: không có rủi ro cạn tài nguyên).

Hai hình dạng:
  direct: f đưa CHÍNH f vào container rồi gọi (đệ quy qua container).
  mutual: f đưa g vào container; g gọi f trực tiếp (đệ quy tương hỗ qua container).
Không có `↓`/`div` ở mọi hàm. Kỳ vọng (D12): ĐỎ (E0702/E0703). XANH ⇒ FB-C nghiêm trọng.
Dùng: python tools/call_edge_probe.py [--gritc ĐƯỜNG_DẪN_TUYỆT_ĐỐI] [--out docs/CALL_EDGE_RESULTS.jsonl]
"""
import json
import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
FT = "ƒ(u32) → u32 ! ∅"
CONTAINERS = ["local", "param", "struct", "option", "tuple", "return"]
# Vec<hàm> KHÔNG viết được ở thí nghiệm này (suy kiểu Vec::new(&arena) + đọc phần tử qua get/⌥): ghi trong docs/CALL_EDGE_AUDIT.md
NL = chr(10)


def program(cont, shape):
    target = "f" if shape == "direct" else "g"
    head = ("ƒ g(n: u32) → u32 ! ∅ { f(n) }" + NL) if shape == "mutual" else ""
    if cont == "param":
        return ("ƒ app(h: " + FT + ", n: u32) → u32 ! ∅ { h(n) }" + NL + head
                + "ƒ f(n: u32) → u32 ! ∅ { app(" + target + ", n) }" + NL
                + "ƒ main() → i32 ! ∅ { f(1u32); 0 }" + NL)
    decl = ""
    if cont == "local":
        body = "μh ≔ " + target + "; h(n)"
    elif cont == "struct":
        decl = "▢ S { h: " + FT + " }" + NL
        body = "s ≔ S { h: " + target + " }; μk ≔ s.h; k(n)"
    elif cont == "option":
        body = "⌥ Some(" + target + ") { Some(h) ↦ h(n), None ↦ n }"
    elif cont == "tuple":
        body = "t ≔ (" + target + ", 1u32); (t.0)(n)"
    elif cont == "return":
        decl = "ƒ pick() → (" + FT + ") ! ∅ { " + target + " }" + NL
        body = "pick()(n)"
    else:
        raise SystemExit("container lạ: " + cont)
    return (decl + head + "ƒ f(n: u32) → u32 ! ∅ { " + body + " }" + NL
            + "ƒ main() → i32 ! ∅ { f(1u32); 0 }" + NL)


def check(gritc, src, name):
    d = os.path.join(ROOT, "target", "probe_call_edge")
    os.makedirs(d, exist_ok=True)
    p = os.path.join(d, name + ".gt")
    with open(p, "w", encoding="utf-8", newline="\n") as f:
        f.write(src)
    try:
        r = subprocess.run([gritc, "check", "--json", p], capture_output=True, text=True,
                           encoding="utf-8", errors="replace", timeout=60)
    except subprocess.TimeoutExpired:
        return "timeout", [], False
    try:
        j = json.loads(r.stdout)
    except ValueError:
        return "error", [], False
    codes = sorted({x.get("code", "") for x in j.get("diagnostics", []) if x.get("code")})
    termination = {"E0702", "E0703", "E0701", "E0312", "N0605", "N0601", "E9901"}
    writable = not any(c not in termination and c[:2] in ("E0", "E1") and c[1:2] == "0" for c in codes) \
        or not codes
    return j.get("verdict"), codes, writable


def main(argv):
    sys.stdout.reconfigure(encoding="utf-8")
    gritc = argv[argv.index("--gritc") + 1] if "--gritc" in argv else os.path.join(ROOT, "target", "release", "gritc.exe")
    out = argv[argv.index("--out") + 1] if "--out" in argv else os.path.join(ROOT, "docs", "CALL_EDGE_RESULTS.jsonl")
    rows = []
    for cont in CONTAINERS:
        for shape in ("direct", "mutual"):
            src = program(cont, shape)
            verdict, codes, writable = check(gritc, src, "%s_%s" % (cont, shape))
            rows.append({"container": cont, "shape": shape, "verdict": verdict, "codes": codes,
                         "writable": writable, "src": src})
    with open(out, "w", encoding="utf-8", newline="\n") as f:
        for r in rows:
            f.write(json.dumps(r, ensure_ascii=False) + NL)
    for r in rows:
        flag = "  <== XANH SAI? (FB-C nghiêm trọng nếu đúng là đệ quy)" if r["verdict"] == "green" else ""
        wr = "" if r["writable"] else "  (không viết được: lỗi kiểu/cú pháp)"
        print("%-7s %-7s %-7s %-16s%s%s" % (r["container"], r["shape"], r["verdict"], ",".join(r["codes"]) or "-", wr, flag))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
