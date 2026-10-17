#![forbid(unsafe_code)]
//! Bộ quét secret (M9, ADR-028 §7; GRIT_PLAN mục 5) — mức VÀNG (`S…`), vẫn cho chạy, in cảnh báo; `--deny-yellow` nâng thành ĐỎ.
//!
//! * **S101** — secret nằm trong nguồn theo MẪU nhận dạng: khoá cloud (AWS `AKIA…`/`ASIA…`), token VCS (`ghp_…`, `github_pat_…`), Slack
//!   (`xox?-…`), khoá Google (`AIza…`), Stripe (`sk_live_…`), khối khoá riêng PEM, JWT. Quét literal chuỗi VÀ comment.
//! * **S102** — literal chuỗi entropy cao gán cho tên giống `key/token/secret/password/passwd/pwd/api`.
//! * **S201** — gọi `declassify` trên `Secret<T>` (D13: ghi nhận và báo VÀNG).
//! * **S202** — giá trị lấy từ `Secret<T>` (hoặc kết quả `declassify`) chảy tới lời gọi có hiệu ứng `io` (sink log/mạng/tệp). Taint
//!   không theo luồng, theo THÂN HÀM (bảo thủ: có thể dương tính giả); không xuyên hàm.
//!
//! `#allow(Sxxx, "lý do")` đặt trước item tắt cảnh báo trong phạm vi item. Lý do BẮT BUỘC, không rỗng; mã phải là mã VÀNG đã biết, nếu
//! không ⇒ **E1401** (ĐỎ, không có cách tắt); `#allow` không dùng tới ⇒ **N1401** (GHI CHÚ). Mọi `#allow` đã áp dụng được báo lại để kiểm toán.
//!
//! **Giới hạn nói thẳng:** không phải bộ quét đầy đủ; mẫu và ngưỡng entropy là kinh nghiệm, có dương tính giả và âm tính giả (secret mã hoá
//! base64 lồng, ghép chuỗi lúc chạy, tên biến không gợi ý…).
use gt_ast::{AttrArgKind, Item, ItemKind, NodeId};
use gt_diag::{Code, Diagnostic, Diagnostics, Report};
use gt_lexer::{TokenKind, TriviaKind, lex};
use gt_mir::{Body, BodyKind, Callee, Operand, Rvalue, StatementKind, TerminatorKind};
use gt_span::Span;
use gt_types::{Lang, Ty, TypesOutput};
use std::collections::BTreeSet;

const LIMIT: usize = 200;

/// Mã VÀNG đã biết (chỉ những mã này được phép trong `#allow`).
pub const YELLOW: [&str; 4] = ["S101", "S102", "S201", "S202"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub code: &'static str,
    pub file: String,
    pub span: Span,
    pub message: String,
}

/// Một `#allow` đã áp dụng (dấu vết kiểm toán).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Allowed {
    pub code: String,
    pub reason: String,
    pub file: String,
    pub span: Span,
}

#[derive(Debug)]
pub struct SecretsOutput {
    /// S101… còn lại sau `#allow`, E1401, N1401
    pub report: Report,
    pub allowed: Vec<Allowed>,
}

// ------------------------------------------------------------------ S101 / S102 (văn bản)

fn is_alnum(c: char) -> bool {
    c.is_ascii_alphanumeric()
}

/// Có `n` ký tự liên tiếp thoả `ok` bắt đầu từ `s`?
fn run_at(s: &[char], i: usize, n: usize, ok: impl Fn(char) -> bool) -> bool {
    i + n <= s.len() && s[i..i + n].iter().all(|c| ok(*c))
}

fn starts_with_at(s: &[char], i: usize, p: &str) -> bool {
    let pc: Vec<char> = p.chars().collect();
    i + pc.len() <= s.len() && s[i..i + pc.len()] == pc[..]
}

