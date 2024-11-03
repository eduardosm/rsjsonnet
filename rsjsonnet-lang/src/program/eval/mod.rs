use std::cell::{Cell, OnceCell};
use std::collections::hash_map::Entry as HashMapEntry;
use std::rc::Rc;

use super::{
    ir, ArrayData, Callbacks, EvalError, EvalErrorKind, EvalErrorValueType, EvalStackTraceItem,
    FuncData, FuncKind, ObjectData, ObjectField, ObjectLayer, PendingThunk, Program, ThunkData,
    ThunkEnv, ThunkEnvData, ThunkState, ValueData,
};
use crate::gc::{Gc, GcView};
use crate::interner::InternedStr;
use crate::span::SpanId;
use crate::{ast, float, FHashMap};

mod call;
mod expr;
mod format;
mod manifest;
mod parse_json;
mod parse_yaml;
mod state;
mod stdlib;

use manifest::ManifestJsonFormat;
use state::State;

// `EvalError` is boxed to reduce the size of `Result`s returned by internal
// functions.
type EvalResult<T> = Result<T, Box<EvalError>>;

pub(super) struct Evaluator<'a, 'p> {
    program: &'a mut Program<'p>,
    callbacks: Option<&'a mut dyn Callbacks<'p>>,
    stack_trace_len: usize,
    state_stack: Vec<State<'p>>,
    value_stack: Vec<ValueData<'p>>,
    bool_stack: Vec<bool>,
    string_stack: Vec<String>,
    array_stack: Vec<Vec<Gc<ThunkData<'p>>>>,
    object_stack: Vec<ObjectData<'p>>,
    comp_spec_stack: Vec<CompSpec<'p>>,
    cmp_ord_stack: Vec<std::cmp::Ordering>,
    byte_array_stack: Vec<Vec<u8>>,
    output: EvalOutput<'p>,
}

pub(super) enum EvalInput<'p> {
    Value(GcView<ThunkData<'p>>),
    Call(GcView<ThunkData<'p>>, TopLevelArgs<'p>),
    ManifestJson(GcView<ThunkData<'p>>, bool),
}

#[must_use]
pub(super) enum EvalOutput<'p> {
    Nothing,
    Value(ValueData<'p>),
    String(String),
}

pub(super) struct TopLevelArgs<'p> {
    pub(super) positional: Box<[GcView<ThunkData<'p>>]>,
    pub(super) named: Box<[(InternedStr<'p>, GcView<ThunkData<'p>>)]>,
}

#[derive(Clone)]
enum TraceItem<'p> {
    Expr {
        span: SpanId,
    },
    Call {
        span: Option<SpanId>,
        name: Option<InternedStr<'p>>,
    },
    Variable {
        span: SpanId,
        name: InternedStr<'p>,
    },
    ArrayItem {
        span: Option<SpanId>,
        index: usize,
    },
    ObjectField {
        span: Option<SpanId>,
        name: InternedStr<'p>,
    },
    CompareArrayItem {
        index: usize,
    },
    CompareObjectField {
        name: InternedStr<'p>,
    },
    ManifestArrayItem {
        index: usize,
    },
    ManifestObjectField {
        name: InternedStr<'p>,
    },
    Import {
        span: SpanId,
    },
}

struct CompSpec<'p> {
    vars: Vec<FHashMap<InternedStr<'p>, GcView<ThunkData<'p>>>>,
}

impl<'p, 'a> Evaluator<'a, 'p> {
    pub(super) fn eval(
        program: &'a mut Program<'p>,
        callbacks: Option<&'a mut dyn Callbacks<'p>>,
        input: EvalInput<'p>,
    ) -> EvalResult<EvalOutput<'p>> {
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
                this.state_stack.push(State::OutputValue);
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

