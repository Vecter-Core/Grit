//! Thông dịch GLIR theo mô hình BYTE (Rust thuần, ĐỘC LẬP với gt-mir/gt-glirgen): oracle ngữ nghĩa cho đường hạ MIR→GLIR (ADR-P2-02; thiết kế cố vấn Opus 2026-11-25).
//! Mỗi slot là một vùng byte có cỡ do `layout` (bố cục GLIR) quyết định; con trỏ là `(vùng, offset)` và mang NGUỒN GỐC (provenance) khi nằm trong bộ nhớ.
//! Hành vi KHÔNG xác định trả `Ub` (KHÔNG phải `Abort`) để một bước hạ quên chốt chặn lộ ra: chia 0, `MIN/-1`, dịch ≥ độ rộng, đọc byte chưa ghi, con trỏ ngoài vùng,
//! đọc con trỏ từ byte không phải mảnh con trỏ nguyên vẹn, gọi hàm ngoài module. Chương trình hạ đúng không bao giờ chạm `Ub`.
//! Hàm `declare` (runtime) CHỈ chạy được khi có MÔ HÌNH tham chiếu ở [`runtime_model_sig`] VÀ khai báo của module trùng chữ ký mô hình; ngoài ra ⇒ `Ub`.
use crate::layout::{field_offset, layout};
use crate::{BinOp, CastOp, CmpPred, Func, Inst, Module, Opd, Term, Ty};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

/// "Vùng" của con trỏ null (`Opd::Null`): không phải chỉ mục vùng nào ⇒ mọi truy cập/tính địa chỉ khác 0 qua nó là `Ub`.
const NULL_REGION: usize = usize::MAX;

/// Cỡ tối đa (byte) của MỘT slot mà thông dịch chịu cấp; lớn hơn ⇒ `OutOfFuel` (giới hạn tài nguyên của oracle, không phải phán quyết).
pub const MAX_SLOT_BYTES: u64 = 1 << 22;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// giá trị trả về (mẫu bit đã cắt) hoặc `None` nếu `ret void`
    Value(Option<u128>),
    /// giá trị tổng hợp trả về (struct/mảng) dưới dạng byte (`None` = byte chưa ghi/đệm)
    Agg(Vec<Option<u8>>),
    /// con trỏ trả về (chỉ có nghĩa trong cùng một lần chạy: `(vùng, offset)`)
    Ptr(usize, u64),
    /// giá trị tổng hợp trả về GIỮ NGUỒN GỐC con trỏ (chỉ dùng giữa các hàm trong một lần chạy; ở mức cao nhất đổi thành `Agg` hoặc `Ub`)
    Bytes(Vec<Byte>),
    Abort(String),
    Ub(String),
    OutOfFuel,
}

/// Đối số của hàm GLIR: số nguyên (mẫu bit) hoặc giá trị tổng hợp dạng byte.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Arg {
    Int(u128),
    Bytes(Vec<Option<u8>>),
    /// con trỏ tới một vùng mới có nội dung ban đầu `bytes` (để thử hàm nhận `&T`/`&!T`)
    Ref(Vec<Option<u8>>),
    /// con trỏ béo `{ptr, len}` tới vùng mới có nội dung `bytes` và độ dài `len` (phần tử) — để thử hàm nhận `&[T]`/`&str`
    Fat(Vec<Option<u8>>, u64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Byte {
    Uninit,
    B(u8),
    /// mảnh `k` (0..8) của con trỏ `(vùng, offset)`
    P(usize, u64, u8),
}

#[derive(Clone, Debug)]
enum Val {
    Int(u128),
    Ptr(usize, u64),
    /// giá trị tổng hợp (struct/mảng) dưới dạng byte, kể cả byte chưa ghi
    Agg(Vec<Byte>),
}

fn mask(w: u32) -> u128 {
    if w >= 128 {
        u128::MAX
    } else {
        (1u128 << w) - 1
    }
}

fn sext(v: u128, w: u32) -> i128 {
    if w >= 128 {
        v as i128
    } else if (v >> (w - 1)) & 1 == 1 {
        (v | !mask(w)) as i128
    } else {
        v as i128
    }
}

struct St<'a> {
    f: &'a Func,
    env: BTreeMap<String, Val>,
    /// tên slot → vùng
    slot: BTreeMap<String, usize>,
    mem: &'a mut Vec<Vec<Byte>>,
    /// tên global → vùng (các vùng `< nro` là CHỈ ĐỌC)
    gl: &'a BTreeMap<String, usize>,
    nro: usize,
    /// vùng slot của các khung đã trả về (FB-G2-04): mọi truy cập/tính địa chỉ qua chúng là `Ub` (con trỏ treo vào ngăn xếp)
    freed: &'a RefCell<BTreeSet<usize>>,
}

