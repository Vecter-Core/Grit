//! TCB (ADR-P2-04): KIỂM chứng chỉ Farkas. Với mỗi ca `i` của `refutation_dnf`, chứng chỉ cho hệ số nguyên `λⱼ ≥ 0` trên các ràng buộc của ca;
//! `Σ λⱼ·(ràng buộc j)` phải có MỌI hệ số biến bằng 0 và hằng `> 0`: khi đó `0 < hằng = Σ λⱼ·(…) ≤ 0`, mâu thuẫn ⇒ ca vô nghiệm.
//! Số học i128 kiểm tràn (tràn ⇒ từ chối). Không dựa vào bộ tìm chứng chỉ.
use crate::linear::{Case, Lin, refutation_dnf};
use gt_oblig::ir::Obligation;
use std::collections::BTreeMap;

/// Bác bỏ MỘT ca: tổ hợp Farkas (chỉ số ràng buộc, hệ số ≥ 0), hoặc mâu thuẫn nguyên tử `a ∧ ¬a`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CaseCert {
    Farkas(Vec<(usize, i128)>),
    Atom(u32),
}

/// Chứng chỉ: mỗi ca của DNF một bác bỏ.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FarkasCert {
    pub cases: Vec<CaseCert>,
}

const PREFIX: &str = "farkas:v1:";

impl FarkasCert {
    /// `farkas:v1:` + ca cách nhau `|`, cặp `chỉ_số*hệ_số` cách nhau `,`.
    #[must_use]
    pub fn encode(&self) -> String {
        let cases: Vec<String> = self
            .cases
            .iter()
            .map(|c| match c {
                CaseCert::Farkas(v) => v
                    .iter()
                    .map(|(i, m)| format!("{i}*{m}"))
                    .collect::<Vec<_>>()
                    .join(","),
                CaseCert::Atom(a) => format!("@{a}"),
            })
            .collect();
        if cases.is_empty() {
            // không có ca nào (giả thiết hiển nhiên mâu thuẫn): ký hiệu riêng, không nhầm với "một ca rỗng"
            return format!("{PREFIX}!");
        }
        format!("{PREFIX}{}", cases.join("|"))
    }

    /// Phân tích chặt: sai định dạng ⇒ `Err`.
    pub fn decode(s: &str) -> Result<FarkasCert, String> {
        let body = s
            .strip_prefix(PREFIX)
            .ok_or("không phải chứng chỉ farkas:v1")?;
        if body == "!" {
            return Ok(FarkasCert { cases: Vec::new() });
        }
        let mut cases = Vec::new();
        for c in body.split('|') {
            if let Some(a) = c.strip_prefix('@') {
                cases.push(CaseCert::Atom(
                    a.parse::<u32>().map_err(|_| "nguyên tử sai")?,
                ));
                continue;
            }
            let mut v = Vec::new();
            if !c.is_empty() {
                for p in c.split(',') {
                    let (i, m) = p.split_once('*').ok_or("cặp sai định dạng")?;
                    v.push((
                        i.parse::<usize>().map_err(|_| "chỉ số sai")?,
                        m.parse::<i128>().map_err(|_| "hệ số sai")?,
                    ));
                }
            }
            cases.push(CaseCert::Farkas(v));
        }
        Ok(FarkasCert { cases })
    }
}

/// Kiểm một ca: `Ok` nếu tổ hợp cho mâu thuẫn.
pub fn check_case(cons: &[Lin], mults: &[(usize, i128)]) -> Result<(), String> {
    let mut sum = Lin {
        coef: BTreeMap::new(),
        c: 0,
    };
    for (j, m) in mults {
        if *m < 0 {
            return Err(format!("hệ số âm {m} cho ràng buộc {j}"));
        }
        let row = cons
            .get(*j)
            .ok_or_else(|| format!("chỉ số ràng buộc {j} ngoài ca"))?;
        sum = sum.add_scaled(row, *m).ok_or("tràn số khi tổ hợp")?;
    }
    if !sum.coef.is_empty() {
        return Err(format!(
            "tổ hợp còn biến: {:?}",
            sum.coef.keys().collect::<Vec<_>>()
        ));
    }
    if sum.c <= 0 {
        return Err(format!("tổ hợp không mâu thuẫn (hằng {} ≤ 0)", sum.c));
    }
    Ok(())
}

/// KIỂM chứng chỉ cho nghĩa vụ: `Ok(())` ⇔ mọi ca của `cận kiểu ∧ giả thiết ∧ ¬mục tiêu` bị chứng chỉ bác bỏ.
pub fn check(ob: &Obligation, cert: &FarkasCert) -> Result<(), String> {
    let d = refutation_dnf(ob).ok_or("công thức vượt giới hạn ca / không dịch được")?;
    if d.len() != cert.cases.len() {
        return Err(format!(
            "chứng chỉ có {} ca, công thức có {}",
            cert.cases.len(),
            d.len()
        ));
    }
    for (i, (case, c)) in d.iter().zip(&cert.cases).enumerate() {
        check_one(case, c).map_err(|e| format!("ca {i}: {e}"))?;
    }
    Ok(())
}

/// Kiểm bác bỏ một ca: Farkas trên ràng buộc, hoặc ca chứa cả `a` và `¬a`.
pub fn check_one(case: &Case, c: &CaseCert) -> Result<(), String> {
    match c {
        CaseCert::Farkas(m) => check_case(&case.cons, m),
        CaseCert::Atom(a) => {
            if case.atoms.contains(&(*a, true)) && case.atoms.contains(&(*a, false)) {
                Ok(())
            } else {
                Err(format!("ca không chứa cả a{a} và ¬a{a}"))
            }
        }
    }
}
