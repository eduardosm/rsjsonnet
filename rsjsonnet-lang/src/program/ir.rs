use std::rc::Rc;

use crate::interner::{InternedStr, StrInterner};
use crate::span::SpanId;
use crate::{ast, FHashMap};

#[derive(Debug)]
pub(super) enum Expr {
    Null,
    Bool(bool),
    Number(f64, SpanId),
    String(Rc<str>),
    Object {
        is_top: bool,
        locals: Rc<Vec<(InternedStr, RcExpr)>>,
        asserts: Vec<Assert>,
        fields: Vec<ObjectField>,
    },
    ObjectComp {
        is_top: bool,
        locals: Rc<Vec<(InternedStr, RcExpr)>>,
        field_name: RcExpr,
        field_name_span: SpanId,
        field_value: RcExpr,
        comp_spec: Vec<CompSpecPart>,
    },
    Array(Vec<RcExpr>),
    ArrayComp {
        value: RcExpr,
        comp_spec: Vec<CompSpecPart>,
    },
    Field {
        object: RcExpr,
        field_name: InternedStr,
        expr_span: SpanId,
    },
    Index {
        object: RcExpr,
        index: RcExpr,
        expr_span: SpanId,
    },
    SuperField {
        super_span: SpanId,
        field_name: InternedStr,
        expr_span: SpanId,
    },
    SuperIndex {
        super_span: SpanId,
        index: RcExpr,
        expr_span: SpanId,
    },
    StdField {
        field_name: InternedStr,
    },
    Call {
        callee: RcExpr,
        positional_args: Vec<RcExpr>,
        named_args: Vec<(InternedStr, SpanId, RcExpr)>,
        tailstrict: bool,
        span: SpanId,
    },
    Var(InternedStr, SpanId),
    SelfObj,
    TopObj,
    Local {
        bindings: Vec<(InternedStr, RcExpr)>,
        inner: RcExpr,
    },
    If {
        cond: RcExpr,
        cond_span: SpanId,
        then_body: RcExpr,
        else_body: Option<RcExpr>,
    },
    Binary {
        op: ast::BinaryOp,
        lhs: RcExpr,
        rhs: RcExpr,
        span: SpanId,
    },
    Unary {
        op: ast::UnaryOp,
        rhs: RcExpr,
        span: SpanId,
    },
    InSuper {
        lhs: RcExpr,
        span: SpanId,
    },
    IdentityFunc,
    Func {
        params: Rc<FuncParams>,
        body: RcExpr,
    },
    Error {
        msg: RcExpr,
        span: SpanId,
    },
    Assert {
        assert: Assert,
        inner: RcExpr,
    },
    Import {
        path: String,
        span: SpanId,
    },
    ImportStr {
        path: String,
        span: SpanId,
    },
    ImportBin {
        path: String,
        span: SpanId,
    },
}

impl Expr {
    #[inline]
    fn drop_take(&mut self) -> Self {
        std::mem::replace(self, Self::Null)
    }

