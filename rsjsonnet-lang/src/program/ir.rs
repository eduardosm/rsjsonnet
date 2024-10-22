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
        locals: Rc<FHashMap<InternedStr, Rc<Expr>>>,
        asserts: Vec<Assert>,
        fields: Vec<ObjectField>,
    },
    ObjectComp {
        is_top: bool,
        locals: Rc<FHashMap<InternedStr, Rc<Expr>>>,
        field_name: Rc<Expr>,
        field_name_span: SpanId,
        field_value: Rc<Expr>,
        comp_spec: Vec<CompSpecPart>,
    },
    FieldPlus {
        field_name: InternedStr,
        field_expr: Rc<Expr>,
    },
    Array(Vec<Rc<Expr>>),
    ArrayComp {
        value: Rc<Expr>,
        comp_spec: Vec<CompSpecPart>,
    },
    Field {
        object: Rc<Expr>,
        field_name: InternedStr,
        expr_span: SpanId,
    },
    Index {
        object: Rc<Expr>,
        index: Rc<Expr>,
        expr_span: SpanId,
    },
    SuperField {
        super_span: SpanId,
        field_name: InternedStr,
        expr_span: SpanId,
    },
    SuperIndex {
        super_span: SpanId,
        index: Rc<Expr>,
        expr_span: SpanId,
    },
    StdField {
        field_name: InternedStr,
    },
    Call {
        callee: Rc<Expr>,
        positional_args: Vec<Rc<Expr>>,
        named_args: Vec<(InternedStr, SpanId, Rc<Expr>)>,
        tailstrict: bool,
        span: SpanId,
    },
    Var(InternedStr, SpanId),
    SelfObj,
    TopObj,
    Local {
        bindings: FHashMap<InternedStr, Rc<Expr>>,
        inner: Rc<Expr>,
    },
    If {
        cond: Rc<Expr>,
        cond_span: SpanId,
        then_body: Rc<Expr>,
        else_body: Option<Rc<Expr>>,
    },
    Binary {
        op: ast::BinaryOp,
        lhs: Rc<Expr>,
        rhs: Rc<Expr>,
        span: SpanId,
    },
    Unary {
        op: ast::UnaryOp,
        rhs: Rc<Expr>,
        span: SpanId,
    },
    InSuper {
        lhs: Rc<Expr>,
        span: SpanId,
    },
    IdentityFunc,
    Func {
        params: Rc<FuncParams>,
        body: Rc<Expr>,
    },
    Error {
        msg: Rc<Expr>,
        span: SpanId,
    },
    Assert {
        assert: Assert,
        inner: Rc<Expr>,
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

#[derive(Debug)]
pub(super) struct Assert {
    pub(super) span: SpanId,
    pub(super) cond: Rc<Expr>,
    pub(super) cond_span: SpanId,
    pub(super) msg: Option<Rc<Expr>>,
}

#[derive(Clone, Debug)]
pub(super) struct ObjectField {
    pub(super) name: FieldName,
    pub(super) name_span: SpanId,
    pub(super) plus: bool,
    pub(super) visibility: ast::Visibility,
    pub(super) value: Rc<Expr>,
}

#[derive(Clone, Debug)]
pub(super) enum FieldName {
    Fix(InternedStr),
    Dyn(Rc<Expr>),
}

#[derive(Debug)]
pub(super) enum CompSpecPart {
    For {
        var: InternedStr,
        value: Rc<Expr>,
        value_span: SpanId,
    },
    If {
        cond: Rc<Expr>,
        cond_span: SpanId,
    },
}

#[derive(Debug)]
pub(super) struct FuncParams {
    pub(super) by_name: FHashMap<InternedStr, (usize, Option<Rc<Expr>>)>,
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

    pub(super) fn create_with_defaults<const N: usize>(
        str_interner: &StrInterner,
        params: [(&str, Option<Rc<Expr>>); N],
    ) -> Self {
        let order: Vec<_> = params.iter().map(|(s, _)| str_interner.intern(s)).collect();
        let by_name = params
            .into_iter()
            .enumerate()
            .map(|(i, (name, default))| (str_interner.intern(name), (i, default)))
            .collect();
        Self { by_name, order }
    }
}
