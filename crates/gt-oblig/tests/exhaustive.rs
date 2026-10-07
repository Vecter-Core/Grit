//! SOUNDNESS VÉT CẠN của hàm chuyển trên miền khoảng (ADR-025 §8, §12): ở độ rộng bit nhỏ, MỌI cặp khoảng con (hộp) × MỌI cặp giá
//! trị cụ thể trong chúng; kết quả cụ thể phải thuộc khoảng kết quả trừu tượng. Dùng bảng cực trị 2-D (sparse table) nên mỗi cặp hộp kiểm
//! O(1) mà vẫn VÉT CẠN mọi cặp giá trị. Độ rộng 2..=6 vét cạn HOÀN TOÀN mọi cặp hộp; 7 và 8 phân tầng (mọi hộp có đầu mút ở tập các giá trị
//! biên + ngẫu nhiên có hạt giống) vì vét cạn mọi cặp hộp 8 bit tốn ~10⁹ ô/phép; công thức cho 16..128 dựa vào `I256` đã kiểm bằng `i128`.
use gt_oblig::int::I256;
use gt_oblig::interval::{B, Interval, Tri, Ty};

type Op = fn(Interval, Interval) -> Option<Interval>;

// ---------------------------------------------------------------- bảng cực trị 2-D

struct Rect {
    n: usize,
    levels: usize,
    mn: Vec<Vec<i64>>, // [kx*levels+ky][x*n+y]
    mx: Vec<Vec<i64>>,
}

fn lg(len: usize) -> usize {
    (usize::BITS - 1 - len.leading_zeros()) as usize
}

impl Rect {
    fn build(n: usize, f: &dyn Fn(usize, usize) -> Option<i64>) -> Rect {
        let levels = lg(n) + 1;
        let mut mn = vec![vec![i64::MAX; n * n]; levels * levels];
        let mut mx = vec![vec![i64::MIN; n * n]; levels * levels];
        for x in 0..n {
            for y in 0..n {
                if let Some(v) = f(x, y) {
                    mn[0][x * n + y] = v;
                    mx[0][x * n + y] = v;
                }
            }
        }
        for kx in 0..levels {
            for ky in 0..levels {
                if kx == 0 && ky == 0 {
                    continue;
                }
                let idx = kx * levels + ky;
                for x in 0..n {
                    for y in 0..n {
                        let (a, b) = if ky > 0 {
                            let p = kx * levels + ky - 1;
                            let h = 1 << (ky - 1);
                            if y + h < n {
                                (p, (x * n + y, x * n + y + h))
                            } else {
                                (p, (x * n + y, x * n + y))
                            }
                        } else {
                            let p = (kx - 1) * levels;
                            let h = 1 << (kx - 1);
                            if x + h < n {
                                (p, (x * n + y, (x + h) * n + y))
                            } else {
                                (p, (x * n + y, x * n + y))
                            }
                        };
                        let v1 = mn[a][b.0];
                        let v2 = mn[a][b.1];
                        mn[idx][x * n + y] = v1.min(v2);
                        let w1 = mx[a][b.0];
                        let w2 = mx[a][b.1];
                        mx[idx][x * n + y] = w1.max(w2);
                    }
                }
            }
        }
        Rect { n, levels, mn, mx }
    }

    /// (min, max) của giá trị xác định trong hình chữ nhật `[x1,x2] × [y1,y2]` (chỉ số 0-based); `None` nếu không có giá trị xác định.
    fn query(&self, x1: usize, x2: usize, y1: usize, y2: usize) -> Option<(i64, i64)> {
        let (kx, ky) = (lg(x2 - x1 + 1), lg(y2 - y1 + 1));
        let t = kx * self.levels + ky;
        let (xa, xb) = (x1, x2 + 1 - (1 << kx));
        let (ya, yb) = (y1, y2 + 1 - (1 << ky));
        let n = self.n;
        let mut lo = i64::MAX;
        let mut hi = i64::MIN;
        for (x, y) in [(xa, ya), (xb, ya), (xa, yb), (xb, yb)] {
            lo = lo.min(self.mn[t][x * n + y]);
            hi = hi.max(self.mx[t][x * n + y]);
        }
        if lo == i64::MAX { None } else { Some((lo, hi)) }
    }
}

// ---------------------------------------------------------------- bộ sinh hộp

fn xorshift(s: &mut u64) -> u64 {
    *s ^= *s << 13;
    *s ^= *s >> 7;
    *s ^= *s << 17;
    *s
}

