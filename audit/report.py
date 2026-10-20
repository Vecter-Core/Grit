#!/usr/bin/env python3
"""Dựng `audit/RESULTS.md` từ `audit/state.tsv` + `audit/results/*.md` (+ `*.note.md` do người viết tay).

Chỉ ĐỌC trạng thái; ghi đúng một tệp RESULTS.md. Chạy lại bao nhiêu lần cũng ra cùng kết quả cho cùng trạng thái.
"""
import os
import re
import subprocess
import sys

AUDIT = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(AUDIT)
STATE = os.path.join(AUDIT, "state.tsv")
RES = os.path.join(AUDIT, "results")


def stage_order():
    """Thứ tự giai đoạn lấy từ `run_all.sh --ids` (một nguồn sự thật)."""
    try:
        out = subprocess.run(
            ["bash", os.path.join(AUDIT, "run_all.sh"), "--ids"],
            capture_output=True, text=True, timeout=30,
        ).stdout
    except Exception:
        return []
    rows = []
    for l in out.splitlines():
        p = l.split("|")
        if len(p) >= 4:
            rows.append((p[0], p[1], p[2], p[3]))  # id, timeout, opt, title
    return rows


def read_state():
    st = {}
    if os.path.exists(STATE):
        for l in open(STATE, encoding="utf-8"):
            p = l.rstrip("\n").split("\t")
            if len(p) >= 5:
                st[p[0]] = p
    return st


def main():
    order = stage_order()
    st = read_state()
    head = ""
    try:
        head = subprocess.run(
            ["git", "-C", ROOT, "log", "-1", "--format=%h %ad %s", "--date=short"],
            capture_output=True, text=True,
        ).stdout.strip()
    except Exception:
        pass

    counts = {}
    for sid, _, _, _ in order:
        s = st.get(sid, [sid, "PENDING"])[1]
        counts[s] = counts.get(s, 0) + 1

    L = []
    L.append("# KẾT QUẢ CHẠY TOÀN BỘ DỰ ÁN GRIT (tự sinh — đừng sửa tay; ghi chú tay đặt ở `audit/results/<ID>.note.md`)")
    L.append("")
    L.append(f"- Mã nguồn được chạy: `{head}` (nhánh làm việc hiện tại; bộ chạy KHÔNG sửa tệp nào của dự án).")
    L.append("- Môi trường: Linux container (xem giai đoạn A00). Dự án được thiết kế cho **Windows x86_64 MSVC + LLVM 20.1.8** (docs/ENVIRONMENT.md).")
    L.append("- Cách chạy/tiếp tục: `bash audit/run_all.sh` (tự bỏ qua giai đoạn đã xong; xem `audit/PLAN.md`).")
    L.append("")
    L.append("## Tổng hợp")
    L.append("")
    L.append("| Trạng thái | Số giai đoạn |")
    L.append("|---|---|")
    for k in ("PASS", "FAIL", "BLOCKED", "TIMEOUT", "SKIP", "RUNNING", "PENDING"):
        if counts.get(k):
            L.append(f"| {k} | {counts[k]} |")
    L.append("")
    L.append("PASS = chạy xong và đạt · FAIL = chạy được nhưng lỗi/thiếu · BLOCKED = KHÔNG chạy được trong môi trường này (có bằng chứng) · "
             "TIMEOUT = quá hạn · SKIP = bỏ qua có chủ ý · PENDING = chưa tới lượt.")
    L.append("")
    L.append("## Bảng giai đoạn")
    L.append("")
    L.append("| ID | Tên | Trạng thái | Thoát | Giây |")
    L.append("|---|---|---|---|---|")
    for sid, _, opt, title in order:
        p = st.get(sid)
        if p:
            L.append(f"| {sid} | {title} | **{p[1]}** | {p[2]} | {p[4]} |")
        else:
            L.append(f"| {sid} | {title} | PENDING | | |")
    L.append("")
    L.append("## Chi tiết từng giai đoạn")
    L.append("")
    for sid, _, _, _ in order:
        f = os.path.join(RES, f"{sid}.md")
        n = os.path.join(RES, f"{sid}.note.md")
        if os.path.exists(f):
            L.append(open(f, encoding="utf-8").read())
            if os.path.exists(n):
                L.append("")
                L.append("**Ghi chú của người phân tích:**")
                L.append("")
                L.append(open(n, encoding="utf-8").read())
            L.append("")
    with open(os.path.join(AUDIT, "RESULTS.md"), "w", encoding="utf-8") as f:
        f.write("\n".join(L) + "\n")


if __name__ == "__main__":
    sys.exit(main())