/// Tên mẫu nhận dạng đầu tiên khớp ở vị trí bất kỳ trong `text`.
fn known_pattern(text: &str) -> Option<&'static str> {
    let s: Vec<char> = text.chars().collect();
    if text.contains("-----BEGIN ") && text.contains("PRIVATE KEY-----") {
        return Some("khối khoá riêng PEM");
    }
    for i in 0..s.len() {
        if (starts_with_at(&s, i, "AKIA") || starts_with_at(&s, i, "ASIA"))
            && run_at(&s, i + 4, 16, |c| {
                c.is_ascii_uppercase() || c.is_ascii_digit()
            })
        {
            return Some("khoá truy cập AWS");
        }
        for p in ["ghp_", "gho_", "ghu_", "ghs_", "ghr_"] {
            if starts_with_at(&s, i, p) && run_at(&s, i + 4, 36, is_alnum) {
                return Some("token GitHub");
            }
        }
        if starts_with_at(&s, i, "github_pat_")
            && run_at(&s, i + 11, 22, |c| is_alnum(c) || c == '_')
        {
            return Some("token GitHub (fine-grained)");
        }
        if starts_with_at(&s, i, "xox")
            && i + 4 < s.len()
            && "baprs".contains(s[i + 3])
            && s[i + 4] == '-'
            && run_at(&s, i + 5, 10, |c| is_alnum(c) || c == '-')
        {
            return Some("token Slack");
        }
        if starts_with_at(&s, i, "AIza")
            && run_at(&s, i + 4, 35, |c| is_alnum(c) || c == '_' || c == '-')
        {
            return Some("khoá API Google");
        }
        for p in ["sk_live_", "rk_live_"] {
            if starts_with_at(&s, i, p) && run_at(&s, i + 8, 24, is_alnum) {
                return Some("khoá Stripe");
            }
        }
        if starts_with_at(&s, i, "eyJ") {
            // JWT: ba đoạn base64url ngăn bởi '.', hai đoạn đầu bắt đầu bằng `eyJ`
            let tail: String = s[i..]
                .iter()
                .take_while(|c| is_alnum(**c) || **c == '_' || **c == '-' || **c == '.')
                .collect();
            let parts: Vec<&str> = tail.split('.').collect();
            if parts.len() == 3
                && parts[0].len() >= 10
                && parts[1].starts_with("eyJ")
                && parts[1].len() >= 10
                && parts[2].len() >= 10
            {
                return Some("JWT");
            }
        }
    }
    None
}

/// Entropy Shannon (bit/ký tự) của chuỗi.
fn entropy(s: &str) -> f64 {
    let mut counts = std::collections::BTreeMap::new();
    let mut n = 0usize;
    for c in s.chars() {
        *counts.entry(c).or_insert(0usize) += 1;
        n += 1;
    }
    if n == 0 {
        return 0.0;
    }
    counts
        .values()
        .map(|&k| {
            let p = k as f64 / n as f64;
            -p * p.log2()
        })
        .sum()
}

fn secret_name(name: &str) -> Option<bool> {
    let l = name.to_lowercase();
    let pw = l.contains("passw") || l.contains("pwd");
    if pw || l.contains("key") || l.contains("token") || l.contains("secret") || l.contains("api") {
        Some(pw)
    } else {
        None
    }
}

/// Nội dung literal chuỗi giữa hai dấu nháy (văn bản nguồn nguyên văn, chưa giải escape).
fn str_body(tok: &str) -> &str {
    tok.strip_prefix('"')
        .and_then(|t| t.strip_suffix('"'))
        .unwrap_or(tok)
}

