use std::cell::{Cell, OnceCell};
use std::collections::hash_map::Entry as HashMapEntry;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::rc::Rc;

use super::{
    ir, ArrayData, Callbacks, EvalError, EvalErrorKind, EvalErrorValueType, EvalStackTraceItem,
    FuncData, ObjectCore, ObjectData, ObjectField, PendingThunk, Program, ThunkData, ThunkEnv,
    ThunkEnvData, ThunkState, ValueData,
};
use crate::gc::{Gc, GcView};
use crate::interner::InternedStr;
use crate::span::SpanId;
use crate::{ast, float};

mod call;
mod expr;
mod format;
mod manifest;
mod parse_json;
mod parse_yaml;
mod state;

use manifest::{escape_string_json, ManifestJsonFormat};
use state::State;

pub(super) struct Evaluator<'a> {
    program: &'a mut Program,
    callbacks: Option<&'a mut dyn Callbacks>,
    stack_trace_len: usize,
    state_stack: Vec<State>,
    value_stack: Vec<ValueData>,
    bool_stack: Vec<bool>,
    string_stack: Vec<String>,
    array_stack: Vec<Vec<Gc<ThunkData>>>,
    object_stack: Vec<ObjectData>,
    comp_spec_stack: Vec<CompSpec>,
    cmp_ord_stack: Vec<std::cmp::Ordering>,
    byte_array_stack: Vec<Vec<u8>>,
    output: EvalOutput,
}

pub(super) enum EvalInput {
    Value(GcView<ThunkData>),
    Call(GcView<ThunkData>, TopLevelArgs),
    ManifestJson(GcView<ThunkData>, bool),
}

#[must_use]
pub(super) enum EvalOutput {
    Nothing,
    Value(ValueData),
    String(String),
}

pub(super) struct TopLevelArgs {
    pub(super) positional: Box<[GcView<ThunkData>]>,
    pub(super) named: Box<[(InternedStr, GcView<ThunkData>)]>,
}

#[derive(Clone)]
enum TraceItem {
    Expr {
        span: SpanId,
    },
    Call {
        span: Option<SpanId>,
        name: Option<InternedStr>,
    },
    Variable {
        span: SpanId,
        name: InternedStr,
    },
    ArrayItem {
        span: Option<SpanId>,
        index: usize,
    },
    ObjectField {
        span: Option<SpanId>,
        name: InternedStr,
    },
    Import {
        span: SpanId,
    },
}

struct CompSpec {
    vars: Vec<HashMap<InternedStr, GcView<ThunkData>>>,
}

impl<'a> Evaluator<'a> {
    pub(super) fn eval(
        program: &'a mut Program,
        callbacks: Option<&'a mut dyn Callbacks>,
        input: EvalInput,
    ) -> Result<EvalOutput, EvalError> {
        let mut this = Self {
            program,
            callbacks,
            stack_trace_len: 0,
            state_stack: Vec::new(),
            value_stack: Vec::new(),
            bool_stack: Vec::new(),
            string_stack: Vec::new(),
            array_stack: Vec::new(),
            object_stack: Vec::new(),
            comp_spec_stack: Vec::new(),
            cmp_ord_stack: Vec::new(),
            byte_array_stack: Vec::new(),
            output: EvalOutput::Nothing,
        };

        match input {
            EvalInput::Value(thunk) => {
                this.state_stack.push(State::DiscardValue);
                this.state_stack.push(State::DeepValue);
                this.state_stack.push(State::DoThunk(thunk));
            }
            EvalInput::Call(func, args) => {
                this.state_stack.push(State::OutputValue);
                this.state_stack.push(State::DeepValue);
                this.state_stack.push(State::TopLevelCall {
                    pos_args: args.positional,
                    named_args: args.named,
                });
                this.state_stack.push(State::DoThunk(func));
            }
            EvalInput::ManifestJson(thunk, multiline) => {
                this.state_stack.push(State::OutputString);
                this.state_stack.push(State::ManifestJson {
                    format: if multiline {
                        ManifestJsonFormat::default_manifest()
                    } else {
                        ManifestJsonFormat::default_to_string()
                    },
                    depth: 0,
                });
                this.state_stack.push(State::DoThunk(thunk));
                this.string_stack.push(String::new());
            }
        }

        this.run()?;

        assert_eq!(this.stack_trace_len, 0);
        assert!(this.state_stack.is_empty());
        assert!(this.value_stack.is_empty());
        assert!(this.bool_stack.is_empty());
        assert!(this.string_stack.is_empty());
        assert!(this.array_stack.is_empty());
        assert!(this.object_stack.is_empty());
        assert!(this.comp_spec_stack.is_empty());
        assert!(this.cmp_ord_stack.is_empty());
        assert!(this.byte_array_stack.is_empty());

        Ok(this.output)
    }

