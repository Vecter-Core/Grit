//! C1 Giai đoạn 2 (ADR-P2-06): `rig test --auto` — sinh test từ chữ ký/hợp đồng của hàm CÔNG KHAI. NGOÀI TCB: công cụ chỉ TÌM lỗi (phản ví dụ),
//! không bao giờ cấp phép hay đổi phán quyết của `gritc`; công cụ sai chỉ làm thiếu phản ví dụ hoặc báo thừa một ứng viên, không làm sai phán quyết.
//!
//! Cách làm (không cần bộ tính hợp đồng riêng): với mỗi hàm `◈ ƒ` tự do, thuần (`! ∅`), không generic, mọi tham số là số nguyên/`bool`, sinh các ứng viên đối số
//! HẰNG ở biên (0, 1, −1, 2, 7, 100, max−1, max, min…); mỗi ứng viên là một chương trình `main` gọi hàm với hằng đó và đi qua CHÍNH `gritc`:
//!   * `gritc` báo E0901 (`⊳` luôn sai tại điểm gọi) ⇒ ứng viên NGOÀI miền của hàm ⇒ BỎ (không phải lỗi của hàm);
//!   * `gritc` báo lỗi khác ⇒ BỎ QUA hàm, nói rõ (không chạy được);
//!   * xanh ⇒ chạy trên thông dịch tham chiếu: abort (`postcondition`, `overflow`, `precondition` của hàm được gọi, `index`…) ⇒ PHẢN VÍ DỤ;
//!     hết nhiên liệu ⇒ ghi là "chậm", KHÔNG phải phản ví dụ (mọi vòng `⟳` đều có biến thể `↓` được M8 chứng minh dừng, nên chạy lâu ≠ không dừng).
//!
//! Hàm không lái được (tham số khác nhóm trên, generic, có hiệu ứng) được LIỆT KÊ là "BỎ QUA" kèm lý do — không bao giờ lặng lẽ bỏ.
use gt_ast::{ContractKind, EffectsKind, ItemKind, ParamKind, PrimType, TypeKind, Visibility};
use gt_driver::pipeline::Options;
use gt_driver::{RunEnd, check};
use gt_resolve::SourceInput;
use std::collections::BTreeSet;

/// Nhiên liệu chạy một ứng viên (bộ chạy tham chiếu); hết ⇒ phản ví dụ "vòng lặp dài/không dừng".
const FUEL: u64 = 5_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Int { bits: u32, signed: bool },
    Bool,
    Float,
    Char,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub module: String,
    pub name: String,
    params: Vec<Kind>,
    /// hàm có `⊳` mà `gritc` KHÔNG quyết định được tại điểm gọi bằng hằng (tham số float/char, hoặc `⊳` có lời gọi): abort `precondition` ở cửa vào không phân biệt được với của hàm được gọi
    amb: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Ok {
        ran: usize,
        dropped: usize,
        slow: usize,
        /// abort `precondition` không phân biệt được cửa vào với hàm được gọi (xem `Target::amb`): không tính là phản ví dụ
        ambiguous: usize,
    },
    Failed {
        found: Vec<String>,
        ran: usize,
    },
    Skipped(String),
}

fn kind_of(p: PrimType) -> Option<(Kind, &'static str)> {
    Some(match p {
        PrimType::I8 => (
            Kind::Int {
                bits: 8,
                signed: true,
            },
            "i8",
        ),
        PrimType::I16 => (
            Kind::Int {
                bits: 16,
                signed: true,
            },
            "i16",
        ),
        PrimType::I32 => (
            Kind::Int {
                bits: 32,
                signed: true,
            },
            "i32",
        ),
        PrimType::I64 => (
            Kind::Int {
                bits: 64,
                signed: true,
            },
            "i64",
        ),
        PrimType::I128 => (
            Kind::Int {
                bits: 128,
                signed: true,
            },
            "i128",
        ),
        PrimType::Isize => (
            Kind::Int {
                bits: 64,
                signed: true,
            },
            "isize",
        ),
        PrimType::U8 => (
            Kind::Int {
                bits: 8,
                signed: false,
            },
            "u8",
        ),
        PrimType::U16 => (
            Kind::Int {
                bits: 16,
                signed: false,
            },
            "u16",
        ),
        PrimType::U32 => (
            Kind::Int {
                bits: 32,
                signed: false,
            },
            "u32",
        ),
        PrimType::U64 => (
            Kind::Int {
                bits: 64,
                signed: false,
            },
            "u64",
        ),
        PrimType::U128 => (
            Kind::Int {
                bits: 128,
                signed: false,
            },
            "u128",
        ),
        PrimType::Usize => (
            Kind::Int {
                bits: 64,
                signed: false,
            },
            "usize",
        ),
        PrimType::Bool => (Kind::Bool, "bool"),
        PrimType::F64 => (Kind::Float, "f64"),
        PrimType::Char => (Kind::Char, ""),
        _ => return None,
    })
}

