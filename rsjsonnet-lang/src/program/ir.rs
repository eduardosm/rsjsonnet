use crate::ast;
use crate::interner::InternedStr;
use crate::span::SpanId;

#[derive(Copy, Clone, Debug)]
pub(super) enum Expr<'p> {
    Null,
    Bool(bool),
    Number(f64, SpanId),
    String(&'p str),
    Object {
        is_top: bool,
        locals: &'p [(InternedStr<'p>, &'p Expr<'p>)],
        asserts: &'p [Assert<'p>],
        fields: &'p [ObjectField<'p>],
    },
    ObjectComp {
        is_top: bool,
        locals: &'p [(InternedStr<'p>, &'p Expr<'p>)],
        field_name: &'p Expr<'p>,
        field_name_span: SpanId,
        field_value: &'p Expr<'p>,
        comp_spec: &'p [CompSpecPart<'p>],
    },
    Array(&'p [&'p Expr<'p>]),
    ArrayComp {
        value: &'p Expr<'p>,
        comp_spec: &'p [CompSpecPart<'p>],
    },
    Field {
        object: &'p Expr<'p>,
        field_name: InternedStr<'p>,
        expr_span: SpanId,
    },
    Index {
        object: &'p Expr<'p>,
        index: &'p Expr<'p>,
        expr_span: SpanId,
    },
    Slice {
        array: &'p Expr<'p>,
        start_index: Option<&'p Expr<'p>>,
        end_index: Option<&'p Expr<'p>>,
        step: Option<&'p Expr<'p>>,
        expr_span: SpanId,
    },
    SuperField {
        super_span: SpanId,
        field_name: InternedStr<'p>,
        expr_span: SpanId,
    },
    SuperIndex {
        super_span: SpanId,
        index: &'p Expr<'p>,
        expr_span: SpanId,
    },
    Call {
        callee: &'p Expr<'p>,
        positional_args: &'p [&'p Expr<'p>],
        named_args: &'p [(InternedStr<'p>, SpanId, &'p Expr<'p>)],
        tailstrict: bool,
        span: SpanId,
    },
    Var(InternedStr<'p>, SpanId),
    SelfObj,
    TopObj,
    Local {
        bindings: &'p [(InternedStr<'p>, &'p Expr<'p>)],
        inner: &'p Expr<'p>,
    },
    If {
        cond: &'p Expr<'p>,
        cond_span: SpanId,
        then_body: &'p Expr<'p>,
        else_body: Option<&'p Expr<'p>>,
    },
    Binary {
        op: ast::BinaryOp,
        lhs: &'p Expr<'p>,
        rhs: &'p Expr<'p>,
        span: SpanId,
    },
    Unary {
        op: ast::UnaryOp,
        rhs: &'p Expr<'p>,
        span: SpanId,
    },
    InSuper {
        lhs: &'p Expr<'p>,
        span: SpanId,
    },
    IdentityFunc,
    Func {
        params: &'p [(InternedStr<'p>, Option<&'p Expr<'p>>)],
        body: &'p Expr<'p>,
    },
    Error {
        msg: &'p Expr<'p>,
        span: SpanId,
    },
    Assert {
        assert: Assert<'p>,
        inner: &'p Expr<'p>,
    },
    Import {
        path: &'p str,
        span: SpanId,
    },
    ImportStr {
        path: &'p str,
        span: SpanId,
    },
    ImportBin {
        path: &'p str,
        span: SpanId,
    },
}

#[derive(Copy, Clone, Debug)]
pub(super) struct Assert<'p> {
    pub(super) span: SpanId,
    pub(super) cond: &'p Expr<'p>,
    pub(super) cond_span: SpanId,
    pub(super) msg: Option<&'p Expr<'p>>,
}

#[derive(Copy, Clone, Debug)]
pub(super) struct ObjectField<'p> {
    pub(super) name: FieldName<'p>,
    pub(super) name_span: SpanId,
    pub(super) plus: bool,
    pub(super) visibility: ast::Visibility,
    pub(super) value: &'p Expr<'p>,
}

#[derive(Copy, Clone, Debug)]
pub(super) enum FieldName<'p> {
    Fix(InternedStr<'p>),
    Dyn(&'p Expr<'p>),
}

#[derive(Copy, Clone, Debug)]
pub(super) enum CompSpecPart<'p> {
    For {
        var: InternedStr<'p>,
        value: &'p Expr<'p>,
        value_span: SpanId,
    },
    If {
        cond: &'p Expr<'p>,
        cond_span: SpanId,
    },
}
