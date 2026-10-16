#!/bin/bash
# Cài hook pre-commit (cục bộ, không nằm trong git): chạy tools/commit_check.sh. Chạy lại sau khi clone.
root="$(cd "$(dirname "$0")/.." && pwd)"
printf '#!/bin/bash\nexec bash "$(git rev-parse --show-toplevel)/tools/commit_check.sh"\n' > "$root/.git/hooks/pre-commit"
chmod +x "$root/.git/hooks/pre-commit"
echo "đã cài $root/.git/hooks/pre-commit"
