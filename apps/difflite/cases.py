"""Bộ ca cho difflite: rỗng, biên, hỏng (không có tệp), UTF-8 và byte lạ, lớn vừa phải, sát giới hạn."""
import random


def cases():
    out = []
    A, B = "{d}/a.txt", "{d}/b.txt"

    def add(name, a, b, args=None):
        files = {}
        if a is not None:
            files["a.txt"] = a
        if b is not None:
            files["b.txt"] = b
        out.append((name, files, args if args is not None else [A, B]))

    add("ca_hai_rong", b"", b"")
    add("a_rong", b"", b"x\ny\n")
    add("b_rong", b"x\ny\n", b"")
    add("giong_nhau", b"a\nb\nc\n", b"a\nb\nc\n")
    add("khac_mot_dong_giua", b"a\nb\nc\n", b"a\nX\nc\n")
    add("them_dau", b"b\nc\n", b"a\nb\nc\n")
    add("them_cuoi", b"a\nb\n", b"a\nb\nc\n")
    add("xoa_dau", b"a\nb\nc\n", b"b\nc\n")
    add("xoa_cuoi", b"a\nb\nc\n", b"a\nb\n")
    add("doi_cho", b"a\nb\n", b"b\na\n")
    add("hoan_toan_khac", b"a\nb\nc\n", b"x\ny\nz\n")
    add("dong_lap", b"a\na\na\n", b"a\na\n")
    add("dong_lap_2", b"a\nb\na\nb\n", b"b\na\nb\na\n")
    add("hoa_hoi_hoa_hoi", b"x\nx\nx\nx\n", b"x\ny\nx\ny\nx\n")
    add("thieu_lf_cuoi_a", b"a\nb", b"a\nb\n")
    add("thieu_lf_cuoi_b", b"a\nb\n", b"a\nb")
    add("thieu_lf_ca_hai", b"a\nb", b"a\nc")
    add("dong_rong_giua", b"a\n\nb\n", b"a\nb\n")
    add("chi_dong_rong", b"\n\n\n", b"\n\n")
    add("crlf_la_du_lieu", b"a\r\nb\r\n", b"a\nb\n")
    add("utf8", "xin chào\nNguyễn\n日本語\n".encode(), "xin chào\nNguyen\n日本語\n😀\n".encode())
    add("byte_la_khong_utf8", b"\xff\xfe\nA\n", b"\xff\xfe\nB\n")
    add("byte_nul", b"a\x00b\nc\n", b"a\x00b\nd\n")
    add("dong_dai", b"x" * 10000 + b"\n", b"x" * 9999 + b"y\n")
    add("khong_co_tep_a", None, b"x\n")
    add("khong_co_tep_b", b"x\n", None)
    add("khong_co_ca_hai", None, None)
    out.append(("thieu_doi_so", {"a.txt": b"x\n"}, [A]))
    out.append(("thua_doi_so", {"a.txt": b"x\n", "b.txt": b"x\n"}, [A, B, A]))
    rnd = random.Random(9)
    for k in range(60):
        pool = [b"a", b"b", b"c", b"d", b"", b"a b", b"\xc3\xa9"]
        na, nb = rnd.randint(0, 30), rnd.randint(0, 30)
        a = b"".join(rnd.choice(pool) + b"\n" for _ in range(na))
        b = b"".join(rnd.choice(pool) + b"\n" for _ in range(nb))
        add("ngau_nhien_%d" % k, a, b)
    # lớn vừa phải và sát giới hạn
    base = [b"line %d" % i for i in range(1500)]
    mod = list(base)
    for i in range(0, 1500, 37):
        mod[i] = b"changed %d" % i
    del mod[700:710]
    mod.insert(300, b"inserted")
    add("lon_1500_dong", b"\n".join(base) + b"\n", b"\n".join(mod) + b"\n")
    add("gioi_han_2000x2000", b"".join(b"%d\n" % (i % 50) for i in range(2000)), b"".join(b"%d\n" % ((i * 7) % 50) for i in range(2000)))
    add("vuot_gioi_han_2001", b"x\n" * 2001, b"x\n")
    add("vuot_gioi_han_o_bang", b"x\n" * 2000, b"y\n" * 2000 + b"z\n")
    return out
