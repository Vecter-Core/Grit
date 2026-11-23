#!/usr/bin/env python3
"""Sinh docs/STD_REFERENCE.md từ lib/core/prelude.gt (không cần import) và lib/std/src/std.gt (gói `std`): chữ ký, hiệu ứng, điều kiện abort. TẤT ĐỊNH.
Dùng: python3 tools/gen_docs.py            ghi docs/STD_REFERENCE.md
      python3 tools/gen_docs.py --stdout   in ra stdout (test `std_reference_matches_generator` so từng byte; sửa tay tệp ⇒ đỏ)
KHÔNG sửa tay docs/STD_REFERENCE.md: sửa nguồn (chữ ký/chú thích `//` ngay trên mục) hoặc bảng ABORTS dưới đây rồi sinh lại.
Chú thích `//` đặt NGAY TRÊN một mục (không dòng trống xen giữa) trở thành lời mô tả của mục đó."""
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# Điều kiện abort của hàm prelude (thân do compiler/runtime hạ; nguồn sự thật: docs/TCB_INTRINSICS.md). Khoá: `Kiểu::hàm` hoặc `hàm` / `core::hàm`.
ABORTS = {
    "Option::unwrap": "không chạy được trong exe/thông dịch (chỉ là khai báo hợp đồng `⊳ self.is_some()`): dùng `⌥` hoặc `unwrap_or`",
    "Option::is_some": "không chạy được (chỉ là khai báo): dùng `⌥` hoặc `is_none` của std",
    "[T]::len": "không",
    "str::len": "không",
    "str::as_bytes": "không",
    "str::slice": "abort `slice_range` nếu `from > to`, `to > len` hoặc một đầu nằm giữa ký tự nhiều byte",
    "File::len": "không (tay cầm do `read`/`open` tạo luôn hợp lệ)",
    "File::bytes": "không",
    "Cap<Fs>::read": "abort `io_error` nếu không đọc được (dùng `open` để nhận `Result`)",
    "Cap<Fs>::open": "không (lỗi I/O trả `Err(IoError)`)",
    "Cap<Fs>::sub": "không",
    "Cap<Fs>::write": "không (lỗi I/O trả `Err(IoError)`)",
    "Cap<Net>::connect": "abort `io_error` nếu kết nối thất bại",
    "Cap<Env>::var": "xem docs/TCB_INTRINSICS.md (giá trị trả về là chỉ số, chưa dùng trong Seed 1.0)",
    "Cap<Env>::arg": "abort `arg_out_of_range` nếu `i ≥ arg_count()`",
    "Cap<Env>::arg_count": "không",
    "Cap<Out>::write_u64": "abort `io_error` nếu ghi stdout thất bại",
    "Cap<Out>::write_str": "abort `io_error` nếu ghi stdout thất bại",
    "Cap<Out>::write_bytes": "abort `io_error` nếu ghi stdout thất bại",
    "Cap<In>::read_all": "không (lỗi I/O trả `Err(IoError)`)",
    "Cap<ErrOut>::write_u64": "không (lỗi ghi stderr bị bỏ qua)",
    "Cap<ErrOut>::write_str": "không (lỗi ghi stderr bị bỏ qua)",
    "Cap<Clock>::now": "không",
    "Cap<Alloc>::arena": "không",
    "Host::fs": "không",
    "Arena::new": "không",
    "Arena::alloc": "abort `arena_exhausted` nếu hết bộ nhớ (hoặc vượt `GRITRT_ARENA_LIMIT`)",
    "Arena::reset": "không (vô hiệu hoá MỌI `&T`/`Vec`/`Text` lấy từ arena: M6 chặn tĩnh)",
    "Vec::new": "không (không cấp phát)",
    "Vec::push": "abort `overflow` nếu `len + 1` hoặc dung lượng tràn; abort nếu cấp phát arena thất bại",
    "Vec::reserve": "abort `overflow` nếu dung lượng tràn; abort nếu cấp phát thất bại",
    "Vec::len": "không",
    "Vec::as_slice": "không",
    "Vec::as_mut_slice": "không",
    "Vec::truncate": "không (cắt còn `min(n, len)`, không bao giờ tăng)",
    "Text::new": "không",
    "Text::push_str": "abort `overflow`/cấp phát thất bại như `Vec::push`",
    "Text::push_char": "abort `overflow`/cấp phát thất bại như `Vec::push`",
    "Text::push_utf8": "như `push_str`; trả `false` (không ghi gì) nếu `b` không phải UTF-8 hợp lệ",
    "Text::from_utf8": "trả `None` nếu không phải UTF-8 hợp lệ; abort nếu cấp phát thất bại",
    "Text::len": "không",
    "Text::as_str": "không",
    "Text::as_bytes": "không",
    "f64::sqrt": "không (NaN cho số âm)",
    "f64::to_i64": "không (trả `None` nếu NaN/∞/ngoài miền)",
    "f64::to_bits": "không",
    "f64::is_nan": "không",
    "shl": "`⊳ n < 32`: vi phạm ⇒ E0901 lúc biên dịch hoặc abort `precondition` lúc chạy",
    "shr": "`⊳ n < 32`: vi phạm ⇒ E0901 lúc biên dịch hoặc abort `precondition` lúc chạy",
    "rotr": "`⊳ n < 32`: vi phạm ⇒ E0901 lúc biên dịch hoặc abort `precondition` lúc chạy",
    "rotl": "`⊳ n < 32`: vi phạm ⇒ E0901 lúc biên dịch hoặc abort `precondition` lúc chạy",
    "band": "không",
    "bor": "không",
    "bxor": "không",
    "bnot": "không",
    "core::abort": "luôn abort (mã 70) kèm thông điệp",
    "core::exit": "không bao giờ trả về; thoát CÓ KIỂM SOÁT với `min(code, 255)` (không phải abort)",
    "core::f64_from_bits": "không",
    "core::fuel_left": "abort `fuel_exhausted` nếu `n == 0` (M7 xoá nếu chứng minh được)",
    "core::hidden": "(chỉ để test)",
    "root_fs": "không (chỉ `main` được gọi: E0310)",
    "root_net": "không (chỉ `main`)",
    "root_env": "không (chỉ `main`)",
    "root_clock": "không (chỉ `main`)",
    "root_alloc": "không (chỉ `main`)",
    "root_out": "không (chỉ `main`)",
    "root_in": "không (chỉ `main`)",
    "root_err": "không (chỉ `main`)",
    "raw_read": "(chỉ cho thử nghiệm bộ nhớ thô)",
    "spawn": "abort nếu không tạo được luồng; không có `join`; đối số phải `Send` (E0501)",
    "mk_buf": "(chỉ cho test)",
    "mk_guard": "(chỉ cho test)",
    "Atomic::new": "không",
    "Atomic::load": "không (thứ tự bộ nhớ phải là hằng tại chỗ gọi, E0503)",
    "Atomic::store": "không (thứ tự là hằng, E0503)",
    "Atomic::swap": "không (thứ tự là hằng, E0503)",
    "Atomic::fetch_add": "không (thứ tự là hằng, E0503)",
    "Atomic::fetch_sub": "không (thứ tự là hằng, E0503)",
    "Atomic::compare_exchange": "không (thứ tự là hằng, E0503)",
    "Mutex::new": "không",
    "Mutex::with": "không (khoá giữ suốt lời gọi `f`; thứ tự khoá kiểm tĩnh E0502)",
    "Cell::new": "không",
    "Cell::get": "không",
    "Cell::set": "không",
    "Owned::new": "hết bộ nhớ ⇒ abort `heap_exhausted` (ADR-C18 bước 2; cần `&Cap<Alloc>`, hiệu ứng `mem`)",
    "Owned::get": "không",
    "Owned::get_mut": "không",
    "Owned::into_inner": "không (trả khối heap; Owned bị move)",
}