impl St<'_> {
    fn opd(&self, o: &Opd) -> Result<Val, String> {
        match o {
            Opd::Val(v) => self
                .env
                .get(v)
                .cloned()
                .ok_or_else(|| format!("%{v} chưa có giá trị")),
            Opd::Slot(s) => Ok(Val::Ptr(*self.slot.get(s).ok_or("slot lạ")?, 0)),
            Opd::Global(g) => Ok(Val::Ptr(*self.gl.get(g).ok_or("global lạ")?, 0)),
            Opd::Null => Ok(Val::Ptr(NULL_REGION, 0)),
            Opd::Int(t, n) => Ok(Val::Int(
                (*n as u128) & mask(t.bits().ok_or("hằng không nguyên")?),
            )),
        }
    }
    fn int(&self, o: &Opd) -> Result<u128, String> {
        match self.opd(o)? {
            Val::Int(v) => Ok(v),
            Val::Ptr(..) | Val::Agg(_) => Err("cần số nguyên".into()),
        }
    }
    fn ptr(&self, o: &Opd) -> Result<(usize, u64), String> {
        match self.opd(o)? {
            Val::Ptr(a, off) => Ok((a, off)),
            Val::Int(_) | Val::Agg(_) => Err("cần con trỏ".into()),
        }
    }
    fn live(&self, a: usize) -> Result<(), String> {
        if a >= self.mem.len() {
            return Err("truy cập qua con trỏ null".into());
        }
        if self.freed.borrow().contains(&a) {
            return Err("dùng vùng đã giải phóng (slot của khung đã trả về hoặc Arena đã reset: con trỏ treo)".into());
        }
        Ok(())
    }
    fn range(&self, a: usize, off: u64, n: u64) -> Result<std::ops::Range<usize>, String> {
        self.live(a)?;
        let len = self.mem[a].len() as u64;
        if off.checked_add(n).is_none_or(|e| e > len) {
            return Err(format!("truy cập [{off}, {off}+{n}) ngoài vùng cỡ {len}"));
        }
        Ok(off as usize..(off + n) as usize)
    }
    fn read(&self, a: usize, off: u64, t: &Ty) -> Result<Vec<Byte>, String> {
        let n = layout(t).size;
        let r = self.range(a, off, n)?;
        Ok(self.mem[a][r].to_vec())
    }
    fn write(&mut self, a: usize, off: u64, bytes: &[Byte]) -> Result<(), String> {
        if a < self.nro {
            return Err("ghi vào vùng chỉ-đọc (global)".into());
        }
        let r = self.range(a, off, bytes.len() as u64)?;
        self.mem[a][r].copy_from_slice(bytes);
        Ok(())
    }
}

fn int_bytes(v: u128, n: u64) -> Vec<Byte> {
    (0..n)
        .map(|i| Byte::B(((v >> (8 * i)) & 0xff) as u8))
        .collect()
}

fn decode_int(bs: &[Byte], w: u32) -> Result<u128, String> {
    let mut v = 0u128;
    for (i, b) in bs.iter().enumerate() {
        match b {
            Byte::B(x) => v |= u128::from(*x) << (8 * i),
            Byte::Uninit => return Err("đọc byte chưa ghi".into()),
            Byte::P(..) => return Err("đọc số nguyên từ mảnh con trỏ".into()),
        }
    }
    Ok(v & mask(w))
}

fn decode_ptr(bs: &[Byte]) -> Result<(usize, u64), String> {
    let mut first = None;
    for (i, b) in bs.iter().enumerate() {
        match b {
            Byte::P(a, off, k) if usize::from(*k) == i && first.is_none_or(|f| f == (*a, *off)) => {
                first = Some((*a, *off))
            }
            Byte::Uninit => return Err("đọc con trỏ từ byte chưa ghi".into()),
            _ => return Err("đọc con trỏ từ byte không phải mảnh con trỏ nguyên vẹn".into()),
        }
    }
    first.ok_or_else(|| "con trỏ rỗng".into())
}

