#![allow(dead_code)]
//! Dựng CFG bằng tay cho test đặc tả dataflow (không qua bước hạ MIR) + PRNG tất định.
use gt_ast::NodeId;
use gt_mir::*;
use gt_span::{FileId, Span};
use gt_types::{IntTy, Ty};

pub fn src() -> SourceInfo {
    SourceInfo {
        span: Span::new(0, 0),
        node: NodeId::new(FileId::new(0), 0),
        scope: ScopeId(0),
    }
}

pub fn st(kind: StatementKind) -> Statement {
    Statement {
        kind,
        source: src(),
    }
}

pub fn i32t() -> Ty {
    Ty::Int(IntTy::I32)
}

pub fn l(i: u32) -> Place {
    Place::local(Local(i))
}

pub fn c(v: i128) -> Operand {
    Operand::Const(Const::Int {
        bits: (v as u128) & 0xffff_ffff,
        ty: IntTy::I32,
    })
}

pub fn assign(i: u32, rv: Rvalue) -> Statement {
    st(StatementKind::Assign(l(i), rv))
}

pub fn use_copy(i: u32) -> Rvalue {
    Rvalue::Use(Operand::Copy(l(i)))
}

/// Thân với `locals[0]` là chỗ trả về, `1..=args` là tham số, còn lại là tạm; khối = (câu lệnh, terminator).
pub fn body(locals: Vec<Ty>, args: usize, blocks: Vec<(Vec<Statement>, TerminatorKind)>) -> Body {
    let decls = locals
        .into_iter()
        .enumerate()
        .map(|(i, ty)| LocalDecl {
            ty,
            regions: Vec::new(),
            kind: if i == 0 {
                LocalKind::Return
            } else if i <= args {
                LocalKind::Arg(Some(format!("a{i}")))
            } else {
                LocalKind::Temp
            },
            mutable: true,
            span: Span::new(0, 0),
        })
        .collect();
    Body {
        key: "test::f".to_string(),
        node: NodeId::new(FileId::new(0), 0),
        sig_node: NodeId::new(FileId::new(0), 0),
        kind: BodyKind::Fn,
        arg_count: args,
        locals: decls,
        blocks: blocks
            .into_iter()
            .map(|(s, t)| BlockData {
                statements: s,
                terminator: Some(Terminator {
                    kind: t,
                    source: src(),
                }),
            })
            .collect(),
        borrows: Vec::new(),
        scopes: vec![SourceScope {
            parent: None,
            unsafe_ctx: false,
        }],
        region_count: 0,
        contracts: Vec::new(),
        fall_off: None,
        bad_literals: Vec::new(),
        uncheckable: Vec::new(),
        loops: Vec::new(),
        var_map: Vec::new(),
        drops_elaborated: false,
        span: Span::new(0, 0),
    }
}

pub fn bb(i: u32) -> BasicBlock {
    BasicBlock(i)
}

pub fn switch(on: u32, zero: u32, other: u32) -> TerminatorKind {
    TerminatorKind::SwitchInt {
        discr: Operand::Copy(l(on)),
        targets: vec![(0, bb(zero))],
        otherwise: bb(other),
    }
}

/// PRNG tất định (xorshift64*), không phụ thuộc thư viện ngoài.
pub struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 { 0 } else { self.next() % n }
    }
}
