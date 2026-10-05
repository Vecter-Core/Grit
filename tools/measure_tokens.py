"""Đo token: canonical vs ASCII alias (M0, plan §2.3/§6.2).

Dùng tiktoken làm PROXY — KHÔNG phải tokenizer của Claude. Kết quả chỉ để so sánh tương đối.
Chạy: python tools/measure_tokens.py tests/pass/*.gt
"""
import sys

import tiktoken

SYMS = [
    ("ƒ", r"\fn"), ("≔", ":="), ("μ", r"\mut"), ("←", "<-"), ("→", "->"), ("⟳", r"\loop"),
    ("↓", r"\dec"), ("⊳", r"\pre"), ("⊲", r"\post"), ("⋈", r"\inv"), ("∧", r"\land"),
    ("∨", r"\lor"), ("¬", r"\neg"), ("⇒", r"\implies"), ("∀", r"\forall"), ("∃", r"\exists"),
    ("≤", "<="), ("≥", ">="), ("≠", "!="), ("∈", r"\in"), ("∅", r"\nil"), ("⚠", r"\unsafe"),
    ("⊢", r"\proof"), ("⎇", r"\if"), ("⇝", r"\else"), ("⌥", r"\match"), ("↦", "=>"),
    ("⏎", r"\ret"), ("▢", r"\struct"), ("⊔", r"\enum"), ("▦", r"\mod"), ("⇲", r"\use"),
    ("◈", r"\pub"), ("⊏", r"\trait"), ("⊐", r"\impl"), ("⊠", r"\break"), ("⊡", r"\cont"),
    ("≡", r"\type"), ("□", r"\const"), ("■", r"\static"),
]
ALIAS = dict(SYMS)


def to_ascii(src: str) -> str:
    for canon, alias in ALIAS.items():
        src = src.replace(canon, alias + (" " if alias[0].isalpha() or alias.startswith(chr(92)) else ""))
    return src


for name in ("cl100k_base", "o200k_base"):
    enc = tiktoken.get_encoding(name)

    def n(s: str) -> int:
        return len(enc.encode(s))

    print(f"== {name} ==")
    worse = 0
    for canon, alias in SYMS:
        nc, na = n(" " + canon + " "), n(" " + alias + " ")
        flag = "   <- canonical đắt hơn" if nc > na else ""
        worse += nc > na
        print(f"{canon:6} canon={nc} ascii={na}{flag}")
    tc = ta = 0
    for path in sys.argv[1:]:
        with open(path, encoding="utf-8") as f:
            src = f.read()
        tc += n(src)
        ta += n(to_ascii(src))
    if sys.argv[1:]:
        print(f"TỔNG {len(sys.argv) - 1} file: canonical={tc} ascii={ta} ({(ta - tc) / tc * 100:+.1f}% ascii so với canonical)")
    print(f"ký hiệu canonical đắt hơn ASCII: {worse}/{len(SYMS)}")
