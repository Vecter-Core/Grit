//! Lexer hai bề mặt (canonical Unicode + alias ASCII). Đặc tả: docs/spec/{symbols.md, semantics.md §12–§15, §19, §24}.
//!
//! Nguyên tắc: không bao giờ panic, mỗi vòng lặp luôn tiến lên ít nhất một byte, lỗi không dừng lexer (D19).
use crate::error::{ErrorCode, LexError};
use crate::token::{SYMBOLS, Token, TokenKind, Trivia, TriviaKind};
use gt_span::Span;
use unicode_normalization::UnicodeNormalization;
use unicode_script::Script;

/// Kết quả của một lần lex: chuỗi token (luôn kết thúc bằng `Eof`) và danh sách lỗi (đã sắp xếp theo vị trí).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LexOutput {
    pub tokens: Vec<Token>,
    pub errors: Vec<LexError>,
    /// Comment `//` theo thứ tự nguồn (không nằm trong luồng token; `gt-fmt` dùng để gắn lại comment).
    pub trivia: Vec<Trivia>,
}

impl LexOutput {
    /// Không có lỗi nào.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Chuỗi token dạng văn bản (`NAME` hoặc `NAME(giá trị)`), bỏ `Eof` — định dạng của `tests/lexer/*.tokens`.
#[must_use]
pub fn render_tokens(out: &LexOutput) -> Vec<String> {
    out.tokens
        .iter()
        .filter(|t| t.kind != TokenKind::Eof)
        .map(|t| t.kind.render())
        .collect()
}

/// Lex một tệp nguồn theo edition 1 (hành vi đóng băng của Seed).
#[must_use]
pub fn lex(source: &str) -> LexOutput {
    lex_with_edition(source, 1)
}

/// Lex theo `edition` (ADR-P2-01/09). Edition ≥ 2 thêm token `?` đứng riêng (`Question`); edition 1 giữ nguyên: `?` đứng riêng là `E0101`.
/// `+?` `-?` `*?` luôn là một token (khớp dài nhất) ở mọi edition.
#[must_use]
pub fn lex_with_edition(source: &str, edition: u32) -> LexOutput {
    let mut lx = Lexer {
        edition,
        src: source,
        pos: 0,
        tokens: Vec::new(),
        errors: Vec::new(),
        trivia: Vec::new(),
        last_dot_end: None,
    };
    if u32::try_from(source.len()).is_err() {
        lx.error(
            ErrorCode::E0101,
            "tệp nguồn quá lớn (tối đa 4 GiB − 1)",
            0,
            0,
        );
        lx.tokens.push(Token {
            kind: TokenKind::Eof,
            span: Span::new(0, 0),
        });
        return LexOutput {
            tokens: lx.tokens,
            errors: lx.errors,
            trivia: lx.trivia,
        };
    }
    lx.prescan();
    lx.run();
    let end = lx.span(source.len(), source.len());
    lx.tokens.push(Token {
        kind: TokenKind::Eof,
        span: end,
    });
    lx.errors
        .sort_by_key(|e| (e.span.start, e.span.end, e.code));
    LexOutput {
        tokens: lx.tokens,
        errors: lx.errors,
        trivia: lx.trivia,
    }
}

// ---------- phân loại ký tự ----------

/// Ký tự điều khiển bidi (D15, E1201). Gồm U+202A–202E, U+2066–2069 và các dấu bidi LRM/RLM/ALM.
fn is_bidi(c: char) -> bool {
    matches!(c, '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{200E}' | '\u{200F}' | '\u{061C}')
}

/// Ký tự vô hình / zero-width / định dạng (D15, E1202).
fn is_invisible(c: char) -> bool {
    matches!(
        c,
        '\u{00AD}'
            | '\u{034F}'
            | '\u{115F}'
            | '\u{1160}'
            | '\u{17B4}'
            | '\u{17B5}'
            | '\u{180B}'..='\u{180E}'
            | '\u{200B}'..='\u{200D}'
            | '\u{2060}'..='\u{2064}'
            | '\u{3164}'
            | '\u{FE00}'..='\u{FE0F}'
            | '\u{FEFF}'
            | '\u{FFA0}'
            | '\u{E0001}'
            | '\u{E0020}'..='\u{E007F}'
            | '\u{E0100}'..='\u{E01EF}'
    )
}

/// Ký tự bị cấm ở mọi nơi (bidi, vô hình, điều khiển trừ tab/xuống dòng, khoảng trắng lạ). `\r` xử lý riêng.
fn is_forbidden(c: char) -> bool {
    if matches!(c, ' ' | '\t' | '\n' | '\r') {
        return false;
    }
    is_bidi(c) || is_invisible(c) || c.is_control() || c.is_whitespace()
}

/// `μ` và `ƒ` là ký hiệu dành riêng nên không bao giờ nằm trong định danh (Q5).
fn is_reserved_letter(c: char) -> bool {
    matches!(c, 'μ' | 'ƒ')
}

fn is_ident_start(c: char) -> bool {
    (unicode_ident::is_xid_start(c) || c == '_') && !is_reserved_letter(c) && !is_forbidden(c)
}

fn is_ident_continue(c: char) -> bool {
    unicode_ident::is_xid_continue(c) && !is_reserved_letter(c) && !is_forbidden(c)
}

/// Hai script khác nhau (không kể Common/Inherited/Unknown) trong cùng một định danh (E1203).
fn mixed_script(text: &str) -> bool {
    let mut seen: Option<Script> = None;
    for c in text.chars() {
        let s = Script::from(c);
        if matches!(s, Script::Common | Script::Inherited | Script::Unknown) {
            continue;
        }
        match seen {
            None => seen = Some(s),
            Some(prev) if prev != s => return true,
            Some(_) => {}
        }
    }
    false
}

fn int_suffix(s: &str) -> bool {
    matches!(
        s,
        "i8" | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
    )
}

fn alias_kind(word: &str) -> Option<TokenKind> {
    match word {
        "extern" => return Some(TokenKind::KwExtern),
        "where" => return Some(TokenKind::KwWhere),
        _ => {}
    }
    if word.is_empty() {
        return None;
    }
    SYMBOLS
        .iter()
        .find(|(_, ascii, _)| ascii.strip_prefix('\\') == Some(word))
        .map(|(_, _, k)| k.clone())
}

fn symbol_kind(c: char) -> Option<TokenKind> {
    SYMBOLS
        .iter()
        .find(|(canon, _, _)| *canon == c)
        .map(|(_, _, k)| k.clone())
}

fn keyword_kind(text: &str) -> TokenKind {
    match text {
        "self" => TokenKind::KwSelfKw,
        "Self" => TokenKind::KwSelfTy,
        "as" => TokenKind::KwAs,
        "for" => TokenKind::KwFor,
        "in" => TokenKind::KwIn,
        "true" => TokenKind::KwTrue,
        "false" => TokenKind::KwFalse,
        "_" => TokenKind::Underscore,
        _ => TokenKind::Ident(text.to_string()),
    }
}

// ---------- lexer ----------

struct Lexer<'a> {
    /// edition của ngôn ngữ (ADR-P2-01): quyết định tập token (hiện chỉ `?` đứng riêng ở edition ≥ 2)
    edition: u32,
    src: &'a str,
    pos: usize,
    tokens: Vec<Token>,
    errors: Vec<LexError>,
    /// Vị trí ngay sau token `.` gần nhất — để đọc chỉ số tuple `t.0.1` (số thập phân thuần).
    last_dot_end: Option<usize>,
    trivia: Vec<Trivia>,
}