fn binop(op: BinOp, w: u32, a: u128, b: u128) -> Result<u128, String> {
    let m = mask(w);
    let (sa, sb) = (sext(a, w), sext(b, w));
    let min = if w >= 128 {
        i128::MIN
    } else {
        -(1i128 << (w - 1))
    };
    Ok(match op {
        BinOp::Add => a.wrapping_add(b) & m,
        BinOp::Sub => a.wrapping_sub(b) & m,
        BinOp::Mul => a.wrapping_mul(b) & m,
        BinOp::UDiv => {
            if b == 0 {
                return Err("udiv cho 0".into());
            }
            a / b
        }
        BinOp::URem => {
            if b == 0 {
                return Err("urem cho 0".into());
            }
            a % b
        }
        BinOp::SDiv => {
            if sb == 0 || (sa == min && sb == -1) {
                return Err("sdiv: chia 0 hoặc MIN/-1".into());
            }
            (sa / sb) as u128 & m
        }
        BinOp::SRem => {
            if sb == 0 || (sa == min && sb == -1) {
                return Err("srem: chia 0 hoặc MIN%-1".into());
            }
            (sa % sb) as u128 & m
        }
        BinOp::And => a & b,
        BinOp::Or => a | b,
        BinOp::Xor => a ^ b,
        BinOp::Shl | BinOp::LShr | BinOp::AShr => {
            if b >= u128::from(w) {
                return Err("dịch ≥ độ rộng".into());
            }
            let s = b as u32;
            match op {
                BinOp::Shl => (a << s) & m,
                BinOp::LShr => a >> s,
                _ => ((sa >> s) as u128) & m,
            }
        }
    })
}

fn cmp(p: CmpPred, w: u32, a: u128, b: u128) -> bool {
    let (sa, sb) = (sext(a, w), sext(b, w));
    match p {
        CmpPred::Eq => a == b,
        CmpPred::Ne => a != b,
        CmpPred::Ult => a < b,
        CmpPred::Ule => a <= b,
        CmpPred::Ugt => a > b,
        CmpPred::Uge => a >= b,
        CmpPred::Slt => sa < sb,
        CmpPred::Sle => sa <= sb,
        CmpPred::Sgt => sa > sb,
        CmpPred::Sge => sa >= sb,
    }
}

/// Chạy `f` với đối số nguyên `args` (mẫu bit; sẽ được cắt theo kiểu tham số). `fuel` = số lệnh tối đa.
#[must_use]
pub fn run(f: &Func, args: &[u128], fuel: u64) -> Outcome {
    let m = Module {
        globals: Vec::new(),
        decls: Vec::new(),
        funcs: vec![f.clone()],
    };
    run_module(&m, &f.name, args, fuel)
}

/// Như `run` nhưng gọi được hàm khác trong `m` (đệ quy tới độ sâu 128, quá ⇒ `OutOfFuel`; hàm `declare` ngoài module ⇒ `Ub`).
#[must_use]
pub fn run_module(m: &Module, name: &str, args: &[u128], fuel: u64) -> Outcome {
    let a: Vec<Arg> = args.iter().map(|x| Arg::Int(*x)).collect();
    run_module_args(m, name, &a, fuel)
}

/// Như `run_module` với đối số có thể là giá trị tổng hợp (không có `Arg::Ref`).
#[must_use]
pub fn run_module_args(m: &Module, name: &str, args: &[Arg], fuel: u64) -> Outcome {
    run_module_refs(m, name, args, fuel).0
}

/// Như `run_module_args`, thêm: với mỗi `Arg::Ref` (theo thứ tự) trả NỘI DUNG cuối của vùng nó trỏ tới (để so với giá trị sau lời gọi của thông dịch MIR).
#[must_use]
pub fn run_module_refs(
    m: &Module,
    name: &str,
    args: &[Arg],
    fuel: u64,
) -> (Outcome, Vec<Vec<Option<u8>>>) {
    let (o, fin, _) = run_module_stats(m, name, args, fuel);
    (o, fin)
}

/// Số liệu mô hình runtime của một lần chạy (để đối chiếu với thông dịch MIR: `secret_wipes()`, `heap_live()`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    /// số lần gọi `gritrt_secret_wipe`
    pub secret_wipes: u64,
    /// số khối `gritrt_heap_alloc` chưa `gritrt_heap_free` khi kết thúc
    pub heap_live: usize,
}

