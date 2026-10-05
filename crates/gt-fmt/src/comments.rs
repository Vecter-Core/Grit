//! Bộ máy gắn comment: lấy comment theo vị trí, đánh dấu đã dùng, và từ chối (E1301) cái không gắn được.
use gt_diag::{Code, Diagnostic};
use gt_lexer::Trivia;
use gt_span::Span;

pub(crate) struct Comment {
    pub span: Span,
    pub text: String,
    pub used: bool,
}

pub(crate) struct Comments<'a> {
    src: &'a str,
    file: String,
    items: Vec<Comment>,
    pub diags: Vec<Diagnostic>,
}

impl<'a> Comments<'a> {
    pub fn new(file: &str, src: &'a str, trivia: Vec<Trivia>) -> Comments<'a> {
        let items = trivia
            .into_iter()
            .map(|t| Comment {
                span: t.span,
                text: t.text,
                used: false,
            })
            .collect();
        Comments {
            src,
            file: file.to_string(),
            items,
            diags: Vec::new(),
        }
    }

    fn between(&self, a: u32, b: u32) -> &str {
        self.src.get(a as usize..b as usize).unwrap_or("")
    }

    fn refuse(&mut self, idx: usize) {
        let c = &mut self.items[idx];
        if c.used {
            return;
        }
        c.used = true;
        let span = c.span;
        if let Ok(code) = Code::parse("E1301") {
            self.diags.push(
                Diagnostic::new(
                    code,
                    &self.file,
                    span,
                    "gt-fmt: comment không gắn được vào đâu (nằm giữa biểu thức, chữ ký hoặc node)".to_string(),
                )
                .with_span_label("comment này")
                .with_note("đưa comment ra một dòng riêng giữa các câu lệnh/item/trường/nhánh, hoặc để cuối dòng sau câu lệnh"),
            );
        }
    }

    /// Có comment chưa dùng với `lo <= start < hi`.
    pub fn has_unused_in(&self, lo: u32, hi: u32) -> bool {
        self.items
            .iter()
            .any(|c| !c.used && c.span.start >= lo && c.span.start < hi)
    }

    /// Mọi comment chưa dùng nằm TRONG `[lo, hi)` mà không ai gắn được → từ chối từng cái.
    pub fn refuse_inside(&mut self, lo: u32, hi: u32) {
        for i in 0..self.items.len() {
            let c = &self.items[i];
            if !c.used && c.span.start >= lo && c.span.start < hi {
                self.refuse(i);
            }
        }
    }

    /// Comment cuối dòng ngay sau một node: cùng dòng, giữa node và comment chỉ có khoảng trắng, `,` hoặc `;`.
    pub fn take_trailing(&mut self, node_end: u32) -> Option<String> {
        for i in 0..self.items.len() {
            let c = &self.items[i];
            if c.used || c.span.start < node_end {
                continue;
            }
            let gap = self.between(node_end, c.span.start);
            if gap.chars().all(|ch| matches!(ch, ' ' | '\t' | ',' | ';')) {
                self.items[i].used = true;
                return Some(self.items[i].text.clone());
            }
            if gap.contains('\n') || c.span.start > node_end {
                // các comment đã sắp theo vị trí: gặp comment đầu tiên không khớp là dừng
                break;
            }
        }
        None
    }

    /// Comment ngay sau dấu `{` mở trên cùng dòng, trước `limit`.
    pub fn take_after_open(&mut self, open: u32, limit: u32) -> Option<String> {
        for i in 0..self.items.len() {
            let c = &self.items[i];
            if c.used || c.span.start <= open {
                continue;
            }
            if c.span.start >= limit {
                break;
            }
            let gap = self.between(open + 1, c.span.start);
            if gap.chars().all(|ch| matches!(ch, ' ' | '\t')) {
                self.items[i].used = true;
                return Some(self.items[i].text.clone());
            }
            break;
        }
        None
    }

    /// Comment đứng một mình trên dòng, trước node (và sau `region_start`). Comment cùng dòng với `prev_end` mà không phải
    /// "cuối dòng" hợp lệ thì không gắn được → từ chối.
    pub fn take_before(
        &mut self,
        node_start: u32,
        region_start: u32,
        prev_end: Option<u32>,
    ) -> Vec<String> {
        let mut out = Vec::new();
        for i in 0..self.items.len() {
            let (used, start) = (self.items[i].used, self.items[i].span.start);
            if used || start < region_start || start >= node_start {
                continue;
            }
            if let Some(pe) = prev_end
                && start >= pe
                && !self.between(pe, start).contains('\n')
            {
                self.refuse(i);
                continue;
            }
            self.items[i].used = true;
            out.push(self.items[i].text.clone());
        }
        out
    }

    /// Comment còn lại trong `[lo, hi)` (dùng cho comment cuối danh sách).
    pub fn dangling(&mut self, lo: u32, hi: u32) -> Vec<String> {
        let mut out = Vec::new();
        for c in &mut self.items {
            if !c.used && c.span.start >= lo && c.span.start < hi {
                c.used = true;
                out.push(c.text.clone());
            }
        }
        out
    }

    /// Kết thúc: comment nào còn lại là không gắn được.
    pub fn finish(&mut self) -> Vec<Diagnostic> {
        for i in 0..self.items.len() {
            if !self.items[i].used {
                self.refuse(i);
            }
        }
        std::mem::take(&mut self.diags)
    }
}

#[cfg(test)]
mod tests {
    //! Kiểm trực tiếp bộ máy: các nhánh "lưới an toàn" (`finish`, chốt chống báo lỗi hai lần) và biên của các khoảng
    //! `[lo, hi)` không thể chạm tới từ API công khai (mọi comment thật đều đã bị `refuse_inside`/`dangling` xử lý trước).
    use super::*;

