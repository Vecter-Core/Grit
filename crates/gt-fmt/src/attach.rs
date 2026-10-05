//! Cơ chế đặt comment TỔNG QUÁT theo token (ADR-016, Q16b) — thay cho các quy tắc theo từng vị trí.
//!
//! 1. Printer in văn bản CƠ SỞ (không có comment). Cả nguồn và văn bản cơ sở được lex lại.
//! 2. Căn hai luồng token. Printer chỉ có thể thêm/bỏ `(` `)` `,` `;`; mọi sai lệch khác là lỗi nội bộ (`PlaceError::Align`).
//! 3. Mỗi comment gắn vào token gần nhất: comment dòng riêng → token KẾ TIẾP; comment cuối dòng → token LIỀN TRƯỚC.
//!    Token nguồn đã bị bỏ (ngoặc thừa…) thì dịch sang token cơ sở khớp gần nhất theo cùng hướng.
//! 4. Chèn: comment dòng riêng được phát trên dòng riêng ngay trước token; comment cuối dòng được phát ngay sau token và
//!    ÉP XUỐNG DÒNG sau nó (khoảng trắng sau token được thay bằng xuống dòng thụt vào).
//! 5. Xác minh: lex đầu ra cho đúng luồng token cơ sở và đúng dãy comment nguồn; không thì `PlaceError::Verify`
//!    (E1301 — phương án cuối cùng; không có luật ngầm nào khác).
use gt_lexer::{Token, TokenKind};

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum PlaceError {
    /// Hai luồng token không khác nhau chỉ ở `( ) , ;` — lỗi nội bộ của printer.
    Align(String),
    /// Đầu ra sau khi chèn không còn là cùng chương trình/cùng dãy comment.
    Verify,
}

fn is_closer(k: &TokenKind) -> bool {
    matches!(
        k,
        TokenKind::RBrace | TokenKind::RParen | TokenKind::RBracket
    )
}

/// Token mà printer được phép bỏ khỏi nguồn / thêm vào văn bản cơ sở.
fn is_adjustable(k: &TokenKind) -> bool {
    matches!(
        k,
        TokenKind::LParen | TokenKind::RParen | TokenKind::Comma | TokenKind::Semi
    )
}

fn without_eof(tokens: Vec<Token>) -> Vec<Token> {
    tokens
        .into_iter()
        .filter(|t| t.kind != TokenKind::Eof)
        .collect()
}

/// `map[i] = Some(j)`: token nguồn `i` ứng với token cơ sở `j`.
fn align(src: &[Token], base: &[Token]) -> Result<Vec<Option<usize>>, PlaceError> {
    let mut map = vec![None; src.len()];
    let (mut i, mut j) = (0, 0);
    while i < src.len() || j < base.len() {
        match (src.get(i), base.get(j)) {
            (Some(s), Some(b)) if s.kind == b.kind => {
                map[i] = Some(j);
                i += 1;
                j += 1;
            }
            (Some(s), _) if is_adjustable(&s.kind) => i += 1,
            (_, Some(b)) if is_adjustable(&b.kind) => j += 1,
            (s, b) => {
                return Err(PlaceError::Align(format!(
                    "token nguồn {:?} ≠ token cơ sở {:?}",
                    s.map(|t| &t.kind),
                    b.map(|t| &t.kind)
                )));
            }
        }
    }
    Ok(map)
}

struct Anchors {
    /// comment dòng riêng đứng trước token cơ sở `j`
    before: Vec<Vec<String>>,
    /// comment cuối dòng đứng sau token cơ sở `j`
    after: Vec<Vec<String>>,
    /// comment dòng riêng sau token cuối cùng
    eof: Vec<String>,
}

