//! Biểu thức (Pratt), khối, câu lệnh, vòng lặp, match, unsafe.
use crate::parser::{Abort, PResult, Parser};
use gt_ast::*;
use gt_lexer::TokenKind;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Assoc {
    Left,
    Right,
    Neither,
}

/// (toán tử, độ ưu tiên, kết hợp). Độ ưu tiên: ⇒ 1 < ∨ 2 < ∧ 3 < so sánh 4 < cộng 5 < nhân 6.
fn binop_info(k: &TokenKind) -> Option<(BinaryOp, u8, Assoc)> {
    use Assoc::*;
    use BinaryOp as B;
    Some(match k {
        TokenKind::Implies => (B::Implies, 1, Right),
        TokenKind::Lor => (B::Or, 2, Left),
        TokenKind::Land => (B::And, 3, Left),
        TokenKind::EqEq => (B::Eq, 4, Neither),
        TokenKind::Neq => (B::Ne, 4, Neither),
        TokenKind::Lt => (B::Lt, 4, Neither),
        TokenKind::Gt => (B::Gt, 4, Neither),
        TokenKind::Leq => (B::Le, 4, Neither),
        TokenKind::Geq => (B::Ge, 4, Neither),
        TokenKind::InSet => (B::InSet, 4, Neither),
        TokenKind::Plus => (B::Add, 5, Left),
        TokenKind::Minus => (B::Sub, 5, Left),
        TokenKind::PlusWrap => (B::AddWrap, 5, Left),
        TokenKind::MinusWrap => (B::SubWrap, 5, Left),
        TokenKind::PlusSat => (B::AddSat, 5, Left),
        TokenKind::MinusSat => (B::SubSat, 5, Left),
        TokenKind::PlusChk => (B::AddChecked, 5, Left),
        TokenKind::MinusChk => (B::SubChecked, 5, Left),
        TokenKind::Star => (B::Mul, 6, Left),
        TokenKind::Slash => (B::Div, 6, Left),
        TokenKind::Percent => (B::Rem, 6, Left),
        TokenKind::StarWrap => (B::MulWrap, 6, Left),
        TokenKind::StarSat => (B::MulSat, 6, Left),
        TokenKind::StarChk => (B::MulChecked, 6, Left),
        _ => return None,
    })
}

/// Kết quả phân tích một câu lệnh trong khối.
enum StmtOrTail {
    Stmt(Box<Stmt>),
    Tail(Box<Expr>),
}

impl Parser {
    fn mk_expr(&mut self, start: u32, kind: ExprKind) -> Expr {
        let id = self.id();
        Expr {
            id,
            span: self.span_from(start),
            kind,
        }
    }

    // ---------- điểm vào ----------

    /// Biểu thức đầy đủ (`expr` trong grammar): khối-biểu-thức hoặc range.
    pub fn parse_expr(&mut self) -> PResult<Expr> {
        self.enter()?;
        let r = match self.tok() {
            TokenKind::KwIf => self.parse_if(),
            TokenKind::KwMatch => self.parse_match(),
            TokenKind::LoopSym => self.parse_loop(),
            TokenKind::KwUnsafe => self.parse_unsafe(),
            _ => self.parse_range(),
        };
        self.leave();
        r
    }

    /// `expr_ns`: cấm struct literal trần ở mức ngoài cùng (Q15a).
    pub fn parse_expr_ns(&mut self) -> PResult<Expr> {
        let saved = std::mem::replace(&mut self.no_struct, true);
        let r = self.parse_expr();
        self.no_struct = saved;
        r
    }

    /// Biểu thức trong ngoặc/ngoặc vuông/đối số/khối: đặt lại luật `expr_ns`.
    fn parse_expr_nested(&mut self) -> PResult<Expr> {
        let saved = std::mem::replace(&mut self.no_struct, false);
        let r = self.parse_expr();
        self.no_struct = saved;
        r
    }

    fn starts_operand(&self) -> bool {
        match self.tok() {
            TokenKind::Ident(_)
            | TokenKind::IntLit(_)
            | TokenKind::FloatLit(_)
            | TokenKind::StrLit(_)
            | TokenKind::CharLit(_)
            | TokenKind::KwTrue
            | TokenKind::KwFalse
            | TokenKind::KwSelfKw
            | TokenKind::KwSelfTy
            | TokenKind::LParen
            | TokenKind::LBracket
            | TokenKind::Lnot
            | TokenKind::Minus
            | TokenKind::Star
            | TokenKind::Borrow
            | TokenKind::BorrowMut
            | TokenKind::ForAll
            | TokenKind::Exists => true,
            TokenKind::LBrace => !self.no_struct,
            _ => false,
        }
    }

