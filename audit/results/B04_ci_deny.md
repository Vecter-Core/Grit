### B04_ci_deny — CI (ci.yml) và deny.toml có tồn tại không

- **Trạng thái:** FAIL (mã thoát 1, 0s, bắt đầu 2026-10-20T11:19:15Z)
- dự án CHƯA có CI và CHƯA có deny.toml dù GRIT_PLAN §0.7 yêu cầu

<details><summary>Đuôi log (18 dòng cuối)</summary>

```
THIẾU .github/workflows/ci.yml (chỉ có TASK.md giữ chỗ)
THIẾU deny.toml (chỉ có deny.toml.TASK.md)
-- GRIT_PLAN §0.7 đòi: clippy -D warnings, cargo deny, cargo audit chạy trong CI
SUMMARY: dự án CHƯA có CI và CHƯA có deny.toml dù GRIT_PLAN §0.7 yêu cầu
```
</details>