fn suffix_of(p: PrimType) -> &'static str {
    kind_of(p).map_or("", |(_, s)| s)
}

/// Hàm công khai của các tệp gói gốc (`(tên tệp, văn bản)`), kèm lý do nếu không lái được. Tệp không phân tích được ⇒ bỏ qua (gritc sẽ báo ở bước check).
pub fn targets(files: &[(String, String)]) -> Vec<(Target, Option<String>, Vec<PrimType>)> {
    let mut out = Vec::new();
    for (fname, text) in files {
        let module = fname.trim_end_matches(".gt").to_string();
        let parsed = gt_parser::parse(fname, text);
        for item in &parsed.program.items {
            let ItemKind::Fn(f) = &item.kind else {
                continue;
            };
            if item.vis != Visibility::Public {
                continue;
            }
            let mut skip: Option<String> = None;
            let pre_text: Vec<&str> = f
                .sig
                .contracts
                .iter()
                .filter(|c| c.kind == ContractKind::Pre)
                .map(|c| {
                    text.get(c.span.start as usize..c.span.end as usize)
                        .unwrap_or("(")
                })
                .collect();
            let mut kinds = Vec::new();
            let mut prims = Vec::new();
            if f.sig.generics.is_some() {
                skip = Some("hàm generic".to_string());
            }
            if !matches!(f.sig.effects.kind, EffectsKind::Pure) && skip.is_none() {
                skip = Some("hàm có hiệu ứng".to_string());
            }
            for p in &f.sig.params {
                let ParamKind::Named { ty, .. } = &p.kind else {
                    skip.get_or_insert_with(|| "tham số `self`".to_string());
                    continue;
                };
                match &ty.kind {
                    TypeKind::Prim(pt) => match kind_of(*pt) {
                        Some((k, _)) => {
                            kinds.push(k);
                            prims.push(*pt);
                        }
                        None => {
                            skip.get_or_insert_with(|| {
                                format!("tham số kiểu {pt:?} chưa hỗ trợ (chỉ số nguyên, bool)")
                            });
                        }
                    },
                    _ => {
                        skip.get_or_insert_with(|| {
                            "tham số không phải kiểu nguyên thuỷ (chỉ số nguyên, bool)".to_string()
                        });
                    }
                }
            }
            let amb = !pre_text.is_empty()
                && (kinds.iter().any(|k| matches!(k, Kind::Float | Kind::Char))
                    || pre_text.iter().any(|t| t.contains('(')));
            out.push((
                Target {
                    module: module.clone(),
                    name: f.sig.name.name.clone(),
                    params: kinds,
                    amb,
                },
                skip,
                prims,
            ));
        }
    }
    out
}

fn int_lit(suf: &str, v: i128) -> String {
    if v >= 0 {
        format!("{v}{suf}")
    } else {
        format!("-{}{suf}", -v)
    }
}

/// Literal biên cho một kiểu (số nguyên 128 bit giới hạn trong i64/u64 để literal luôn biểu diễn được).
fn literals(k: Kind, suf: &str) -> Vec<String> {
    match k {
        Kind::Bool => vec!["false".to_string(), "true".to_string()],
        Kind::Float => ["0.0", "1.0", "-1.0", "0.5", "1e300", "-1e300", "1e-300"]
            .iter()
            .map(|v| format!("{v}f64"))
            .collect(),
        Kind::Char => ["'a'", "'Z'", "'0'", "' '", "'\\0'", "'é'"]
            .iter()
            .map(|v| (*v).to_string())
            .collect(),
        Kind::Int { bits, signed } => {
            let (lo, hi): (i128, i128) = if signed {
                if bits >= 128 {
                    (-(i64::MAX as i128), i64::MAX as i128)
                } else {
                    (-(1i128 << (bits - 1)) + 1, (1i128 << (bits - 1)) - 1)
                }
            } else if bits >= 128 {
                (0, u64::MAX as i128)
            } else {
                (0, (1i128 << bits) - 1)
            };
            // `min` thật của kiểu có dấu không có literal một dấu `-` an toàn (−min tràn): dùng `min+1`
            let mut v: BTreeSet<i128> = BTreeSet::new();
            for c in [0, 1, 2, 7, 100, hi - 1, hi] {
                if (lo..=hi).contains(&c) {
                    v.insert(c);
                }
            }
            if signed {
                for c in [-1, -2, -7, -100, lo, lo + 1] {
                    if (lo..=hi).contains(&c) {
                        v.insert(c);
                    }
                }
            }
            v.into_iter().map(|x| int_lit(suf, x)).collect()
        }
    }
}