    // ---------- range / nhị phân / cast / đơn nguyên / postfix ----------

    fn parse_range(&mut self) -> PResult<Expr> {
        let start = self.start();
        if self.at(&TokenKind::DotDot) {
            self.bump();
            let end = if self.starts_operand() {
                Some(Box::new(self.parse_bin(0)?))
            } else {
                None
            };
            return Ok(self.mk_expr(
                start,
                ExprKind::Range {
                    start: None,
                    end,
                    inclusive: false,
                },
            ));
        }
        if self.at(&TokenKind::DotDotEq) {
            return Err(self.expected("biểu thức (`..=` cần có cận đầu)"));
        }
        let lhs = self.parse_bin(0)?;
        match self.tok() {
            TokenKind::DotDot => {
                self.bump();
                let end = if self.starts_operand() {
                    Some(Box::new(self.parse_bin(0)?))
                } else {
                    None
                };
                Ok(self.mk_expr(
                    start,
                    ExprKind::Range {
                        start: Some(Box::new(lhs)),
                        end,
                        inclusive: false,
                    },
                ))
            }
            TokenKind::DotDotEq => {
                self.bump();
                let end = Box::new(self.parse_bin(0)?);
                Ok(self.mk_expr(
                    start,
                    ExprKind::Range {
                        start: Some(Box::new(lhs)),
                        end: Some(end),
                        inclusive: true,
                    },
                ))
            }
            _ => Ok(lhs),
        }
    }

