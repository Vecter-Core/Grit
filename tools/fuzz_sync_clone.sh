#!/usr/bin/env bash
# GÓI 14 E2 — đồng bộ clone fuzz (trong WSL) với nhánh của kho chính, GIỮ các dòng log chỉ có ở clone (không mất phiên đã chạy, không trùng).
#   bash tools/fuzz_sync_clone.sh <thư_mục_clone> [kho_nguồn=/mnt/d/Grit] [nhánh=integration]
# Lỗi gốc (kiểm tay GÓI 14): `git checkout FETCH_HEAD` từ chối vì docs/FUZZ_LOG.jsonl của clone khác HEAD; sửa bằng `-f` kèm lưu và nối lại dòng riêng của clone.
# Viết thành tệp riêng (không nhúng chuỗi dài qua `wsl.exe -- bash -lc`): chuỗi dài bị mất phần awk (đo: dòng log phiên cũ biến mất sau lần đồng bộ thứ hai).
set -e
clone="${1:?thiếu thư mục clone}"; src="${2:-/mnt/d/Grit}"; br="${3:-integration}"
[ -d "$clone/.git" ] || git clone -q "$src" "$clone"
cd "$clone"
git config core.autocrlf false
git fetch -q "$src" "$br"
keep="$(mktemp)"
cp -f docs/FUZZ_LOG.jsonl "$keep" 2>/dev/null || : > "$keep"
git checkout -q -f FETCH_HEAD
awk 'NR==FNR{s[$0]=1;next} !($0 in s)' docs/FUZZ_LOG.jsonl "$keep" >> docs/FUZZ_LOG.jsonl
rm -f "$keep"
echo "fuzz_night: clone $(git rev-parse --show-toplevel) tại $(git rev-parse --short HEAD); log $(grep -c '' docs/FUZZ_LOG.jsonl) dòng"