STD_ABORT_DEFAULT = (
    "TOÀN PHẦN theo thiết kế: mọi chỉ số/tràn trong thân đều được bảo vệ logic; kiểm tra lúc chạy còn lại (ghi chú N0601/N0602 khi biên dịch gói std) "
    "chỉ là lưới an toàn — chạm vào nó nghĩa là lỗi của std (hãy báo). Hàm cần Arena có thể abort khi cấp phát thất bại."
)

STD_ABORTS = {
    "Vec::insert": "kẹp chỉ số `min(i, len)`; không abort ngoài cấp phát (như `push`)",
    "Vec::extend_from_slice": "như `push` cho từng phần tử",
    "Vec::pop": "không (trả `None` khi rỗng)",
    "Vec::remove": "không (trả `None` nếu `i ≥ len`)",
    "Vec::get": "không (trả `None` nếu ngoài biên)",
    "Vec::first": "không",
    "Vec::last": "không",
    "Vec::clear": "không",
    "Vec::contains": "không",
    "Vec::is_empty": "không",
    "Vec::push_str": "như `Vec::push` cho từng byte",
    "Vec::push_u64": "như `Vec::push`",
    "Vec::push_i64": "như `Vec::push`",
    "HashMap::new": "không",
    "HashMap::insert": "abort nếu cấp phát arena thất bại (bảng nở khi cần)",
    "HashMap::remove": "không",
    "HashMap::get": "không",
    "HashMap::contains_key": "không",
    "HashMap::len": "không",
    "HashMap::entry_count": "không",
    "HashMap::entry_at": "không (trả `None` nếu ngoài biên hoặc entry đã xoá)",
    "pow_u64": "không (trả `None` khi tràn)",
    "abs_i64": "không (trả `None` cho `i64::MIN`)",
    "min": "không",
    "max": "không",
    "[T]::sort": "không (sắp xếp tại chỗ; heapsort, KHÔNG ổn định)",
    "[u8]::find_byte_or_end": "không (có hậu điều kiện `result >= from`)",
    "[u8]::find_byte": "không (trả `None` nếu không có)",
    "[u8]::find": "không (mẫu rỗng ⇒ `Some(0)`)",
    "[u8]::starts_with": "không",
    "[u8]::ends_with": "không",
    "[u8]::trim_ascii": "không",
    "[u8]::equals": "không",
    "[u8]::parse_u64": "không (trả `Err(ParseError)`: Empty > Invalid > Overflow)",
    "[u8]::parse_i64": "không (trả `Err(ParseError)`)",
    "str::parse_u64": "không (trả `Err(ParseError)`)",
    "str::parse_i64": "không (trả `Err(ParseError)`)",
    "fmt_u64": "không (`buf` 24 byte đủ cho mọi u64)",
    "fmt_i64": "không",
    "fmt_hex": "không",
}

