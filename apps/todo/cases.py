"""Bộ ca cho todo: dãy lệnh có trạng thái tệp; rỗng, biên, hỏng, UTF-8 và byte lạ, lớn vừa phải."""
import random

F = "{d}/t.txt"


def final_files(d):
    import os
    p = os.path.join(d, "t.txt")
    return open(p, "rb").read() if os.path.exists(p) else None


def cases():
    out = []

    def seq(name, init, *cmds):
        files = {} if init is None else {"t.txt": init}
        out.append((name, files, [[F] + list(c) for c in cmds]))

    seq("list_khong_co_tep", None, ["list"])
    seq("list_tep_rong", b"", ["list"])
    seq("add_vao_tep_chua_co", None, ["add", "mua sua"], ["list"])
    seq("add_nhieu", None, ["add", "a"], ["add", "b"], ["add", "c"], ["list"])
    seq("done_va_list", None, ["add", "a"], ["add", "b"], ["done", "2"], ["list"])
    seq("done_hai_lan", None, ["add", "a"], ["done", "1"], ["done", "1"], ["list"])
    seq("remove_giua", None, ["add", "a"], ["add", "b"], ["add", "c"], ["remove", "2"], ["list"])
    seq("remove_het", None, ["add", "a"], ["remove", "1"], ["list"], ["add", "z"], ["list"])
    seq("so_ngoai_pham_vi", None, ["add", "a"], ["done", "2"], ["remove", "0"], ["remove", "9"], ["list"])
    seq("so_khong_hop_le", None, ["add", "a"], ["done", "x"], ["done", ""], ["done", "-1"], ["done", "+1"], ["done", "1.0"], ["list"])
    seq("so_qua_lon", None, ["add", "a"], ["done", "18446744073709551616"], ["done", "18446744073709551615"], ["list"])
    seq("so_so_0_dau", None, ["add", "a"], ["done", "001"], ["list"])
    seq("add_rong", None, ["add", ""], ["list"])
    seq("add_co_xuong_dong", None, ["add", "a\nb"], ["list"])
    seq("add_utf8", None, ["add", "Nguyễn Văn A 日本語 😀"], ["list"])
    seq("add_ky_tu_dac_biet", None, ["add", '"q" \ \t [x] [ ] '], ["add", " "], ["list"], ["done", "1"], ["list"])
    seq("lenh_la", None, ["fly", "1"], ["list"])
    seq("thieu_doi_so", None, ["add"], ["done"], ["remove"], ["list", "x"])
    seq("qua_nhieu_doi_so", None, ["add", "a", "b"])
    out.append(("khong_co_lenh", {}, [[F]]))
    out.append(("khong_doi_so", {}, [[]]))
    seq("tep_hong_khong_xuong_dong_cuoi", b"[ ] a", ["list"], ["add", "b"])
    seq("tep_hong_dong_sai_dinh_dang", b"[ ] a\nxyz\n", ["list"], ["done", "1"])
    seq("tep_hong_dong_qua_ngan", b"[ ]\n", ["list"])
    seq("tep_hong_dong_rong", b"[ ] a\n\n[ ] b\n", ["list"])
    seq("tep_hong_dau_ngoac", b"[y] a\n", ["list"])
    seq("tep_dung_san", b"[ ] a\n[x] b\n", ["list"], ["done", "1"], ["remove", "2"], ["list"])
    seq("tep_chi_5_byte", b"[ ] a\n", ["list"], ["done", "1"], ["list"])
    out.append(("duong_dan_la_thu_muc", {}, [["{d}"] + ["list"]]))
    rnd = random.Random(5)
    for k in range(25):
        cmds = []
        n = 0
        for _ in range(rnd.randint(1, 25)):
            c = rnd.choice(["add", "add", "done", "remove", "list"])
            if c == "add":
                cmds.append(["add", "task%d" % rnd.randint(0, 99)])
                n += 1
            elif c == "list":
                cmds.append(["list"])
            else:
                cmds.append([c, str(rnd.randint(0, n + 1))])
        seq("ngau_nhien_%d" % k, None, *cmds)
    big = [["add", "item %d" % i] for i in range(400)] + [["done", str(i)] for i in range(1, 400, 7)] + [["remove", "3"]] * 50 + [["list"]]
    seq("lon_400_muc", None, *big)
    return out