    fn run(&mut self) -> EvalResult<()> {
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
                            PendingThunk::FieldPlus { expr, field, env } => {
                                let env = env.view();
                                let (object, layer_i) = env.get_object();
                                let object = object.view();
                                if let Some(super_field) = self.program.find_object_field_thunk(
                                    &object,
                                    layer_i + 1,
                                    field,
                                ) {
                                    self.state_stack.push(State::BinaryOp {
                                        span: None,
                                        op: ast::BinaryOp::Add,
                                    });
                                    self.state_stack.push(State::Expr { expr, env });
                                    self.want_thunk_direct(super_field, || {
                                        TraceItem::ObjectField {
                                            span: None,
                                            name: field,
                                        }
                                    });
                                } else {
                                    self.state_stack.push(State::Expr { expr, env });
                                }
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
                    fn might_need_deep(thunk: &ThunkData<'_>) -> bool {
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
                        for &(field_name, field_visible) in object.get_fields_order().iter().rev() {
                            if field_visible {
                                let thunk = self
                                    .program
                                    .find_object_field_thunk(&object, 0, field_name)
                                    .unwrap();
                                if might_need_deep(&thunk) {
                                    self.push_trace_item(TraceItem::ObjectField {
                                        span: None,
                                        name: field_name,
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
                State::CoerceAppendToString => {
                    if let ValueData::String(s) = self.value_stack.last().unwrap() {
                        self.string_stack.last_mut().unwrap().push_str(s);
                        self.value_stack.pop().unwrap();
                    } else {
                        self.state_stack.push(State::ManifestJson {
                            format: ManifestJsonFormat::default_to_string(),
                            depth: 0,
                        });
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
                State::ObjectToValue => {
                    let object = self.object_stack.pop().unwrap();
                    let object = self.program.gc_alloc(object);
                    self.value_stack.push(ValueData::Object(object));
                }
                State::Slice {
                    span,
                    has_start,
                    has_end,
                    has_step,
                } => {
                    let step = if has_step {
                        self.value_stack.pop().unwrap()
                    } else {
                        ValueData::Null
                    };
                    let end = if has_end {
                        self.value_stack.pop().unwrap()
                    } else {
                        ValueData::Null
                    };
                    let start = if has_start {
                        self.value_stack.pop().unwrap()
                    } else {
                        ValueData::Null
                    };
                    let indexable = self.value_stack.pop().unwrap();

                    let start = match start {
                        ValueData::Null => None,
                        ValueData::Number(start) => Some(start),
                        _ => {
                            return Err(self.report_error(
                                EvalErrorKind::SliceIndexOrStepIsNotNumber {
                                    span,
                                    got_type: EvalErrorValueType::from_value(&start),
                                },
                            ));
                        }
                    };
                    let end = match end {
                        ValueData::Null => None,
                        ValueData::Number(end) => Some(end),
                        _ => {
                            return Err(self.report_error(
                                EvalErrorKind::SliceIndexOrStepIsNotNumber {
                                    span,
                                    got_type: EvalErrorValueType::from_value(&end),
                                },
                            ));
                        }
                    };
                    let step = match step {
                        ValueData::Null => None,
                        ValueData::Number(step) => Some(step),
                        _ => {
                            return Err(self.report_error(
                                EvalErrorKind::SliceIndexOrStepIsNotNumber {
                                    span,
                                    got_type: EvalErrorValueType::from_value(&step),
                                },
                            ));
                        }
                    };

                    self.do_slice(indexable, start, end, step, false, Some(span))?;
                }
                State::ManifestIniSection => self.do_manifest_ini_section()?,
                State::ManifestIniSectionItem { name } => {
                    self.do_manifest_ini_section_item(name)?
                }
                State::ManifestPython => self.do_manifest_python()?,
                State::ManifestJson { format, depth } => self.do_manifest_json(format, depth)?,
                State::ManifestYamlDoc {
                    indent_array_in_object,
                    quote_keys,
                    depth,
                    parent_is_array,
                    parent_is_object,
                } => self.do_manifest_yaml_doc(
                    indent_array_in_object,
                    quote_keys,
                    depth,
                    parent_is_array,
                    parent_is_object,
                )?,
                State::ManifestTomlPeekSubTable => self.do_manifest_toml_peek_sub_table(),
                State::ManifestTomlPeekSubTableArrayItem { array, index } => {
                    self.do_manifest_toml_peek_sub_table_array_item(array, index)
                }
                State::ManifestTomlTable {
                    object,
                    has_header,
                    path,
                    indent,
                } => self.do_manifest_toml_table(object, has_header, path, indent),
                State::ManifestTomlValue {
                    indent,
                    depth,
                    single_line,
                } => self.do_manifest_toml_value(indent, depth, single_line)?,
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
                        let name = self.program.intern_str(&name);
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
                        let mut vars = FHashMap::default();
                        vars.insert(var_name, item.view());
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
                            inner_env_data.set_var(*var_name, Gc::from(var_value));
                        }
                        let inner_env = self.program.gc_alloc_view(ThunkEnv::from(inner_env_data));
                        self.state_stack.push(State::Expr {
                            expr: value,
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
                            new_vars.insert(var_name, item.view());
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
                            inner_env_data.set_var(*var_name, Gc::from(var_value));
                        }
                        let inner_env = self.program.gc_alloc_view(ThunkEnv::from(inner_env_data));
                        self.state_stack.push(State::Expr {
                            expr: cond,
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
                                item_env_data.set_var(*var_name, Gc::from(var_value));
                            }
                            let item_env = self.program.gc_alloc(ThunkEnv::from(item_env_data));
                            array.push(self.program.new_pending_expr_thunk(item, item_env, None));
                        }
                        self.value_stack.push(ValueData::Array(
                            self.program.gc_alloc(array.into_boxed_slice()),
                        ));
                    }
                }
                State::ObjectComp { expr, env } => {
                    let ir::Expr::ObjectComp {
                        is_top,
                        locals: ir_locals,
                        field_name,
                        field_name_span,
                        field_value,
                        ..
                    } = *expr
                    else {
                        unreachable!();
                    };

                    let comp_spec = self.comp_spec_stack.pop().unwrap();

                    self.object_stack.push(ObjectData {
                        self_layer: ObjectLayer {
                            is_top,
                            locals: ir_locals,
                            base_env: None,
                            env: OnceCell::new(),
                            fields: FHashMap::default(),
                            asserts: &[],
                        },
                        super_layers: Vec::new(),
                        fields_order: OnceCell::new(),
                        asserts_checked: Cell::new(true),
                    });

                    self.state_stack.push(State::FinishObjectComp);

                    for vars in comp_spec.vars.iter().rev() {
                        let mut item_env_data = ThunkEnvData::new(Some(Gc::from(&env)));
                        for (var_name, var_value) in vars.iter() {
                            item_env_data.set_var(*var_name, Gc::from(var_value));
                        }
                        let outer_env = self.program.gc_alloc_view(ThunkEnv::from(item_env_data));

                        self.state_stack.push(State::ObjectDynField {
                            name_span: field_name_span,
                            plus: false,
                            visibility: ast::Visibility::Default,
                            value: field_value,
                            base_env: Some(Gc::from(&outer_env)),
                        });
                        self.state_stack.push(State::Expr {
                            expr: field_name,
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
                        self.want_field(&object.view(), field_name, span)?;
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
                                self.value_stack.push(ValueData::from_char(chr));
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
                                self.want_thunk_direct(item.view(), || TraceItem::ArrayItem {
                                    span: Some(span),
                                    index: index_usize,
                                });
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
                                self.want_field(&object.view(), field_name, span)?;
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
                        self.want_super_field(&env, super_span, field_name, span)?;
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
                            let (object, layer_i) = env.get_object();
                            let object = object.view();
                            object.has_field(layer_i + 1, field_name)
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
                                self.push_trace_item(TraceItem::CompareArrayItem { index: 0 });
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
                                .filter_map(|(name, visible)| visible.then_some(name))
                                .cloned()
                                .collect();
                            let rhs_fields: Vec<_> = rhs
                                .get_fields_order()
                                .iter()
                                .rev()
                                .filter_map(|(name, visible)| visible.then_some(name))
                                .cloned()
                                .collect();

                            if lhs_fields == rhs_fields {
                                let mut fields = lhs_fields;
                                if let Some(first_field) = fields.pop() {
                                    let lhs_field = self
                                        .program
                                        .find_object_field_thunk(&lhs, 0, first_field)
                                        .unwrap();
                                    let rhs_field = self
                                        .program
                                        .find_object_field_thunk(&rhs, 0, first_field)
                                        .unwrap();
                                    self.state_stack.push(State::EqualsObject {
                                        lhs: lhs.clone(),
                                        rhs: rhs.clone(),
                                        rem_fields: fields,
                                    });
                                    self.push_trace_item(TraceItem::CompareObjectField {
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
                            self.push_trace_item(TraceItem::CompareArrayItem { index });
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
                                .find_object_field_thunk(&lhs, 0, next_field)
                                .unwrap();
                            let rhs_field = self
                                .program
                                .find_object_field_thunk(&rhs, 0, next_field)
                                .unwrap();
                            self.state_stack.push(State::EqualsObject {
                                lhs,
                                rhs,
                                rem_fields,
                            });
                            self.push_trace_item(TraceItem::CompareObjectField {
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
                                    self.push_trace_item(TraceItem::CompareArrayItem { index: 0 });
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
                                self.push_trace_item(TraceItem::CompareArrayItem { index });
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
                        positional_args,
                        named_args,
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
                    let (func_name, func_env) = self.get_func_info(&func);
                    let args_thunks = self.check_call_expr_args(
                        &func.params,
                        positional_args,
                        named_args,
                        call_env,
                        func_env,
                        call_span,
                    )?;
                    match func.kind {
                        FuncKind::Normal { .. } if tailstrict => {
                            let params_order = func.params.order;
                            self.state_stack.push(State::ExecTailstrictCall {
                                func,
                                args: args_thunks.clone(),
                            });

                            for (i, arg_thunk) in args_thunks.iter().enumerate().rev() {
                                self.push_trace_item(TraceItem::Variable {
                                    span: call_span,
                                    name: params_order[i].0,
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
                    match callbacks.native_call(self.program, name, &args) {
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
                State::StdExtVar => self.do_std_ext_var()?,
                State::StdType => self.do_std_type(),
                State::StdIsArray => self.do_std_is_array(),
                State::StdIsBoolean => self.do_std_is_boolean(),
                State::StdIsFunction => self.do_std_is_function(),
                State::StdIsNumber => self.do_std_is_number(),
                State::StdIsObject => self.do_std_is_object(),
                State::StdIsString => self.do_std_is_string(),
                State::StdLength => self.do_std_length()?,
                State::StdPruneValue => self.do_std_prune_value(),
                State::StdPruneArrayItem => self.do_std_prune_array_item(),
                State::StdPruneObjectField { name } => self.do_std_prune_object_field(name),
                State::StdObjectHasEx => self.do_std_object_has_ex()?,
                State::StdObjectFieldsEx => self.do_std_object_fields_ex()?,
                State::StdPrimitiveEquals => self.do_std_primitive_equals()?,
                State::StdCompareArray => self.do_std_compare_array()?,
                State::StdExponent => self.do_std_exponent()?,
                State::StdMantissa => self.do_std_mantissa()?,
                State::StdFloor => self.do_std_floor()?,
                State::StdCeil => self.do_std_ceil()?,
                State::StdModulo => self.do_std_modulo()?,
                State::StdPow => self.do_std_pow()?,
                State::StdExp => self.do_std_exp()?,
                State::StdLog => self.do_std_log()?,
                State::StdSqrt => self.do_std_sqrt()?,
                State::StdSin => self.do_std_sin()?,
                State::StdCos => self.do_std_cos()?,
                State::StdTan => self.do_std_tan()?,
                State::StdAsin => self.do_std_asin()?,
                State::StdAcos => self.do_std_acos()?,
                State::StdAtan => self.do_std_atan()?,
                State::StdAssertEqual => self.do_std_assert_equal(),
                State::StdAssertEqualCheck => self.do_std_assert_equal_check(),
                State::StdAssertEqualFail1 => self.do_std_assert_equal_fail_1(),
                State::StdAssertEqualFail2 => self.do_std_assert_equal_fail_2()?,
                State::StdCodepoint => self.do_std_codepoint()?,
                State::StdChar => self.do_std_char()?,
                State::StdSubstr => self.do_std_substr()?,
                State::StdFindSubstr => self.do_std_find_substr()?,
                State::StdStartsWith => self.do_std_starts_with()?,
                State::StdEndsWith => self.do_std_ends_with()?,
                State::StdSplit => self.do_std_split()?,
                State::StdSplitLimit => self.do_std_split_limit()?,
                State::StdSplitLimitR => self.do_std_split_limit_r()?,
                State::StdStrReplace => self.do_std_str_replace()?,
                State::StdAsciiUpper => self.do_std_ascii_upper()?,
                State::StdAsciiLower => self.do_std_ascii_lower()?,
                State::StdStringChars => self.do_std_string_chars()?,
                State::StdFormat => self.do_std_format()?,
                State::StdFormatCodesArray1 {
                    parts,
                    array,
                    part_i,
                    array_i,
                } => self.do_std_format_codes_array_1(parts, array, part_i, array_i)?,
                State::StdFormatCodesArray2 {
                    parts,
                    array,
                    part_i,
                    array_i,
                } => self.do_std_format_codes_array_2(parts, array, part_i, array_i)?,
                State::StdFormatCodesArray3 {
                    parts,
                    array,
                    part_i,
                    array_i,
                    fw,
                } => self.do_std_format_codes_array_3(parts, array, part_i, array_i, fw)?,
                State::StdFormatCodesObject1 {
                    parts,
                    object,
                    part_i,
                } => self.do_std_format_codes_object_1(parts, object, part_i)?,
                State::StdFormatCodesObject2 {
                    parts,
                    object,
                    part_i,
                    fw,
                } => self.do_std_format_codes_object_2(parts, object, part_i, fw)?,
                State::StdFormatCode {
                    parts,
                    part_i,
                    fw,
                    prec,
                } => self.do_std_format_code(&parts, part_i, fw, prec)?,
                State::StdEscapeStringJson => self.do_std_escape_string_json(),
                State::StdEscapeStringPython => self.do_std_escape_string_python(),
                State::StdEscapeStringBash => self.do_std_escape_string_bash(),
                State::StdEscapeStringDollars => self.do_std_escape_string_dollars(),
                State::StdEscapeStringXml => self.do_std_escape_string_xml(),
                State::StdParseInt => self.do_std_parse_int()?,
                State::StdParseOctal => self.do_std_parse_octal()?,
                State::StdParseHex => self.do_std_parse_hex()?,
                State::StdParseJson => self.do_std_parse_json()?,
                State::StdParseYaml => self.do_std_parse_yaml()?,
                State::StdEncodeUtf8 => self.do_std_encode_utf8()?,
                State::StdDecodeUtf8 => self.do_std_decode_utf8()?,
                State::StdDecodeUtf8CheckItem => self.do_std_decode_utf8_check_item()?,
                State::StdDecodeUtf8Finish => self.do_std_decode_utf8_finish(),
                State::StdManifestIni => self.do_std_manifest_ini()?,
                State::StdManifestIniSections => self.do_std_manifest_ini_sections()?,
                State::StdManifestPython => self.do_std_manifest_python(),
                State::StdManifestPythonVars => self.do_std_manifest_python_vars()?,
                State::StdManifestJsonEx => self.do_std_manifest_json_ex()?,
                State::StdManifestYamlDoc => self.do_std_manifest_yaml_doc()?,
                State::StdManifestYamlStream => self.do_std_manifest_yaml_stream()?,
                State::StdManifestXmlJsonml => self.do_std_manifest_xml_jsonml()?,
                State::StdManifestXmlJsonmlItem0 { array } => {
                    self.do_std_manifest_xml_jsonml_item_0(array)?
                }
                State::StdManifestXmlJsonmlItem1 => self.do_std_manifest_xml_jsonml_item_1()?,
                State::StdManifestXmlJsonmlItemN => self.do_std_manifest_xml_jsonml_item_n()?,
                State::StdManifestTomlEx => self.do_std_manifest_toml_ex()?,
                State::StdMakeArray => self.do_std_make_array()?,
                State::StdMember { value } => self.do_std_member(value)?,
                State::StdMemberString { string } => self.do_std_member_string(string)?,
                State::StdMemberArray { array, index } => self.do_std_member_array(array, index),
                State::StdCount { value } => self.do_std_count(value)?,
                State::StdCountInner {
                    array,
                    index,
                    count,
                } => self.do_std_count_inner(array, index, count),
                State::StdFind { value } => self.do_std_find(value)?,
                State::StdFindInner { array, index } => self.do_std_find_inner(array, index),
                State::StdMap => self.do_std_map()?,
                State::StdMapWithIndex => self.do_std_map_with_index()?,
                State::StdFilter => self.do_std_filter()?,
                State::StdFilterCheck { item } => self.do_std_filter_check(item)?,
                State::StdFoldl { init } => self.do_std_foldl(init)?,
                State::StdFoldlItem { func, item } => self.do_std_foldl_item(func, item)?,
                State::StdFoldr { init } => self.do_std_foldr(init)?,
                State::StdFoldrItem { func, item } => self.do_std_foldr_item(func, item)?,
                State::StdRange => self.do_std_range()?,
                State::StdSlice => self.do_std_slice()?,
                State::StdJoin => self.do_std_join()?,
                State::StdJoinStrItem { sep } => self.do_std_join_str_item(sep)?,
                State::StdJoinStrFinish => self.do_std_join_str_finish(),
                State::StdJoinArrayItem { sep } => self.do_std_join_array_item(sep)?,
                State::StdJoinArrayFinish => self.do_std_join_array_finish(),
                State::StdReverse => self.do_std_reverse()?,
                State::StdSort => self.do_std_sort()?,
                State::StdSortSetKey { keys, index } => self.do_std_sort_set_key(keys, index),
                State::StdSortCompare { keys, lhs, rhs } => {
                    self.do_std_sort_compare(keys, lhs, rhs)
                }
                State::StdSortSlice {
                    keys,
                    sorted,
                    range,
                } => self.do_std_sort_slice(keys, sorted, range),
                State::StdSortQuickSort1 {
                    keys,
                    sorted,
                    range,
                } => self.do_std_sort_quick_sort_1(keys, sorted, range),
                State::StdSortQuickSort2 {
                    keys,
                    sorted,
                    range,
                } => self.do_std_sort_quick_sort_2(keys, sorted, range),
                State::StdSortMergePrepare {
                    keys,
                    sorted,
                    range,
                    mid,
                } => self.do_std_sort_merge_prepare(keys, sorted, range, mid),
                State::StdSortMergePreCompare {
                    keys,
                    sorted,
                    start,
                    unmerged,
                } => self.do_std_sort_merge_pre_compare(keys, sorted, start, unmerged),
                State::StdSortMergePostCompare {
                    keys,
                    sorted,
                    start,
                    unmerged,
                } => self.do_std_sort_merge_post_compare(keys, sorted, start, unmerged),
                State::StdSortFinish { orig_array, sorted } => {
                    self.do_std_sort_finish(orig_array, sorted)
                }
                State::StdUniq => self.do_std_uniq()?,
                State::StdUniqCompareItem {
                    keyf,
                    item,
                    is_last,
                } => self.do_std_uniq_compare_item(keyf, item, is_last)?,
                State::StdUniqDupValue => self.do_std_uniq_dup_value(),
                State::StdUniqCheckItem { item } => self.do_std_uniq_check_item(item),
                State::StdAll => self.do_std_all()?,
                State::StdAllItem { array, index } => self.do_std_all_item(array, index)?,
                State::StdAny => self.do_std_any()?,
                State::StdAnyItem { array, index } => self.do_std_any_item(array, index)?,
                State::StdSet => self.do_std_set()?,
                State::StdSetUniq {
                    orig_array,
                    keys,
                    sorted,
                } => self.do_std_set_uniq(orig_array, keys, sorted),
                State::StdSetUniqCompareItem {
                    orig_array,
                    keys,
                    sorted,
                    index,
                } => self.do_std_set_uniq_compare_item(orig_array, keys, sorted, index),
                State::StdSetUniqCheckItem {
                    orig_array,
                    sorted,
                    index,
                } => self.do_std_set_uniq_check_item(orig_array, sorted, index),
                State::StdSetInter => self.do_std_set_inter()?,
                State::StdSetInterAux { keyf, a, b, i, j } => {
                    self.do_std_set_inter_aux(keyf, a, b, i, j)?
                }
                State::StdSetUnion => self.do_std_set_union()?,
                State::StdSetUnionAux { keyf, a, b, i, j } => {
                    self.do_std_set_union_aux(keyf, a, b, i, j)?
                }
                State::StdSetDiff => self.do_std_set_diff()?,
                State::StdSetDiffAux { keyf, a, b, i, j } => {
                    self.do_std_set_diff_aux(keyf, a, b, i, j)?
                }
                State::StdSetMember { x } => self.do_std_set_member(x)?,
                State::StdSetMemberSlice {
                    keyf,
                    arr,
                    start,
                    end,
                } => self.do_std_set_member_slice(keyf, arr, start, end)?,
                State::StdSetMemberCheck {
                    keyf,
                    arr,
                    start,
                    end,
                    mid,
                } => self.do_std_set_member_check(keyf, arr, start, end, mid)?,
                State::StdBase64 => self.do_std_base64()?,
                State::StdBase64Array { input, bytes } => self.do_std_base64_array(input, bytes)?,
                State::StdBase64DecodeBytes => self.do_std_base64_decode_bytes()?,
                State::StdBase64Decode => self.do_std_base64_decode()?,
                State::StdMd5 => self.do_std_md5()?,
                State::StdNative => self.do_std_native()?,
                State::StdTrace => self.do_std_trace()?,
                State::StdMod => self.do_std_mod()?,
            }

            if self.stack_trace_len > self.program.max_stack {
                return Err(self.report_error(EvalErrorKind::StackOverflow));
            }

            self.program.maybe_gc();
        }

        Ok(())
    }

    #[inline]
    fn push_trace_item(&mut self, item: TraceItem<'p>) {
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

    #[inline]
    fn want_thunk_direct(
        &mut self,
        thunk: GcView<ThunkData<'p>>,
        trace_item: impl FnOnce() -> TraceItem<'p>,
    ) {
        if let Some(value) = thunk.get_value() {
            self.value_stack.push(value);
        } else {
            self.push_trace_item(trace_item());
            self.state_stack.push(State::DoThunk(thunk));
        }
    }

    fn add_object_field(
        &mut self,
        name: InternedStr<'p>,
        name_span: SpanId,
        plus: bool,
        visibility: ast::Visibility,
        value: &'p ir::Expr<'p>,
        base_env: Option<Gc<ThunkEnv<'p>>>,
    ) -> EvalResult<()> {
        let object = self.object_stack.last_mut().unwrap();
        match object.self_layer.fields.entry(name) {
            HashMapEntry::Occupied(_) => Err(self.report_error(EvalErrorKind::RepeatedFieldName {
                span: name_span,
                name: name.value().into(),
            })),
            HashMapEntry::Vacant(entry) => {
                let actual_expr;
                let thunk;
                if plus {
                    actual_expr = Some((value, true));
                    thunk = OnceCell::new();
                } else if let Some(value) = self.program.try_value_from_expr(value) {
                    actual_expr = None;
                    thunk = OnceCell::from(self.program.gc_alloc(ThunkData::new_done(value)));
                } else {
                    actual_expr = Some((value, false));
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

    fn check_object_asserts(&mut self, object: &GcView<ObjectData<'p>>) {
        if !object.asserts_checked.get() {
            object.asserts_checked.set(true);
            let layer_iter = object
                .super_layers
                .iter()
                .rev()
                .chain(std::iter::once(&object.self_layer));
            for (i, layer) in layer_iter.enumerate() {
                for (assert_i, assert) in layer.asserts.iter().enumerate().rev() {
                    let env = self.program.get_object_assert_env(
                        object,
                        object.super_layers.len() - i,
                        assert_i,
                    );
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
            }
        }
    }

    fn check_number_value(&mut self, value: f64, span: Option<SpanId>) -> EvalResult<()> {
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
        value: ValueData<'p>,
        func_name: &str,
        arg_index: usize,
    ) -> EvalResult<bool> {
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
        value: ValueData<'p>,
        func_name: &str,
        arg_index: usize,
    ) -> EvalResult<f64> {
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
        value: ValueData<'p>,
        func_name: &str,
        arg_index: usize,
    ) -> EvalResult<Option<f64>> {
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
        value: ValueData<'p>,
        func_name: &str,
        arg_index: usize,
    ) -> EvalResult<Rc<str>> {
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
        value: ValueData<'p>,
        func_name: &str,
        arg_index: usize,
    ) -> EvalResult<GcView<ArrayData<'p>>> {
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
        value: ValueData<'p>,
        func_name: &str,
        arg_index: usize,
    ) -> EvalResult<GcView<ObjectData<'p>>> {
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
        value: ValueData<'p>,
        func_name: &str,
        arg_index: usize,
    ) -> EvalResult<GcView<FuncData<'p>>> {
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
    fn report_error(&self, error_kind: EvalErrorKind) -> Box<EvalError> {
        Box::new(EvalError {
            stack_trace: self.get_stack_trace(),
            kind: error_kind,
        })
    }

    fn get_stack_trace(&self) -> Vec<EvalStackTraceItem> {
        let mut stack_trace = Vec::new();
        for stack_item in self.state_stack.iter() {
            fn conv_trace_item(item: &TraceItem<'_>) -> EvalStackTraceItem {
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
                    TraceItem::CompareArrayItem { index } => {
                        EvalStackTraceItem::CompareArrayItem { index }
                    }
                    TraceItem::CompareObjectField { ref name } => {
                        EvalStackTraceItem::CompareObjectField {
                            name: name.value().into(),
                        }
                    }
                    TraceItem::ManifestArrayItem { index } => {
                        EvalStackTraceItem::ManifestArrayItem { index }
                    }
                    TraceItem::ManifestObjectField { ref name } => {
                        EvalStackTraceItem::ManifestObjectField {
                            name: name.value().into(),
                        }
                    }
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

enum ParseNumRadixError {
    Empty,
    InvalidDigit(char),
    Overflow,
}

fn parse_num_radix<const RADIX: u8>(s: &str) -> Result<f64, ParseNumRadixError> {
    if s.is_empty() {
        return Err(ParseNumRadixError::Empty);
    }
    let s = s.trim_start_matches('0');

    let max_digits_128 = match RADIX {
        8 => 128 / 3,
        16 => 128 / 4,
        _ => unreachable!(),
    };
    let num_digits_128 = s.len().min(max_digits_128);

    let mut number = 0u128;
    for chr in s[..num_digits_128].chars() {
        let digit = chr
            .to_digit(RADIX.into())
            .ok_or(ParseNumRadixError::InvalidDigit(chr))?;
        number = number * u128::from(RADIX) + u128::from(digit);
    }

    let mut number = number as f64;
    for chr in s[num_digits_128..].chars() {
        if !chr.is_ascii_hexdigit() {
            return Err(ParseNumRadixError::InvalidDigit(chr));
        }
        number *= f64::from(RADIX);
    }

    if !number.is_finite() {
        return Err(ParseNumRadixError::Overflow);
    }

    Ok(number)
}
