//! Thu thập định nghĩa, bảng tên theo module, nhập (`⇲`), tra cứu có kiểm khả kiến, cạnh phụ thuộc module (ADR-017 d).
use crate::model::{Def, DefId, DefKind, LocalId, Res};
use crate::prelude::{PRELUDE_FILE_NAME, Prelude, prelude_file};
use gt_ast::{Item, ItemKind, NodeId, Program, SegmentKind, Visibility};
use gt_diag::{Code, Diagnostic, Diagnostics, Report};
use gt_span::{FileId, Span};
use std::collections::BTreeMap;

/// Giới hạn số chẩn đoán (D19) — cùng giá trị với các crate trước.
pub(crate) const LIMIT: usize = 100;

#[derive(Clone, Copy, Debug)]
pub(crate) enum Entry {
    Def(DefId),
    /// Chỉ số trong `Tables::uses`.
    Use(usize),
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum UseState {
    Todo,
    Busy,
    /// `None` = đã thất bại (đã báo lỗi).
    Done(Option<DefId>),
}

/// Một tên được nhập bởi `⇲` (đã bung dạng nhóm: mỗi tên một khai báo).
#[derive(Clone, Debug)]
pub(crate) struct UseDecl {
    pub file: FileId,
    /// Vị trí báo lỗi (cả đường dẫn, hoặc tên trong nhóm).
    pub span: Span,
    pub segs: Vec<String>,
    pub seg_spans: Vec<Span>,
    pub public: bool,
    /// Module chứa khai báo `⇲`.
    pub module: DefId,
    pub state: UseState,
}

/// Chẩn đoán kèm vị trí logic (đường dẫn item bao) — để so sánh bất biến thứ tự.
#[derive(Debug)]
pub(crate) struct Logical {
    pub diag: Diagnostic,
    pub site: String,
}

pub(crate) struct Tables {
    pub defs: Vec<Def>,
    /// Bảng tên của mỗi định nghĩa (module và enum có nội dung; còn lại rỗng), chỉ mục theo `DefId`.
    pub tables: Vec<BTreeMap<String, Vec<Entry>>>,
    pub uses: Vec<UseDecl>,
    pub root: DefId,
    pub prelude: DefId,
    /// Tên tệp theo `FileId`.
    pub file_names: Vec<String>,
    pub item_def: BTreeMap<NodeId, DefId>,
    pub diags: Vec<Logical>,
    /// Cạnh phụ thuộc module (S → T) kèm các vị trí tham chiếu.
    pub edges: BTreeMap<(DefId, DefId), Vec<(FileId, Span)>>,
    pub canon: Vec<String>,
    /// Ngăn xếp các khai báo nhập đang được phân giải (phát hiện vòng nhập).
    pub busy: Vec<usize>,
    /// Kết quả phân giải từng vị trí dùng và khai báo cục bộ.
    pub res: BTreeMap<NodeId, Res>,
    pub decls: BTreeMap<NodeId, LocalId>,
    /// Văn bản nguồn theo `FileId` (để dựng khoá logic cho `⊐`).
    pub texts: Vec<String>,
}

/// Kết quả đi dọc các đoạn sau đoạn đầu.
#[derive(Debug)]
pub(crate) enum Walked {
    Def(DefId),
    /// Dừng ở một kiểu; đoạn thứ `i` (chỉ số trong danh sách đoạn còn lại) trở đi là mục kết hợp.
    Assoc(DefId, usize),
}

#[derive(Debug)]
pub(crate) enum WalkErr {
    Missing {
        container: DefId,
        name: String,
        at: usize,
    },
    Hidden {
        name: String,
        site: (FileId, Span),
        owner: DefId,
        at: usize,
    },
    NotContainer {
        def: DefId,
        name: String,
        at: usize,
    },
    /// Đã có lỗi gốc (trùng tên, nhập hỏng…): không báo thêm.
    Silent,
}

pub(crate) enum Found {
    Def(DefId),
    Hidden { site: (FileId, Span), owner: DefId },
    Missing,
    Silent,
}

fn join(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_string()
    } else {
        format!("{prefix}::{name}")
    }
}

pub(crate) fn seg_name(k: &SegmentKind) -> String {
    match k {
        SegmentKind::Name(n) => n.clone(),
        SegmentKind::SelfType => "Self".to_string(),
        SegmentKind::SelfValue => "self".to_string(),
    }
}