STRING_RE = re.compile(r'"(?:\\.|[^"\\])*"')


def strip_code(line):
    """bỏ chuỗi và chú thích `//` để đếm ngoặc"""
    line = STRING_RE.sub('""', line)
    i = line.find("//")
    return line if i < 0 else line[:i]


def collapse(s):
    return re.sub(r"\s+", " ", s).strip()


def extract_sig(text):
    """chữ ký của `ƒ …` từ đầu `text` (có thể nhiều dòng) tới trước thân `{` (hoặc `;`); `! {io}` là tập hiệu ứng chứ không phải thân"""
    i = text.index("ƒ")
    depth = 0
    k = i
    n = len(text)
    prev_nonspace = ""
    while k < n:
        c = text[k]
        if c == "(":
            depth += 1
        elif c == ")":
            depth -= 1
        elif depth == 0 and c == ";":
            return collapse(text[i:k])
        elif depth == 0 and c == "{":
            before = text[i:k].rstrip()
            if before.endswith("!"):
                # tập hiệu ứng: nhảy tới `}` tương ứng
                d = 1
                k += 1
                while k < n and d:
                    d += (text[k] == "{") - (text[k] == "}")
                    k += 1
                continue
            return collapse(text[i:k])
        k += 1
    return collapse(text[i:])


class Item:
    def __init__(self, kind, text, doc, owner=None, name=None):
        self.kind, self.text, self.doc, self.owner, self.name = kind, text, doc, owner, name