    fn parse_bin(&mut self, min_bp: u8) -> PResult<Expr> {
        let start = self.start();
        let mut lhs = self.parse_cast()?;
        let mut chain = 0u32;
        loop {
            self.work += 1;
            let Some((op, bp, assoc)) = binop_info(self.tok()) else {
                break;
            };
            if bp < min_bp {
                break;
            }
            chain += 1;
            self.chain_check(chain)?;
            self.bump();
            let rhs = if assoc == Assoc::Right {
                self.enter()?;
                let r = self.parse_bin(bp);
                self.leave();
                r?
            } else {
                self.parse_bin(bp + 1)?
            };
            lhs = self.mk_expr(
                start,
                ExprKind::Binary {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
            );
            if assoc == Assoc::Neither
                && let Some((_, bp2, Assoc::Neither)) = binop_info(self.tok())
                && bp2 == bp
            {
                let span = self.span_here();
                return Err(self.error_at(
                    span,
                    "so sánh không kết hợp (`a < b < c`): dùng `∧` hoặc ngoặc".to_string(),
                ));
            }
        }
        Ok(lhs)
    }

    fn parse_cast(&mut self) -> PResult<Expr> {
        let start = self.start();
        let mut e = self.parse_unary()?;
        let mut chain = 0u32;
        while self.at(&TokenKind::KwAs) {
            chain += 1;
            self.chain_check(chain)?;
            self.bump();
            let snap = self.snapshot();
            let ty = match self.parse_type() {
                Ok(ty) => ty,
                Err(abort) => {
                    // `x as T < y`: quy tắc ngôn ngữ (Q16c) → một chẩn đoán riêng kèm gợi ý bọc ngoặc
                    self.explain_cast_lt(&snap, start);
                    return Err(abort);
                }
            };
            e = self.mk_expr(
                start,
                ExprKind::Cast {
                    expr: Box::new(e),
                    ty,
                },
            );
        }
        Ok(e)
    }

    fn parse_unary(&mut self) -> PResult<Expr> {
        let start = self.start();
        let op = match self.tok() {
            TokenKind::Lnot => UnaryOp::Not,
            TokenKind::Minus => UnaryOp::Neg,
            TokenKind::Star => UnaryOp::Deref,
            TokenKind::Borrow => UnaryOp::Borrow,
            TokenKind::BorrowMut => UnaryOp::BorrowMut,
            _ => return self.parse_postfix(),
        };
        // `-` ngay trước literal số (không có postfix theo sau) được gấp vào literal (ADR-014)
        if op == UnaryOp::Neg
            && matches!(
                self.peek_kind(1),
                TokenKind::IntLit(_) | TokenKind::FloatLit(_)
            )
            && !matches!(
                self.peek_kind(2),
                TokenKind::Dot | TokenKind::LParen | TokenKind::LBracket
            )
        {
            self.bump();
            let mut lit = self.parse_literal()?;
            lit.negative = true;
            lit.span = self.span_from(start);
            return Ok(self.mk_expr(start, ExprKind::Lit(lit)));
        }
        self.enter()?;
        self.bump();
        let operand = self.parse_unary();
        self.leave();
        let operand = Box::new(operand?);
        Ok(self.mk_expr(start, ExprKind::Unary { op, operand }))
    }

    pub fn parse_literal(&mut self) -> PResult<Lit> {
        let kind = match self.tok() {
            TokenKind::IntLit(t) => LitKind::Int(t.clone()),
            TokenKind::FloatLit(t) => LitKind::Float(t.clone()),
            TokenKind::StrLit(t) => LitKind::Str(t.clone()),
            TokenKind::CharLit(t) => LitKind::Char(t.clone()),
            TokenKind::KwTrue => LitKind::Bool(true),
            TokenKind::KwFalse => LitKind::Bool(false),
            _ => return Err(self.expected("literal")),
        };
        let span = self.bump();
        let id = self.id();
        Ok(Lit {
            id,
            span,
            negative: false,
            kind,
        })
    }

    fn parse_postfix(&mut self) -> PResult<Expr> {
        let start = self.start();
        let mut e = self.parse_primary()?;
        let mut chain = 0u32;
        loop {
            self.work += 1;
            match self.tok() {
                TokenKind::LParen => {
                    chain += 1;
                    self.chain_check(chain)?;
                    self.bump();
                    let mut args = Vec::new();
                    loop {
                        if self.at(&TokenKind::RParen) {
                            break;
                        }
                        args.push(self.parse_expr_nested()?);
                        if !self.eat(&TokenKind::Comma) {
                            break;
                        }
                    }
                    self.expect(&TokenKind::RParen, "`)`")?;
                    e = self.mk_expr(
                        start,
                        ExprKind::Call {
                            callee: Box::new(e),
                            args,
                        },
                    );
                }
                TokenKind::Dot => {
                    chain += 1;
                    self.chain_check(chain)?;
                    self.bump();
                    let name = match self.tok() {
                        TokenKind::Ident(_) => FieldName::Named(self.parse_ident("tên trường")?),
                        TokenKind::IntLit(t) => {
                            let value = t.parse::<u32>().ok();
                            let span = self.span_here();
                            let Some(value) = value else {
                                return Err(self.error_at(
                                    span,
                                    "chỉ số tuple phải là số nguyên thập phân không hậu tố"
                                        .to_string(),
                                ));
                            };
                            self.bump();
                            let id = self.id();
                            FieldName::Index(TupleIndex { id, span, value })
                        }
                        _ => return Err(self.expected("tên trường hoặc chỉ số tuple")),
                    };
                    e = self.mk_expr(
                        start,
                        ExprKind::Field {
                            base: Box::new(e),
                            name,
                        },
                    );
                }
                TokenKind::LBracket => {
                    chain += 1;
                    self.chain_check(chain)?;
                    self.bump();
                    let index = self.parse_expr_nested()?;
                    self.expect(&TokenKind::RBracket, "`]`")?;
                    e = self.mk_expr(
                        start,
                        ExprKind::Index {
                            base: Box::new(e),
                            index: Box::new(index),
                        },
                    );
                }
                _ => break,
            }
        }
        Ok(e)
    }

    fn parse_primary(&mut self) -> PResult<Expr> {
        let start = self.start();
        match self.tok() {
            TokenKind::IntLit(_)
            | TokenKind::FloatLit(_)
            | TokenKind::StrLit(_)
            | TokenKind::CharLit(_)
            | TokenKind::KwTrue
            | TokenKind::KwFalse => {
                let lit = self.parse_literal()?;
                Ok(self.mk_expr(start, ExprKind::Lit(lit)))
            }
            TokenKind::Ident(_) | TokenKind::KwSelfTy | TokenKind::KwSelfKw => {
                let path = self.parse_path(true)?;
                let is_value_self = matches!(path.segments.first().map(|s| &s.kind), Some(SegmentKind::SelfValue));
                if self.at(&TokenKind::LBrace) && !self.no_struct && !is_value_self {
                    return self.parse_struct_lit(start, path);
                }
                Ok(self.mk_expr(start, ExprKind::Path(path)))
            }
            TokenKind::LParen => self.parse_paren(start),
            TokenKind::LBracket => self.parse_array(start),
            TokenKind::LBrace => {
                let block = self.parse_block()?;
                Ok(self.mk_expr(start, ExprKind::Block(block)))
            }
            TokenKind::ForAll | TokenKind::Exists => {
                let kind = if self.at(&TokenKind::ForAll) { QuantifierKind::ForAll } else { QuantifierKind::Exists };
                self.bump();
                let var = self.parse_ident("tên biến lượng từ")?;
                self.expect(&TokenKind::InSet, "`∈`")?;
                let domain = Box::new(self.parse_range_nested()?);
                self.expect(&TokenKind::Colon, "`:`")?;
                let body = Box::new(self.parse_expr()?);
                Ok(self.mk_expr(start, ExprKind::Quantifier { kind, var, domain, body }))
            }
            TokenKind::KwIf | TokenKind::KwMatch | TokenKind::LoopSym | TokenKind::KwUnsafe => {
                Err(self.expected("toán hạng (khối-biểu-thức `⎇ ⌥ ⟳ ⚠` không dùng làm toán hạng; gán vào biến trước)"))
            }
            _ => Err(self.expected("biểu thức")),
        }
    }

    fn parse_range_nested(&mut self) -> PResult<Expr> {
        let saved = std::mem::replace(&mut self.no_struct, false);
        self.enter()?;
        let r = self.parse_range();
        self.leave();
        self.no_struct = saved;
        r
    }

    fn parse_paren(&mut self, start: u32) -> PResult<Expr> {
        self.bump();
        if self.eat(&TokenKind::RParen) {
            return Ok(self.mk_expr(start, ExprKind::Tuple(vec![])));
        }
        let first = self.parse_expr_nested()?;
        if self.eat(&TokenKind::RParen) {
            return Ok(first); // ngoặc nhóm: không có node
        }
        self.expect(&TokenKind::Comma, "`,` hoặc `)`")?;
        let mut elems = vec![first];
        loop {
            if self.at(&TokenKind::RParen) {
                break;
            }
            elems.push(self.parse_expr_nested()?);
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(&TokenKind::RParen, "`)`")?;
        Ok(self.mk_expr(start, ExprKind::Tuple(elems)))
    }

    fn parse_array(&mut self, start: u32) -> PResult<Expr> {
        self.bump();
        if self.eat(&TokenKind::RBracket) {
            return Ok(self.mk_expr(start, ExprKind::Array(vec![])));
        }
        let first = self.parse_expr_nested()?;
        if self.eat(&TokenKind::Semi) {
            let len = self.parse_expr_nested()?;
            self.expect(&TokenKind::RBracket, "`]`")?;
            return Ok(self.mk_expr(
                start,
                ExprKind::ArrayRepeat {
                    value: Box::new(first),
                    len: Box::new(len),
                },
            ));
        }
        let mut elems = vec![first];
        while self.eat(&TokenKind::Comma) {
            if self.at(&TokenKind::RBracket) {
                break;
            }
            elems.push(self.parse_expr_nested()?);
        }
        self.expect(&TokenKind::RBracket, "`]`")?;
        Ok(self.mk_expr(start, ExprKind::Array(elems)))
    }

    fn parse_struct_lit(&mut self, start: u32, path: Path) -> PResult<Expr> {
        self.bump(); // `{`
        let saved = std::mem::replace(&mut self.no_struct, false);
        let mut fields = Vec::new();
        let r = (|| -> PResult<()> {
            loop {
                if self.at(&TokenKind::RBrace) {
                    break;
                }
                let fstart = self.start();
                let name = self.parse_ident("tên trường")?;
                self.expect(&TokenKind::Colon, "`:`")?;
                let value = self.parse_expr()?;
                let id = self.id();
                fields.push(FieldInit {
                    id,
                    span: self.span_from(fstart),
                    name,
                    value,
                });
                if !self.eat(&TokenKind::Comma) {
                    break;
                }
            }
            Ok(())
        })();
        self.no_struct = saved;
        r?;
        self.expect(&TokenKind::RBrace, "`}`")?;
        Ok(self.mk_expr(start, ExprKind::StructLit { path, fields }))
    }

    // ---------- khối-biểu-thức ----------

    fn parse_if(&mut self) -> PResult<Expr> {
        let start = self.start();
        self.bump();
        let cond = Box::new(self.parse_expr_ns()?);
        let then_block = self.parse_block()?;
        let else_branch = if self.eat(&TokenKind::KwElse) {
            let estart = self.start();
            if self.at(&TokenKind::KwIf) {
                Some(Box::new(self.parse_expr()?))
            } else {
                let b = self.parse_block()?;
                Some(Box::new(self.mk_expr(estart, ExprKind::Block(b))))
            }
        } else {
            None
        };
        Ok(self.mk_expr(
            start,
            ExprKind::If {
                cond,
                then_block,
                else_branch,
            },
        ))
    }

    fn parse_match(&mut self) -> PResult<Expr> {
        let start = self.start();
        self.bump();
        let scrutinee = Box::new(self.parse_expr_ns()?);
        self.expect(&TokenKind::LBrace, "`{`")?;
        let saved = std::mem::replace(&mut self.no_struct, false);
        let mut arms = Vec::new();
        loop {
            self.work += 1;
            if self.at(&TokenKind::RBrace) || self.at_eof() {
                break;
            }
            let before = self.pos;
            match self.parse_arm() {
                Ok(a) => {
                    arms.push(a);
                    if !self.eat(&TokenKind::Comma) && !self.at(&TokenKind::RBrace) {
                        let _ = self.expected("`,` hoặc `}` sau nhánh match");
                        self.recover_arm();
                    }
                }
                Err(Abort) => self.recover_arm(),
            }
            if self.pos == before {
                self.bump();
            }
        }
        self.no_struct = saved;
        self.expect(&TokenKind::RBrace, "`}`")?;
        Ok(self.mk_expr(start, ExprKind::Match { scrutinee, arms }))
    }

    fn parse_arm(&mut self) -> PResult<Arm> {
        let start = self.start();
        let pattern = self.parse_pattern()?;
        self.expect(&TokenKind::FatArrow, "`↦`")?;
        let body = self.parse_expr()?;
        let id = self.id();
        Ok(Arm {
            id,
            span: self.span_from(start),
            pattern,
            body,
        })
    }

    fn parse_loop(&mut self) -> PResult<Expr> {
        let start = self.start();
        self.bump();
        let kind = if self.at(&TokenKind::LBrace) || self.at(&TokenKind::Dec) {
            LoopKind::Infinite
        } else if matches!(self.tok(), TokenKind::Ident(_)) && self.peek_is(1, &TokenKind::KwIn) {
            let var = self.parse_ident("tên biến lặp")?;
            self.bump(); // `in`
            let iter = Box::new(self.parse_expr_ns()?);
            LoopKind::ForIn { var, iter }
        } else {
            LoopKind::While(Box::new(self.parse_expr_ns()?))
        };
        let variant = if self.eat(&TokenKind::Dec) {
            Some(Box::new(self.parse_expr_ns()?))
        } else {
            None
        };
        let mut invariants = Vec::new();
        while self.eat(&TokenKind::Inv) {
            invariants.push(self.parse_expr_ns()?);
        }
        let body = self.parse_block()?;
        Ok(self.mk_expr(
            start,
            ExprKind::Loop(Loop {
                kind,
                variant,
                invariants,
                body,
            }),
        ))
    }

    fn parse_unsafe(&mut self) -> PResult<Expr> {
        let start = self.start();
        self.bump();
        let mut proofs = Vec::new();
        while self.eat(&TokenKind::ProofSym) {
            proofs.push(self.parse_expr_ns()?);
        }
        let body = self.parse_block()?;
        Ok(self.mk_expr(start, ExprKind::Unsafe { proofs, body }))
    }

    // ---------- khối & câu lệnh ----------

    pub fn parse_block(&mut self) -> PResult<Block> {
        let start = self.start();
        self.expect(&TokenKind::LBrace, "`{`")?;
        self.enter()?;
        let saved = std::mem::replace(&mut self.no_struct, false);
        let mut stmts = Vec::new();
        let mut tail: Option<Box<Expr>> = None;
        loop {
            self.work += 1;
            if self.at(&TokenKind::RBrace) || self.at_eof() {
                break;
            }
            let before = self.pos;
            match self.parse_stmt() {
                Ok(StmtOrTail::Stmt(s)) => stmts.push(*s),
                Ok(StmtOrTail::Tail(e)) => {
                    tail = Some(e);
                    break;
                }
                Err(Abort) => self.recover_stmt(),
            }
            if self.pos == before {
                self.bump();
            }
        }
        self.no_struct = saved;
        self.leave();
        self.expect(&TokenKind::RBrace, "`}`")?;
        let id = self.id();
        Ok(Block {
            id,
            span: self.span_from(start),
            stmts,
            tail,
        })
    }

    fn mk_stmt(&mut self, start: u32, kind: StmtKind) -> StmtOrTail {
        let id = self.id();
        StmtOrTail::Stmt(Box::new(Stmt {
            id,
            span: self.span_from(start),
            kind,
        }))
    }

    fn is_place(e: &Expr) -> bool {
        match &e.kind {
            ExprKind::Path(p) => p.segments.len() == 1,
            ExprKind::Field { base, .. } | ExprKind::Index { base, .. } => Self::is_place(base),
            ExprKind::Unary {
                op: UnaryOp::Deref,
                operand,
            } => Self::is_place(operand),
            _ => false,
        }
    }

    fn parse_stmt(&mut self) -> PResult<StmtOrTail> {
        let start = self.start();
        match self.tok() {
            TokenKind::DocComment(_) => {
                let span = self.span_here();
                let abort = self.error_at(
                    span,
                    "doc comment `///` không gắn được vào node nào ở đây".to_string(),
                );
                self.bump();
                Err(abort)
            }
            TokenKind::KwMut => {
                self.bump();
                self.parse_let(start, true)
            }
            TokenKind::Ident(_)
                if matches!(self.peek_kind(1), TokenKind::Walrus | TokenKind::Colon) =>
            {
                self.parse_let(start, false)
            }
            TokenKind::KwRet => {
                self.bump();
                let value = if self.at(&TokenKind::Semi) {
                    None
                } else {
                    Some(self.parse_expr()?)
                };
                self.expect(&TokenKind::Semi, "`;`")?;
                Ok(self.mk_stmt(start, StmtKind::Return(value)))
            }
            TokenKind::KwBreak => {
                self.bump();
                self.expect(&TokenKind::Semi, "`;`")?;
                Ok(self.mk_stmt(start, StmtKind::Break))
            }
            TokenKind::KwCont => {
                self.bump();
                self.expect(&TokenKind::Semi, "`;`")?;
                Ok(self.mk_stmt(start, StmtKind::Continue))
            }
            TokenKind::KwIf | TokenKind::KwMatch | TokenKind::LoopSym | TokenKind::KwUnsafe => {
                let e = self.parse_expr()?;
                if self.eat(&TokenKind::Semi) {
                    Ok(self.mk_stmt(
                        start,
                        StmtKind::Expr {
                            expr: e,
                            semi: true,
                        },
                    ))
                } else if self.at(&TokenKind::RBrace) {
                    Ok(StmtOrTail::Tail(Box::new(e)))
                } else {
                    Ok(self.mk_stmt(
                        start,
                        StmtKind::Expr {
                            expr: e,
                            semi: false,
                        },
                    ))
                }
            }
            _ => {
                let e = self.parse_expr()?;
                if self.at(&TokenKind::Assign) {
                    if !Self::is_place(&e) {
                        let span = e.span;
                        return Err(self.error_at(
                            span,
                            "đích của `←` phải là biến, trường, chỉ số hoặc `*` của chúng"
                                .to_string(),
                        ));
                    }
                    self.bump();
                    let value = self.parse_expr()?;
                    self.expect(&TokenKind::Semi, "`;`")?;
                    return Ok(self.mk_stmt(start, StmtKind::Assign { target: e, value }));
                }
                if self.eat(&TokenKind::Semi) {
                    Ok(self.mk_stmt(
                        start,
                        StmtKind::Expr {
                            expr: e,
                            semi: true,
                        },
                    ))
                } else if self.at(&TokenKind::RBrace) {
                    Ok(StmtOrTail::Tail(Box::new(e)))
                } else {
                    Err(self.expected("`;` hoặc `}`"))
                }
            }
        }
    }

    fn parse_let(&mut self, start: u32, mutable: bool) -> PResult<StmtOrTail> {
        let name = self.parse_ident("tên biến")?;
        let ty = if self.eat(&TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };
        self.expect(&TokenKind::Walrus, "`≔`")?;
        let init = self.parse_expr()?;
        self.expect(&TokenKind::Semi, "`;`")?;
        Ok(self.mk_stmt(
            start,
            StmtKind::Let {
                mutable,
                name,
                ty,
                init,
            },
        ))
    }
}
