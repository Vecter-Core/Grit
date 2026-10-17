//! Chỉ cho test đơn vị trong crate: dựng `Env` thật (resolve + index + collect) từ một nguồn nhỏ, để gọi các hàm riêng tư của gt-types với DỮ LIỆU
//! BỊ SỬA trong bộ nhớ (FAIL_OPEN_AUDIT: nhánh "thiếu dữ liệu" mà chương trình Grit hợp lệ không bao giờ tạo ra).
use crate::collect::Env;
use gt_resolve::{Prelude, SourceInput, resolve_program};

pub(crate) fn with_env<R>(src: &str, f: impl FnOnce(&mut Env<'_>) -> R) -> R {
    let prelude =
        Prelude::from_source(include_str!("../../../lib/core/prelude.gt")).expect("prelude");
    let inputs = [SourceInput {
        name: "main.gt".to_string(),
        text: src.to_string(),
    }];
    let res = resolve_program(&inputs, &prelude).expect("đầu vào");
    assert!(res.report.diagnostics.is_empty(), "resolve báo lỗi");
    let mut env = Env::new(&res, vec!["main.gt".to_string()]);
    env.index();
    env.collect();
    f(&mut env)
}
