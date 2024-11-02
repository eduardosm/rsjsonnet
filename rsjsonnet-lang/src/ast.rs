//! The abstract syntax tree (AST) of Jsonnet code.
//!
//! Since Jsonnet is a purely functional language, it is composed mainly of
//! expressions. See [`Expr`] and [`ExprKind`] as starting point.
//!
//! # Lifetimes
//!
//! AST structures in this module are intended to be allocated with arena
//! allocation. Two lifetime are used:
//!
//! * `'p`, which represents interned strings.
//! * `'ast`, which represents most allocations of AST nodes.
//!
//! Independent arenas can be used for each lifetime, allowing to free most of
//! an AST while keeping interned strings.

use crate::interner::InternedStr;
use crate::span::SpanId;
use crate::token::Number;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Ident<'p> {
    pub value: InternedStr<'p>,
    pub span: SpanId,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Expr<'p, 'ast> {
    pub kind: ExprKind<'p, 'ast>,
    pub span: SpanId,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ExprKind<'p, 'ast> {
    Null,
    Bool(bool),
    SelfObj,
    Dollar,
    String(&'ast str),
    TextBlock(&'ast str),
    Number(Number<'ast>),
    Paren(&'ast Expr<'p, 'ast>),
    Object(ObjInside<'p, 'ast>),
    Array(&'ast [Expr<'p, 'ast>]),
    ArrayComp(&'ast Expr<'p, 'ast>, &'ast [CompSpecPart<'p, 'ast>]),
    Field(&'ast Expr<'p, 'ast>, Ident<'p>),
    Index(&'ast Expr<'p, 'ast>, &'ast Expr<'p, 'ast>),
    Slice(
        &'ast Expr<'p, 'ast>,
        Option<&'ast Expr<'p, 'ast>>,
        Option<&'ast Expr<'p, 'ast>>,
        Option<&'ast Expr<'p, 'ast>>,
    ),
    SuperField(SpanId, Ident<'p>),
    SuperIndex(SpanId, &'ast Expr<'p, 'ast>),
    Call(&'ast Expr<'p, 'ast>, &'ast [Arg<'p, 'ast>], bool),
    Ident(Ident<'p>),
    Local(&'ast [Bind<'p, 'ast>], &'ast Expr<'p, 'ast>),
    If(
        &'ast Expr<'p, 'ast>,
        &'ast Expr<'p, 'ast>,
        Option<&'ast Expr<'p, 'ast>>,
    ),
    Binary(&'ast Expr<'p, 'ast>, BinaryOp, &'ast Expr<'p, 'ast>),
    Unary(UnaryOp, &'ast Expr<'p, 'ast>),
    ObjExt(&'ast Expr<'p, 'ast>, ObjInside<'p, 'ast>, SpanId),
    Func(&'ast [Param<'p, 'ast>], &'ast Expr<'p, 'ast>),
    Assert(&'ast Assert<'p, 'ast>, &'ast Expr<'p, 'ast>),
    Import(&'ast Expr<'p, 'ast>),
    ImportStr(&'ast Expr<'p, 'ast>),
    ImportBin(&'ast Expr<'p, 'ast>),
    Error(&'ast Expr<'p, 'ast>),
    InSuper(&'ast Expr<'p, 'ast>, SpanId),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ObjInside<'p, 'ast> {
    Members(&'ast [Member<'p, 'ast>]),
    Comp {
        locals1: &'ast [ObjLocal<'p, 'ast>],
        name: &'ast Expr<'p, 'ast>,
        body: &'ast Expr<'p, 'ast>,
        locals2: &'ast [ObjLocal<'p, 'ast>],
        comp_spec: &'ast [CompSpecPart<'p, 'ast>],
    },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Member<'p, 'ast> {
    Local(ObjLocal<'p, 'ast>),
    Assert(Assert<'p, 'ast>),
    Field(Field<'p, 'ast>),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Field<'p, 'ast> {
    Value(FieldName<'p, 'ast>, bool, Visibility, Expr<'p, 'ast>),
    Func(
        FieldName<'p, 'ast>,
        &'ast [Param<'p, 'ast>],
        SpanId,
        Visibility,
        Expr<'p, 'ast>,
    ),
}

impl<'p, 'ast> Field<'p, 'ast> {
    #[inline]
    pub fn name(&self) -> &FieldName<'p, 'ast> {
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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ObjLocal<'p, 'ast> {
    pub bind: Bind<'p, 'ast>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CompSpecPart<'p, 'ast> {
    For(ForSpec<'p, 'ast>),
    If(IfSpec<'p, 'ast>),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ForSpec<'p, 'ast> {
    pub var: Ident<'p>,
    pub inner: Expr<'p, 'ast>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct IfSpec<'p, 'ast> {
    pub cond: Expr<'p, 'ast>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FieldName<'p, 'ast> {
    Ident(Ident<'p>),
    String(InternedStr<'p>, SpanId),
    Expr(Expr<'p, 'ast>, SpanId),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Assert<'p, 'ast> {
    pub span: SpanId,
    pub cond: Expr<'p, 'ast>,
    pub msg: Option<Expr<'p, 'ast>>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Bind<'p, 'ast> {
    pub name: Ident<'p>,
    pub params: Option<(&'ast [Param<'p, 'ast>], SpanId)>,
    pub value: Expr<'p, 'ast>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Arg<'p, 'ast> {
    Positional(Expr<'p, 'ast>),
    Named(Ident<'p>, Expr<'p, 'ast>),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Param<'p, 'ast> {
    pub name: Ident<'p>,
    pub default_value: Option<Expr<'p, 'ast>>,
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