fn anchor(
    src_tokens: &[Token],
    base: &[Token],
    map: &[Option<usize>],
    trivia: &[gt_lexer::Trivia],
) -> Anchors {
    let base_len = base.len();
    let mut reverse = vec![false; base_len];
    for j in map.iter().flatten() {
        reverse[*j] = true;
    }
    let mut a = Anchors {
        before: vec![Vec::new(); base_len],
        after: vec![Vec::new(); base_len],
        eof: Vec::new(),
    };
    for c in trivia {
        let next = src_tokens.partition_point(|t| t.span.start < c.span.end);
        let prev = src_tokens.partition_point(|t| t.span.end <= c.span.start);
        if c.own_line {
            // token cơ sở khớp đầu tiên không nhỏ hơn `next`
            match map[next.min(map.len())..].iter().flatten().next() {
                Some(&j) => {
                    // đi lùi qua `(` do printer thêm vào ngay trước token đó: comment đứng trước ngoặc
                    let mut j = j;
                    while j > 0 && !reverse[j - 1] && base[j - 1].kind == TokenKind::LParen {
                        j -= 1;
                    }
                    a.before[j].push(c.text.clone());
                }
                None => a.eof.push(c.text.clone()),
            }
        } else {
            // token cơ sở khớp cuối cùng không lớn hơn `prev - 1`
            match map[..prev].iter().rev().flatten().next() {
                Some(&j) => {
                    let mut j = j;
                    // bỏ qua `,` `;` do printer thêm vào ngay sau token đó: comment đứng sau dấu phân tách
                    while j + 1 < base_len
                        && !reverse[j + 1]
                        && matches!(base[j + 1].kind, TokenKind::Comma | TokenKind::Semi)
                    {
                        j += 1;
                    }
                    a.after[j].push(c.text.clone());
                }
                None => a.before[0].push(c.text.clone()),
            }
        }
    }
    a
}

/// Thụt lề (chuỗi khoảng trắng đầu) của dòng cuối cùng trong `out`.
fn line_indent(out: &str) -> String {
    let line = out.rsplit('\n').next().unwrap_or("");
    line.chars().take_while(|c| *c == ' ').collect()
}

/// Đặt comment của `src` vào văn bản cơ sở `base`.
pub(crate) fn place(src: &str, base: &str) -> Result<String, PlaceError> {
    let lexed = gt_lexer::lex(src);
    let src_tokens = without_eof(lexed.tokens);
    let trivia = lexed.trivia;
    let base_tokens = without_eof(gt_lexer::lex(base).tokens);
    let map = align(&src_tokens, &base_tokens)?;
    let a = anchor(&src_tokens, &base_tokens, &map, &trivia);

    let mut out = String::new();
    let mut pos = 0usize;
    let mut trail_pending = false;
    for (j, tok) in base_tokens.iter().enumerate() {
        let (start, end) = (tok.span.start as usize, tok.span.end as usize);
        let gap = &base[pos..start];
        let leading = &a.before[j];
        let closer = is_closer(&tok.kind);
        if j == 0 {
            for c in leading {
                out.push_str(c);
                out.push('\n');
            }
            out.push_str(gap);
        } else if !gap.contains('\n') && (trail_pending || !leading.is_empty()) {
            // token đang ở giữa dòng nhưng phải xuống dòng (sau comment cuối dòng, hoặc trước comment dòng riêng)
            let base_ind = line_indent(&out);
            let cind = format!("{base_ind}  ");
            out.push('\n');
            for c in leading {
                out.push_str(&cind);
                out.push_str(c);
                out.push('\n');
            }
            out.push_str(if closer { &base_ind } else { &cind });
        } else if let Some(nl) = gap.rfind('\n') {
            let (pre, ind) = gap.split_at(nl + 1);
            out.push_str(pre);
            for c in leading {
                if closer {
                    out.push_str(ind);
                    out.push_str("  ");
                } else {
                    out.push_str(ind);
                }
                out.push_str(c);
                out.push('\n');
            }
            out.push_str(ind);
        } else {
            out.push_str(gap);
        }
        out.push_str(&base[start..end]);
        for c in &a.after[j] {
            out.push_str("  ");
            out.push_str(c);
        }
        trail_pending = !a.after[j].is_empty();
        pos = end;
    }
    out.push_str(&base[pos..]);
    if !a.eof.is_empty() {
        for c in &a.eof {
            out.push_str(c);
            out.push('\n');
        }
    }
    verify(src, &base_tokens, &out)?;
    Ok(out)
}