impl Tables {
    pub fn new(file_names: Vec<String>, texts: Vec<String>) -> Tables {
        let mut t = Tables {
            defs: Vec::new(),
            tables: Vec::new(),
            uses: Vec::new(),
            root: DefId::from_index(0),
            prelude: DefId::from_index(1),
            file_names,
            item_def: BTreeMap::new(),
            diags: Vec::new(),
            edges: BTreeMap::new(),
            canon: Vec::new(),
            busy: Vec::new(),
            res: BTreeMap::new(),
            decls: BTreeMap::new(),
            texts,
        };
        for path in ["", "$prelude"] {
            t.defs.push(Def {
                path: path.to_string(),
                kind: DefKind::Module,
                file: prelude_file(),
                name_span: Span::new(0, 0),
                node: NodeId::new(prelude_file(), 0),
                public: true,
                parent: None,
                has_fields: false,
            });
            t.tables.push(BTreeMap::new());
        }
        t
    }

    pub fn fname(&self, f: FileId) -> String {
        if f == prelude_file() {
            PRELUDE_FILE_NAME.to_string()
        } else {
            self.file_names
                .get(f.index())
                .cloned()
                .unwrap_or_else(|| "?".to_string())
        }
    }

    // ---------------- thu thập ----------------

    #[allow(clippy::too_many_arguments)]
    fn add_def(
        &mut self,
        module: DefId,
        file: FileId,
        name: &str,
        name_span: Span,
        node: NodeId,
        kind: DefKind,
        public: bool,
    ) -> DefId {
        let path = join(&self.defs[module.index()].path, name);
        let id = DefId::from_index(self.defs.len());
        self.defs.push(Def {
            path,
            kind,
            file,
            name_span,
            node,
            public,
            parent: Some(module),
            has_fields: false,
        });
        self.tables.push(BTreeMap::new());
        self.tables[module.index()]
            .entry(name.to_string())
            .or_default()
            .push(Entry::Def(id));
        id
    }

    /// Tạo module-tệp dưới gốc và thu thập các item của nó.
    pub fn collect_file(&mut self, file: FileId, module_name: &str, program: &Program) {
        let m = self.add_def(
            self.root,
            file,
            module_name,
            Span::new(0, 0),
            program.id,
            DefKind::Module,
            true,
        );
        self.item_def.insert(program.id, m);
        self.collect_items(&program.items, m, file, false);
    }

    /// Thu thập prelude vào module giả `$prelude`.
    pub fn collect_prelude(&mut self, prelude: &Prelude) {
        let p = self.prelude;
        self.collect_items(&prelude.program.items, p, prelude_file(), true);
    }