    fn drop_take_exprs(&mut self, out: &mut Vec<RcExpr>) {
        match self.drop_take() {
            Self::Null => {}
            Self::Bool(_) => {}
            Self::Number(_, _) => {}
            Self::String(_) => {}
            Self::Object {
                is_top: _,
                locals,
                asserts,
                fields,
            } => {
                if let Some(locals) = Rc::into_inner(locals) {
                    for (_, v) in locals {
                        out.push(v);
                    }
                }
                for assert in asserts {
                    out.push(assert.cond.clone());
                    if let Some(msg) = assert.msg {
                        out.push(msg);
                    }
                }
                for field in fields {
                    out.push(field.value.clone());
                }
            }
            Self::ObjectComp {
                is_top: _,
                locals,
                field_name,
                field_name_span: _,
                field_value: _,
                comp_spec,
            } => {
                if let Some(locals) = Rc::into_inner(locals) {
                    for (_, v) in locals {
                        out.push(v);
                    }
                }
                out.push(field_name);
                for part in comp_spec {
                    match part {
                        CompSpecPart::For {
                            var: _,
                            value,
                            value_span: _,
                        } => {
                            out.push(value);
                        }
                        CompSpecPart::If { cond, cond_span: _ } => {
                            out.push(cond);
                        }
                    }
                }
            }
            Self::Array(items) => {
                for item in items {
                    out.push(item);
                }
            }
            Self::ArrayComp { value, comp_spec } => {
                out.push(value);
                for part in comp_spec {
                    match part {
                        CompSpecPart::For {
                            var: _,
                            value,
                            value_span: _,
                        } => {
                            out.push(value);
                        }
                        CompSpecPart::If { cond, cond_span: _ } => {
                            out.push(cond);
                        }
                    }
                }
            }
            Self::Field {
                object,
                field_name: _,
                expr_span: _,
            } => {
                out.push(object);
            }
            Self::Index {
                object,
                index,
                expr_span: _,
            } => {
                out.push(object);
                out.push(index);
            }
            Self::SuperField {
                super_span: _,
                field_name: _,
                expr_span: _,
            } => {}
            Self::SuperIndex {
                super_span: _,
                index,
                expr_span: _,
            } => {
                out.push(index);
            }
            Self::StdField { field_name: _ } => {}
            Self::Call {
                callee,
                positional_args,
                named_args,
                tailstrict: _,
                span: _,
            } => {
                out.push(callee);
                for arg in positional_args {
                    out.push(arg);
                }
                for (_, _, arg) in named_args {
                    out.push(arg);
                }
            }
            Self::Var(_, _) => {}
            Self::SelfObj => {}
            Self::TopObj => {}
            Self::Local { bindings, inner } => {
                for (_, v) in bindings {
                    out.push(v);
                }
                out.push(inner);
            }
            Self::If {
                cond,
                cond_span: _,
                then_body,
                else_body,
            } => {
                out.push(cond);
                out.push(then_body);
                if let Some(else_body) = else_body {
                    out.push(else_body);
                }
            }
            Self::Binary {
                op: _,
                lhs,
                rhs,
                span: _,
            } => {
                out.push(lhs);
                out.push(rhs);
            }
            Self::Unary {
                op: _,
                rhs,
                span: _,
            } => {
                out.push(rhs);
            }
            Self::InSuper { lhs, span: _ } => {
                out.push(lhs);
            }
            Self::IdentityFunc => {}
            Self::Func { params: _, body } => {
                out.push(body);
            }
            Self::Error { msg, span: _ } => {
                out.push(msg);
            }
            Self::Assert { assert, inner } => {
                out.push(assert.cond.clone());
                if let Some(msg) = assert.msg {
                    out.push(msg);
                }
                out.push(inner);
            }
            Self::Import { path: _, span: _ } => {}
            Self::ImportStr { path: _, span: _ } => {}
            Self::ImportBin { path: _, span: _ } => {}
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct RcExpr {
    inner: Rc<Expr>,
}

impl Drop for RcExpr {
    fn drop(&mut self) {
        if matches!(*self.inner, Expr::Null) {
            return;
        }

        if let Some(inner) = Rc::get_mut(&mut self.inner) {
            // Avoid stack overflow when there is too much nesting
            let mut queue = Vec::new();
            inner.drop_take_exprs(&mut queue);
            while let Some(mut cur) = queue.pop() {
                if let Some(cur) = Rc::get_mut(&mut cur.inner) {
                    cur.drop_take_exprs(&mut queue);
                }
            }
        }
    }
}

impl RcExpr {
    #[inline]
    pub(super) fn new(inner: Expr) -> Self {
        Self {
            inner: Rc::new(inner),
        }
    }
}

impl std::ops::Deref for RcExpr {
    type Target = Expr;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug)]
pub(super) struct Assert {
    pub(super) span: SpanId,
    pub(super) cond: RcExpr,
    pub(super) cond_span: SpanId,
    pub(super) msg: Option<RcExpr>,
}

#[derive(Clone, Debug)]
pub(super) struct ObjectField {
    pub(super) name: FieldName,
    pub(super) name_span: SpanId,
    pub(super) plus: bool,
    pub(super) visibility: ast::Visibility,
    pub(super) value: RcExpr,
}

#[derive(Clone, Debug)]
pub(super) enum FieldName {
    Fix(InternedStr),
    Dyn(RcExpr),
}

#[derive(Debug)]
pub(super) enum CompSpecPart {
    For {
        var: InternedStr,
        value: RcExpr,
        value_span: SpanId,
    },
    If {
        cond: RcExpr,
        cond_span: SpanId,
    },
}

#[derive(Debug)]
pub(super) struct FuncParams {
    pub(super) by_name: FHashMap<InternedStr, (usize, Option<RcExpr>)>,
    pub(super) order: Vec<InternedStr>,
}

impl FuncParams {
    pub(super) fn create_simple(str_interner: &StrInterner, params: &[&str]) -> Self {
        let order: Vec<_> = params.iter().map(|s| str_interner.intern(s)).collect();
        let by_name = order
            .iter()
            .enumerate()
            .map(|(i, name)| (name.clone(), (i, None)))
            .collect();
        Self { by_name, order }
    }

    pub(super) fn create_with_defaults(
        str_interner: &StrInterner,
        params: &[(&str, Option<RcExpr>)],
    ) -> Self {
        let order: Vec<_> = params.iter().map(|(s, _)| str_interner.intern(s)).collect();
        let by_name = params
            .iter()
            .enumerate()
            .map(|(i, (name, default))| (str_interner.intern(name), (i, default.clone())))
            .collect();
        Self { by_name, order }
    }
}
