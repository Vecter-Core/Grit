//! Lõi parser: con trỏ token, báo lỗi, khôi phục lỗi, giới hạn độ sâu, và các item.
use crate::convert::describe;
use gt_ast::*;

/// Gợi ý kèm mọi lỗi E0102.
const DEPTH_HINT: &str =
    "hãy tách biểu thức con ra biến phụ bằng `≔` (hoặc chia nhỏ khối/kiểu/pattern)";
use gt_diag::{Applicability, Code, Diagnostic, Edit, Suggestion};
use gt_lexer::{Token, TokenKind};
use gt_span::Span;

/// Lỗi đã được báo (hoặc cố ý im lặng vì lexer đã báo); chỉ dùng để dừng production hiện tại.
#[derive(Debug)]
pub(crate) struct Abort;

/// Điểm quay lui (xem `Parser::snapshot`).
pub(crate) struct Snapshot {
    pos: usize,
    diags: usize,
}

pub(crate) type PResult<T> = Result<T, Abort>;

static EOF_KIND: TokenKind = TokenKind::Eof;

pub(crate) struct Parser {
    pub file: String,
    pub src_len: u32,
    tokens: Vec<Token>,
    pub pos: usize,
    pub ids: NodeIdGen,
    pub diags: Vec<Diagnostic>,
    pub depth: u32,
    pub no_struct: bool,
    pub in_assoc: bool,
    pub fatal: bool,
    pub work: usize,
    /// Độ sâu lớn nhất đã chạm (kiểm chứng luật độ sâu: semantics §26).
    pub max_depth: u32,
    last_error_pos: Option<usize>,
    prev_end: u32,
}

impl Parser {
    pub fn new(file_id: gt_span::FileId, file: &str, src_len: usize, tokens: Vec<Token>) -> Parser {
        Parser {
            file: file.to_string(),
            src_len: u32::try_from(src_len).unwrap_or(u32::MAX),
            tokens,
            pos: 0,
            ids: NodeIdGen::new(file_id),
            diags: Vec::new(),
            depth: 0,
            no_struct: false,
            in_assoc: false,
            fatal: false,
            work: 0,
            max_depth: 0,
            last_error_pos: None,
            prev_end: 0,
        }
    }

    // ---------- con trỏ token ----------

    pub fn tok(&self) -> &TokenKind {
        if self.fatal {
            return &EOF_KIND;
        }
        self.tokens.get(self.pos).map_or(&EOF_KIND, |t| &t.kind)
    }

    pub fn peek_kind(&self, n: usize) -> &TokenKind {
        if self.fatal {
            return &EOF_KIND;
        }
        self.tokens.get(self.pos + n).map_or(&EOF_KIND, |t| &t.kind)
    }

    pub fn at(&self, k: &TokenKind) -> bool {
        std::mem::discriminant(self.tok()) == std::mem::discriminant(k)
    }

    pub fn peek_is(&self, n: usize, k: &TokenKind) -> bool {
        std::mem::discriminant(self.peek_kind(n)) == std::mem::discriminant(k)
    }

    pub fn at_eof(&self) -> bool {
        matches!(self.tok(), TokenKind::Eof)
    }

    /// Span của token hiện tại (rỗng ở cuối tệp).
    pub fn span_here(&self) -> Span {
        if self.fatal {
            return Span::new(self.src_len, self.src_len);
        }
        self.tokens
            .get(self.pos)
            .map_or(Span::new(self.src_len, self.src_len), |t| t.span)
    }

    pub fn start(&self) -> u32 {
        self.span_here().start
    }

    pub fn span_from(&self, start: u32) -> Span {
        Span::new(start, self.prev_end.max(start))
    }

    /// Tiêu thụ token hiện tại, trả về span của nó. Không đi quá `Eof`.
    pub fn bump(&mut self) -> Span {
        self.work += 1;
        let span = self.span_here();
        if !self.fatal
            && !matches!(
                self.tokens.get(self.pos).map(|t| &t.kind),
                Some(TokenKind::Eof) | None
            )
        {
            self.pos += 1;
            self.prev_end = span.end;
        }
        span
    }