/// Như `run_module_refs`, thêm [`Stats`].
#[must_use]
pub fn run_module_stats(
    m: &Module,
    name: &str,
    args: &[Arg],
    mut fuel: u64,
) -> (Outcome, Vec<Vec<Option<u8>>>, Stats) {
    let Some(f) = m.funcs.iter().find(|f| f.name == name) else {
        return (
            Outcome::Ub(format!("không có hàm @{name}")),
            Vec::new(),
            Stats::default(),
        );
    };
    let mut mem: Vec<Vec<Byte>> = Vec::new();
    // các global chiếm các vùng đầu (chỉ đọc)
    let mut gl: BTreeMap<String, usize> = BTreeMap::new();
    for g in &m.globals {
        gl.insert(g.name.clone(), mem.len());
        mem.push(g.bytes.iter().map(|b| Byte::B(*b)).collect());
    }
    let nro = mem.len();
    let mut refs: Vec<usize> = Vec::new();
    let vals: Vec<Val> = args
        .iter()
        .map(|a| match a {
            Arg::Int(x) => Val::Int(*x),
            Arg::Bytes(bs) => {
                Val::Agg(bs.iter().map(|b| b.map_or(Byte::Uninit, Byte::B)).collect())
            }
            Arg::Fat(bs, len) => {
                mem.push(bs.iter().map(|b| b.map_or(Byte::Uninit, Byte::B)).collect());
                let a = mem.len() - 1;
                refs.push(a);
                let mut fat: Vec<Byte> = (0..8u8).map(|k| Byte::P(a, 0, k)).collect();
                fat.extend(int_bytes(u128::from(*len), 8));
                Val::Agg(fat)
            }
            Arg::Ref(bs) => {
                mem.push(bs.iter().map(|b| b.map_or(Byte::Uninit, Byte::B)).collect());
                refs.push(mem.len() - 1);
                Val::Ptr(mem.len() - 1, 0)
            }
        })
        .collect();
    let funcs = m.funcs.iter().map(|g| (g.name.as_str(), g)).collect();
    let decls = m
        .decls
        .iter()
        .map(|d| (d.name.as_str(), (d.params.clone(), d.ret.clone())))
        .collect();
    let glob = Glob {
        gl,
        nro,
        funcs,
        decls,
        freed: RefCell::new(BTreeSet::new()),
        arenas: RefCell::new(Vec::new()),
        heap: RefCell::new(BTreeMap::new()),
        wipes: RefCell::new(0),
    };
    let mut out = exec(f, &vals, &mut fuel, 0, &mut mem, &glob);
    if let Outcome::Ptr(a, _) = out
        && glob.freed.borrow().contains(&a)
    {
        out = Outcome::Ub("trả về con trỏ tới slot của khung đã trả về".into());
    }
    if let Outcome::Bytes(bs) = out {
        out = if bs.iter().any(|b| matches!(b, Byte::P(..))) {
            Outcome::Ub("trả về giá trị chứa con trỏ ở mức cao nhất".into())
        } else {
            Outcome::Agg(
                bs.iter()
                    .map(|b| if let Byte::B(x) = b { Some(*x) } else { None })
                    .collect(),
            )
        };
    }
    let fin = refs
        .iter()
        .map(|i| {
            mem[*i]
                .iter()
                .map(|b| match b {
                    Byte::B(x) => Some(*x),
                    _ => None,
                })
                .collect()
        })
        .collect();
    let stats = Stats {
        secret_wipes: *glob.wipes.borrow(),
        heap_live: glob.heap.borrow().len(),
    };
    (out, fin, stats)
}

/// Môi trường chung của một lần chạy: tên global → vùng và số vùng chỉ-đọc đứng đầu.
struct Glob<'m> {
    gl: BTreeMap<String, usize>,
    nro: usize,
    /// chỉ mục tên hàm → hàm (tra callee không tuyến tính theo số hàm của module)
    funcs: BTreeMap<&'m str, &'m Func>,
    /// `declare` của module: tên → (kiểu tham số, kiểu trả)
    decls: BTreeMap<&'m str, (Vec<Ty>, Ty)>,
    freed: RefCell<BTreeSet<usize>>,
    /// mô hình Arena của runtime: tay cầm (chỉ số, như `gritrt_arena_new`) → các vùng đã cấp từ nó (chưa `reset`)
    arenas: RefCell<Vec<Vec<usize>>>,
    /// mô hình heap của runtime (`gritrt_heap_alloc`): vùng của khối còn sống → (cỡ, căn lề) khi cấp
    heap: RefCell<BTreeMap<usize, (u128, u128)>>,
    /// số lần `gritrt_secret_wipe`
    wipes: RefCell<u64>,
}

/// Chữ ký (tham số, trả về) của các hàm runtime CÓ mô hình tham chiếu trong thông dịch — đúng chữ ký codegen LLVM phát cho `gritrt_*` cùng tên
/// (`runtime/gt-rt`). Hàm `declare` không có ở đây ⇒ gọi tới là `Ub`.
#[must_use]
pub fn runtime_model_sig(name: &str) -> Option<(Vec<Ty>, Ty)> {
    Some(match name {
        "gritrt_arena_new" => (Vec::new(), Ty::I32),
        "gritrt_arena_alloc" => (vec![Ty::I32, Ty::I64, Ty::I64], Ty::Ptr),
        "gritrt_arena_reset" => (vec![Ty::I32], Ty::Void),
        "gritrt_utf8_valid" => (vec![Ty::Ptr, Ty::I64], Ty::I32),
        "gritrt_heap_alloc" => (vec![Ty::I64, Ty::I64], Ty::Ptr),
        "gritrt_heap_free" => (vec![Ty::Ptr, Ty::I64, Ty::I64], Ty::Void),
        "gritrt_secret_wipe" => (vec![Ty::Ptr, Ty::I64], Ty::Void),
        _ => return None,
    })
}

