//! ADR-017a: `parse_file` gắn FileId vào mọi NodeId; `parse` mặc định FileId 0. Viết TRƯỚC (TDD).
use gt_span::FileId;

const SRC: &str = "ƒ f(a: i32) → i32 ! ∅ { a + 1 }\n▢ P { x: i32 }\n";

/// Gom mọi NodeId có trong Debug của cây (chuỗi `NodeId { file: FileId(f), index: i }`).
fn ids(dbg: &str) -> Vec<(u32, u32)> {
    let mut out = Vec::new();
    let mut rest = dbg;
    while let Some(i) = rest.find("NodeId {") {
        let tail = &rest[i..];
        let nums: Vec<u32> = tail
            .split(|c: char| !c.is_ascii_digit())
            .filter(|t| !t.is_empty())
            .take(2)
            .map(|t| t.parse().expect("số"))
            .collect();
        out.push((nums[0], nums[1]));
        rest = &tail[8..];
    }
    out
}

#[test]
fn parse_file_stamps_the_file_id_on_every_node() {
    let out = gt_parser::parse_file(FileId::new(5), "x.gt", SRC);
    assert!(out.report.diagnostics.is_empty());
    let all = ids(&format!("{:?}", out.program));
    assert!(!all.is_empty());
    assert!(
        all.iter().all(|(f, _)| *f == 5),
        "mọi node phải mang FileId 5"
    );
    // chỉ số duy nhất trong tệp và nằm trong [0, node_count) (parser cấp id dày từ 0; một số id không ở lại trong cây)
    let mut idx: Vec<u32> = all.iter().map(|(_, i)| *i).collect();
    idx.sort_unstable();
    let n = idx.len();
    idx.dedup();
    assert_eq!(idx.len(), n, "NodeId phải duy nhất trong tệp");
    assert!(idx.iter().all(|&i| i < out.node_count));
}

#[test]
fn parse_defaults_to_file_id_zero_and_is_the_same_as_parse_file_zero() {
    let a = gt_parser::parse("x.gt", SRC);
    let b = gt_parser::parse_file(FileId::new(0), "x.gt", SRC);
    assert_eq!(format!("{:?}", a.program), format!("{:?}", b.program));
    assert!(
        ids(&format!("{:?}", a.program))
            .iter()
            .all(|(f, _)| *f == 0)
    );
}

#[test]
fn two_files_give_disjoint_node_id_sets() {
    let a = gt_parser::parse_file(FileId::new(0), "a.gt", SRC);
    let b = gt_parser::parse_file(FileId::new(1), "b.gt", SRC);
    let sa: std::collections::HashSet<_> = ids(&format!("{:?}", a.program)).into_iter().collect();
    let sb: std::collections::HashSet<_> = ids(&format!("{:?}", b.program)).into_iter().collect();
    assert!(sa.is_disjoint(&sb));
    assert_eq!(sa.len(), sb.len());
}

#[test]
fn ice_output_also_carries_the_file_id() {
    // nhánh ICE dùng chung bộ cấp NodeId: kiểm qua parse_file với tệp rỗng (cây tối thiểu vẫn mang FileId)
    let out = gt_parser::parse_file(FileId::new(2), "e.gt", "");
    let all = ids(&format!("{:?}", out.program));
    assert!(!all.is_empty());
    assert!(all.iter().all(|(f, _)| *f == 2));
}
