#!/usr/bin/env python3
"""Tóm tắt log của MỘT giai đoạn thành đoạn Markdown `audit/results/<ID>.md`.

Dùng: summarize.py <ID> <log> <status> <exit> <giây> <bắt đầu> <tiêu đề> [<ghi chú>]
Chỉ ĐỌC log; không chạm tệp nào của dự án.
"""
import re
import sys

MAX_FAIL_DETAIL = 12
MAX_ERR_LINES = 40


def main():
    sid, log, status, rc, secs, started, title = sys.argv[1:8]
    note = sys.argv[8] if len(sys.argv) > 8 else ""
    try:
        text = open(log, encoding="utf-8", errors="replace").read()
    except OSError:
        text = ""
    lines = text.splitlines()

    out = [f"### {sid} — {title}", ""]
    out.append(f"- **Trạng thái:** {status} (mã thoát {rc}, {secs}s, bắt đầu {started})")
    if note:
        out.append(f"- **Ghi chú của bộ chạy:** {note}")

    # dòng SUMMARY do chính giai đoạn in ra
    summ = [l[len("SUMMARY:"):].strip() for l in lines if l.startswith("SUMMARY:")]
    for s in summ:
        out.append(f"- {s}")

    # tổng số test của cargo
    tot = {"passed": 0, "failed": 0, "ignored": 0, "measured": 0}
    suites = 0
    for l in lines:
        m = re.match(
            r"test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured", l
        )
        if m:
            suites += 1
            tot["passed"] += int(m.group(2))
            tot["failed"] += int(m.group(3))
            tot["ignored"] += int(m.group(4))
            tot["measured"] += int(m.group(5))
    if suites:
        out.append(
            f"- **cargo test:** {tot['passed']} đạt / {tot['failed']} lỗi / {tot['ignored']} ignore "
            f"trong {suites} bộ test"
        )

    # tổng kết unittest Python
    for l in lines:
        m = re.match(r"Ran (\d+) tests? in", l)
        if m:
            out.append(f"- **unittest:** {l.strip()}")
        if l.strip() in ("OK",) or l.startswith("FAILED (") or l.startswith("OK ("):
            out.append(f"- **unittest:** {l.strip()}")

    # clippy / rustc
    ncw = sum(1 for l in lines if re.match(r"^warning(\[.*\])?: ", l) and "generated" not in l)
    nce = sum(1 for l in lines if re.match(r"^error(\[.*\])?: ", l))
    if nce or ncw:
        out.append(f"- **rustc/clippy:** {nce} dòng `error`, {ncw} dòng `warning`")

    fmt_diffs = sum(1 for l in lines if l.startswith("Diff in "))
    if fmt_diffs:
        out.append(f"- **rustfmt:** {fmt_diffs} chỗ khác định dạng chuẩn")

    # test lỗi
    failed = [l for l in lines if re.match(r"^test .+ \.\.\. FAILED$", l)]
    if failed:
        out.append("")
        out.append(f"**Test lỗi ({len(failed)}):**")
        for l in failed[:60]:
            out.append(f"- `{l[len('test '):-len(' ... FAILED')]}`")
        if len(failed) > 60:
            out.append(f"- … và {len(failed) - 60} test nữa (xem log đầy đủ)")
        # chi tiết panic
        blocks = re.split(r"^---- (.+?) stdout ----$", text, flags=re.M)
        details = []
        for i in range(1, len(blocks) - 1, 2):
            name, body = blocks[i], blocks[i + 1]
            body_lines = [b for b in body.strip().splitlines() if b.strip()]
            details.append((name, body_lines[:6]))
        if details:
            out.append("")
            out.append("<details><summary>Chi tiết panic (tối đa %d)</summary>" % MAX_FAIL_DETAIL)
            out.append("")
            for name, bl in details[:MAX_FAIL_DETAIL]:
                out.append(f"`{name}`")
                out.append("```")
                out.extend(x[:300] for x in bl)
                out.append("```")
            out.append("</details>")

    # lỗi biên dịch
    errs = []
    for i, l in enumerate(lines):
        if re.match(r"^error(\[E\d+\])?: ", l) and "could not compile" not in l:
            errs.append((i, l))
    if errs and (status in ("FAIL", "BLOCKED") or nce):
        out.append("")
        out.append("**Lỗi (dòng `error` đầu tiên):**")
        out.append("```")
        shown = 0
        for i, l in errs:
            if shown >= 12:
                break
            out.extend(x[:260] for x in lines[i : i + 4])
            shown += 1
        out.append("```")

    # đuôi log
    tail = lines[-18:]
    out.append("")
    out.append("<details><summary>Đuôi log (18 dòng cuối)</summary>")
    out.append("")
    out.append("```")
    out.extend(x[:300] for x in tail)
    out.append("```")
    out.append("</details>")
    out.append("")

    with open(f"{log.rsplit('/logs/', 1)[0]}/results/{sid}.md", "w", encoding="utf-8") as f:
        f.write("\n".join(out))


if __name__ == "__main__":
    main()
