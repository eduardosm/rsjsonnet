use std::cell::{Cell, OnceCell};

use super::super::{
    ir, FuncData, FuncKind, ImportError, ObjectData, ObjectLayer, ThunkEnv, ThunkEnvData, ValueData,
};
use super::{EvalErrorKind, EvalErrorValueType, EvalResult, Evaluator, State, TraceItem};
use crate::gc::{Gc, GcView};
use crate::interner::InternedStr;
use crate::span::SpanId;
use crate::{ast, FHashMap};

impl<'p> Evaluator<'_, 'p> {
    pub(super) fn do_expr(
        &mut self,
        expr: &'p ir::Expr<'p>,
        env: GcView<ThunkEnv<'p>>,
    ) -> EvalResult<()> {
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
            ir::Expr::String(s) => {
                self.value_stack.push(ValueData::String(s.into()));
            }
            ir::Expr::Object {
                is_top,
                locals: ir_locals,
                asserts: ir_asserts,
                fields: ir_fields,
            } => {
                self.object_stack.push(ObjectData {
                    self_layer: ObjectLayer {
                        is_top,
                        locals: ir_locals,
                        base_env: Some(Gc::from(&env)),
                        env: OnceCell::new(),
                        fields: FHashMap::default(),
                        asserts: ir_asserts,
                    },
                    super_layers: Vec::new(),
                    fields_order: OnceCell::new(),
                    asserts_checked: Cell::new(false),
                });

                self.state_stack.push(State::FinishObject);

                for field in ir_fields.iter().rev() {
                    match field.name {
                        ir::FieldName::Fix(name) => {
                            self.state_stack.push(State::ObjectFixField {
                                name,
                                name_span: field.name_span,
                                plus: field.plus,
                                visibility: field.visibility,
                                value: field.value,
                                base_env: None,
                            });
                        }
                        ir::FieldName::Dyn(name_expr) => {
                            self.state_stack.push(State::ObjectDynField {
                                name_span: field.name_span,
                                plus: field.plus,
                                visibility: field.visibility,
                                value: field.value,
                                base_env: None,
                            });
                            self.state_stack.push(State::Expr {
                                expr: name_expr,
                                env: env.clone(),
                            });
                        }
                    }
                }
            }
            ir::Expr::ObjectComp { comp_spec, .. } => {
                self.state_stack.push(State::ObjectComp {
                    expr,
                    env: env.clone(),
                });
                self.want_comp_spec(comp_spec, env);
            }
            ir::Expr::Array(items_exprs) => {
                if items_exprs.is_empty() {
                    self.value_stack
                        .push(ValueData::Array(Gc::from(&self.program.empty_array)));
                } else {
                    let items = items_exprs
                        .iter()
                        .map(|item| {
                            self.program
                                .new_pending_expr_thunk(item, Gc::from(&env), None)
                        })
                        .collect();
                    self.value_stack
                        .push(ValueData::Array(self.program.gc_alloc(items)));
                }
            }
            ir::Expr::ArrayComp { value, comp_spec } => {
                self.state_stack.push(State::ArrayComp {
                    item: value,
                    env: env.clone(),
                });
                self.want_comp_spec(comp_spec, env);
            }
            ir::Expr::Field {
                object,
                field_name,
                expr_span,
            } => {
                self.state_stack.push(State::Field {
                    span: expr_span,
                    field_name,
                });
                self.state_stack.push(State::Expr { expr: object, env });
            }
            ir::Expr::Index {
                object,
                index,
                expr_span,
            } => {
                self.state_stack.push(State::Index { span: expr_span });
                self.state_stack.push(State::Expr {
                    expr: index,
                    env: env.clone(),
                });
                self.state_stack.push(State::Expr { expr: object, env });
            }
            ir::Expr::Slice {
                array,
                start_index,
                end_index,
                step,
                expr_span,
            } => {
                self.state_stack.push(State::Slice {
                    span: expr_span,
                    has_start: start_index.is_some(),
                    has_end: end_index.is_some(),
                    has_step: step.is_some(),
                });
                if let Some(step) = step {
                    self.state_stack.push(State::Expr {
                        expr: step,
                        env: env.clone(),
                    });
                }
                if let Some(end_index) = end_index {
                    self.state_stack.push(State::Expr {
                        expr: end_index,
                        env: env.clone(),
                    });
                }
                if let Some(start_index) = start_index {
                    self.state_stack.push(State::Expr {
                        expr: start_index,
                        env: env.clone(),
                    });
                }
                self.state_stack.push(State::Expr { expr: array, env });
            }
            ir::Expr::SuperField {
                super_span,
                field_name,
                expr_span,
            } => {
                self.want_super_field(&env, super_span, field_name, expr_span)?;
            }
            ir::Expr::SuperIndex {
                super_span,
                index,
                expr_span,
            } => {
                self.state_stack.push(State::SuperIndex {
                    span: expr_span,
                    super_span,
                    env: env.clone(),
                });
                self.state_stack.push(State::Expr { expr: index, env });
            }
            ir::Expr::Call { callee, .. } => {
                self.state_stack.push(State::CallWithExpr {
                    call_expr: expr,
                    call_env: env.clone(),
                });
                self.state_stack.push(State::Expr {
                    expr: callee,
                    env: env.clone(),
                });
            }
            ir::Expr::Var(var_name, span) => {
                let thunk = env.get_var(var_name).view();
                self.want_thunk_direct(thunk, || TraceItem::Variable {
                    span,
                    name: var_name,
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
            ir::Expr::Local { bindings, inner } => {
                let new_env = self.program.gc_alloc_view(ThunkEnv::new());
                let mut new_env_data = ThunkEnvData::new(Some(Gc::from(&env)));
                for &(var_name, value_expr) in bindings.iter() {
                    let var_thunk = self.program.new_pending_expr_thunk(
                        value_expr,
                        Gc::from(&new_env),
                        Some(var_name),
                    );
                    new_env_data.set_var(var_name, var_thunk);
                }
                new_env.set_data(new_env_data);
                self.state_stack.push(State::Expr {
                    expr: inner,
                    env: new_env,
                });
            }
            ir::Expr::If {
                cond,
                cond_span,
                then_body,
                else_body,
            } => {
                self.state_stack.push(State::If {
                    cond_span,
                    then_body,
                    else_body,
                    env: env.clone(),
                });
                self.state_stack.push(State::Expr { expr: cond, env });
            }
            ir::Expr::Binary { op, lhs, rhs, span } => {
                match op {
                    ast::BinaryOp::Lt => {
                        self.push_trace_item(TraceItem::Expr { span });
                        self.state_stack.push(State::CmpOrdToBoolValueIsLt);
                        self.state_stack.push(State::CompareValue);
                        self.state_stack.push(State::Expr {
                            expr: rhs,
                            env: env.clone(),
                        });
                        self.state_stack.push(State::Expr {
                            expr: lhs,
                            env: env.clone(),
                        });
                    }
                    ast::BinaryOp::Le => {
                        self.push_trace_item(TraceItem::Expr { span });
                        self.state_stack.push(State::CmpOrdToBoolValueIsLe);
                        self.state_stack.push(State::CompareValue);
                        self.state_stack.push(State::Expr {
                            expr: rhs,
                            env: env.clone(),
                        });
                        self.state_stack.push(State::Expr {
                            expr: lhs,
                            env: env.clone(),
                        });
                    }
                    ast::BinaryOp::Gt => {
                        self.push_trace_item(TraceItem::Expr { span });
                        self.state_stack.push(State::CmpOrdToBoolValueIsGt);
                        self.state_stack.push(State::CompareValue);
                        self.state_stack.push(State::Expr {
                            expr: rhs,
                            env: env.clone(),
                        });
                        self.state_stack.push(State::Expr {
                            expr: lhs,
                            env: env.clone(),
                        });
                    }
                    ast::BinaryOp::Ge => {
                        self.push_trace_item(TraceItem::Expr { span });
                        self.state_stack.push(State::CmpOrdToBoolValueIsGe);
                        self.state_stack.push(State::CompareValue);
                        self.state_stack.push(State::Expr {
                            expr: rhs,
                            env: env.clone(),
                        });
                        self.state_stack.push(State::Expr {
                            expr: lhs,
                            env: env.clone(),
                        });
                    }
                    ast::BinaryOp::Eq => {
                        self.push_trace_item(TraceItem::Expr { span });
                        self.state_stack.push(State::BoolToValue);
                        self.state_stack.push(State::EqualsValue);
                        self.state_stack.push(State::Expr {
                            expr: rhs,
                            env: env.clone(),
                        });
                        self.state_stack.push(State::Expr {
                            expr: lhs,
                            env: env.clone(),
                        });
                    }
                    ast::BinaryOp::Ne => {
                        self.push_trace_item(TraceItem::Expr { span });
                        self.state_stack.push(State::BoolToValue);
                        self.state_stack.push(State::InvertBool);
                        self.state_stack.push(State::EqualsValue);
                        self.state_stack.push(State::Expr {
                            expr: rhs,
                            env: env.clone(),
                        });
                        self.state_stack.push(State::Expr {
                            expr: lhs,
                            env: env.clone(),
                        });
                    }
                    // Handle short-circuit for `&&` and `||`.
                    ast::BinaryOp::LogicAnd => {
                        self.state_stack.push(State::LogicAnd {
                            span,
                            rhs,
                            env: env.clone(),
                        });
                        self.state_stack.push(State::Expr { expr: lhs, env });
                    }
                    ast::BinaryOp::LogicOr => {
                        self.state_stack.push(State::LogicOr {
                            span,
                            rhs,
                            env: env.clone(),
                        });
                        self.state_stack.push(State::Expr { expr: lhs, env });
                    }
                    _ => {
                        self.state_stack.push(State::BinaryOp {
                            span: Some(span),
                            op,
                        });
                        self.state_stack.push(State::Expr {
                            expr: rhs,
                            env: env.clone(),
                        });
                        self.state_stack.push(State::Expr {
                            expr: lhs,
                            env: env.clone(),
                        });
                    }
                }
            }
            ir::Expr::Unary { op, rhs, span } => {
                self.state_stack.push(State::UnaryOp { span, op });
                self.state_stack.push(State::Expr {
                    expr: rhs,
                    env: env.clone(),
                });
            }
            ir::Expr::InSuper { lhs, span } => {
                self.state_stack.push(State::InSuper {
                    span,
                    env: env.clone(),
                });
                self.state_stack.push(State::Expr { expr: lhs, env });
            }
            ir::Expr::IdentityFunc => {
                self.value_stack
                    .push(ValueData::Function(Gc::from(&self.program.identity_func)));
            }
            ir::Expr::Func { params, body } => {
                self.value_stack
                    .push(ValueData::Function(self.program.gc_alloc(FuncData::new(
                        params,
                        FuncKind::Normal {
                            name: None,
                            body,
                            env: Gc::from(&env),
                        },
                    ))));
            }
            ir::Expr::Error { msg, span } => {
                self.state_stack.push(State::Error { span });
                self.push_trace_item(TraceItem::Expr { span });
                self.state_stack.push(State::CoerceToString);
                self.delay_trace_item();
                self.state_stack.push(State::Expr { expr: msg, env });
            }
            ir::Expr::Assert { ref assert, inner } => {
                self.state_stack.push(State::Expr {
                    expr: inner,
                    env: env.clone(),
                });
                self.state_stack.push(State::Assert {
                    assert_span: assert.span,
                    cond_span: assert.cond_span,
                    msg_expr: assert.msg.map(|e| (e, env.clone())),
                });
                self.state_stack.push(State::Expr {
                    expr: assert.cond,
                    env,
                });
            }
            ir::Expr::Import { path, span } => {
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
                            path: path.into(),
                        }));
                    }
                }
            }
            ir::Expr::ImportStr { path, span } => {
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
                            path: path.into(),
                        }));
                    }
                }
            }
            ir::Expr::ImportBin { path, span } => {
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
                            path: path.into(),
                        }));
                    }
                }
            }
        }

        Ok(())
    }

    fn want_comp_spec(&mut self, comp_spec: &[ir::CompSpecPart<'p>], env: GcView<ThunkEnv<'p>>) {
        for comp_spec_part in comp_spec[1..].iter().rev() {
            match *comp_spec_part {
                ir::CompSpecPart::For {
                    var,
                    value,
                    value_span,
                } => {
                    self.state_stack.push(State::ForSpec {
                        var_name: var,
                        value,
                        value_span,
                        env: env.clone(),
                    });
                }
                ir::CompSpecPart::If { cond, cond_span } => {
                    self.state_stack.push(State::IfSpec {
                        cond,
                        cond_span,
                        env: env.clone(),
                    });
                }
            }
        }
        match comp_spec[0] {
            ir::CompSpecPart::For {
                var,
                value,
                value_span,
            } => {
                self.state_stack.push(State::InitCompSpec {
                    var_name: var,
                    value,
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
        object: &GcView<ObjectData<'p>>,
        field_name: InternedStr<'p>,
        expr_span: SpanId,
    ) -> EvalResult<()> {
        if let Some(field_thunk) = self.program.find_object_field_thunk(object, 0, field_name) {
            if object.asserts_checked.get() {
                self.want_thunk_direct(field_thunk, || TraceItem::ObjectField {
                    span: Some(expr_span),
                    name: field_name,
                });
            } else {
                self.push_trace_item(TraceItem::ObjectField {
                    span: Some(expr_span),
                    name: field_name,
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
        env: &ThunkEnv<'p>,
        super_span: SpanId,
        field_name: InternedStr<'p>,
        expr_span: SpanId,
    ) -> EvalResult<()> {
        let (object, layer_i) = env.get_object();
        let object = object.view();
        if layer_i == object.super_layers.len() {
            return Err(
                self.report_error(EvalErrorKind::SuperWithoutSuperObject { span: super_span })
            );
        }
        if let Some(field_thunk) =
            self.program
                .find_object_field_thunk(&object, layer_i + 1, field_name)
        {
            self.want_thunk_direct(field_thunk, || TraceItem::ObjectField {
                span: Some(expr_span),
                name: field_name,
            });
            Ok(())
        } else {
            Err(self.report_error(EvalErrorKind::UnknownObjectField {
                span: expr_span,
                field_name: field_name.value().into(),
            }))
        }
    }

    pub(super) fn do_slice(
        &mut self,
        indexable: ValueData<'p>,
        start: Option<f64>,
        end: Option<f64>,
        step: Option<f64>,
        is_func: bool,
        span: Option<SpanId>,
    ) -> EvalResult<()> {
        let start = if let Some(start) = start {
            if !start.is_finite() || start.trunc() != start || start < 0.0 {
                return Err(self.report_error(EvalErrorKind::Other {
                    span,
                    message: format!("slice start {start} is not a non-negative integer"),
                }));
            }
            start as usize
        } else {
            0
        };
        let end = if let Some(end) = end {
            if !end.is_finite() || end.trunc() != end || end < 0.0 {
                return Err(self.report_error(EvalErrorKind::Other {
                    span,
                    message: format!("slice end {end} is not a non-negative integer"),
                }));
            }
            (end as usize).max(start)
        } else {
            usize::MAX
        };
        let step = if let Some(step) = step {
            if !step.is_finite() || step.trunc() != step || step < 1.0 {
                return Err(self.report_error(EvalErrorKind::Other {
                    span,
                    message: format!("slice step {step} is not a positive integer"),
                }));
            }
            step as usize
        } else {
            1
        };

        match indexable {
            ValueData::String(s) => {
                let r: String = s
                    .chars()
                    .skip(start)
                    .take(end - start)
                    .step_by(step)
                    .collect();
                self.value_stack.push(ValueData::String(r.into()));
                Ok(())
            }
            ValueData::Array(array) => {
                let array = array.view();
                let result = self.program.slice_array(&array, start, end, step);
                self.value_stack.push(ValueData::Array(result));
                Ok(())
            }
            _ => {
                if is_func {
                    Err(self.report_error(EvalErrorKind::InvalidStdFuncArgType {
                        func_name: "slice".into(),
                        arg_index: 0,
                        expected_types: vec![EvalErrorValueType::String, EvalErrorValueType::Array],
                        got_type: EvalErrorValueType::from_value(&indexable),
                    }))
                } else {
                    Err(self.report_error(EvalErrorKind::InvalidSlicedType {
                        span: span.unwrap(),
                        got_type: EvalErrorValueType::from_value(&indexable),
                    }))
                }
            }
        }
    }

    pub(super) fn do_binary_op(
        &mut self,
        span: Option<SpanId>,
        op: ast::BinaryOp,
    ) -> EvalResult<()> {
        let rhs = self.value_stack.pop().unwrap();
        let lhs = self.value_stack.pop().unwrap();
        match (op, lhs, rhs) {
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
            (ast::BinaryOp::Rem, ValueData::Number(lhs), ValueData::Number(rhs)) => {
                if rhs == 0.0 {
                    return Err(self.report_error(EvalErrorKind::DivByZero { span }));
                }
                let r = lhs % rhs;
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
            (ast::BinaryOp::Rem, lhs @ ValueData::String(_), rhs) => {
                self.value_stack.push(lhs);
                self.value_stack.push(rhs);
                if let Some(span) = span {
                    self.push_trace_item(TraceItem::Expr { span });
                }
                self.state_stack.push(State::StdFormat);
            }
            (ast::BinaryOp::In, ValueData::String(lhs), ValueData::Object(rhs)) => {
                let field_name = self.program.intern_str(&lhs);
                let object = rhs.view();

                self.value_stack
                    .push(ValueData::Bool(object.has_field(0, field_name)));
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