fn boxes(n: usize, full: bool, seed: u64) -> Vec<(usize, usize)> {
    if full {
        return (0..n).flat_map(|a| (a..n).map(move |b| (a, b))).collect();
    }
    let mut ends: Vec<usize> = vec![0, 1, 2, n / 2 - 1, n / 2, n / 2 + 1, n - 3, n - 2, n - 1];
    let mut s = seed | 1;
    for _ in 0..4 {
        ends.push((xorshift(&mut s) as usize) % n);
    }
    ends.retain(|e| *e < n);
    ends.sort_unstable();
    ends.dedup();
    let mut out = Vec::new();
    for (i, a) in ends.iter().enumerate() {
        for b in &ends[i..] {
            out.push((*a, *b));
        }
    }
    for _ in 0..3000 {
        let a = (xorshift(&mut s) as usize) % n;
        let b = a + (xorshift(&mut s) as usize) % (n - a);
        out.push((a, b));
    }
    out
}

fn to_iv(v: i64) -> I256 {
    I256::from_i128(i128::from(v))
}

fn iv(lo: i64, hi: i64) -> Interval {
    Interval::new(B::Fin(to_iv(lo)), B::Fin(to_iv(hi)))
}

fn bound_i64(b: B) -> Option<i64> {
    b.fin()
        .and_then(I256::to_i128)
        .and_then(|v| i64::try_from(v).ok())
}

// ---------------------------------------------------------------- kiểm một phép toán nhị phân trên mọi cặp hộp

struct Stats {
    pairs: u64,
    exact: u64,
    inexact: u64,
}

fn check_binary(
    ty: Ty,
    name: &str,
    op: Op,
    conc: &dyn Fn(i64, i64) -> Option<i64>,
    must_be_exact: bool,
    full: bool,
) -> Stats {
    let (lo, hi) = (
        i64::try_from(ty.min().to_i128().unwrap_or(0)).unwrap_or(0),
        i64::try_from(ty.max().to_i128().unwrap_or(0)).unwrap_or(0),
    );
    let n = (hi - lo + 1) as usize;
    let table = Rect::build(n, &|x, y| conc(lo + x as i64, lo + y as i64));
    let bx = boxes(n, full, 0x1234_5678 + u64::from(ty.bits));
    let mut st = Stats {
        pairs: 0,
        exact: 0,
        inexact: 0,
    };
    for &(a1, a2) in &bx {
        for &(b1, b2) in &bx {
            st.pairs += 1;
            let (ia, ib) = (
                iv(lo + a1 as i64, lo + a2 as i64),
                iv(lo + b1 as i64, lo + b2 as i64),
            );
            let abs = op(ia, ib);
            let truth = table.query(a1, a2, b1, b2);
            match (abs, truth) {
                (None, None) => st.exact += 1,
                (None, Some(t)) => panic!(
                    "{} {name}: kết quả trừu tượng RỖNG nhưng có giá trị cụ thể {t:?} (a=[{a1},{a2}] b=[{b1},{b2}])",
                    ty.name()
                ),
                (Some(_), None) => st.inexact += 1, // trừu tượng không rỗng, không có giá trị cụ thể (vd chia cho {0})
                (Some(r), Some((tmin, tmax))) => {
                    let (rl, rh) = (r.lo, r.hi);
                    assert!(
                        rl <= B::Fin(to_iv(tmin)) && B::Fin(to_iv(tmax)) <= rh,
                        "{} {name}: KHÔNG SOUND: a=[{},{}] b=[{},{}] ⇒ cụ thể [{tmin},{tmax}] ⊄ trừu tượng [{rl:?},{rh:?}]",
                        ty.name(),
                        lo + a1 as i64,
                        lo + a2 as i64,
                        lo + b1 as i64,
                        lo + b2 as i64
                    );
                    let exact = bound_i64(rl) == Some(tmin) && bound_i64(rh) == Some(tmax);
                    if exact {
                        st.exact += 1;
                    } else {
                        st.inexact += 1;
                        assert!(
                            !must_be_exact,
                            "{} {name}: phải CHÍNH XÁC: a=[{},{}] b=[{},{}] cụ thể [{tmin},{tmax}] trừu tượng [{rl:?},{rh:?}]",
                            ty.name(),
                            lo + a1 as i64,
                            lo + a2 as i64,
                            lo + b1 as i64,
                            lo + b2 as i64
                        );
                    }
                }
            }
        }
    }
    st
}

fn widths() -> Vec<(u32, bool)> {
    let mut v = Vec::new();
    for w in 2..=8 {
        v.push((w, false));
        v.push((w, true));
    }
    v
}

/// 2..=7 bit: vét cạn HOÀN TOÀN mọi cặp hộp; 8 bit phân tầng, trừ khi `GT_FULL8=1` (chạy tay một lần, ~10⁹ ô/phép).
fn full_for(bits: u32) -> bool {
    bits <= 7 || (bits == 8 && std::env::var("GT_FULL8").is_ok_and(|v| v == "1"))
}

