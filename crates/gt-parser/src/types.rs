//! Kiểu.
use crate::parser::{PResult, Parser};
use gt_ast::*;
use gt_lexer::TokenKind;

fn prim(name: &str) -> Option<PrimType> {
    Some(match name {
        "i8" => PrimType::I8,
        "i16" => PrimType::I16,
        "i32" => PrimType::I32,
        "i64" => PrimType::I64,
        "i128" => PrimType::I128,
        "isize" => PrimType::Isize,
        "u8" => PrimType::U8,
        "u16" => PrimType::U16,
        "u32" => PrimType::U32,
        "u64" => PrimType::U64,
        "u128" => PrimType::U128,
        "usize" => PrimType::Usize,
        "f32" => PrimType::F32,
        "f64" => PrimType::F64,
        "bool" => PrimType::Bool,
        "char" => PrimType::Char,
        "str" => PrimType::Str,
        _ => return None,
    })
}

impl Parser {
    pub fn parse_type(&mut self) -> PResult<Type> {
        self.enter()?;
        let r = self.parse_type_inner();
        self.leave();
        r
    }

    fn mk(&mut self, start: u32, kind: TypeKind) -> Type {
        let id = self.id();
        Type {
            id,
            span: self.span_from(start),
            kind,
        }
    }

    fn parse_type_inner(&mut self) -> PResult<Type> {
        let start = self.start();
        match self.tok() {
            TokenKind::Ident(_) => {
                let path = self.parse_path(false)?;
                if let [seg] = path.segments.as_slice()
                    && let SegmentKind::Name(n) = &seg.kind
                    && let Some(p) = prim(n)
                    && !self.at(&TokenKind::Lt)
                {
                    return Ok(self.mk(start, TypeKind::Prim(p)));
                }
                let mut args = Vec::new();
                if self.at(&TokenKind::Lt) {
                    self.bump();
                    loop {
                        if self.at(&TokenKind::Gt) {
                            break;
                        }
                        args.push(self.parse_type()?);
                        if !self.eat(&TokenKind::Comma) {
                            break;
                        }
                    }
                    self.expect(&TokenKind::Gt, "`>`")?;
                }
                Ok(self.mk(start, TypeKind::Path { path, args }))
            }
            TokenKind::KwSelfTy => {
                self.bump();
                Ok(self.mk(start, TypeKind::SelfType))
            }
            TokenKind::Bang => {
                self.bump();
                Ok(self.mk(start, TypeKind::Never))
            }
            TokenKind::Borrow | TokenKind::BorrowMut => {
                let mutable = self.at(&TokenKind::BorrowMut);
                self.bump();
                let lifetime = if let TokenKind::Lifetime(name) = self.tok() {
                    let name = name.clone();
                    let span = self.bump();
                    let id = self.id();
                    Some(Lifetime { id, span, name })
                } else {
                    None
                };
                let inner = Box::new(self.parse_type()?);
                Ok(self.mk(
                    start,
                    TypeKind::Ref {
                        lifetime,
                        mutable,
                        inner,
                    },
                ))
            }
            TokenKind::LBracket => {
                self.bump();
                let elem = Box::new(self.parse_type()?);
                if self.eat(&TokenKind::Semi) {
                    let saved = std::mem::replace(&mut self.no_struct, false);
                    let len = self.parse_expr();
                    self.no_struct = saved;
                    let len = Box::new(len?);
                    self.expect(&TokenKind::RBracket, "`]`")?;
                    Ok(self.mk(start, TypeKind::Array { elem, len }))
                } else {
                    self.expect(&TokenKind::RBracket, "`]` hoặc `;`")?;
                    Ok(self.mk(start, TypeKind::Slice(elem)))
                }
            }
            TokenKind::LParen => {
                self.bump();
                if self.eat(&TokenKind::RParen) {
                    return Ok(self.mk(start, TypeKind::Tuple(vec![])));
                }
                let first = self.parse_type()?;
                if self.eat(&TokenKind::RParen) {
                    return Ok(first); // ngoặc nhóm: không có node
                }
                self.expect(&TokenKind::Comma, "`,` hoặc `)`")?;
                let mut elems = vec![first];
                loop {
                    if self.at(&TokenKind::RParen) {
                        break;
                    }
                    elems.push(self.parse_type()?);
                    if !self.eat(&TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(&TokenKind::RParen, "`)`")?;
                Ok(self.mk(start, TypeKind::Tuple(elems)))
            }
            TokenKind::Star => {
                self.bump();
                let mutable = match self.tok() {
                    TokenKind::KwConst => false,
                    TokenKind::KwMut => true,
                    _ => return Err(self.expected("`□` hoặc `μ` sau `*` (con trỏ thô)")),
                };
                self.bump();
                let inner = Box::new(self.parse_type()?);
                Ok(self.mk(start, TypeKind::RawPtr { mutable, inner }))
            }
            TokenKind::KwFn => {
                self.bump();
                self.expect(&TokenKind::LParen, "`(`")?;
                let mut params = Vec::new();
                loop {
                    if self.at(&TokenKind::RParen) {
                        break;
                    }
                    params.push(self.parse_type()?);
                    if !self.eat(&TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(&TokenKind::RParen, "`)`")?;
                let ret = if self.eat(&TokenKind::Arrow) {
                    Some(Box::new(self.parse_type()?))
                } else {
                    None
                };
                let effects = self.parse_effects()?;
                Ok(self.mk(
                    start,
                    TypeKind::Fn {
                        params,
                        ret,
                        effects,
                    },
                ))
            }
            _ => Err(self.expected("kiểu")),
        }
    }
}
