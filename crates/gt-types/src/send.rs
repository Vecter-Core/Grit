//! `Send`-like và `Sync`-like (M8, ADR-026 §2, ADR-027 §2): trait TỰ ĐỘNG suy THEO CẤU TRÚC. Một kiểu chuyển được sang luồng khác (`Send`)
//! / chia sẻ giữa các luồng (`Sync`) nếu mọi thành phần đều như vậy.
//!
//! Phase 1 bảo thủ (fail-closed): MỌI tham chiếu và con trỏ thô đều KHÔNG `Send` — chưa có cách nói `'static`/luồng có phạm vi nên `&T`
//! trỏ vào ngăn xếp người gọi sẽ treo khi luồng sống lâu hơn (Q51). `&T`/`&!T` là `Sync` iff `T` là `Sync` (chia sẻ qua `■` static).
//! `Arena` (vùng nhớ gắn với luồng tạo ra nó) không `Send`/`Sync`. `Cell<T>`: `Send` iff `T: Send`, KHÔNG `Sync`. `Mutex<T, L>`: `Send` và
//! `Sync` iff `T: Send`. `Atomic<T>`: cả hai. Tham số kiểu chỉ thoả khi có ràng buộc tương ứng trong phạm vi.
use crate::collect::{AdtKind, Env, Lang};
use crate::traits::subst_with;
use crate::ty::Ty;
use gt_resolve::DefId;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Auto {
    Send,
    Sync,
}

impl Auto {
    fn lang(self) -> Lang {
        match self {
            Auto::Send => Lang::Send,
            Auto::Sync => Lang::Sync,
        }
    }
}

impl Env<'_> {
    /// `Ok` nếu `ty` là `Sync`-like.
    pub(crate) fn sync_like(&self, ty: &Ty, scope: &[crate::traits::Bound]) -> Result<(), String> {
        self.auto_like(ty, scope, Auto::Sync)
    }

    pub(crate) fn auto_like(
        &self,
        ty: &Ty,
        scope: &[crate::traits::Bound],
        which: Auto,
    ) -> Result<(), String> {
        let mut seen: Vec<(DefId, Vec<Ty>, Auto)> = Vec::new();
        self.auto_rec(ty, scope, which, &mut seen, 0)
    }

    fn auto_rec(
        &self,
        ty: &Ty,
        scope: &[crate::traits::Bound],
        which: Auto,
        seen: &mut Vec<(DefId, Vec<Ty>, Auto)>,
        depth: usize,
    ) -> Result<(), String> {
        if depth > 64 {
            return Err("kiểu lồng quá sâu để kiểm".to_string());
        }
        let name = if which == Auto::Send { "Send" } else { "Sync" };
        match ty {
            Ty::Int(_) | Ty::Float(_) | Ty::Bool | Ty::Char | Ty::Str | Ty::Never | Ty::Err => {
                Ok(())
            }
            Ty::Fn(..) => Ok(()),
            Ty::Tuple(v) => v
                .iter()
                .try_for_each(|t| self.auto_rec(t, scope, which, seen, depth + 1)),
            Ty::Array(t, _) | Ty::Slice(t) => self.auto_rec(t, scope, which, seen, depth + 1),
            Ty::Ref(m, inner) => match which {
                Auto::Send => Err(format!(
                    "chứa tham chiếu `{}` (Phase 1: không tham chiếu nào được chuyển sang luồng khác)",
                    if *m { "&!" } else { "&" }
                )),
                Auto::Sync => self.auto_rec(inner, scope, which, seen, depth + 1),
            },
            Ty::RawPtr(..) => Err("chứa con trỏ thô".to_string()),
            Ty::Param(_) => {
                let tr = self.db.lang.get(&which.lang());
                if scope
                    .iter()
                    .any(|b| &b.ty == ty && Some(&b.trait_def) == tr)
                {
                    Ok(())
                } else {
                    Err(format!("tham số kiểu chưa có ràng buộc `{name}`"))
                }
            }
            Ty::Infer(_) => Err("kiểu chưa xác định".to_string()),
            Ty::Adt(d, args) => {
                let Some(adt) = self.db.adts.get(d) else {
                    return Ok(());
                };
                match adt.lang {
                    Some(Lang::Arena) => {
                        return Err("`Arena` gắn với luồng tạo ra nó".to_string());
                    }
                    Some(Lang::Cell) if which == Auto::Sync => {
                        return Err("`Cell` có mutability bên trong không đồng bộ".to_string());
                    }
                    // Mutex<T, L>: Send và Sync iff T: Send (L là kiểu đánh dấu cấp khoá, không phải dữ liệu)
                    Some(Lang::Mutex) => {
                        return match args.first() {
                            Some(t) => self
                                .auto_rec(t, scope, Auto::Send, seen, depth + 1)
                                .map_err(|e| format!("`Mutex` → {e}")),
                            None => Ok(()),
                        };
                    }
                    // Atomic<T>: T chỉ là bool/số nguyên
                    Some(Lang::Atomic) => return Ok(()),
                    _ => {}
                }
                if seen
                    .iter()
                    .any(|(sd, sa, sw)| sd == d && sa == args && *sw == which)
                {
                    return Ok(());
                }
                seen.push((*d, args.clone(), which));
                let dname = self.defs()[d.index()].path.clone();
                let fields: Vec<Ty> = match &adt.kind {
                    AdtKind::Struct(fs) => fs.iter().map(|f| f.ty.clone()).collect(),
                    AdtKind::Enum(vs) => vs.iter().flat_map(|v| v.fields.clone()).collect(),
                };
                for f in fields {
                    let ft = subst_with(&f, &adt.generics, args);
                    self.auto_rec(&ft, scope, which, seen, depth + 1)
                        .map_err(|e| format!("`{dname}` → {e}"))?;
                }
                Ok(())
            }
        }
    }
}