/// Mô hình tham chiếu của một hàm runtime (bộ nhớ Arena KHÔNG giới hạn, như thông dịch MIR; hết bộ nhớ thật ⇒ abort chỉ kiểm trên exe):
/// `arena_new` ⇒ tay cầm mới (0, 1, … như runtime); `arena_alloc(h, size, align)` ⇒ vùng MỚI `size` byte chưa ghi (tay cầm lạ hoặc `align` không phải
/// luỹ thừa 2 ≤ 4096 ⇒ `Ub`: runtime abort, bước hạ đúng không bao giờ gây ra); `arena_reset(h)` ⇒ mọi vùng cấp từ `h` thành ĐÃ GIẢI PHÓNG (truy cập sau ⇒ `Ub`).
fn runtime_call(
    name: &str,
    vals: &[Val],
    mem: &mut Vec<Vec<Byte>>,
    glob: &Glob<'_>,
) -> Result<Option<Val>, Outcome> {
    let ub = |m: &str| Outcome::Ub(format!("@{name}: {m}"));
    let (Some(want), Some(have)) = (runtime_model_sig(name), glob.decls.get(name)) else {
        return Err(ub("gọi hàm ngoài module không có mô hình runtime"));
    };
    if want != *have {
        return Err(ub("`declare` sai chữ ký so với runtime"));
    }
    if name == "gritrt_utf8_valid" {
        // `gritrt_utf8_valid(p, n)`: n = 0 ⇒ 1 (không đọc p, kể cả null); ngược lại 1 ⇔ n byte tại p là UTF-8 hợp lệ (RFC 3629, như
        // `std::str::from_utf8` của runtime). Byte chưa ghi/mảnh con trỏ/ngoài vùng/vùng đã giải phóng ⇒ `Ub`.
        let [Val::Ptr(a, off), Val::Int(n)] = vals else {
            return Err(ub("đối số phải là (ptr, i64)"));
        };
        if *n == 0 {
            return Ok(Some(Val::Int(1)));
        }
        if *a >= mem.len() || glob.freed.borrow().contains(a) {
            return Err(ub("đọc qua con trỏ null/treo"));
        }
        let end = u64::try_from(*n).ok().and_then(|n| off.checked_add(n));
        let Some(bs) = end.and_then(|e| mem[*a].get(*off as usize..e as usize)) else {
            return Err(ub("đọc ngoài vùng"));
        };
        let bytes: Vec<u8> = bs
            .iter()
            .map(|b| match b {
                Byte::B(x) => Ok(*x),
                _ => Err(ub("đọc byte chưa ghi/mảnh con trỏ")),
            })
            .collect::<Result<_, _>>()?;
        return Ok(Some(Val::Int(u128::from(
            std::str::from_utf8(&bytes).is_ok(),
        ))));
    }
    match (name, vals) {
        // `gritrt_heap_free(p, size, align)`: `p` phải là ĐẦU một khối heap còn sống cấp với ĐÚNG (size, align) (runtime trả cho allocator với
        // `Layout` đó — sai là UB của Rust); khối thành đã giải phóng (truy cập/trả lần hai sau ⇒ `Ub`)
        ("gritrt_heap_free", [Val::Ptr(a, off), Val::Int(size), Val::Int(align)]) => {
            let mut heap = glob.heap.borrow_mut();
            if *off != 0 || heap.get(a) != Some(&(*size, *align)) {
                return Err(ub(
                    "trả khối không phải đầu khối heap còn sống cùng (cỡ, căn lề)",
                ));
            }
            heap.remove(a);
            glob.freed.borrow_mut().insert(*a);
            return Ok(None);
        }
        // `gritrt_secret_wipe(p, n)`: ghi 0 lên n byte tại p (vùng còn sống, ghi được, trong biên) và ĐẾM (như `secret_wipes` của thông dịch MIR)
        ("gritrt_secret_wipe", [Val::Ptr(a, off), Val::Int(n)]) => {
            if *a >= mem.len() || *a < glob.nro || glob.freed.borrow().contains(a) {
                return Err(ub("xoá qua con trỏ null/treo/chỉ-đọc"));
            }
            let end = u64::try_from(*n).ok().and_then(|n| off.checked_add(n));
            let Some(bs) = end.and_then(|e| mem[*a].get_mut(*off as usize..e as usize)) else {
                return Err(ub("xoá ngoài vùng"));
            };
            bs.fill(Byte::B(0));
            *glob.wipes.borrow_mut() += 1;
            return Ok(None);
        }
        ("gritrt_heap_free" | "gritrt_secret_wipe", _) => {
            return Err(ub("đối số sai dạng"));
        }
        _ => {}
    }
    let ints: Vec<u128> = vals
        .iter()
        .map(|v| match v {
            Val::Int(x) => Ok(*x),
            Val::Ptr(..) | Val::Agg(_) => Err(ub("đối số không phải số nguyên")),
        })
        .collect::<Result<_, _>>()?;
    if ints.len() != want.0.len() {
        return Err(ub("sai số đối số"));
    }
    let mut arenas = glob.arenas.borrow_mut();
    let handle = |h: u128| -> Result<usize, Outcome> {
        usize::try_from(h)
            .ok()
            .filter(|k| *k < arenas.len())
            .ok_or_else(|| ub("tay cầm Arena không tồn tại"))
    };
    match name {
        "gritrt_arena_new" => {
            arenas.push(Vec::new());
            Ok(Some(Val::Int((arenas.len() - 1) as u128)))
        }
        "gritrt_arena_alloc" => {
            let k = handle(ints[0])?;
            let (size, align) = (ints[1], ints[2]);
            if !align.is_power_of_two() || align > 4096 {
                return Err(ub("căn lề không hợp lệ"));
            }
            if size > u128::from(MAX_SLOT_BYTES) {
                return Err(Outcome::OutOfFuel);
            }
            mem.push(vec![Byte::Uninit; size as usize]);
            arenas[k].push(mem.len() - 1);
            Ok(Some(Val::Ptr(mem.len() - 1, 0)))
        }
        "gritrt_heap_alloc" => {
            let (size, align) = (ints[0], ints[1]);
            if !align.is_power_of_two() || align > 4096 {
                return Err(ub("căn lề không hợp lệ"));
            }
            if size > u128::from(MAX_SLOT_BYTES) {
                return Err(Outcome::OutOfFuel);
            }
            mem.push(vec![Byte::Uninit; size as usize]);
            glob.heap.borrow_mut().insert(mem.len() - 1, (size, align));
            Ok(Some(Val::Ptr(mem.len() - 1, 0)))
        }
        "gritrt_arena_reset" => {
            let k = handle(ints[0])?;
            glob.freed.borrow_mut().extend(arenas[k].drain(..));
            Ok(None)
        }
        _ => Err(ub("không có mô hình runtime")),
    }
}

