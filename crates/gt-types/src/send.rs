//! `Send`-like (M8, ADR-026 §2): trait tự động suy THEO CẤU TRÚC. Một kiểu chuyển được sang luồng khác nếu mọi thành phần chuyển được.
//! Phase 1 bảo thủ (fail-closed): MỌI tham chiếu (`&T`, `&!T`) và con trỏ thô đều KHÔNG `Send` — chưa có cách nói `'static`/luồng có
//! phạm vi nên `&T` trỏ vào ngăn xếp người gọi sẽ treo khi luồng sống lâu hơn; chia sẻ an toàn sẽ qua kiểu đồng bộ (Mutex/atomics, sau).
//! `Arena` (vùng nhớ gắn với luồng tạo ra nó) không `Send`. Tham số kiểu chỉ `Send` khi có ràng buộc `Send` trong phạm vi.
use crate::collect::{AdtKind, Env, Lang};
use crate::traits::subst_with;
use crate::ty::Ty;
use gt_resolve::DefId;

impl Env<'_> {
    /// `Ok` nếu `ty` là `Send`-like; `Err(lý do)` chỉ ra thành phần vi phạm (đường đi từ ngoài vào trong).
    pub(crate) fn send_like(&self, ty: &Ty, scope: &[crate::traits::Bound]) -> Result<(), String> {
        let mut seen: Vec<(DefId, Vec<Ty>)> = Vec::new();
        self.send_rec(ty, scope, &mut seen, 0)
    }

    fn send_rec(
        &self,
        ty: &Ty,
        scope: &[crate::traits::Bound],
        seen: &mut Vec<(DefId, Vec<Ty>)>,
        depth: usize,
    ) -> Result<(), String> {
        if depth > 64 {
            return Err("kiểu lồng quá sâu để kiểm".to_string());
        }
        match ty {
            Ty::Int(_) | Ty::Float(_) | Ty::Bool | Ty::Char | Ty::Str | Ty::Never | Ty::Err => {
                Ok(())
            }
            Ty::Fn(..) => Ok(()),
            Ty::Tuple(v) => v
                .iter()
                .try_for_each(|t| self.send_rec(t, scope, seen, depth + 1)),
            Ty::Array(t, _) | Ty::Slice(t) => self.send_rec(t, scope, seen, depth + 1),
            Ty::Ref(m, _) => Err(format!(
                "chứa tham chiếu `{}` (Phase 1: không tham chiếu nào được chuyển sang luồng khác)",
                if *m { "&!" } else { "&" }
            )),
            Ty::RawPtr(..) => Err("chứa con trỏ thô".to_string()),
            Ty::Param(_) => {
                let send = self.db.lang.get(&Lang::Send);
                if scope
                    .iter()
                    .any(|b| &b.ty == ty && Some(&b.trait_def) == send)
                {
                    Ok(())
                } else {
                    Err("tham số kiểu chưa có ràng buộc `Send`".to_string())
                }
            }
            Ty::Infer(_) => Err("kiểu chưa xác định".to_string()),
            Ty::Adt(d, args) => {
                let Some(adt) = self.db.adts.get(d) else {
                    return Ok(());
                };
                if adt.lang == Some(Lang::Arena) {
                    return Err("`Arena` gắn với luồng tạo ra nó".to_string());
                }
                if seen.iter().any(|(sd, sa)| sd == d && sa == args) {
                    return Ok(());
                }
                seen.push((*d, args.clone()));
                let name = self.defs()[d.index()].path.clone();
                let fields: Vec<Ty> = match &adt.kind {
                    AdtKind::Struct(fs) => fs.iter().map(|f| f.ty.clone()).collect(),
                    AdtKind::Enum(vs) => vs.iter().flat_map(|v| v.fields.clone()).collect(),
                };
                for f in fields {
                    let ft = subst_with(&f, &adt.generics, args);
                    self.send_rec(&ft, scope, seen, depth + 1)
                        .map_err(|e| format!("`{name}` → {e}"))?;
                }
                Ok(())
            }
        }
    }
}
