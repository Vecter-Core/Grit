#!/usr/bin/env python3
"""Sinh lib/std/src/std_ord_int.gt (⊐ Ord cho số nguyên, char, bool) — TẤT ĐỊNH, không macro (D8) nên phải lặp.
Dùng: python tools/gen_std.py            ghi tệp
      python tools/gen_std.py --stdout   in ra stdout (test `std_ord_int_file_matches_generator` so từng byte; sửa tay tệp ⇒ đỏ)
Không sửa tay lib/std/src/std_ord_int.gt: sửa script rồi sinh lại."""
import os
import sys

INTS = ["u8", "u16", "u32", "u64", "usize", "i8", "i16", "i32", "i64", "isize"]
PRIM = INTS + ["char"]

HEADER = """// std_ord_int: `⊐ Ord` cho số nguyên, `char`, `bool`. SINH bằng `python tools/gen_std.py` — KHÔNG sửa tay (test khoá từng byte).
// Thứ tự số học thường (có dấu cho i*, mã điểm cho char, false < true cho bool). Không macro (D8) nên mỗi kiểu một impl.
"""

PRIM_IMPL = """⊐ Ord for {t} {{
  ƒ cmp(&self, o: &Self) → Ordering ! ∅ {{
    ⎇ *self < *o {{ Ordering::Less }} ⇝ {{ ⎇ *self > *o {{ Ordering::Greater }} ⇝ {{ Ordering::Equal }} }}
  }}
}}
"""

BOOL_IMPL = """⊐ Ord for bool {
  ƒ cmp(&self, o: &Self) → Ordering ! ∅ {
    ⎇ *self { ⎇ *o { Ordering::Equal } ⇝ { Ordering::Greater } } ⇝ { ⎇ *o { Ordering::Less } ⇝ { Ordering::Equal } }
  }
}
"""


def generate() -> str:
    parts = [HEADER]
    for t in PRIM:
        parts.append(PRIM_IMPL.format(t=t))
    parts.append(BOOL_IMPL)
    return "\n".join(parts)


if __name__ == "__main__":
    text = generate()
    if "--stdout" in sys.argv:
        sys.stdout.buffer.write(text.encode("utf-8"))
    else:
        root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
        with open(os.path.join(root, "lib", "std", "src", "std_ord_int.gt"), "w", encoding="utf-8", newline="\n") as f:
            f.write(text)
