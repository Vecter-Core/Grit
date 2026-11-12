#!/usr/bin/env python3
"""Đo cách `gritc check` đối xử với intrinsic/hàm dùng làm GIÁ TRỊ (docs/INTRINSIC_AS_VALUE.md). Mỗi probe là một chương trình nhỏ; in bảng markdown
(phán quyết + tập mã) cho từng gritc truyền vào. Dùng: intrinsic_value_probe.py NHÃN=ĐƯỜNG_DẪN_GRITC [NHÃN=ĐƯỜNG_DẪN_GRITC ...]
Nhãn "ĐO" nghĩa là kết quả do chạy thật lệnh này; cột nào chạy trên bản gritc cũ (trước một sửa) thì người gọi đặt nhãn nêu rõ commit."""
import json
import os
import subprocess
import sys
import tempfile

PROBES = [
    ("cap_source_called_in_main_via_value", "main tự đặt nguồn capability vào biến rồi gọi (main được phép)",
     "ƒ main() → i32 ! {io} { mk ≔ root_out; o ≔ mk(); o.write_u64(1); 0 }"),
    ("cap_source_passed_to_helper_which_calls", "main đưa nguồn capability làm giá trị cho hàm khác GỌI nó (đúc capability ngoài main)",
     "ƒ helper(mk: ƒ() → Cap<Out> ! ∅) ! {io} { o ≔ mk(); o.write_u64(1); }\nƒ main() → i32 ! {io} { helper(root_out); 0 }"),
    ("cap_source_in_nonmain_local", "hàm không phải main đặt nguồn capability vào biến rồi gọi",
     "ƒ f() ! {io} { mk ≔ root_out; o ≔ mk(); o.write_u64(1); }\nƒ main() → i32 ! ∅ { 0 }"),
    ("cap_param_value_ok", "hàm giá trị nhận Cap qua tham số (không phải nguồn)",
     "ƒ emit(out: &Cap<Out>) ! {io} { out.write_u64(1); }\nƒ apply(f: ƒ(&Cap<Out>) ! {io}, out: &Cap<Out>) ! {io} { f(out); }\nƒ main() → i32 ! {io} { o ≔ root_out(); apply(emit, &o); 0 }"),
    ("pre_shl_direct_violated", "đối chứng: gọi TRỰC TIẾP shl với n=40 vi phạm ⊳ n < 32",
     "ƒ main() → u32 ! ∅ { shl(1u32, 40u32) }"),
    ("pre_shl_value_violated", "gọi shl QUA GIÁ TRỊ hàm với n=40 (⊳ n < 32 có còn được kiểm?)",
     "ƒ main() → u32 ! ∅ { f ≔ shl; f(1u32, 40u32) }"),
    ("pre_rotr_value_violated", "như trên với rotr",
     "ƒ main() → u32 ! ∅ { f ≔ rotr; f(1u32, 33u32) }"),
    ("pre_user_fn_value_violated", "hàm người dùng có ⊳ gọi qua giá trị với đối số vi phạm",
     "ƒ pos(n: i32) → i32 ! ∅ ⊳ n > 0 { n }\nƒ main() → i32 ! ∅ { f ≔ pos; f(0) }"),
    ("pre_option_unwrap_value_none", "Option::unwrap (⊳ self.is_some()) qua giá trị với None",
     "ƒ main() → u32 ! ∅ { f ≔ Option::unwrap; f(Option::None) }"),
    ("effect_io_into_pure_param", "hàm có io truyền vào tham số ƒ(u64) ! ∅",
     "ƒ ap(f: ƒ(u64) ! ∅, x: u64) ! ∅ { f(x); }\nƒ w(x: u64) ! {io} { }\nƒ main() ! ∅ { ap(w, 1u64); }"),
    ("effect_div_into_pure_param", "hàm có div (core::abort) truyền vào tham số ƒ(&str) ! ∅",
     "ƒ ap(f: ƒ(&str) ! ∅) ! ∅ { f(\"x\"); }\nƒ main() ! ∅ { ap(core::abort); }"),
    ("effect_mem_into_pure_param", "hàm có mem truyền vào tham số thuần",
     "ƒ ap(f: ƒ() → Arena ! ∅) ! ∅ { a ≔ f(); }\nƒ main() ! ∅ { ap(Arena::new); }"),
    ("effect_subset_ok", "đối chứng: hàm thuần truyền vào tham số cho phép io",
     "ƒ ap(f: ƒ(u64) ! {io}, x: u64) ! {io} { f(x); }\nƒ w(x: u64) ! ∅ { }\nƒ main() ! {io} { ap(w, 1u64); }"),
    ("spawn_as_value", "spawn làm giá trị",
     "ƒ w(x: u32) ! {detach} { }\nƒ main() ! {detach} { s ≔ spawn; s(w, 1u32); }"),
    ("arena_new_value_ok", "Arena::new làm giá trị rồi gọi trong hàm khai báo mem",
     "ƒ main() ! {mem} { f ≔ Arena::new; a ≔ f(); }"),
    ("assoc_fn_value_mem_into_pure_param", "hàm kết hợp Arena::new ({mem}) truyền vào tham số ƒ() → Arena ! ∅",
     "ƒ ap(f: ƒ() → Arena ! ∅) ! ∅ { a ≔ f(); }\nƒ main() ! ∅ { ap(Arena::new); }"),
    ("assoc_fn_value_io_local", "hàm kết hợp người dùng có {io} đặt vào biến rồi gọi trong hàm thuần",
     "▢ S { x: i32 }\n⊐ S { ƒ mk() ! {io} { } }\nƒ main() ! ∅ { f ≔ S::mk; f(); }"),
    ("assoc_fn_value_io_into_pure_param", "hàm kết hợp có {io} truyền vào tham số thuần",
     "▢ S { x: i32 }\n⊐ S { ƒ mk() ! {io} { } }\nƒ ap(f: ƒ() ! ∅) ! ∅ { f(); }\nƒ main() ! ∅ { ap(S::mk); }"),
    ("free_fn_value_io_local", "đối chứng: hàm TỰ DO có {io} đặt vào biến rồi gọi trong hàm thuần",
     "ƒ mk() ! {io} { }\nƒ main() ! ∅ { f ≔ mk; f(); }"),
    ("assoc_fn_direct_io_in_pure", "đối chứng: gọi TRỰC TIẾP hàm kết hợp có {io} trong hàm thuần",
     "▢ S { x: i32 }\n⊐ S { ƒ mk() ! {io} { } }\nƒ main() ! ∅ { S::mk(); }"),
    ("ref_fn_value_two_refs", "hàm hai tham chiếu trả một trong hai, gọi qua giá trị rồi dùng kết quả sau khi nguồn chết",
     "ƒ pick<'a>(a: &'a i32, b: &'a i32) → &'a i32 ! ∅ { a }\nƒ main() → i32 ! ∅ { f ≔ pick; μr ≔ &0; { x ≔ 1; r ← f(&x, &x); } *r }"),
]


def run(gritc, src):
    d = tempfile.mkdtemp(prefix="ivp_")
    p = os.path.join(d, "main.gt")
    with open(p, "w", encoding="utf-8") as f:
        f.write(src + "\n")
    r = subprocess.run([gritc, "check", p, "--json"], capture_output=True, text=True, timeout=120)
    try:
        j = json.loads(r.stdout)
        return f"{j['verdict']} {sorted(set(x['code'] for x in j['diagnostics']))}"
    except Exception:
        return f"NOJSON(rc={r.returncode})"


def main(argv):
    bins = [a.split("=", 1) for a in argv[1:]]
    if not bins:
        print(__doc__)
        return 3
    print("| probe | ý nghĩa | " + " | ".join(b[0] for b in bins) + " |")
    print("|---|---|" + "---|" * len(bins))
    for name, why, src in PROBES:
        print(f"| `{name}` | {why} | " + " | ".join(run(b[1], src) for b in bins) + " |")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