    /// `// a` bắt đầu ở 0, `// b` ở 5, `// c` ở 10.
    const SRC: &str = "// a\n// b\n// c\n";

    fn engine() -> Comments<'static> {
        Comments::new("t.gt", SRC, gt_lexer::lex(SRC).trivia)
    }

    fn spans(ds: &[Diagnostic]) -> Vec<(u32, u32)> {
        ds.iter().map(|d| (d.span.start, d.span.end)).collect()
    }

    #[test]
    fn finish_refuses_every_leftover_exactly_once_in_order() {
        let mut c = engine();
        assert_eq!(c.dangling(5, 10), ["// b"]);
        let ds = c.finish();
        assert_eq!(spans(&ds), [(0, 4), (10, 14)]);
        assert!(ds.iter().all(|d| d.code.as_str() == "E1301"));
        // sau finish không còn gì để từ chối
        assert!(c.finish().is_empty());
    }

    #[test]
    fn finish_with_nothing_left_is_silent() {
        let mut c = engine();
        assert_eq!(c.dangling(0, 100).len(), 3);
        assert!(c.finish().is_empty());
    }

    #[test]
    fn a_comment_is_refused_at_most_once() {
        let mut c = engine();
        c.refuse(1);
        c.refuse(1);
        assert_eq!(spans(&c.diags), [(5, 9)]);
        // đã bị từ chối thì các hàm khác không lấy lại
        assert!(c.dangling(0, 100).len() == 2);
    }

    #[test]
    fn ranges_are_half_open_lo_inclusive_hi_exclusive() {
        let c = engine();
        assert!(!c.has_unused_in(5, 5));
        assert!(c.has_unused_in(5, 6));
        assert!(!c.has_unused_in(1, 5)); // `// b` bắt đầu đúng ở hi=5 → ngoài
        assert!(c.has_unused_in(0, 1)); // `// a` bắt đầu đúng ở lo=0 → trong

        let mut c = engine();
        c.refuse_inside(0, 5);
        assert_eq!(spans(&c.diags), [(0, 4)]); // `// b` ở đúng hi → không bị từ chối
        let mut c = engine();
        c.refuse_inside(5, 6);
        assert_eq!(spans(&c.diags), [(5, 9)]); // lo bao gồm

        let mut c = engine();
        assert_eq!(c.dangling(0, 5), ["// a"]); // hi loại trừ
        assert_eq!(c.dangling(5, 10), ["// b"]); // lo bao gồm
    }

    #[test]
    fn trailing_accepts_only_same_line_gap_of_space_tab_comma_semicolon() {
        let src = "x ,;\t// t\ny\n// n\n";
        let mk = || Comments::new("t.gt", src, gt_lexer::lex(src).trivia);
        assert_eq!(mk().take_trailing(1).as_deref(), Some("// t"));
        assert_eq!(mk().take_trailing(4).as_deref(), Some("// t")); // sát comment (khoảng cách chỉ là tab)
        // `y` rồi xuống dòng: comment ở dòng sau KHÔNG phải comment cuối dòng của `y`
        assert_eq!(mk().take_trailing(11), None);
        // khoảng cách chứa ký tự khác → không nhận
        let src2 = "x + // t\n";
        let mut c2 = Comments::new("t.gt", src2, gt_lexer::lex(src2).trivia);
        assert_eq!(c2.take_trailing(1), None);
    }

    #[test]
    fn after_open_needs_same_line_blank_gap_and_respects_limit() {
        let src = "{ // t\n";
        let mk = || Comments::new("t.gt", src, gt_lexer::lex(src).trivia);
        assert_eq!(mk().take_after_open(0, 100).as_deref(), Some("// t"));
        assert_eq!(mk().take_after_open(0, 2), None); // comment bắt đầu ở 2 = limit → ngoài
        let src2 = "{ x // t\n";
        let mut c2 = Comments::new("t.gt", src2, gt_lexer::lex(src2).trivia);
        assert_eq!(c2.take_after_open(0, 100), None); // có mã chen giữa
        let src3 = "{\n// t\n";
        let mut c3 = Comments::new("t.gt", src3, gt_lexer::lex(src3).trivia);
        assert_eq!(c3.take_after_open(0, 100), None); // dòng khác: không phải "sau {"
    }

    #[test]
    fn take_before_refuses_a_comment_on_the_same_line_as_the_previous_node() {
        let src = "a // x\nb\n";
        let mut c = Comments::new("t.gt", src, gt_lexer::lex(src).trivia);
        // `a` kết thúc ở 1, `b` bắt đầu ở 7; `// x` cùng dòng với `a` nhưng gọi như thể không phải trailing hợp lệ
        let got = c.take_before(7, 0, Some(1));
        assert!(got.is_empty());
        assert_eq!(spans(&c.diags), [(2, 6)]);
        // biên: comment bắt đầu ĐÚNG ở prev_end (dính liền node trước) cũng là cùng dòng → từ chối
        let src = "a// x\nb\n";
        let mut c = Comments::new("t.gt", src, gt_lexer::lex(src).trivia);
        assert!(c.take_before(6, 0, Some(1)).is_empty());
        assert_eq!(spans(&c.diags), [(1, 5)]);
        // nhưng nếu comment nằm dòng riêng sau prev_end thì được lấy
        let src = "a\n// x\nb\n";
        let mut c = Comments::new("t.gt", src, gt_lexer::lex(src).trivia);
        assert_eq!(c.take_before(7, 0, Some(1)), ["// x"]);
        assert!(c.diags.is_empty());
    }
}