    fn run(&mut self) -> Result<(), EvalError> {
        while let Some(state) = self.state_stack.pop() {
            match state {
                State::TraceItem(_) => {
                    self.dec_trace_len();
                }
                State::DelayedTraceItem => {
                    self.inc_trace_len();
                }
                State::DiscardValue => {
                    self.value_stack.pop().unwrap();
                }
                State::DoThunk(thunk) => match thunk.switch_state() {
                    ThunkState::Done(value) => {
                        self.value_stack.push(value);
                    }
                    ThunkState::Pending(pending) => {
                        self.state_stack.push(State::GotThunk(thunk));
                        match pending {
                            PendingThunk::Expr { expr, env } => {
                                self.state_stack.push(State::Expr {
                                    expr,
                                    env: env.view(),
                                });
                            }
                            PendingThunk::Call { func, args } => {
                                self.execute_call(&func.view(), args);
                            }
                        }
                    }
                    ThunkState::InProgress => {
                        return Err(self.report_error(EvalErrorKind::InfiniteRecursion));
                    }
                },
                State::GotThunk(thunk) => {
                    let value = self.value_stack.last().unwrap();
                    thunk.set_done(value.clone());
                }
                State::DeepValue => {
                    #[inline]
                    fn might_need_deep(thunk: &ThunkData) -> bool {
                        if let ThunkState::Done(ref value) = *thunk.state() {
                            value.might_need_deep()
                        } else {
                            true
                        }
                    }

                    let value = self.value_stack.last().unwrap();
                    if let ValueData::Array(ref array) = value {
                        let array = array.view();
                        for (i, item) in array.iter().enumerate().rev() {
                            let item = item.view();
                            if might_need_deep(&item) {
                                self.push_trace_item(TraceItem::ArrayItem {
                                    span: None,
                                    index: i,
                                });
                                self.state_stack.push(State::DiscardValue);
                                self.state_stack.push(State::DeepValue);
                                self.state_stack.push(State::DoThunk(item));
                                self.delay_trace_item();
                            }
                        }
                    } else if let ValueData::Object(ref object) = value {
                        let object = object.view();
                        for field_name in object.get_fields_order().iter().rev() {
                            if object.field_is_visible(field_name) {
                                let thunk = self
                                    .program
                                    .find_object_field_thunk(&object, 0, field_name)
                                    .unwrap();
                                if might_need_deep(&thunk) {
                                    self.push_trace_item(TraceItem::ObjectField {
                                        span: None,
                                        name: field_name.clone(),
                                    });
                                    self.state_stack.push(State::DiscardValue);
                                    self.state_stack.push(State::DeepValue);
                                    self.state_stack.push(State::DoThunk(thunk));
                                    self.delay_trace_item();
                                }
                            }
                        }
                        self.check_object_asserts(&object);
                    }
                }
                State::SwapLastValues => {
                    let value_stack_len = self.value_stack.len();
                    self.value_stack
                        .swap(value_stack_len - 1, value_stack_len - 2);
                }
                State::CoerceToString => {
                    if let ValueData::String(s) = self.value_stack.last().unwrap() {
                        self.string_stack.push((**s).into());
                        self.value_stack.pop().unwrap();
                    } else {
                        self.state_stack.push(State::ManifestJson {
                            format: ManifestJsonFormat::default_to_string(),
                            depth: 0,
                        });
                        self.string_stack.push(String::new());
                    }
                }
                State::CoerceToStringValue => {
                    if !matches!(self.value_stack.last().unwrap(), ValueData::String(_)) {
                        self.state_stack.push(State::StringToValue);
                        self.state_stack.push(State::ManifestJson {
                            format: ManifestJsonFormat::default_to_string(),
                            depth: 0,
                        });
                        self.string_stack.push(String::new());
                    }
                }
                State::BoolToValue => {
                    let value = self.bool_stack.pop().unwrap();
                    self.value_stack.push(ValueData::Bool(value));
                }
                State::StringToValue => {
                    let s = self.string_stack.pop().unwrap();
                    self.value_stack.push(ValueData::String(s.into()));
                }
                State::PushU32AsValue(value) => {
                    self.value_stack.push(ValueData::Number(f64::from(value)));
                }
                State::CmpOrdToBoolValueIsLt => {
                    let cmp_ord = self.cmp_ord_stack.pop().unwrap();
                    self.value_stack.push(ValueData::Bool(cmp_ord.is_lt()));
                }
                State::CmpOrdToBoolValueIsLe => {
                    let cmp_ord = self.cmp_ord_stack.pop().unwrap();
                    self.value_stack.push(ValueData::Bool(cmp_ord.is_le()));
                }
                State::CmpOrdToBoolValueIsGt => {
                    let cmp_ord = self.cmp_ord_stack.pop().unwrap();
                    self.value_stack.push(ValueData::Bool(cmp_ord.is_gt()));
                }
                State::CmpOrdToBoolValueIsGe => {
                    let cmp_ord = self.cmp_ord_stack.pop().unwrap();
                    self.value_stack.push(ValueData::Bool(cmp_ord.is_ge()));
                }
                State::CmpOrdToIntValueThreeWay => {
                    let cmp_ord = self.cmp_ord_stack.pop().unwrap();
                    let value: i8 = match cmp_ord {
                        std::cmp::Ordering::Less => -1,
                        std::cmp::Ordering::Equal => 0,
                        std::cmp::Ordering::Greater => 1,
                    };
                    self.value_stack.push(ValueData::Number(f64::from(value)));
                }
                State::InvertBool => {
                    let value = self.bool_stack.last_mut().unwrap();
                    *value = !*value;
                }
                State::AppendToString(s) => {
                    self.string_stack.last_mut().unwrap().push_str(&s);
                }
                State::OutputValue => {
                    let value = self.value_stack.pop().unwrap();
                    assert!(matches!(self.output, EvalOutput::Nothing));
                    self.output = EvalOutput::Value(value);
                }
                State::OutputString => {
                    let s = self.string_stack.pop().unwrap();
                    assert!(matches!(self.output, EvalOutput::Nothing));
                    self.output = EvalOutput::String(s);
                }
                State::ArrayToValue => {
                    let array = self.array_stack.pop().unwrap();
                    self.value_stack.push(ValueData::Array(
                        self.program.gc_alloc(array.into_boxed_slice()),
                    ));
                }
                State::ManifestJson { format, depth } => {
                    self.do_manifest_json(format, depth)?;
                }
                State::Expr { expr, env } => self.do_expr(expr, env)?,
                State::Error { span } => {
                    let msg = self.string_stack.pop().unwrap();
                    return Err(
                        self.report_error(EvalErrorKind::ExplicitError { span, message: msg })
                    );
                }
                State::Assert {
                    assert_span,
                    cond_span,
                    msg_expr,
                } => {
                    let cond_value = self.value_stack.pop().unwrap();
                    if let ValueData::Bool(cond_value) = cond_value {
                        if !cond_value {
                            if let Some((msg_expr, msg_env)) = msg_expr {
                                self.state_stack.push(State::AssertMsg { assert_span });
                                self.state_stack.push(State::CoerceToString);
                                self.state_stack.push(State::Expr {
                                    expr: msg_expr,
                                    env: msg_env,
                                });
                            } else {
                                return Err(self.report_error(EvalErrorKind::AssertFailed {
                                    span: assert_span,
                                    message: None,
                                }));
                            }
                        }
                    } else {
                        return Err(self.report_error(EvalErrorKind::CondIsNotBool {
                            span: cond_span,
                            got_type: EvalErrorValueType::from_value(&cond_value),
                        }));
                    }
                }
                State::AssertMsg { assert_span } => {
                    let msg = self.string_stack.pop().unwrap();
                    return Err(self.report_error(EvalErrorKind::AssertFailed {
                        span: assert_span,
                        message: Some(msg),
                    }));
                }
                State::ObjectFixField {
                    name,
                    name_span,
                    plus,
                    visibility,
                    value,
                    base_env,
                } => {
                    self.add_object_field(name, name_span, plus, visibility, value, base_env)?;
                }
                State::ObjectDynField {
                    name_span,
                    plus,
                    visibility,
                    value,
                    base_env,
                } => {
                    let name_value = self.value_stack.pop().unwrap();
                    if let ValueData::String(name) = name_value {
                        let name = self.program.str_interner.intern(&name);
                        self.add_object_field(name, name_span, plus, visibility, value, base_env)?;
                    } else if !matches!(name_value, ValueData::Null) {
                        return Err(self.report_error(EvalErrorKind::FieldNameIsNotString {
                            span: name_span,
                            got_type: EvalErrorValueType::from_value(&name_value),
                        }));
                    }
                }
                State::FinishObject => {
                    let object = self.object_stack.pop().unwrap();
                    let object = self.program.gc_alloc(object);

                    self.value_stack.push(ValueData::Object(object));
                }
                State::InitCompSpec {
                    var_name,
                    value,
                    value_span,
                    env,
                } => {
                    self.state_stack.push(State::GotInitCompSpec {
                        var_name,
                        value_span,
                    });
                    self.state_stack.push(State::Expr { expr: value, env });
                }
                State::GotInitCompSpec {
                    var_name,
                    value_span,
                } => {
                    let value = self.value_stack.pop().unwrap();
                    let ValueData::Array(array) = value else {
                        return Err(self.report_error(EvalErrorKind::ForSpecValueIsNotArray {
                            span: value_span,
                            got_type: EvalErrorValueType::from_value(&value),
                        }));
                    };
                    let array = array.view();

                    let mut comp_spec = CompSpec { vars: Vec::new() };
                    for item in array.iter() {
                        let mut vars = HashMap::new();
                        vars.insert(var_name.clone(), item.view());
                        comp_spec.vars.push(vars);
                    }

                    self.comp_spec_stack.push(comp_spec);
                }
                State::ForSpec {
                    var_name,
                    value,
                    value_span,
                    env,
                } => {
                    let comp_spec = self.comp_spec_stack.last().unwrap();
                    self.state_stack.push(State::GotForSpec {
                        var_name,
                        value_span,
                    });
                    for vars in comp_spec.vars.iter().rev() {
                        let mut inner_env_data = ThunkEnvData::new(Some(Gc::from(&env)));
                        for (var_name, var_value) in vars.iter() {
                            inner_env_data.set_var(var_name.clone(), Gc::from(var_value));
                        }
                        let inner_env = self.program.gc_alloc_view(ThunkEnv::from(inner_env_data));
                        self.state_stack.push(State::Expr {
                            expr: value.clone(),
                            env: inner_env,
                        });
                    }
                }
                State::GotForSpec {
                    var_name,
                    value_span,
                } => {
                    let comp_spec = self.comp_spec_stack.last_mut().unwrap();
                    let old_vars = std::mem::take(&mut comp_spec.vars);

                    let mut values = self
                        .value_stack
                        .drain(self.value_stack.len() - old_vars.len()..);
                    for old_vars in old_vars.iter() {
                        let value = values.next().unwrap();
                        let ValueData::Array(array) = value else {
                            drop(values);
                            return Err(self.report_error(EvalErrorKind::ForSpecValueIsNotArray {
                                span: value_span,
                                got_type: EvalErrorValueType::from_value(&value),
                            }));
                        };
                        let array = array.view();
                        for item in array.iter() {
                            let mut new_vars = old_vars.clone();
                            new_vars.insert(var_name.clone(), item.view());
                            comp_spec.vars.push(new_vars);
                        }
                    }
                }
                State::IfSpec {
                    cond,
                    cond_span,
                    env,
                } => {
                    let comp_spec = self.comp_spec_stack.last().unwrap();
                    self.state_stack.push(State::GotIfSpec { cond_span });
                    for vars in comp_spec.vars.iter().rev() {
                        let mut inner_env_data = ThunkEnvData::new(Some(Gc::from(&env)));
                        for (var_name, var_value) in vars.iter() {
                            inner_env_data.set_var(var_name.clone(), Gc::from(var_value));
                        }
                        let inner_env = self.program.gc_alloc_view(ThunkEnv::from(inner_env_data));
                        self.state_stack.push(State::Expr {
                            expr: cond.clone(),
                            env: inner_env,
                        });
                    }
                }
                State::GotIfSpec { cond_span } => {
                    let comp_spec = self.comp_spec_stack.last_mut().unwrap();
                    let old_vars = std::mem::take(&mut comp_spec.vars);

                    let mut values = self
                        .value_stack
                        .drain(self.value_stack.len() - old_vars.len()..);
                    for old_vars in old_vars {
                        let cond_value = values.next().unwrap();
                        let ValueData::Bool(cond_value) = cond_value else {
                            drop(values);
                            return Err(self.report_error(EvalErrorKind::CondIsNotBool {
                                span: cond_span,
                                got_type: EvalErrorValueType::from_value(&cond_value),
                            }));
                        };
                        if cond_value {
                            comp_spec.vars.push(old_vars);
                        }
                    }
                }
                State::ArrayComp { item, env } => {
                    let comp_spec = self.comp_spec_stack.pop().unwrap();
                    if comp_spec.vars.is_empty() {
                        self.value_stack
                            .push(ValueData::Array(Gc::from(&self.program.empty_array)));
                    } else {
                        let mut array = Vec::with_capacity(comp_spec.vars.len());
                        for vars in comp_spec.vars.iter() {
                            let mut item_env_data = ThunkEnvData::new(Some(Gc::from(&env)));
                            for (var_name, var_value) in vars.iter() {
                                item_env_data.set_var(var_name.clone(), Gc::from(var_value));
                            }
                            let item_env = self.program.gc_alloc(ThunkEnv::from(item_env_data));
                            array.push(self.new_pending_expr_thunk(item.clone(), item_env));
                        }
                        self.value_stack.push(ValueData::Array(
                            self.program.gc_alloc(array.into_boxed_slice()),
                        ));
                    }
                }
                State::ObjectComp { expr, env } => {
                    let ir::Expr::ObjectComp {
                        is_top,
                        locals: ref ir_locals,
                        ref field_name,
                        field_name_span,
                        ref field_value,
                        ..
                    } = *expr
                    else {
                        unreachable!();
                    };

                    let comp_spec = self.comp_spec_stack.pop().unwrap();

                    self.object_stack.push(ObjectData {
                        self_core: ObjectCore {
                            is_top,
                            locals: ir_locals.clone(),
                            base_env: None,
                            env: OnceCell::new(),
                            fields: HashMap::new(),
                            asserts: Vec::new(),
                        },
                        super_cores: Vec::new(),
                        fields_order: OnceCell::new(),
                        asserts_checked: Cell::new(true),
                    });

                    self.state_stack.push(State::FinishObjectComp);

                    for vars in comp_spec.vars.iter().rev() {
                        let mut item_env_data = ThunkEnvData::new(Some(Gc::from(&env)));
                        for (var_name, var_value) in vars.iter() {
                            item_env_data.set_var(var_name.clone(), Gc::from(var_value));
                        }
                        let outer_env = self.program.gc_alloc_view(ThunkEnv::from(item_env_data));

                        self.state_stack.push(State::ObjectDynField {
                            name_span: field_name_span,
                            plus: false,
                            visibility: ast::Visibility::Default,
                            value: field_value.clone(),
                            base_env: Some(Gc::from(&outer_env)),
                        });
                        self.state_stack.push(State::Expr {
                            expr: field_name.clone(),
                            env: outer_env,
                        });
                    }
                }
                State::FinishObjectComp => {
                    let object = self.object_stack.pop().unwrap();
                    let object = self.program.gc_alloc(object);

                    self.value_stack.push(ValueData::Object(object));
                }
                State::Field { span, field_name } => {
                    let object = self.value_stack.pop().unwrap();
                    if let ValueData::Object(ref object) = object {
                        self.want_field(&object.view(), &field_name, span)?;
                    } else {
                        return Err(self.report_error(EvalErrorKind::FieldOfNonObject { span }));
                    }
                }
                State::Index { span } => {
                    let index_value = self.value_stack.pop().unwrap();
                    let object_value = self.value_stack.pop().unwrap();
                    match object_value {
                        ValueData::String(ref s) => {
                            let ValueData::Number(index) = index_value else {
                                return Err(self.report_error(
                                    EvalErrorKind::StringIndexIsNotNumber {
                                        span,
                                        got_type: EvalErrorValueType::from_value(&index_value),
                                    },
                                ));
                            };
                            let Some(index_usize) = float::try_to_usize_exact(index) else {
                                return Err(self.report_error(
                                    EvalErrorKind::NumericIndexIsNotValid {
                                        span,
                                        index: index.to_string(),
                                    },
                                ));
                            };
                            if let Some(chr) = s.chars().nth(index_usize) {
                                let mut buf = [0; 4];
                                let chr_str: &str = chr.encode_utf8(&mut buf);
                                self.value_stack.push(ValueData::String(chr_str.into()));
                            } else {
                                return Err(self.report_error(
                                    EvalErrorKind::NumericIndexOutOfRange {
                                        span,
                                        index: index_usize,
                                        length: s.chars().count(),
                                    },
                                ));
                            }
                        }
                        ValueData::Array(ref array) => {
                            let array = array.view();
                            let ValueData::Number(index) = index_value else {
                                return Err(self.report_error(
                                    EvalErrorKind::ArrayIndexIsNotNumber {
                                        span,
                                        got_type: EvalErrorValueType::from_value(&index_value),
                                    },
                                ));
                            };
                            let Some(index_usize) = float::try_to_usize_exact(index) else {
                                return Err(self.report_error(
                                    EvalErrorKind::NumericIndexIsNotValid {
                                        span,
                                        index: index.to_string(),
                                    },
                                ));
                            };
                            if let Some(item) = array.get(index_usize) {
                                self.push_trace_item(TraceItem::ArrayItem {
                                    span: Some(span),
                                    index: index_usize,
                                });
                                self.state_stack.push(State::DoThunk(item.view()));
                            } else {
                                return Err(self.report_error(
                                    EvalErrorKind::NumericIndexOutOfRange {
                                        span,
                                        index: index_usize,
                                        length: array.len(),
                                    },
                                ));
                            }
                        }
                        ValueData::Object(ref object) => {
                            let ValueData::String(ref field_name) = index_value else {
                                return Err(self.report_error(
                                    EvalErrorKind::ObjectIndexIsNotString {
                                        span,
                                        got_type: EvalErrorValueType::from_value(&index_value),
                                    },
                                ));
                            };
                            if let Some(field_name) =
                                self.program.str_interner.get_interned(field_name)
                            {
                                self.want_field(&object.view(), &field_name, span)?;
                            } else {
                                return Err(self.report_error(EvalErrorKind::UnknownObjectField {
                                    span,
                                    field_name: (**field_name).into(),
                                }));
                            }
                        }
                        _ => {
                            return Err(self.report_error(EvalErrorKind::InvalidIndexedType {
                                span,
                                got_type: EvalErrorValueType::from_value(&object_value),
                            }));
                        }
                    }
                }
                State::SuperIndex {
                    span,
                    super_span,
                    env,
                } => {
                    let index_value = self.value_stack.pop().unwrap();
                    let ValueData::String(ref field_name) = index_value else {
                        return Err(self.report_error(EvalErrorKind::ObjectIndexIsNotString {
                            span,
                            got_type: EvalErrorValueType::from_value(&index_value),
                        }));
                    };
                    if let Some(field_name) = self.program.str_interner.get_interned(field_name) {
                        self.want_super_field(&env, super_span, &field_name, span)?;
                    } else {
                        return Err(self.report_error(EvalErrorKind::UnknownObjectField {
                            span,
                            field_name: (**field_name).into(),
                        }));
                    }
                }
                State::UnaryOp { span, op } => {
                    let rhs = self.value_stack.pop().unwrap();
                    match (op, rhs) {
                        (ast::UnaryOp::Minus, ValueData::Number(rhs)) => {
                            self.value_stack.push(ValueData::Number(-rhs));
                        }
                        (ast::UnaryOp::Plus, ValueData::Number(rhs)) => {
                            self.value_stack.push(ValueData::Number(rhs));
                        }
                        (ast::UnaryOp::BitwiseNot, ValueData::Number(rhs)) => {
                            let int = rhs as i64;
                            self.value_stack.push(ValueData::Number(!int as f64));
                        }
                        (ast::UnaryOp::LogicNot, ValueData::Bool(rhs)) => {
                            self.value_stack.push(ValueData::Bool(!rhs));
                        }
                        (_, rhs) => {
                            return Err(self.report_error(EvalErrorKind::InvalidUnaryOpType {
                                span,
                                op,
                                rhs_type: EvalErrorValueType::from_value(&rhs),
                            }));
                        }
                    }
                }
                State::BinaryOp { span, op } => {
                    self.do_binary_op(span, op)?;
                }
                State::LogicAnd { span, rhs, env } => {
                    if matches!(self.value_stack.last().unwrap(), ValueData::Bool(false)) {
                        self.value_stack.pop().unwrap();
                        self.value_stack.push(ValueData::Bool(false));
                    } else {
                        self.state_stack.push(State::BinaryOp {
                            span: Some(span),
                            op: ast::BinaryOp::LogicAnd,
                        });
                        self.state_stack.push(State::Expr { expr: rhs, env });
                    }
                }
                State::LogicOr { span, rhs, env } => {
                    if matches!(self.value_stack.last().unwrap(), ValueData::Bool(true)) {
                        self.value_stack.pop().unwrap();
                        self.value_stack.push(ValueData::Bool(true));
                    } else {
                        self.state_stack.push(State::BinaryOp {
                            span: Some(span),
                            op: ast::BinaryOp::LogicOr,
                        });
                        self.state_stack.push(State::Expr { expr: rhs, env });
                    }
                }
                State::InSuper { span, env } => {
                    let lhs = self.value_stack.pop().unwrap();
                    let ValueData::String(ref field_name) = lhs else {
                        return Err(self.report_error(EvalErrorKind::InvalidBinaryOpTypes {
                            span: Some(span),
                            op: ast::BinaryOp::In,
                            lhs_type: EvalErrorValueType::from_value(&lhs),
                            rhs_type: EvalErrorValueType::Object,
                        }));
                    };

                    let has_field = self
                        .program
                        .str_interner
                        .get_interned(field_name)
                        .is_some_and(|field_name| {
                            let (object, core_i) = env.get_object();
                            let object = object.view();
                            object.has_field(core_i + 1, &field_name)
                        });

                    self.value_stack.push(ValueData::Bool(has_field));
                }
                State::EqualsValue => {
                    let rhs = self.value_stack.pop().unwrap();
                    let lhs = self.value_stack.pop().unwrap();
                    match (lhs, rhs) {
                        (ValueData::Null, ValueData::Null) => {
                            self.bool_stack.push(true);
                        }
                        (ValueData::Bool(lhs), ValueData::Bool(rhs)) => {
                            self.bool_stack.push(lhs == rhs);
                        }
                        (ValueData::Number(lhs), ValueData::Number(rhs)) => {
                            self.bool_stack.push(lhs == rhs);
                        }
                        (ValueData::String(lhs), ValueData::String(rhs)) => {
                            self.bool_stack.push(lhs == rhs);
                        }
                        (ValueData::Array(lhs), ValueData::Array(rhs)) => {
                            let lhs = lhs.view();
                            let rhs = rhs.view();
                            let lhs_len = lhs.len();
                            let rhs_len = rhs.len();
                            if lhs_len != rhs_len {
                                self.bool_stack.push(false);
                            } else if lhs_len == 0 {
                                self.bool_stack.push(true);
                            } else {
                                self.state_stack.push(State::EqualsArray {
                                    lhs: lhs.clone(),
                                    rhs: rhs.clone(),
                                    index: 0,
                                });
                                self.push_trace_item(TraceItem::ArrayItem {
                                    span: None,
                                    index: 0,
                                });
                                self.state_stack.push(State::EqualsValue);
                                self.state_stack.push(State::DoThunk(rhs[0].view()));
                                self.state_stack.push(State::DoThunk(lhs[0].view()));
                            }
                        }
                        (ValueData::Object(lhs), ValueData::Object(rhs)) => {
                            let lhs = lhs.view();
                            let rhs = rhs.view();
                            let lhs_fields: Vec<_> = lhs
                                .get_fields_order()
                                .iter()
                                .rev()
                                .filter(|name| lhs.field_is_visible(name))
                                .cloned()
                                .collect();
                            let rhs_fields: Vec<_> = rhs
                                .get_fields_order()
                                .iter()
                                .rev()
                                .filter(|name| rhs.field_is_visible(name))
                                .cloned()
                                .collect();

                            if lhs_fields == rhs_fields {
                                let mut fields = lhs_fields;
                                if let Some(first_field) = fields.pop() {
                                    let lhs_field = self
                                        .program
                                        .find_object_field_thunk(&lhs, 0, &first_field)
                                        .unwrap();
                                    let rhs_field = self
                                        .program
                                        .find_object_field_thunk(&rhs, 0, &first_field)
                                        .unwrap();
                                    self.state_stack.push(State::EqualsObject {
                                        lhs: lhs.clone(),
                                        rhs: rhs.clone(),
                                        rem_fields: fields,
                                    });
                                    self.push_trace_item(TraceItem::ObjectField {
                                        span: None,
                                        name: first_field,
                                    });
                                    self.state_stack.push(State::EqualsValue);
                                    self.state_stack.push(State::DoThunk(rhs_field));
                                    self.state_stack.push(State::DoThunk(lhs_field));

                                    self.check_object_asserts(&rhs);
                                    self.check_object_asserts(&lhs);
                                } else {
                                    self.bool_stack.push(true);
                                }
                            } else {
                                self.bool_stack.push(false);
                            }
                        }
                        (ValueData::Function(_), ValueData::Function(_)) => {
                            return Err(self.report_error(EvalErrorKind::CompareFunctions));
                        }
                        _ => {
                            // Different types compare to false.
                            self.bool_stack.push(false);
                        }
                    }
                }
                State::EqualsArray { lhs, rhs, index } => {
                    assert_eq!(lhs.len(), rhs.len());
                    if index != lhs.len() - 1 {
                        let item_eq = self.bool_stack.pop().unwrap();
                        if item_eq {
                            let index = index + 1;
                            self.state_stack.push(State::EqualsArray {
                                lhs: lhs.clone(),
                                rhs: rhs.clone(),
                                index,
                            });
                            self.push_trace_item(TraceItem::ArrayItem { span: None, index });
                            self.state_stack.push(State::EqualsValue);
                            self.state_stack.push(State::DoThunk(rhs[index].view()));
                            self.state_stack.push(State::DoThunk(lhs[index].view()));
                        } else {
                            self.bool_stack.push(false);
                        }
                    }
                }
                State::EqualsObject {
                    lhs,
                    rhs,
                    mut rem_fields,
                } => {
                    if let Some(next_field) = rem_fields.pop() {
                        let field_eq = self.bool_stack.pop().unwrap();
                        if field_eq {
                            let lhs_field = self
                                .program
                                .find_object_field_thunk(&lhs, 0, &next_field)
                                .unwrap();
                            let rhs_field = self
                                .program
                                .find_object_field_thunk(&rhs, 0, &next_field)
                                .unwrap();
                            self.state_stack.push(State::EqualsObject {
                                lhs,
                                rhs,
                                rem_fields,
                            });
                            self.push_trace_item(TraceItem::ObjectField {
                                span: None,
                                name: next_field,
                            });
                            self.state_stack.push(State::EqualsValue);
                            self.state_stack.push(State::DoThunk(rhs_field));
                            self.state_stack.push(State::DoThunk(lhs_field));
                        } else {
                            self.bool_stack.push(false);
                        }
                    }
                }
                State::CompareValue => {
                    let rhs = self.value_stack.pop().unwrap();
                    let lhs = self.value_stack.pop().unwrap();
                    match (lhs, rhs) {
                        (ValueData::Null, ValueData::Null) => {
                            return Err(self.report_error(EvalErrorKind::CompareNullInequality));
                        }
                        (ValueData::Bool(_), ValueData::Bool(_)) => {
                            return Err(self.report_error(EvalErrorKind::CompareBooleanInequality));
                        }
                        (ValueData::Number(lhs), ValueData::Number(rhs)) => {
                            // NaNs are not allowed.
                            self.cmp_ord_stack.push(lhs.partial_cmp(&rhs).unwrap());
                        }
                        (ValueData::String(lhs), ValueData::String(rhs)) => {
                            self.cmp_ord_stack.push(lhs.cmp(&rhs));
                        }
                        (ValueData::Array(lhs), ValueData::Array(rhs)) => {
                            let lhs = lhs.view();
                            let rhs = rhs.view();
                            match (lhs.is_empty(), rhs.is_empty()) {
                                (true, true) => {
                                    self.cmp_ord_stack.push(std::cmp::Ordering::Equal);
                                }
                                (true, false) => {
                                    self.cmp_ord_stack.push(std::cmp::Ordering::Less);
                                }
                                (false, true) => {
                                    self.cmp_ord_stack.push(std::cmp::Ordering::Greater);
                                }
                                (false, false) => {
                                    self.state_stack.push(State::CompareArray {
                                        lhs: lhs.clone(),
                                        rhs: rhs.clone(),
                                        index: 0,
                                    });
                                    self.push_trace_item(TraceItem::ArrayItem {
                                        span: None,
                                        index: 0,
                                    });
                                    self.state_stack.push(State::CompareValue);
                                    self.state_stack.push(State::DoThunk(rhs[0].view()));
                                    self.state_stack.push(State::DoThunk(lhs[0].view()));
                                }
                            }
                        }
                        (ValueData::Object(_), ValueData::Object(_)) => {
                            return Err(self.report_error(EvalErrorKind::CompareObjectInequality));
                        }
                        (ValueData::Function(_), ValueData::Function(_)) => {
                            return Err(self.report_error(EvalErrorKind::CompareFunctions));
                        }
                        (lhs, rhs) => {
                            return Err(self.report_error(
                                EvalErrorKind::CompareDifferentTypesInequality {
                                    lhs_type: EvalErrorValueType::from_value(&lhs),
                                    rhs_type: EvalErrorValueType::from_value(&rhs),
                                },
                            ));
                        }
                    }
                }
                State::CompareArray { lhs, rhs, index } => {
                    let item_cmp = self.cmp_ord_stack.pop().unwrap();
                    if item_cmp.is_eq() {
                        match (index == lhs.len() - 1, index == rhs.len() - 1) {
                            (true, true) => {
                                self.cmp_ord_stack.push(std::cmp::Ordering::Equal);
                            }
                            (true, false) => {
                                self.cmp_ord_stack.push(std::cmp::Ordering::Less);
                            }
                            (false, true) => {
                                self.cmp_ord_stack.push(std::cmp::Ordering::Greater);
                            }
                            (false, false) => {
                                let index = index + 1;
                                self.state_stack.push(State::CompareArray {
                                    lhs: lhs.clone(),
                                    rhs: rhs.clone(),
                                    index,
                                });
                                self.push_trace_item(TraceItem::ArrayItem { span: None, index });
                                self.state_stack.push(State::CompareValue);
                                self.state_stack.push(State::DoThunk(rhs[index].view()));
                                self.state_stack.push(State::DoThunk(lhs[index].view()));
                            }
                        }
                    } else {
                        self.cmp_ord_stack.push(item_cmp);
                    }
                }
                State::CallWithExpr {
                    call_expr,
                    call_env,
                } => {
                    let ir::Expr::Call {
                        ref positional_args,
                        ref named_args,
                        tailstrict,
                        span: call_span,
                        ..
                    } = *call_expr
                    else {
                        unreachable!();
                    };
                    let callee_value = self.value_stack.pop().unwrap();
                    let ValueData::Function(func) = callee_value else {
                        return Err(self.report_error(EvalErrorKind::CalleeIsNotFunction {
                            span: Some(call_span),
                            got_type: EvalErrorValueType::from_value(&callee_value),
                        }));
                    };
                    let func = func.view();
                    let (func_name, params, func_env) = self.get_func_info(&func);
                    let args_thunks = self.check_call_expr_args(
                        &params,
                        positional_args,
                        named_args,
                        call_env,
                        func_env,
                        call_span,
                    )?;
                    match *func {
                        FuncData::Normal { .. } if tailstrict => {
                            self.state_stack.push(State::ExecTailstrictCall {
                                func,
                                args: args_thunks.clone(),
                            });

                            for (i, arg_thunk) in args_thunks.iter().enumerate().rev() {
                                self.push_trace_item(TraceItem::Variable {
                                    span: call_span,
                                    name: params.order[i].clone(),
                                });
                                self.state_stack.push(State::DiscardValue);
                                self.state_stack.push(State::DoThunk(arg_thunk.view()));
                                self.delay_trace_item();
                            }
                        }
                        _ => {
                            self.push_trace_item(TraceItem::Call {
                                span: Some(call_span),
                                name: func_name,
                            });

                            self.execute_call(&func, args_thunks);
                        }
                    }
                }
                State::TopLevelCall {
                    pos_args: positional_args,
                    named_args,
                } => {
                    let callee_value = self.value_stack.pop().unwrap();
                    let ValueData::Function(func) = callee_value else {
                        return Err(self.report_error(EvalErrorKind::CalleeIsNotFunction {
                            span: None,
                            got_type: EvalErrorValueType::from_value(&callee_value),
                        }));
                    };
                    let func = func.view();
                    self.check_thunk_args_and_execute_call(
                        &func,
                        &positional_args,
                        &named_args,
                        None,
                    )?;
                }
                State::ExecTailstrictCall { func, args } => {
                    self.execute_call(&func, args);
                }
                State::ExecNativeCall { name, args } => {
                    let callbacks = self
                        .callbacks
                        .as_deref_mut()
                        .expect("unexpected call to native function without callbacks");
                    let args: Vec<_> = args
                        .iter()
                        .map(|arg| super::Value::from_thunk(arg))
                        .collect();
                    match callbacks.native_call(self.program, &name, &args) {
                        Ok(result_value) => {
                            self.value_stack.push(result_value.inner);
                        }
                        Err(_) => {
                            return Err(self.report_error(EvalErrorKind::NativeCallFailed));
                        }
                    }
                }
                State::If {
                    cond_span,
                    then_body,
                    else_body,
                    env,
                } => {
                    let cond_value = self.value_stack.pop().unwrap();
                    if let ValueData::Bool(cond_value) = cond_value {
                        if cond_value {
                            self.state_stack.push(State::Expr {
                                expr: then_body,
                                env,
                            });
                        } else if let Some(else_body) = else_body {
                            self.state_stack.push(State::Expr {
                                expr: else_body,
                                env,
                            });
                        } else {
                            self.value_stack.push(ValueData::Null);
                        }
                    } else {
                        return Err(self.report_error(EvalErrorKind::CondIsNotBool {
                            span: cond_span,
                            got_type: EvalErrorValueType::from_value(&cond_value),
                        }));
                    }
                }
                State::StdExtVar => {
                    let arg = self.value_stack.pop().unwrap();
                    let name = self.expect_std_func_arg_string(arg, "extVar", 0)?;

                    let thunk = self
                        .program
                        .str_interner
                        .get_interned(&name)
                        .and_then(|name| self.program.ext_vars.get(&name).cloned())
                        .ok_or_else(|| {
                            self.report_error(EvalErrorKind::UnknownExtVar {
                                name: (*name).into(),
                            })
                        })?;

                    self.state_stack.push(State::DoThunk(thunk));
                }
                State::StdType => {
                    let arg = self.value_stack.pop().unwrap();
                    let type_str = match arg {
                        ValueData::Null => "null",
                        ValueData::Bool(_) => "boolean",
                        ValueData::Number(_) => "number",
                        ValueData::String(_) => "string",
                        ValueData::Array(_) => "array",
                        ValueData::Object(_) => "object",
                        ValueData::Function(_) => "function",
                    };
                    self.value_stack.push(ValueData::String(type_str.into()));
                }
                State::StdIsArray => {
                    let arg = self.value_stack.pop().unwrap();
                    self.value_stack
                        .push(ValueData::Bool(matches!(arg, ValueData::Array(_))));
                }
                State::StdIsBoolean => {
                    let arg = self.value_stack.pop().unwrap();
                    self.value_stack
                        .push(ValueData::Bool(matches!(arg, ValueData::Bool(_))));
                }
                State::StdIsFunction => {
                    let arg = self.value_stack.pop().unwrap();
                    self.value_stack
                        .push(ValueData::Bool(matches!(arg, ValueData::Function(_))));
                }
                State::StdIsNumber => {
                    let arg = self.value_stack.pop().unwrap();
                    self.value_stack
                        .push(ValueData::Bool(matches!(arg, ValueData::Number(_))));
                }
                State::StdIsObject => {
                    let arg = self.value_stack.pop().unwrap();
                    self.value_stack
                        .push(ValueData::Bool(matches!(arg, ValueData::Object(_))));
                }
                State::StdIsString => {
                    let arg = self.value_stack.pop().unwrap();
                    self.value_stack
                        .push(ValueData::Bool(matches!(arg, ValueData::String(_))));
                }
                State::StdLength => {
                    let arg = self.value_stack.pop().unwrap();
                    let length = match arg {
                        ValueData::String(s) => s.chars().count(),
                        ValueData::Array(array) => array.view().len(),
                        ValueData::Object(object) => {
                            let object = object.view();
                            object
                                .get_fields_order()
                                .iter()
                                .filter(|name| object.field_is_visible(name))
                                .count()
                        }
                        ValueData::Function(func) => match *func.view() {
                            FuncData::Normal { ref params, .. } => params.order.len(),
                            FuncData::BuiltIn { ref params, .. } => params.order.len(),
                            FuncData::Native { ref params, .. } => params.order.len(),
                        },
                        _ => {
                            return Err(self.report_error(EvalErrorKind::InvalidStdFuncArgType {
                                func_name: "length".into(),
                                arg_index: 0,
                                expected_types: vec![
                                    EvalErrorValueType::String,
                                    EvalErrorValueType::Array,
                                    EvalErrorValueType::Object,
                                    EvalErrorValueType::Function,
                                ],
                                got_type: EvalErrorValueType::from_value(&arg),
                            }));
                        }
                    };
                    self.value_stack.push(ValueData::Number(length as f64));
                }
                State::StdObjectHasEx => {
                    let inc_hidden = self.value_stack.pop().unwrap();
                    let field_name = self.value_stack.pop().unwrap();
                    let object = self.value_stack.pop().unwrap();

                    let object = self.expect_std_func_arg_object(object, "objectHasEx", 0)?;
                    let field_name =
                        self.expect_std_func_arg_string(field_name, "objectHasEx", 1)?;
                    let inc_hidden = self.expect_std_func_arg_bool(inc_hidden, "objectHasEx", 2)?;

                    let has_field = self
                        .program
                        .str_interner
                        .get_interned(&field_name)
                        .is_some_and(|field_name| {
                            object.has_field(0, &field_name)
                                && (inc_hidden || object.field_is_visible(&field_name))
                        });

                    self.value_stack.push(ValueData::Bool(has_field));
                }
                State::StdObjectFieldsEx => {
                    let inc_hidden = self.value_stack.pop().unwrap();
                    let object = self.value_stack.pop().unwrap();

                    let object = self.expect_std_func_arg_object(object, "objectFieldsEx", 0)?;
                    let inc_hidden =
                        self.expect_std_func_arg_bool(inc_hidden, "objectFieldsEx", 1)?;

                    let fields_order = object.get_fields_order();

                    let array = self
                        .program
                        .make_value_array(fields_order.iter().filter_map(|name| {
                            if inc_hidden || object.field_is_visible(name) {
                                Some(ValueData::String(name.value().into()))
                            } else {
                                None
                            }
                        }));

                    self.value_stack.push(ValueData::Array(array));
                }
                State::StdPrimitiveEquals => {
                    let rhs = self.value_stack.pop().unwrap();
                    let lhs = self.value_stack.pop().unwrap();
                    match (lhs, rhs) {
                        (ValueData::Null, ValueData::Null) => {
                            self.value_stack.push(ValueData::Bool(true));
                        }
                        (ValueData::Bool(lhs), ValueData::Bool(rhs)) => {
                            self.value_stack.push(ValueData::Bool(lhs == rhs));
                        }
                        (ValueData::Number(lhs), ValueData::Number(rhs)) => {
                            self.value_stack.push(ValueData::Bool(lhs == rhs));
                        }
                        (ValueData::String(lhs), ValueData::String(rhs)) => {
                            self.value_stack.push(ValueData::Bool(lhs == rhs));
                        }
                        (ValueData::Array(_), ValueData::Array(_)) => {
                            return Err(self.report_error(
                                EvalErrorKind::PrimitiveEqualsNonPrimitive {
                                    got_type: EvalErrorValueType::Array,
                                },
                            ));
                        }
                        (ValueData::Object(_), ValueData::Object(_)) => {
                            return Err(self.report_error(
                                EvalErrorKind::PrimitiveEqualsNonPrimitive {
                                    got_type: EvalErrorValueType::Object,
                                },
                            ));
                        }
                        (ValueData::Function(_), ValueData::Function(_)) => {
                            return Err(self.report_error(EvalErrorKind::CompareFunctions));
                        }
                        _ => {
                            // Different types compare to false.
                            self.value_stack.push(ValueData::Bool(false));
                        }
                    }
                }
                State::StdEquals => {
                    self.state_stack.push(State::BoolToValue);
                    self.state_stack.push(State::EqualsValue);
                }
                State::StdCompareArray => {
                    let rhs = &self.value_stack[self.value_stack.len() - 1];
                    let lhs = &self.value_stack[self.value_stack.len() - 2];
                    if !matches!(lhs, ValueData::Array(_)) {
                        return Err(self.report_error(EvalErrorKind::InvalidStdFuncArgType {
                            func_name: "__compare_array".into(),
                            arg_index: 0,
                            expected_types: vec![EvalErrorValueType::Array],
                            got_type: EvalErrorValueType::from_value(lhs),
                        }));
                    }
                    if !matches!(rhs, ValueData::Array(_)) {
                        return Err(self.report_error(EvalErrorKind::InvalidStdFuncArgType {
                            func_name: "__compare_array".into(),
                            arg_index: 1,
                            expected_types: vec![EvalErrorValueType::Array],
                            got_type: EvalErrorValueType::from_value(rhs),
                        }));
                    }
                    self.state_stack.push(State::CmpOrdToIntValueThreeWay);
                    self.state_stack.push(State::CompareValue);
                }
                State::StdExponent => {
                    let arg = self.value_stack.pop().unwrap();
                    let arg = self.expect_std_func_arg_number(arg, "exponent", 0)?;
                    let (_, exp) = float::frexp(arg);
                    self.value_stack.push(ValueData::Number(exp.into()));
                }
                State::StdMantissa => {
                    let arg = self.value_stack.pop().unwrap();
                    let arg = self.expect_std_func_arg_number(arg, "mantissa", 0)?;
                    let (mant, _) = float::frexp(arg);
                    self.value_stack.push(ValueData::Number(mant));
                }
                State::StdFloor => {
                    let arg = self.value_stack.pop().unwrap();
                    let arg = self.expect_std_func_arg_number(arg, "floor", 0)?;
                    self.value_stack.push(ValueData::Number(arg.floor()));
                }
                State::StdCeil => {
                    let arg = self.value_stack.pop().unwrap();
                    let arg = self.expect_std_func_arg_number(arg, "ceil", 0)?;
                    self.value_stack.push(ValueData::Number(arg.ceil()));
                }
                State::StdModulo => {
                    let rhs = self.value_stack.pop().unwrap();
                    let lhs = self.value_stack.pop().unwrap();

                    let lhs = self.expect_std_func_arg_number(lhs, "modulo", 0)?;
                    let rhs = self.expect_std_func_arg_number(rhs, "modulo", 1)?;

                    if rhs == 0.0 {
                        return Err(self.report_error(EvalErrorKind::DivByZero { span: None }));
                    }
                    let r = lhs % rhs;
                    self.check_number_value(r, None)?;
                    self.value_stack.push(ValueData::Number(r));
                }
                State::StdPow => {
                    let rhs = self.value_stack.pop().unwrap();
                    let lhs = self.value_stack.pop().unwrap();

                    let lhs = self.expect_std_func_arg_number(lhs, "pow", 0)?;
                    let rhs = self.expect_std_func_arg_number(rhs, "pow", 1)?;

                    let r = lhs.powf(rhs);
                    self.check_number_value(r, None)?;
                    self.value_stack.push(ValueData::Number(r));
                }
                State::StdExp => {
                    let arg = self.value_stack.pop().unwrap();
                    let arg = self.expect_std_func_arg_number(arg, "exp", 0)?;

                    let r = arg.exp();
                    self.check_number_value(r, None)?;
                    self.value_stack.push(ValueData::Number(r));
                }
                State::StdLog => {
                    let arg = self.value_stack.pop().unwrap();
                    let arg = self.expect_std_func_arg_number(arg, "log", 0)?;

                    let r = arg.ln();
                    self.check_number_value(r, None)?;
                    self.value_stack.push(ValueData::Number(r));
                }
                State::StdSqrt => {
                    let arg = self.value_stack.pop().unwrap();
                    let arg = self.expect_std_func_arg_number(arg, "sqrt", 0)?;

                    let r = arg.sqrt();
                    self.check_number_value(r, None)?;
                    self.value_stack.push(ValueData::Number(r));
                }
                State::StdSin => {
                    let arg = self.value_stack.pop().unwrap();
                    let arg = self.expect_std_func_arg_number(arg, "sin", 0)?;

                    let r = arg.sin();
                    self.check_number_value(r, None)?;
                    self.value_stack.push(ValueData::Number(r));
                }
                State::StdCos => {
                    let arg = self.value_stack.pop().unwrap();
                    let arg = self.expect_std_func_arg_number(arg, "cos", 0)?;

                    let r = arg.cos();
                    self.check_number_value(r, None)?;
                    self.value_stack.push(ValueData::Number(r));
                }
                State::StdTan => {
                    let arg = self.value_stack.pop().unwrap();
                    let arg = self.expect_std_func_arg_number(arg, "tan", 0)?;

                    let r = arg.tan();
                    self.check_number_value(r, None)?;
                    self.value_stack.push(ValueData::Number(r));
                }
                State::StdAsin => {
                    let arg = self.value_stack.pop().unwrap();
                    let arg = self.expect_std_func_arg_number(arg, "asin", 0)?;

                    let r = arg.asin();
                    self.check_number_value(r, None)?;
                    self.value_stack.push(ValueData::Number(r));
                }
                State::StdAcos => {
                    let arg = self.value_stack.pop().unwrap();
                    let arg = self.expect_std_func_arg_number(arg, "acos", 0)?;

                    let r = arg.acos();
                    self.check_number_value(r, None)?;
                    self.value_stack.push(ValueData::Number(r));
                }
                State::StdAtan => {
                    let arg = self.value_stack.pop().unwrap();
                    let arg = self.expect_std_func_arg_number(arg, "atan", 0)?;

                    let r = arg.atan();
                    self.check_number_value(r, None)?;
                    self.value_stack.push(ValueData::Number(r));
                }
                State::StdAssertEqual => {
                    let lhs = self.value_stack[self.value_stack.len() - 2].clone();
                    let rhs = self.value_stack[self.value_stack.len() - 1].clone();
                    self.value_stack.push(lhs);
                    self.value_stack.push(rhs);

                    self.state_stack.push(State::StdAssertEqualCheck);
                    self.state_stack.push(State::EqualsValue);
                }
                State::StdAssertEqualCheck => {
                    let equal = self.bool_stack.pop().unwrap();
                    if equal {
                        self.value_stack.pop().unwrap();
                        self.value_stack.pop().unwrap();
                        self.value_stack.push(ValueData::Bool(true));
                    } else {
                        let value_stack_len = self.value_stack.len();
                        self.value_stack
                            .swap(value_stack_len - 1, value_stack_len - 2);
                        self.string_stack.push(String::new());
                        self.state_stack.push(State::StdAssertEqualFail1);
                        self.state_stack.push(State::ManifestJson {
                            format: ManifestJsonFormat::default_to_string(),
                            depth: 0,
                        });
                    }
                }
                State::StdAssertEqualFail1 => {
                    self.string_stack.push(String::new());
                    self.state_stack.push(State::StdAssertEqualFail2);
                    self.state_stack.push(State::ManifestJson {
                        format: ManifestJsonFormat::default_to_string(),
                        depth: 0,
                    });
                }
                State::StdAssertEqualFail2 => {
                    let rhs = self.string_stack.pop().unwrap();
                    let lhs = self.string_stack.pop().unwrap();
                    return Err(self.report_error(EvalErrorKind::AssertEqualFailed { lhs, rhs }));
                }
                State::StdCodepoint => {
                    let arg = self.value_stack.pop().unwrap();
                    let arg = self.expect_std_func_arg_string(arg, "codepoint", 0)?;

                    let mut chars = arg.chars();
                    let first_char = chars.next();
                    match (first_char, chars.next()) {
                        (Some(first_char), None) => {
                            let codepoint = u32::from(first_char);
                            self.value_stack
                                .push(ValueData::Number(f64::from(codepoint)));
                        }
                        _ => {
                            return Err(self.report_error(EvalErrorKind::Other {
                                span: None,
                                message: "string is not single-character".into(),
                            }));
                        }
                    }
                }
                State::StdChar => {
                    let arg = self.value_stack.pop().unwrap();
                    let arg = self.expect_std_func_arg_number(arg, "char", 0)?;
                    let arg = arg.trunc();

                    let Some(chr) = float::try_to_u32(arg).and_then(char::from_u32) else {
                        return Err(self.report_error(EvalErrorKind::Other {
                            span: None,
                            message: format!("{arg} is not a valid unicode codepoint"),
                        }));
                    };

                    let mut utf8_buf = [0; 4];
                    let utf8 = chr.encode_utf8(&mut utf8_buf);

                    self.value_stack.push(ValueData::String((*utf8).into()));
                }
                State::StdSubstr => {
                    let len_value = self.value_stack.pop().unwrap();
                    let from_value = self.value_stack.pop().unwrap();
                    let str_value = self.value_stack.pop().unwrap();

                    let str_value = self.expect_std_func_arg_string(str_value, "substr", 0)?;
                    let from = self.expect_std_func_arg_number(from_value, "substr", 1)?;
                    let len = self.expect_std_func_arg_number(len_value, "substr", 2)?;

                    if !from.is_finite() || from.trunc() != from || from < 0.0 {
                        return Err(self.report_error(EvalErrorKind::Other {
                            span: None,
                            message: format!("`from` value {from} is not a non-negative integer"),
                        }));
                    }
                    let from = from as usize;

                    if !len.is_finite() || len.trunc() != len || len < 0.0 {
                        return Err(self.report_error(EvalErrorKind::Other {
                            span: None,
                            message: format!("`len` value {len} is not a non-negative integer"),
                        }));
                    }
                    let len = len as usize;

                    let sub_str: String = str_value.chars().skip(from).take(len).collect();
                    self.value_stack.push(ValueData::String(sub_str.into()));
                }
                State::StdFindSubstr => {
                    let str = self.value_stack.pop().unwrap();
                    let pat = self.value_stack.pop().unwrap();

                    let pat = self.expect_std_func_arg_string(pat, "findSubstr", 0)?;
                    let str = self.expect_std_func_arg_string(str, "findSubstr", 1)?;

                    let array = if let Some(first_pat_chr) = pat.chars().next() {
                        let first_pat_chr_len = first_pat_chr.len_utf8();
                        let mut rem_str = &*str;
                        let mut chr_index = 0;
                        self.program.make_value_array(std::iter::from_fn(|| {
                            rem_str.find(&*pat).map(|i| {
                                let (before, after) = rem_str.split_at(i);
                                let match_pos = chr_index + before.chars().count();
                                rem_str = &after[first_pat_chr_len..];
                                chr_index = match_pos + 1;
                                ValueData::Number(match_pos as f64)
                            })
                        }))
                    } else {
                        // An empty pattern does not produce any matches.
                        Gc::from(&self.program.empty_array)
                    };

                    self.value_stack.push(ValueData::Array(array));
                }
                State::StdStartsWith => {
                    let b = self.value_stack.pop().unwrap();
                    let a = self.value_stack.pop().unwrap();

                    let a = self.expect_std_func_arg_string(a, "startsWith", 0)?;
                    let b = self.expect_std_func_arg_string(b, "startsWith", 1)?;

                    self.value_stack.push(ValueData::Bool(a.starts_with(&*b)));
                }
                State::StdEndsWith => {
                    let b = self.value_stack.pop().unwrap();
                    let a = self.value_stack.pop().unwrap();

                    let a = self.expect_std_func_arg_string(a, "endsWith", 0)?;
                    let b = self.expect_std_func_arg_string(b, "endsWith", 1)?;

                    self.value_stack.push(ValueData::Bool(a.ends_with(&*b)));
                }
                State::StdSplitLimit => {
                    let maxsplits_value = self.value_stack.pop().unwrap();
                    let c_value = self.value_stack.pop().unwrap();
                    let str_value = self.value_stack.pop().unwrap();

                    let str_value = self.expect_std_func_arg_string(str_value, "splitLimit", 0)?;
                    let c_value = self.expect_std_func_arg_string(c_value, "splitLimit", 1)?;
                    let maxsplits =
                        self.expect_std_func_arg_number(maxsplits_value, "splitLimit", 2)?;

                    if c_value.is_empty() {
                        return Err(self.report_error(EvalErrorKind::Other {
                            span: None,
                            message: "split delimiter is empty".into(),
                        }));
                    }

                    if !maxsplits.is_finite() || maxsplits.trunc() != maxsplits {
                        return Err(self.report_error(EvalErrorKind::Other {
                            span: None,
                            message: format!("`maxsplits` value {maxsplits} is not an integer"),
                        }));
                    }

                    let maxsplits = if maxsplits < 0.0 {
                        if maxsplits != -1.0 {
                            return Err(self.report_error(EvalErrorKind::Other {
                                span: None,
                                message: format!(
                                    "`maxsplits` value {maxsplits} is not -1 or non-negative"
                                ),
                            }));
                        }
                        None
                    } else {
                        float::try_to_usize(maxsplits).and_then(|v| v.checked_add(1))
                    };

                    let result_array = if let Some(maxsplits) = maxsplits {
                        self.program.make_value_array(
                            str_value
                                .splitn(maxsplits, &*c_value)
                                .map(|s| ValueData::String(s.into())),
                        )
                    } else {
                        self.program.make_value_array(
                            str_value
                                .split(&*c_value)
                                .map(|s| ValueData::String(s.into())),
                        )
                    };

                    self.value_stack.push(ValueData::Array(result_array));
                }
                State::StdSplitLimitR => {
                    let maxsplits_value = self.value_stack.pop().unwrap();
                    let c_value = self.value_stack.pop().unwrap();
                    let str_value = self.value_stack.pop().unwrap();

                    let str_value = self.expect_std_func_arg_string(str_value, "splitLimitR", 0)?;
                    let c_value = self.expect_std_func_arg_string(c_value, "splitLimitR", 1)?;
                    let maxsplits =
                        self.expect_std_func_arg_number(maxsplits_value, "splitLimitR", 2)?;

                    if c_value.is_empty() {
                        return Err(self.report_error(EvalErrorKind::Other {
                            span: None,
                            message: "split delimiter is empty".into(),
                        }));
                    }

                    if !maxsplits.is_finite() || maxsplits.trunc() != maxsplits {
                        return Err(self.report_error(EvalErrorKind::Other {
                            span: None,
                            message: format!("`maxsplits` value {maxsplits} is not an integer"),
                        }));
                    }

                    let maxsplits = if maxsplits < 0.0 {
                        if maxsplits != -1.0 {
                            return Err(self.report_error(EvalErrorKind::Other {
                                span: None,
                                message: format!(
                                    "`maxsplits` value {maxsplits} is not -1 or non-negative"
                                ),
                            }));
                        }
                        None
                    } else {
                        float::try_to_usize(maxsplits).and_then(|v| v.checked_add(1))
                    };

                    let result_array = if let Some(maxsplits) = maxsplits {
                        let mut splitted: Vec<_> = str_value
                            .rsplitn(maxsplits, &*c_value)
                            .map(|s| ValueData::String(s.into()))
                            .collect();
                        splitted.reverse();
                        self.program.make_value_array(splitted)
                    } else {
                        self.program.make_value_array(
                            str_value
                                .split(&*c_value)
                                .map(|s| ValueData::String(s.into())),
                        )
                    };

                    self.value_stack.push(ValueData::Array(result_array));
                }
                State::StdStrReplace => {
                    let to = self.value_stack.pop().unwrap();
                    let from = self.value_stack.pop().unwrap();
                    let s = self.value_stack.pop().unwrap();

                    let s = self.expect_std_func_arg_string(s, "strReplace", 0)?;
                    let from = self.expect_std_func_arg_string(from, "strReplace", 1)?;
                    let to = self.expect_std_func_arg_string(to, "strReplace", 2)?;

                    let result = s.replace(&*from, &to);
                    self.value_stack.push(ValueData::String(result.into()));
                }
                State::StdAsciiUpper => {
                    let arg = self.value_stack.pop().unwrap();
                    let arg = self.expect_std_func_arg_string(arg, "asciiUpper", 0)?;
                    let r = arg.to_ascii_uppercase();
                    self.value_stack.push(ValueData::String(r.into()));
                }
                State::StdAsciiLower => {
                    let arg = self.value_stack.pop().unwrap();
                    let arg = self.expect_std_func_arg_string(arg, "asciiLower", 0)?;
                    let r = arg.to_ascii_lowercase();
                    self.value_stack.push(ValueData::String(r.into()));
                }
                State::StdStringChars => {
                    let arg = self.value_stack.pop().unwrap();
                    let arg = self.expect_std_func_arg_string(arg, "stringChars", 0)?;
                    let array = self.program.make_value_array(arg.chars().map(|c| {
                        let mut utf8_buf = [0; 4];
                        let utf8: &str = c.encode_utf8(&mut utf8_buf);
                        ValueData::String(utf8.into())
                    }));
                    self.value_stack.push(ValueData::Array(array));
                }
                State::StdFormat => {
                    let vals = self.value_stack.pop().unwrap();
                    let fmt = self.value_stack.pop().unwrap();

                    let fmt = self.expect_std_func_arg_string(fmt, "format", 0)?;
                    let fmt_parts = self.parse_format_codes(&fmt)?;

                    match vals {
                        ValueData::Array(array) => {
                            self.want_format_array(fmt_parts, array.view());
                        }
                        ValueData::Object(object) => {
                            self.want_format_object(fmt_parts, object.view());
                        }
                        val => {
                            let val = self.program.gc_alloc(ThunkData::new_done(val));
                            let array: GcView<Box<[_]>> =
                                self.program.gc_alloc_view(Box::new([val]));

                            self.want_format_array(fmt_parts, array);
                        }
                    }
                }
                State::StdFormatCodesArray1 {
                    parts,
                    array,
                    part_i,
                    array_i,
                } => {
                    self.format_codes_array_1(parts, array, part_i, array_i)?;
                }
                State::StdFormatCodesArray2 {
                    parts,
                    array,
                    part_i,
                    array_i,
                } => {
                    self.format_codes_array_2(parts, array, part_i, array_i)?;
                }
                State::StdFormatCodesArray3 {
                    parts,
                    array,
                    part_i,
                    array_i,
                    fw,
                } => {
                    self.format_codes_array_3(parts, array, part_i, array_i, fw)?;
                }
                State::StdFormatCodesObject1 {
                    parts,
                    object,
                    part_i,
                } => {
                    self.format_codes_object_1(parts, object, part_i)?;
                }
                State::StdFormatCodesObject2 {
                    parts,
                    object,
                    part_i,
                    fw,
                } => {
                    self.format_codes_object_2(parts, object, part_i, fw)?;
                }
                State::StdFormatCode {
                    parts,
                    part_i,
                    fw,
                    prec,
                } => {
                    self.format_code(&parts, part_i, fw, prec)?;
                }
                State::StdEscapeStringJson => {
                    let s = self.string_stack.pop().unwrap();
                    let mut escaped = String::new();
                    escape_string_json(&s, &mut escaped);
                    self.value_stack.push(ValueData::String(escaped.into()));
                }
                State::StdEscapeStringBash => {
                    let s = self.string_stack.pop().unwrap();
                    let mut escaped = String::new();
                    escaped.push('\'');
                    for chr in s.chars() {
                        if chr == '\'' {
                            escaped.push_str("'\"'\"'");
                        } else {
                            escaped.push(chr);
                        }
                    }
                    escaped.push('\'');
                    self.value_stack.push(ValueData::String(escaped.into()));
                }
                State::StdEscapeStringDollars => {
                    let s = self.string_stack.pop().unwrap();
                    let escaped = s.replace('$', "$$");
                    self.value_stack.push(ValueData::String(escaped.into()));
                }
                State::StdEscapeStringXml => {
                    let s = self.string_stack.pop().unwrap();
                    let mut escaped = String::new();
                    for chr in s.chars() {
                        match chr {
                            '<' => escaped.push_str("&lt;"),
                            '>' => escaped.push_str("&gt;"),
                            '&' => escaped.push_str("&amp;"),
                            '"' => escaped.push_str("&quot;"),
                            '\'' => escaped.push_str("&apos;"),
                            _ => escaped.push(chr),
                        }
                    }
                    self.value_stack.push(ValueData::String(escaped.into()));
                }
                State::StdParseInt => {
                    let arg = self.value_stack.pop().unwrap();
                    let s = self.expect_std_func_arg_string(arg, "parseInt", 0)?;
                    let sub_s = s.strip_prefix('-').unwrap_or(&s);
                    if sub_s.is_empty() {
                        return Err(self.report_error(EvalErrorKind::Other {
                            span: None,
                            message: format!("integer without digits: {s:?}"),
                        }));
                    }
                    for chr in sub_s.chars() {
                        if !chr.is_ascii_digit() {
                            return Err(self.report_error(EvalErrorKind::Other {
                                span: None,
                                message: format!("invalid base 10: {chr:?}"),
                            }));
                        }
                    }
                    let number = s.parse::<f64>().unwrap();
                    if !number.is_finite() {
                        return Err(self.report_error(EvalErrorKind::NumberOverflow { span: None }));
                    }
                    self.value_stack.push(ValueData::Number(number));
                }
                State::StdParseOctal => {
                    let arg = self.value_stack.pop().unwrap();
                    let s = self.expect_std_func_arg_string(arg, "parseOctal", 0)?;
                    if s.is_empty() {
                        return Err(self.report_error(EvalErrorKind::Other {
                            span: None,
                            message: format!("octal integer without digits: {s:?}"),
                        }));
                    }
                    let mut int = 0u128;
                    let mut chars = s.chars().peekable();
                    while let Some(chr) = chars.peek() {
                        let digit = chr.to_digit(8).ok_or_else(|| {
                            self.report_error(EvalErrorKind::Other {
                                span: None,
                                message: format!("invalid octal digit: {chr:?}"),
                            })
                        })?;
                        let new_int = int.checked_mul(8).and_then(|v| v.checked_add(digit.into()));
                        if let Some(new_int) = new_int {
                            int = new_int;
                            chars.next();
                        } else {
                            break;
                        }
                    }

                    let mut number = int as f64;
                    for chr in chars {
                        let digit = chr.to_digit(8).ok_or_else(|| {
                            self.report_error(EvalErrorKind::Other {
                                span: None,
                                message: format!("invalid octal digit: {chr:?}"),
                            })
                        })?;
                        number = number.mul_add(8.0, f64::from(digit));
                    }
                    if !number.is_finite() {
                        return Err(self.report_error(EvalErrorKind::NumberOverflow { span: None }));
                    }
                    self.value_stack.push(ValueData::Number(number));
                }
                State::StdParseHex => {
                    let arg = self.value_stack.pop().unwrap();
                    let s = self.expect_std_func_arg_string(arg, "parseHex", 0)?;
                    if s.is_empty() {
                        return Err(self.report_error(EvalErrorKind::Other {
                            span: None,
                            message: format!("hexadecimal integer without digits: {s:?}"),
                        }));
                    }
                    let mut int = 0u128;
                    let mut chars = s.chars().peekable();
                    while let Some(chr) = chars.peek() {
                        let digit = chr.to_digit(16).ok_or_else(|| {
                            self.report_error(EvalErrorKind::Other {
                                span: None,
                                message: format!("invalid hexadecimal digit: {chr:?}"),
                            })
                        })?;
                        let new_int = int
                            .checked_mul(16)
                            .and_then(|v| v.checked_add(digit.into()));
                        if let Some(new_int) = new_int {
                            int = new_int;
                            chars.next();
                        } else {
                            break;
                        }
                    }

                    let mut number = int as f64;
                    for chr in chars {
                        let digit = chr.to_digit(16).ok_or_else(|| {
                            self.report_error(EvalErrorKind::Other {
                                span: None,
                                message: format!("invalid hexadecimal digit: {chr:?}"),
                            })
                        })?;
                        number = number.mul_add(16.0, f64::from(digit));
                    }
                    if !number.is_finite() {
                        return Err(self.report_error(EvalErrorKind::NumberOverflow { span: None }));
                    }
                    self.value_stack.push(ValueData::Number(number));
                }
                State::StdParseJson => {
                    let arg = self.value_stack.pop().unwrap();
                    let s = self.expect_std_func_arg_string(arg, "parseJson", 0)?;
                    match parse_json::parse_json(self.program, &s) {
                        Ok(value) => {
                            self.value_stack.push(value);
                        }
                        Err(e) => {
                            return Err(self.report_error(EvalErrorKind::Other {
                                span: None,
                                message: format!("failed to parse JSON: {e}"),
                            }));
                        }
                    }
                }
                State::StdParseYaml => {
                    let arg = self.value_stack.pop().unwrap();
                    let s = self.expect_std_func_arg_string(arg, "parseYaml", 0)?;
                    match parse_yaml::parse_yaml(self.program, &s) {
                        Ok(value) => {
                            self.value_stack.push(value);
                        }
                        Err(e) => {
                            return Err(self.report_error(EvalErrorKind::Other {
                                span: None,
                                message: format!("failed to parse YAML: {e}"),
                            }));
                        }
                    }
                }
                State::StdEncodeUtf8 => {
                    let arg = self.value_stack.pop().unwrap();
                    let s = self.expect_std_func_arg_string(arg, "encodeUTF8", 0)?;
                    let array = self
                        .program
                        .make_value_array(s.bytes().map(|byte| ValueData::Number(f64::from(byte))));
                    self.value_stack.push(ValueData::Array(array));
                }
                State::StdDecodeUtf8 => {
                    let arg = self.value_stack.pop().unwrap();
                    let array = self.expect_std_func_arg_array(arg, "decodeUTF8", 0)?;

                    self.byte_array_stack.push(Vec::with_capacity(array.len()));
                    self.state_stack.push(State::StdDecodeUtf8Finish);
                    for item in array.iter().rev() {
                        self.state_stack.push(State::StdDecodeUtf8CheckItem);
                        self.state_stack.push(State::DoThunk(item.view()));
                    }
                }
                State::StdDecodeUtf8CheckItem => {
                    let item = self.value_stack.pop().unwrap();
                    let ValueData::Number(value) = item else {
                        return Err(self.report_error(EvalErrorKind::Other {
                            span: None,
                            message: format!(
                                "array item must be a number, got {}",
                                EvalErrorValueType::from_value(&item).to_str(),
                            ),
                        }));
                    };
                    let Some(byte) = float::try_to_u8_exact(value) else {
                        return Err(self.report_error(EvalErrorKind::Other {
                            span: None,
                            message: format!("array item value {value} is not a byte"),
                        }));
                    };
                    self.byte_array_stack.last_mut().unwrap().push(byte);
                }
                State::StdDecodeUtf8Finish => {
                    let bytes = self.byte_array_stack.pop().unwrap();
                    let s = String::from_utf8_lossy(&bytes);
                    self.value_stack.push(ValueData::String(s.into()));
                }
                State::StdManifestJsonEx => {
                    let key_val_sep = self.value_stack.pop().unwrap();
                    let newline = self.value_stack.pop().unwrap();
                    let indent = self.value_stack.pop().unwrap();

                    let indent = self.expect_std_func_arg_string(indent, "manifestJsonEx", 1)?;
                    let newline = self.expect_std_func_arg_string(newline, "manifestJsonEx", 2)?;
                    let key_val_sep =
                        self.expect_std_func_arg_string(key_val_sep, "manifestJsonEx", 3)?;

                    self.string_stack.push(String::new());
                    self.state_stack.push(State::StringToValue);
                    self.state_stack.push(State::ManifestJson {
                        format: ManifestJsonFormat::for_std_manifest_ex(
                            &indent,
                            &newline,
                            &key_val_sep,
                        ),
                        depth: 0,
                    });
                }
                State::StdMakeArray => {
                    let func_value = self.value_stack.pop().unwrap();
                    let sz_value = self.value_stack.pop().unwrap();

                    let length = self.expect_std_func_arg_number(sz_value, "makeArray", 0)?;
                    let func = self.expect_std_func_arg_func(func_value, "makeArray", 1)?;
                    let (_, func_params, _) = self.get_func_info(&func);

                    let Some(length) =
                        float::try_to_i32_exact(length).and_then(|v| usize::try_from(v).ok())
                    else {
                        return Err(self.report_error(EvalErrorKind::Other {
                            span: None,
                            message: format!("invalid size value {length}"),
                        }));
                    };

                    if func_params.order.len() != 1 {
                        return Err(self.report_error(EvalErrorKind::Other {
                            span: None,
                            message: format!(
                                "function must have exactly 1 parameter, got {}",
                                func_params.order.len(),
                            ),
                        }));
                    }

                    let mut array = Vec::with_capacity(length);
                    for i in 0..length {
                        let args_thunks = Box::new([self
                            .program
                            .gc_alloc(ThunkData::new_done(ValueData::Number(i as f64)))]);

                        array.push(
                            self.program.gc_alloc(ThunkData::new_pending_call(
                                Gc::from(&func),
                                args_thunks,
                            )),
                        );
                    }

                    self.value_stack.push(ValueData::Array(
                        self.program.gc_alloc(array.into_boxed_slice()),
                    ));
                }
                State::StdFilter => {
                    let arr_value = self.value_stack.pop().unwrap();
                    let func_value = self.value_stack.pop().unwrap();

                    let func = self.expect_std_func_arg_func(func_value, "filter", 0)?;
                    let array = self.expect_std_func_arg_array(arr_value, "filter", 1)?;

                    self.state_stack.push(State::ArrayToValue);
                    self.array_stack.push(Vec::new());

                    for item in array.iter().rev() {
                        self.state_stack
                            .push(State::StdFilterCheck { item: item.view() });
                        self.check_thunk_args_and_execute_call(&func, &[item.view()], &[], None)?;
                    }
                }
                State::StdFilterCheck { item } => {
                    let cond_value = self.value_stack.pop().unwrap();
                    let ValueData::Bool(cond_value) = cond_value else {
                        return Err(self.report_error(EvalErrorKind::Other {
                            span: None,
                            message: format!(
                                "filter function must return a boolean, got {}",
                                EvalErrorValueType::from_value(&cond_value).to_str(),
                            ),
                        }));
                    };

                    if cond_value {
                        self.array_stack.last_mut().unwrap().push(Gc::from(&item));
                    }
                }
                State::StdFoldl { init } => {
                    let arr_value = self.value_stack.pop().unwrap();
                    let func_value = self.value_stack.pop().unwrap();

                    let func = self.expect_std_func_arg_func(func_value, "foldl", 0)?;
                    let array = self.expect_std_func_arg_array(arr_value, "foldl", 1)?;

                    if array.is_empty() {
                        self.state_stack.push(State::DoThunk(init));
                    } else {
                        let (item0, rem_items) = array.split_first().unwrap();

                        for item in rem_items.iter().rev() {
                            self.state_stack.push(State::StdFoldlItem {
                                func: func.clone(),
                                item: item.view(),
                            });
                        }

                        self.check_thunk_args_and_execute_call(
                            &func,
                            &[init, item0.view()],
                            &[],
                            None,
                        )?;
                    }
                }
                State::StdFoldlItem { func, item } => {
                    let acc = self.value_stack.pop().unwrap();
                    let acc = self.program.gc_alloc_view(ThunkData::new_done(acc));

                    self.check_thunk_args_and_execute_call(&func, &[acc, item], &[], None)?;
                }
                State::StdFoldr { init } => {
                    let arr_value = self.value_stack.pop().unwrap();
                    let func_value = self.value_stack.pop().unwrap();

                    let func = self.expect_std_func_arg_func(func_value, "foldr", 0)?;
                    let array = self.expect_std_func_arg_array(arr_value, "foldr", 1)?;

                    if array.is_empty() {
                        self.state_stack.push(State::DoThunk(init));
                    } else {
                        let (last_item, rem_items) = array.split_last().unwrap();

                        for item in rem_items.iter() {
                            self.state_stack.push(State::StdFoldrItem {
                                func: func.clone(),
                                item: item.view(),
                            });
                        }

                        self.check_thunk_args_and_execute_call(
                            &func,
                            &[last_item.view(), init],
                            &[],
                            None,
                        )?;
                    }
                }
                State::StdFoldrItem { func, item } => {
                    let acc = self.value_stack.pop().unwrap();
                    let acc = self.program.gc_alloc_view(ThunkData::new_done(acc));

                    self.check_thunk_args_and_execute_call(&func, &[item, acc], &[], None)?;
                }
                State::StdRange => {
                    let to = self.value_stack.pop().unwrap();
                    let from = self.value_stack.pop().unwrap();

                    let from = self.expect_std_func_arg_number(from, "range", 0)?;
                    let to = self.expect_std_func_arg_number(to, "range", 1)?;

                    let from = float::try_to_i32_exact(from).ok_or_else(|| {
                        self.report_error(EvalErrorKind::Other {
                            span: None,
                            message: format!("invalid `from` value {from}"),
                        })
                    })?;
                    let to = float::try_to_i32_exact(to).ok_or_else(|| {
                        self.report_error(EvalErrorKind::Other {
                            span: None,
                            message: format!("invalid `to` value {to}"),
                        })
                    })?;

                    let array = self
                        .program
                        .make_value_array((from..=to).map(|i| ValueData::Number(f64::from(i))));

                    self.value_stack.push(ValueData::Array(array));
                }
                State::StdSlice => {
                    let step = self.value_stack.pop().unwrap();
                    let end = self.value_stack.pop().unwrap();
                    let start = self.value_stack.pop().unwrap();
                    let indexable = self.value_stack.pop().unwrap();

                    let start = self.expect_std_func_arg_null_or_number(start, "slice", 1)?;
                    let end = self.expect_std_func_arg_null_or_number(end, "slice", 2)?;
                    let step = self.expect_std_func_arg_null_or_number(step, "slice", 3)?;

                    let start = if let Some(start) = start {
                        if !start.is_finite() || start.trunc() != start || start < 0.0 {
                            return Err(self.report_error(EvalErrorKind::Other {
                                span: None,
                                message: format!(
                                    "slice start {start} is not a non-negative integer"
                                ),
                            }));
                        }
                        start as usize
                    } else {
                        0
                    };
                    let end = if let Some(end) = end {
                        if !end.is_finite() || end.trunc() != end || end < 0.0 {
                            return Err(self.report_error(EvalErrorKind::Other {
                                span: None,
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
                                span: None,
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
                        }
                        ValueData::Array(array) => {
                            let array = array.view();
                            let result = self.program.slice_array(&array, start, end, step);
                            self.value_stack.push(ValueData::Array(result));
                        }
                        _ => {
                            return Err(self.report_error(EvalErrorKind::InvalidStdFuncArgType {
                                func_name: "slice".into(),
                                arg_index: 0,
                                expected_types: vec![
                                    EvalErrorValueType::String,
                                    EvalErrorValueType::Array,
                                ],
                                got_type: EvalErrorValueType::from_value(&indexable),
                            }));
                        }
                    }
                }
                State::StdJoin => {
                    let arr = self.value_stack.pop().unwrap();
                    let sep = self.value_stack.pop().unwrap();

                    let array = self.expect_std_func_arg_array(arr, "join", 1)?;

                    match sep {
                        ValueData::String(sep) => {
                            self.bool_stack.push(true);
                            self.state_stack.push(State::StdJoinStrFinish);
                            self.string_stack.push(String::new());
                            for item in array.iter().rev() {
                                self.state_stack
                                    .push(State::StdJoinStrItem { sep: sep.clone() });
                                self.state_stack.push(State::DoThunk(item.view()));
                            }
                        }
                        ValueData::Array(sep) => {
                            let sep = sep.view();
                            self.bool_stack.push(true);
                            self.state_stack.push(State::StdJoinArrayFinish);
                            self.array_stack.push(Vec::new());
                            for item in array.iter().rev() {
                                self.state_stack
                                    .push(State::StdJoinArrayItem { sep: sep.clone() });
                                self.state_stack.push(State::DoThunk(item.view()));
                            }
                        }
                        _ => {
                            return Err(self.report_error(EvalErrorKind::InvalidStdFuncArgType {
                                func_name: "join".into(),
                                arg_index: 0,
                                expected_types: vec![
                                    EvalErrorValueType::String,
                                    EvalErrorValueType::Array,
                                ],
                                got_type: EvalErrorValueType::from_value(&sep),
                            }));
                        }
                    }
                }
                State::StdJoinStrItem { sep } => {
                    let item = self.value_stack.pop().unwrap();
                    if !matches!(item, ValueData::Null) {
                        let ValueData::String(item) = item else {
                            return Err(self.report_error(EvalErrorKind::Other {
                                span: None,
                                message: format!(
                                    "array item must null or string, got {}",
                                    EvalErrorValueType::from_value(&item).to_str(),
                                ),
                            }));
                        };
                        let s = self.string_stack.last_mut().unwrap();
                        let first = self.bool_stack.last_mut().unwrap();
                        if *first {
                            *first = false;
                        } else {
                            s.push_str(&sep);
                        }
                        s.push_str(&item);
                    }
                }
                State::StdJoinStrFinish => {
                    let s = self.string_stack.pop().unwrap();
                    self.bool_stack.pop().unwrap();
                    self.value_stack.push(ValueData::String(s.into()));
                }
                State::StdJoinArrayItem { sep } => {
                    let item = self.value_stack.pop().unwrap();
                    if !matches!(item, ValueData::Null) {
                        let ValueData::Array(item) = item else {
                            return Err(self.report_error(EvalErrorKind::Other {
                                span: None,
                                message: format!(
                                    "array item must null or array, got {}",
                                    EvalErrorValueType::from_value(&item).to_str(),
                                ),
                            }));
                        };
                        let item = item.view();
                        let array = self.array_stack.last_mut().unwrap();
                        let first = self.bool_stack.last_mut().unwrap();
                        if *first {
                            *first = false;
                        } else {
                            array.extend(sep.iter().cloned());
                        }
                        array.extend(item.iter().cloned());
                    }
                }
                State::StdJoinArrayFinish => {
                    let array = self.array_stack.pop().unwrap();
                    self.bool_stack.pop().unwrap();
                    self.value_stack.push(ValueData::Array(
                        self.program.gc_alloc(array.into_boxed_slice()),
                    ));
                }
                State::StdSort => {
                    let keyf = self.value_stack.pop().unwrap();
                    let arr = self.value_stack.pop().unwrap();

                    let array = self.expect_std_func_arg_array(arr, "sort", 0)?;
                    let keyf = self.expect_std_func_arg_func(keyf, "sort", 1)?;

                    let array_len = array.len();
                    if array_len <= 1 {
                        self.value_stack.push(ValueData::Array(Gc::from(&array)));
                    } else {
                        let keys: Rc<Vec<_>> =
                            Rc::new((0..array_len).map(|_| OnceCell::new()).collect());

                        let sorted: Rc<Vec<_>> = Rc::new((0..array_len).map(Cell::new).collect());
                        self.state_stack.push(State::StdSortFinish {
                            orig_array: array.clone(),
                            sorted: sorted.clone(),
                        });

                        self.state_stack.push(State::StdSortSlice {
                            keys: keys.clone(),
                            sorted,
                            range: 0..array_len,
                        });

                        for i in (0..array_len).rev() {
                            self.state_stack.push(State::StdSortSetKey {
                                keys: keys.clone(),
                                index: i,
                            });
                            self.check_thunk_args_and_execute_call(
                                &keyf,
                                &[array[i].view()],
                                &[],
                                None,
                            )?;
                        }
                    }
                }
                State::StdSortSetKey { keys, index } => {
                    let value = self.value_stack.pop().unwrap();
                    keys[index].set(value).ok().unwrap();
                }
                State::StdSortCompare { keys, lhs, rhs } => {
                    self.value_stack.push(keys[lhs].get().unwrap().clone());
                    self.value_stack.push(keys[rhs].get().unwrap().clone());
                    self.state_stack.push(State::CompareValue);
                }
                State::StdSortSlice {
                    keys,
                    sorted,
                    range,
                } => {
                    let len = range.end - range.start;
                    if len > 30 {
                        // Merge sort
                        let mid = range.start + len / 2;
                        self.state_stack.push(State::StdSortMergePrepare {
                            keys: keys.clone(),
                            sorted: sorted.clone(),
                            range: range.clone(),
                            mid,
                        });
                        self.state_stack.push(State::StdSortSlice {
                            keys: keys.clone(),
                            sorted: sorted.clone(),
                            range: mid..range.end,
                        });
                        self.state_stack.push(State::StdSortSlice {
                            keys,
                            sorted,
                            range: range.start..mid,
                        });
                    } else if (range.end - range.start) > 1 {
                        // Quick sort
                        self.state_stack.push(State::StdSortQuickSort1 {
                            keys,
                            sorted,
                            range,
                        });
                    }
                }
                State::StdSortQuickSort1 {
                    keys,
                    sorted,
                    range,
                } => {
                    assert!((range.end - range.start) > 1);

                    self.state_stack.push(State::StdSortQuickSort2 {
                        keys: keys.clone(),
                        sorted: sorted.clone(),
                        range: range.clone(),
                    });

                    let pivot = sorted[range.start].get();
                    for i in ((range.start + 1)..range.end).rev() {
                        let item = sorted[i].get();
                        self.state_stack.push(State::StdSortCompare {
                            keys: keys.clone(),
                            lhs: item,
                            rhs: pivot,
                        });
                    }
                }
                State::StdSortQuickSort2 {
                    keys,
                    sorted,
                    range,
                } => {
                    let len = range.end - range.start;
                    assert!(len > 1);
                    let lenm1 = len - 1;
                    let mut items_lt = Vec::with_capacity(lenm1);
                    let mut items_ge = Vec::with_capacity(lenm1);
                    for (i, item_ord) in self
                        .cmp_ord_stack
                        .drain(self.cmp_ord_stack.len() - lenm1..)
                        .enumerate()
                    {
                        let item = sorted[range.start + 1 + i].get();
                        if item_ord.is_lt() {
                            items_lt.push(item);
                        } else {
                            items_ge.push(item);
                        }
                    }

                    let pivot = sorted[range.start].get();

                    let mut i = range.start;
                    for &item in items_lt.iter() {
                        sorted[i].set(item);
                        i += 1;
                    }
                    sorted[i].set(pivot);
                    i += 1;
                    for &item in items_ge.iter() {
                        sorted[i].set(item);
                        i += 1;
                    }

                    if items_ge.len() > 1 {
                        self.state_stack.push(State::StdSortQuickSort1 {
                            keys: keys.clone(),
                            sorted: sorted.clone(),
                            range: (range.start + items_lt.len() + 1)..range.end,
                        });
                    }
                    if items_lt.len() > 1 {
                        self.state_stack.push(State::StdSortQuickSort1 {
                            keys: keys.clone(),
                            sorted: sorted.clone(),
                            range: range.start..(range.start + items_lt.len()),
                        });
                    }
                }
                State::StdSortMergePrepare {
                    keys,
                    sorted,
                    range,
                    mid,
                } => {
                    let left = sorted[range.start..mid].iter().map(Cell::get).collect();
                    let right = sorted[mid..range.end].iter().map(Cell::get).collect();

                    self.state_stack.push(State::StdSortMergePreCompare {
                        keys,
                        sorted,
                        start: range.start,
                        unmerged: Rc::new((Cell::new(0), left, Cell::new(0), right)),
                    });
                }
                State::StdSortMergePreCompare {
                    keys,
                    sorted,
                    start,
                    unmerged,
                } => {
                    let (left_i, left, right_i, right) = &*unmerged;
                    if left_i.get() == left.len() {
                        let array_i = start + left_i.get() + right_i.get();
                        for (sub_i, &item) in right[right_i.get()..].iter().enumerate() {
                            sorted[array_i + sub_i].set(item);
                        }
                    } else if right_i.get() == right.len() {
                        let array_i = start + left_i.get() + right_i.get();
                        for (sub_i, &item) in left[left_i.get()..].iter().enumerate() {
                            sorted[array_i + sub_i].set(item);
                        }
                    } else {
                        self.state_stack.push(State::StdSortMergePostCompare {
                            keys: keys.clone(),
                            sorted,
                            start,
                            unmerged: unmerged.clone(),
                        });

                        self.value_stack
                            .push(keys[left[left_i.get()]].get().unwrap().clone());
                        self.value_stack
                            .push(keys[right[right_i.get()]].get().unwrap().clone());
                        self.state_stack.push(State::CompareValue);
                    }
                }
                State::StdSortMergePostCompare {
                    keys,
                    sorted,
                    start,
                    unmerged,
                } => {
                    let cmp_ord = self.cmp_ord_stack.pop().unwrap();

                    let (left_i, left, right_i, right) = &*unmerged;
                    let array_i = start + left_i.get() + right_i.get();

                    if cmp_ord.is_le() {
                        sorted[array_i].set(left[left_i.get()]);
                        left_i.set(left_i.get() + 1);
                    } else {
                        sorted[array_i].set(right[right_i.get()]);
                        right_i.set(right_i.get() + 1);
                    }

                    self.state_stack.push(State::StdSortMergePreCompare {
                        keys,
                        sorted,
                        start,
                        unmerged,
                    });
                }
                State::StdSortFinish { orig_array, sorted } => {
                    let new_array = sorted.iter().map(|i| orig_array[i.get()].clone()).collect();

                    self.value_stack
                        .push(ValueData::Array(self.program.gc_alloc(new_array)));
                }
                State::StdUniq => {
                    let keyf = self.value_stack.pop().unwrap();
                    let arr = self.value_stack.pop().unwrap();

                    let array = self.expect_std_func_arg_array(arr, "uniq", 0)?;
                    let keyf = self.expect_std_func_arg_func(keyf, "uniq", 1)?;

                    if array.len() <= 1 {
                        self.value_stack.push(ValueData::Array(Gc::from(&array)));
                    } else {
                        let mut new_array = Vec::with_capacity(array.len());
                        new_array.push(array[0].clone());
                        self.array_stack.push(new_array);
                        self.state_stack.push(State::ArrayToValue);

                        for (i, item) in array[1..].iter().rev().enumerate() {
                            self.state_stack.push(State::StdUniqCompareItem {
                                keyf: keyf.clone(),
                                item: item.view(),
                                is_last: i == 0,
                            });
                        }

                        self.check_thunk_args_and_execute_call(
                            &keyf,
                            &[array[0].view()],
                            &[],
                            None,
                        )?;
                    }
                }
                State::StdUniqCompareItem {
                    keyf,
                    item,
                    is_last,
                } => {
                    self.state_stack
                        .push(State::StdUniqCheckItem { item: item.clone() });
                    self.state_stack.push(State::EqualsValue);
                    if !is_last {
                        self.state_stack.push(State::StdUniqDupValue);
                    }
                    self.check_thunk_args_and_execute_call(&keyf, &[item], &[], None)?;
                }
                State::StdUniqDupValue => {
                    let value = self.value_stack.last().unwrap().clone();
                    self.value_stack.insert(self.value_stack.len() - 2, value);
                }
                State::StdUniqCheckItem { item } => {
                    let is_equal = self.bool_stack.pop().unwrap();
                    if !is_equal {
                        let array = self.array_stack.last_mut().unwrap();
                        array.push(Gc::from(&item));
                    }
                }
                State::StdAll => {
                    let arr = self.value_stack.pop().unwrap();

                    let array = self.expect_std_func_arg_array(arr, "all", 0)?;

                    if array.is_empty() {
                        self.value_stack.push(ValueData::Bool(true));
                    } else {
                        let item_thunk = array[0].view();
                        self.state_stack.push(State::StdAllItem { array, index: 0 });
                        self.state_stack.push(State::DoThunk(item_thunk));
                    }
                }
                State::StdAllItem { array, index } => {
                    let item_value = self.value_stack.pop().unwrap();
                    let ValueData::Bool(item_value) = item_value else {
                        return Err(self.report_error(EvalErrorKind::Other {
                            span: None,
                            message: format!(
                                "array item {index} must be a boolean, got {}",
                                EvalErrorValueType::from_value(&item_value).to_str(),
                            ),
                        }));
                    };

                    if item_value {
                        let index = index + 1;
                        if index == array.len() {
                            self.value_stack.push(ValueData::Bool(true));
                        } else {
                            let item_thunk = array[index].view();
                            self.state_stack.push(State::StdAllItem { array, index });
                            self.state_stack.push(State::DoThunk(item_thunk));
                        }
                    } else {
                        self.value_stack.push(ValueData::Bool(false));
                    }
                }
                State::StdAny => {
                    let arr = self.value_stack.pop().unwrap();

                    let array = self.expect_std_func_arg_array(arr, "any", 0)?;

                    if array.is_empty() {
                        self.value_stack.push(ValueData::Bool(false));
                    } else {
                        let item_thunk = array[0].view();
                        self.state_stack.push(State::StdAnyItem { array, index: 0 });
                        self.state_stack.push(State::DoThunk(item_thunk));
                    }
                }
                State::StdAnyItem { array, index } => {
                    let item_value = self.value_stack.pop().unwrap();
                    let ValueData::Bool(item_value) = item_value else {
                        return Err(self.report_error(EvalErrorKind::Other {
                            span: None,
                            message: format!(
                                "array item {index} must be a boolean, got {}",
                                EvalErrorValueType::from_value(&item_value).to_str(),
                            ),
                        }));
                    };

                    if item_value {
                        self.value_stack.push(ValueData::Bool(true));
                    } else {
                        let index = index + 1;
                        if index == array.len() {
                            self.value_stack.push(ValueData::Bool(false));
                        } else {
                            let item_thunk = array[index].view();
                            self.state_stack.push(State::StdAnyItem { array, index });
                            self.state_stack.push(State::DoThunk(item_thunk));
                        }
                    }
                }
                State::StdSet => {
                    let keyf = self.value_stack.pop().unwrap();
                    let arr = self.value_stack.pop().unwrap();

                    let array = self.expect_std_func_arg_array(arr, "set", 0)?;
                    let keyf = self.expect_std_func_arg_func(keyf, "set", 1)?;

                    let array_len = array.len();
                    if array_len <= 1 {
                        self.value_stack.push(ValueData::Array(Gc::from(&array)));
                    } else {
                        // Sort and then uniq, reusing the keyF evaluations from sort

                        let keys: Rc<Vec<_>> =
                            Rc::new((0..array_len).map(|_| OnceCell::new()).collect());

                        let sorted: Rc<Vec<_>> = Rc::new((0..array_len).map(Cell::new).collect());

                        // Uniq
                        self.state_stack.push(State::StdSetUniq {
                            orig_array: array.clone(),
                            keys: keys.clone(),
                            sorted: sorted.clone(),
                        });

                        // Sort
                        self.state_stack.push(State::StdSortSlice {
                            keys: keys.clone(),
                            sorted,
                            range: 0..array_len,
                        });

                        // Calculate keys
                        for i in (0..array_len).rev() {
                            self.state_stack.push(State::StdSortSetKey {
                                keys: keys.clone(),
                                index: i,
                            });
                            self.check_thunk_args_and_execute_call(
                                &keyf,
                                &[array[i].view()],
                                &[],
                                None,
                            )?;
                        }
                    }
                }
                State::StdSetUniq {
                    orig_array,
                    keys,
                    sorted,
                } => {
                    let mut new_array = Vec::with_capacity(orig_array.len());
                    new_array.push(orig_array[sorted[0].get()].clone());
                    self.array_stack.push(new_array);
                    self.state_stack.push(State::ArrayToValue);

                    for i in (1..orig_array.len()).rev() {
                        self.state_stack.push(State::StdSetUniqCompareItem {
                            orig_array: orig_array.clone(),
                            keys: keys.clone(),
                            sorted: sorted.clone(),
                            index: i,
                        });
                    }
                }
                State::StdSetUniqCompareItem {
                    orig_array,
                    keys,
                    sorted,
                    index,
                } => {
                    self.state_stack.push(State::StdSetUniqCheckItem {
                        orig_array,
                        sorted: sorted.clone(),
                        index,
                    });
                    self.state_stack.push(State::EqualsValue);
                    self.value_stack
                        .push(keys[sorted[index - 1].get()].get().unwrap().clone());
                    self.value_stack
                        .push(keys[sorted[index].get()].get().unwrap().clone());
                }
                State::StdSetUniqCheckItem {
                    orig_array,
                    sorted,
                    index,
                } => {
                    let is_equal = self.bool_stack.pop().unwrap();
                    if !is_equal {
                        let array = self.array_stack.last_mut().unwrap();
                        array.push(orig_array[sorted[index].get()].clone());
                    }
                }
                State::StdSetInter => {
                    let keyf = self.value_stack.pop().unwrap();
                    let b = self.value_stack.pop().unwrap();
                    let a = self.value_stack.pop().unwrap();

                    let a = self.expect_std_func_arg_array(a, "setInter", 0)?;
                    let b = self.expect_std_func_arg_array(b, "setInter", 1)?;
                    let keyf = self.expect_std_func_arg_func(keyf, "setInter", 2)?;

                    if a.is_empty() || b.is_empty() {
                        self.value_stack
                            .push(ValueData::Array(Gc::from(&self.program.empty_array)));
                    } else {
                        self.array_stack.push(Vec::new());
                        self.state_stack.push(State::StdSetInterAux {
                            keyf: keyf.clone(),
                            a: a.clone(),
                            b: b.clone(),
                            i: 0,
                            j: 0,
                        });
                        self.state_stack.push(State::CompareValue);
                        self.check_thunk_args_and_execute_call(&keyf, &[b[0].view()], &[], None)?;
                        self.check_thunk_args_and_execute_call(&keyf, &[a[0].view()], &[], None)?;
                    }
                }
                State::StdSetInterAux {
                    keyf,
                    a,
                    b,
                    mut i,
                    mut j,
                } => {
                    let cmp_ord = self.cmp_ord_stack.pop().unwrap();
                    let result_array = self.array_stack.last_mut().unwrap();
                    match cmp_ord {
                        std::cmp::Ordering::Less => {
                            i += 1;
                        }
                        std::cmp::Ordering::Equal => {
                            result_array.push(a[i].clone());
                            i += 1;
                            j += 1;
                        }
                        std::cmp::Ordering::Greater => {
                            j += 1;
                        }
                    }

                    if i == a.len() || j == b.len() {
                        self.state_stack.push(State::ArrayToValue);
                    } else {
                        self.state_stack.push(State::StdSetInterAux {
                            keyf: keyf.clone(),
                            a: a.clone(),
                            b: b.clone(),
                            i,
                            j,
                        });
                        self.state_stack.push(State::CompareValue);
                        // FIXME: We could avoid redundant calls to keyf
                        self.check_thunk_args_and_execute_call(&keyf, &[b[j].view()], &[], None)?;
                        self.check_thunk_args_and_execute_call(&keyf, &[a[i].view()], &[], None)?;
                    }
                }
                State::StdSetUnion => {
                    let keyf = self.value_stack.pop().unwrap();
                    let b = self.value_stack.pop().unwrap();
                    let a = self.value_stack.pop().unwrap();

                    let a = self.expect_std_func_arg_array(a, "setUnion", 0)?;
                    let b = self.expect_std_func_arg_array(b, "setUnion", 1)?;
                    let keyf = self.expect_std_func_arg_func(keyf, "setUnion", 2)?;

                    if a.is_empty() {
                        self.value_stack.push(ValueData::Array(Gc::from(&b)));
                    } else if b.is_empty() {
                        self.value_stack.push(ValueData::Array(Gc::from(&a)));
                    } else {
                        self.array_stack.push(Vec::new());
                        self.state_stack.push(State::StdSetUnionAux {
                            keyf: keyf.clone(),
                            a: a.clone(),
                            b: b.clone(),
                            i: 0,
                            j: 0,
                        });
                        self.state_stack.push(State::CompareValue);
                        self.check_thunk_args_and_execute_call(&keyf, &[b[0].view()], &[], None)?;
                        self.check_thunk_args_and_execute_call(&keyf, &[a[0].view()], &[], None)?;
                    }
                }
                State::StdSetUnionAux {
                    keyf,
                    a,
                    b,
                    mut i,
                    mut j,
                } => {
                    let cmp_ord = self.cmp_ord_stack.pop().unwrap();
                    let result_array = self.array_stack.last_mut().unwrap();
                    match cmp_ord {
                        std::cmp::Ordering::Less => {
                            result_array.push(a[i].clone());
                            i += 1;
                        }
                        std::cmp::Ordering::Equal => {
                            result_array.push(a[i].clone());
                            i += 1;
                            j += 1;
                        }
                        std::cmp::Ordering::Greater => {
                            result_array.push(b[j].clone());
                            j += 1;
                        }
                    }

                    if i == a.len() {
                        result_array.extend(b[j..].iter().cloned());
                        self.state_stack.push(State::ArrayToValue);
                    } else if j == b.len() {
                        result_array.extend(a[i..].iter().cloned());
                        self.state_stack.push(State::ArrayToValue);
                    } else {
                        self.state_stack.push(State::StdSetUnionAux {
                            keyf: keyf.clone(),
                            a: a.clone(),
                            b: b.clone(),
                            i,
                            j,
                        });
                        self.state_stack.push(State::CompareValue);
                        // FIXME: We could avoid redundant calls to keyf
                        self.check_thunk_args_and_execute_call(&keyf, &[b[j].view()], &[], None)?;
                        self.check_thunk_args_and_execute_call(&keyf, &[a[i].view()], &[], None)?;
                    }
                }
                State::StdSetDiff => {
                    let keyf = self.value_stack.pop().unwrap();
                    let b = self.value_stack.pop().unwrap();
                    let a = self.value_stack.pop().unwrap();

                    let a = self.expect_std_func_arg_array(a, "setDiff", 0)?;
                    let b = self.expect_std_func_arg_array(b, "setDiff", 1)?;
                    let keyf = self.expect_std_func_arg_func(keyf, "setDiff", 2)?;

                    if a.is_empty() || b.is_empty() {
                        self.value_stack.push(ValueData::Array(Gc::from(&a)));
                    } else {
                        self.array_stack.push(Vec::new());
                        self.state_stack.push(State::StdSetDiffAux {
                            keyf: keyf.clone(),
                            a: a.clone(),
                            b: b.clone(),
                            i: 0,
                            j: 0,
                        });
                        self.state_stack.push(State::CompareValue);
                        self.check_thunk_args_and_execute_call(&keyf, &[b[0].view()], &[], None)?;
                        self.check_thunk_args_and_execute_call(&keyf, &[a[0].view()], &[], None)?;
                    }
                }
                State::StdSetDiffAux {
                    keyf,
                    a,
                    b,
                    mut i,
                    mut j,
                } => {
                    let cmp_ord = self.cmp_ord_stack.pop().unwrap();
                    let result_array = self.array_stack.last_mut().unwrap();
                    match cmp_ord {
                        std::cmp::Ordering::Less => {
                            result_array.push(a[i].clone());
                            i += 1;
                        }
                        std::cmp::Ordering::Equal => {
                            i += 1;
                            j += 1;
                        }
                        std::cmp::Ordering::Greater => {
                            j += 1;
                        }
                    }

                    if i == a.len() {
                        self.state_stack.push(State::ArrayToValue);
                    } else if j == b.len() {
                        result_array.extend(a[i..].iter().cloned());
                        self.state_stack.push(State::ArrayToValue);
                    } else {
                        self.state_stack.push(State::StdSetDiffAux {
                            keyf: keyf.clone(),
                            a: a.clone(),
                            b: b.clone(),
                            i,
                            j,
                        });
                        self.state_stack.push(State::CompareValue);
                        // FIXME: We could avoid redundant calls to keyf
                        self.check_thunk_args_and_execute_call(&keyf, &[b[j].view()], &[], None)?;
                        self.check_thunk_args_and_execute_call(&keyf, &[a[i].view()], &[], None)?;
                    }
                }
                State::StdSetMember { x } => {
                    let keyf = self.value_stack.pop().unwrap();
                    let arr = self.value_stack.pop().unwrap();

                    let arr = self.expect_std_func_arg_array(arr, "setMember", 1)?;
                    let keyf = self.expect_std_func_arg_func(keyf, "setMember", 2)?;

                    if arr.is_empty() {
                        self.value_stack.push(ValueData::Bool(false));
                    } else {
                        let start = 0;
                        let end = arr.len() - 1;
                        self.state_stack.push(State::StdSetMemberSlice {
                            keyf: keyf.clone(),
                            arr,
                            start,
                            end,
                        });

                        self.check_thunk_args_and_execute_call(&keyf, &[x], &[], None)?;
                    }
                }
                State::StdSetMemberSlice {
                    keyf,
                    arr,
                    start,
                    end,
                } => {
                    let mid = start + (end - start) / 2;

                    self.value_stack
                        .push(self.value_stack.last().unwrap().clone());
                    self.state_stack.push(State::StdSetMemberCheck {
                        keyf: keyf.clone(),
                        arr: arr.clone(),
                        start,
                        end,
                        mid,
                    });
                    self.state_stack.push(State::CompareValue);
                    self.check_thunk_args_and_execute_call(&keyf, &[arr[mid].view()], &[], None)?;
                }
                State::StdSetMemberCheck {
                    keyf,
                    arr,
                    start,
                    end,
                    mid,
                } => {
                    let cmp_ord = self.cmp_ord_stack.pop().unwrap();
                    match cmp_ord {
                        std::cmp::Ordering::Equal => {
                            self.value_stack.pop().unwrap();
                            self.value_stack.push(ValueData::Bool(true));
                        }
                        std::cmp::Ordering::Less => {
                            if mid == start {
                                self.value_stack.pop().unwrap();
                                self.value_stack.push(ValueData::Bool(false));
                            } else {
                                self.state_stack.push(State::StdSetMemberSlice {
                                    keyf: keyf.clone(),
                                    arr,
                                    start,
                                    end: mid - 1,
                                });
                            }
                        }
                        std::cmp::Ordering::Greater => {
                            if mid == end {
                                self.value_stack.pop().unwrap();
                                self.value_stack.push(ValueData::Bool(false));
                            } else {
                                self.state_stack.push(State::StdSetMemberSlice {
                                    keyf: keyf.clone(),
                                    arr,
                                    start: mid + 1,
                                    end,
                                });
                            }
                        }
                    }
                }
                State::StdMd5 => {
                    let arg = self.value_stack.pop().unwrap();
                    let arg = self.expect_std_func_arg_string(arg, "md5", 0)?;

                    use md5::Digest as _;
                    let hash = md5::Md5::digest(arg.as_bytes());

                    let mut hash_string = String::with_capacity(32);
                    for &byte in hash.iter() {
                        write!(hash_string, "{byte:02x}").unwrap();
                    }

                    self.value_stack.push(ValueData::String(hash_string.into()));
                }
                State::StdNative => {
                    let arg = self.value_stack.pop().unwrap();
                    let name = self.expect_std_func_arg_string(arg, "native", 0)?;
                    let value = self
                        .program
                        .str_interner
                        .get_interned(&name)
                        .and_then(|name| self.program.native_funcs.get(&name))
                        .map_or(ValueData::Null, |func| ValueData::Function(Gc::from(func)));

                    self.value_stack.push(value);
                }
                State::StdTrace => {
                    let arg = self.value_stack.pop().unwrap();
                    let msg = self.expect_std_func_arg_string(arg, "trace", 0)?;

                    let stack = self.get_stack_trace();
                    if let Some(callbacks) = self.callbacks.as_deref_mut() {
                        callbacks.trace(self.program, &msg, &stack);
                    }
                }
            }

            if self.stack_trace_len > self.program.max_stack {
                return Err(self.report_error(EvalErrorKind::StackOverflow));
            }

            self.program.maybe_gc();
        }

        Ok(())
    }

    #[inline]
    fn push_trace_item(&mut self, item: TraceItem) {
        self.state_stack.push(State::TraceItem(item));
        self.inc_trace_len();
    }

    #[inline]
    fn delay_trace_item(&mut self) {
        self.state_stack.push(State::DelayedTraceItem);
        self.dec_trace_len();
    }

    #[inline]
    fn inc_trace_len(&mut self) {
        self.stack_trace_len += 1;
    }

    #[inline]
    fn dec_trace_len(&mut self) {
        self.stack_trace_len = self.stack_trace_len.checked_sub(1).unwrap();
    }

    fn get_stdlib_field(&self, name: &InternedStr) -> GcView<ThunkData> {
        let stdlib_obj = self.program.stdlib_obj.as_ref().unwrap();
        self.program
            .find_object_field_thunk(stdlib_obj, 0, name)
            .unwrap()
    }

    fn try_value_from_expr(program: &Program, expr: &ir::Expr) -> Option<ValueData> {
        match *expr {
            ir::Expr::Null => Some(ValueData::Null),
            ir::Expr::Bool(value) => Some(ValueData::Bool(value)),
            ir::Expr::Number(value, _) if value.is_finite() => Some(ValueData::Number(value)),
            ir::Expr::String(ref s) => Some(ValueData::String(s.clone())),
            ir::Expr::Array(ref items) if items.is_empty() => {
                Some(ValueData::Array(Gc::from(&program.empty_array)))
            }
            _ => None,
        }
    }

    fn new_pending_expr_thunk(&self, expr: Rc<ir::Expr>, env: Gc<ThunkEnv>) -> Gc<ThunkData> {
        let thunk = if let Some(value) = Self::try_value_from_expr(self.program, &expr) {
            ThunkData::new_done(value)
        } else {
            ThunkData::new_pending_expr(expr, env)
        };
        self.program.gc_alloc(thunk)
    }

    fn add_object_field(
        &mut self,
        name: InternedStr,
        name_span: SpanId,
        plus: bool,
        visibility: ast::Visibility,
        value: Rc<ir::Expr>,
        base_env: Option<Gc<ThunkEnv>>,
    ) -> Result<(), EvalError> {
        let object = self.object_stack.last_mut().unwrap();
        match object.self_core.fields.entry(name.clone()) {
            HashMapEntry::Occupied(_) => Err(self.report_error(EvalErrorKind::RepeatedFieldName {
                span: name_span,
                name: name.value().into(),
            })),
            HashMapEntry::Vacant(entry) => {
                let actual_expr;
                let thunk;
                if plus {
                    actual_expr = Some(Rc::new(ir::Expr::FieldPlus {
                        field_name: name,
                        field_expr: value,
                    }));
                    thunk = OnceCell::new();
                } else if let Some(value) = Self::try_value_from_expr(self.program, &value) {
                    actual_expr = None;
                    thunk = OnceCell::from(self.program.gc_alloc(ThunkData::new_done(value)));
                } else {
                    actual_expr = Some(value.clone());
                    thunk = OnceCell::new();
                }

                entry.insert(ObjectField {
                    base_env,
                    visibility,
                    expr: actual_expr,
                    thunk,
                });
                Ok(())
            }
        }
    }

    fn check_object_asserts(&mut self, object: &GcView<ObjectData>) {
        if !object.asserts_checked.get() {
            object.asserts_checked.set(true);
            let core_iter = object
                .super_cores
                .iter()
                .rev()
                .chain(std::iter::once(&object.self_core));
            for (i, core) in core_iter.enumerate() {
                for (assert_i, assert) in core.asserts.iter().enumerate().rev() {
                    let env = self.program.get_object_assert_env(
                        object,
                        object.super_cores.len() - i,
                        assert_i,
                    );
                    self.state_stack.push(State::Assert {
                        assert_span: assert.assert_span,
                        cond_span: assert.cond_span,
                        msg_expr: assert.msg.as_ref().map(|e| (e.clone(), env.clone())),
                    });
                    self.state_stack.push(State::Expr {
                        expr: assert.cond.clone(),
                        env,
                    });
                }
            }
        }
    }

    fn check_number_value(&mut self, value: f64, span: Option<SpanId>) -> Result<(), EvalError> {
        match value.classify() {
            std::num::FpCategory::Nan => Err(self.report_error(EvalErrorKind::NumberNan { span })),
            std::num::FpCategory::Infinite => {
                Err(self.report_error(EvalErrorKind::NumberOverflow { span }))
            }
            std::num::FpCategory::Zero
            | std::num::FpCategory::Subnormal
            | std::num::FpCategory::Normal => Ok(()),
        }
    }

    #[inline]
    fn expect_std_func_arg_bool(
        &self,
        value: ValueData,
        func_name: &str,
        arg_index: usize,
    ) -> Result<bool, EvalError> {
        match value {
            ValueData::Bool(b) => Ok(b),
            value => Err(self.report_error(EvalErrorKind::InvalidStdFuncArgType {
                func_name: func_name.into(),
                arg_index,
                expected_types: vec![EvalErrorValueType::Bool],
                got_type: EvalErrorValueType::from_value(&value),
            })),
        }
    }

    #[inline]
    fn expect_std_func_arg_number(
        &self,
        value: ValueData,
        func_name: &str,
        arg_index: usize,
    ) -> Result<f64, EvalError> {
        match value {
            ValueData::Number(n) => Ok(n),
            value => Err(self.report_error(EvalErrorKind::InvalidStdFuncArgType {
                func_name: func_name.into(),
                arg_index,
                expected_types: vec![EvalErrorValueType::Number],
                got_type: EvalErrorValueType::from_value(&value),
            })),
        }
    }

    #[inline]
    fn expect_std_func_arg_null_or_number(
        &self,
        value: ValueData,
        func_name: &str,
        arg_index: usize,
    ) -> Result<Option<f64>, EvalError> {
        match value {
            ValueData::Null => Ok(None),
            ValueData::Number(n) => Ok(Some(n)),
            value => Err(self.report_error(EvalErrorKind::InvalidStdFuncArgType {
                func_name: func_name.into(),
                arg_index,
                expected_types: vec![EvalErrorValueType::Null, EvalErrorValueType::Number],
                got_type: EvalErrorValueType::from_value(&value),
            })),
        }
    }

    #[inline]
    fn expect_std_func_arg_string(
        &self,
        value: ValueData,
        func_name: &str,
        arg_index: usize,
    ) -> Result<Rc<str>, EvalError> {
        match value {
            ValueData::String(s) => Ok(s),
            value => Err(self.report_error(EvalErrorKind::InvalidStdFuncArgType {
                func_name: func_name.into(),
                arg_index,
                expected_types: vec![EvalErrorValueType::String],
                got_type: EvalErrorValueType::from_value(&value),
            })),
        }
    }

    #[inline]
    fn expect_std_func_arg_array(
        &self,
        value: ValueData,
        func_name: &str,
        arg_index: usize,
    ) -> Result<GcView<ArrayData>, EvalError> {
        match value {
            ValueData::Array(arr) => Ok(arr.view()),
            value => Err(self.report_error(EvalErrorKind::InvalidStdFuncArgType {
                func_name: func_name.into(),
                arg_index,
                expected_types: vec![EvalErrorValueType::Array],
                got_type: EvalErrorValueType::from_value(&value),
            })),
        }
    }

    #[inline]
    fn expect_std_func_arg_object(
        &self,
        value: ValueData,
        func_name: &str,
        arg_index: usize,
    ) -> Result<GcView<ObjectData>, EvalError> {
        match value {
            ValueData::Object(obj) => Ok(obj.view()),
            value => Err(self.report_error(EvalErrorKind::InvalidStdFuncArgType {
                func_name: func_name.into(),
                arg_index,
                expected_types: vec![EvalErrorValueType::Object],
                got_type: EvalErrorValueType::from_value(&value),
            })),
        }
    }

    #[inline]
    fn expect_std_func_arg_func(
        &self,
        value: ValueData,
        func_name: &str,
        arg_index: usize,
    ) -> Result<GcView<FuncData>, EvalError> {
        match value {
            ValueData::Function(func) => Ok(func.view()),
            value => Err(self.report_error(EvalErrorKind::InvalidStdFuncArgType {
                func_name: func_name.into(),
                arg_index,
                expected_types: vec![EvalErrorValueType::Function],
                got_type: EvalErrorValueType::from_value(&value),
            })),
        }
    }

    #[cold]
    #[must_use]
    fn report_error(&self, error_kind: EvalErrorKind) -> EvalError {
        EvalError {
            stack_trace: self.get_stack_trace(),
            kind: error_kind,
        }
    }

    fn get_stack_trace(&self) -> Vec<EvalStackTraceItem> {
        let mut stack_trace = Vec::new();
        for stack_item in self.state_stack.iter() {
            fn conv_trace_item(item: &TraceItem) -> EvalStackTraceItem {
                match *item {
                    TraceItem::Expr { span } => EvalStackTraceItem::Expr { span },
                    TraceItem::Call { span, ref name } => EvalStackTraceItem::Call {
                        span,
                        name: name.as_ref().map(|s| s.value().into()),
                    },
                    TraceItem::Variable { span, ref name } => EvalStackTraceItem::Variable {
                        span,
                        name: name.value().into(),
                    },
                    TraceItem::ArrayItem { span, index } => {
                        EvalStackTraceItem::ArrayItem { span, index }
                    }
                    TraceItem::ObjectField { span, ref name } => EvalStackTraceItem::ObjectField {
                        span,
                        name: name.value().into(),
                    },
                    TraceItem::Import { span } => EvalStackTraceItem::Import { span },
                }
            }

            match stack_item {
                State::TraceItem(trace_item) => {
                    stack_trace.push(conv_trace_item(trace_item));
                }
                State::DelayedTraceItem => {
                    stack_trace.pop().unwrap();
                }
                _ => {}
            }
        }
        stack_trace
    }
}