/// Xác minh: đầu ra lex ra đúng luồng token cơ sở và đúng dãy comment của nguồn.
fn verify(src: &str, base_tokens: &[Token], out: &str) -> Result<(), PlaceError> {
    let lexed = gt_lexer::lex(out);
    let out_tokens = without_eof(lexed.tokens);
    let same_tokens = out_tokens.len() == base_tokens.len()
        && out_tokens
            .iter()
            .zip(base_tokens)
            .all(|(a, b)| a.kind == b.kind);
    let texts = |t: &[gt_lexer::Trivia]| t.iter().map(|c| c.text.clone()).collect::<Vec<_>>();
    if same_tokens && texts(&lexed.trivia) == texts(&gt_lexer::lex(src).trivia) {
        Ok(())
    } else {
        Err(PlaceError::Verify)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(s: &str) -> Vec<Token> {
        without_eof(gt_lexer::lex(s).tokens)
    }

    #[test]
    fn align_accepts_only_paren_comma_semicolon_differences() {
        let m = align(&toks("((a)) , b ;"), &toks("a b")).expect("căn");
        // a ↔ a, b ↔ b; `(` `(` `)` `)` `,` `;` bị bỏ
        assert_eq!(m, [None, None, Some(0), None, None, None, Some(1), None]);
        let m = align(&toks("a b"), &toks("(a) , b;")).expect("căn");
        assert_eq!(m, [Some(1), Some(4)]);
        // khác ở token khác: lỗi nội bộ
        assert!(matches!(
            align(&toks("a + b"), &toks("a - b")),
            Err(PlaceError::Align(_))
        ));
        assert!(matches!(
            align(&toks("a b"), &toks("a")),
            Err(PlaceError::Align(_))
        ));
    }

    #[test]
    fn verify_rejects_changed_tokens_or_changed_comment_sequence() {
        let src = "a // x\n";
        let base = toks("a\n");
        assert_eq!(verify(src, &base, "a // x\n"), Ok(()));
        assert_eq!(verify(src, &base, "a\n"), Err(PlaceError::Verify)); // mất comment
        assert_eq!(verify(src, &base, "a // y\n"), Err(PlaceError::Verify)); // đổi nội dung
        assert_eq!(verify(src, &base, "b // x\n"), Err(PlaceError::Verify)); // đổi token
        assert_eq!(verify(src, &base, "a a // x\n"), Err(PlaceError::Verify)); // thêm token
        assert_eq!(
            verify(src, &base, "a // x\n// x\n"),
            Err(PlaceError::Verify)
        ); // thêm comment
    }

    #[test]
    fn comments_anchored_to_removed_tokens_move_to_the_nearest_kept_token() {
        // cuối dòng sau `a`; `)` nguồn bị bỏ sau đó
        assert_eq!(
            place(
                "(a // c
)",
                "a
"
            )
            .unwrap(),
            "a  // c
"
        );
        // cuối dòng sau `(` bị bỏ, không có token giữ lại phía trước → thành comment dòng riêng trước token đầu
        assert_eq!(
            place(
                "( // c
a)",
                "a
"
            )
            .unwrap(),
            "// c
a
"
        );
        // dòng riêng trước `)` bị bỏ → gắn vào token giữ lại kế tiếp (`+`)
        assert_eq!(
            place(
                "(a
// c
) + b",
                "a + b
"
            )
            .unwrap(),
            "a
  // c
  + b
"
        );
    }

    #[test]
    fn trailing_comment_skips_separators_added_by_the_printer() {
        // nguồn không có `,` cuối; cơ sở thêm `,` → comment đứng sau dấu phẩy
        assert_eq!(
            place("▢ P{x:i32 // c\n}", "▢ P {\n  x: i32,\n}\n").unwrap(),
            "▢ P {\n  x: i32,  // c\n}\n"
        );
    }

    #[test]
    fn own_line_comment_goes_before_a_paren_added_by_the_printer() {
        // nguồn `a * // c ⏎ b` (comment dòng riêng trước `b`); cơ sở thêm `(` quanh `b` → comment đứng trước `(`
        assert_eq!(
            place(
                "a *
// c
b",
                "a * (b)
"
            )
            .unwrap(),
            "a *
  // c
  (b)
"
        );
    }
}