def parse(path):
    with open(path, encoding="utf-8") as f:
        lines = f.read().replace("\r\n", "\n").split("\n")
    items = []
    doc = []
    depth = 0
    owner = None  # (kind, header) của khối đang mở ở độ sâu 1
    i = 0
    while i < len(lines):
        raw = lines[i]
        s = raw.strip()
        if s.startswith("//"):
            body = s[2:].strip()
            if depth <= 1 and body and not body.startswith(("---", "===")):
                doc.append(body)
            i += 1
            continue
        if s == "":
            doc = []
            i += 1
            continue
        code = strip_code(raw)
        head = s[2:].lstrip() if s.startswith("◈ ") else s
        if depth == 0:
            if head.startswith("ƒ "):
                blk = "\n".join(lines[i : i + 12])
                sig = extract_sig(blk)
                m = re.match(r"ƒ\s+([A-Za-z_0-9]+)", sig)
                items.append(Item("fn", sig, doc, None, m.group(1) if m else "?"))
            elif head[:1] in ("▢", "⊔", "⊏"):
                full = collapse(code) if code.rstrip().endswith("}") else collapse(code)
                m = re.match(r"[▢⊔⊏]\s+([A-Za-z_0-9]+)", head)
                items.append(Item("type", ("◈ " if s.startswith("◈") else "") + collapse(code.strip().removeprefix("◈ ")) if code.count("{") == code.count("}") else collapse(code.split("{")[0]) + " { … }", doc, None, m.group(1) if m else "?"))
            elif head.startswith("⊐ "):
                hdr = collapse(head.split("{")[0])
                owner = ("impl", hdr)
                items.append(Item("impl", hdr, doc, None, hdr))
                # impl viết gọn trên MỘT dòng: `⊐ Arena { ƒ new() → … { } ƒ reset(&!self) … { } }` — tách từng `ƒ`
                if code.count("{") == code.count("}") and "ƒ " in code:
                    body = code[code.index("{") + 1 : code.rindex("}")]
                    for piece in re.split(r"(?=ƒ\s+[A-Za-z_])", body):
                        if piece.strip().startswith("ƒ"):
                            sig = extract_sig(piece)
                            m = re.match(r"ƒ\s+([A-Za-z_0-9]+)", sig)
                            items.append(Item("method", sig, [], owner, m.group(1) if m else "?"))
            elif head.startswith("▦ "):
                hdr = collapse(head.split("{")[0])
                owner = ("mod", hdr)
            doc = []
        elif depth == 1 and owner is not None:
            if head.startswith("ƒ ") or head.startswith("◈ ƒ"):
                h2 = head[2:].lstrip() if head.startswith("◈ ") else head
                blk = "\n".join(lines[i : i + 12])
                sig = extract_sig(blk)
                m = re.match(r"ƒ\s+([A-Za-z_0-9]+)", sig)
                name = m.group(1) if m else "?"
                items.append(Item("method", sig, doc, owner, name))
            doc = []
        delta = code.count("{") - code.count("}")
        if depth == 0 and delta > 0 and owner is None:
            pass
        depth += delta
        if depth <= 0:
            depth = 0
            owner = None
        i += 1
    return items


def owner_key(item):
    """`Kiểu::hàm` cho khoá bảng ABORTS: lấy phần `Kiểu` của header `⊐ <T> Option<T>` / `⊐ Cap<Out>` / `⊐ [u8]` / `▦ core`"""
    kind, hdr = item.owner
    if kind == "mod":
        return hdr.split()[-1]
    h = hdr[2:].strip()
    h = re.sub(r"^<[^>]*(?:<[^>]*>[^>]*)*>\s*", "", h)  # bỏ `<T: …>` đầu
    h = re.sub(r"^\S+ for ", "", h)  # `Trait for Kiểu`
    base = re.match(r"(\[[^\]]*\]|[A-Za-z_0-9]+(?:<[^>]*(?:<[^>]*>)*[^>]*>)?)", h)
    h = base.group(1) if base else h
    # `Option<T>` → `Option`; `Cap<Out>` giữ nguyên (khác tham số = khác hàng)
    if h.startswith("["):
        return "[T]" if not h.startswith("[u8]") else "[u8]"
    m = re.match(r"([A-Za-z_0-9]+)<(.*)>$", h)
    if m and m.group(1) in ("Cap",):
        return h
    if m:
        return m.group(1)
    return h


def effects_of(sig):
    """tập hiệu ứng của hàm: dấu `!` ở độ sâu ngoặc 0 (không nằm trong danh sách tham số)"""
    depth = 0
    for i, c in enumerate(sig):
        if c in "([<" and c != "<":
            depth += 1
        elif c in ")]":
            depth -= 1
        elif c == "!" and depth == 0:
            rest = sig[i + 1 :].lstrip()
            if rest.startswith("∅"):
                return "∅"
            if rest.startswith("{"):
                d = 0
                for k, ch in enumerate(rest):
                    d += (ch == "{") - (ch == "}")
                    if d == 0:
                        return rest[: k + 1]
    return "∅"


INTERNAL = {"swap_at", "sift_down", "parse_digits", "first_non_ws", "end_after_last_non_ws", "next_cap", "Ent"}


