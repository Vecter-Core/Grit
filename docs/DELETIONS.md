# DELETIONS — danh sách cho phép MỌI lần xoá tệp đã theo dõi (lệnh Giang 2026-10-16)

Quy tắc: mọi tệp từng bị xoá trong lịch sử git (`git log --diff-filter=D --name-only`) PHẢI có một dòng bảng ở đây: `| đường dẫn | commit | lý do |`.
Xoá dưới `tests/`, `docs/errors/`, `lib/`, `crates/*/src/` mà lý do RỖNG ⇒ cổng ĐỎ (giai đoạn `deletions` của `tools/gate.sh`, `tools/deletions_check.sh`).
Commit xoá tệp phải cập nhật tệp này CÙNG commit (`tools/commit_check.sh`, hook pre-commit).

| đường dẫn | commit | lý do |
|---|---|---|
| tests/TASK.md | 8dad6c1 | tệp giữ chỗ "việc cho milestone sau"; xoá khi chốt đặc tả M0 (Q7–Q14) |
| runtime/gt-rt/TASK.md | ca0ba0a | tệp giữ chỗ; xoá khi M10 bước 4 tạo `runtime/gt-rt` thật |
| tests/bench/TASK.md | 5f110ab | tệp giữ chỗ; xoá khi M10 bước 8–9 thêm benchmark nền (`tests/bench/*.gt`) |
| crates/gt-fmt/src/comments.rs | d370890 | M2e: luật chú thích theo từng vị trí được thay bằng MỘT cơ chế gắn token (`attach.rs`, ADR-016, Q16b); `comments.rs` bị thay thế |
| tests/fail/E1301_comment_in_expression.gt | d370890 | M2e: cơ chế gắn token chấp nhận chú thích ở vị trí này (ADR-016: 0 từ chối khi chèn `// zz` sau mọi token), nên mẫu "fail E1301" không còn đúng; E1301 chỉ còn là biện pháp cuối cùng (KNOWN_LIMITATIONS) |
| tests/fail/E1301_comment_in_signature.gt | d370890 | như trên (chú thích trong chữ ký) |
| .github/workflows/TASK.md | d0a9180 | việc giữ chỗ đã làm: thay bằng .github/workflows/ci.yml |
| deny.toml.TASK.md | d0a9180 | việc giữ chỗ đã làm: thay bằng deny.toml |

