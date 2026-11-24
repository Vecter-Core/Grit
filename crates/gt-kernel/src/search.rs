//! NGOÀI TCB (ADR-P2-04): TÌM chứng chỉ Farkas bằng khử Fourier–Motzkin, mỗi hàng mang vector hệ số trên các ràng buộc GỐC của ca.
//! Sai ở đây chỉ làm mất chứng minh (kernel từ chối), không bao giờ làm sai phán quyết. Giới hạn số hàng/bước ⇒ bỏ cuộc (`None`).
use crate::linear::{Key, Lin};
use std::collections::{BTreeMap, BTreeSet};

/// Số hàng tối đa trong lúc khử.
pub const MAX_ROWS: usize = 400;

#[derive(Clone)]
struct Row {
    lin: Lin,
    mult: Vec<i128>,
}

fn combine(a: &Row, ma: i128, b: &Row, mb: i128) -> Option<Row> {
    let lin = a.lin.scale(ma)?.add_scaled(&b.lin, mb)?;
    let mut mult = Vec::with_capacity(a.mult.len());
    for (x, y) in a.mult.iter().zip(&b.mult) {
        mult.push(x.checked_mul(ma)?.checked_add(y.checked_mul(mb)?)?);
    }
    Some(Row { lin, mult })
}

fn gcd(a: i128, b: i128) -> i128 {
    // FB-C51 (Gemini G-09b): `abs()` panic trên i128::MIN ⇒ dùng `unsigned_abs`; ước chung > i128::MAX ⇒ 1 (an toàn: chỉ không rút gọn)
    let (mut a, mut b) = (a.unsigned_abs(), b.unsigned_abs());
    while b != 0 {
        (a, b) = (b, a % b);
    }
    i128::try_from(a).unwrap_or(1)
}

/// Bỏ hàng thừa: hàng hằng đúng hiển nhiên (`c ≤ 0`), và trong các hàng CÙNG phần biến chỉ giữ hàng chặt nhất (`c` lớn nhất: `Σ + c ≤ 0`).
/// Hàng giữ lại vẫn mang hệ số Farkas của chính nó nên mọi chứng chỉ tìm được vẫn đúng dạng; kernel kiểm lại tất cả.
fn prune(rows: Vec<Row>) -> Vec<Row> {
    let mut best: BTreeMap<Vec<(Key, i128)>, Row> = BTreeMap::new();
    for r in rows {
        if r.lin.coef.is_empty() && r.lin.c <= 0 {
            continue;
        }
        let key: Vec<(Key, i128)> = r.lin.coef.iter().map(|(k, v)| (k.clone(), *v)).collect();
        match best.get(&key) {
            Some(o) if o.lin.c >= r.lin.c => {}
            _ => {
                best.insert(key, r);
            }
        }
    }
    best.into_values().collect()
}

/// Tìm hệ số `λ ≥ 0` (theo chỉ số ràng buộc của ca) cho thấy ca vô nghiệm; `steps` là ngân sách (trừ dần).
pub fn refute_case(cons: &[Lin], steps: &mut u64) -> Option<Vec<(usize, i128)>> {
    let n = cons.len();
    let mut rows: Vec<Row> = cons
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let mut mult = vec![0; n];
            mult[i] = 1;
            Row {
                lin: l.clone(),
                mult,
            }
        })
        .collect();
    rows = prune(rows);
    loop {
        if let Some(r) = rows.iter().find(|r| r.lin.coef.is_empty() && r.lin.c > 0) {
            return Some(
                r.mult
                    .iter()
                    .enumerate()
                    .filter(|(_, m)| **m != 0)
                    .map(|(i, m)| (i, *m))
                    .collect(),
            );
        }
        let keys: BTreeSet<Key> = rows
            .iter()
            .flat_map(|r| r.lin.coef.keys().cloned())
            .collect();
        // khử biến có ít cặp (dương × âm) nhất (tất định: hoà ⇒ khoá nhỏ nhất)
        let pick = keys.iter().min_by_key(|k| {
            let p = rows
                .iter()
                .filter(|r| r.lin.coef.get(*k).is_some_and(|v| *v > 0))
                .count();
            let q = rows
                .iter()
                .filter(|r| r.lin.coef.get(*k).is_some_and(|v| *v < 0))
                .count();
            p * q
        })?;
        let k = pick.clone();
        let (pos, rest): (Vec<Row>, Vec<Row>) = rows
            .into_iter()
            .partition(|r| r.lin.coef.get(&k).is_some_and(|v| *v > 0));
        let (neg, mut next): (Vec<Row>, Vec<Row>) = rest
            .into_iter()
            .partition(|r| r.lin.coef.get(&k).is_some_and(|v| *v < 0));
        for p in &pos {
            for q in &neg {
                *steps = steps.checked_sub(1)?;
                let Some(b) = q.lin.coef[&k].checked_neg() else {
                    continue;
                };
                let a = p.lin.coef[&k];
                let g = gcd(a, b).max(1);
                // tràn i128 ở một cặp ⇒ bỏ cặp đó (chỉ mất một đường suy diễn), không bỏ cả lần tìm
                if let Some(r) = combine(p, b / g, q, a / g) {
                    next.push(r);
                }
            }
        }
        rows = prune(next);
        if rows.len() > MAX_ROWS {
            return None;
        }
    }
}