/// Chạy một khung; khi khung kết thúc (bất kể kết quả) các vùng slot của nó bị đánh dấu đã giải phóng (FB-G2-04).
fn exec(
    f: &Func,
    args: &[Val],
    fuel: &mut u64,
    depth: u32,
    mem: &mut Vec<Vec<Byte>>,
    glob: &Glob<'_>,
) -> Outcome {
    let base = mem.len();
    let out = exec_frame(f, args, fuel, depth, mem, glob);
    // `exec_frame` cấp slot của khung NGAY SAU `base` (trước mọi lời gọi con)
    let end = (base + f.slots.len()).min(mem.len());
    glob.freed.borrow_mut().extend(base..end);
    out
}

fn exec_frame(
    f: &Func,
    args: &[Val],
    fuel: &mut u64,
    depth: u32,
    mem: &mut Vec<Vec<Byte>>,
    glob: &Glob<'_>,
) -> Outcome {
    let mut st = St {
        f,
        env: BTreeMap::new(),
        slot: BTreeMap::new(),
        mem,
        gl: &glob.gl,
        nro: glob.nro,
        freed: &glob.freed,
    };
    if args.len() != f.params.len() {
        return Outcome::Ub("sai số đối số".into());
    }
    for ((n, t), a) in f.params.iter().zip(args) {
        let v = match (t, a) {
            (t, Val::Int(x)) if t.is_int() => Val::Int(x & mask(t.bits().unwrap_or(128))),
            (Ty::Struct(_) | Ty::Array(..), Val::Agg(bs)) if bs.len() as u64 == layout(t).size => {
                Val::Agg(bs.clone())
            }
            (Ty::Ptr, Val::Ptr(a, o)) => Val::Ptr(*a, *o),
            _ => return Outcome::Ub("tham số không khớp kiểu".into()),
        };
        st.env.insert(n.clone(), v);
    }
    for (n, t) in &f.slots {
        let size = layout(t).size;
        if size > MAX_SLOT_BYTES {
            return Outcome::OutOfFuel;
        }
        st.slot.insert(n.clone(), st.mem.len());
        st.mem.push(vec![Byte::Uninit; size as usize]);
    }
    let ub = |m: String| Outcome::Ub(m);
    let mut cur = 0usize;
    let mut prev: Option<usize> = None;
    loop {
        let b = &st.f.blocks[cur];
        // phi chạy đồng thời theo khối tiền nhiệm
        let mut phis: Vec<(String, Val)> = Vec::new();
        for i in &b.insts {
            if let Inst::Phi { d, ins, .. } = i {
                let Some(p) = prev else {
                    return ub("phi ở khối vào".into());
                };
                let Some((v, _)) = ins.iter().find(|(_, bn)| *bn == st.f.blocks[p].name) else {
                    return ub("phi thiếu nguồn".into());
                };
                match st.opd(v) {
                    Ok(x) => phis.push((d.clone(), x)),
                    Err(e) => return ub(e),
                }
            }
        }
        for (d, v) in phis {
            st.env.insert(d, v);
        }
        for i in &b.insts {
            if *fuel == 0 {
                return Outcome::OutOfFuel;
            }
            *fuel -= 1;
            if let Inst::Call {
                d,
                callee,
                args: cargs,
                ..
            } = i
            {
                if depth >= 128 {
                    return Outcome::OutOfFuel;
                }
                let mut vals = Vec::new();
                for a in cargs {
                    match st.opd(a) {
                        Ok(v) => vals.push(v),
                        Err(e) => return ub(e),
                    }
                }
                let Some(g) = glob.funcs.get(callee.as_str()).copied() else {
                    match runtime_call(callee, &vals, st.mem, glob) {
                        Ok(v) => {
                            if let (Some(d), Some(v)) = (d, v) {
                                st.env.insert(d.clone(), v);
                            }
                        }
                        Err(o) => return o,
                    }
                    continue;
                };
                match exec(g, &vals, fuel, depth + 1, st.mem, glob) {
                    Outcome::Value(v) => {
                        if let (Some(d), Some(v)) = (d, v) {
                            st.env.insert(d.clone(), Val::Int(v));
                        }
                    }
                    Outcome::Ptr(a, o) => {
                        if let Some(d) = d {
                            st.env.insert(d.clone(), Val::Ptr(a, o));
                        }
                    }
                    Outcome::Bytes(bs) => {
                        if let Some(d) = d {
                            st.env.insert(d.clone(), Val::Agg(bs));
                        }
                    }
                    Outcome::Agg(bs) => {
                        if let Some(d) = d {
                            let bytes =
                                bs.iter().map(|b| b.map_or(Byte::Uninit, Byte::B)).collect();
                            st.env.insert(d.clone(), Val::Agg(bytes));
                        }
                    }
                    other => return other,
                }
                continue;
            }
            let r: Result<(), String> = (|| {
                match i {
                    Inst::Phi { .. } | Inst::Call { .. } => {}
                    Inst::Bin { d, op, ty, a, b } => {
                        let w = ty.bits().ok_or("bin không nguyên")?;
                        let v = binop(*op, w, st.int(a)?, st.int(b)?)?;
                        st.env.insert(d.clone(), Val::Int(v));
                    }
                    Inst::ICmp { d, pred, ty, a, b } => {
                        let v = if *ty == Ty::Ptr {
                            let (pa, pb) = (st.ptr(a)?, st.ptr(b)?);
                            match pred {
                                CmpPred::Eq => pa == pb,
                                CmpPred::Ne => pa != pb,
                                _ => return Err("so sánh thứ tự trên con trỏ".into()),
                            }
                        } else {
                            let w = ty.bits().ok_or("icmp không nguyên")?;
                            cmp(*pred, w, st.int(a)?, st.int(b)?)
                        };
                        st.env.insert(d.clone(), Val::Int(u128::from(v)));
                    }
                    Inst::Select { d, c, a, b, .. } => {
                        let v = if st.int(c)? & 1 == 1 {
                            st.opd(a)?
                        } else {
                            st.opd(b)?
                        };
                        st.env.insert(d.clone(), v);
                    }
                    Inst::Cast { d, op, from, v, to } => {
                        let (wf, wt) = (from.bits().ok_or("cast")?, to.bits().ok_or("cast")?);
                        let x = st.int(v)? & mask(wf);
                        let r = match op {
                            CastOp::Trunc => x & mask(wt),
                            CastOp::ZExt => x,
                            CastOp::SExt => (sext(x, wf) as u128) & mask(wt),
                        };
                        st.env.insert(d.clone(), Val::Int(r));
                    }
                    Inst::Load { d, ty, ptr } => {
                        let (a, off) = st.ptr(ptr)?;
                        let bs = st.read(a, off, ty)?;
                        let v = match ty {
                            Ty::Ptr => {
                                let (pa, po) = decode_ptr(&bs)?;
                                Val::Ptr(pa, po)
                            }
                            t if t.is_int() => Val::Int(decode_int(&bs, t.bits().unwrap_or(128))?),
                            Ty::Struct(_) | Ty::Array(..) => Val::Agg(bs),
                            _ => return Err("load kiểu không hỗ trợ".into()),
                        };
                        st.env.insert(d.clone(), v);
                    }
                    Inst::Store { ty, val, ptr } => {
                        let (a, off) = st.ptr(ptr)?;
                        let bytes = match (ty, st.opd(val)?) {
                            (Ty::Ptr, Val::Ptr(pa, po)) => {
                                (0..8u8).map(|k| Byte::P(pa, po, k)).collect()
                            }
                            (t, Val::Int(v)) if t.is_int() => int_bytes(v, layout(t).size),
                            (Ty::Struct(_) | Ty::Array(..), Val::Agg(bs))
                                if bs.len() as u64 == layout(ty).size =>
                            {
                                bs
                            }
                            _ => return Err("store: kiểu/giá trị không khớp".into()),
                        };
                        st.write(a, off, &bytes)?;
                    }
                    Inst::FieldPtr { d, ty, ptr, idx } => {
                        let (a, off) = st.ptr(ptr)?;
                        st.live(a)?;
                        let fo = field_offset(ty, *idx).ok_or("fieldptr ngoài struct")?;
                        let total = layout(ty).size;
                        if off
                            .checked_add(total)
                            .is_none_or(|e| e > st.mem[a].len() as u64)
                        {
                            return Err("fieldptr: struct vượt vùng".into());
                        }
                        st.env.insert(d.clone(), Val::Ptr(a, off + fo));
                    }
                    Inst::IndexPtr { d, elem, ptr, idx } => {
                        let (a, off) = st.ptr(ptr)?;
                        let i = sext(st.int(idx)?, 64);
                        let n = i128::from(layout(elem).size) * i;
                        // `gep inbounds null, 0` là hợp lệ ở LLVM (con trỏ của slice rỗng `{null, 0}` từ `Vec`): độ dời 0 giữ null
                        if a == NULL_REGION && n == 0 {
                            st.env.insert(d.clone(), Val::Ptr(a, off));
                            return Ok(());
                        }
                        st.live(a)?;
                        let no = i128::from(off) + n;
                        if no < 0 || no > st.mem[a].len() as i128 {
                            return Err("indexptr ra ngoài vùng".into());
                        }
                        st.env.insert(d.clone(), Val::Ptr(a, no as u64));
                    }
                }
                Ok(())
            })();
            if let Err(e) = r {
                return ub(e);
            }
        }
        let next = |n: &str| st.f.blocks.iter().position(|x| x.name == n);
        match &b.term {
            Term::Br(t) => {
                let Some(n) = next(t) else {
                    return ub("đích lạ".into());
                };
                prev = Some(cur);
                cur = n;
            }
            Term::CondBr(c, t, e) => {
                let v = match st.int(c) {
                    Ok(v) => v,
                    Err(e) => return ub(e),
                };
                let Some(n) = next(if v & 1 == 1 { t } else { e }) else {
                    return ub("đích lạ".into());
                };
                prev = Some(cur);
                cur = n;
            }
            Term::Ret(None) => return Outcome::Value(None),
            Term::Ret(Some((t, v))) => {
                if *t == Ty::Ptr {
                    return match st.opd(v) {
                        Ok(Val::Ptr(a, o)) => Outcome::Ptr(a, o),
                        Ok(_) => ub("ret ptr không phải con trỏ".into()),
                        Err(e) => ub(e),
                    };
                }
                if matches!(t, Ty::Struct(_) | Ty::Array(..)) {
                    return match st.opd(v) {
                        Ok(Val::Agg(bs)) => Outcome::Bytes(bs),
                        Ok(_) => ub("ret tổng hợp không phải giá trị tổng hợp".into()),
                        Err(e) => ub(e),
                    };
                }
                let w = t.bits().unwrap_or(128);
                return match st.int(v) {
                    Ok(x) => Outcome::Value(Some(x & mask(w))),
                    Err(e) => ub(e),
                };
            }
            Term::Abort(k) => return Outcome::Abort(k.clone()),
        }
        if *fuel == 0 {
            return Outcome::OutOfFuel;
        }
        *fuel -= 1;
    }
}