fn fnv(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Tối đa `cap` tổ hợp đối số (tất định theo tên hàm): ít tổ hợp thì lấy hết, nhiều thì lấy mẫu không lặp.
fn combos(vals: &[Vec<String>], cap: usize, seed: &str) -> Vec<Vec<String>> {
    let total: usize = vals.iter().map(Vec::len).product();
    let pick = |mut idx: usize| -> Vec<String> {
        vals.iter()
            .map(|v| {
                let x = v[idx % v.len()].clone();
                idx /= v.len();
                x
            })
            .collect()
    };
    if total <= cap {
        return (0..total).map(pick).collect();
    }
    let mut chosen: BTreeSet<usize> = BTreeSet::new();
    let mut s = fnv(seed) | 1;
    while chosen.len() < cap {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        chosen.insert((s % total as u64) as usize);
    }
    chosen.into_iter().map(pick).collect()
}

fn call_text(t: &Target, args: &[String]) -> String {
    format!("{}::{}({})", t.module, t.name, args.join(", "))
}

enum Cand {
    /// `⊳` luôn sai với hằng này (E0901): ngoài miền của hàm
    OutOfDomain,
    Ran,
    /// hết nhiên liệu của bộ chạy tham chiếu
    Slow,
    Counter(String),
}

/// Chạy một ứng viên; `Err(lý do)` nếu không dựng/chạy được.
fn try_candidate(base: &[(String, String)], call: &str, opts: &Options) -> Result<Cand, String> {
    let mut srcs = base.to_vec();
    srcs.push((
        "auto_probe.gt".to_string(),
        format!("ƒ main() → () ! ∅ {{\n  r ≔ {call};\n}}\n"),
    ));
    srcs.sort();
    let inputs: Vec<SourceInput> = srcs
        .iter()
        .map(|(n, tx)| SourceInput {
            name: n.clone(),
            text: tx.clone(),
        })
        .collect();
    let mut res = check(&inputs, opts).map_err(|e| format!("đầu vào không hợp lệ: {e}"))?;
    let errs: BTreeSet<String> = res
        .report
        .diagnostics
        .iter()
        .filter(|d| d.code.as_str().starts_with('E'))
        .map(|d| d.code.as_str().to_string())
        .collect();
    if !errs.is_empty() {
        if errs.len() == 1 && errs.contains("E0901") {
            return Ok(Cand::OutOfDomain);
        }
        return Err(format!(
            "gritc từ chối chương trình thử ({})",
            errs.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }
    let Some(art) = res.take_gated() else {
        return Err("cổng gritc không mở".to_string());
    };
    let rep = gt_driver::runner::run_main_with(art, FUEL);
    Ok(match rep.end {
        RunEnd::Normal | RunEnd::ProgramExit(0) => Cand::Ran,
        RunEnd::Aborted(k) => Cand::Counter(format!("abort {k}")),
        RunEnd::Limit => Cand::Slow,
        other => return Err(format!("bộ chạy: {other:?}")),
    })
}

/// Thử một hàm. `base` = mọi tệp của dự án trừ `main.gt` gốc.
pub fn run_target(
    base: &[(String, String)],
    t: &Target,
    prims: &[PrimType],
    cap: usize,
    opts: &Options,
) -> Outcome {
    let vals: Vec<Vec<String>> = t
        .params
        .iter()
        .zip(prims)
        .map(|(k, p)| literals(*k, suffix_of(*p)))
        .collect();
    let cands = combos(&vals, cap.max(1), &format!("{}::{}", t.module, t.name));
    let (mut ran, mut dropped, mut slow, mut ambiguous) = (0usize, 0usize, 0usize, 0usize);
    let mut found: Vec<String> = Vec::new();
    for c in cands {
        let call = call_text(t, &c);
        match try_candidate(base, &call, opts) {
            Err(why) => return Outcome::Skipped(why),
            Ok(Cand::OutOfDomain) => dropped += 1,
            Ok(Cand::Ran) => ran += 1,
            Ok(Cand::Slow) => slow += 1,
            // hàm có `⊳` mà gritc không quyết định được: abort `precondition` có thể là của cửa vào (đầu vào ngoài miền) — không đổ lỗi
            Ok(Cand::Counter(what)) if t.amb && what == "abort precondition" => ambiguous += 1,
            Ok(Cand::Counter(what)) => {
                ran += 1;
                found.push(format!("{call} → {what}"));
            }
        }
    }
    if found.is_empty() {
        Outcome::Ok {
            ran,
            dropped,
            slow,
            ambiguous,
        }
    } else {
        Outcome::Failed { found, ran }
    }
}
