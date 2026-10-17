//! Mục 2d (lệnh Giang 2026-10-17): M4 nay từ chối intrinsic dùng làm giá trị (E0313), nhưng nếu một `Const::FnItem` trỏ tới intrinsic (thân prelude KHÔNG có MIR) vẫn lọt xuống tới
//! codegen thì backend LLVM phải TỪ CHỐI (không sinh IR/exe) — không bao giờ để lại IR thoát Guard. Dựng trong bộ nhớ: đổi `node` của một `FnItem` hợp lệ thành node của `shl`.
#![cfg(feature = "llvm")]
use gt_driver::native::build_exe_profile;
use gt_driver::pipeline::{attestation, mir_dump};
use gt_driver::{Options, check};
use gt_mir::{Const, Operand, Rvalue, StatementKind};
use gt_resolve::SourceInput;

const SRC: &str =
    "ƒ inc(x: u32) → u32 ! ∅ { x +% 1 }\nƒ main() → u32 ! ∅ {\n  f ≔ inc;\n  f(1u32)\n}\n";

#[test]
fn an_intrinsic_fn_item_that_reaches_codegen_is_refused_without_producing_an_executable() {
    for opt in [
        gt_codegen_llvm::Opt::Dev,
        gt_codegen_llvm::Opt::Release,
        gt_codegen_llvm::Opt::ReleaseO2,
    ] {
        let inputs = [SourceInput {
            name: "main.gt".to_string(),
            text: SRC.to_string(),
        }];
        let mut res = check(&inputs, &Options::default()).expect("check");
        assert!(res.verdict.allows_output());
        let att = {
            let art = res.open_gate().expect("cổng");
            attestation(&res, &mir_dump(art))
        };
        let mut art = res.take_gated().expect("cổng");
        let shl = art
            .types
            .db
            .fns
            .values()
            .find(|(s, _)| s.name == "shl")
            .map(|(s, _)| s.node)
            .expect("shl");
        let mut changed = 0;
        for b in &mut art.obl.bodies {
            for bb in &mut b.blocks {
                for s in &mut bb.statements {
                    if let StatementKind::Assign(
                        _,
                        Rvalue::Use(Operand::Const(Const::FnItem { node, .. })),
                    ) = &mut s.kind
                        && b.key.ends_with("main")
                    {
                        *node = shl;
                        changed += 1;
                    }
                }
            }
        }
        assert_eq!(changed, 1, "không tìm thấy FnItem để đổi");
        let dir = std::env::temp_dir().join(format!("gt-ivc-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("tạo");
        let exe = dir.join(format!("ivc_{opt:?}.exe"));
        let _ = std::fs::remove_file(&exe);
        let r = build_exe_profile(&art, &exe, true, &att, opt);
        let msg = match &r {
            Err(e) => format!("{e:?}"),
            Ok(_) => String::new(),
        };
        assert!(
            r.is_err(),
            "{opt:?}: intrinsic FnItem lọt xuống codegen mà vẫn dựng được exe"
        );
        assert!(msg.contains("không có thân MIR"), "{opt:?}: {msg}");
        assert!(!exe.exists(), "{opt:?}: có exe dù bị từ chối");
    }
}
