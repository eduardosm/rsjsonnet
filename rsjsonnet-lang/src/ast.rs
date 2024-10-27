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

impl Drop for Expr {
    fn drop(&mut self) {
        if matches!(self.kind, ExprKind::Null) {
            return;
        }

        // Avoid stack overflow when there is too much nesting
        let mut queue = Vec::new();
        queue.push(self.kind.drop_take());
        while let Some(mut cur) = queue.pop() {
            cur.drop_take_exprs(&mut queue);
        }
    }
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

impl ExprKind {
    #[inline]
    fn drop_take(&mut self) -> Self {
        std::mem::replace(self, Self::Null)
    }

    fn drop_take_exprs(&mut self, out: &mut Vec<Self>) {
        match self.drop_take() {
            Self::Null => {}
            Self::Bool(..) => {}
            Self::SelfObj => {}
            Self::Dollar => {}
            Self::String(..) => {}
            Self::TextBlock(..) => {}
            Self::Number(..) => {}
            Self::Paren(mut inner) => {
                out.push(inner.kind.drop_take());
            }
            Self::Object(mut obj) => {
                obj.drop_take_exprs(out);
            }
            Self::Array(mut items) => {
                for item in items.iter_mut() {
                    out.push(item.kind.drop_take());
                }
            }
            Self::ArrayComp(mut inner, mut comp_spec) => {
                out.push(inner.kind.drop_take());
                for spec in comp_spec.iter_mut() {
                    spec.drop_take_exprs(out);
                }
            }
            Self::Field(mut obj, _) => {
                out.push(obj.kind.drop_take());
            }
            Self::Index(mut obj, mut index) => {
                out.push(obj.kind.drop_take());
                out.push(index.kind.drop_take());
            }
            Self::Slice(mut obj, start, end, step) => {
                out.push(obj.kind.drop_take());
                if let Some(mut start) = start {
                    out.push(start.kind.drop_take());
                }
                if let Some(mut end) = end {
                    out.push(end.kind.drop_take());
                }
                if let Some(mut step) = step {
                    out.push(step.kind.drop_take());
                }
            }
            Self::SuperField(_, _) => {}
            Self::SuperIndex(_, mut index) => {
                out.push(index.kind.drop_take());
            }
            Self::Call(mut func, mut args, _) => {
                out.push(func.kind.drop_take());
                for arg in args.iter_mut() {
                    match arg {
                        Arg::Positional(expr) => {
                            out.push(expr.kind.drop_take());
                        }
                        Arg::Named(_, expr) => {
                            out.push(expr.kind.drop_take());
                        }
                    }
                }
            }
            Self::Ident(_) => {}
            Self::Local(mut binds, mut body) => {
                for bind in binds.iter_mut() {
                    if let Some((ref mut params, _)) = bind.params {
                        for param in params.iter_mut() {
                            if let Some(ref mut default_value) = param.default_value {
                                out.push(default_value.kind.drop_take());
                            }
                        }
                    }
                    out.push(bind.value.kind.drop_take());
                }
                out.push(body.kind.drop_take());
            }
            Self::If(mut cond, mut then_body, else_body) => {
                out.push(cond.kind.drop_take());
                out.push(then_body.kind.drop_take());
                if let Some(mut else_body) = else_body {
                    out.push(else_body.kind.drop_take());
                }
            }
            Self::Binary(mut lhs, _, mut rhs) => {
                out.push(lhs.kind.drop_take());
                out.push(rhs.kind.drop_take());
            }
            Self::Unary(_, mut rhs) => {
                out.push(rhs.kind.drop_take());
            }
            Self::ObjExt(mut base, mut obj, _) => {
                out.push(base.kind.drop_take());
                obj.drop_take_exprs(out);
            }
            Self::Func(mut params, mut body) => {
                for param in params.iter_mut() {
                    if let Some(ref mut default_value) = param.default_value {
                        out.push(default_value.kind.drop_take());
                    }
                }
                out.push(body.kind.drop_take());
            }
            Self::Assert(mut assert, mut body) => {
                out.push(assert.cond.kind.drop_take());
                if let Some(ref mut msg) = assert.msg {
                    out.push(msg.kind.drop_take());
                }
                out.push(body.kind.drop_take());
            }
            Self::Import(mut expr) => {
                out.push(expr.kind.drop_take());
            }
            Self::ImportStr(mut expr) => {
                out.push(expr.kind.drop_take());
            }
            Self::ImportBin(mut expr) => {
                out.push(expr.kind.drop_take());
            }
            Self::Error(mut expr) => {
                out.push(expr.kind.drop_take());
            }
            Self::InSuper(mut expr, _) => {
                out.push(expr.kind.drop_take());
            }
        }
    }
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

impl ObjInside {
    fn drop_take_exprs(&mut self, out: &mut Vec<ExprKind>) {
        match self {
            Self::Members(members) => {
                for member in members.iter_mut() {
                    match member {
                        Member::Local(local) => {
                            out.push(local.bind.value.kind.drop_take());
                        }
                        Member::Assert(assert) => {
                            out.push(assert.cond.kind.drop_take());
                            if let Some(ref mut msg) = assert.msg {
                                out.push(msg.kind.drop_take());
                            }
                        }
                        Member::Field(field) => match field {
                            Field::Value(_, _, _, expr) => {
                                out.push(expr.kind.drop_take());
                            }
                            Field::Func(_, params, _, _, expr) => {
                                for param in params.iter_mut() {
                                    if let Some(ref mut default_value) = param.default_value {
                                        out.push(default_value.kind.drop_take());
                                    }
                                }
                                out.push(expr.kind.drop_take());
                            }
                        },
                    }
                }
            }
            Self::Comp {
                locals1,
                name,
                body,
                locals2,
                comp_spec,
            } => {
                for local in locals1.iter_mut() {
                    out.push(local.bind.value.kind.drop_take());
                }
                out.push(name.kind.drop_take());
                out.push(body.kind.drop_take());
                for local in locals2.iter_mut() {
                    out.push(local.bind.value.kind.drop_take());
                }
                for spec in comp_spec.iter_mut() {
                    spec.drop_take_exprs(out);
                }
            }
        }
    }
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

impl CompSpecPart {
    fn drop_take_exprs(&mut self, out: &mut Vec<ExprKind>) {
        match self {
            Self::For(for_spec) => {
                out.push(for_spec.inner.kind.drop_take());
            }
            Self::If(if_spec) => {
                out.push(if_spec.cond.kind.drop_take());
            }
        }
    }
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
