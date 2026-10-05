//! Pattern.
use crate::parser::{PResult, Parser};
use gt_ast::*;
use gt_lexer::TokenKind;

impl Parser {
    fn mk_pat(&mut self, start: u32, kind: PatternKind) -> Pattern {
        let id = self.id();
        Pattern {
            id,
            span: self.span_from(start),
            kind,
        }
    }

    pub fn parse_pattern(&mut self) -> PResult<Pattern> {
        self.enter()?;
        let r = self.parse_pattern_inner(false);
        self.leave();
        r
    }

    fn starts_pat_lit(&self) -> bool {
        matches!(
            self.tok(),
            TokenKind::IntLit(_)
                | TokenKind::FloatLit(_)
                | TokenKind::CharLit(_)
                | TokenKind::Minus
        )
    }

    /// Literal trong pattern: `-`? số | ký tự | chuỗi | true | false.
    fn parse_pat_lit(&mut self, numeric_only: bool) -> PResult<Lit> {
        let start = self.start();
        let negative = self.eat(&TokenKind::Minus);
        if negative && !matches!(self.tok(), TokenKind::IntLit(_) | TokenKind::FloatLit(_)) {
            return Err(self.expected("số sau `-`"));
        }
        if numeric_only
            && !matches!(
                self.tok(),
                TokenKind::IntLit(_) | TokenKind::FloatLit(_) | TokenKind::CharLit(_)
            )
        {
            return Err(self.expected("số hoặc ký tự (cận của range)"));
        }
        let mut lit = self.parse_literal()?;
        lit.negative = negative;
        lit.span = self.span_from(start);
        Ok(lit)
    }

    fn parse_pattern_inner(&mut self, allow_rest: bool) -> PResult<Pattern> {
        let start = self.start();
        match self.tok() {
            TokenKind::Underscore => {
                self.bump();
                Ok(self.mk_pat(start, PatternKind::Wildcard))
            }
            TokenKind::DotDot => {
                self.bump();
                if self.starts_pat_lit() {
                    // `..b`
                    let end = self.parse_pat_lit(true)?;
                    return Ok(self.mk_pat(
                        start,
                        PatternKind::Range {
                            start: None,
                            end: Some(end),
                            inclusive: false,
                        },
                    ));
                }
                if allow_rest {
                    Ok(self.mk_pat(start, PatternKind::Rest))
                } else {
                    Err(self.expected(
                        "pattern (`..` đơn lẻ chỉ hợp lệ trong danh sách tuple/variant/struct)",
                    ))
                }
            }
            TokenKind::IntLit(_)
            | TokenKind::FloatLit(_)
            | TokenKind::CharLit(_)
            | TokenKind::StrLit(_)
            | TokenKind::KwTrue
            | TokenKind::KwFalse
            | TokenKind::Minus => {
                let first_is_range_capable = self.starts_pat_lit();
                let lit = self.parse_pat_lit(false)?;
                if first_is_range_capable && self.at(&TokenKind::DotDot) {
                    self.bump();
                    let end = if self.starts_pat_lit() {
                        Some(self.parse_pat_lit(true)?)
                    } else {
                        None
                    };
                    return Ok(self.mk_pat(
                        start,
                        PatternKind::Range {
                            start: Some(lit),
                            end,
                            inclusive: false,
                        },
                    ));
                }
                if first_is_range_capable && self.at(&TokenKind::DotDotEq) {
                    self.bump();
                    let end = self.parse_pat_lit(true)?;
                    return Ok(self.mk_pat(
                        start,
                        PatternKind::Range {
                            start: Some(lit),
                            end: Some(end),
                            inclusive: true,
                        },
                    ));
                }
                Ok(self.mk_pat(start, PatternKind::Lit(lit)))
            }
            TokenKind::Ident(_) | TokenKind::KwSelfTy => {
                let path = self.parse_path(false)?;
                if self.at(&TokenKind::LParen) {
                    self.bump();
                    let args = self.parse_pat_list(&TokenKind::RParen)?;
                    self.expect(&TokenKind::RParen, "`)`")?;
                    return Ok(self.mk_pat(start, PatternKind::Variant { path, args }));
                }
                if self.at(&TokenKind::LBrace) {
                    return self.parse_struct_pat(start, path);
                }
                if path.segments.len() == 1
                    && let SegmentKind::Name(n) = &path.segments[0].kind
                {
                    let ident = Ident {
                        id: path.segments[0].id,
                        span: path.segments[0].span,
                        name: n.clone(),
                    };
                    return Ok(self.mk_pat(start, PatternKind::Binding(ident)));
                }
                Ok(self.mk_pat(start, PatternKind::Variant { path, args: vec![] }))
            }
            TokenKind::LParen => {
                self.bump();
                if self.eat(&TokenKind::RParen) {
                    return Ok(self.mk_pat(start, PatternKind::Tuple(vec![])));
                }
                let first = self.parse_pattern_elem()?;
                if self.at(&TokenKind::RParen) {
                    self.bump();
                    if matches!(first.kind, PatternKind::Rest) {
                        return Ok(self.mk_pat(start, PatternKind::Tuple(vec![first])));
                    }
                    return Ok(first); // ngoặc nhóm
                }
                self.expect(&TokenKind::Comma, "`,` hoặc `)`")?;
                let mut elems = vec![first];
                let rest = self.parse_pat_list(&TokenKind::RParen)?;
                elems.extend(rest);
                Self::check_single_rest(self, &elems)?;
                self.expect(&TokenKind::RParen, "`)`")?;
                Ok(self.mk_pat(start, PatternKind::Tuple(elems)))
            }
            _ => Err(self.expected("pattern")),
        }
    }

    fn parse_pattern_elem(&mut self) -> PResult<Pattern> {
        self.enter()?;
        let r = self.parse_pattern_inner(true);
        self.leave();
        r
    }

    fn check_single_rest(&mut self, elems: &[Pattern]) -> PResult<()> {
        if elems
            .iter()
            .filter(|p| matches!(p.kind, PatternKind::Rest))
            .count()
            > 1
        {
            let span = elems[0].span;
            return Err(self.error_at(
                span,
                "chỉ được có một `..` trong danh sách pattern".to_string(),
            ));
        }
        Ok(())
    }

    /// Các phần tử pattern phân tách bằng `,` cho tới `close` (không tiêu thụ `close`); cho phép dấu phẩy cuối.
    fn parse_pat_list(&mut self, close: &TokenKind) -> PResult<Vec<Pattern>> {
        let mut elems = Vec::new();
        loop {
            if self.at(close) {
                break;
            }
            elems.push(self.parse_pattern_elem()?);
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.check_single_rest(&elems)?;
        Ok(elems)
    }

    fn parse_struct_pat(&mut self, start: u32, path: Path) -> PResult<Pattern> {
        self.bump(); // `{`
        let mut fields = Vec::new();
        let mut rest = false;
        loop {
            if self.at(&TokenKind::RBrace) {
                break;
            }
            if self.at(&TokenKind::DotDot) {
                self.bump();
                rest = true;
                break;
            }
            let fstart = self.start();
            let name = self.parse_ident("tên trường")?;
            let pattern = if self.eat(&TokenKind::Colon) {
                Some(self.parse_pattern()?)
            } else {
                None
            };
            let id = self.id();
            fields.push(FieldPat {
                id,
                span: self.span_from(fstart),
                name,
                pattern,
            });
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(&TokenKind::RBrace, "`}`")?;
        Ok(self.mk_pat(start, PatternKind::Struct { path, fields, rest }))
    }
}
