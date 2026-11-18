"""Bộ ca cho csv2json: rỗng, biên, hỏng, UTF-8 và byte lạ, lớn vừa phải."""
import random


def cases():
    out = []
    f = "{d}/in.csv"
    def add(name, data, args=None):
        out.append((name, {"in.csv": data}, args if args is not None else [f]))
    add("rong", b"")
    add("chi_tieu_de", b"a,b\n")
    add("chi_tieu_de_khong_xuong_dong", b"a,b")
    add("mot_dong", b"a,b\n1,2\n")
    add("crlf", b"a,b\r\n1,2\r\n3,4\r\n")
    add("khong_xuong_dong_cuoi", b"a,b\n1,2")
    add("dong_rong_giua", b"a,b\n\n1,2\n\n\n3,4\n")
    add("crlf_dong_rong", b"a,b\r\n\r\n1,2\r\n")
    add("cr_le", b"a,b\n1\r2,3\n")
    add("truong_rong", b"a,b,c\n,,\n1,,3\n")
    add("dau_phay_cuoi", b"a,b\n1,\n")
    add("nhay_don_gian", b'a,b\n"x,y","z"\n')
    add("nhay_kep_trong_nhay", b'a\n"he said ""hi"""\n')
    add("nhay_xuong_dong", b'a,b\n"line1\nline2",x\n')
    add("nhay_crlf_trong", b'a,b\n"l1\r\nl2",x\n')
    add("nhay_rong", b'a,b\n"",""\n')
    add("tieu_de_co_nhay", b'"a,1","b ""q"""\n1,2\n')
    add("thoat_json", b'a\n"\\ \t \x01 \x1f \x7f"\n')
    add("tab_va_dieu_khien", b"a\n\x01\x02\x1f\n")
    add("utf8_nhieu_byte", "ten,thanh_pho\nNguyễn Văn A,Hà Nội\n日本語,😀\n".encode("utf-8"))
    add("sai_so_truong_it", b"a,b\n1\n")
    add("sai_so_truong_nhieu", b"a,b\n1,2,3\n")
    add("nhay_trong_truong_khong_nhay", b'a\nab"c\n')
    add("nhay_khong_dong", b'a\n"abc\n')
    add("ky_tu_la_sau_nhay_dong", b'a,b\n"x"y,z\n')
    add("utf8_hong", b"a\n\xff\xfe\n")
    add("utf8_cat_giua", b"a\n\xe6\x97\n")
    add("bom", b"\xef\xbb\xbfa,b\n1,2\n")
    add("chi_dong_rong", b"\n\n\n")
    add("tieu_de_trung_khoa", b"a,a\n1,2\n")
    out.append(("khong_co_doi_so", {}, []))
    out.append(("nhieu_doi_so", {"in.csv": b"a\n1\n"}, [f, f]))
    out.append(("khong_co_tep", {}, ["{d}/khong_co.csv"]))
    rnd = random.Random(11)
    alpha = [b"a", b"b", b",", b'"', b"\n", b"\r\n", b"\r", b"x", b" ", b"1", b"\\", b"\t", "é".encode()]
    for k in range(40):
        add("ngau_nhien_%d" % k, b"".join(rnd.choice(alpha) for _ in range(rnd.randint(0, 40))))
    rows = []
    for r in range(3000):
        rows.append(",".join(['"%d,%d"' % (r, c) if c % 3 == 0 else "v%d_%d" % (r, c) for c in range(8)]))
    add("lon_vua_3000_dong", ("c0,c1,c2,c3,c4,c5,c6,c7\n" + "\n".join(rows) + "\n").encode())
    return out
