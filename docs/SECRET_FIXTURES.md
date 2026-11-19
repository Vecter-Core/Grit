# SECRET_FIXTURES — phân loại NGOẠI TUYẾN các fixture giống bí mật (GÓI 8 S0f)

Sinh bởi `tools/fixture_secret_check.py`. KHÔNG in giá trị (chỉ 4 ký tự đầu), KHÔNG gọi mạng.
Loại: **(i)** cấu trúc hợp lệ theo cách tính đã nêu (có thể là thật — cần Giang kiểm); **(ii)** ví dụ công khai; **(iii)** giả hiển nhiên; **(iv)** không phân loại được.
Cách tính checksum (i) là CRC32 + base62 đệm 6 ký tự, theo mô tả của phiên này; CHƯA đối chiếu tài liệu nhà cung cấp (không có mạng).

| tệp | dòng | loại bí mật | 4 ký tự đầu | phân loại | ghi chú |
|---|---:|---|---|---|---|
| crates/gt-secrets/tests/secrets.rs | 33 | aws | AKIA**** | (ii) | ví dụ công khai của nhà cung cấp |
| crates/gt-secrets/tests/secrets.rs | 50 | aws | AKIA**** | (ii) | ví dụ công khai của nhà cung cấp |
| crates/gt-secrets/tests/secrets.rs | 140 | aws | AKIA**** | (ii) | ví dụ công khai của nhà cung cấp |
| crates/gt-secrets/tests/secrets.rs | 173 | aws | AKIA**** | (ii) | ví dụ công khai của nhà cung cấp |
| crates/gt-secrets/tests/secrets.rs | 173 | aws | AKIA**** | (ii) | ví dụ công khai của nhà cung cấp |
| crates/gt-secrets/tests/secrets.rs | 178 | aws | AKIA**** | (ii) | ví dụ công khai của nhà cung cấp |
| crates/gt-secrets/tests/secrets.rs | 295 | aws | AKIA**** | (ii) | ví dụ công khai của nhà cung cấp |
| crates/gt-secrets/tests/secrets.rs | 410 | aws | AKIA**** | (ii) | ví dụ công khai của nhà cung cấp |
| tests/fail/E1401_allow_no_reason.gt | 4 | aws | AKIA**** | (ii) | ví dụ công khai của nhà cung cấp |
| tests/pass/S101_ok_allow_with_reason.gt | 4 | aws | AKIA**** | (ii) | ví dụ công khai của nhà cung cấp |
| tests/warn/S101_key_in_source.gt | 3 | aws | AKIA**** | (ii) | ví dụ công khai của nhà cung cấp |
| crates/gt-secrets/tests/secrets.rs | 34 | aws | ASIA**** | (iii) | dấu hiệu giả (từ khoá giả hoặc ký tự lặp) |
| crates/gt-secrets/tests/secrets.rs | 35 | github | ghp_**** | (iii) | checksum sai và có dấu hiệu giả |
| crates/gt-secrets/tests/secrets.rs | 37 | slack | xoxb**** | (iii) | dấu hiệu giả (từ khoá giả hoặc ký tự lặp) |
| tests/warn/S101_github_token.gt | 3 | github | ghp_**** | (iii) | checksum sai và có dấu hiệu giả |
| crates/gt-secrets/tests/secrets.rs | 36 | github | gith**** | (iv) | dạng github_pat_ (checksum chưa biết cách tính đã nêu) |
| crates/gt-secrets/tests/secrets.rs | 38 | google | AIza**** | (iv) | không phân loại được |
| crates/gt-secrets/tests/secrets.rs | 39 | other | sk_l**** | (iv) | không phân loại được |
| crates/gt-secrets/tests/secrets.rs | 40 | pem | ----**** | (iv) | không phân loại được |
| crates/gt-secrets/tests/secrets.rs | 263 | pem | ----**** | (iv) | không phân loại được |
| tests/warn/S101_private_key_block.gt | 3 | pem | ----**** | (iv) | không phân loại được |

Tổng: 21 mục — (i) 0, (ii) 11, (iii) 4, (iv) 6.
