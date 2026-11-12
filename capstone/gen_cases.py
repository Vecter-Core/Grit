#!/usr/bin/env python3
"""Sinh bộ tệp đầu vào cho bốn capstone (docs/CAPSTONE.md), TẤT ĐỊNH (hạt giống cố định). Dùng: gen_cases.py <thư-mục-đích> [--large]
Mỗi ca = một thư mục `<k>/<tên>/` chứa `args` (mỗi dòng một đối số; `@DIR@` được thay bằng thư mục của ca lúc chạy), `stdin` (tuỳ chọn) và các tệp dữ liệu.
`--large` thêm các ca lớn (vài MB); mặc định chỉ ca nhỏ/biên/lỗi."""
import os
import random
import sys


def w(base, k, name, args=(), stdin=None, files=None):
    d = os.path.join(base, k, name)
    os.makedirs(d, exist_ok=True)
    with open(os.path.join(d, "args"), "w", encoding="utf-8", newline="\n") as f:
        f.write("".join(a + "\n" for a in args))
    if stdin is not None:
        with open(os.path.join(d, "stdin"), "wb") as f:
            f.write(stdin if isinstance(stdin, bytes) else stdin.encode("utf-8"))
    for fn, data in (files or {}).items():
        with open(os.path.join(d, fn), "wb") as f:
            f.write(data if isinstance(data, bytes) else data.encode("utf-8"))


def csvstat(base, large):
    k = "csvstat"
    w(base, k, "empty_stdin", [], "")
    w(base, k, "header_only", [], "a,b,c\n")
    w(base, k, "header_no_newline", [], "a,b,c")
    w(base, k, "one_row", [], "x,y\n1,2\n")
    w(base, k, "negatives", [], "n\n-5\n3\n-1\n0\n")
    w(base, k, "mean_negative_fraction", [], "n\n-1\n0\n")
    w(base, k, "mean_trunc", [], "n\n1\n2\n2\n")
    w(base, k, "mixed_columns", [], "id,name,score\n1,ann,90\n2,bob,85\n3,cy,100\n")
    w(base, k, "blank_lines", [], "a,b\n1,2\n\n3,4\n\n\n5,6\n")
    w(base, k, "crlf", [], "a,b\r\n1,2\r\n3,4\r\n")
    w(base, k, "no_trailing_newline", [], "a\n7\n8")
    w(base, k, "unicode_header", [], "tên,số\nx,1\n".encode("utf-8"))
    w(base, k, "single_column", [], "v\n10\n20\n30\n")
    w(base, k, "empty_field_makes_nonnumeric", [], "a,b\n1,\n2,3\n")
    w(base, k, "plus_sign_nonnumeric", [], "a\n+5\n6\n")
    w(base, k, "i64_extremes", [], "a\n9223372036854775807\n-9223372036854775808\n")
    w(base, k, "overflow_sum", [], "a\n9223372036854775807\n1\n")
    w(base, k, "overflow_literal_is_nonnumeric", [], "a\n9223372036854775808\n")
    w(base, k, "negative_overflow_sum", [], "a\n-9223372036854775808\n-1\n")
    w(base, k, "wrong_field_count_short", [], "a,b,c\n1,2,3\n4,5\n")
    w(base, k, "wrong_field_count_long", [], "a,b\n1,2\n3,4,5\n")
    w(base, k, "file_arg", ["@DIR@/data.csv"], None, {"data.csv": "a,b\n1,2\n3,4\n"})
    w(base, k, "dash_means_stdin", ["-"], "a\n4\n")
    w(base, k, "missing_file", ["@DIR@/nope.csv"], None)
    w(base, k, "too_many_args", ["a", "b"], "")
    w(base, k, "directory_as_file", ["@DIR@"], None)
    rnd = random.Random(1)
    rows = "\n".join(",".join(str(rnd.randint(-1000, 1000)) for _ in range(5)) for _ in range(2000))
    w(base, k, "random_2000x5", [], "a,b,c,d,e\n" + rows + "\n")
    rows = "\n".join(f"{rnd.randint(-10**15, 10**15)},w{i}" for i in range(500))
    w(base, k, "big_values_and_text", [], "n,t\n" + rows + "\n")
    if large:
        rows = "\n".join(",".join(str(rnd.randint(-10**9, 10**9)) for _ in range(5)) for _ in range(150000))
        w(base, k, "large_150000x5", [], "a,b,c,d,e\n" + rows + "\n")