    fn collect_items(&mut self, items: &[Item], module: DefId, file: FileId, prelude_top: bool) {
        for item in items {
            let public = prelude_top || item.vis == Visibility::Public;
            match &item.kind {
                ItemKind::Fn(f) => {
                    let n = &f.sig.name;
                    let d =
                        self.add_def(module, file, &n.name, n.span, item.id, DefKind::Fn, public);
                    self.item_def.insert(item.id, d);
                }
                ItemKind::Struct(s) => {
                    let d = self.add_def(
                        module,
                        file,
                        &s.name.name,
                        s.name.span,
                        item.id,
                        DefKind::Struct,
                        public,
                    );
                    self.item_def.insert(item.id, d);
                }
                ItemKind::Enum(e) => {
                    let d = self.add_def(
                        module,
                        file,
                        &e.name.name,
                        e.name.span,
                        item.id,
                        DefKind::Enum,
                        public,
                    );
                    self.item_def.insert(item.id, d);
                    for v in &e.variants {
                        let vid = DefId::from_index(self.defs.len());
                        let path = join(&self.defs[d.index()].path, &v.name.name);
                        self.defs.push(Def {
                            path,
                            kind: DefKind::Variant,
                            file,
                            name_span: v.name.span,
                            node: v.id,
                            public,
                            parent: Some(d),
                            has_fields: !v.fields.is_empty(),
                        });
                        self.tables.push(BTreeMap::new());
                        self.tables[d.index()]
                            .entry(v.name.name.clone())
                            .or_default()
                            .push(Entry::Def(vid));
                        // biến thể cũng là tên trong không gian tên của module chứa enum (Q19): `Circle(r)` trần hợp lệ như `Some(v)`
                        self.tables[module.index()]
                            .entry(v.name.name.clone())
                            .or_default()
                            .push(Entry::Def(vid));
                    }
                }
                ItemKind::Trait(t) => {
                    let d = self.add_def(
                        module,
                        file,
                        &t.name.name,
                        t.name.span,
                        item.id,
                        DefKind::Trait,
                        public,
                    );
                    self.item_def.insert(item.id, d);
                }
                ItemKind::TypeAlias(t) => {
                    let d = self.add_def(
                        module,
                        file,
                        &t.name.name,
                        t.name.span,
                        item.id,
                        DefKind::TypeAlias,
                        public,
                    );
                    self.item_def.insert(item.id, d);
                }
                ItemKind::Const(c) => {
                    let d = self.add_def(
                        module,
                        file,
                        &c.name.name,
                        c.name.span,
                        item.id,
                        DefKind::Const,
                        public,
                    );
                    self.item_def.insert(item.id, d);
                }
                ItemKind::Static(c) => {
                    let d = self.add_def(
                        module,
                        file,
                        &c.name.name,
                        c.name.span,
                        item.id,
                        DefKind::Static,
                        public,
                    );
                    self.item_def.insert(item.id, d);
                }
                ItemKind::Mod(m) => {
                    let d = self.add_def(
                        module,
                        file,
                        &m.name.name,
                        m.name.span,
                        item.id,
                        DefKind::Module,
                        public,
                    );
                    self.item_def.insert(item.id, d);
                    self.collect_items(&m.items, d, file, false);
                }
                ItemKind::Impl(_) => {}
                ItemKind::Use(u) => {
                    let segs: Vec<String> =
                        u.path.segments.iter().map(|s| seg_name(&s.kind)).collect();
                    let seg_spans: Vec<Span> = u.path.segments.iter().map(|s| s.span).collect();
                    if u.names.is_empty() {
                        let name = segs.last().cloned().unwrap_or_default();
                        self.add_use(module, file, u.path.span, segs, seg_spans, public, name);
                    } else {
                        for n in &u.names {
                            let mut s = segs.clone();
                            s.push(n.name.clone());
                            let mut sp = seg_spans.clone();
                            sp.push(n.span);
                            self.add_use(module, file, n.span, s, sp, public, n.name.clone());
                        }
                    }
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn add_use(
        &mut self,
        module: DefId,
        file: FileId,
        span: Span,
        segs: Vec<String>,
        seg_spans: Vec<Span>,
        public: bool,
        name: String,
    ) {
        let idx = self.uses.len();
        self.uses.push(UseDecl {
            file,
            span,
            segs,
            seg_spans,
            public,
            module,
            state: UseState::Todo,
        });
        self.tables[module.index()]
            .entry(name)
            .or_default()
            .push(Entry::Use(idx));
    }

    /// Gán `DefId` cuối cùng theo (đường dẫn, loại, tên tệp, vị trí) — KHÔNG theo thứ tự khai báo — rồi sửa mọi tham chiếu.
    pub fn finalize_order(&mut self) {
        let n = self.defs.len();
        let mut order: Vec<usize> = (0..n).collect();
        {
            let names: Vec<String> = self.defs.iter().map(|d| self.fname(d.file)).collect();
            let defs = &self.defs;
            order.sort_by(|&a, &b| {
                let (x, y) = (&defs[a], &defs[b]);
                (
                    &x.path,
                    x.kind,
                    &names[a],
                    x.name_span.start,
                    x.name_span.end,
                )
                    .cmp(&(
                        &y.path,
                        y.kind,
                        &names[b],
                        y.name_span.start,
                        y.name_span.end,
                    ))
            });
        }
        let mut new_id = vec![DefId::from_index(0); n];
        for (new, &old) in order.iter().enumerate() {
            new_id[old] = DefId::from_index(new);
        }
        let old_defs = std::mem::take(&mut self.defs);
        let mut old_tables = std::mem::take(&mut self.tables);
        for &old in &order {
            let mut d = old_defs[old].clone();
            d.parent = d.parent.map(|p| new_id[p.index()]);
            self.defs.push(d);
            let mut t = std::mem::take(&mut old_tables[old]);
            for entries in t.values_mut() {
                for e in entries.iter_mut() {
                    if let Entry::Def(d) = e {
                        *d = new_id[d.index()];
                    }
                }
            }
            self.tables.push(t);
        }
        for u in &mut self.uses {
            u.module = new_id[u.module.index()];
        }
        self.root = new_id[self.root.index()];
        self.prelude = new_id[self.prelude.index()];
        for v in self.item_def.values_mut() {
            *v = new_id[v.index()];
        }
    }

    // ---------------- chẩn đoán ----------------

    pub fn push(&mut self, code: &str, diag: Diagnostic, site: &str) {
        let _ = code;
        self.diags.push(Logical {
            diag,
            site: site.to_string(),
        });
    }

    pub fn mk(&self, code: &str, file: FileId, span: Span, msg: String) -> Option<Diagnostic> {
        Code::parse(code)
            .ok()
            .map(|c| Diagnostic::new(c, &self.fname(file), span, msg))
    }

    /// E0202 cho mọi tên có >1 khai báo trong cùng một module/enum (trừ prelude).
    pub fn report_duplicates(&mut self) {
        for c in 0..self.tables.len() {
            if DefId::from_index(c) == self.prelude {
                continue;
            }
            let dups: Vec<(String, Vec<Entry>)> = self.tables[c]
                .iter()
                .filter(|(_, v)| v.len() > 1)
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            for (name, entries) in dups {
                // trùng biến thể trong CÙNG một enum đã được báo ở bảng của enum
                let parents: Vec<Option<DefId>> = entries
                    .iter()
                    .map(|e| match e {
                        Entry::Def(d) if self.defs[d.index()].kind == DefKind::Variant => {
                            self.defs[d.index()].parent
                        }
                        _ => None,
                    })
                    .collect();
                if self.defs[c].kind == DefKind::Module
                    && parents[0].is_some()
                    && parents.iter().all(|p| *p == parents[0])
                {
                    continue;
                }
                self.dup_diag(DefId::from_index(c), &name, &entries);
            }
        }
    }

    fn entry_site(&self, e: &Entry) -> (FileId, Span) {
        match e {
            Entry::Def(d) => (self.defs[d.index()].file, self.defs[d.index()].name_span),
            Entry::Use(i) => (self.uses[*i].file, self.uses[*i].span),
        }
    }

    fn dup_diag(&mut self, container: DefId, name: &str, entries: &[Entry]) {
        let mut sites: Vec<(String, FileId, Span)> = entries
            .iter()
            .map(|e| {
                let (f, s) = self.entry_site(e);
                (self.fname(f), f, s)
            })
            .collect();
        sites.sort_by(|a, b| (&a.0, a.2.start, a.2.end).cmp(&(&b.0, b.2.start, b.2.end)));
        let path = self.defs[container.index()].path.clone();
        let k = sites.len();
        let msg = if self.defs[container.index()].kind == DefKind::Enum {
            format!("biến thể `{name}` được khai báo {k} lần trong enum `{path}`")
        } else {
            format!("tên `{name}` được định nghĩa {k} lần trong module `{path}`")
        };
        let (_, f0, s0) = sites[0].clone();
        if let Some(mut d) = self.mk("E0202", f0, s0, msg) {
            d = d.with_span_label(format!("một trong {k} định nghĩa của `{name}`"));
            for (fname, _, s) in sites.iter().skip(1) {
                d = d.with_label(fname, *s, format!("định nghĩa khác của `{name}`"));
            }
            self.push("E0202", d, &path);
        }
    }

    // ---------------- khả kiến và tra cứu ----------------

    /// `a` nằm trong `anc` (bằng hoặc là hậu duệ)?
    pub fn is_within(&self, a: DefId, anc: DefId) -> bool {
        let mut cur = Some(a);
        while let Some(c) = cur {
            if c == anc {
                return true;
            }
            cur = self.defs[c.index()].parent;
        }
        false
    }

    fn visible(&self, from: DefId, owner: DefId, public: bool) -> bool {
        public || self.is_within(from, owner)
    }

    /// Module "nhà" của một định nghĩa (chính nó nếu là module).
    pub fn home_module(&self, d: DefId) -> DefId {
        let mut cur = d;
        loop {
            if self.defs[cur.index()].kind == DefKind::Module {
                return cur;
            }
            match self.defs[cur.index()].parent {
                Some(p) => cur = p,
                None => return cur,
            }
        }
    }

    /// Ghi cạnh phụ thuộc `from` → module nhà của `target` (ADR-017 mục 6).
    pub fn note_edge(&mut self, from: DefId, target: DefId, file: FileId, span: Span) {
        let t = self.home_module(target);
        if self.is_within(t, from) {
            return; // chính nó hoặc hậu duệ
        }
        // prelude (và thành viên của nó) không tạo cạnh
        if self.is_within(t, self.prelude) {
            return;
        }
        self.edges.entry((from, t)).or_default().push((file, span));
    }

    /// Tra `name` trong bảng của `container`. `from = Some(m)` kiểm khả kiến từ module `m`.
    pub fn lookup_in(&mut self, container: DefId, name: &str, from: Option<DefId>) -> Found {
        let Some(entries) = self.tables[container.index()].get(name).cloned() else {
            return Found::Missing;
        };
        if entries.len() > 1 {
            return Found::Silent;
        }
        match entries[0] {
            Entry::Def(d) => {
                let public = self.defs[d.index()].public;
                // biến thể của enum: khả kiến tính theo module chứa enum
                let owner = if self.defs[container.index()].kind == DefKind::Enum {
                    self.defs[container.index()].parent.unwrap_or(container)
                } else {
                    container
                };
                if from.is_none_or(|f| self.visible(f, owner, public)) {
                    Found::Def(d)
                } else {
                    Found::Hidden {
                        site: (self.defs[d.index()].file, self.defs[d.index()].name_span),
                        owner,
                    }
                }
            }
            Entry::Use(i) => {
                let (ipub, imod, ifile, ispan) = {
                    let u = &self.uses[i];
                    (u.public, u.module, u.file, u.span)
                };
                let target = self.resolve_use(i);
                if !from.is_none_or(|f| self.visible(f, imod, ipub)) {
                    return Found::Hidden {
                        site: (ifile, ispan),
                        owner: imod,
                    };
                }
                match target {
                    Some(t) => Found::Def(t),
                    None => Found::Silent,
                }
            }
        }
    }

    /// Đi dọc `rest` bắt đầu từ định nghĩa `cur`; kiểm khả kiến từ module `from`.
    pub fn walk_rest(
        &mut self,
        mut cur: DefId,
        rest: &[String],
        from: DefId,
    ) -> Result<Walked, WalkErr> {
        for (i, seg) in rest.iter().enumerate() {
            match self.defs[cur.index()].kind {
                DefKind::Module => match self.lookup_in(cur, seg, Some(from)) {
                    Found::Def(d) => cur = d,
                    Found::Hidden { site, owner } => {
                        return Err(WalkErr::Hidden {
                            name: seg.clone(),
                            site,
                            owner,
                            at: i,
                        });
                    }
                    Found::Missing => {
                        return Err(WalkErr::Missing {
                            container: cur,
                            name: seg.clone(),
                            at: i,
                        });
                    }
                    Found::Silent => return Err(WalkErr::Silent),
                },
                DefKind::Enum => match self.lookup_in(cur, seg, Some(from)) {
                    Found::Def(d) => cur = d,
                    Found::Missing => return Ok(Walked::Assoc(cur, i)),
                    Found::Hidden { site, owner } => {
                        return Err(WalkErr::Hidden {
                            name: seg.clone(),
                            site,
                            owner,
                            at: i,
                        });
                    }
                    Found::Silent => return Err(WalkErr::Silent),
                },
                DefKind::Struct | DefKind::TypeAlias | DefKind::Trait => {
                    return Ok(Walked::Assoc(cur, i));
                }
                DefKind::Fn | DefKind::Const | DefKind::Static | DefKind::Variant => {
                    return Err(WalkErr::NotContainer {
                        def: cur,
                        name: seg.clone(),
                        at: i,
                    });
                }
            }
        }
        Ok(Walked::Def(cur))
    }

    /// Phân giải một khai báo nhập (một lần; kết quả nhớ lại). Báo lỗi E0201/E0203 tại chỗ `⇲`.
    pub fn resolve_use(&mut self, i: usize) -> Option<DefId> {
        match self.uses[i].state {
            UseState::Done(r) => return r,
            UseState::Busy => {
                // vòng nhập: a = ⇲ b::a, b = ⇲ a::a — báo MỘT lần, ở thành viên nhỏ nhất của vòng (độc lập thứ tự)
                self.report_import_cycle(i);
                return None;
            }
            UseState::Todo => {}
        }
        self.uses[i].state = UseState::Busy;
        self.busy.push(i);
        let u = self.uses[i].clone();
        let site = format!("{} ⇲", self.defs[u.module.index()].path);
        let result = self.resolve_use_inner(&u, &site);
        self.busy.pop();
        if let Some(t) = result {
            self.note_edge(u.module, t, u.file, u.span);
        }
        self.uses[i].state = UseState::Done(result);
        let line = format!(
            "{} import {} -> {}",
            self.defs[u.module.index()].path,
            u.segs.join("::"),
            result.map_or("<lỗi>".to_string(), |t| self.defs[t.index()].path.clone())
        );
        self.canon.push(line);
        result
    }

    /// Vòng nhập: các khai báo từ `i` trở đi trong ngăn xếp phụ thuộc lẫn nhau → E0201 một lần.
    fn report_import_cycle(&mut self, i: usize) {
        let Some(pos) = self.busy.iter().position(|&b| b == i) else {
            return;
        };
        let mut members: Vec<usize> = self.busy[pos..].to_vec();
        let key = |t: &Tables, m: usize| {
            let u = &t.uses[m];
            (t.defs[u.module.index()].path.clone(), u.segs.join("::"))
        };
        members.sort_by_key(|&m| key(self, m));
        let names: Vec<String> = members
            .iter()
            .map(|&m| {
                format!(
                    "`{}` (⇲ {})",
                    self.defs[self.uses[m].module.index()].path,
                    self.uses[m].segs.join("::")
                )
            })
            .collect();
        let first = members[0];
        let (file, span, module) = {
            let u = &self.uses[first];
            (u.file, u.span, u.module)
        };
        let msg = format!(
            "các tên nhập phụ thuộc vòng vào nhau nên không phân giải được: {}",
            names.join(", ")
        );
        if let Some(mut d) = self.mk("E0201", file, span, msg) {
            d = d.with_span_label("vòng nhập");
            for &m in members.iter().skip(1) {
                let (f, sp) = (self.uses[m].file, self.uses[m].span);
                let fname = self.fname(f);
                d = d.with_label(&fname, sp, "thành viên khác của vòng nhập");
            }
            let site = format!("{} ⇲", self.defs[module.index()].path);
            self.push("E0201", d, &site);
        }
    }

    fn resolve_use_inner(&mut self, u: &UseDecl, site: &str) -> Option<DefId> {
        let first = u.segs.first()?.clone();
        // đoạn đầu: tên module-tệp (gốc), rồi prelude
        let start = match self.lookup_in(self.root, &first, None) {
            Found::Def(d) => Some(d),
            _ => match self.lookup_in(self.prelude, &first, None) {
                Found::Def(d) => Some(d),
                _ => None,
            },
        };
        let Some(start) = start else {
            self.err_first_missing(u.file, u.seg_spans[0], &first, site);
            return None;
        };
        match self.walk_rest(start, &u.segs[1..], u.module) {
            Ok(Walked::Def(d)) => Some(d),
            Ok(Walked::Assoc(_, at)) => {
                // `⇲ Kiểu::mục` — mục kết hợp không nhập được
                let span = u.seg_spans.get(at + 1).copied().unwrap_or(u.span);
                let msg = format!(
                    "không thể nhập `{}`: mục kết hợp của kiểu không phải item để nhập",
                    u.segs[at + 1..].join("::")
                );
                if let Some(d) = self.mk("E0201", u.file, span, msg) {
                    self.push("E0201", d, site);
                }
                None
            }
            Err(e) => {
                self.report_walk_err(u.file, &u.seg_spans, 1, &e, site);
                None
            }
        }
    }

    pub fn err_first_missing(&mut self, file: FileId, span: Span, name: &str, site: &str) {
        if let Some(d) = self.mk(
            "E0201",
            file,
            span,
            format!("tên `{name}` chưa được định nghĩa"),
        ) {
            self.push(
                "E0201",
                d.with_span_label("không tìm thấy trong phạm vi này"),
                site,
            );
        }
    }

    /// `base` = chỉ số đoạn của `rest[0]` trong danh sách đoạn đầy đủ (để lấy đúng span).
    pub fn report_walk_err(
        &mut self,
        file: FileId,
        seg_spans: &[Span],
        base: usize,
        e: &WalkErr,
        site: &str,
    ) {
        match e {
            WalkErr::Silent => {}
            WalkErr::Missing {
                container,
                name,
                at,
            } => {
                let span = seg_spans.get(base + at).copied().unwrap_or(Span::new(0, 0));
                let path = self.defs[container.index()].path.clone();
                let msg = format!("`{path}` không có thành viên `{name}`");
                if let Some(d) = self.mk("E0201", file, span, msg) {
                    self.push("E0201", d.with_span_label("không tìm thấy"), site);
                }
            }
            WalkErr::NotContainer { def, name, at } => {
                let span = seg_spans.get(base + at).copied().unwrap_or(Span::new(0, 0));
                let path = self.defs[def.index()].path.clone();
                let kind = self.defs[def.index()].kind.as_str();
                let msg = format!(
                    "`{path}` là {kind}, không phải module hoặc kiểu nên không có thành viên `{name}`"
                );
                if let Some(d) = self.mk("E0201", file, span, msg) {
                    self.push("E0201", d, site);
                }
            }
            WalkErr::Hidden {
                name,
                site: (df, ds),
                owner,
                at,
            } => {
                let span = seg_spans.get(base + at).copied().unwrap_or(Span::new(0, 0));
                let owner_path = self.defs[owner.index()].path.clone();
                let msg = format!("`{name}` là private trong module `{owner_path}` (thiếu `◈`)");
                let fname = self.fname(*df);
                if let Some(d) = self.mk("E0203", file, span, msg) {
                    let d = d.with_span_label("private").with_label(
                        &fname,
                        *ds,
                        "định nghĩa ở đây, thiếu `◈`",
                    );
                    self.push("E0203", d, site);
                }
            }
        }
    }

    pub fn resolve_all_uses(&mut self) {
        for i in 0..self.uses.len() {
            self.resolve_use(i);
        }
    }
}

/// Kiểm prelude: tên trùng (kể cả biến thể trần trùng item) → báo cáo.
pub(crate) fn prelude_duplicates(p: &Prelude) -> Report {
    let mut t = Tables::new(Vec::new(), Vec::new());
    t.collect_prelude(p);
    t.finalize_order();
    let prelude = t.prelude;
    let names: Vec<String> = t.tables[prelude.index()]
        .iter()
        .filter(|(_, v)| v.len() > 1)
        .map(|(k, _)| k.clone())
        .collect();
    let mut bag = Diagnostics::new(LIMIT);
    for name in names {
        let entries = t.tables[prelude.index()][&name].clone();
        let (f, s) = t.entry_site(&entries[0]);
        if let Some(d) = t.mk(
            "E0202",
            f,
            s,
            format!(
                "prelude: tên `{name}` được định nghĩa {} lần",
                entries.len()
            ),
        ) {
            bag.push(d);
        }
    }
    // biến thể trùng trong cùng một enum của prelude
    for c in 0..t.tables.len() {
        if DefId::from_index(c) == prelude {
            continue;
        }
        let dups: Vec<String> = t.tables[c]
            .iter()
            .filter(|(_, v)| v.len() > 1)
            .map(|(k, _)| k.clone())
            .collect();
        for name in dups {
            let entries = t.tables[c][&name].clone();
            let (f, s) = t.entry_site(&entries[0]);
            if let Some(d) = t.mk(
                "E0202",
                f,
                s,
                format!("prelude: `{name}` trùng trong `{}`", t.defs[c].path),
            ) {
                bag.push(d);
            }
        }
    }
    bag.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FAIL_OPEN_AUDIT S-C: `mk` với mã sai định dạng không được trả `None` (chẩn đoán biến mất).
    #[test]
    fn mk_with_a_malformed_code_still_yields_a_diagnostic() {
        let t = Tables::new(vec!["a.gt".to_string()], vec![String::new()]);
        let d = t
            .mk("E02", FileId::new(0), Span::new(0, 0), "thử".to_string())
            .expect("chẩn đoán bị nuốt");
        assert_eq!(d.code.as_str(), "E9901");
    }
}
