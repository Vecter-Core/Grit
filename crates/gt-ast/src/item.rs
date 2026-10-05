//! Item: hàm, struct, enum, trait, impl, mod, use, type, const, static — kèm doc comment và attribute.
use crate::expr::{Block, Expr};
use crate::node::{NodeId, impl_node};
use crate::ty::Type;
use gt_span::Span;

/// Tên (định danh). Không bao giờ chứa từ khoá cứng hay ký hiệu dành riêng.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ident {
    pub id: NodeId,
    pub span: Span,
    pub name: String,
}

/// Lifetime/region có tên; `name` KHÔNG gồm dấu `'`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lifetime {
    pub id: NodeId,
    pub span: Span,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SegmentKind {
    Name(String),
    /// `Self` đứng đầu đường dẫn (`Self { … }`, `Self::f`).
    SelfType,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathSegment {
    pub id: NodeId,
    pub span: Span,
    pub kind: SegmentKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Path {
    pub id: NodeId,
    pub span: Span,
    pub segments: Vec<PathSegment>,
}

/// Cả tệp nguồn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Program {
    pub id: NodeId,
    pub span: Span,
    pub items: Vec<Item>,
}

/// Doc comment `///` (văn bản đã bỏ `///` và một khoảng trắng đầu).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocComment {
    pub id: NodeId,
    pub span: Span,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AttrArgKind {
    Ident(String),
    /// Văn bản nguồn nguyên văn giữa hai dấu nháy (chưa giải mã escape).
    Str(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttrArg {
    pub id: NodeId,
    pub span: Span,
    pub kind: AttrArgKind,
}

/// `#tên(đối số, …)`, ví dụ `#allow(S101, "lý do")`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attribute {
    pub id: NodeId,
    pub span: Span,
    pub name: Ident,
    pub args: Vec<AttrArg>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Visibility {
    Private,
    Public,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub id: NodeId,
    pub span: Span,
    pub docs: Vec<DocComment>,
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub kind: ItemKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ItemKind {
    Fn(FnItem),
    Struct(StructItem),
    Enum(EnumItem),
    Trait(TraitItem),
    Impl(ImplItem),
    Mod(ModItem),
    Use(UseItem),
    TypeAlias(TypeAliasItem),
    Const(ConstItem),
    Static(StaticItem),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Generics {
    pub id: NodeId,
    pub span: Span,
    pub params: Vec<GenericParam>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GenericParam {
    pub id: NodeId,
    pub span: Span,
    pub kind: GenericParamKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GenericParamKind {
    /// `T` hoặc `T: A + B` (ràng buộc trait inline, Q7).
    Type {
        name: Ident,
        bounds: Vec<Path>,
    },
    Lifetime(Lifetime),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelfKind {
    /// `self`
    Value,
    /// `&self`
    Ref,
    /// `&!self`
    RefMut,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Param {
    pub id: NodeId,
    pub span: Span,
    pub kind: ParamKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParamKind {
    SelfParam(SelfKind),
    Named {
        mutable: bool,
        name: Ident,
        ty: Type,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Effects {
    pub id: NodeId,
    pub span: Span,
    pub kind: EffectsKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EffectsKind {
    /// Tập hiệu ứng rỗng (thuần).
    Pure,
    Set(Vec<Effect>),
}

/// `io`, `mem`, `st`, `div`, `ffi`, `lock(L)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Effect {
    pub id: NodeId,
    pub span: Span,
    pub name: Ident,
    pub arg: Option<Ident>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContractKind {
    Pre,
    Post,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Contract {
    pub id: NodeId,
    pub span: Span,
    pub kind: ContractKind,
    pub expr: Expr,
}

/// Chữ ký hàm (dùng chung cho hàm tự do, thành viên trait, thành viên impl).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FnSig {
    pub name: Ident,
    pub generics: Option<Generics>,
    pub params: Vec<Param>,
    pub ret: Option<Type>,
    pub effects: Effects,
    pub contracts: Vec<Contract>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FnItem {
    pub sig: FnSig,
    pub body: Block,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldDef {
    pub id: NodeId,
    pub span: Span,
    pub docs: Vec<DocComment>,
    pub vis: Visibility,
    pub name: Ident,
    pub ty: Type,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructItem {
    pub name: Ident,
    pub generics: Option<Generics>,
    pub fields: Vec<FieldDef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Variant {
    pub id: NodeId,
    pub span: Span,
    pub docs: Vec<DocComment>,
    pub name: Ident,
    /// Kiểu các trường của variant dạng tuple (rỗng = variant đơn).
    pub fields: Vec<Type>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnumItem {
    pub name: Ident,
    pub generics: Option<Generics>,
    pub variants: Vec<Variant>,
}

/// Chữ ký trong trait (không có thân).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraitMember {
    pub id: NodeId,
    pub span: Span,
    pub docs: Vec<DocComment>,
    pub sig: FnSig,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraitItem {
    pub name: Ident,
    pub generics: Option<Generics>,
    pub members: Vec<TraitMember>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImplMember {
    pub id: NodeId,
    pub span: Span,
    pub docs: Vec<DocComment>,
    pub func: FnItem,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImplItem {
    pub generics: Option<Generics>,
    /// `Some` cho `⊐ Trait for Type`, `None` cho impl thường.
    pub trait_path: Option<Path>,
    pub self_ty: Type,
    pub members: Vec<ImplMember>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModItem {
    pub name: Ident,
    pub items: Vec<Item>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UseItem {
    pub path: Path,
    /// `⇲ a::b::{x, y}` → `names = [x, y]`; rỗng nếu không có danh sách.
    pub names: Vec<Ident>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeAliasItem {
    pub name: Ident,
    pub generics: Option<Generics>,
    pub ty: Type,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConstItem {
    pub name: Ident,
    pub ty: Type,
    pub value: Expr,
}

/// Static BẤT BIẾN (Phase 1, D11).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StaticItem {
    pub name: Ident,
    pub ty: Type,
    pub value: Expr,
}

impl_node!(
    Ident,
    Lifetime,
    PathSegment,
    Path,
    Program,
    DocComment,
    AttrArg,
    Attribute,
    Item,
    Generics,
    GenericParam,
    Param,
    Effects,
    Effect,
    Contract,
    FieldDef,
    Variant,
    TraitMember,
    ImplMember,
);