    pub fn eat(&mut self, k: &TokenKind) -> bool {
        if self.at(k) {
            self.bump();
            true
        } else {
            false
        }
    }

    pub fn expect(&mut self, k: &TokenKind, what: &str) -> PResult<Span> {
        if self.at(k) {
            Ok(self.bump())
        } else {
            Err(self.expected(what))
        }
    }

    // ---------- NodeId ----------

    pub fn id(&mut self) -> NodeId {
        match self.ids.next_id() {
            Some(id) => id,
            None => {
                self.depth_error("cây AST quá lớn (hết NodeId)");
                NodeId::new(self.ids.file(), u32::MAX)
            }
        }
    }

    // ---------- lỗi ----------

    fn push_diag(&mut self, code: &str, span: Span, msg: String) {
        let c = Code::parse_or_internal(code);
        self.diags.push(Diagnostic::new(c, &self.file, span, msg));
    }

    /// Báo lỗi cú pháp E0101 tại `span`. Im lặng nếu: đã dừng vì độ sâu, token hiện tại là `UNKNOWN`
    /// (lexer đã báo), hoặc đã có lỗi tại cùng vị trí token (tránh lỗi dây chuyền).
    pub fn error_at(&mut self, span: Span, msg: String) -> Abort {
        self.work += 1;
        let silent = self.fatal
            || matches!(self.tok(), TokenKind::Unknown)
            || self.last_error_pos == Some(self.pos);
        if !silent {
            self.last_error_pos = Some(self.pos);
            self.push_diag("E0101", span, msg);
        }
        Abort
    }