def calc(base, large):
    k = "calc"
    w(base, k, "empty", [], "")
    w(base, k, "blank_lines_only", [], "\n  \n\t\n")
    w(base, k, "basic", [], "1+2\n3*4\n10-3\n9/2\n")
    w(base, k, "precedence", [], "1+2*3\n(1+2)*3\n2*3+4*5\n100-10-5\n100/10/5\n")
    w(base, k, "unary", [], "-5\n--5\n-(-5)\n3--2\n-2*-3\n- 4\n")
    w(base, k, "whitespace", [], "  1 +\t2  \n\t(  3 )  \n")
    w(base, k, "division_truncates", [], "7/2\n-7/2\n7/-2\n-7/-2\n0/5\n")
    w(base, k, "division_by_zero", [], "1/0\n5/(2-2)\n")
    w(base, k, "overflow_add", [], "9223372036854775807+1\n")
    w(base, k, "overflow_mul", [], "4611686018427387904*2\n")
    w(base, k, "overflow_sub", [], "0-9223372036854775807-2\n")
    w(base, k, "min_div_minus_one", [], "(0-9223372036854775807-1)/-1\n")
    w(base, k, "integer_too_large", [], "9223372036854775808\n")
    w(base, k, "max_literal_ok", [], "9223372036854775807\n-9223372036854775807\n")
    w(base, k, "errors_positions", [], "1+\n(1+2\n1 2\n)\n1+*2\n@\n1+a\n()\n")
    w(base, k, "mixed_good_and_bad", [], "1+1\n1+\n2*2\n")
    w(base, k, "no_trailing_newline", [], "6*7")
    w(base, k, "crlf_is_bad_char", [], "1+1\r\n")
    rnd = random.Random(2)

    def expr(d):
        if d == 0 or rnd.random() < 0.3:
            return str(rnd.randint(0, 99))
        a, b = expr(d - 1), expr(d - 1)
        op = rnd.choice("+-*/")
        if rnd.random() < 0.3:
            a = "(" + a + ")"
        if rnd.random() < 0.3:
            b = "-" + b if rnd.random() < 0.5 else "(" + b + ")"
        return a + " " + op + " " + b

    w(base, k, "random_500", [], "\n".join(expr(4) for _ in range(500)) + "\n")
    w(base, k, "deep_parens", [], "(" * 40 + "1" + ")" * 40 + "\n" + "(" * 40 + "1\n")
    if large:
        w(base, k, "random_20000", [], "\n".join(expr(5) for _ in range(20000)) + "\n")


def kvstore(base, large):
    k = "kvstore"
    w(base, k, "empty", [], "")
    w(base, k, "basic", [], "set a 1\nset b 2\nget a\nget c\ncount\ndump\n")
    w(base, k, "overwrite_and_del", [], "set a 1\nset a 2\nget a\ndel a\ndel a\nget a\ncount\n")
    w(base, k, "value_with_spaces", [], "set k hello world  two\nget k\ndump\n")
    w(base, k, "empty_value", [], "set k\nget k\ndump\ncount\n")
    w(base, k, "bad_keys", [], "set a=b 1\nset  1\nset\nget a=b\ndump\n")
    w(base, k, "unknown_and_usage", [], "frob\nget\ndel\nset\ncount 1\ndump x\nSET a 1\n")
    w(base, k, "sorted_dump", [], "set b 1\nset a 2\nset ab 3\nset B 4\nset a1 5\ndump\n")
    w(base, k, "binary_values", [], b"set k \xff\xfe\x01\nget k\ndump\n")
    w(base, k, "blank_lines", [], "set a 1\n\n\nget a\n")
    w(base, k, "no_trailing_newline", [], "set a 1\nget a")
    w(base, k, "save_and_load_roundtrip", [], "set b 2\nset a 1\nsave @DIR@/db.txt\nset c 3\nload @DIR@/db.txt\ndump\ncount\n")
    w(base, k, "load_existing_file", [], "load @DIR@/in.db\ndump\n", {"in.db": "x=1\ny=hello there\nz=\n"})
    w(base, k, "load_bad_line", [], "set a 1\nload @DIR@/bad.db\ndump\n", {"bad.db": "x=1\nnoequals\ny=2\n"})
    w(base, k, "load_bad_key", [], "load @DIR@/bad.db\ncount\n", {"bad.db": "x=1\n=v\n"})
    w(base, k, "load_missing", [], "load @DIR@/nope.db\ncount\n")
    w(base, k, "save_unwritable", [], "set a 1\nsave @DIR@/no_such_dir/x.db\n")
    w(base, k, "load_overrides", [], "set x old\nload @DIR@/in.db\nget x\n", {"in.db": "x=new\n"})
    rnd = random.Random(3)
    cmds = []
    keys = [f"key{rnd.randint(0, 300)}" for _ in range(2000)]
    for key in keys:
        r = rnd.random()
        if r < 0.5:
            cmds.append(f"set {key} v{rnd.randint(0, 10**6)}")
        elif r < 0.75:
            cmds.append(f"get {key}")
        elif r < 0.9:
            cmds.append(f"del {key}")
        else:
            cmds.append("count")
    cmds.append("dump")
    w(base, k, "random_2000", [], "\n".join(cmds) + "\n")
    if large:
        cmds = [f"set k{i} value{i}" for i in range(50000)] + [f"get k{i}" for i in range(0, 50000, 7)] + ["count", "dump"]
        w(base, k, "large_50000", [], "\n".join(cmds) + "\n")


