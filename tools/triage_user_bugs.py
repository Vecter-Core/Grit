#!/usr/bin/env python3
"""GÓI 5 S4.b: đọc docs/USER_BUGS.md, in số mục theo loại/trạng thái. Chỉ tính các mục `### UB-<số>:` NẰM SAU dấu `<!-- MỤC MỚI Ở DƯỚI -->` (mẫu ở trên không tính).
Thoát 1 nếu có mục 'chấp nhận sai' chưa đóng (false negative còn mở); 0 nếu không."""
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
MARK = "<!-- MỤC MỚI Ở DƯỚI -->"


def parse(text):
    if MARK not in text:
        return []
    body = text.split(MARK, 1)[1]
    items = []
    for blk in re.split(r"(?m)^### ", body)[1:]:
        head = blk.split("\n", 1)[0]
        if not re.match(r"UB-\d+", head):
            continue
        def f(k):
            m = re.search(r"(?m)^- " + k + r":\s*(.*)$", blk)
            return m.group(1).strip().lower() if m else ""
        loai = f("Loại")
        kind = "chap_nhan_sai" if "chấp nhận sai" in loai else ("tu_choi_oan" if "từ chối oan" in loai else "khong_ro")
        st = f("Trạng thái")
        closed = st.startswith("đóng")
        items.append({"id": head.split(":")[0], "kind": kind, "closed": closed, "annoy": f("Mức phiền")})
    return items


def main():
    p = os.path.join(ROOT, "docs", "USER_BUGS.md")
    items = parse(open(p, encoding="utf-8").read()) if os.path.isfile(p) else []
    out = sys.stdout
    try:
        out.reconfigure(encoding="utf-8")
    except Exception:
        pass
    n = lambda k, c=None: sum(1 for i in items if i["kind"] == k and (c is None or i["closed"] == c))
    print(f"USER_BUGS tổng={len(items)}")
    print(f"  chấp nhận sai (nghiêm trọng): mở={n('chap_nhan_sai', False)} đóng={n('chap_nhan_sai', True)}")
    print(f"  từ chối oan: mở={n('tu_choi_oan', False)} đóng={n('tu_choi_oan', True)}")
    for a in ("chặn việc", "có cách né", "hiếm"):
        print(f"    mức phiền '{a}' (mở): {sum(1 for i in items if i['kind'] == 'tu_choi_oan' and not i['closed'] and i['annoy'] == a)}")
    print(f"  không rõ loại: {n('khong_ro')}")
    return 1 if n("chap_nhan_sai", False) else 0


if __name__ == "__main__":
    sys.exit(main())