    /// Trạng thái để quay lui đúng một lần ở chỗ `x as T <` (Q16c).
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            pos: self.pos,
            diags: self.diags.len(),
        }
    }

    /// Sau khi `parse_type` sau `as` thất bại: nếu kiểu là đường dẫn (có thể sau `&`/`&!`) theo sau bởi `<`, thì đây là quy tắc
    /// `x as T < y` (Q16c). Thay các chẩn đoán vừa phát bằng MỘT E0101 trỏ vào `<`, kèm gợi ý machine-applicable bọc
    /// `(x as T)` (hai edit: `(` ở đầu vế trái của phép ép kiểu, `)` ngay sau kiểu). Trả `false` (không đổi gì) nếu không khớp.
    pub fn explain_cast_lt(&mut self, snap: &Snapshot, lhs_start: u32) -> bool {
        if self.fatal {
            return false;
        }
        let mut i = snap.pos;
        while matches!(
            self.tokens.get(i).map(|t| &t.kind),
            Some(TokenKind::Borrow | TokenKind::BorrowMut)
        ) {
            i += 1;
        }
        if !matches!(
            self.tokens.get(i).map(|t| &t.kind),
            Some(TokenKind::Ident(_))
        ) {
            return false;
        }
        i += 1;
        while matches!(
            self.tokens.get(i).map(|t| &t.kind),
            Some(TokenKind::ColonColon)
        ) && matches!(
            self.tokens.get(i + 1).map(|t| &t.kind),
            Some(TokenKind::Ident(_))
        ) {
            i += 2;
        }
        let (Some(lt), Some(last)) = (self.tokens.get(i), self.tokens.get(i - 1)) else {
            return false;
        };
        if lt.kind != TokenKind::Lt {
            return false;
        }
        let (lt_span, ty_end) = (lt.span, last.span.end);
        // Thử: gợi ý chỉ "machine-applicable" khi phần sau `<` thật sự parse được thành biểu thức (vế phải của phép so sánh)
        let (err_pos, err_last) = (self.pos, self.last_error_pos);
        let saved = self.diags.split_off(snap.diags);
        self.pos = i + 1;
        self.last_error_pos = None;
        let rhs_ok = self.parse_expr().is_ok() && self.diags.len() == snap.diags && !self.fatal;
        self.diags.truncate(snap.diags);
        self.pos = err_pos;
        if !rhs_ok {
            self.diags.extend(saved);
            self.last_error_pos = err_last;
            return false;
        }
        if let Ok(code) = Code::parse("E0101") {
            self.diags.push(
                Diagnostic::new(
                    code,
                    &self.file,
                    lt_span,
                    "`as T <`: `<` sau kiểu mở danh sách tham số kiểu; bọc phép ép kiểu trong ngoặc: `(x as T) < y`".to_string(),
                )
                .with_span_label("đọc là tham số kiểu của kiểu đứng trước")
                .with_suggestion(Suggestion::new(
                    "bọc phép ép kiểu trong ngoặc",
                    Applicability::MachineApplicable,
                    vec![
                        Edit::new(Span::new(lhs_start, lhs_start), "("),
                        Edit::new(Span::new(ty_end, ty_end), ")"),
                    ],
                )),
            );
        }
        self.last_error_pos = Some(self.pos);
        true
    }

    pub fn expected(&mut self, what: &str) -> Abort {
        let found = describe(self.tok());
        let span = self.span_here();
        self.error_at(span, format!("mong đợi {what}, gặp {found}"))
    }

    /// Lỗi độ sâu: ĐỎ `E0102`, rồi dừng toàn bộ phân tích (mọi token sau đó coi như hết tệp).
    pub fn depth_error(&mut self, msg: &str) {
        if !self.fatal {
            let span = self.span_here();
            self.push_diag("E0102", span, msg.to_string());
            if let Some(d) = self.diags.last_mut() {
                d.notes.push(DEPTH_HINT.to_string());
            }
            self.fatal = true;
        }
    }

    pub fn enter(&mut self) -> PResult<()> {
        self.work += 1;
        self.depth += 1;
        self.max_depth = self.max_depth.max(self.depth);
        if self.depth > MAX_DEPTH {
            self.depth -= 1;
            self.depth_error(&format!("lồng nhau quá sâu (> {MAX_DEPTH} mức)"));
            return Err(Abort);
        }
        Ok(())
    }

    pub fn leave(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    /// Chuỗi toán tử/postfix tạo cây lệch trái: độ sâu cây = độ lồng + độ dài chuỗi.
    pub fn chain_check(&mut self, chain: u32) -> PResult<()> {
        let total = self.depth.saturating_add(chain);
        if total <= MAX_DEPTH {
            self.max_depth = self.max_depth.max(total);
        }
        if total > MAX_DEPTH {
            self.depth_error(&format!(
                "chuỗi toán tử/postfix quá dài (> {MAX_DEPTH} mức)"
            ));
            return Err(Abort);
        }
        Ok(())
    }

    // ---------- khôi phục lỗi ----------

    /// Bỏ qua token tới ranh giới câu lệnh: tiêu thụ `;` ở độ sâu 0, hoặc dừng TRƯỚC `}` đóng khối.
    /// Luôn tiêu thụ ít nhất một token trừ khi gặp `}`/hết tệp ngay lập tức (người gọi bảo đảm tiến lên).
    pub fn recover_stmt(&mut self) {
        let mut depth: i32 = 0;
        loop {
            self.work += 1;
            match self.tok() {
                TokenKind::Eof => return,
                TokenKind::Semi if depth == 0 => {
                    self.bump();
                    return;
                }
                TokenKind::RBrace if depth == 0 => return,
                TokenKind::LBrace | TokenKind::LParen | TokenKind::LBracket => depth += 1,
                TokenKind::RBrace | TokenKind::RParen | TokenKind::RBracket => {
                    depth = (depth - 1).max(0)
                }
                _ => {}
            }
            self.bump();
        }
    }

    /// Bỏ qua tới dấu `,` (tiêu thụ) hoặc `}` (dừng trước) ở độ sâu 0 — dùng cho nhánh `match`.
    pub fn recover_arm(&mut self) {
        let mut depth: i32 = 0;
        loop {
            self.work += 1;
            match self.tok() {
                TokenKind::Eof => return,
                TokenKind::Comma if depth == 0 => {
                    self.bump();
                    return;
                }
                TokenKind::RBrace if depth == 0 => return,
                TokenKind::LBrace | TokenKind::LParen | TokenKind::LBracket => depth += 1,
                TokenKind::RBrace | TokenKind::RParen | TokenKind::RBracket => {
                    depth = (depth - 1).max(0)
                }
                _ => {}
            }
            self.bump();
        }
    }

    fn is_item_start(k: &TokenKind) -> bool {
        matches!(
            k,
            TokenKind::KwFn
                | TokenKind::KwStruct
                | TokenKind::KwEnum
                | TokenKind::KwTrait
                | TokenKind::KwImpl
                | TokenKind::KwMod
                | TokenKind::KwUse
                | TokenKind::KwPub
                | TokenKind::KwType
                | TokenKind::KwConst
                | TokenKind::KwStatic
                | TokenKind::Hash
                | TokenKind::DocComment(_)
        )
    }

    /// Bỏ qua tới ranh giới item: sau khi đóng hết `{…}` của item hỏng, sau `;` ở độ sâu 0, hoặc trước từ khoá mở item kế tiếp.
    /// Luôn tiêu thụ ít nhất một token (trừ hết tệp).
    pub fn recover_item(&mut self) {
        let mut depth: i32 = 0;
        let mut first = true;
        loop {
            self.work += 1;
            let k = self.tok();
            if matches!(k, TokenKind::Eof) {
                return;
            }
            if depth == 0 && !first && Self::is_item_start(k) {
                return;
            }
            let is_open = matches!(
                k,
                TokenKind::LBrace | TokenKind::LParen | TokenKind::LBracket
            );
            let is_close = matches!(
                k,
                TokenKind::RBrace | TokenKind::RParen | TokenKind::RBracket
            );
            let is_rbrace = matches!(k, TokenKind::RBrace);
            let is_semi = matches!(k, TokenKind::Semi);
            if is_open {
                depth += 1;
            } else if is_close {
                depth = (depth - 1).max(0);
            }
            self.bump();
            first = false;
            if depth == 0 && (is_rbrace || is_semi) {
                return;
            }
        }
    }

    // ---------- chương trình & item ----------

    pub fn parse_program(&mut self) -> Program {
        let mut items = Vec::new();
        self.parse_items_until(&mut items, false);
        let id = self.id();
        Program {
            id,
            span: Span::new(0, self.src_len),
            items,
        }
    }

    /// Vòng lặp item dùng chung cho tệp và `▦ { … }`. `in_mod`: dừng ở `}` đóng.
    fn parse_items_until(&mut self, items: &mut Vec<Item>, in_mod: bool) {
        loop {
            self.work += 1;
            if self.at_eof() || (in_mod && matches!(self.tok(), TokenKind::RBrace)) {
                return;
            }
            let before = self.pos;
            match self.parse_item() {
                Ok(i) => items.push(i),
                Err(_) => self.recover_item(),
            }
            if self.pos == before {
                self.bump();
            }
        }
    }

    pub fn parse_ident(&mut self, what: &str) -> PResult<Ident> {
        if let TokenKind::Ident(name) = self.tok() {
            let name = name.clone();
            let span = self.bump();
            let id = self.id();
            Ok(Ident { id, span, name })
        } else {
            Err(self.expected(what))
        }
    }

    fn parse_docs(&mut self) -> Vec<DocComment> {
        let mut docs = Vec::new();
        while let TokenKind::DocComment(text) = self.tok() {
            let text = text.clone();
            let span = self.bump();
            let id = self.id();
            docs.push(DocComment { id, span, text });
        }
        docs
    }

    fn parse_attribute(&mut self) -> PResult<Attribute> {
        let start = self.start();
        self.expect(&TokenKind::Hash, "`#`")?;
        let name = self.parse_ident("tên thuộc tính")?;
        let mut args = Vec::new();
        if self.eat(&TokenKind::LParen) {
            loop {
                if self.at(&TokenKind::RParen) {
                    break;
                }
                let astart = self.start();
                let kind = match self.tok() {
                    TokenKind::Ident(s) => AttrArgKind::Ident(s.clone()),
                    TokenKind::StrLit(s) => AttrArgKind::Str(s.clone()),
                    _ => return Err(self.expected("đối số thuộc tính (tên hoặc chuỗi)")),
                };
                self.bump();
                let id = self.id();
                args.push(AttrArg {
                    id,
                    span: self.span_from(astart),
                    kind,
                });
                if !self.eat(&TokenKind::Comma) {
                    break;
                }
            }
            self.expect(&TokenKind::RParen, "`)`")?;
        }
        let id = self.id();
        Ok(Attribute {
            id,
            span: self.span_from(start),
            name,
            args,
        })
    }

    pub fn parse_item(&mut self) -> PResult<Item> {
        let start = self.start();
        let docs = self.parse_docs();
        let mut attrs = Vec::new();
        while self.at(&TokenKind::Hash) {
            attrs.push(self.parse_attribute()?);
        }
        let vis = if self.eat(&TokenKind::KwPub) {
            Visibility::Public
        } else {
            Visibility::Private
        };
        let kind = match self.tok() {
            TokenKind::KwFn => {
                let sig = self.parse_fn_sig()?;
                let body = self.parse_block()?;
                ItemKind::Fn(FnItem { sig, body })
            }
            TokenKind::KwStruct => self.parse_struct()?,
            TokenKind::KwEnum => self.parse_enum()?,
            TokenKind::KwTrait => self.parse_trait()?,
            TokenKind::KwImpl => self.parse_impl()?,
            TokenKind::KwMod => self.parse_mod()?,
            TokenKind::KwUse => self.parse_use()?,
            TokenKind::KwType => self.parse_type_alias()?,
            TokenKind::KwConst => {
                let (name, ty, value) = self.parse_const_like("`□`")?;
                ItemKind::Const(ConstItem { name, ty, value })
            }
            TokenKind::KwStatic => {
                let (name, ty, value) = self.parse_const_like("`■`")?;
                ItemKind::Static(StaticItem { name, ty, value })
            }
            _ => {
                let what = if docs.is_empty() && attrs.is_empty() && vis == Visibility::Private {
                    "item (`ƒ`, `▢`, `⊔`, `⊏`, `⊐`, `▦`, `⇲`, `≡`, `□`, `■`)"
                } else {
                    "item sau doc comment/thuộc tính/`◈`"
                };
                return Err(self.expected(what));
            }
        };
        let id = self.id();
        Ok(Item {
            id,
            span: self.span_from(start),
            docs,
            attrs,
            vis,
            kind,
        })
    }

    fn parse_generics_opt(&mut self) -> PResult<Option<Generics>> {
        if !self.at(&TokenKind::Lt) {
            return Ok(None);
        }
        let start = self.start();
        self.bump();
        let mut params = Vec::new();
        loop {
            if self.at(&TokenKind::Gt) {
                break;
            }
            let pstart = self.start();
            let kind = if let TokenKind::Lifetime(name) = self.tok() {
                let name = name.clone();
                let span = self.bump();
                let id = self.id();
                GenericParamKind::Lifetime(Lifetime { id, span, name })
            } else {
                let name = self.parse_ident("tên tham số kiểu")?;
                let mut bounds = Vec::new();
                if self.eat(&TokenKind::Colon) {
                    bounds.push(self.parse_path(false)?);
                    while self.eat(&TokenKind::Plus) {
                        bounds.push(self.parse_path(false)?);
                    }
                }
                GenericParamKind::Type { name, bounds }
            };
            let id = self.id();
            params.push(GenericParam {
                id,
                span: self.span_from(pstart),
                kind,
            });
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(&TokenKind::Gt, "`>`")?;
        let id = self.id();
        Ok(Some(Generics {
            id,
            span: self.span_from(start),
            params,
        }))
    }

    pub fn parse_effects(&mut self) -> PResult<Effects> {
        let start = self.start();
        self.expect(&TokenKind::Bang, "`!` (khai báo hiệu ứng là bắt buộc)")?;
        let kind = if self.eat(&TokenKind::EmptySet) {
            EffectsKind::Pure
        } else if self.eat(&TokenKind::LBrace) {
            let mut effects = Vec::new();
            loop {
                if self.at(&TokenKind::RBrace) {
                    break;
                }
                let estart = self.start();
                let name = self.parse_ident("tên hiệu ứng")?;
                let arg = if self.eat(&TokenKind::LParen) {
                    let a = self.parse_ident("tham số hiệu ứng")?;
                    self.expect(&TokenKind::RParen, "`)`")?;
                    Some(a)
                } else {
                    None
                };
                let id = self.id();
                effects.push(Effect {
                    id,
                    span: self.span_from(estart),
                    name,
                    arg,
                });
                if !self.eat(&TokenKind::Comma) {
                    break;
                }
            }
            self.expect(&TokenKind::RBrace, "`}`")?;
            EffectsKind::Set(effects)
        } else {
            return Err(self.expected("`∅` hoặc `{hiệu ứng, …}`"));
        };
        let id = self.id();
        Ok(Effects {
            id,
            span: self.span_from(start),
            kind,
        })
    }

    fn parse_param(&mut self, first: bool) -> PResult<Param> {
        let start = self.start();
        let self_kind = match (self.tok(), self.peek_kind(1)) {
            (TokenKind::KwSelfKw, _) => Some((SelfKind::Value, 1)),
            (TokenKind::Borrow, TokenKind::KwSelfKw) => Some((SelfKind::Ref, 2)),
            (TokenKind::BorrowMut, TokenKind::KwSelfKw) => Some((SelfKind::RefMut, 2)),
            _ => None,
        };
        let kind = if let Some((sk, n)) = self_kind {
            if !(first && self.in_assoc) {
                let span = self.span_here();
                return Err(self.error_at(
                    span,
                    "`self` chỉ được làm tham số đầu của hàm trong `⊏`/`⊐`".to_string(),
                ));
            }
            for _ in 0..n {
                self.bump();
            }
            ParamKind::SelfParam(sk)
        } else {
            let mutable = self.eat(&TokenKind::KwMut);
            let name = self.parse_ident("tên tham số")?;
            self.expect(&TokenKind::Colon, "`:`")?;
            let ty = self.parse_type()?;
            ParamKind::Named { mutable, name, ty }
        };
        let id = self.id();
        Ok(Param {
            id,
            span: self.span_from(start),
            kind,
        })
    }

    /// `ƒ tên<…>(tham số) → kiểu ! hiệu_ứng ⊳ … ⊲ …` (không gồm thân hay `;`).
    fn parse_fn_sig(&mut self) -> PResult<FnSig> {
        self.expect(&TokenKind::KwFn, "`ƒ`")?;
        let name = self.parse_ident("tên hàm")?;
        let generics = self.parse_generics_opt()?;
        self.expect(&TokenKind::LParen, "`(`")?;
        let mut params = Vec::new();
        loop {
            if self.at(&TokenKind::RParen) {
                break;
            }
            let first = params.is_empty();
            params.push(self.parse_param(first)?);
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(&TokenKind::RParen, "`)`")?;
        let ret = if self.eat(&TokenKind::Arrow) {
            Some(self.parse_type()?)
        } else {
            None
        };
        let effects = self.parse_effects()?;
        let mut contracts = Vec::new();
        while matches!(
            self.tok(),
            TokenKind::Pre | TokenKind::Post | TokenKind::Dec
        ) {
            let cstart = self.start();
            let ckind = match self.tok() {
                TokenKind::Pre => ContractKind::Pre,
                TokenKind::Post => ContractKind::Post,
                _ => ContractKind::Variant,
            };
            self.bump();
            let expr = self.parse_expr_ns()?;
            let id = self.id();
            contracts.push(Contract {
                id,
                span: self.span_from(cstart),
                kind: ckind,
                expr,
            });
        }
        Ok(FnSig {
            name,
            generics,
            params,
            ret,
            effects,
            contracts,
        })
    }

    fn parse_struct(&mut self) -> PResult<ItemKind> {
        self.bump();
        let name = self.parse_ident("tên struct")?;
        let generics = self.parse_generics_opt()?;
        self.expect(&TokenKind::LBrace, "`{`")?;
        let mut fields = Vec::new();
        loop {
            if self.at(&TokenKind::RBrace) {
                break;
            }
            let fstart = self.start();
            let docs = self.parse_docs();
            let vis = if self.eat(&TokenKind::KwPub) {
                Visibility::Public
            } else {
                Visibility::Private
            };
            let fname = self.parse_ident("tên trường")?;
            self.expect(&TokenKind::Colon, "`:`")?;
            let ty = self.parse_type()?;
            let id = self.id();
            fields.push(FieldDef {
                id,
                span: self.span_from(fstart),
                docs,
                vis,
                name: fname,
                ty,
            });
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(&TokenKind::RBrace, "`}`")?;
        Ok(ItemKind::Struct(StructItem {
            name,
            generics,
            fields,
        }))
    }

    fn parse_enum(&mut self) -> PResult<ItemKind> {
        self.bump();
        let name = self.parse_ident("tên enum")?;
        let generics = self.parse_generics_opt()?;
        self.expect(&TokenKind::LBrace, "`{`")?;
        let mut variants = Vec::new();
        loop {
            if self.at(&TokenKind::RBrace) {
                break;
            }
            let vstart = self.start();
            let docs = self.parse_docs();
            let vname = self.parse_ident("tên biến thể")?;
            let mut fields = Vec::new();
            if self.eat(&TokenKind::LParen) {
                loop {
                    if self.at(&TokenKind::RParen) {
                        break;
                    }
                    fields.push(self.parse_type()?);
                    if !self.eat(&TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(&TokenKind::RParen, "`)`")?;
            }
            let id = self.id();
            variants.push(Variant {
                id,
                span: self.span_from(vstart),
                docs,
                name: vname,
                fields,
            });
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(&TokenKind::RBrace, "`}`")?;
        Ok(ItemKind::Enum(EnumItem {
            name,
            generics,
            variants,
        }))
    }

    fn parse_trait(&mut self) -> PResult<ItemKind> {
        self.bump();
        let name = self.parse_ident("tên trait")?;
        let generics = self.parse_generics_opt()?;
        self.expect(&TokenKind::LBrace, "`{`")?;
        let saved = std::mem::replace(&mut self.in_assoc, true);
        let mut members = Vec::new();
        let r = (|| -> PResult<()> {
            loop {
                if self.at(&TokenKind::RBrace) {
                    break;
                }
                let mstart = self.start();
                let docs = self.parse_docs();
                let sig = self.parse_fn_sig()?;
                self.expect(&TokenKind::Semi, "`;` sau chữ ký trong trait")?;
                let id = self.id();
                members.push(TraitMember {
                    id,
                    span: self.span_from(mstart),
                    docs,
                    sig,
                });
            }
            Ok(())
        })();
        self.in_assoc = saved;
        r?;
        self.expect(&TokenKind::RBrace, "`}`")?;
        Ok(ItemKind::Trait(TraitItem {
            name,
            generics,
            members,
        }))
    }

    fn parse_impl(&mut self) -> PResult<ItemKind> {
        self.bump();
        let generics = self.parse_generics_opt()?;
        let first_start = self.start();
        let first = self.parse_type()?;
        let (trait_path, self_ty) = if self.eat(&TokenKind::KwFor) {
            let trait_path = match first.kind {
                TypeKind::Path {
                    path,
                    lifetimes,
                    args,
                } if args.is_empty() && lifetimes.is_empty() => path,
                _ => {
                    let span = self.span_from(first_start);
                    return Err(
                        self.error_at(span, "trước `for` phải là đường dẫn trait".to_string())
                    );
                }
            };
            (Some(trait_path), self.parse_type()?)
        } else {
            (None, first)
        };
        self.expect(&TokenKind::LBrace, "`{`")?;
        let saved = std::mem::replace(&mut self.in_assoc, true);
        let mut members = Vec::new();
        let r = (|| -> PResult<()> {
            loop {
                if self.at(&TokenKind::RBrace) {
                    break;
                }
                let mstart = self.start();
                let docs = self.parse_docs();
                let sig = self.parse_fn_sig()?;
                let body = self.parse_block()?;
                let id = self.id();
                members.push(ImplMember {
                    id,
                    span: self.span_from(mstart),
                    docs,
                    func: FnItem { sig, body },
                });
            }
            Ok(())
        })();
        self.in_assoc = saved;
        r?;
        self.expect(&TokenKind::RBrace, "`}`")?;
        Ok(ItemKind::Impl(ImplItem {
            generics,
            trait_path,
            self_ty,
            members,
        }))
    }

    fn parse_mod(&mut self) -> PResult<ItemKind> {
        self.bump();
        let name = self.parse_ident("tên module")?;
        self.expect(&TokenKind::LBrace, "`{`")?;
        self.enter()?;
        let mut items = Vec::new();
        self.parse_items_until(&mut items, true);
        self.leave();
        self.expect(&TokenKind::RBrace, "`}`")?;
        Ok(ItemKind::Mod(ModItem { name, items }))
    }

    fn parse_use(&mut self) -> PResult<ItemKind> {
        self.bump();
        let path = self.parse_path_for_use()?;
        let mut names = Vec::new();
        if self.at(&TokenKind::ColonColon) && self.peek_is(1, &TokenKind::LBrace) {
            self.bump();
            self.bump();
            loop {
                names.push(self.parse_ident("tên")?);
                if !self.eat(&TokenKind::Comma) || self.at(&TokenKind::RBrace) {
                    break;
                }
            }
            self.expect(&TokenKind::RBrace, "`}`")?;
        }
        self.expect(&TokenKind::Semi, "`;`")?;
        Ok(ItemKind::Use(UseItem { path, names }))
    }

    fn parse_type_alias(&mut self) -> PResult<ItemKind> {
        self.bump();
        let name = self.parse_ident("tên bí danh kiểu")?;
        let generics = self.parse_generics_opt()?;
        self.expect(&TokenKind::Walrus, "`≔`")?;
        let ty = self.parse_type()?;
        self.expect(&TokenKind::Semi, "`;`")?;
        Ok(ItemKind::TypeAlias(TypeAliasItem { name, generics, ty }))
    }

    fn parse_const_like(&mut self, _what: &str) -> PResult<(Ident, Type, Expr)> {
        self.bump();
        let name = self.parse_ident("tên")?;
        self.expect(&TokenKind::Colon, "`:`")?;
        let ty = self.parse_type()?;
        self.expect(&TokenKind::Walrus, "`≔`")?;
        let value = self.parse_expr()?;
        self.expect(&TokenKind::Semi, "`;`")?;
        Ok((name, ty, value))
    }

    // ---------- đường dẫn ----------

    /// `Ident(::Ident)*`, hoặc bắt đầu bằng `Self` (kiểu) / `self` (giá trị, khi `allow_self_value`).
    pub fn parse_path(&mut self, allow_self_value: bool) -> PResult<Path> {
        let start = self.start();
        let mut segments = Vec::new();
        loop {
            let sstart = self.start();
            let kind = match self.tok() {
                TokenKind::Ident(n) => SegmentKind::Name(n.clone()),
                TokenKind::KwSelfTy if segments.is_empty() => SegmentKind::SelfType,
                TokenKind::KwSelfKw if segments.is_empty() && allow_self_value => {
                    SegmentKind::SelfValue
                }
                _ => return Err(self.expected("tên")),
            };
            self.bump();
            let id = self.id();
            segments.push(PathSegment {
                id,
                span: self.span_from(sstart),
                kind,
            });
            if self.at(&TokenKind::ColonColon) && matches!(self.peek_kind(1), TokenKind::Ident(_)) {
                self.bump();
            } else {
                break;
            }
        }
        let id = self.id();
        Ok(Path {
            id,
            span: self.span_from(start),
            segments,
        })
    }

    fn parse_path_for_use(&mut self) -> PResult<Path> {
        // như parse_path nhưng dừng trước `::{`
        self.parse_path(false)
    }
}

#[cfg(test)]
mod push_diag_tests {
    use super::*;

    /// FAIL_OPEN_AUDIT S-C: `push_diag` với mã sai định dạng không được bỏ chẩn đoán.
    #[test]
    fn push_diag_with_a_malformed_code_still_records_a_diagnostic() {
        let mut p = Parser::new(gt_span::FileId::new(0), "t.gt", 0, Vec::new());
        p.push_diag("E01", Span::new(0, 0), "thử".to_string());
        assert_eq!(p.diags.len(), 1, "chẩn đoán bị nuốt");
        assert_eq!(p.diags[0].code.as_str(), "E9901");
    }
}