/// S101/S102 trên văn bản nguồn của một tệp.
#[must_use]
pub fn scan_text(file: &str, text: &str) -> Vec<Finding> {
    let lx = lex(text);
    let mut out: Vec<Finding> = Vec::new();
    let mut s101_spans: BTreeSet<(u32, u32)> = BTreeSet::new();
    for t in &lx.tokens {
        if let TokenKind::StrLit(v) = &t.kind
            && let Some(what) = known_pattern(str_body(v))
        {
            s101_spans.insert((t.span.start, t.span.end));
            out.push(Finding {
                code: "S101",
                file: file.to_string(),
                span: t.span,
                message: format!("literal chuỗi giống {what}: secret nằm trong nguồn sẽ nằm nguyên văn trong binary"),
            });
        }
    }
    for tr in &lx.trivia {
        if matches!(tr.kind, TriviaKind::LineComment)
            && let Some(what) = known_pattern(&tr.text)
        {
            out.push(Finding {
                code: "S101",
                file: file.to_string(),
                span: tr.span,
                message: format!("comment chứa chuỗi giống {what}"),
            });
        }
    }
    // S102: tên gợi ý secret, tối đa vài token sau là gán (`≔` hoặc `←`) rồi literal chuỗi
    let toks = &lx.tokens;
    for (i, t) in toks.iter().enumerate() {
        let TokenKind::Ident(name) = &t.kind else {
            continue;
        };
        let Some(pw) = secret_name(name) else {
            continue;
        };
        let mut j = i + 1;
        let mut steps = 0;
        while j < toks.len() && steps < 8 {
            if matches!(toks[j].kind, TokenKind::Walrus | TokenKind::Assign) {
                break;
            }
            if matches!(
                toks[j].kind,
                TokenKind::Semi | TokenKind::LBrace | TokenKind::RBrace
            ) {
                j = toks.len();
                break;
            }
            j += 1;
            steps += 1;
        }
        if j + 1 >= toks.len() || !matches!(toks[j].kind, TokenKind::Walrus | TokenKind::Assign) {
            continue;
        }
        let TokenKind::StrLit(v) = &toks[j + 1].kind else {
            continue;
        };
        let span = toks[j + 1].span;
        if s101_spans.contains(&(span.start, span.end)) {
            continue;
        }
        let body = str_body(v);
        let len = body.chars().count();
        let e = entropy(body);
        let has_digit = body.chars().any(|c| c.is_ascii_digit());
        let high = (len >= 12 && e >= 3.3) || (pw && len >= 8 && e >= 2.5 && has_digit);
        if high {
            out.push(Finding {
                code: "S102",
                file: file.to_string(),
                span,
                message: format!(
                    "literal chuỗi entropy cao ({e:.1} bit/ký tự, {len} ký tự) gán cho `{name}`: có thể là secret"
                ),
            });
        }
    }
    out.sort_by(|a, b| (a.span.start, a.span.end, a.code).cmp(&(b.span.start, b.span.end, b.code)));
    out
}

// ------------------------------------------------------------------ S201 / S202 (luồng Secret<T>)

fn ty_has_secret(t: &TypesOutput, ty: &Ty, depth: usize) -> bool {
    if depth > 12 {
        // quá sâu ⇒ KHÔNG biết ⇒ coi là có Secret (cảnh báo bảo thủ), không im lặng (FAIL_OPEN_AUDIT S1)
        return true;
    }
    let sec = t.db.lang.get(&Lang::Secret);
    match ty {
        Ty::Adt(d, args) => {
            Some(d) == sec
                || args.iter().any(|a| ty_has_secret(t, a, depth + 1))
                || t.db.adts.get(d).is_some_and(|adt| match &adt.kind {
                    gt_types::AdtKind::Struct(fs) => {
                        fs.iter().any(|f| ty_has_secret(t, &f.ty, depth + 1))
                    }
                    gt_types::AdtKind::Enum(vs) => vs
                        .iter()
                        .any(|v| v.fields.iter().any(|f| ty_has_secret(t, f, depth + 1))),
                })
        }
        Ty::Tuple(v) => v.iter().any(|a| ty_has_secret(t, a, depth + 1)),
        Ty::Array(a, _) | Ty::Slice(a) | Ty::Ref(_, a) | Ty::RawPtr(_, a) => {
            ty_has_secret(t, a, depth + 1)
        }
        _ => false,
    }
}

fn operand_locals(o: &Operand) -> Option<usize> {
    match o {
        Operand::Copy(p) | Operand::Move(p) => Some(p.local.index()),
        Operand::Const(_) => None,
    }
}

fn rvalue_operands(rv: &Rvalue) -> Vec<&Operand> {
    match rv {
        Rvalue::Use(o)
        | Rvalue::Cast(o, _)
        | Rvalue::CastFits(o, _)
        | Rvalue::UnaryOp(_, o)
        | Rvalue::Repeat(o, _) => vec![o],
        Rvalue::BinaryOp(_, a, b) | Rvalue::CheckedBinaryOp(_, a, b) => vec![a, b],
        Rvalue::Aggregate(_, ops) => ops.iter().collect(),
        _ => Vec::new(),
    }
}

fn place_local_of_rvalue(rv: &Rvalue) -> Option<usize> {
    match rv {
        Rvalue::Ref(_, _, p) | Rvalue::Len(p) | Rvalue::Discriminant(p) => Some(p.local.index()),
        _ => None,
    }
}

