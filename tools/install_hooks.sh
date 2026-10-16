#!/bin/bash
# Cài hook pre-commit (cục bộ, không nằm trong git): bản sao của tools/pre-commit.hook. Chạy lại sau khi clone hoặc đổi máy.
root="$(cd "$(dirname "$0")/.." && pwd)"
cp "$root/tools/pre-commit.hook" "$root/.git/hooks/pre-commit"
chmod +x "$root/.git/hooks/pre-commit"
echo "đã cài $root/.git/hooks/pre-commit"
