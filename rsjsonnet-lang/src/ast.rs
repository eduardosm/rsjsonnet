use crate::interner::InternedStr;
use crate::span::SpanId;
use crate::token::Number;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ident {
    pub value: InternedStr,
    pub span: SpanId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: SpanId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExprKind {
    Null,
    Bool(bool),
    SelfObj,
    Dollar,
    String(Box<str>),
    TextBlock(Box<str>),
    Number(Number),
    Paren(Box<Expr>),
    Object(ObjInside),
    Array(Box<[Expr]>),
    ArrayComp(Box<Expr>, Box<[CompSpecPart]>),
    Field(Box<Expr>, Ident),
    Index(Box<Expr>, Box<Expr>),
    Slice(
        Box<Expr>,
        Option<Box<Expr>>,
        Option<Box<Expr>>,
        Option<Box<Expr>>,
    ),
    SuperField(SpanId, Ident),
    SuperIndex(SpanId, Box<Expr>),
    Call(Box<Expr>, Box<[Arg]>, bool),
    Ident(Ident),
    Local(Box<[Bind]>, Box<Expr>),
    If(Box<Expr>, Box<Expr>, Option<Box<Expr>>),
    Binary(Box<Expr>, BinaryOp, Box<Expr>),
    Unary(UnaryOp, Box<Expr>),
    ObjExt(Box<Expr>, ObjInside, SpanId),
    Func(Box<[Param]>, Box<Expr>),
    Assert(Box<Assert>, Box<Expr>),
    Import(Box<Expr>),
    ImportStr(Box<Expr>),
    ImportBin(Box<Expr>),
    Error(Box<Expr>),
    InSuper(Box<Expr>, SpanId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObjInside {
    Members(Box<[Member]>),
    Comp {
        locals1: Box<[ObjLocal]>,
        name: Box<Expr>,
        body: Box<Expr>,
        locals2: Box<[ObjLocal]>,
        comp_spec: Box<[CompSpecPart]>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Member {
    Local(ObjLocal),
    Assert(Assert),
    Field(Field),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Field {
    Value(FieldName, bool, Visibility, Expr),
    Func(FieldName, Box<[Param]>, SpanId, Visibility, Expr),
}

impl Field {
    #[inline]
    pub fn name(&self) -> &FieldName {
        match self {
            Self::Value(name, ..) => name,
            Self::Func(name, ..) => name,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Visibility {
    Default,
    Hidden,
    ForceVisible,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjLocal {
    pub bind: Bind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompSpecPart {
    For(ForSpec),
    If(IfSpec),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForSpec {
    pub var: Ident,
    pub inner: Expr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IfSpec {
    pub cond: Expr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldName {
    Ident(Ident),
    String(InternedStr, SpanId),
    Expr(Expr, SpanId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Assert {
    pub span: SpanId,
    pub cond: Expr,
    pub msg: Option<Expr>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bind {
    pub name: Ident,
    pub params: Option<(Box<[Param]>, SpanId)>,
    pub value: Expr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Arg {
    Positional(Expr),
    Named(Ident, Expr),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Param {
    pub name: Ident,
    pub default_value: Option<Expr>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Shl,
    Shr,
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
    In,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    LogicAnd,
    LogicOr,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum UnaryOp {
    Minus,
    Plus,
    BitwiseNot,
    LogicNot,
}