def render_items(items, aborts, default_abort):
    out = []
    internal = [x for x in items if x.name in INTERNAL and x.kind in ("fn", "type")]
    items = [x for x in items if not (x.name in INTERNAL and x.kind in ("fn", "type"))]
    types = [x for x in items if x.kind == "type"]
    fns = [x for x in items if x.kind == "fn"]
    impls = {}
    order = []
    for x in items:
        if x.kind == "method":
            k = x.owner[1]
            if k not in impls:
                impls[k] = []
                order.append(k)
            impls[k].append(x)
    if types:
        out.append("### Kiểu\n")
        for t in types:
            d = " ".join(t.doc)
            out.append(f"- `{t.text}`" + (f" — {d}" if d else ""))
        out.append("")
    if fns:
        out.append("### Hàm tự do\n")
        for f in fns:
            out.append(entry(f, f.name, aborts, default_abort))
        out.append("")
    for k in order:
        out.append(f"### `{k}`\n")
        for m in impls[k]:
            out.append(entry(m, f"{owner_key(m)}::{m.name}", aborts, default_abort))
        out.append("")
    if internal:
        names = ", ".join(f"`{x.name}`" for x in internal)
        out.append(f"### Nội bộ (công khai vì các impl dùng chung; KHÔNG dựa vào, có thể đổi)\n\n{names}\n")
    return out


def entry(x, key, aborts, default_abort):
    d = " ".join(x.doc)
    ab = aborts.get(key)
    if ab is None:
        ab = default_abort
    line = f"- `{x.text}`"
    if d:
        line += f" — {d}"
    ab = ab[6:] if ab.startswith("abort ") else ab
    line += f"\n  - hiệu ứng: `{effects_of(x.text)}`; abort: {ab if ab else '(chưa ghi)'}"
    return line


HEADER = """# STD_REFERENCE — tham chiếu thư viện chuẩn (SINH TỰ ĐỘNG)

> Tệp này được SINH bởi `python3 tools/gen_docs.py` từ `lib/core/prelude.gt` và `lib/std/src/std.gt`; test `std_reference_matches_generator` khoá từng byte.
> KHÔNG sửa tay: sửa nguồn (chữ ký, chú thích `//` ngay trên mục) hoặc bảng điều kiện abort trong `tools/gen_docs.py`, rồi sinh lại.

## Cách đọc
- Chữ ký: `ƒ tên(tham số) → kiểu trả về ! hiệu-ứng [⊳ tiền-điều-kiện]`. Hiệu ứng `∅` = thuần; `{io}` = I/O (cần capability); `{mem}` = cấp phát Arena; `{div}` = có thể không dừng/thoát.
- **abort**: cách chương trình dừng KHẨN CẤP (mã thoát 70) khi vi phạm — `overflow` (số học tràn), `bounds` (chỉ số/lát ngoài biên), `precondition` (`⊳` bị vi phạm), `slice_range`, `io_error`, `arg_out_of_range`, `fuel_exhausted`, `cast_lossy`. "không" = hàm toàn phần.
- Hàm **prelude** có sẵn không cần nhập. Hàm của gói **`std`** là gói Rig `lib/std`: `rig new` tự thêm; tên tự do phải gọi `std::tên` (`std::HashMap::new`, `std::fmt_u64`); phương thức trên kiểu có sẵn (`Vec`, `[u8]`, `str`, `Option`, …) dùng được ngay khi gói std có trong đồ thị.
- Mọi tham chiếu đến `Arena`: `Vec<'a, T>`/`Text<'a>` giữ `&'a Arena` CHIA SẺ; phần tử `Vec` phải không cần drop (`Copy`): E0318.
- Mã lỗi giải thích ở `docs/errors/` và `docs/spec/error_codes.md`.

"""


def generate():
    pre = parse(os.path.join(ROOT, "lib", "core", "prelude.gt"))
    std = parse(os.path.join(ROOT, "lib", "std", "src", "std.gt"))
    out = [HEADER.rstrip("\n"), "", "## Prelude (lib/core/prelude.gt)", ""]
    out += render_items(pre, ABORTS, "(chưa ghi)")
    out += ["## Gói `std` (lib/std/src/std.gt)", "", f"Mặc định cho hàm không có ghi chú riêng: {STD_ABORT_DEFAULT}", ""]
    out += render_items(std, STD_ABORTS, "toàn phần theo thiết kế (xem mặc định ở trên)")
    return "\n".join(out).rstrip("\n") + "\n"


if __name__ == "__main__":
    text = generate()
    if "--stdout" in sys.argv:
        sys.stdout.buffer.write(text.encode("utf-8"))
    else:
        with open(os.path.join(ROOT, "docs", "STD_REFERENCE.md"), "w", encoding="utf-8", newline="\n") as f:
            f.write(text)
        print("đã ghi docs/STD_REFERENCE.md", len(text.splitlines()), "dòng")