## Ghi chú về `lib/core/TASK.md`
Tệp này biến mất khỏi cây làm việc (không qua commit) ngày 2026-10-16 và đã khôi phục bằng `git checkout`; nguyên nhân chưa rõ (xem KNOWN_LIMITATIONS "Tệp bị xoá không rõ nguyên nhân"). Nó vẫn được theo dõi, nên KHÔNG có dòng ở đây (chưa bị xoá trong lịch sử). Nếu sau này chủ ý xoá (giống các `TASK.md` giữ chỗ khác khi M12 đóng), thêm dòng ở đây trong CÙNG commit.
| --help/calc/basic/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/calc/basic/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/calc/blank_lines_only/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/calc/blank_lines_only/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/calc/crlf_is_bad_char/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/calc/crlf_is_bad_char/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/calc/deep_parens/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/calc/deep_parens/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/calc/division_by_zero/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/calc/division_by_zero/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/calc/division_truncates/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/calc/division_truncates/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/calc/empty/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/calc/empty/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/calc/errors_positions/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/calc/errors_positions/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/calc/integer_too_large/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/calc/integer_too_large/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/calc/max_literal_ok/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/calc/max_literal_ok/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/calc/min_div_minus_one/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/calc/min_div_minus_one/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/calc/mixed_good_and_bad/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/calc/mixed_good_and_bad/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/calc/no_trailing_newline/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/calc/no_trailing_newline/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/calc/overflow_add/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/calc/overflow_add/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/calc/overflow_mul/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/calc/overflow_mul/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/calc/overflow_sub/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/calc/overflow_sub/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/calc/precedence/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/calc/precedence/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/calc/random_500/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/calc/random_500/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/calc/unary/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/calc/unary/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/calc/whitespace/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/calc/whitespace/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/big_values_and_text/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/big_values_and_text/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/blank_lines/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/blank_lines/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/crlf/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/crlf/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/dash_means_stdin/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/dash_means_stdin/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/directory_as_file/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/empty_field_makes_nonnumeric/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/empty_field_makes_nonnumeric/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/empty_stdin/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/empty_stdin/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/file_arg/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/file_arg/data.csv | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/header_no_newline/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/header_no_newline/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/header_only/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/header_only/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/i64_extremes/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/i64_extremes/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/mean_negative_fraction/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/mean_negative_fraction/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/mean_trunc/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/mean_trunc/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/missing_file/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/mixed_columns/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/mixed_columns/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/negative_overflow_sum/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/negative_overflow_sum/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/negatives/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/negatives/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/no_trailing_newline/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/no_trailing_newline/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/one_row/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/one_row/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/overflow_literal_is_nonnumeric/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/overflow_literal_is_nonnumeric/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/overflow_sum/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/overflow_sum/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/plus_sign_nonnumeric/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/plus_sign_nonnumeric/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/random_2000x5/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/random_2000x5/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/single_column/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/single_column/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/too_many_args/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/too_many_args/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/unicode_header/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/unicode_header/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/wrong_field_count_long/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/wrong_field_count_long/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/wrong_field_count_short/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/csvstat/wrong_field_count_short/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/grep-lite/basic/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/grep-lite/basic/f1 | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/grep-lite/basic/f2 | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/grep-lite/binary_bytes/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/grep-lite/binary_bytes/b | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/grep-lite/count/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/grep-lite/count/f1 | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/grep-lite/count/f2 | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/grep-lite/count_and_numbers/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/grep-lite/count_and_numbers/f1 | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/grep-lite/crlf_lines/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/grep-lite/crlf_lines/c | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/grep-lite/directory_as_file/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/grep-lite/double_dash/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/grep-lite/double_dash/f1 | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/grep-lite/empty_file/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/grep-lite/empty_file/e | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/grep-lite/empty_file_count/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/grep-lite/empty_file_count/e | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/grep-lite/empty_pattern_matches_all/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/grep-lite/empty_pattern_matches_all/f1 | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/grep-lite/flags_after_pattern_are_files/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/grep-lite/flags_after_pattern_are_files/f1 | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/grep-lite/missing_file_continues/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/grep-lite/missing_file_continues/f1 | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/grep-lite/no_match/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/grep-lite/no_match/f1 | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/grep-lite/numbers/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/grep-lite/numbers/f1 | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/grep-lite/numbers/f2 | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/grep-lite/only_missing/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/grep-lite/overlapping_pattern/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/grep-lite/overlapping_pattern/o | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/grep-lite/pattern_longer_than_line/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/grep-lite/pattern_longer_than_line/o | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/grep-lite/random_3000_lines/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/grep-lite/random_3000_lines/t1 | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/grep-lite/random_3000_lines/t2 | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/grep-lite/usage_no_file/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/grep-lite/usage_nothing/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/kvstore/bad_keys/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/kvstore/bad_keys/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/kvstore/basic/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/kvstore/basic/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/kvstore/binary_values/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/kvstore/binary_values/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/kvstore/blank_lines/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/kvstore/blank_lines/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/kvstore/empty/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/kvstore/empty/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/kvstore/empty_value/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/kvstore/empty_value/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/kvstore/load_bad_key/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/kvstore/load_bad_key/bad.db | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/kvstore/load_bad_key/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/kvstore/load_bad_line/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/kvstore/load_bad_line/bad.db | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/kvstore/load_bad_line/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/kvstore/load_existing_file/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/kvstore/load_existing_file/in.db | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/kvstore/load_existing_file/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/kvstore/load_missing/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/kvstore/load_missing/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/kvstore/load_overrides/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/kvstore/load_overrides/in.db | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/kvstore/load_overrides/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/kvstore/no_trailing_newline/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/kvstore/no_trailing_newline/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/kvstore/overwrite_and_del/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/kvstore/overwrite_and_del/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/kvstore/random_2000/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/kvstore/random_2000/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/kvstore/save_and_load_roundtrip/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/kvstore/save_and_load_roundtrip/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/kvstore/save_unwritable/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/kvstore/save_unwritable/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/kvstore/sorted_dump/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/kvstore/sorted_dump/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/kvstore/unknown_and_usage/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/kvstore/unknown_and_usage/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/kvstore/value_with_spaces/args | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| --help/kvstore/value_with_spaces/stdin | 540b0e5 | thư mục sinh nhầm khi chạy `gen_cases.py --help` (đối số bị coi là thư mục đích; đã sửa gen_cases.py) |
| tests/fail/E0402_text_two_on_one_arena.gt | 3b5989e | R6 (Vec/Text giữ `&'a Arena` chia sẻ): mẫu fail này hết đúng (hai Vec/Text trên một Arena nay HỢP LỆ); chuyển/thay bằng mẫu pass hoặc mẫu fail mới của R6 (ADR-C03) |
| tests/fail/E0402_vec_reset_while_vec_alive.gt | 3b5989e | R6 (Vec/Text giữ `&'a Arena` chia sẻ): mẫu fail này hết đúng (hai Vec/Text trên một Arena nay HỢP LỆ); chuyển/thay bằng mẫu pass hoặc mẫu fail mới của R6 (ADR-C03) |
| tests/fail/E0402_vec_two_on_one_arena.gt | 3b5989e | R6 (Vec/Text giữ `&'a Arena` chia sẻ): mẫu fail này hết đúng (hai Vec/Text trên một Arena nay HỢP LỆ); chuyển/thay bằng mẫu pass hoặc mẫu fail mới của R6 (ADR-C03) |
| tests/fail/E0403_vec_arena_used_while_vec_alive.gt | 3b5989e | R6 (Vec/Text giữ `&'a Arena` chia sẻ): mẫu fail này hết đúng (hai Vec/Text trên một Arena nay HỢP LỆ); chuyển/thay bằng mẫu pass hoặc mẫu fail mới của R6 (ADR-C03) |
| tests/fail/E0310_call_through_fn_value.gt | cc51dea | đổi mã kỳ vọng sang E0313 vì quy tắc intrinsic không làm giá trị chặn root_out trước khi kiểm tra E0310 |
| tests/fail/E0901_pre_fn_value_intrinsic_shl.gt | cc51dea | đổi mã kỳ vọng sang E0313 vì intrinsic không làm giá trị; giữ E0901 cho lời gọi trực tiếp |
| tests/fail/E0901_pre_fn_value_option_unwrap.gt | cc51dea | đổi mã kỳ vọng sang E0313 vì intrinsic không làm giá trị; giữ E0901 cho lời gọi trực tiếp |
| {} | (commit dọn dẹp 2026-11-17) | tệp RỖNG tên `{}` ở thư mục gốc, tạo nhầm bởi một lệnh shell (2026-10-18); không chứa gì, không được tham chiếu |
| tiến trình dự án.md | (chưa commit) | báo cáo chưa theo dõi, đổi tên không dấu thành docs/PROJECT_STATUS.md (lệnh P0b); không có commit xoá |
