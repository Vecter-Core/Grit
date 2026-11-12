# Kiểm kê tài liệu và giao diện (G2.4)

Kiểm ngày 2026-11-12 trên nhánh `integration`; đây là kiểm kê tồn tại tệp/mã, chưa tuyên bố chất lượng nội dung.

| Mục | Trạng thái | Bằng chứng |
|---|---|---|
| QUICKSTART | CÓ | `docs/QUICKSTART.md` |
| QUICKSTART cloud | CÓ | `docs/QUICKSTART_CLOUD.md` |
| GRIT_BOOK | CÓ | `docs/GRIT_BOOK.md` |
| LANGUAGE_REFERENCE | CÓ | `docs/LANGUAGE_REFERENCE.md` |
| STD_REFERENCE sinh tự động | CÓ | `docs/STD_REFERENCE.md`; generator `tools/gen_docs.py`; test khóa byte trong `crates/gt-driver/tests/docs_lock.rs` |
| AI_GUIDE | CÓ | `docs/AI_GUIDE.md` |
| SAFETY_CLAIMS | CÓ | `docs/SAFETY_CLAIMS.md` |
| `tools/doc_test.py` | CÓ | script và tests `tools/test_doc_test.py` |
| `tools/setup_cloud.sh` | CÓ | script tồn tại |
| `gritc doctor` | CÓ | `crates/gt-driver/src/cli.rs`, unit/integration tests trong `crates/gt-driver/tests/cli.rs` |
| `gritc lsp` | CÓ | `crates/gt-driver/src/lsp.rs`, `crates/gt-driver/tests/lsp.rs` |

Không phát hiện mục THIẾU trong danh sách G2.4; theo chỉ thị G2.6 chỉ tạo nội dung còn thiếu, nên G2.4 không tạo tài liệu bổ sung.
