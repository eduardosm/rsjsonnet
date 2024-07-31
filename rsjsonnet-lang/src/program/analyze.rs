use std::collections::hash_map::Entry as HashMapEntry;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use super::{ir, AnalyzeError, Program};
use crate::ast;
use crate::interner::InternedStr;
use crate::span::SpanId;

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
        env: HashSet<InternedStr>,
    ) -> Result<Rc<ir::Expr>, AnalyzeError> {
        let env = Env {
            is_obj: false,
            vars: env,
        };
        self.analyze_expr(ast, &env, false)
    }

    fn analyze_expr(
        &mut self,
        mut expr_ast: &ast::Expr,
        env: &Env,
        can_be_tailstrict: bool,
    ) -> Result<Rc<ir::Expr>, AnalyzeError> {
        while let ast::ExprKind::Paren(ref inner) = expr_ast.kind {
            expr_ast = inner;
        }

        let ir_expr = match expr_ast.kind {
            ast::ExprKind::Null => self.program.null_expr.clone(),
            ast::ExprKind::Bool(value) => {
                if value {
                    self.program.true_expr.clone()
                } else {
                    self.program.false_expr.clone()
                }
            }
            ast::ExprKind::SelfObj => {
                if env.is_obj {
                    self.program.self_obj_expr.clone()
                } else {
                    return Err(AnalyzeError::SelfOutsideObject {
                        self_span: expr_ast.span,
                    });
                }
            }
            ast::ExprKind::Dollar => {
                if env.is_obj {
                    self.program.top_obj_expr.clone()
                } else {
                    return Err(AnalyzeError::DollarOutsideObject {
                        dollar_span: expr_ast.span,
                    });
                }
            }
            ast::ExprKind::String(ref s) => Rc::new(ir::Expr::String((**s).into())),
            ast::ExprKind::TextBlock(ref s) => Rc::new(ir::Expr::String((**s).into())),
            ast::ExprKind::Number(ref value) => {
                let float_value = format!("{}e{}", value.digits, value.exp).parse().unwrap();
                Rc::new(ir::Expr::Number(float_value, expr_ast.span))
            }
            ast::ExprKind::Paren(_) => unreachable!(),
            ast::ExprKind::Object(ref inside) => {
                return self.analyze_objinside(inside, env);
            }
            ast::ExprKind::Array(ref items_ast) => {
                let mut items = Vec::with_capacity(items_ast.len());
                for item_ast in items_ast.iter() {
                    items.push(self.analyze_expr(item_ast, env, false)?);
                }
                Rc::new(ir::Expr::Array(items))
            }
            ast::ExprKind::ArrayComp(ref body_ast, ref comp_spec_ast) => {
                let (comp_spec, env) = self.analyze_comp_spec(comp_spec_ast, env)?;
                let body = self.analyze_expr(body_ast, &env, false)?;
                Rc::new(ir::Expr::ArrayComp {
                    value: body,
                    comp_spec,
                })
            }
            ast::ExprKind::Field(ref obj_ast, ref field_name_ast) => {
                let object = self.analyze_expr(obj_ast, env, false)?;
                Rc::new(ir::Expr::Field {
                    object,
                    field_name: field_name_ast.value.clone(),
                    expr_span: expr_ast.span,
                })
            }
            ast::ExprKind::Index(ref obj_ast, ref index_ast) => {
                let object = self.analyze_expr(obj_ast, env, false)?;
                let index = self.analyze_expr(index_ast, env, false)?;
                Rc::new(ir::Expr::Index {
                    object,
                    index,
                    expr_span: expr_ast.span,
                })
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
                    .unwrap_or_else(|| Rc::new(ir::Expr::Null));
                let end_index = end_index_ast
                    .as_ref()
                    .map(|e| self.analyze_expr(e, env, false))
                    .transpose()?
                    .unwrap_or_else(|| Rc::new(ir::Expr::Null));
                let step = step_ast
                    .as_ref()
                    .map(|e| self.analyze_expr(e, env, false))
                    .transpose()?
                    .unwrap_or_else(|| Rc::new(ir::Expr::Null));

                Rc::new(ir::Expr::Call {
                    callee: Rc::new(ir::Expr::StdField {
                        field_name: self.program.str_interner.intern("slice"),
                    }),
                    positional_args: vec![object, start_index, end_index, step],
                    named_args: Vec::new(),
                    tailstrict: false,
                    span: expr_ast.span,
                })
            }
            ast::ExprKind::SuperField(super_span, ref field_name_ast) => {
                if env.is_obj {
                    Rc::new(ir::Expr::SuperField {
                        super_span,
                        field_name: field_name_ast.value.clone(),
                        expr_span: expr_ast.span,
                    })
                } else {
                    return Err(AnalyzeError::SuperOutsideObject { super_span });
                }
            }
            ast::ExprKind::SuperIndex(super_span, ref index_ast) => {
                if env.is_obj {
                    let index = self.analyze_expr(index_ast, env, false)?;
                    Rc::new(ir::Expr::SuperIndex {
                        super_span,
                        index,
                        expr_span: expr_ast.span,
                    })
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

                Rc::new(ir::Expr::Call {
                    callee,
                    positional_args,
                    named_args,
                    tailstrict: can_be_tailstrict && tailstrict,
                    span: expr_ast.span,
                })
            }
            ast::ExprKind::Ident(ref name) => {
                if env.vars.contains(&name.value) {
                    Rc::new(ir::Expr::Var(name.value.clone(), expr_ast.span))
                } else {
                    return Err(AnalyzeError::UnknownVariable {
                        span: name.span,
                        name: name.value.value().into(),
                    });
                }
            }
            ast::ExprKind::Local(ref binds_ast, ref inner_ast) => {
                let mut inner_env = env.clone();

                let mut locals_spans = HashMap::<InternedStr, SpanId>::new();
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

                let mut bindings = HashMap::new();
                for bind_ast in binds_ast.iter() {
                    let name = &bind_ast.name.value;
                    if let HashMapEntry::Vacant(entry) = bindings.entry(name.clone()) {
                        let value = if let Some((ref params, _)) = bind_ast.params {
                            self.analyze_function(params, &bind_ast.value, &inner_env)?
                        } else {
                            self.analyze_expr(&bind_ast.value, &inner_env, false)?
                        };
                        entry.insert(value);
                    }
                }

                let inner = self.analyze_expr(inner_ast, &inner_env, can_be_tailstrict)?;

                Rc::new(ir::Expr::Local { bindings, inner })
            }
            ast::ExprKind::If(ref cond_ast, ref then_body_ast, ref else_body_ast) => {
                let cond = self.analyze_expr(cond_ast, env, false)?;
                let then_body = self.analyze_expr(then_body_ast, env, can_be_tailstrict)?;
                let else_body = else_body_ast
                    .as_ref()
                    .map(|e| self.analyze_expr(e, env, can_be_tailstrict))
                    .transpose()?;

                Rc::new(ir::Expr::If {
                    cond,
                    cond_span: cond_ast.span,
                    then_body,
                    else_body,
                })
            }
            ast::ExprKind::Binary(ref lhs_ast, op, ref rhs_ast) => {
                let lhs = self.analyze_expr(lhs_ast, env, false)?;
                let rhs = self.analyze_expr(rhs_ast, env, false)?;

                match op {
                    ast::BinaryOp::Rem => Rc::new(ir::Expr::Call {
                        callee: Rc::new(ir::Expr::StdField {
                            field_name: self.program.str_interner.intern("mod"),
                        }),
                        positional_args: vec![lhs, rhs],
                        named_args: Vec::new(),
                        tailstrict: false,
                        span: expr_ast.span,
                    }),
                    _ => Rc::new(ir::Expr::Binary {
                        op,
                        lhs,
                        rhs,
                        span: expr_ast.span,
                    }),
                }
            }
            ast::ExprKind::Unary(op, ref rhs_ast) => {
                let rhs = self.analyze_expr(rhs_ast, env, false)?;

                Rc::new(ir::Expr::Unary {
                    op,
                    rhs,
                    span: expr_ast.span,
                })
            }
            ast::ExprKind::ObjExt(ref lhs_ast, ref rhs_ast, _) => {
                let lhs = self.analyze_expr(lhs_ast, env, false)?;
                let rhs = self.analyze_objinside(rhs_ast, env)?;

                Rc::new(ir::Expr::Binary {
                    op: ast::BinaryOp::Add,
                    lhs,
                    rhs,
                    span: expr_ast.span,
                })
            }
            ast::ExprKind::Func(ref params_ast, ref body_ast) => {
                return self.analyze_function(params_ast, body_ast, env);
            }
            ast::ExprKind::Assert(ref assert_ast, ref inner_ast) => {
                let assert = self.analyze_assert(assert_ast, env)?;
                let inner = self.analyze_expr(inner_ast, env, can_be_tailstrict)?;

                Rc::new(ir::Expr::Assert { assert, inner })
            }
            ast::ExprKind::Import(ref path_ast) => match path_ast.kind {
                ast::ExprKind::String(ref path) => Rc::new(ir::Expr::Import {
                    path: (**path).into(),
                    span: expr_ast.span,
                }),
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
                ast::ExprKind::String(ref path) => Rc::new(ir::Expr::ImportStr {
                    path: (**path).into(),
                    span: expr_ast.span,
                }),
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
                ast::ExprKind::String(ref path) => Rc::new(ir::Expr::ImportBin {
                    path: (**path).into(),
                    span: expr_ast.span,
                }),
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

                Rc::new(ir::Expr::Error {
                    msg,
                    span: expr_ast.span,
                })
            }
            ast::ExprKind::InSuper(ref lhs_ast, super_span) => {
                if env.is_obj {
                    let lhs = self.analyze_expr(lhs_ast, env, false)?;
                    Rc::new(ir::Expr::InSuper {
                        lhs,
                        span: expr_ast.span,
                    })
                } else {
                    return Err(AnalyzeError::SuperOutsideObject { super_span });
                }
            }
        };
        Ok(ir_expr)
    }

    fn analyze_objinside(
        &mut self,
        obj_inside_ast: &ast::ObjInside,
        env: &Env,
    ) -> Result<Rc<ir::Expr>, AnalyzeError> {
        match *obj_inside_ast {
            ast::ObjInside::Members(ref members_ast) => {
                let mut inner_env = env.clone();
                inner_env.is_obj = true;

                let mut locals_spans = HashMap::<InternedStr, SpanId>::new();
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

                let mut locals = HashMap::new();
                let mut asserts = Vec::new();
                let mut fields = Vec::<ir::ObjectField>::new();
                let mut fix_fields = HashMap::<InternedStr, usize>::new();

                for member_ast in members_ast.iter() {
                    match member_ast {
                        ast::Member::Local(local_ast) => {
                            let name = &local_ast.bind.name.value;
                            if let HashMapEntry::Vacant(entry) = locals.entry(name.clone()) {
                                let value = if let Some((ref params, _)) = local_ast.bind.params {
                                    self.analyze_function(
                                        params,
                                        &local_ast.bind.value,
                                        &inner_env,
                                    )?
                                } else {
                                    self.analyze_expr(&local_ast.bind.value, &inner_env, false)?
                                };
                                entry.insert(value);
                            }
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

                Ok(Rc::new(ir::Expr::Object {
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

                let mut locals_spans = HashMap::new();
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

                let mut locals = HashMap::new();
                for local_ast in locals1_ast.iter().chain(locals2_ast.iter()) {
                    let name = &local_ast.bind.name.value;
                    if let HashMapEntry::Vacant(entry) = locals.entry(name.clone()) {
                        let value = if let Some((ref params, _)) = local_ast.bind.params {
                            self.analyze_function(params, &local_ast.bind.value, &inner_env)?
                        } else {
                            self.analyze_expr(&local_ast.bind.value, &inner_env, false)?
                        };
                        entry.insert(value);
                    }
                }

                let field_name = self.analyze_expr(name_ast, &env, false)?;
                let field_value = self.analyze_expr(body_ast, &inner_env, false)?;

                Ok(Rc::new(ir::Expr::ObjectComp {
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
    ) -> Result<Rc<ir::Expr>, AnalyzeError> {
        let mut inner_env = env.clone();

        let mut params_spans = HashMap::new();
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

        let mut params = ir::FuncParams {
            by_name: HashMap::new(),
            order: Vec::new(),
        };
        for (param_i, param_ast) in params_ast.iter().enumerate() {
            let name = &param_ast.name.value;
            if let HashMapEntry::Vacant(entry) = params.by_name.entry(name.clone()) {
                let default_value = param_ast
                    .default_value
                    .as_ref()
                    .map(|e| self.analyze_expr(e, &inner_env, false))
                    .transpose()?;
                entry.insert((param_i, default_value));
                params.order.push(name.clone());
            }
        }

        let body = self.analyze_expr(body_ast, &inner_env, true)?;

        Ok(Rc::new(ir::Expr::Func {
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
    pub(super) vars: HashSet<InternedStr>,
}
