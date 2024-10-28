use std::collections::hash_map::Entry as HashMapEntry;
use std::rc::Rc;

use super::{ir, AnalyzeError, Program};
use crate::interner::InternedStr;
use crate::span::SpanId;
use crate::{ast, FHashMap, FHashSet};

pub(super) struct Analyzer<'a> {
    program: &'a Program,
}

impl<'a> Analyzer<'a> {
    pub(super) fn new(program: &'a Program) -> Self {
        Self { program }
    }

    pub(super) fn analyze(
        mut self,
        ast: &ast::Expr,
        env: FHashSet<InternedStr>,
    ) -> Result<ir::RcExpr, AnalyzeError> {
        let env = Env {
            is_obj: false,
            vars: env,
        };
        self.analyze_expr(ast, &env, false)
    }

    fn analyze_expr(
        &mut self,
        root_expr_ast: &ast::Expr,
        env: &Env,
        can_be_tailstrict: bool,
    ) -> Result<ir::RcExpr, AnalyzeError> {
        enum State<'a> {
            Analyzed(ir::RcExpr),
            Expr(&'a ast::Expr),
        }

        enum StackItem<'a> {
            ArrayItem(SpanId, Vec<ir::RcExpr>, &'a [ast::Expr]),
            BinaryLhs(SpanId, ast::BinaryOp, &'a ast::Expr),
            BinaryRhs(SpanId, ast::BinaryOp, ir::RcExpr),
            UnaryRhs(SpanId, ast::UnaryOp),
        }

        let mut stack = Vec::new();
        let mut state = State::Expr(root_expr_ast);

        loop {
            match state {
                State::Analyzed(expr_ir) => match stack.pop() {
                    None => return Ok(expr_ir),
                    Some(StackItem::ArrayItem(span, mut items_ir, rem_items_ast)) => {
                        items_ir.push(expr_ir);

                        if let Some((next_item_ast, rem_items_ast)) = rem_items_ast.split_first() {
                            stack.push(StackItem::ArrayItem(span, items_ir, rem_items_ast));
                            state = State::Expr(next_item_ast);
                        } else {
                            state = State::Analyzed(ir::RcExpr::new(ir::Expr::Array(items_ir)));
                        }
                    }
                    Some(StackItem::BinaryLhs(span, op, rhs_ast)) => {
                        stack.push(StackItem::BinaryRhs(span, op, expr_ir));
                        state = State::Expr(rhs_ast);
                    }
                    Some(StackItem::BinaryRhs(span, op, lhs)) => {
                        let rhs = expr_ir;
                        state = State::Analyzed(ir::RcExpr::new(ir::Expr::Binary {
                            op,
                            lhs,
                            rhs,
                            span,
                        }));
                    }
                    Some(StackItem::UnaryRhs(span, op)) => {
                        let rhs = expr_ir;
                        state = State::Analyzed(ir::RcExpr::new(ir::Expr::Unary { op, rhs, span }));
                    }
                },
                State::Expr(expr_ast) => match expr_ast.kind {
                    ast::ExprKind::Null => {
                        state = State::Analyzed(self.program.null_expr.clone());
                    }
                    ast::ExprKind::Bool(value) => {
                        state = State::Analyzed(if value {
                            self.program.true_expr.clone()
                        } else {
                            self.program.false_expr.clone()
                        });
                    }
                    ast::ExprKind::SelfObj => {
                        if env.is_obj {
                            state = State::Analyzed(self.program.self_obj_expr.clone());
                        } else {
                            return Err(AnalyzeError::SelfOutsideObject {
                                self_span: expr_ast.span,
                            });
                        }
                    }
                    ast::ExprKind::Dollar => {
                        if env.is_obj {
                            state = State::Analyzed(self.program.top_obj_expr.clone())
                        } else {
                            return Err(AnalyzeError::DollarOutsideObject {
                                dollar_span: expr_ast.span,
                            });
                        }
                    }
                    ast::ExprKind::String(ref s) => {
                        state = State::Analyzed(ir::RcExpr::new(ir::Expr::String((**s).into())));
                    }
                    ast::ExprKind::TextBlock(ref s) => {
                        state = State::Analyzed(ir::RcExpr::new(ir::Expr::String((**s).into())));
                    }
                    ast::ExprKind::Number(ref value) => {
                        let float_value =
                            format!("{}e{}", value.digits, value.exp).parse().unwrap();
                        state = State::Analyzed(ir::RcExpr::new(ir::Expr::Number(
                            float_value,
                            expr_ast.span,
                        )));
                    }
                    ast::ExprKind::Paren(ref inner) => {
                        state = State::Expr(inner);
                    }
                    ast::ExprKind::Object(ref inside) => {
                        state = State::Analyzed(self.analyze_objinside(inside, env)?);
                    }
                    ast::ExprKind::Array(ref items_ast) => {
                        if let Some((first_item_ast, rem_items_ast)) = items_ast.split_first() {
                            stack.push(StackItem::ArrayItem(
                                expr_ast.span,
                                Vec::new(),
                                rem_items_ast,
                            ));
                            state = State::Expr(first_item_ast);
                        } else {
                            state = State::Analyzed(ir::RcExpr::new(ir::Expr::Array(Vec::new())));
                        }
                    }
                    ast::ExprKind::ArrayComp(ref body_ast, ref comp_spec_ast) => {
                        let (comp_spec, env) = self.analyze_comp_spec(comp_spec_ast, env)?;
                        let body = self.analyze_expr(body_ast, &env, false)?;
                        state = State::Analyzed(ir::RcExpr::new(ir::Expr::ArrayComp {
                            value: body,
                            comp_spec,
                        }));
                    }
                    ast::ExprKind::Field(ref obj_ast, ref field_name_ast) => {
                        let object = self.analyze_expr(obj_ast, env, false)?;
                        state = State::Analyzed(ir::RcExpr::new(ir::Expr::Field {
                            object,
                            field_name: field_name_ast.value.clone(),
                            expr_span: expr_ast.span,
                        }));
                    }
                    ast::ExprKind::Index(ref obj_ast, ref index_ast) => {
                        let object = self.analyze_expr(obj_ast, env, false)?;
                        let index = self.analyze_expr(index_ast, env, false)?;
                        state = State::Analyzed(ir::RcExpr::new(ir::Expr::Index {
                            object,
                            index,
                            expr_span: expr_ast.span,
                        }));
                    }
                    ast::ExprKind::Slice(
                        ref object_ast,
                        ref start_index_ast,
                        ref end_index_ast,
                        ref step_ast,
                    ) => {
                        let object = self.analyze_expr(object_ast, env, false)?;
                        let start_index = start_index_ast
                            .as_ref()
                            .map(|e| self.analyze_expr(e, env, false))
                            .transpose()?
                            .unwrap_or_else(|| ir::RcExpr::new(ir::Expr::Null));
                        let end_index = end_index_ast
                            .as_ref()
                            .map(|e| self.analyze_expr(e, env, false))
                            .transpose()?
                            .unwrap_or_else(|| ir::RcExpr::new(ir::Expr::Null));
                        let step = step_ast
                            .as_ref()
                            .map(|e| self.analyze_expr(e, env, false))
                            .transpose()?
                            .unwrap_or_else(|| ir::RcExpr::new(ir::Expr::Null));

                        state = State::Analyzed(ir::RcExpr::new(ir::Expr::Call {
                            callee: ir::RcExpr::new(ir::Expr::StdField {
                                field_name: self.program.intern_str("slice"),
                            }),
                            positional_args: vec![object, start_index, end_index, step],
                            named_args: Vec::new(),
                            tailstrict: false,
                            span: expr_ast.span,
                        }));
                    }
                    ast::ExprKind::SuperField(super_span, ref field_name_ast) => {
                        if env.is_obj {
                            state = State::Analyzed(ir::RcExpr::new(ir::Expr::SuperField {
                                super_span,
                                field_name: field_name_ast.value.clone(),
                                expr_span: expr_ast.span,
                            }));
                        } else {
                            return Err(AnalyzeError::SuperOutsideObject { super_span });
                        }
                    }
                    ast::ExprKind::SuperIndex(super_span, ref index_ast) => {
                        if env.is_obj {
                            let index = self.analyze_expr(index_ast, env, false)?;
                            state = State::Analyzed(ir::RcExpr::new(ir::Expr::SuperIndex {
                                super_span,
                                index,
                                expr_span: expr_ast.span,
                            }));
                        } else {
                            return Err(AnalyzeError::SuperOutsideObject { super_span });
                        }
                    }
                    ast::ExprKind::Call(ref callee_ast, ref args_ast, tailstrict) => {
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
                                        arg_name.value.clone(),
                                        arg_name.span,
                                        self.analyze_expr(value_ast, env, false)?,
                                    ));
                                }
                            }
                        }

                        state = State::Analyzed(ir::RcExpr::new(ir::Expr::Call {
                            callee,
                            positional_args,
                            named_args,
                            tailstrict: can_be_tailstrict && tailstrict,
                            span: expr_ast.span,
                        }));
                    }
                    ast::ExprKind::Ident(ref name) => {
                        if env.vars.contains(&name.value) {
                            state = State::Analyzed(ir::RcExpr::new(ir::Expr::Var(
                                name.value.clone(),
                                expr_ast.span,
                            )));
                        } else {
                            return Err(AnalyzeError::UnknownVariable {
                                span: name.span,
                                name: name.value.value().into(),
                            });
                        }
                    }
                    ast::ExprKind::Local(ref binds_ast, ref inner_ast) => {
                        let mut inner_env = env.clone();

                        let mut locals_spans = FHashMap::<InternedStr, SpanId>::default();
                        for bind_ast in binds_ast.iter() {
                            let name = &bind_ast.name.value;
                            match locals_spans.entry(name.clone()) {
                                HashMapEntry::Occupied(entry) => {
                                    return Err(AnalyzeError::RepeatedLocalName {
                                        original_span: *entry.get(),
                                        repeated_span: bind_ast.name.span,
                                        name: name.value().into(),
                                    });
                                }
                                HashMapEntry::Vacant(entry) => {
                                    entry.insert(bind_ast.name.span);
                                    inner_env.vars.insert(name.clone());
                                }
                            }
                        }

                        let mut bindings = Vec::new();
                        for bind_ast in binds_ast.iter() {
                            let name = &bind_ast.name.value;
                            let value = if let Some((ref params, _)) = bind_ast.params {
                                self.analyze_function(params, &bind_ast.value, &inner_env)?
                            } else {
                                self.analyze_expr(&bind_ast.value, &inner_env, false)?
                            };
                            bindings.push((name.clone(), value));
                        }

                        let inner = self.analyze_expr(inner_ast, &inner_env, can_be_tailstrict)?;

                        state =
                            State::Analyzed(ir::RcExpr::new(ir::Expr::Local { bindings, inner }));
                    }
                    ast::ExprKind::If(ref cond_ast, ref then_body_ast, ref else_body_ast) => {
                        let cond = self.analyze_expr(cond_ast, env, false)?;
                        let then_body = self.analyze_expr(then_body_ast, env, can_be_tailstrict)?;
                        let else_body = else_body_ast
                            .as_ref()
                            .map(|e| self.analyze_expr(e, env, can_be_tailstrict))
                            .transpose()?;

                        state = State::Analyzed(ir::RcExpr::new(ir::Expr::If {
                            cond,
                            cond_span: cond_ast.span,
                            then_body,
                            else_body,
                        }));
                    }
                    ast::ExprKind::Binary(ref lhs_ast, op, ref rhs_ast) => {
                        stack.push(StackItem::BinaryLhs(expr_ast.span, op, rhs_ast));
                        state = State::Expr(lhs_ast);
                    }
                    ast::ExprKind::Unary(op, ref rhs_ast) => {
                        stack.push(StackItem::UnaryRhs(expr_ast.span, op));
                        state = State::Expr(rhs_ast);
                    }
                    ast::ExprKind::ObjExt(ref lhs_ast, ref rhs_ast, _) => {
                        let lhs = self.analyze_expr(lhs_ast, env, false)?;
                        let rhs = self.analyze_objinside(rhs_ast, env)?;

                        state = State::Analyzed(ir::RcExpr::new(ir::Expr::Binary {
                            op: ast::BinaryOp::Add,
                            lhs,
                            rhs,
                            span: expr_ast.span,
                        }));
                    }
                    ast::ExprKind::Func(ref params_ast, ref body_ast) => {
                        state = State::Analyzed(self.analyze_function(params_ast, body_ast, env)?);
                    }
                    ast::ExprKind::Assert(ref assert_ast, ref inner_ast) => {
                        let assert = self.analyze_assert(assert_ast, env)?;
                        let inner = self.analyze_expr(inner_ast, env, can_be_tailstrict)?;

                        state =
                            State::Analyzed(ir::RcExpr::new(ir::Expr::Assert { assert, inner }));
                    }
                    ast::ExprKind::Import(ref path_ast) => match path_ast.kind {
                        ast::ExprKind::String(ref path) => {
                            state = State::Analyzed(ir::RcExpr::new(ir::Expr::Import {
                                path: (**path).into(),
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
                    ast::ExprKind::ImportStr(ref path_ast) => match path_ast.kind {
                        ast::ExprKind::String(ref path) => {
                            state = State::Analyzed(ir::RcExpr::new(ir::Expr::ImportStr {
                                path: (**path).into(),
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
                    ast::ExprKind::ImportBin(ref path_ast) => match path_ast.kind {
                        ast::ExprKind::String(ref path) => {
                            state = State::Analyzed(ir::RcExpr::new(ir::Expr::ImportBin {
                                path: (**path).into(),
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
                    ast::ExprKind::Error(ref msg_ast) => {
                        let msg = self.analyze_expr(msg_ast, env, false)?;

                        state = State::Analyzed(ir::RcExpr::new(ir::Expr::Error {
                            msg,
                            span: expr_ast.span,
                        }));
                    }
                    ast::ExprKind::InSuper(ref lhs_ast, super_span) => {
                        if env.is_obj {
                            let lhs = self.analyze_expr(lhs_ast, env, false)?;
                            state = State::Analyzed(ir::RcExpr::new(ir::Expr::InSuper {
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
        obj_inside_ast: &ast::ObjInside,
        env: &Env,
    ) -> Result<ir::RcExpr, AnalyzeError> {
        match *obj_inside_ast {
            ast::ObjInside::Members(ref members_ast) => {
                let mut inner_env = env.clone();
                inner_env.is_obj = true;

                let mut locals_spans = FHashMap::<InternedStr, SpanId>::default();
                for member_ast in members_ast.iter() {
                    if let ast::Member::Local(local_ast) = member_ast {
                        let name = &local_ast.bind.name.value;
                        match locals_spans.entry(name.clone()) {
                            HashMapEntry::Occupied(entry) => {
                                return Err(AnalyzeError::RepeatedLocalName {
                                    original_span: *entry.get(),
                                    repeated_span: local_ast.bind.name.span,
                                    name: name.value().into(),
                                });
                            }
                            HashMapEntry::Vacant(entry) => {
                                entry.insert(local_ast.bind.name.span);
                                inner_env.vars.insert(name.clone());
                            }
                        }
                    }
                }

                let mut locals = Vec::new();
                let mut asserts = Vec::new();
                let mut fields = Vec::<ir::ObjectField>::new();
                let mut fix_fields = FHashMap::<InternedStr, usize>::default();

                for member_ast in members_ast.iter() {
                    match member_ast {
                        ast::Member::Local(local_ast) => {
                            let name = &local_ast.bind.name.value;
                            let value = if let Some((ref params, _)) = local_ast.bind.params {
                                self.analyze_function(params, &local_ast.bind.value, &inner_env)?
                            } else {
                                self.analyze_expr(&local_ast.bind.value, &inner_env, false)?
                            };
                            locals.push((name.clone(), value));
                        }
                        ast::Member::Assert(assert_ast) => {
                            asserts.push(self.analyze_assert(assert_ast, &inner_env)?);
                        }
                        ast::Member::Field(field_ast) => {
                            let value = match *field_ast {
                                ast::Field::Value(_, _, _, ref value_ast) => {
                                    self.analyze_expr(value_ast, &inner_env, false)?
                                }
                                ast::Field::Func(_, ref params_ast, _, _, ref body_ast) => {
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
                                    value: ref field_name,
                                    span: field_name_span,
                                })
                                | ast::FieldName::String(ref field_name, field_name_span) => {
                                    match fix_fields.entry(field_name.clone()) {
                                        HashMapEntry::Occupied(entry) => {
                                            return Err(AnalyzeError::RepeatedFieldName {
                                                original_span: fields[*entry.get()].name_span,
                                                repeated_span: field_name_span,
                                                name: field_name.value().into(),
                                            });
                                        }
                                        HashMapEntry::Vacant(entry) => {
                                            entry.insert(fields.len());
                                            Some((
                                                ir::FieldName::Fix(field_name.clone()),
                                                field_name_span,
                                            ))
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

                Ok(ir::RcExpr::new(ir::Expr::Object {
                    is_top: !env.is_obj,
                    locals: Rc::new(locals),
                    asserts,
                    fields,
                }))
            }
            ast::ObjInside::Comp {
                locals1: ref locals1_ast,
                name: ref name_ast,
                body: ref body_ast,
                locals2: ref locals2_ast,
                comp_spec: ref comp_spec_ast,
            } => {
                let (comp_spec, env) = self.analyze_comp_spec(comp_spec_ast, env)?;
                let mut inner_env = env.clone();
                inner_env.is_obj = true;

                let mut locals_spans = FHashMap::default();
                for local_ast in locals1_ast.iter().chain(locals2_ast.iter()) {
                    let name = &local_ast.bind.name.value;
                    match locals_spans.entry(name.clone()) {
                        HashMapEntry::Occupied(entry) => {
                            return Err(AnalyzeError::RepeatedLocalName {
                                original_span: *entry.get(),
                                repeated_span: local_ast.bind.name.span,
                                name: name.value().into(),
                            });
                        }
                        HashMapEntry::Vacant(entry) => {
                            entry.insert(local_ast.bind.name.span);
                            inner_env.vars.insert(name.clone());
                        }
                    }
                }

                let mut locals = Vec::new();
                for local_ast in locals1_ast.iter().chain(locals2_ast.iter()) {
                    let name = &local_ast.bind.name.value;
                    let value = if let Some((ref params, _)) = local_ast.bind.params {
                        self.analyze_function(params, &local_ast.bind.value, &inner_env)?
                    } else {
                        self.analyze_expr(&local_ast.bind.value, &inner_env, false)?
                    };
                    locals.push((name.clone(), value));
                }

                let field_name = self.analyze_expr(name_ast, &env, false)?;
                let field_value = self.analyze_expr(body_ast, &inner_env, false)?;

                Ok(ir::RcExpr::new(ir::Expr::ObjectComp {
                    is_top: !env.is_obj,
                    locals: Rc::new(locals),
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
        params_ast: &[ast::Param],
        body_ast: &ast::Expr,
        env: &Env,
    ) -> Result<ir::RcExpr, AnalyzeError> {
        let mut inner_env = env.clone();

        let mut params_spans = FHashMap::default();
        for param_ast in params_ast.iter() {
            let name = &param_ast.name.value;
            match params_spans.entry(name.clone()) {
                HashMapEntry::Occupied(entry) => {
                    return Err(AnalyzeError::RepeatedParamName {
                        original_span: *entry.get(),
                        repeated_span: param_ast.name.span,
                        name: name.value().into(),
                    });
                }
                HashMapEntry::Vacant(entry) => {
                    entry.insert(param_ast.name.span);
                    inner_env.vars.insert(name.clone());
                }
            }
        }

        let mut params = Vec::new();
        for param_ast in params_ast.iter() {
            let name = &param_ast.name.value;
            let default_value = param_ast
                .default_value
                .as_ref()
                .map(|e| self.analyze_expr(e, &inner_env, false))
                .transpose()?;
            params.push((name.clone(), default_value));
        }

        let body = self.analyze_expr(body_ast, &inner_env, true)?;

        Ok(ir::RcExpr::new(ir::Expr::Func {
            params: Rc::new(params),
            body,
        }))
    }

    fn analyze_assert(
        &mut self,
        assert_ast: &ast::Assert,
        env: &Env,
    ) -> Result<ir::Assert, AnalyzeError> {
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
        comp_spec_ast: &[ast::CompSpecPart],
        env: &Env,
    ) -> Result<(Vec<ir::CompSpecPart>, Env), AnalyzeError> {
        let mut parts = Vec::new();
        let mut current_env = env.clone();
        for part_ast in comp_spec_ast.iter() {
            match part_ast {
                ast::CompSpecPart::For(for_spec_ast) => {
                    let inner = self.analyze_expr(&for_spec_ast.inner, &current_env, false)?;
                    current_env.vars.insert(for_spec_ast.var.value.clone());
                    parts.push(ir::CompSpecPart::For {
                        var: for_spec_ast.var.value.clone(),
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

        Ok((parts, current_env))
    }
}

#[derive(Clone)]
pub(super) struct Env {
    pub(super) is_obj: bool,
    pub(super) vars: FHashSet<InternedStr>,
}
