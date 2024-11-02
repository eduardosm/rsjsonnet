use std::collections::hash_map::Entry as HashMapEntry;

use super::{ir, AnalyzeError, Program};
use crate::interner::InternedStr;
use crate::span::SpanId;
use crate::{ast, FHashMap, FHashSet};

pub(super) struct Analyzer<'a, 'p> {
    program: &'a Program<'p>,
}

impl<'a, 'p> Analyzer<'a, 'p> {
    pub(super) fn new(program: &'a Program<'p>) -> Self {
        Self { program }
    }

    pub(super) fn analyze(
        mut self,
        ast: &ast::Expr<'p, '_>,
        env: FHashSet<InternedStr<'p>>,
    ) -> Result<&'p ir::Expr<'p>, AnalyzeError> {
        let env = Env {
            is_obj: false,
            vars: env,
        };
        self.analyze_expr(ast, &env, false)
    }

    fn analyze_expr<'ast>(
        &mut self,
        root_expr_ast: &'ast ast::Expr<'p, 'ast>,
        env: &Env<'p>,
        root_can_be_tailstrict: bool,
    ) -> Result<&'p ir::Expr<'p>, AnalyzeError> {
        enum State<'p, 'ast> {
            Analyzed(&'p ir::Expr<'p>),
            Expr(&'ast ast::Expr<'p, 'ast>, bool),
        }

        enum StackItem<'p, 'ast> {
            ArrayItem(SpanId, Vec<&'p ir::Expr<'p>>, &'ast [ast::Expr<'p, 'ast>]),
            BinaryLhs(SpanId, ast::BinaryOp, &'ast ast::Expr<'p, 'ast>),
            BinaryRhs(SpanId, ast::BinaryOp, &'p ir::Expr<'p>),
            UnaryRhs(SpanId, ast::UnaryOp),
        }

        let mut stack = Vec::new();
        let mut state = State::Expr(root_expr_ast, root_can_be_tailstrict);

        loop {
            match state {
                State::Analyzed(expr_ir) => match stack.pop() {
                    None => return Ok(expr_ir),
                    Some(StackItem::ArrayItem(span, mut items_ir, rem_items_ast)) => {
                        items_ir.push(expr_ir);

                        if let Some((next_item_ast, rem_items_ast)) = rem_items_ast.split_first() {
                            stack.push(StackItem::ArrayItem(span, items_ir, rem_items_ast));
                            state = State::Expr(next_item_ast, false);
                        } else {
                            state =
                                State::Analyzed(self.program.arena.alloc(ir::Expr::Array(
                                    self.program.arena.alloc_slice(&items_ir),
                                )));
                        }
                    }
                    Some(StackItem::BinaryLhs(span, op, rhs_ast)) => {
                        stack.push(StackItem::BinaryRhs(span, op, expr_ir));
                        state = State::Expr(rhs_ast, false);
                    }
                    Some(StackItem::BinaryRhs(span, op, lhs)) => {
                        let rhs = expr_ir;
                        state = State::Analyzed(self.program.arena.alloc(ir::Expr::Binary {
                            op,
                            lhs,
                            rhs,
                            span,
                        }));
                    }
                    Some(StackItem::UnaryRhs(span, op)) => {
                        let rhs = expr_ir;
                        state = State::Analyzed(self.program.arena.alloc(ir::Expr::Unary {
                            op,
                            rhs,
                            span,
                        }));
                    }
                },
                State::Expr(expr_ast, can_be_tailstrict) => match expr_ast.kind {
                    ast::ExprKind::Null => {
                        state = State::Analyzed(self.program.exprs.null);
                    }
                    ast::ExprKind::Bool(value) => {
                        state = State::Analyzed(if value {
                            self.program.exprs.true_
                        } else {
                            self.program.exprs.false_
                        });
                    }
                    ast::ExprKind::SelfObj => {
                        if env.is_obj {
                            state = State::Analyzed(self.program.exprs.self_obj);
                        } else {
                            return Err(AnalyzeError::SelfOutsideObject {
                                self_span: expr_ast.span,
                            });
                        }
                    }
                    ast::ExprKind::Dollar => {
                        if env.is_obj {
                            state = State::Analyzed(self.program.exprs.top_obj)
                        } else {
                            return Err(AnalyzeError::DollarOutsideObject {
                                dollar_span: expr_ast.span,
                            });
                        }
                    }
                    ast::ExprKind::String(s) => {
                        state = State::Analyzed(
                            self.program
                                .arena
                                .alloc(ir::Expr::String(self.program.arena.alloc_str(s))),
                        );
                    }
                    ast::ExprKind::TextBlock(s) => {
                        state = State::Analyzed(
                            self.program
                                .arena
                                .alloc(ir::Expr::String(self.program.arena.alloc_str(s))),
                        );
                    }
                    ast::ExprKind::Number(ref value) => {
                        let float_value =
                            format!("{}e{}", value.digits, value.exp).parse().unwrap();
                        state = State::Analyzed(
                            self.program
                                .arena
                                .alloc(ir::Expr::Number(float_value, expr_ast.span)),
                        );
                    }
                    ast::ExprKind::Paren(inner) => {
                        state = State::Expr(inner, false);
                    }
                    ast::ExprKind::Object(ref inside) => {
                        state = State::Analyzed(self.analyze_objinside(inside, env)?);
                    }
                    ast::ExprKind::Array(items_ast) => {
                        if let Some((first_item_ast, rem_items_ast)) = items_ast.split_first() {
                            stack.push(StackItem::ArrayItem(
                                expr_ast.span,
                                Vec::new(),
                                rem_items_ast,
                            ));
                            state = State::Expr(first_item_ast, false);
                        } else {
                            state = State::Analyzed(self.program.arena.alloc(ir::Expr::Array(&[])));
                        }
                    }
                    ast::ExprKind::ArrayComp(body_ast, comp_spec_ast) => {
                        let (comp_spec, env) = self.analyze_comp_spec(comp_spec_ast, env)?;
                        let body = self.analyze_expr(body_ast, &env, false)?;
                        state = State::Analyzed(self.program.arena.alloc(ir::Expr::ArrayComp {
                            value: body,
                            comp_spec,
                        }));
                    }
                    ast::ExprKind::Field(obj_ast, field_name_ast) => {
                        let object = self.analyze_expr(obj_ast, env, false)?;
                        state = State::Analyzed(self.program.arena.alloc(ir::Expr::Field {
                            object,
                            field_name: field_name_ast.value,
                            expr_span: expr_ast.span,
                        }));
                    }
                    ast::ExprKind::Index(obj_ast, index_ast) => {
                        let object = self.analyze_expr(obj_ast, env, false)?;
                        let index = self.analyze_expr(index_ast, env, false)?;
                        state = State::Analyzed(self.program.arena.alloc(ir::Expr::Index {
                            object,
                            index,
                            expr_span: expr_ast.span,
                        }));
                    }
                    ast::ExprKind::Slice(array_ast, start_index_ast, end_index_ast, step_ast) => {
                        let array = self.analyze_expr(array_ast, env, false)?;
                        let start_index = start_index_ast
                            .map(|e| self.analyze_expr(e, env, false))
                            .transpose()?;
                        let end_index = end_index_ast
                            .map(|e| self.analyze_expr(e, env, false))
                            .transpose()?;
                        let step = step_ast
                            .map(|e| self.analyze_expr(e, env, false))
                            .transpose()?;

                        state = State::Analyzed(self.program.arena.alloc(ir::Expr::Slice {
                            array,
                            start_index,
                            end_index,
                            step,
                            expr_span: expr_ast.span,
                        }));
                    }
                    ast::ExprKind::SuperField(super_span, ref field_name_ast) => {
                        if env.is_obj {
                            state =
                                State::Analyzed(self.program.arena.alloc(ir::Expr::SuperField {
                                    super_span,
                                    field_name: field_name_ast.value,
                                    expr_span: expr_ast.span,
                                }));
                        } else {
                            return Err(AnalyzeError::SuperOutsideObject { super_span });
                        }
                    }
                    ast::ExprKind::SuperIndex(super_span, index_ast) => {
                        if env.is_obj {
                            let index = self.analyze_expr(index_ast, env, false)?;
                            state =
                                State::Analyzed(self.program.arena.alloc(ir::Expr::SuperIndex {
                                    super_span,
                                    index,
                                    expr_span: expr_ast.span,
                                }));
                        } else {
                            return Err(AnalyzeError::SuperOutsideObject { super_span });
                        }
                    }
                    ast::ExprKind::Call(callee_ast, args_ast, tailstrict) => {
                        let callee = self.analyze_expr(callee_ast, env, false)?;

                        let mut positional_args = Vec::new();
                        let mut named_args = Vec::new();
                        for arg_ast in args_ast.iter() {
                            match arg_ast {
                                ast::Arg::Positional(value_ast) => {
                                    if !named_args.is_empty() {
                                        return Err(AnalyzeError::PositionalArgAfterNamed {
                                            arg_span: value_ast.span,
                                        });
                                    }
                                    positional_args.push(self.analyze_expr(value_ast, env, false)?);
                                }
                                ast::Arg::Named(arg_name, value_ast) => {
                                    named_args.push((
                                        arg_name.value,
                                        arg_name.span,
                                        self.analyze_expr(value_ast, env, false)?,
                                    ));
                                }
                            }
                        }

                        state = State::Analyzed(self.program.arena.alloc(ir::Expr::Call {
                            callee,
                            positional_args: self.program.arena.alloc_slice(&positional_args),
                            named_args: self.program.arena.alloc_slice(&named_args),
                            tailstrict: can_be_tailstrict && tailstrict,
                            span: expr_ast.span,
                        }));
                    }
                    ast::ExprKind::Ident(ref name) => {
                        if env.vars.contains(&name.value) {
                            state = State::Analyzed(
                                self.program
                                    .arena
                                    .alloc(ir::Expr::Var(name.value, expr_ast.span)),
                            );
                        } else {
                            return Err(AnalyzeError::UnknownVariable {
                                span: name.span,
                                name: name.value.value().into(),
                            });
                        }
                    }
                    ast::ExprKind::Local(binds_ast, inner_ast) => {
                        let mut inner_env = env.clone();

                        let mut locals_spans = FHashMap::<InternedStr<'p>, SpanId>::default();
                        for bind_ast in binds_ast.iter() {
                            let name = bind_ast.name.value;
                            match locals_spans.entry(name) {
                                HashMapEntry::Occupied(entry) => {
                                    return Err(AnalyzeError::RepeatedLocalName {
                                        original_span: *entry.get(),
                                        repeated_span: bind_ast.name.span,
                                        name: name.value().into(),
                                    });
                                }
                                HashMapEntry::Vacant(entry) => {
                                    entry.insert(bind_ast.name.span);
                                    inner_env.vars.insert(name);
                                }
                            }
                        }

                        let mut bindings = Vec::new();
                        for bind_ast in binds_ast.iter() {
                            let name = bind_ast.name.value;
                            let value = if let Some((params, _)) = bind_ast.params {
                                self.analyze_function(params, &bind_ast.value, &inner_env)?
                            } else {
                                self.analyze_expr(&bind_ast.value, &inner_env, false)?
                            };
                            bindings.push((name, value));
                        }

                        let inner = self.analyze_expr(inner_ast, &inner_env, can_be_tailstrict)?;

                        state = State::Analyzed(self.program.arena.alloc(ir::Expr::Local {
                            bindings: self.program.arena.alloc_slice(&bindings),
                            inner,
                        }));
                    }
                    ast::ExprKind::If(cond_ast, then_body_ast, else_body_ast) => {
                        let cond = self.analyze_expr(cond_ast, env, false)?;
                        let then_body = self.analyze_expr(then_body_ast, env, can_be_tailstrict)?;
                        let else_body = else_body_ast
                            .map(|e| self.analyze_expr(e, env, can_be_tailstrict))
                            .transpose()?;

                        state = State::Analyzed(self.program.arena.alloc(ir::Expr::If {
                            cond,
                            cond_span: cond_ast.span,
                            then_body,
                            else_body,
                        }));
                    }
                    ast::ExprKind::Binary(lhs_ast, op, rhs_ast) => {
                        stack.push(StackItem::BinaryLhs(expr_ast.span, op, rhs_ast));
                        state = State::Expr(lhs_ast, false);
                    }
                    ast::ExprKind::Unary(op, rhs_ast) => {
                        stack.push(StackItem::UnaryRhs(expr_ast.span, op));
                        state = State::Expr(rhs_ast, false);
                    }
                    ast::ExprKind::ObjExt(lhs_ast, ref rhs_ast, _) => {
                        let lhs = self.analyze_expr(lhs_ast, env, false)?;
                        let rhs = self.analyze_objinside(rhs_ast, env)?;

                        state = State::Analyzed(self.program.arena.alloc(ir::Expr::Binary {
                            op: ast::BinaryOp::Add,
                            lhs,
                            rhs,
                            span: expr_ast.span,
                        }));
                    }
                    ast::ExprKind::Func(params_ast, body_ast) => {
                        state = State::Analyzed(self.analyze_function(params_ast, body_ast, env)?);
                    }
                    ast::ExprKind::Assert(assert_ast, inner_ast) => {
                        let assert = self.analyze_assert(assert_ast, env)?;
                        let inner = self.analyze_expr(inner_ast, env, can_be_tailstrict)?;

                        state = State::Analyzed(
                            self.program.arena.alloc(ir::Expr::Assert { assert, inner }),
                        );
                    }
                    ast::ExprKind::Import(path_ast) => match path_ast.kind {
                        ast::ExprKind::String(path) => {
                            state = State::Analyzed(self.program.arena.alloc(ir::Expr::Import {
                                path: self.program.arena.alloc_str(path),
                                span: expr_ast.span,
                            }));
                        }
                        ast::ExprKind::TextBlock(_) => {
                            return Err(AnalyzeError::TextBlockAsImportPath {
                                span: path_ast.span,
                            });
                        }
                        _ => {
                            return Err(AnalyzeError::ComputedImportPath {
                                span: path_ast.span,
                            });
                        }
                    },
                    ast::ExprKind::ImportStr(path_ast) => match path_ast.kind {
                        ast::ExprKind::String(path) => {
                            state =
                                State::Analyzed(self.program.arena.alloc(ir::Expr::ImportStr {
                                    path: self.program.arena.alloc_str(path),
                                    span: expr_ast.span,
                                }));
                        }
                        ast::ExprKind::TextBlock(_) => {
                            return Err(AnalyzeError::TextBlockAsImportPath {
                                span: path_ast.span,
                            });
                        }
                        _ => {
                            return Err(AnalyzeError::ComputedImportPath {
                                span: path_ast.span,
                            });
                        }
                    },
                    ast::ExprKind::ImportBin(path_ast) => match path_ast.kind {
                        ast::ExprKind::String(path) => {
                            state =
                                State::Analyzed(self.program.arena.alloc(ir::Expr::ImportBin {
                                    path: self.program.arena.alloc_str(path),
                                    span: expr_ast.span,
                                }));
                        }
                        ast::ExprKind::TextBlock(_) => {
                            return Err(AnalyzeError::TextBlockAsImportPath {
                                span: path_ast.span,
                            });
                        }
                        _ => {
                            return Err(AnalyzeError::ComputedImportPath {
                                span: path_ast.span,
                            });
                        }
                    },
                    ast::ExprKind::Error(msg_ast) => {
                        let msg = self.analyze_expr(msg_ast, env, false)?;

                        state = State::Analyzed(self.program.arena.alloc(ir::Expr::Error {
                            msg,
                            span: expr_ast.span,
                        }));
                    }
                    ast::ExprKind::InSuper(lhs_ast, super_span) => {
                        if env.is_obj {
                            let lhs = self.analyze_expr(lhs_ast, env, false)?;
                            state = State::Analyzed(self.program.arena.alloc(ir::Expr::InSuper {
                                lhs,
                                span: expr_ast.span,
                            }));
                        } else {
                            return Err(AnalyzeError::SuperOutsideObject { super_span });
                        }
                    }
                },
            }
        }
    }

    fn analyze_objinside(
        &mut self,
        obj_inside_ast: &ast::ObjInside<'p, '_>,
        env: &Env<'p>,
    ) -> Result<&'p ir::Expr<'p>, AnalyzeError> {
        match *obj_inside_ast {
            ast::ObjInside::Members(members_ast) => {
                let mut inner_env = env.clone();
                inner_env.is_obj = true;

                let mut locals_spans = FHashMap::<InternedStr<'p>, SpanId>::default();
                for member_ast in members_ast.iter() {
                    if let ast::Member::Local(local_ast) = member_ast {
                        let name = local_ast.bind.name.value;
                        match locals_spans.entry(name) {
                            HashMapEntry::Occupied(entry) => {
                                return Err(AnalyzeError::RepeatedLocalName {
                                    original_span: *entry.get(),
                                    repeated_span: local_ast.bind.name.span,
                                    name: name.value().into(),
                                });
                            }
                            HashMapEntry::Vacant(entry) => {
                                entry.insert(local_ast.bind.name.span);
                                inner_env.vars.insert(name);
                            }
                        }
                    }
                }

                let mut locals = Vec::new();
                let mut asserts = Vec::new();
                let mut fields = Vec::<ir::ObjectField<'p>>::new();
                let mut fix_fields = FHashMap::<InternedStr<'p>, usize>::default();

                for member_ast in members_ast.iter() {
                    match member_ast {
                        ast::Member::Local(local_ast) => {
                            let name = local_ast.bind.name.value;
                            let value = if let Some((params, _)) = local_ast.bind.params {
                                self.analyze_function(params, &local_ast.bind.value, &inner_env)?
                            } else {
                                self.analyze_expr(&local_ast.bind.value, &inner_env, false)?
                            };
                            locals.push((name, value));
                        }
                        ast::Member::Assert(assert_ast) => {
                            asserts.push(self.analyze_assert(assert_ast, &inner_env)?);
                        }
                        ast::Member::Field(field_ast) => {
                            let value = match *field_ast {
                                ast::Field::Value(_, _, _, ref value_ast) => {
                                    self.analyze_expr(value_ast, &inner_env, false)?
                                }
                                ast::Field::Func(_, params_ast, _, _, ref body_ast) => {
                                    self.analyze_function(params_ast, body_ast, &inner_env)?
                                }
                            };

                            let (field_name_ast, plus, visibility) = match *field_ast {
                                ast::Field::Value(ref name, plus, visibility, _) => {
                                    (name, plus, visibility)
                                }
                                ast::Field::Func(ref name, _, _, visibility, _) => {
                                    (name, false, visibility)
                                }
                            };
                            let name = match *field_name_ast {
                                ast::FieldName::Ident(ast::Ident {
                                    value: field_name,
                                    span: field_name_span,
                                })
                                | ast::FieldName::String(field_name, field_name_span) => {
                                    match fix_fields.entry(field_name) {
                                        HashMapEntry::Occupied(entry) => {
                                            return Err(AnalyzeError::RepeatedFieldName {
                                                original_span: fields[*entry.get()].name_span,
                                                repeated_span: field_name_span,
                                                name: field_name.value().into(),
                                            });
                                        }
                                        HashMapEntry::Vacant(entry) => {
                                            entry.insert(fields.len());
                                            Some((ir::FieldName::Fix(field_name), field_name_span))
                                        }
                                    }
                                }
                                ast::FieldName::Expr(ref name_ast, name_span) => {
                                    let name = self.analyze_expr(name_ast, env, false)?;
                                    Some((ir::FieldName::Dyn(name), name_span))
                                }
                            };

                            if let Some((name, name_span)) = name {
                                fields.push(ir::ObjectField {
                                    name,
                                    name_span,
                                    plus,
                                    visibility,
                                    value,
                                });
                            }
                        }
                    }
                }

                Ok(self.program.arena.alloc(ir::Expr::Object {
                    is_top: !env.is_obj,
                    locals: self.program.arena.alloc_slice(&locals),
                    asserts: self.program.arena.alloc_slice(&asserts),
                    fields: self.program.arena.alloc_slice(&fields),
                }))
            }
            ast::ObjInside::Comp {
                locals1: locals1_ast,
                name: name_ast,
                body: body_ast,
                locals2: locals2_ast,
                comp_spec: comp_spec_ast,
            } => {
                let (comp_spec, env) = self.analyze_comp_spec(comp_spec_ast, env)?;
                let mut inner_env = env.clone();
                inner_env.is_obj = true;

                let mut locals_spans = FHashMap::default();
                for local_ast in locals1_ast.iter().chain(locals2_ast.iter()) {
                    let name = local_ast.bind.name.value;
                    match locals_spans.entry(name) {
                        HashMapEntry::Occupied(entry) => {
                            return Err(AnalyzeError::RepeatedLocalName {
                                original_span: *entry.get(),
                                repeated_span: local_ast.bind.name.span,
                                name: name.value().into(),
                            });
                        }
                        HashMapEntry::Vacant(entry) => {
                            entry.insert(local_ast.bind.name.span);
                            inner_env.vars.insert(name);
                        }
                    }
                }

                let mut locals = Vec::new();
                for local_ast in locals1_ast.iter().chain(locals2_ast.iter()) {
                    let name = local_ast.bind.name.value;
                    let value = if let Some((params, _)) = local_ast.bind.params {
                        self.analyze_function(params, &local_ast.bind.value, &inner_env)?
                    } else {
                        self.analyze_expr(&local_ast.bind.value, &inner_env, false)?
                    };
                    locals.push((name, value));
                }

                let field_name = self.analyze_expr(name_ast, &env, false)?;
                let field_value = self.analyze_expr(body_ast, &inner_env, false)?;

                Ok(self.program.arena.alloc(ir::Expr::ObjectComp {
                    is_top: !env.is_obj,
                    locals: self.program.arena.alloc_slice(&locals),
                    field_name,
                    field_name_span: name_ast.span,
                    field_value,
                    comp_spec,
                }))
            }
        }
    }

    fn analyze_function(
        &mut self,
        params_ast: &[ast::Param<'p, '_>],
        body_ast: &ast::Expr<'p, '_>,
        env: &Env<'p>,
    ) -> Result<&'p ir::Expr<'p>, AnalyzeError> {
        let mut inner_env = env.clone();

        let mut params_spans = FHashMap::default();
        for param_ast in params_ast.iter() {
            let name = param_ast.name.value;
            match params_spans.entry(name) {
                HashMapEntry::Occupied(entry) => {
                    return Err(AnalyzeError::RepeatedParamName {
                        original_span: *entry.get(),
                        repeated_span: param_ast.name.span,
                        name: name.value().into(),
                    });
                }
                HashMapEntry::Vacant(entry) => {
                    entry.insert(param_ast.name.span);
                    inner_env.vars.insert(name);
                }
            }
        }

        let mut params = Vec::new();
        for param_ast in params_ast.iter() {
            let name = param_ast.name.value;
            let default_value = param_ast
                .default_value
                .as_ref()
                .map(|e| self.analyze_expr(e, &inner_env, false))
                .transpose()?;
            params.push((name, default_value));
        }

        let body = self.analyze_expr(body_ast, &inner_env, true)?;

        Ok(self.program.arena.alloc(ir::Expr::Func {
            params: self.program.arena.alloc_slice(&params),
            body,
        }))
    }

    fn analyze_assert(
        &mut self,
        assert_ast: &ast::Assert<'p, '_>,
        env: &Env<'p>,
    ) -> Result<ir::Assert<'p>, AnalyzeError> {
        let cond = self.analyze_expr(&assert_ast.cond, env, false)?;
        let msg = assert_ast
            .msg
            .as_ref()
            .map(|e| self.analyze_expr(e, env, false))
            .transpose()?;
        Ok(ir::Assert {
            span: assert_ast.span,
            cond,
            cond_span: assert_ast.cond.span,
            msg,
        })
    }

    fn analyze_comp_spec(
        &mut self,
        comp_spec_ast: &[ast::CompSpecPart<'p, '_>],
        env: &Env<'p>,
    ) -> Result<(&'p [ir::CompSpecPart<'p>], Env<'p>), AnalyzeError> {
        let mut parts = Vec::new();
        let mut current_env = env.clone();
        for part_ast in comp_spec_ast.iter() {
            match part_ast {
                ast::CompSpecPart::For(for_spec_ast) => {
                    let inner = self.analyze_expr(&for_spec_ast.inner, &current_env, false)?;
                    current_env.vars.insert(for_spec_ast.var.value);
                    parts.push(ir::CompSpecPart::For {
                        var: for_spec_ast.var.value,
                        value: inner,
                        value_span: for_spec_ast.inner.span,
                    });
                }
                ast::CompSpecPart::If(if_spec_ast) => {
                    let cond = self.analyze_expr(&if_spec_ast.cond, &current_env, false)?;
                    parts.push(ir::CompSpecPart::If {
                        cond,
                        cond_span: if_spec_ast.cond.span,
                    });
                }
            }
        }

        Ok((self.program.arena.alloc_slice(&parts), current_env))
    }
}

#[derive(Clone)]
pub(super) struct Env<'p> {
    pub(super) is_obj: bool,
    pub(super) vars: FHashSet<InternedStr<'p>>,
}