#[test]
fn add_sub_mul_are_sound_and_exact_on_every_box_pair() {
    type Oracle = fn(i64, i64) -> Option<i64>;
    let ops: [(&str, Op, Oracle); 3] = [
        ("add", |a, b| Some(a.add(b)), |x, y| Some(x + y)),
        ("sub", |a, b| Some(a.sub(b)), |x, y| Some(x - y)),
        ("mul", |a, b| Some(a.mul(b)), |x, y| Some(x * y)),
    ];
    let mut total = 0;
    for (bits, signed) in widths() {
        let ty = Ty::new(bits, signed);
        for (name, op, conc) in ops {
            let st = check_binary(ty, name, op, &conc, true, full_for(bits));
            total += st.pairs;
            assert_eq!(st.inexact, 0, "{} {name}", ty.name());
        }
    }
    eprintln!("add/sub/mul: {total} cặp hộp, mọi cặp giá trị cụ thể, chính xác");
}

#[test]
fn div_and_rem_are_sound_for_every_box_pair() {
    let div: Op = |a, b| a.div_trunc_nz(b);
    let rem: Op = |a, b| a.rem_trunc_nz(b);
    let mut total = 0;
    for (bits, signed) in widths() {
        let ty = Ty::new(bits, signed);
        let st = check_binary(
            ty,
            "div",
            div,
            &|x, y| if y == 0 { None } else { Some(x / y) },
            true,
            full_for(bits),
        );
        total += st.pairs;
        // chia: cực trị tại các góc (sau khi tách dấu) ⇒ chính xác
        let st = check_binary(
            ty,
            "rem",
            rem,
            &|x, y| if y == 0 { None } else { Some(x % y) },
            false,
            full_for(bits),
        );
        total += st.pairs;
        // dư: chỉ sound (không đòi chính xác); ghi nhận độ chính xác để theo dõi
        eprintln!("{} rem: chính xác {}/{} cặp", ty.name(), st.exact, st.pairs);
    }
    eprintln!("div/rem: {total} cặp hộp");
}

fn neg_range(ty: Ty) -> Vec<Interval> {
    let (lo, hi) = (
        i64::try_from(ty.min().to_i128().unwrap_or(0)).unwrap_or(0),
        i64::try_from(ty.max().to_i128().unwrap_or(0)).unwrap_or(0),
    );
    let mut v = Vec::new();
    for a in lo..=hi {
        for b in a..=hi {
            v.push(iv(a, b));
        }
    }
    v
}

#[test]
fn negation_is_exact_on_every_box() {
    for (bits, signed) in widths().into_iter().filter(|(b, _)| *b <= 7) {
        let ty = Ty::new(bits, signed);
        for a in neg_range(ty) {
            let n = a.neg();
            let (al, ah) = (
                bound_i64(a.lo).expect("hữu hạn"),
                bound_i64(a.hi).expect("hữu hạn"),
            );
            assert_eq!((bound_i64(n.lo), bound_i64(n.hi)), (Some(-ah), Some(-al)));
        }
    }
}

#[test]
fn wrap_saturate_fits_and_clamp_are_sound_on_every_math_interval() {
    for (bits, signed) in widths().into_iter().filter(|(b, _)| *b <= 6) {
        let ty = Ty::new(bits, signed);
        let (m, mx) = (
            i64::try_from(ty.min().to_i128().unwrap_or(0)).unwrap_or(0),
            i64::try_from(ty.max().to_i128().unwrap_or(0)).unwrap_or(0),
        );
        let p = mx - m + 1;
        let (lo_all, hi_all) = (m - 2 * p, mx + 2 * p);
        for a in lo_all..=hi_all {
            for b in a..=hi_all {
                let i = iv(a, b);
                let w = i.wrap(ty);
                let s = i.saturate(ty);
                let fits = i.fits(ty);
                let clamp = i.clamp(ty);
                let mut any_in = false;
                let mut all_in = true;
                for r in a..=b {
                    let inside = r >= m && r <= mx;
                    any_in |= inside;
                    all_in &= inside;
                    // wrap: giá trị quấn thật
                    let wrapped = (r - m).rem_euclid(p) + m;
                    assert!(
                        w.contains(to_iv(wrapped)),
                        "{} wrap [{a},{b}] ∌ wrap({r})={wrapped}: {w:?}",
                        ty.name()
                    );
                    let sat = r.clamp(m, mx);
                    assert!(
                        s.contains(to_iv(sat)),
                        "{} sat [{a},{b}] ∌ {sat}",
                        ty.name()
                    );
                    if inside {
                        assert!(
                            clamp.is_some_and(|c| c.contains(to_iv(r))),
                            "{} clamp [{a},{b}] mất {r}",
                            ty.name()
                        );
                    }
                }
                match fits {
                    Tri::Yes => assert!(all_in, "{} fits Yes sai [{a},{b}]", ty.name()),
                    Tri::No => assert!(!any_in, "{} fits No sai [{a},{b}]", ty.name()),
                    Tri::Maybe => {}
                }
                assert_eq!(
                    clamp.is_none(),
                    !any_in,
                    "{} clamp rỗng ⇔ không giá trị trong kiểu [{a},{b}]",
                    ty.name()
                );
                // wrap chính xác khi nằm trong kiểu
                if all_in {
                    assert_eq!(w, i);
                }
            }
        }
    }
}
