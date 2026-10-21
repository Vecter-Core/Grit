# GHI CHÚ MERGE với bản Windows của Giang

Bối cảnh: nhánh `claude/gallant-darwin-m251cq` (origin) được làm từ container Linux; bản Windows của Giang có thể đã tiến xa hơn (báo cáo gần nhất của bản đó: cổng 943 test, FB-15, E0313, `fn_value_gen`, oracle match 13 kiểu) và CHƯA lên origin (2026-10-21). Quy ước để merge sau dễ:
- Commit NHỎ; không đổi tên tệp/mã lỗi/tài liệu đã có.
- FB mới dùng tiền tố **FB-C**n (đã dùng FB-C1…FB-C8); ADR mới dùng tiền tố **ADR-C**nn; mã lỗi mới chỉ khi bắt buộc và ghi dưới đây ("có thể trùng số").
- Số liệu test của nhánh này KHÔNG so được với 943 của bản Windows (khác nền tảng, khác tập tệp).

## Mã lỗi mới của nhánh này
(chưa có — các sửa F-01…F-08 dùng lại E0401/E0301/E0310; ghi thêm ở đây nếu S2 buộc phải thêm)

## Tệp bị sửa nhiều (nơi dễ xung đột)
- `crates/gt-borrowck/src/check.rs` (`check_uses`: bản sao trạng thái + áp move; FB-C1)
- `crates/gt-types/src/check/call.rs` (`call`: CapUse qua giá trị hàm FB-C3; S1 sẽ sửa effects/⊳ của đường dẫn kết hợp)
- `crates/gt-types/src/collect.rs` (`sized_ty`, `contains_itself_by_value`, Local ở vị trí kiểu: FB-C2/C4)
- `crates/gt-llvm/build.rs`, `crates/gt-driver/{build.rs,src/native.rs,src/cli.rs}`, `crates/rig/build.rs`, `runtime/gt-rt/src/lib.rs` (port Linux, ngăn xếp 256 MiB)
- `tools/gate.sh` (viết lại: đa nền, điểm lưu theo đơn vị, giai đoạn nul/doc), thêm `tools/preflight.sh`, `tools/nul_scan.py`
- `docs/HANDOFF.md`, `docs/FIELD_BUGS.md`, `docs/KNOWN_LIMITATIONS.md`, `docs/DELETIONS.md` (nối thêm cuối tệp)
- Test đổi vì kiểu đệ quy theo giá trị nay bị E0301: `gt-types/tests/spawn.rs`, `gt-oblig/tests/pure_preds_interior_mut.rs`
- `docs/errors/{E0301,E0310,E0401}.md` (nối danh sách mẫu); E1201–E1204, E1301, E9901 là tệp mới
- `tests/golden/borrowck/diagnostics.txt` (snapshot, thêm 14 dòng)