impl Lexer<'_> {
    fn span(&self, start: usize, end: usize) -> Span {
        let u = |x: usize| u32::try_from(x).unwrap_or(u32::MAX);
        Span::new(u(start), u(end))
    }

    fn error(&mut self, code: ErrorCode, message: &str, start: usize, end: usize) {
        let span = self.span(start, end);
        self.errors.push(LexError {
            code,
            message: message.to_string(),
            span,
        });
    }

    /// Báo lỗi và phát token `UNKNOWN` phủ đoạn `[start, pos)` — điểm neo để parser khôi phục lỗi.
    fn bad(&mut self, code: ErrorCode, message: &str, start: usize) {
        let end = self.pos;
        self.error(code, message, start, end);
        self.push(TokenKind::Unknown, start);
    }

    fn push(&mut self, kind: TokenKind, start: usize) {
        let span = self.span(start, self.pos);
        self.tokens.push(Token { kind, span });
    }

    fn rest(&self) -> &str {
        &self.src[self.pos..]
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn peek_nth(&self, n: usize) -> Option<char> {
        self.rest().chars().nth(n)
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    /// Quét cả tệp một lần: bidi/vô hình/điều khiển (E1201/E1202) và NFC (E1204). Áp dụng ở MỌI nơi, kể cả chuỗi và comment.
    fn prescan(&mut self) {
        let mut chars = self.src.char_indices().peekable();
        while let Some((i, c)) = chars.next() {
            let end = i + c.len_utf8();
            if c == '\r' {
                if chars.peek().map(|&(_, n)| n) != Some('\n') {
                    self.error(
                        ErrorCode::E1202,
                        "ký tự '\\r' lẻ (chỉ chấp nhận \\r\\n)",
                        i,
                        end,
                    );
                }
            } else if is_bidi(c) {
                self.error(
                    ErrorCode::E1201,
                    "ký tự điều khiển bidi bị cấm (Trojan Source)",
                    i,
                    end,
                );
            } else if is_forbidden(c) {
                self.error(
                    ErrorCode::E1202,
                    "ký tự vô hình/điều khiển/khoảng trắng lạ bị cấm",
                    i,
                    end,
                );
            }
        }
        if !unicode_normalization::is_nfc(self.src) {
            let (start, end) = self
                .src
                .char_indices()
                .zip(self.src.nfc())
                .find(|((_, a), b)| a != b)
                .map_or(
                    (0, self.src.chars().next().map_or(0, char::len_utf8)),
                    |((i, a), _)| (i, i + a.len_utf8()),
                );
            self.error(
                ErrorCode::E1204,
                "nguồn không phải NFC (chuẩn hoá Unicode)",
                start,
                end,
            );
        }
    }

    fn run(&mut self) {
        while let Some(c) = self.peek() {
            let start = self.pos;
            match c {
                ' ' | '\t' | '\n' | '\r' => {
                    self.bump();
                }
                // đã báo ở prescan; bỏ qua để lexer tiếp tục
                c if is_forbidden(c) => {
                    self.bump();
                }
                // MICRO SIGN trông giống μ (ký hiệu dành riêng) — chặn như homoglyph
                '\u{00B5}' => {
                    self.bump();
                    self.bad(
                        ErrorCode::E1203,
                        "'µ' (U+00B5) trông giống 'μ' (ký hiệu dành riêng KW_MUT)",
                        start,
                    );
                }
                '/' => self.slash(start),
                '"' => self.string(start),
                '\'' => self.quote(start),
                '\\' => self.backslash(start),
                d if d.is_ascii_digit() => self.number(start),
                c if is_ident_start(c) => self.ident(start),
                c => {
                    if let Some(kind) = symbol_kind(c) {
                        self.bump();
                        self.push(kind, start);
                    } else if matches!(
                        c,
                        '(' | ')'
                            | '{'
                            | '}'
                            | '['
                            | ']'
                            | ','
                            | ';'
                            | ':'
                            | '.'
                            | '#'
                            | '-'
                            | '+'
                            | '*'
                            | '<'
                            | '>'
                            | '='
                            | '!'
                            | '&'
                            | '%'
                    ) {
                        self.op(c, start);
                    } else if c == '?' && self.edition >= 2 {
                        self.single(TokenKind::Question, start);
                    } else {
                        self.bump();
                        self.bad(ErrorCode::E0101, "ký tự không thuộc ngôn ngữ Grit", start);
                    }
                }
            }
        }
    }

    // ----- toán tử & dấu câu -----

    fn single(&mut self, kind: TokenKind, start: usize) {
        self.bump();
        self.push(kind, start);
    }

    fn double(&mut self, kind: TokenKind, start: usize) {
        self.bump();
        self.bump();
        self.push(kind, start);
    }

    fn op(&mut self, c: char, start: usize) {
        use TokenKind::*;
        let n1 = self.peek_nth(1);
        match c {
            '(' => self.single(LParen, start),
            ')' => self.single(RParen, start),
            '{' => self.single(LBrace, start),
            '}' => self.single(RBrace, start),
            '[' => self.single(LBracket, start),
            ']' => self.single(RBracket, start),
            ',' => self.single(Comma, start),
            ';' => self.single(Semi, start),
            '#' => self.single(Hash, start),
            '%' => self.single(Percent, start),
            ':' => match n1 {
                Some(':') => self.double(ColonColon, start),
                Some('=') => self.double(Walrus, start),
                _ => self.single(Colon, start),
            },
            '.' => {
                if self.rest().starts_with("..=") {
                    self.bump();
                    self.bump();
                    self.bump();
                    self.push(DotDotEq, start);
                } else if self.rest().starts_with("..") {
                    self.double(DotDot, start);
                } else {
                    self.single(Dot, start);
                    self.last_dot_end = Some(self.pos);
                }
            }
            '-' => match n1 {
                Some('>') => self.double(Arrow, start),
                Some('%') => self.double(MinusWrap, start),
                Some('^') => self.double(MinusSat, start),
                Some('?') => self.double(MinusChk, start),
                _ => self.single(Minus, start),
            },
            '+' => match n1 {
                Some('%') => self.double(PlusWrap, start),
                Some('^') => self.double(PlusSat, start),
                Some('?') => self.double(PlusChk, start),
                _ => self.single(Plus, start),
            },
            '*' => match n1 {
                Some('/') => {
                    self.bump();
                    self.bump();
                    self.bad(
                        ErrorCode::E0101,
                        "'*/' không hợp lệ: comment khối bị cấm ở Phase 1",
                        start,
                    );
                }
                Some('%') => self.double(StarWrap, start),
                Some('^') => self.double(StarSat, start),
                Some('?') => self.double(StarChk, start),
                _ => self.single(Star, start),
            },
            '<' => match n1 {
                Some('-') => self.double(Assign, start),
                Some('=') => self.double(Leq, start),
                _ => self.single(Lt, start),
            },
            '>' => match n1 {
                Some('=') => self.double(Geq, start),
                _ => self.single(Gt, start),
            },
            '=' => match n1 {
                Some('=') => self.double(EqEq, start),
                Some('>') => self.double(FatArrow, start),
                _ => {
                    self.bump();
                    self.bad(
                        ErrorCode::E0101,
                        "'=' đứng riêng không hợp lệ: gán là '←', so sánh bằng là '=='",
                        start,
                    );
                }
            },
            '!' => match n1 {
                Some('=') => self.double(Neq, start),
                _ => self.single(Bang, start),
            },
            '&' => match n1 {
                Some('!') => self.double(BorrowMut, start),
                _ => self.single(Borrow, start),
            },
            _ => {
                // không thể xảy ra: `run` chỉ gọi `op` với các ký tự đã liệt kê
                self.bump();
                self.bad(ErrorCode::E0101, "ký tự không thuộc ngôn ngữ Grit", start);
            }
        }
    }

    // ----- comment -----

    fn slash(&mut self, start: usize) {
        match self.peek_nth(1) {
            Some('/') => {
                let slashes = self.rest().chars().take_while(|&c| c == '/').count();
                let line_end = self
                    .rest()
                    .find('\n')
                    .map_or(self.src.len(), |i| self.pos + i);
                if slashes == 3 {
                    let body = &self.src[self.pos + 3..line_end];
                    let body = body.strip_suffix('\r').unwrap_or(body);
                    let body = body.strip_prefix(' ').unwrap_or(body);
                    let text = body.to_string();
                    self.pos = line_end;
                    self.push(TokenKind::DocComment(text), start);
                } else {
                    // comment thường: không vào luồng token nhưng PHẢI giữ lại trong bảng trivia (gt-fmt)
                    let raw = &self.src[start..line_end];
                    let text = raw.strip_suffix('\r').unwrap_or(raw).to_string();
                    let line_start = self.src[..start].rfind('\n').map_or(0, |i| i + 1);
                    let own_line = self.src[line_start..start]
                        .chars()
                        .all(|c| c == ' ' || c == '\t');
                    let span = self.span(start, start + text.len());
                    self.trivia.push(Trivia {
                        kind: TriviaKind::LineComment,
                        text,
                        span,
                        own_line,
                    });
                    self.pos = line_end;
                }
            }
            Some('*') => {
                self.bump();
                self.bump();
                self.bad(
                    ErrorCode::E0101,
                    "'/*' không hợp lệ: comment khối bị cấm ở Phase 1 (chỉ có '//' và '///')",
                    start,
                );
            }
            _ => self.single(TokenKind::Slash, start),
        }
    }

    // ----- định danh / từ khoá -----

    fn ident(&mut self, start: usize) {
        self.bump();
        while let Some(c) = self.peek() {
            if is_ident_continue(c) {
                self.bump();
            } else {
                break;
            }
        }
        let text = &self.src[start..self.pos];
        if mixed_script(text) {
            self.error(
                ErrorCode::E1203,
                "định danh trộn nhiều bảng chữ (nghi homoglyph)",
                start,
                self.pos,
            );
        }
        let kind = keyword_kind(text);
        self.push(kind, start);
    }

    fn backslash(&mut self, start: usize) {
        self.bump();
        let word_start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == '_' {
                self.bump();
            } else {
                break;
            }
        }
        let word = &self.src[word_start..self.pos];
        match alias_kind(word) {
            Some(kind) => self.push(kind, start),
            None => self.bad(
                ErrorCode::E0101,
                "bí danh ASCII '\\…' không tồn tại (bảng: docs/spec/symbols.md)",
                start,
            ),
        }
    }

    // ----- số -----

    /// Quét chữ số cơ số `radix`; dấu `_` chỉ được nuốt khi nằm GIỮA hai chữ số. Trả về số chữ số.
    fn scan_digits(&mut self, radix: u32) -> usize {
        let mut n = 0;
        loop {
            match self.peek() {
                Some(c) if c.is_digit(radix) => {
                    self.bump();
                    n += 1;
                }
                Some('_') if n > 0 && self.peek_nth(1).is_some_and(|d| d.is_digit(radix)) => {
                    self.bump();
                }
                _ => break,
            }
        }
        n
    }

    fn number(&mut self, start: usize) {
        // Chỉ số tuple: ngay sau '.' đọc CHỈ chữ số thập phân (t.0.1 = t . 0 . 1), không bao giờ float.
        if self.last_dot_end == Some(start) {
            while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                self.bump();
            }
            let text = self.src[start..self.pos].to_string();
            self.push(TokenKind::IntLit(text), start);
            return;
        }
        let mut radix = 10;
        if self.peek() == Some('0') {
            radix = match self.peek_nth(1) {
                Some('x') => 16,
                Some('o') => 8,
                Some('b') => 2,
                _ => 10,
            };
        }
        let mut is_float = false;
        if radix == 10 {
            self.scan_digits(10);
            if self.peek() == Some('.') && self.peek_nth(1).is_some_and(|c| c.is_ascii_digit()) {
                self.bump();
                self.scan_digits(10);
                is_float = true;
            }
            if matches!(self.peek(), Some('e' | 'E')) {
                let exp_ok = match self.peek_nth(1) {
                    Some(d) if d.is_ascii_digit() => true,
                    Some('+' | '-') => self.peek_nth(2).is_some_and(|d| d.is_ascii_digit()),
                    _ => false,
                };
                if exp_ok {
                    self.bump();
                    if matches!(self.peek(), Some('+' | '-')) {
                        self.bump();
                    }
                    self.scan_digits(10);
                    is_float = true;
                }
            }
        } else {
            self.bump();
            self.bump(); // tiền tố 0x / 0o / 0b
            if self.scan_digits(radix) == 0 {
                self.error(
                    ErrorCode::E0101,
                    "thiếu chữ số sau tiền tố 0x/0o/0b",
                    start,
                    self.pos,
                );
            }
        }
        let suffix_start = self.pos;
        while self
            .peek()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            self.bump();
        }
        let suffix = &self.src[suffix_start..self.pos];
        let text = self.src[start..self.pos].to_string();
        let kind = if suffix.is_empty() {
            Some(if is_float {
                TokenKind::FloatLit(text)
            } else {
                TokenKind::IntLit(text)
            })
        } else if radix == 10 && matches!(suffix, "f32" | "f64") {
            Some(TokenKind::FloatLit(text))
        } else if !is_float && int_suffix(suffix) {
            Some(TokenKind::IntLit(text))
        } else {
            None
        };
        match kind {
            Some(k) => self.push(k, start),
            None => {
                self.bad(ErrorCode::E0101, "hậu tố số không hợp lệ (chấp nhận i8…u128/isize/usize, f32/f64; '_' phải nằm giữa chữ số)", start);
            }
        }
    }

    // ----- chuỗi / ký tự / lifetime -----

    /// Đọc một escape, con trỏ đang ở '\'. Trả về true nếu hợp lệ. Lỗi được báo tại đây.
    fn escape(&mut self) -> bool {
        let start = self.pos;
        self.bump(); // '\'
        match self.peek() {
            Some('n' | 'r' | 't' | '0' | '\\' | '\'' | '"') => {
                self.bump();
                true
            }
            Some('u') => {
                self.bump();
                if self.peek() != Some('{') {
                    self.error(
                        ErrorCode::E0101,
                        "escape '\\u' phải có dạng \\u{hex}",
                        start,
                        self.pos,
                    );
                    return false;
                }
                self.bump();
                let hex_start = self.pos;
                while self.peek().is_some_and(|c| c.is_ascii_hexdigit()) {
                    self.bump();
                }
                let hex = &self.src[hex_start..self.pos];
                let closed = self.peek() == Some('}');
                if closed {
                    self.bump();
                }
                let valid = closed
                    && (1..=6).contains(&hex.len())
                    && u32::from_str_radix(hex, 16)
                        .ok()
                        .and_then(char::from_u32)
                        .is_some();
                if !valid {
                    self.error(
                        ErrorCode::E0101,
                        "escape \\u{…} không hợp lệ (1–6 chữ số hex, phải là Unicode scalar value)",
                        start,
                        self.pos,
                    );
                }
                valid
            }
            Some(c) if c != '\n' && c != '\r' => {
                self.bump();
                self.error(
                    ErrorCode::E0101,
                    "escape không hợp lệ (chấp nhận \\n \\r \\t \\0 \\\\ \\' \\\" \\u{…})",
                    start,
                    self.pos,
                );
                false
            }
            _ => {
                self.error(
                    ErrorCode::E0101,
                    "escape dang dở ở cuối dòng/tệp",
                    start,
                    self.pos,
                );
                false
            }
        }
    }

    fn string(&mut self, start: usize) {
        self.bump(); // '"'
        let content_start = self.pos;
        loop {
            match self.peek() {
                None => {
                    self.bad(ErrorCode::E0101, "chuỗi chưa đóng", start);
                    return;
                }
                Some('"') => {
                    let inner = self.src[content_start..self.pos].to_string();
                    self.bump();
                    self.push(TokenKind::StrLit(inner), start);
                    return;
                }
                Some('\n') => {
                    self.bad(
                        ErrorCode::E0101,
                        "chuỗi không được xuống dòng thô (chưa có chuỗi nhiều dòng)",
                        start,
                    );
                    return;
                }
                Some('\r') if self.peek_nth(1) == Some('\n') => {
                    self.bad(
                        ErrorCode::E0101,
                        "chuỗi không được xuống dòng thô (chưa có chuỗi nhiều dòng)",
                        start,
                    );
                    return;
                }
                Some('\\') => {
                    self.escape();
                }
                Some(_) => {
                    self.bump();
                }
            }
        }
    }

    /// Ký tự `'x'` hay lifetime `'a` (semantics §15): có dấu `'` đóng ngay sau MỘT ký tự = char, ngược lại = lifetime.
    fn quote(&mut self, start: usize) {
        self.bump(); // '\''
        match self.peek() {
            None => self.bad(ErrorCode::E0101, "dấu ' lẻ ở cuối tệp", start),
            Some('\'') => {
                self.bump();
                self.bad(ErrorCode::E0101, "literal ký tự rỗng ''", start);
            }
            Some('\\') => {
                let ok = self.escape();
                let inner_end = self.pos;
                if self.peek() == Some('\'') {
                    self.bump();
                    if ok {
                        let inner = self.src[start + 1..inner_end].to_string();
                        self.push(TokenKind::CharLit(inner), start);
                    } else {
                        // escape hỏng: lỗi đã báo trong `escape()`, chỉ cần token neo
                        self.push(TokenKind::Unknown, start);
                    }
                } else if ok {
                    self.bad(ErrorCode::E0101, "literal ký tự chưa đóng bằng '", start);
                } else {
                    self.push(TokenKind::Unknown, start);
                }
            }
            Some('\n' | '\r') => self.bad(ErrorCode::E0101, "dấu ' lẻ", start),
            Some(c) => {
                if self.peek_nth(1) == Some('\'') {
                    self.bump();
                    self.bump();
                    self.push(TokenKind::CharLit(c.to_string()), start);
                } else if is_ident_start(c) {
                    self.bump();
                    while let Some(n) = self.peek() {
                        if is_ident_continue(n) {
                            self.bump();
                        } else {
                            break;
                        }
                    }
                    let name = &self.src[start + 1..self.pos];
                    if mixed_script(name) {
                        self.error(
                            ErrorCode::E1203,
                            "tên lifetime trộn nhiều bảng chữ (nghi homoglyph)",
                            start,
                            self.pos,
                        );
                    }
                    let name = name.to_string();
                    self.push(TokenKind::Lifetime(name), start);
                } else {
                    self.bad(
                        ErrorCode::E0101,
                        "dấu ' lẻ (không phải char hay lifetime hợp lệ)",
                        start,
                    );
                }
            }
        }
    }
}