/// S201/S202 trên MỘT thân hàm.
fn scan_body(t: &TypesOutput, b: &Body, out: &mut Vec<Finding>) {
    let n = b.locals.len();
    let mut tainted = vec![false; n];
    for (i, d) in b.locals.iter().enumerate() {
        if ty_has_secret(t, &d.ty, 0) {
            tainted[i] = true;
        }
    }
    // không biết hiệu ứng của callee (chữ ký/thành viên/kiểu hàm không tra được) ⇒ coi là CÓ `io` (bảo thủ), không im lặng (FAIL_OPEN_AUDIT R3)
    let unknown = || vec!["io".to_string()];
    let callee_effects = |c: &Callee| -> Vec<String> {
        match c {
            Callee::Fn { node, .. } => {
                t.db.sig_by_node(*node)
                    .map(|s| s.effects.clone())
                    .unwrap_or_else(unknown)
            }
            Callee::TraitMethod {
                trait_def, member, ..
            } => {
                t.db.traits
                    .get(trait_def)
                    .and_then(|td| td.members.get(*member))
                    .map(|m| m.effects.clone())
                    .unwrap_or_else(unknown)
            }
            Callee::Value(o) => match gt_mir::types::operand_ty(b, o) {
                Some(Ty::Fn(_, _, e)) => e,
                _ => unknown(),
            },
        }
    };
    let is_declassify = |c: &Callee| -> bool {
        let Callee::Fn { node, .. } = c else {
            return false;
        };
        let Some(sig) = t.db.sig_by_node(*node) else {
            return false;
        };
        if sig.name != "declassify" {
            return false;
        }
        // người nhận/đối số đầu là `Secret<_>` (theo impl chứa thành viên)
        t.db.impls.iter().any(|i| {
            i.members.iter().any(|m| m.node == *node)
                && matches!(&i.self_ty, Ty::Adt(d, _) if Some(d) == t.db.lang.get(&Lang::Secret))
        })
    };
    // lan truyền taint tới điểm cố định (không theo luồng điều khiển: bảo thủ)
    loop {
        let mut changed = false;
        for bb in &b.blocks {
            for s in &bb.statements {
                if let StatementKind::Assign(dst, rv) = &s.kind {
                    let src = rvalue_operands(rv)
                        .into_iter()
                        .filter_map(operand_locals)
                        .chain(place_local_of_rvalue(rv))
                        .any(|l| tainted[l]);
                    if src && !tainted[dst.local.index()] {
                        tainted[dst.local.index()] = true;
                        changed = true;
                    }
                }
            }
            if let Some(TerminatorKind::Call {
                callee, args, dest, ..
            }) = bb.terminator.as_ref().map(|x| &x.kind)
            {
                let any = args.iter().filter_map(operand_locals).any(|l| tainted[l])
                    || is_declassify(callee);
                if any && !tainted[dest.local.index()] {
                    tainted[dest.local.index()] = true;
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    let file = t.file_name(b.node);
    for bb in &b.blocks {
        let Some(term) = &bb.terminator else { continue };
        let TerminatorKind::Call { callee, args, .. } = &term.kind else {
            continue;
        };
        if is_declassify(callee) {
            out.push(Finding {
                code: "S201",
                file: file.clone(),
                span: term.source.span,
                message: format!("`declassify` trên `Secret<…>` trong `{}`: giá trị nhạy cảm bị lộ ra khỏi kiểu bảo vệ", b.key),
            });
        }
        if callee_effects(callee).iter().any(|e| e == "io")
            && args.iter().filter_map(operand_locals).any(|l| tainted[l])
        {
            out.push(Finding {
                code: "S202",
                file: file.clone(),
                span: term.source.span,
                message: format!("giá trị nhạy cảm (từ `Secret<…>`) chảy tới lời gọi có hiệu ứng `io` trong `{}`", b.key),
            });
        }
    }
}

/// S201/S202 trên mọi thân hàm.
#[must_use]
pub fn scan_flow(t: &TypesOutput, m: &gt_mir::MirOutput) -> Vec<Finding> {
    let mut out = Vec::new();
    for b in m.bodies.iter().filter(|b| b.kind == BodyKind::Fn) {
        scan_body(t, b, &mut out);
    }
    out.sort_by(|a, b| {
        (&a.file, a.span.start, a.span.end, a.code).cmp(&(
            &b.file,
            b.span.start,
            b.span.end,
            b.code,
        ))
    });
    out.dedup();
    out
}

// ------------------------------------------------------------------ #allow

struct AllowRec {
    code: String,
    reason: String,
    file: String,
    item_span: Span,
    attr_span: Span,
    node: NodeId,
    used: bool,
}

fn collect_allows(
    t: &TypesOutput,
    items: &[Item],
    recs: &mut Vec<AllowRec>,
    bag: &mut Diagnostics,
) {
    for it in items {
        let file = t.file_name(it.id);
        for a in it.attrs.iter().filter(|a| a.name.name == "allow") {
            let bad = |why: &str, bag: &mut Diagnostics| {
                let c = Code::parse("E1401")
                    .unwrap_or_else(|_| Code::parse("E9901").expect("E9901 hợp lệ"));
                bag.push(Diagnostic::new(
                    c,
                    &file,
                    a.span,
                    format!("`#allow` không hợp lệ: {why}"),
                ));
            };
            let code = match a.args.first().map(|x| &x.kind) {
                Some(AttrArgKind::Ident(c)) => c.clone(),
                _ => {
                    bad("tham số đầu phải là mã cảnh báo (vd `S101`)", bag);
                    continue;
                }
            };
            if !YELLOW.contains(&code.as_str()) {
                bad(
                    &format!("`{code}` không phải mã VÀNG đã biết (ĐỎ không có cách tắt)"),
                    bag,
                );
                continue;
            }
            let reason = match a.args.get(1).map(|x| &x.kind) {
                Some(AttrArgKind::Str(r)) if !r.trim().is_empty() => r.trim().to_string(),
                _ => {
                    bad(
                        "thiếu lý do: `#allow(Sxxx, \"lý do\")` — lý do là chuỗi KHÔNG rỗng",
                        bag,
                    );
                    continue;
                }
            };
            if a.args.len() > 2 {
                bad("thừa tham số", bag);
                continue;
            }
            recs.push(AllowRec {
                code,
                reason,
                file: file.clone(),
                item_span: it.span,
                attr_span: a.span,
                node: it.id,
                used: false,
            });
        }
        match &it.kind {
            ItemKind::Mod(m) => collect_allows(t, &m.items, recs, bag),
            ItemKind::Impl(_) | ItemKind::Fn(_) => {}
            _ => {}
        }
    }
}

/// Chạy bộ quét đầy đủ: S101/S102 trên `sources` (tên tệp, văn bản), S201/S202 trên MIR, áp `#allow`.
#[must_use]
pub fn check(
    t: &TypesOutput,
    m: &gt_mir::MirOutput,
    sources: &[(String, String)],
) -> SecretsOutput {
    let mut bag = Diagnostics::new(LIMIT);
    let mut findings: Vec<Finding> = Vec::new();
    for (name, text) in sources {
        findings.extend(scan_text(name, text));
    }
    findings.extend(scan_flow(t, m));
    let mut recs: Vec<AllowRec> = Vec::new();
    for p in &t.resolve.programs {
        collect_allows(t, &p.items, &mut recs, &mut bag);
    }
    let mut allowed: Vec<Allowed> = Vec::new();
    for f in findings {
        let hit = recs.iter_mut().find(|r| {
            r.code == f.code
                && r.file == f.file
                && r.item_span.start <= f.span.start
                && f.span.end <= r.item_span.end
        });
        if let Some(r) = hit {
            r.used = true;
            allowed.push(Allowed {
                code: f.code.to_string(),
                reason: r.reason.clone(),
                file: f.file.clone(),
                span: f.span,
            });
            continue;
        }
        let c = Code::parse(f.code).unwrap_or_else(|_| Code::parse("E9901").expect("E9901 hợp lệ"));
        bag.push(Diagnostic::new(c, &f.file, f.span, f.message));
    }
    for r in &recs {
        if !r.used {
            let c = Code::parse("N1401")
                .unwrap_or_else(|_| Code::parse("E9901").expect("E9901 hợp lệ"));
            bag.push(Diagnostic::new(
                c,
                &r.file,
                r.attr_span,
                format!(
                    "`#allow({})` không tắt cảnh báo nào trong phạm vi item (lý do: {})",
                    r.code, r.reason
                ),
            ));
        }
        let _ = r.node;
    }
    SecretsOutput {
        report: bag.finish(),
        allowed,
    }
}