def grep_lite(base, large):
    k = "grep-lite"
    f1 = "alpha\nbeta gamma\ngamma\ndelta alpha\n"
    f2 = "no match here\nGamma upper\nlast gamma"
    w(base, k, "basic", ["gamma", "@DIR@/f1", "@DIR@/f2"], None, {"f1": f1, "f2": f2})
    w(base, k, "numbers", ["-n", "gamma", "@DIR@/f1", "@DIR@/f2"], None, {"f1": f1, "f2": f2})
    w(base, k, "count", ["-c", "gamma", "@DIR@/f1", "@DIR@/f2"], None, {"f1": f1, "f2": f2})
    w(base, k, "count_and_numbers", ["-c", "-n", "alpha", "@DIR@/f1"], None, {"f1": f1})
    w(base, k, "flags_after_pattern_are_files", ["gamma", "-n", "@DIR@/f1"], None, {"f1": f1})
    w(base, k, "double_dash", ["--", "-n", "@DIR@/f1"], None, {"f1": "a -n b\n-n\nzzz\n"})
    w(base, k, "no_match", ["zzz", "@DIR@/f1"], None, {"f1": f1})
    w(base, k, "empty_pattern_matches_all", ["", "@DIR@/f1"], None, {"f1": f1})
    w(base, k, "empty_file", ["x", "@DIR@/e"], None, {"e": ""})
    w(base, k, "empty_file_count", ["-c", "x", "@DIR@/e"], None, {"e": ""})
    w(base, k, "missing_file_continues", ["alpha", "@DIR@/nope", "@DIR@/f1"], None, {"f1": f1})
    w(base, k, "only_missing", ["alpha", "@DIR@/nope"], None)
    w(base, k, "usage_no_file", ["alpha"], None)
    w(base, k, "usage_nothing", [], None)
    w(base, k, "binary_bytes", ["ÿ", "@DIR@/b"], None, {"b": b"a\xc3\xbf\nplain\n\x00\xc3\xbf\x00\n"})
    w(base, k, "overlapping_pattern", ["aa", "@DIR@/o"], None, {"o": "aaa\nbaab\nab\n"})
    w(base, k, "pattern_longer_than_line", ["abcdef", "@DIR@/o"], None, {"o": "abc\nabcdef\nabcde\n"})
    w(base, k, "crlf_lines", ["b", "@DIR@/c"], None, {"c": "a\r\nb\r\nc\r\n"})
    w(base, k, "directory_as_file", ["x", "@DIR@"], None)
    rnd = random.Random(4)
    words = ["foo", "bar", "baz", "qux", "needle", "hay", "stack", "x"]
    txt = "\n".join(" ".join(rnd.choice(words) for _ in range(rnd.randint(0, 8))) for _ in range(3000)) + "\n"
    w(base, k, "random_3000_lines", ["-n", "needle", "@DIR@/t1", "@DIR@/t2"], None, {"t1": txt, "t2": txt[::-1]})
    if large:
        big = "\n".join(" ".join(rnd.choice(words) for _ in range(10)) for _ in range(200000)) + "\n"
        w(base, k, "large_200000_lines", ["-c", "needle stack", "@DIR@/big"], None, {"big": big})


def main(argv):
    if len(argv) < 2:
        print(__doc__)
        return 3
    base = argv[1]
    if base.startswith("-"):  # `--help` từng bị coi là THƯ MỤC và sinh ra ./--help/ trong kho (2026-10-22)
        print(__doc__)
        return 3
    large = "--large" in argv
    for f in (csvstat, calc, kvstore, grep_lite):
        f(base, large)
    n = sum(len(os.listdir(os.path.join(base, d))) for d in os.listdir(base))
    print(f"gen_cases: {n} ca vào {base}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
