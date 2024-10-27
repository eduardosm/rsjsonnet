use std::cell::{Cell, OnceCell};

use super::super::{
    ir, FuncData, FuncKind, ImportError, ObjectAssert, ObjectCore, ObjectData, ThunkEnv,
    ThunkEnvData, ValueData,
};
use super::{EvalError, EvalErrorKind, EvalErrorValueType, Evaluator, State, TraceItem};
use crate::gc::{Gc, GcView};
use crate::interner::InternedStr;
use crate::span::SpanId;
use crate::{ast, FHashMap};

impl Evaluator<'_> {
    pub(super) fn do_expr(
        &mut self,
        expr: ir::RcExpr,
        env: GcView<ThunkEnv>,
    ) -> Result<(), Box<EvalError>> {
        match *expr {
            ir::Expr::Null => {
                self.value_stack.push(ValueData::Null);
            }
            ir::Expr::Bool(value) => {
                self.value_stack.push(ValueData::Bool(value));
            }
            ir::Expr::Number(value, span) => {
                self.check_number_value(value, Some(span))?;
                self.value_stack.push(ValueData::Number(value));
            }
            ir::Expr::String(ref s) => {
                self.value_stack.push(ValueData::String(s.clone()));
            }
            ir::Expr::Object {
                is_top,
                locals: ref ir_locals,
                asserts: ref ir_asserts,
                fields: ref ir_fields,
            } => {
                let mut asserts = Vec::with_capacity(ir_asserts.len());
                for assert in ir_asserts.iter().rev() {
                    asserts.push(ObjectAssert {
                        cond: assert.cond.clone(),
                        cond_span: assert.cond_span,
                        msg: assert.msg.clone(),
                        assert_span: assert.span,
                    });
                }

                self.object_stack.push(ObjectData {
                    self_core: ObjectCore {
                        is_top,
                        locals: ir_locals.clone(),
                        base_env: Some(Gc::from(&env)),
                        env: OnceCell::new(),
                        fields: FHashMap::default(),
                        asserts,
                    },
                    super_cores: Vec::new(),
                    fields_order: OnceCell::new(),
                    asserts_checked: Cell::new(false),
                });

                self.state_stack.push(State::FinishObject);

                for field in ir_fields.iter().rev() {
                    match field.name {
                        ir::FieldName::Fix(ref name) => {
                            self.state_stack.push(State::ObjectFixField {
                                name: name.clone(),
                                name_span: field.name_span,
                                plus: field.plus,
                                visibility: field.visibility,
                                value: field.value.clone(),
                                base_env: None,
                            });
                        }
                        ir::FieldName::Dyn(ref name_expr) => {
                            self.state_stack.push(State::ObjectDynField {
                                name_span: field.name_span,
                                plus: field.plus,
                                visibility: field.visibility,
                                value: field.value.clone(),
                                base_env: None,
                            });
                            self.state_stack.push(State::Expr {
                                expr: name_expr.clone(),
                                env: env.clone(),
                            });
                        }
                    }
                }
            }
            ir::Expr::ObjectComp { ref comp_spec, .. } => {
                self.state_stack.push(State::ObjectComp {
                    expr: expr.clone(),
                    env: env.clone(),
                });
                self.want_comp_spec(comp_spec, env);
            }
            ir::Expr::Array(ref items_exprs) => {
                if items_exprs.is_empty() {
                    self.value_stack
                        .push(ValueData::Array(Gc::from(&self.program.empty_array)));
                } else {
                    let items = items_exprs
                        .iter()
                        .map(|item| self.new_pending_expr_thunk(item.clone(), Gc::from(&env)))
                        .collect();
                    self.value_stack
                        .push(ValueData::Array(self.program.gc_alloc(items)));
                }
            }
            ir::Expr::ArrayComp {
                ref value,
                ref comp_spec,
            } => {
                self.state_stack.push(State::ArrayComp {
                    item: value.clone(),
                    env: env.clone(),
                });
                self.want_comp_spec(comp_spec, env);
            }
            ir::Expr::Field {
                ref object,
                ref field_name,
                expr_span,
            } => {
                self.state_stack.push(State::Field {
                    span: expr_span,
                    field_name: field_name.clone(),
                });
                self.state_stack.push(State::Expr {
                    expr: object.clone(),
                    env,
                });
            }
            ir::Expr::Index {
                ref object,
                ref index,
                expr_span,
            } => {
                self.state_stack.push(State::Index { span: expr_span });
                self.state_stack.push(State::Expr {
                    expr: index.clone(),
                    env: env.clone(),
                });
                self.state_stack.push(State::Expr {
                    expr: object.clone(),
                    env,
                });
            }
            ir::Expr::SuperField {
                super_span,
                ref field_name,
                expr_span,
            } => {
                self.want_super_field(&env, super_span, field_name, expr_span)?;
            }
            ir::Expr::SuperIndex {
                super_span,
                ref index,
                expr_span,
            } => {
                self.state_stack.push(State::SuperIndex {
                    span: expr_span,
                    super_span,
                    env: env.clone(),
                });
                self.state_stack.push(State::Expr {
                    expr: index.clone(),
                    env,
                });
            }
            ir::Expr::StdField { ref field_name } => {
                let field = self.get_stdlib_field(field_name);
                self.state_stack.push(State::DoThunk(field));
            }
            ir::Expr::Call { ref callee, .. } => {
                let callee = callee.clone();
                self.state_stack.push(State::CallWithExpr {
                    call_expr: expr,
                    call_env: env.clone(),
                });
                self.state_stack.push(State::Expr {
                    expr: callee,
                    env: env.clone(),
                });
            }
            ir::Expr::Var(ref var_name, span) => {
                let thunk = env.get_var(var_name).view();
                self.want_thunk_direct(thunk, || TraceItem::Variable {
                    span,
                    name: var_name.clone(),
                });
            }
            ir::Expr::TopObj => {
                let object = env.get_top_object();
                self.value_stack.push(ValueData::Object(object));
            }
            ir::Expr::SelfObj => {
                let (object, _) = env.get_object();
                self.value_stack.push(ValueData::Object(object));
            }
            ir::Expr::Local {
                ref bindings,
                ref inner,
            } => {
                let new_env = self.program.gc_alloc_view(ThunkEnv::new());
                let mut new_env_data = ThunkEnvData::new(Some(Gc::from(&env)));
                for (var_name, value_expr) in bindings.iter() {
                    let var_thunk =
                        self.new_pending_expr_thunk(value_expr.clone(), Gc::from(&new_env));
                    new_env_data.set_var(var_name.clone(), var_thunk);
                }
                new_env.set_data(new_env_data);
                self.state_stack.push(State::Expr {
                    expr: inner.clone(),
                    env: new_env,
                });
            }
            ir::Expr::If {
                ref cond,
                cond_span,
                ref then_body,
                ref else_body,
            } => {
                self.state_stack.push(State::If {
                    cond_span,
                    then_body: then_body.clone(),
                    else_body: else_body.clone(),
                    env: env.clone(),
                });
                self.state_stack.push(State::Expr {
                    expr: cond.clone(),
                    env,
                });
            }
            ir::Expr::Binary {
                op,
                ref lhs,
                ref rhs,
                span,
            } => {
                match op {
                    ast::BinaryOp::Lt => {
                        self.push_trace_item(TraceItem::Expr { span });
                        self.state_stack.push(State::CmpOrdToBoolValueIsLt);
                        self.state_stack.push(State::CompareValue);
                        self.state_stack.push(State::Expr {
                            expr: rhs.clone(),
                            env: env.clone(),
                        });
                        self.state_stack.push(State::Expr {
                            expr: lhs.clone(),
                            env: env.clone(),
                        });
                    }
                    ast::BinaryOp::Le => {
                        self.push_trace_item(TraceItem::Expr { span });
                        self.state_stack.push(State::CmpOrdToBoolValueIsLe);
                        self.state_stack.push(State::CompareValue);
                        self.state_stack.push(State::Expr {
                            expr: rhs.clone(),
                            env: env.clone(),
                        });
                        self.state_stack.push(State::Expr {
                            expr: lhs.clone(),
                            env: env.clone(),
                        });
                    }
                    ast::BinaryOp::Gt => {
                        self.push_trace_item(TraceItem::Expr { span });
                        self.state_stack.push(State::CmpOrdToBoolValueIsGt);
                        self.state_stack.push(State::CompareValue);
                        self.state_stack.push(State::Expr {
                            expr: rhs.clone(),
                            env: env.clone(),
                        });
                        self.state_stack.push(State::Expr {
                            expr: lhs.clone(),
                            env: env.clone(),
                        });
                    }
                    ast::BinaryOp::Ge => {
                        self.push_trace_item(TraceItem::Expr { span });
                        self.state_stack.push(State::CmpOrdToBoolValueIsGe);
                        self.state_stack.push(State::CompareValue);
                        self.state_stack.push(State::Expr {
                            expr: rhs.clone(),
                            env: env.clone(),
                        });
                        self.state_stack.push(State::Expr {
                            expr: lhs.clone(),
                            env: env.clone(),
                        });
                    }
                    ast::BinaryOp::Eq => {
                        self.push_trace_item(TraceItem::Expr { span });
                        self.state_stack.push(State::BoolToValue);
                        self.state_stack.push(State::EqualsValue);
                        self.state_stack.push(State::Expr {
                            expr: rhs.clone(),
                            env: env.clone(),
                        });
                        self.state_stack.push(State::Expr {
                            expr: lhs.clone(),
                            env: env.clone(),
                        });
                    }
                    ast::BinaryOp::Ne => {
                        self.push_trace_item(TraceItem::Expr { span });
                        self.state_stack.push(State::BoolToValue);
                        self.state_stack.push(State::InvertBool);
                        self.state_stack.push(State::EqualsValue);
                        self.state_stack.push(State::Expr {
                            expr: rhs.clone(),
                            env: env.clone(),
                        });
                        self.state_stack.push(State::Expr {
                            expr: lhs.clone(),
                            env: env.clone(),
                        });
                    }
                    // Handle short-circuit for `&&` and `||`.
                    ast::BinaryOp::LogicAnd => {
                        self.state_stack.push(State::LogicAnd {
                            span,
                            rhs: rhs.clone(),
                            env: env.clone(),
                        });
                        self.state_stack.push(State::Expr {
                            expr: lhs.clone(),
                            env,
                        });
                    }
                    ast::BinaryOp::LogicOr => {
                        self.state_stack.push(State::LogicOr {
                            span,
                            rhs: rhs.clone(),
                            env: env.clone(),
                        });
                        self.state_stack.push(State::Expr {
                            expr: lhs.clone(),
                            env,
                        });
                    }
                    _ => {
                        self.state_stack.push(State::BinaryOp {
                            span: Some(span),
                            op,
                        });
                        self.state_stack.push(State::Expr {
                            expr: rhs.clone(),
                            env: env.clone(),
                        });
                        self.state_stack.push(State::Expr {
                            expr: lhs.clone(),
                            env: env.clone(),
                        });
                    }
                }
            }
            ir::Expr::Unary { op, ref rhs, span } => {
                self.state_stack.push(State::UnaryOp { span, op });
                self.state_stack.push(State::Expr {
                    expr: rhs.clone(),
                    env: env.clone(),
                });
            }
            ir::Expr::InSuper { ref lhs, span } => {
                self.state_stack.push(State::InSuper {
                    span,
                    env: env.clone(),
                });
                self.state_stack.push(State::Expr {
                    expr: lhs.clone(),
                    env,
                });
            }
            ir::Expr::IdentityFunc => {
                self.value_stack
                    .push(ValueData::Function(Gc::from(&self.program.identity_func)));
            }
            ir::Expr::Func {
                ref params,
                ref body,
            } => {
                self.value_stack
                    .push(ValueData::Function(self.program.gc_alloc(FuncData::new(
                        params.clone(),
                        FuncKind::Normal {
                            name: None, // TODO: Function name
                            body: body.clone(),
                            env: Gc::from(&env),
                        },
                    ))));
            }
            ir::Expr::Error { ref msg, span } => {
                self.state_stack.push(State::Error { span });
                self.push_trace_item(TraceItem::Expr { span });
                self.state_stack.push(State::CoerceToString);
                self.delay_trace_item();
                self.state_stack.push(State::Expr {
                    expr: msg.clone(),
                    env,
                });
            }
            ir::Expr::Assert {
                ref assert,
                ref inner,
            } => {
                self.state_stack.push(State::Expr {
                    expr: inner.clone(),
                    env: env.clone(),
                });
                self.state_stack.push(State::Assert {
                    assert_span: assert.span,
                    cond_span: assert.cond_span,
                    msg_expr: assert.msg.as_ref().map(|e| (e.clone(), env.clone())),
                });
                self.state_stack.push(State::Expr {
                    expr: assert.cond.clone(),
                    env,
                });
            }
            ir::Expr::Import { ref path, span } => {
                let callbacks = self
                    .callbacks
                    .as_deref_mut()
                    .expect("unexpected `import` without callbacks");
                match callbacks.import(self.program, span, path) {
                    Ok(thunk) => {
                        self.push_trace_item(TraceItem::Import { span });
                        self.state_stack.push(State::DoThunk(thunk.data));
                    }
                    Err(ImportError) => {
                        return Err(self.report_error(EvalErrorKind::ImportFailed {
                            span,
                            path: path.clone(),
                        }));
                    }
                }
            }
            ir::Expr::ImportStr { ref path, span } => {
                let callbacks = self
                    .callbacks
                    .as_deref_mut()
                    .expect("unexpected `importstr` without callbacks");
                match callbacks.import_str(self.program, span, path) {
                    Ok(s) => {
                        self.value_stack.push(ValueData::String(s.into()));
                    }
                    Err(ImportError) => {
                        return Err(self.report_error(EvalErrorKind::ImportFailed {
                            span,
                            path: path.clone(),
                        }));
                    }
                }
            }
            ir::Expr::ImportBin { ref path, span } => {
                let callbacks = self
                    .callbacks
                    .as_deref_mut()
                    .expect("unexpected `importbin` without callbacks");
                match callbacks.import_bin(self.program, span, path) {
                    Ok(data) => {
                        let array = self.program.make_value_array(
                            data.iter().map(|&byte| ValueData::Number(f64::from(byte))),
                        );
                        self.value_stack.push(ValueData::Array(array));
                    }
                    Err(ImportError) => {
                        return Err(self.report_error(EvalErrorKind::ImportFailed {
                            span,
                            path: path.clone(),
                        }));
                    }
                }
            }
        }

        Ok(())
    }

    fn want_comp_spec(&mut self, comp_spec: &[ir::CompSpecPart], env: GcView<ThunkEnv>) {
        for comp_spec_part in comp_spec[1..].iter().rev() {
            match *comp_spec_part {
                ir::CompSpecPart::For {
                    ref var,
                    ref value,
                    value_span,
                } => {
                    self.state_stack.push(State::ForSpec {
                        var_name: var.clone(),
                        value: value.clone(),
                        value_span,
                        env: env.clone(),
                    });
                }
                ir::CompSpecPart::If {
                    ref cond,
                    cond_span,
                } => {
                    self.state_stack.push(State::IfSpec {
                        cond: cond.clone(),
                        cond_span,
                        env: env.clone(),
                    });
                }
            }
        }
        match comp_spec[0] {
            ir::CompSpecPart::For {
                ref var,
                ref value,
                value_span,
            } => {
                self.state_stack.push(State::InitCompSpec {
                    var_name: var.clone(),
                    value: value.clone(),
                    value_span,
                    env,
                });
            }
            ir::CompSpecPart::If { .. } => {
                unreachable!();
            }
        }
    }

    pub(super) fn want_field(
        &mut self,
        object: &GcView<ObjectData>,
        field_name: &InternedStr,
        expr_span: SpanId,
    ) -> Result<(), Box<EvalError>> {
        if let Some(field_thunk) = self.program.find_object_field_thunk(object, 0, field_name) {
            if object.asserts_checked.get() {
                self.want_thunk_direct(field_thunk, || TraceItem::ObjectField {
                    span: Some(expr_span),
                    name: field_name.clone(),
                });
            } else {
                self.push_trace_item(TraceItem::ObjectField {
                    span: Some(expr_span),
                    name: field_name.clone(),
                });
                self.state_stack.push(State::DoThunk(field_thunk));
                self.check_object_asserts(object);
            }
            Ok(())
        } else {
            Err(self.report_error(EvalErrorKind::UnknownObjectField {
                span: expr_span,
                field_name: field_name.value().into(),
            }))
        }
    }

    pub(super) fn want_super_field(
        &mut self,
        env: &ThunkEnv,
        super_span: SpanId,
        field_name: &InternedStr,
        expr_span: SpanId,
    ) -> Result<(), Box<EvalError>> {
        let (object, core_i) = env.get_object();
        let object = object.view();
        if core_i == object.super_cores.len() {
            return Err(
                self.report_error(EvalErrorKind::SuperWithoutSuperObject { span: super_span })
            );
        }
        if let Some(field_thunk) =
            self.program
                .find_object_field_thunk(&object, core_i + 1, field_name)
        {
            self.want_thunk_direct(field_thunk, || TraceItem::ObjectField {
                span: Some(expr_span),
                name: field_name.clone(),
            });
            Ok(())
        } else {
            Err(self.report_error(EvalErrorKind::UnknownObjectField {
                span: expr_span,
                field_name: field_name.value().into(),
            }))
        }
    }

    pub(super) fn do_binary_op(
        &mut self,
        span: Option<SpanId>,
        op: ast::BinaryOp,
    ) -> Result<(), Box<EvalError>> {
        let rhs = self.value_stack.pop().unwrap();
        let lhs = self.value_stack.pop().unwrap();
        match (op, lhs, rhs) {
            (ast::BinaryOp::Rem, _, _) => {
                unreachable!("'%' operator should have been desugared");
            }
            // Handled in a different path.
            (
                ast::BinaryOp::Lt
                | ast::BinaryOp::Le
                | ast::BinaryOp::Gt
                | ast::BinaryOp::Ge
                | ast::BinaryOp::Eq
                | ast::BinaryOp::Ne,
                _,
                _,
            ) => {
                unreachable!();
            }
            // Boolean
            (ast::BinaryOp::LogicAnd, ValueData::Bool(lhs), ValueData::Bool(rhs)) => {
                // Short-circuiting should have already been handled.
                assert!(lhs);
                self.value_stack.push(ValueData::Bool(rhs));
            }
            (ast::BinaryOp::LogicOr, ValueData::Bool(lhs), ValueData::Bool(rhs)) => {
                // Short-circuiting should have already been handled.
                assert!(!lhs);
                self.value_stack.push(ValueData::Bool(rhs));
            }
            // Integer
            (ast::BinaryOp::Add, ValueData::Number(lhs), ValueData::Number(rhs)) => {
                let r = lhs + rhs;
                self.check_number_value(r, span)?;
                self.value_stack.push(ValueData::Number(r));
            }
            (ast::BinaryOp::Sub, ValueData::Number(lhs), ValueData::Number(rhs)) => {
                let r = lhs - rhs;
                self.check_number_value(r, span)?;
                self.value_stack.push(ValueData::Number(r));
            }
            (ast::BinaryOp::Mul, ValueData::Number(lhs), ValueData::Number(rhs)) => {
                let r = lhs * rhs;
                self.check_number_value(r, span)?;
                self.value_stack.push(ValueData::Number(r));
            }
            (ast::BinaryOp::Div, ValueData::Number(lhs), ValueData::Number(rhs)) => {
                if rhs == 0.0 {
                    return Err(self.report_error(EvalErrorKind::DivByZero { span }));
                }
                let r = lhs / rhs;
                self.check_number_value(r, span)?;
                self.value_stack.push(ValueData::Number(r));
            }
            (ast::BinaryOp::Shl, ValueData::Number(lhs), ValueData::Number(rhs)) => {
                let lhs = lhs as i64;
                if rhs.is_sign_negative() {
                    return Err(self.report_error(EvalErrorKind::ShiftByNegative { span }));
                }
                let rhs = rhs as i64;
                let r = lhs << (rhs & 63);
                self.value_stack.push(ValueData::Number(r as f64));
            }
            (ast::BinaryOp::Shr, ValueData::Number(lhs), ValueData::Number(rhs)) => {
                let lhs = lhs as i64;
                if rhs.is_sign_negative() {
                    return Err(self.report_error(EvalErrorKind::ShiftByNegative { span }));
                }
                let rhs = rhs as i64;
                let r = lhs >> (rhs & 63);
                self.value_stack.push(ValueData::Number(r as f64));
            }
            (ast::BinaryOp::BitwiseAnd, ValueData::Number(lhs), ValueData::Number(rhs)) => {
                let lhs = lhs as i64;
                let rhs = rhs as i64;
                let r = lhs & rhs;
                self.value_stack.push(ValueData::Number(r as f64));
            }
            (ast::BinaryOp::BitwiseOr, ValueData::Number(lhs), ValueData::Number(rhs)) => {
                let lhs = lhs as i64;
                let rhs = rhs as i64;
                let r = lhs | rhs;
                self.value_stack.push(ValueData::Number(r as f64));
            }
            (ast::BinaryOp::BitwiseXor, ValueData::Number(lhs), ValueData::Number(rhs)) => {
                let lhs = lhs as i64;
                let rhs = rhs as i64;
                let r = lhs ^ rhs;
                self.value_stack.push(ValueData::Number(r as f64));
            }
            // String
            (ast::BinaryOp::Add, ValueData::String(lhs), ValueData::String(rhs)) => {
                if lhs.is_empty() {
                    self.value_stack.push(ValueData::String(rhs));
                } else if rhs.is_empty() {
                    self.value_stack.push(ValueData::String(lhs));
                } else {
                    let mut r = String::with_capacity(lhs.len() + rhs.len());
                    r.push_str(&lhs);
                    r.push_str(&rhs);
                    self.value_stack.push(ValueData::String(r.into()));
                }
            }
            // Array
            (ast::BinaryOp::Add, ValueData::Array(lhs), ValueData::Array(rhs)) => {
                let lhs = lhs.view();
                let rhs = rhs.view();
                let r = self.program.concat_arrays(&lhs, &rhs);
                self.value_stack.push(ValueData::Array(r));
            }
            // Object
            (ast::BinaryOp::Add, ValueData::Object(lhs), ValueData::Object(rhs)) => {
                let r = self.program.extend_object(&lhs.view(), &rhs.view());
                self.value_stack.push(ValueData::Object(r));
            }
            // Mixed
            (ast::BinaryOp::Add, lhs @ ValueData::String(_), rhs) => {
                self.value_stack.push(lhs);
                self.value_stack.push(rhs);
                self.state_stack.push(State::BinaryOp { span, op });
                if let Some(span) = span {
                    self.push_trace_item(TraceItem::Expr { span });
                }
                self.state_stack.push(State::CoerceToStringValue);
            }
            (ast::BinaryOp::Add, lhs, rhs @ ValueData::String(_)) => {
                self.value_stack.push(rhs);
                self.value_stack.push(lhs);
                self.state_stack.push(State::BinaryOp { span, op });
                self.state_stack.push(State::SwapLastValues);
                if let Some(span) = span {
                    self.push_trace_item(TraceItem::Expr { span });
                }
                self.state_stack.push(State::CoerceToStringValue);
            }
            (ast::BinaryOp::In, ValueData::String(lhs), ValueData::Object(rhs)) => {
                let field_name = self.program.intern_str(&lhs);
                let object = rhs.view();

                self.value_stack
                    .push(ValueData::Bool(object.has_field(0, &field_name)));
            }
            (op, lhs, rhs) => {
                return Err(self.report_error(EvalErrorKind::InvalidBinaryOpTypes {
                    span,
                    op,
                    lhs_type: EvalErrorValueType::from_value(&lhs),
                    rhs_type: EvalErrorValueType::from_value(&rhs),
                }));
            }
        }
        Ok(())
    }
}
