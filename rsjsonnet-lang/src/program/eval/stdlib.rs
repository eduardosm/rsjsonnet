use std::cell::{Cell, OnceCell};
use std::fmt::Write as _;
use std::rc::Rc;

use super::manifest::{escape_string_json, escape_string_python};
use super::{
    float, parse_num_radix, ArrayData, EvalErrorKind, EvalErrorValueType, EvalResult, Evaluator,
    FuncData, ManifestJsonFormat, ObjectData, ObjectField, ParseNumRadixError, State, ThunkData,
    TraceItem, ValueData,
};
use crate::gc::{Gc, GcView};
use crate::interner::InternedStr;
use crate::{ast, FHashMap, FHashSet};

impl<'p> Evaluator<'_, 'p> {
    pub(super) fn do_std_ext_var(&mut self) -> EvalResult<()> {
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
        Ok(())
    }

    pub(super) fn do_std_type(&mut self) {
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

    pub(super) fn do_std_is_array(&mut self) {
        let arg = self.value_stack.pop().unwrap();
        self.value_stack
            .push(ValueData::Bool(matches!(arg, ValueData::Array(_))));
    }

    pub(super) fn do_std_is_boolean(&mut self) {
        let arg = self.value_stack.pop().unwrap();
        self.value_stack
            .push(ValueData::Bool(matches!(arg, ValueData::Bool(_))));
    }

    pub(super) fn do_std_is_function(&mut self) {
        let arg = self.value_stack.pop().unwrap();
        self.value_stack
            .push(ValueData::Bool(matches!(arg, ValueData::Function(_))));
    }

    pub(super) fn do_std_is_number(&mut self) {
        let arg = self.value_stack.pop().unwrap();
        self.value_stack
            .push(ValueData::Bool(matches!(arg, ValueData::Number(_))));
    }

    pub(super) fn do_std_is_object(&mut self) {
        let arg = self.value_stack.pop().unwrap();
        self.value_stack
            .push(ValueData::Bool(matches!(arg, ValueData::Object(_))));
    }

    pub(super) fn do_std_is_string(&mut self) {
        let arg = self.value_stack.pop().unwrap();
        self.value_stack
            .push(ValueData::Bool(matches!(arg, ValueData::String(_))));
    }

    pub(super) fn do_std_length(&mut self) -> EvalResult<()> {
        let arg = self.value_stack.pop().unwrap();
        let length = match arg {
            ValueData::String(s) => s.chars().count(),
            ValueData::Array(array) => array.view().len(),
            ValueData::Object(object) => {
                let object = object.view();
                object.get_visible_fields_order().count()
            }
            ValueData::Function(func) => func.view().params.order.len(),
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
        Ok(())
    }

    pub(super) fn do_std_prune_value(&mut self) {
        match self.value_stack.pop().unwrap() {
            ValueData::Array(array) => {
                let array = array.view();

                self.array_stack.push(Vec::new());
                self.state_stack.push(State::ArrayToValue);

                for item in array.iter().rev() {
                    self.state_stack.push(State::StdPruneArrayItem);
                    self.state_stack.push(State::StdPruneValue);
                    self.state_stack.push(State::DoThunk(item.view()));
                }
            }
            ValueData::Object(object) => {
                let object = object.view();
                let visible_fields = object.get_visible_fields_order();

                self.object_stack.push(ObjectData::new_empty());
                self.state_stack.push(State::ObjectToValue);

                for field_name in visible_fields.rev() {
                    let field_thunk = self
                        .program
                        .find_object_field_thunk(&object, 0, field_name)
                        .unwrap();

                    self.state_stack
                        .push(State::StdPruneObjectField { name: field_name });
                    self.state_stack.push(State::StdPruneValue);
                    self.state_stack.push(State::DoThunk(field_thunk));
                }

                self.check_object_asserts(&object);
            }
            value => {
                self.value_stack.push(value);
            }
        }
    }

    pub(super) fn do_std_prune_array_item(&mut self) {
        let item_value = self.value_stack.pop().unwrap();
        let is_empty = match item_value {
            ValueData::Null => true,
            ValueData::Array(ref array) => array.view().is_empty(),
            ValueData::Object(ref object) => object.view().self_layer.fields.is_empty(),
            _ => false,
        };
        if !is_empty {
            self.array_stack
                .last_mut()
                .unwrap()
                .push(self.program.gc_alloc(ThunkData::new_done(item_value)));
        }
    }

    pub(super) fn do_std_prune_object_field(&mut self, name: InternedStr<'p>) {
        let item_value = self.value_stack.pop().unwrap();
        let is_empty = match item_value {
            ValueData::Null => true,
            ValueData::Array(ref array) => array.view().is_empty(),
            ValueData::Object(ref object) => object.view().self_layer.fields.is_empty(),
            _ => false,
        };
        if !is_empty {
            let object = self.object_stack.last_mut().unwrap();
            object.self_layer.fields.insert(
                name,
                ObjectField {
                    base_env: None,
                    visibility: ast::Visibility::Default,
                    expr: None,
                    thunk: OnceCell::from(self.program.gc_alloc(ThunkData::new_done(item_value))),
                },
            );
        }
    }

    pub(super) fn do_std_object_has_ex(&mut self) -> EvalResult<()> {
        let inc_hidden = self.value_stack.pop().unwrap();
        let field_name = self.value_stack.pop().unwrap();
        let object = self.value_stack.pop().unwrap();

        let object = self.expect_std_func_arg_object(object, "objectHasEx", 0)?;
        let field_name = self.expect_std_func_arg_string(field_name, "objectHasEx", 1)?;
        let inc_hidden = self.expect_std_func_arg_bool(inc_hidden, "objectHasEx", 2)?;

        let has_field = self
            .program
            .str_interner
            .get_interned(&field_name)
            .is_some_and(|field_name| {
                if inc_hidden {
                    object.has_field(0, field_name)
                } else {
                    object.has_visible_field(field_name)
                }
            });

        self.value_stack.push(ValueData::Bool(has_field));

        Ok(())
    }

    pub(super) fn do_std_object_fields_ex(&mut self) -> EvalResult<()> {
        let inc_hidden = self.value_stack.pop().unwrap();
        let object = self.value_stack.pop().unwrap();

        let object = self.expect_std_func_arg_object(object, "objectFieldsEx", 0)?;
        let inc_hidden = self.expect_std_func_arg_bool(inc_hidden, "objectFieldsEx", 1)?;

        let fields_order = object.get_fields_order();

        let array = self
            .program
            .make_value_array(fields_order.iter().filter_map(|(name, visible)| {
                if inc_hidden || *visible {
                    Some(ValueData::String(name.value().into()))
                } else {
                    None
                }
            }));

        self.value_stack.push(ValueData::Array(array));

        Ok(())
    }

    pub(super) fn do_std_object_remove_key(&mut self) -> EvalResult<()> {
        let key = self.value_stack.pop().unwrap();
        let object = self.value_stack.pop().unwrap();

        let object = self.expect_std_func_arg_object(object, "objectRemoveKey", 0)?;
        let key = self.expect_std_func_arg_string(key, "objectRemoveKey", 1)?;

        let key = self.program.str_interner.get_interned(&key);

        let mut fields = FHashMap::default();
        for field_name in object.get_visible_fields_order() {
            if Some(field_name) != key {
                let field_thunk = self
                    .program
                    .find_object_field_thunk(&object, 0, field_name)
                    .unwrap();
                fields.insert(
                    field_name,
                    ObjectField {
                        base_env: None,
                        visibility: ast::Visibility::Default,
                        expr: None,
                        thunk: OnceCell::from(Gc::from(&field_thunk)),
                    },
                );
            }
        }

        self.value_stack.push(ValueData::Object(
            self.program.gc_alloc(ObjectData::new_simple(fields)),
        ));
        self.check_object_asserts(&object);

        Ok(())
    }

    pub(super) fn do_std_map_with_key(&mut self) -> EvalResult<()> {
        let object = self.value_stack.pop().unwrap();
        let func = self.value_stack.pop().unwrap();

        let func = self.expect_std_func_arg_func(func, "mapWithKey", 0)?;
        let object = self.expect_std_func_arg_object(object, "mapWithKey", 1)?;

        let mut mapped_fields = FHashMap::default();
        for field_name in object.get_visible_fields_order() {
            let field_thunk = self
                .program
                .find_object_field_thunk(&object, 0, field_name)
                .unwrap();

            let args_thunks = Box::new([
                self.program.gc_alloc(ThunkData::new_done(ValueData::String(
                    field_name.value().into(),
                ))),
                Gc::from(&field_thunk),
            ]);

            let mapped_field = ObjectField {
                base_env: None,
                visibility: ast::Visibility::Default,
                expr: None,
                thunk: OnceCell::from(
                    self.program
                        .gc_alloc(ThunkData::new_pending_call(Gc::from(&func), args_thunks)),
                ),
            };

            mapped_fields.insert(field_name, mapped_field);
        }

        let mapped_object = ObjectData::new_simple(mapped_fields);
        self.value_stack
            .push(ValueData::Object(self.program.gc_alloc(mapped_object)));

        self.check_object_asserts(&object);

        Ok(())
    }

    pub(super) fn do_std_primitive_equals(&mut self) -> EvalResult<()> {
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
                return Err(
                    self.report_error(EvalErrorKind::PrimitiveEqualsNonPrimitive {
                        got_type: EvalErrorValueType::Array,
                    }),
                );
            }
            (ValueData::Object(_), ValueData::Object(_)) => {
                return Err(
                    self.report_error(EvalErrorKind::PrimitiveEqualsNonPrimitive {
                        got_type: EvalErrorValueType::Object,
                    }),
                );
            }
            (ValueData::Function(_), ValueData::Function(_)) => {
                return Err(self.report_error(EvalErrorKind::CompareFunctions));
            }
            _ => {
                // Different types compare to false.
                self.value_stack.push(ValueData::Bool(false));
            }
        }
        Ok(())
    }

    pub(super) fn do_std_compare_array(&mut self) -> EvalResult<()> {
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
        Ok(())
    }

    pub(super) fn do_std_exponent(&mut self) -> EvalResult<()> {
        let arg = self.value_stack.pop().unwrap();
        let arg = self.expect_std_func_arg_number(arg, "exponent", 0)?;
        let (_, exp) = float::frexp(arg);
        self.value_stack.push(ValueData::Number(exp.into()));
        Ok(())
    }

    pub(super) fn do_std_mantissa(&mut self) -> EvalResult<()> {
        let arg = self.value_stack.pop().unwrap();
        let arg = self.expect_std_func_arg_number(arg, "mantissa", 0)?;
        let (mant, _) = float::frexp(arg);
        self.value_stack.push(ValueData::Number(mant));
        Ok(())
    }

    pub(super) fn do_std_floor(&mut self) -> EvalResult<()> {
        let arg = self.value_stack.pop().unwrap();
        let arg = self.expect_std_func_arg_number(arg, "floor", 0)?;
        self.value_stack.push(ValueData::Number(arg.floor()));
        Ok(())
    }

    pub(super) fn do_std_ceil(&mut self) -> EvalResult<()> {
        let arg = self.value_stack.pop().unwrap();
        let arg = self.expect_std_func_arg_number(arg, "ceil", 0)?;
        self.value_stack.push(ValueData::Number(arg.ceil()));
        Ok(())
    }

    pub(super) fn do_std_modulo(&mut self) -> EvalResult<()> {
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

        Ok(())
    }

    pub(super) fn do_std_pow(&mut self) -> EvalResult<()> {
        let rhs = self.value_stack.pop().unwrap();
        let lhs = self.value_stack.pop().unwrap();

        let lhs = self.expect_std_func_arg_number(lhs, "pow", 0)?;
        let rhs = self.expect_std_func_arg_number(rhs, "pow", 1)?;

        let r = lhs.powf(rhs);
        self.check_number_value(r, None)?;
        self.value_stack.push(ValueData::Number(r));

        Ok(())
    }

    pub(super) fn do_std_exp(&mut self) -> EvalResult<()> {
        let arg = self.value_stack.pop().unwrap();
        let arg = self.expect_std_func_arg_number(arg, "exp", 0)?;
        let r = arg.exp();
        self.check_number_value(r, None)?;
        self.value_stack.push(ValueData::Number(r));
        Ok(())
    }

    pub(super) fn do_std_log(&mut self) -> EvalResult<()> {
        let arg = self.value_stack.pop().unwrap();
        let arg = self.expect_std_func_arg_number(arg, "log", 0)?;
        let r = arg.ln();
        self.check_number_value(r, None)?;
        self.value_stack.push(ValueData::Number(r));
        Ok(())
    }

    pub(super) fn do_std_log2(&mut self) -> EvalResult<()> {
        let arg = self.value_stack.pop().unwrap();
        let arg = self.expect_std_func_arg_number(arg, "log2", 0)?;
        let r = arg.log2();
        self.check_number_value(r, None)?;
        self.value_stack.push(ValueData::Number(r));
        Ok(())
    }

    pub(super) fn do_std_log10(&mut self) -> EvalResult<()> {
        let arg = self.value_stack.pop().unwrap();
        let arg = self.expect_std_func_arg_number(arg, "log10", 0)?;
        let r = arg.log10();
        self.check_number_value(r, None)?;
        self.value_stack.push(ValueData::Number(r));
        Ok(())
    }

    pub(super) fn do_std_sqrt(&mut self) -> EvalResult<()> {
        let arg = self.value_stack.pop().unwrap();
        let arg = self.expect_std_func_arg_number(arg, "sqrt", 0)?;
        let r = arg.sqrt();
        self.check_number_value(r, None)?;
        self.value_stack.push(ValueData::Number(r));
        Ok(())
    }

    pub(super) fn do_std_sin(&mut self) -> EvalResult<()> {
        let arg = self.value_stack.pop().unwrap();
        let arg = self.expect_std_func_arg_number(arg, "sin", 0)?;
        let r = arg.sin();
        self.check_number_value(r, None)?;
        self.value_stack.push(ValueData::Number(r));
        Ok(())
    }

    pub(super) fn do_std_cos(&mut self) -> EvalResult<()> {
        let arg = self.value_stack.pop().unwrap();
        let arg = self.expect_std_func_arg_number(arg, "cos", 0)?;
        let r = arg.cos();
        self.check_number_value(r, None)?;
        self.value_stack.push(ValueData::Number(r));
        Ok(())
    }

    pub(super) fn do_std_tan(&mut self) -> EvalResult<()> {
        let arg = self.value_stack.pop().unwrap();
        let arg = self.expect_std_func_arg_number(arg, "tan", 0)?;
        let r = arg.tan();
        self.check_number_value(r, None)?;
        self.value_stack.push(ValueData::Number(r));
        Ok(())
    }

    pub(super) fn do_std_asin(&mut self) -> EvalResult<()> {
        let arg = self.value_stack.pop().unwrap();
        let arg = self.expect_std_func_arg_number(arg, "asin", 0)?;
        let r = arg.asin();
        self.check_number_value(r, None)?;
        self.value_stack.push(ValueData::Number(r));
        Ok(())
    }

    pub(super) fn do_std_acos(&mut self) -> EvalResult<()> {
        let arg = self.value_stack.pop().unwrap();
        let arg = self.expect_std_func_arg_number(arg, "acos", 0)?;
        let r = arg.acos();
        self.check_number_value(r, None)?;
        self.value_stack.push(ValueData::Number(r));
        Ok(())
    }

    pub(super) fn do_std_atan(&mut self) -> EvalResult<()> {
        let arg = self.value_stack.pop().unwrap();
        let arg = self.expect_std_func_arg_number(arg, "atan", 0)?;
        let r = arg.atan();
        self.check_number_value(r, None)?;
        self.value_stack.push(ValueData::Number(r));
        Ok(())
    }

    pub(super) fn do_std_atan2(&mut self) -> EvalResult<()> {
        let arg1 = self.value_stack.pop().unwrap();
        let arg0 = self.value_stack.pop().unwrap();
        let arg0 = self.expect_std_func_arg_number(arg0, "atan2", 0)?;
        let arg1 = self.expect_std_func_arg_number(arg1, "atan2", 1)?;
        let r = f64::atan2(arg0, arg1);
        self.check_number_value(r, None)?;
        self.value_stack.push(ValueData::Number(r));
        Ok(())
    }

    pub(super) fn do_std_deg2rad(&mut self) -> EvalResult<()> {
        let arg = self.value_stack.pop().unwrap();
        let arg = self.expect_std_func_arg_number(arg, "deg2rad", 0)?;
        let r = arg.to_radians();
        self.check_number_value(r, None)?;
        self.value_stack.push(ValueData::Number(r));
        Ok(())
    }

    pub(super) fn do_std_rad2deg(&mut self) -> EvalResult<()> {
        let arg = self.value_stack.pop().unwrap();
        let arg = self.expect_std_func_arg_number(arg, "rad2deg", 0)?;
        let r = arg.to_degrees();
        self.check_number_value(r, None)?;
        self.value_stack.push(ValueData::Number(r));
        Ok(())
    }

    pub(super) fn do_std_hypot(&mut self) -> EvalResult<()> {
        let arg1 = self.value_stack.pop().unwrap();
        let arg0 = self.value_stack.pop().unwrap();
        let arg0 = self.expect_std_func_arg_number(arg0, "hypot", 0)?;
        let arg1 = self.expect_std_func_arg_number(arg1, "hypot", 1)?;
        let r = f64::hypot(arg0, arg1);
        self.check_number_value(r, None)?;
        self.value_stack.push(ValueData::Number(r));
        Ok(())
    }

    pub(super) fn do_std_is_even(&mut self) -> EvalResult<()> {
        let arg = self.value_stack.pop().unwrap();
        let arg = self.expect_std_func_arg_number(arg, "isEven", 0)?;
        let r = arg % 2.0 == 0.0;
        self.value_stack.push(ValueData::Bool(r));
        Ok(())
    }

    pub(super) fn do_std_is_odd(&mut self) -> EvalResult<()> {
        let arg = self.value_stack.pop().unwrap();
        let arg = self.expect_std_func_arg_number(arg, "isOdd", 0)?;
        let r = arg % 2.0 != 0.0;
        self.value_stack.push(ValueData::Bool(r));
        Ok(())
    }

    pub(super) fn do_std_is_integer(&mut self) -> EvalResult<()> {
        let arg = self.value_stack.pop().unwrap();
        let arg = self.expect_std_func_arg_number(arg, "isInteger", 0)?;
        let r = arg.trunc() == arg;
        self.value_stack.push(ValueData::Bool(r));
        Ok(())
    }

    pub(super) fn do_std_is_decimal(&mut self) -> EvalResult<()> {
        let arg = self.value_stack.pop().unwrap();
        let arg = self.expect_std_func_arg_number(arg, "isDecimal", 0)?;
        let r = arg.trunc() != arg;
        self.value_stack.push(ValueData::Bool(r));
        Ok(())
    }

    pub(super) fn do_std_assert_equal(&mut self) {
        let lhs = self.value_stack[self.value_stack.len() - 2].clone();
        let rhs = self.value_stack[self.value_stack.len() - 1].clone();
        self.value_stack.push(lhs);
        self.value_stack.push(rhs);

        self.state_stack
            .push(State::FnInfallible(Self::do_std_assert_equal_check));
        self.state_stack.push(State::EqualsValue);
    }

    fn do_std_assert_equal_check(&mut self) {
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
            self.state_stack
                .push(State::FnInfallible(Self::do_std_assert_equal_fail_1));
            self.state_stack.push(State::ManifestJson {
                format: ManifestJsonFormat::default_to_string(),
                depth: 0,
            });
        }
    }

    fn do_std_assert_equal_fail_1(&mut self) {
        self.string_stack.push(String::new());
        self.state_stack
            .push(State::FnFallible(Self::do_std_assert_equal_fail_2));
        self.state_stack.push(State::ManifestJson {
            format: ManifestJsonFormat::default_to_string(),
            depth: 0,
        });
    }

    fn do_std_assert_equal_fail_2(&mut self) -> EvalResult<()> {
        let rhs = self.string_stack.pop().unwrap();
        let lhs = self.string_stack.pop().unwrap();
        Err(self.report_error(EvalErrorKind::AssertEqualFailed { lhs, rhs }))
    }

    pub(super) fn do_std_codepoint(&mut self) -> EvalResult<()> {
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

        Ok(())
    }

    pub(super) fn do_std_char(&mut self) -> EvalResult<()> {
        let arg = self.value_stack.pop().unwrap();
        let arg = self.expect_std_func_arg_number(arg, "char", 0)?;
        let arg = arg.trunc();

        let Some(chr) = float::try_to_u32(arg).and_then(char::from_u32) else {
            return Err(self.report_error(EvalErrorKind::Other {
                span: None,
                message: format!("{arg} is not a valid unicode codepoint"),
            }));
        };

        self.value_stack.push(ValueData::from_char(chr));

        Ok(())
    }

    pub(super) fn do_std_substr(&mut self) -> EvalResult<()> {
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

        Ok(())
    }

    pub(super) fn do_std_find_substr(&mut self) -> EvalResult<()> {
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

        Ok(())
    }

    pub(super) fn do_std_starts_with(&mut self) -> EvalResult<()> {
        let b = self.value_stack.pop().unwrap();
        let a = self.value_stack.pop().unwrap();

        let a = self.expect_std_func_arg_string(a, "startsWith", 0)?;
        let b = self.expect_std_func_arg_string(b, "startsWith", 1)?;

        self.value_stack.push(ValueData::Bool(a.starts_with(&*b)));

        Ok(())
    }

    pub(super) fn do_std_ends_with(&mut self) -> EvalResult<()> {
        let b = self.value_stack.pop().unwrap();
        let a = self.value_stack.pop().unwrap();

        let a = self.expect_std_func_arg_string(a, "endsWith", 0)?;
        let b = self.expect_std_func_arg_string(b, "endsWith", 1)?;

        self.value_stack.push(ValueData::Bool(a.ends_with(&*b)));

        Ok(())
    }

    pub(super) fn do_std_strip_chars(&mut self) -> EvalResult<()> {
        let chars = self.value_stack.pop().unwrap();
        let str = self.value_stack.pop().unwrap();

        let str = self.expect_std_func_arg_string(str, "stripChars", 0)?;
        let chars = self.expect_std_func_arg_string(chars, "stripChars", 1)?;
        let chars: Vec<_> = chars.chars().collect();

        let mut res = &*str;
        while let Some(stripped) = res.strip_prefix(chars.as_slice()) {
            res = stripped;
        }
        while let Some(stripped) = res.strip_suffix(chars.as_slice()) {
            res = stripped;
        }

        self.value_stack.push(ValueData::String(res.into()));

        Ok(())
    }

    pub(super) fn do_std_lstrip_chars(&mut self) -> EvalResult<()> {
        let chars = self.value_stack.pop().unwrap();
        let str = self.value_stack.pop().unwrap();

        let str = self.expect_std_func_arg_string(str, "lstripChars", 0)?;
        let chars = self.expect_std_func_arg_string(chars, "lstripChars", 1)?;
        let chars: Vec<_> = chars.chars().collect();

        let mut res = &*str;
        while let Some(stripped) = res.strip_prefix(chars.as_slice()) {
            res = stripped;
        }

        self.value_stack.push(ValueData::String(res.into()));

        Ok(())
    }

    pub(super) fn do_std_rstrip_chars(&mut self) -> EvalResult<()> {
        let chars = self.value_stack.pop().unwrap();
        let str = self.value_stack.pop().unwrap();

        let str = self.expect_std_func_arg_string(str, "rstripChars", 0)?;
        let chars = self.expect_std_func_arg_string(chars, "rstripChars", 1)?;
        let chars: Vec<_> = chars.chars().collect();

        let mut res = &*str;
        while let Some(stripped) = res.strip_suffix(chars.as_slice()) {
            res = stripped;
        }

        self.value_stack.push(ValueData::String(res.into()));

        Ok(())
    }

    pub(super) fn do_std_split(&mut self) -> EvalResult<()> {
        let c_value = self.value_stack.pop().unwrap();
        let str_value = self.value_stack.pop().unwrap();

        let str_value = self.expect_std_func_arg_string(str_value, "split", 0)?;
        let c_value = self.expect_std_func_arg_string(c_value, "split", 1)?;

        if c_value.is_empty() {
            return Err(self.report_error(EvalErrorKind::Other {
                span: None,
                message: "split delimiter is empty".into(),
            }));
        }

        let result_array = self.program.make_value_array(
            str_value
                .split(&*c_value)
                .map(|s| ValueData::String(s.into())),
        );
        self.value_stack.push(ValueData::Array(result_array));

        Ok(())
    }

    pub(super) fn do_std_split_limit(&mut self) -> EvalResult<()> {
        let maxsplits_value = self.value_stack.pop().unwrap();
        let c_value = self.value_stack.pop().unwrap();
        let str_value = self.value_stack.pop().unwrap();

        let str_value = self.expect_std_func_arg_string(str_value, "splitLimit", 0)?;
        let c_value = self.expect_std_func_arg_string(c_value, "splitLimit", 1)?;
        let maxsplits = self.expect_std_func_arg_number(maxsplits_value, "splitLimit", 2)?;

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
                    message: format!("`maxsplits` value {maxsplits} is not -1 or non-negative"),
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

        Ok(())
    }

    pub(super) fn do_std_split_limit_r(&mut self) -> EvalResult<()> {
        let maxsplits_value = self.value_stack.pop().unwrap();
        let c_value = self.value_stack.pop().unwrap();
        let str_value = self.value_stack.pop().unwrap();

        let str_value = self.expect_std_func_arg_string(str_value, "splitLimitR", 0)?;
        let c_value = self.expect_std_func_arg_string(c_value, "splitLimitR", 1)?;
        let maxsplits = self.expect_std_func_arg_number(maxsplits_value, "splitLimitR", 2)?;

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
                    message: format!("`maxsplits` value {maxsplits} is not -1 or non-negative"),
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

        Ok(())
    }

    pub(super) fn do_std_str_replace(&mut self) -> EvalResult<()> {
        let to = self.value_stack.pop().unwrap();
        let from = self.value_stack.pop().unwrap();
        let s = self.value_stack.pop().unwrap();

        let s = self.expect_std_func_arg_string(s, "strReplace", 0)?;
        let from = self.expect_std_func_arg_string(from, "strReplace", 1)?;
        let to = self.expect_std_func_arg_string(to, "strReplace", 2)?;

        let result = s.replace(&*from, &to);
        self.value_stack.push(ValueData::String(result.into()));

        Ok(())
    }

    pub(super) fn do_std_trim(&mut self) -> EvalResult<()> {
        let s = self.value_stack.pop().unwrap();
        let s = self.expect_std_func_arg_string(s, "trim", 0)?;
        let result = s.trim_matches(|c| {
            matches!(c, '\t' | '\n' | '\u{0C}' | '\r' | ' ' | '\u{85}' | '\u{A0}')
        });
        self.value_stack.push(ValueData::String(result.into()));
        Ok(())
    }

    pub(super) fn do_std_equals_ignore_case(&mut self) -> EvalResult<()> {
        let s2 = self.value_stack.pop().unwrap();
        let s1 = self.value_stack.pop().unwrap();

        let s1 = self.expect_std_func_arg_string(s1, "equalsIgnoreCase", 0)?;
        let s2 = self.expect_std_func_arg_string(s2, "equalsIgnoreCase", 1)?;

        self.value_stack
            .push(ValueData::Bool(s1.eq_ignore_ascii_case(&s2)));

        Ok(())
    }

    pub(super) fn do_std_ascii_upper(&mut self) -> EvalResult<()> {
        let arg = self.value_stack.pop().unwrap();
        let arg = self.expect_std_func_arg_string(arg, "asciiUpper", 0)?;
        let r = arg.to_ascii_uppercase();
        self.value_stack.push(ValueData::String(r.into()));
        Ok(())
    }

    pub(super) fn do_std_ascii_lower(&mut self) -> EvalResult<()> {
        let arg = self.value_stack.pop().unwrap();
        let arg = self.expect_std_func_arg_string(arg, "asciiLower", 0)?;
        let r = arg.to_ascii_lowercase();
        self.value_stack.push(ValueData::String(r.into()));
        Ok(())
    }

    pub(super) fn do_std_string_chars(&mut self) -> EvalResult<()> {
        let arg = self.value_stack.pop().unwrap();
        let arg = self.expect_std_func_arg_string(arg, "stringChars", 0)?;
        let array = self
            .program
            .make_value_array(arg.chars().map(ValueData::from_char));
        self.value_stack.push(ValueData::Array(array));
        Ok(())
    }

    pub(super) fn do_std_escape_string_json(&mut self) {
        let s = self.string_stack.pop().unwrap();
        let mut escaped = String::new();
        escape_string_json(&s, &mut escaped);
        self.value_stack.push(ValueData::String(escaped.into()));
    }

    pub(super) fn do_std_escape_string_python(&mut self) {
        let s = self.string_stack.pop().unwrap();
        let mut escaped = String::new();
        escape_string_python(&s, &mut escaped);
        self.value_stack.push(ValueData::String(escaped.into()));
    }

    pub(super) fn do_std_escape_string_bash(&mut self) {
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

    pub(super) fn do_std_escape_string_dollars(&mut self) {
        let s = self.string_stack.pop().unwrap();
        let escaped = s.replace('$', "$$");
        self.value_stack.push(ValueData::String(escaped.into()));
    }

    pub(super) fn do_std_escape_string_xml(&mut self) {
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

    pub(super) fn do_std_parse_int(&mut self) -> EvalResult<()> {
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

        Ok(())
    }

    pub(super) fn do_std_parse_octal(&mut self) -> EvalResult<()> {
        let arg = self.value_stack.pop().unwrap();
        let s = self.expect_std_func_arg_string(arg, "parseOctal", 0)?;

        let number = parse_num_radix::<8>(&s).map_err(|e| match e {
            ParseNumRadixError::Empty => self.report_error(EvalErrorKind::Other {
                span: None,
                message: format!("octal integer without digits: {s:?}"),
            }),
            ParseNumRadixError::InvalidDigit(chr) => self.report_error(EvalErrorKind::Other {
                span: None,
                message: format!("invalid octal digit: {chr:?}"),
            }),
            ParseNumRadixError::Overflow => {
                self.report_error(EvalErrorKind::NumberOverflow { span: None })
            }
        })?;

        self.value_stack.push(ValueData::Number(number));

        Ok(())
    }

    pub(super) fn do_std_parse_hex(&mut self) -> EvalResult<()> {
        let arg = self.value_stack.pop().unwrap();
        let s = self.expect_std_func_arg_string(arg, "parseHex", 0)?;

        let number = parse_num_radix::<16>(&s).map_err(|e| match e {
            ParseNumRadixError::Empty => self.report_error(EvalErrorKind::Other {
                span: None,
                message: format!("hexadecimal integer without digits: {s:?}"),
            }),
            ParseNumRadixError::InvalidDigit(chr) => self.report_error(EvalErrorKind::Other {
                span: None,
                message: format!("invalid hexadecimal digit: {chr:?}"),
            }),
            ParseNumRadixError::Overflow => {
                self.report_error(EvalErrorKind::NumberOverflow { span: None })
            }
        })?;

        self.value_stack.push(ValueData::Number(number));

        Ok(())
    }

    pub(super) fn do_std_parse_json(&mut self) -> EvalResult<()> {
        let arg = self.value_stack.pop().unwrap();
        let s = self.expect_std_func_arg_string(arg, "parseJson", 0)?;
        match super::parse_json::parse_json(self.program, &s) {
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
        Ok(())
    }

    pub(super) fn do_std_parse_yaml(&mut self) -> EvalResult<()> {
        let arg = self.value_stack.pop().unwrap();
        let s = self.expect_std_func_arg_string(arg, "parseYaml", 0)?;
        match super::parse_yaml::parse_yaml(self.program, &s) {
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
        Ok(())
    }

    pub(super) fn do_std_encode_utf8(&mut self) -> EvalResult<()> {
        let arg = self.value_stack.pop().unwrap();
        let s = self.expect_std_func_arg_string(arg, "encodeUTF8", 0)?;
        let array = self
            .program
            .make_value_array(s.bytes().map(|byte| ValueData::Number(f64::from(byte))));
        self.value_stack.push(ValueData::Array(array));
        Ok(())
    }

    pub(super) fn do_std_decode_utf8(&mut self) -> EvalResult<()> {
        let arg = self.value_stack.pop().unwrap();
        let array = self.expect_std_func_arg_array(arg, "decodeUTF8", 0)?;

        self.byte_array_stack.push(Vec::with_capacity(array.len()));
        self.state_stack
            .push(State::FnInfallible(Self::do_std_decode_utf8_finish));
        for item in array.iter().rev() {
            self.state_stack
                .push(State::FnFallible(Self::do_std_decode_utf8_check_item));
            self.state_stack.push(State::DoThunk(item.view()));
        }

        Ok(())
    }

    fn do_std_decode_utf8_check_item(&mut self) -> EvalResult<()> {
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
        Ok(())
    }

    fn do_std_decode_utf8_finish(&mut self) {
        let bytes = self.byte_array_stack.pop().unwrap();
        let s = String::from_utf8_lossy(&bytes);
        self.value_stack.push(ValueData::String(s.into()));
    }

    pub(super) fn do_std_manifest_ini(&mut self) -> EvalResult<()> {
        let ini = self.value_stack.pop().unwrap();
        let ini = self.expect_std_func_arg_object(ini, "manifestIni", 0)?;

        let main_name = self.program.intern_str("main");
        let sections_name = self.program.intern_str("sections");

        let Some(sections_thunk) = self.program.find_object_field_thunk(&ini, 0, sections_name)
        else {
            return Err(self.report_error(EvalErrorKind::Other {
                span: None,
                message: "missing \"sections\" field".into(),
            }));
        };

        let main_thunk = self.program.find_object_field_thunk(&ini, 0, main_name);

        self.string_stack.push(String::new());
        self.state_stack.push(State::StringToValue);
        self.state_stack.push(State::StdManifestIniSections);
        self.state_stack.push(State::DoThunk(sections_thunk));
        if let Some(main_thunk) = main_thunk {
            self.state_stack.push(State::ManifestIniSection);
            self.state_stack.push(State::DoThunk(main_thunk));
        }
        self.check_object_asserts(&ini);

        Ok(())
    }

    pub(super) fn do_std_manifest_ini_sections(&mut self) -> EvalResult<()> {
        let ValueData::Object(object) = self.value_stack.pop().unwrap() else {
            return Err(self.report_error(EvalErrorKind::Other {
                span: None,
                message: "\"sections\" field must be an object".into(),
            }));
        };
        let object = object.view();

        let visible_fields = object.get_visible_fields_order();
        for field_name in visible_fields.rev() {
            let field_thunk = self
                .program
                .find_object_field_thunk(&object, 0, field_name)
                .unwrap();

            self.state_stack.push(State::ManifestIniSection);
            self.state_stack.push(State::DoThunk(field_thunk));

            self.state_stack
                .push(State::AppendToString(format!("[{}]\n", field_name.value())));
        }
        self.check_object_asserts(&object);

        Ok(())
    }

    pub(super) fn do_std_manifest_python(&mut self) {
        self.string_stack.push(String::new());
        self.state_stack.push(State::StringToValue);
        self.state_stack.push(State::ManifestPython);
    }

    pub(super) fn do_std_manifest_python_vars(&mut self) -> EvalResult<()> {
        let value = self.value_stack.pop().unwrap();
        let object = self.expect_std_func_arg_object(value, "manifestPythonVars", 0)?;

        self.string_stack.push(String::new());
        self.state_stack.push(State::StringToValue);

        let visible_fields = object.get_visible_fields_order();

        for field_name in visible_fields.rev() {
            self.state_stack.push(State::AppendToString('\n'.into()));

            self.push_trace_item(TraceItem::ManifestObjectField { name: field_name });
            self.state_stack.push(State::ManifestPython);
            let field_thunk = self
                .program
                .find_object_field_thunk(&object, 0, field_name)
                .unwrap();
            self.state_stack.push(State::DoThunk(field_thunk));
            self.delay_trace_item();
            self.state_stack.push(State::AppendToString(" = ".into()));
            self.state_stack
                .push(State::AppendToString(field_name.value().into()));
        }
        self.check_object_asserts(&object);

        Ok(())
    }

    pub(super) fn do_std_manifest_json_ex(&mut self) -> EvalResult<()> {
        let key_val_sep = self.value_stack.pop().unwrap();
        let newline = self.value_stack.pop().unwrap();
        let indent = self.value_stack.pop().unwrap();

        let indent = self.expect_std_func_arg_string(indent, "manifestJsonEx", 1)?;
        let newline = self.expect_std_func_arg_string(newline, "manifestJsonEx", 2)?;
        let key_val_sep = self.expect_std_func_arg_string(key_val_sep, "manifestJsonEx", 3)?;

        self.string_stack.push(String::new());
        self.state_stack.push(State::StringToValue);
        self.state_stack.push(State::ManifestJson {
            format: ManifestJsonFormat::for_std_manifest_ex(&indent, &newline, &key_val_sep),
            depth: 0,
        });

        Ok(())
    }

    pub(super) fn do_std_manifest_yaml_doc(&mut self) -> EvalResult<()> {
        let quote_keys = self.value_stack.pop().unwrap();
        let indent_array_in_object = self.value_stack.pop().unwrap();

        let indent_array_in_object =
            self.expect_std_func_arg_bool(indent_array_in_object, "manifestYamlDoc", 1)?;
        let quote_keys = self.expect_std_func_arg_bool(quote_keys, "manifestYamlDoc", 2)?;

        self.string_stack.push(String::new());
        self.state_stack.push(State::StringToValue);
        self.state_stack.push(State::ManifestYamlDoc {
            indent_array_in_object,
            quote_keys,
            depth: 0,
            parent_is_array: false,
            parent_is_object: false,
        });

        Ok(())
    }

    pub(super) fn do_std_manifest_yaml_stream(&mut self) -> EvalResult<()> {
        let quote_keys = self.value_stack.pop().unwrap();
        let c_document_end = self.value_stack.pop().unwrap();
        let indent_array_in_object = self.value_stack.pop().unwrap();
        let value = self.value_stack.pop().unwrap();

        let array = self.expect_std_func_arg_array(value, "manifestYamlStream", 0)?;
        let indent_array_in_object =
            self.expect_std_func_arg_bool(indent_array_in_object, "manifestYamlStream", 1)?;
        let c_document_end =
            self.expect_std_func_arg_bool(c_document_end, "manifestYamlStream", 2)?;
        let quote_keys = self.expect_std_func_arg_bool(quote_keys, "manifestYamlStream", 3)?;

        self.string_stack.push("---\n".into());
        self.state_stack.push(State::StringToValue);

        if c_document_end {
            self.state_stack
                .push(State::AppendToString("\n...\n".into()));
        } else {
            self.state_stack.push(State::AppendToString('\n'.into()));
        }

        for (i, item) in array.iter().rev().enumerate() {
            if i != 0 {
                self.state_stack
                    .push(State::AppendToString("\n---\n".into()));
            }
            self.state_stack.push(State::ManifestYamlDoc {
                indent_array_in_object,
                quote_keys,
                depth: 0,
                parent_is_array: false,
                parent_is_object: false,
            });
            self.state_stack.push(State::DoThunk(item.view()));
        }

        Ok(())
    }

    pub(super) fn do_std_manifest_xml_jsonml(&mut self) -> EvalResult<()> {
        let value = self.value_stack.pop().unwrap();
        let array = self.expect_std_func_arg_array(value, "manifestXmlJsonml", 0)?;

        self.string_stack.push(String::new());
        self.state_stack.push(State::StringToValue);
        self.prepare_manifest_xml_jsonml_array(array)?;

        Ok(())
    }

    fn prepare_manifest_xml_jsonml_array(
        &mut self,
        array: GcView<ArrayData<'p>>,
    ) -> EvalResult<()> {
        let Some(item0) = array.first() else {
            return Err(self.report_error(EvalErrorKind::Other {
                span: None,
                message: "JsonML array is empty".into(),
            }));
        };
        let item0 = item0.view();

        self.state_stack
            .push(State::StdManifestXmlJsonmlItem0 { array });
        self.state_stack.push(State::DoThunk(item0));

        Ok(())
    }

    pub(super) fn do_std_manifest_xml_jsonml_item_0(
        &mut self,
        array: GcView<ArrayData<'p>>,
    ) -> EvalResult<()> {
        let item_value = self.value_stack.pop().unwrap();
        let ValueData::String(tag) = item_value else {
            return Err(self.report_error(EvalErrorKind::Other {
                span: None,
                message: format!(
                    "first element of JsonML array must be a string, got {}",
                    EvalErrorValueType::from_value(&item_value).to_str(),
                ),
            }));
        };

        let result = self.string_stack.last_mut().unwrap();
        result.push('<');
        result.push_str(&tag);

        self.state_stack
            .push(State::AppendToString(format!("</{tag}>")));

        if let Some(item1) = array.get(1) {
            for item in array[2..].iter().rev() {
                self.state_stack.push(State::StdManifestXmlJsonmlItemN);
                self.state_stack.push(State::DoThunk(item.view()));
            }
            self.state_stack.push(State::StdManifestXmlJsonmlItem1);
            self.state_stack.push(State::DoThunk(item1.view()));
        } else {
            result.push('>');
        }

        Ok(())
    }

    pub(super) fn do_std_manifest_xml_jsonml_item_1(&mut self) -> EvalResult<()> {
        let item_value = self.value_stack.pop().unwrap();
        match item_value {
            ValueData::String(s) => {
                let result = self.string_stack.last_mut().unwrap();
                result.push('>');
                result.push_str(&s);
                Ok(())
            }
            ValueData::Array(array) => {
                let result = self.string_stack.last_mut().unwrap();
                result.push('>');

                self.prepare_manifest_xml_jsonml_array(array.view())?;
                Ok(())
            }
            ValueData::Object(object) => {
                self.state_stack.push(State::AppendToString('>'.into()));

                let object = object.view();
                let visible_fields = object.get_visible_fields_order();

                for field_name in visible_fields.rev() {
                    let field_thunk = self
                        .program
                        .find_object_field_thunk(&object, 0, field_name)
                        .unwrap();

                    self.state_stack.push(State::AppendToString('"'.into()));
                    self.state_stack.push(State::CoerceAppendToString);
                    self.state_stack.push(State::DoThunk(field_thunk));
                    self.state_stack
                        .push(State::AppendToString(format!(" {}=\"", field_name.value())));
                }

                Ok(())
            }
            _ => Err(self.report_error(EvalErrorKind::Other {
                span: None,
                message: format!(
                    "second element of JsonML array must be a string, array or object, got {}",
                    EvalErrorValueType::from_value(&item_value).to_str(),
                ),
            })),
        }
    }

    pub(super) fn do_std_manifest_xml_jsonml_item_n(&mut self) -> EvalResult<()> {
        let item_value = self.value_stack.pop().unwrap();
        match item_value {
            ValueData::String(s) => {
                let result = self.string_stack.last_mut().unwrap();
                result.push_str(&s);
                Ok(())
            }
            ValueData::Array(array) => self.prepare_manifest_xml_jsonml_array(array.view()),
            _ => Err(self.report_error(EvalErrorKind::Other {
                span: None,
                message: format!(
                    "second element of JsonML array must be a string or array, got {}",
                    EvalErrorValueType::from_value(&item_value).to_str(),
                ),
            })),
        }
    }

    pub(super) fn do_std_manifest_toml_ex(&mut self) -> EvalResult<()> {
        let indent = self.value_stack.pop().unwrap();
        let value = self.value_stack.pop().unwrap();

        let object = self.expect_std_func_arg_object(value, "manifestTomlEx", 0)?;
        let indent = self.expect_std_func_arg_string(indent, "manifestTomlEx", 1)?;

        self.string_stack.push(String::new());
        self.state_stack.push(State::StringToValue);

        self.prepare_manifest_toml_table(object, false, Rc::new([]), indent);

        Ok(())
    }

    pub(super) fn do_std_make_array(&mut self) -> EvalResult<()> {
        let func_value = self.value_stack.pop().unwrap();
        let sz_value = self.value_stack.pop().unwrap();

        let length = self.expect_std_func_arg_number(sz_value, "makeArray", 0)?;
        let func = self.expect_std_func_arg_func(func_value, "makeArray", 1)?;

        let Some(length) = float::try_to_i32_exact(length).and_then(|v| usize::try_from(v).ok())
        else {
            return Err(self.report_error(EvalErrorKind::Other {
                span: None,
                message: format!("invalid size value {length}"),
            }));
        };

        if func.params.order.len() != 1 {
            return Err(self.report_error(EvalErrorKind::Other {
                span: None,
                message: format!(
                    "function must have exactly 1 parameter, got {}",
                    func.params.order.len(),
                ),
            }));
        }

        let mut array = Vec::with_capacity(length);
        for i in 0..length {
            let args_thunks = Box::new([self
                .program
                .gc_alloc(ThunkData::new_done(ValueData::Number(i as f64)))]);

            array.push(
                self.program
                    .gc_alloc(ThunkData::new_pending_call(Gc::from(&func), args_thunks)),
            );
        }

        self.value_stack.push(ValueData::Array(
            self.program.gc_alloc(array.into_boxed_slice()),
        ));

        Ok(())
    }

    pub(super) fn do_std_member(&mut self, value: GcView<ThunkData<'p>>) -> EvalResult<()> {
        let arr_or_str = self.value_stack.pop().unwrap();
        match arr_or_str {
            ValueData::String(s) => {
                self.state_stack.push(State::StdMemberString { string: s });
                self.state_stack.push(State::DoThunk(value));
                Ok(())
            }
            ValueData::Array(array) => {
                let array = array.view();
                if let Some(item0) = array.first() {
                    let item0 = item0.view();
                    self.state_stack
                        .push(State::StdMemberArray { array, index: 0 });
                    self.state_stack.push(State::EqualsValue);
                    self.state_stack.push(State::DoThunk(item0));
                    self.state_stack.push(State::DoThunk(value.clone()));
                    self.state_stack.push(State::DoThunk(value));
                } else {
                    self.value_stack.push(ValueData::Bool(false));
                }
                Ok(())
            }
            _ => Err(self.report_error(EvalErrorKind::InvalidStdFuncArgType {
                func_name: "member".into(),
                arg_index: 0,
                expected_types: vec![EvalErrorValueType::String, EvalErrorValueType::Array],
                got_type: EvalErrorValueType::from_value(&arr_or_str),
            })),
        }
    }

    pub(super) fn do_std_member_string(&mut self, string: Rc<str>) -> EvalResult<()> {
        let needle = self.value_stack.pop().unwrap();
        let needle = self.expect_std_func_arg_string(needle, "member", 1)?;

        let found = string.find(&*needle).is_some();
        self.value_stack.push(ValueData::Bool(found));

        Ok(())
    }

    pub(super) fn do_std_member_array(&mut self, array: GcView<ArrayData<'p>>, index: usize) {
        let equal = self.bool_stack.pop().unwrap();
        if equal {
            *self.value_stack.last_mut().unwrap() = ValueData::Bool(true);
        } else {
            let next_index = index + 1;
            if let Some(next_item) = array.get(next_index) {
                let next_item = next_item.view();
                self.value_stack
                    .push(self.value_stack.last().unwrap().clone());
                self.state_stack.push(State::StdMemberArray {
                    array,
                    index: next_index,
                });
                self.state_stack.push(State::EqualsValue);
                self.state_stack.push(State::DoThunk(next_item));
            } else {
                *self.value_stack.last_mut().unwrap() = ValueData::Bool(false);
            }
        }
    }

    pub(super) fn do_std_count(&mut self, value: GcView<ThunkData<'p>>) -> EvalResult<()> {
        let array = self.value_stack.pop().unwrap();
        let array = self.expect_std_func_arg_array(array, "count", 0)?;

        if let Some(item0) = array.first() {
            let item0 = item0.view();

            self.state_stack.push(State::StdCountInner {
                array,
                index: 0,
                count: 0,
            });
            self.state_stack.push(State::EqualsValue);
            self.state_stack.push(State::DoThunk(item0));
            self.state_stack.push(State::DoThunk(value.clone()));
            self.state_stack.push(State::DoThunk(value));
        } else {
            self.value_stack.push(ValueData::Number(0.0));
        }

        Ok(())
    }

    pub(super) fn do_std_count_inner(
        &mut self,
        array: GcView<ArrayData<'p>>,
        index: usize,
        mut count: usize,
    ) {
        let equal = self.bool_stack.pop().unwrap();
        if equal {
            count += 1;
        }

        let next_index = index + 1;
        if let Some(next_item) = array.get(next_index) {
            let next_item = next_item.view();
            self.value_stack
                .push(self.value_stack.last().unwrap().clone());
            self.state_stack.push(State::StdCountInner {
                array,
                index: next_index,
                count,
            });
            self.state_stack.push(State::EqualsValue);
            self.state_stack.push(State::DoThunk(next_item));
        } else {
            *self.value_stack.last_mut().unwrap() = ValueData::Number(count as f64);
        }
    }

    pub(super) fn do_std_find(&mut self, value: GcView<ThunkData<'p>>) -> EvalResult<()> {
        let array = self.value_stack.pop().unwrap();
        let array = self.expect_std_func_arg_array(array, "find", 1)?;

        if let Some(item0) = array.first() {
            let item0 = item0.view();

            self.array_stack.push(Vec::new());
            self.state_stack
                .push(State::StdFindInner { array, index: 0 });
            self.state_stack.push(State::EqualsValue);
            self.state_stack.push(State::DoThunk(item0));
            self.state_stack.push(State::DoThunk(value.clone()));
            self.state_stack.push(State::DoThunk(value));
        } else {
            self.value_stack
                .push(ValueData::Array(Gc::from(&self.program.empty_array)));
        }

        Ok(())
    }

    pub(super) fn do_std_find_inner(&mut self, array: GcView<ArrayData<'p>>, index: usize) {
        let equal = self.bool_stack.pop().unwrap();
        if equal {
            self.array_stack.last_mut().unwrap().push(
                self.program
                    .gc_alloc(ThunkData::new_done(ValueData::Number(index as f64))),
            );
        }

        let next_index = index + 1;
        if let Some(next_item) = array.get(next_index) {
            let next_item = next_item.view();
            self.value_stack
                .push(self.value_stack.last().unwrap().clone());
            self.state_stack.push(State::StdFindInner {
                array,
                index: next_index,
            });
            self.state_stack.push(State::EqualsValue);
            self.state_stack.push(State::DoThunk(next_item));
        } else {
            let result = self.array_stack.pop().unwrap();
            *self.value_stack.last_mut().unwrap() =
                ValueData::Array(self.program.gc_alloc(result.into_boxed_slice()));
        }
    }

    pub(super) fn do_std_map(&mut self) -> EvalResult<()> {
        let arr_value = self.value_stack.pop().unwrap();
        let func_value = self.value_stack.pop().unwrap();

        let func = self.expect_std_func_arg_func(func_value, "map", 0)?;

        match arr_value {
            ValueData::String(s) => {
                let mut array = Vec::new();
                for chr in s.chars() {
                    let args_thunks = Box::new([self
                        .program
                        .gc_alloc(ThunkData::new_done(ValueData::from_char(chr)))]);

                    array.push(
                        self.program
                            .gc_alloc(ThunkData::new_pending_call(Gc::from(&func), args_thunks)),
                    );
                }

                self.value_stack.push(ValueData::Array(
                    self.program.gc_alloc(array.into_boxed_slice()),
                ));

                Ok(())
            }
            ValueData::Array(array) => {
                let array = array.view();

                let mut new_array = Vec::with_capacity(array.len());
                for item in array.iter() {
                    let args_thunks = Box::new([item.clone()]);

                    new_array.push(
                        self.program
                            .gc_alloc(ThunkData::new_pending_call(Gc::from(&func), args_thunks)),
                    );
                }

                self.value_stack.push(ValueData::Array(
                    self.program.gc_alloc(new_array.into_boxed_slice()),
                ));

                Ok(())
            }
            _ => Err(self.report_error(EvalErrorKind::InvalidStdFuncArgType {
                func_name: "map".into(),
                arg_index: 1,
                expected_types: vec![EvalErrorValueType::String, EvalErrorValueType::Array],
                got_type: EvalErrorValueType::from_value(&arr_value),
            })),
        }
    }

    pub(super) fn do_std_map_with_index(&mut self) -> EvalResult<()> {
        let arr_value = self.value_stack.pop().unwrap();
        let func_value = self.value_stack.pop().unwrap();

        let func = self.expect_std_func_arg_func(func_value, "mapWithIndex", 0)?;

        match arr_value {
            ValueData::String(s) => {
                let mut array = Vec::new();
                for (i, chr) in s.chars().enumerate() {
                    let args_thunks = Box::new([
                        self.program
                            .gc_alloc(ThunkData::new_done(ValueData::Number(i as f64))),
                        self.program
                            .gc_alloc(ThunkData::new_done(ValueData::from_char(chr))),
                    ]);

                    array.push(
                        self.program
                            .gc_alloc(ThunkData::new_pending_call(Gc::from(&func), args_thunks)),
                    );
                }

                self.value_stack.push(ValueData::Array(
                    self.program.gc_alloc(array.into_boxed_slice()),
                ));

                Ok(())
            }
            ValueData::Array(array) => {
                let array = array.view();

                let mut new_array = Vec::with_capacity(array.len());
                for (i, item) in array.iter().enumerate() {
                    let args_thunks = Box::new([
                        self.program
                            .gc_alloc(ThunkData::new_done(ValueData::Number(i as f64))),
                        item.clone(),
                    ]);

                    new_array.push(
                        self.program
                            .gc_alloc(ThunkData::new_pending_call(Gc::from(&func), args_thunks)),
                    );
                }

                self.value_stack.push(ValueData::Array(
                    self.program.gc_alloc(new_array.into_boxed_slice()),
                ));

                Ok(())
            }
            _ => Err(self.report_error(EvalErrorKind::InvalidStdFuncArgType {
                func_name: "mapWithIndex".into(),
                arg_index: 1,
                expected_types: vec![EvalErrorValueType::String, EvalErrorValueType::Array],
                got_type: EvalErrorValueType::from_value(&arr_value),
            })),
        }
    }

    pub(super) fn do_std_filter_map(&mut self) -> EvalResult<()> {
        let array = self.value_stack.pop().unwrap();
        let map_func = self.value_stack.pop().unwrap();
        let filter_func = self.value_stack.pop().unwrap();

        let filter_func = self.expect_std_func_arg_func(filter_func, "filterMap", 0)?;
        let map_func = self.expect_std_func_arg_func(map_func, "filterMap", 1)?;
        let array = self.expect_std_func_arg_array(array, "filterMap", 2)?;

        self.state_stack.push(State::ArrayToValue);
        self.array_stack.push(Vec::new());

        for item in array.iter().rev() {
            self.state_stack.push(State::StdFilterMapCheck {
                item: item.view(),
                map_func: map_func.clone(),
            });
            self.check_thunk_args_and_execute_call(&filter_func, &[item.view()], &[], None)?;
        }

        Ok(())
    }

    pub(super) fn do_std_filter_map_check(
        &mut self,
        item: GcView<ThunkData<'p>>,
        map_func: GcView<FuncData<'p>>,
    ) -> EvalResult<()> {
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
            let array = self.array_stack.last_mut().unwrap();

            let args_thunks = Box::new([Gc::from(&item)]);

            array.push(self.program.gc_alloc(ThunkData::new_pending_call(
                Gc::from(&map_func),
                args_thunks,
            )));
        }

        Ok(())
    }

    pub(super) fn do_std_flat_map(&mut self) -> EvalResult<()> {
        let arr = self.value_stack.pop().unwrap();
        let func = self.value_stack.pop().unwrap();

        let func = self.expect_std_func_arg_func(func, "flatMap", 0)?;

        match arr {
            ValueData::String(s) => {
                self.string_stack.push(String::new());
                self.state_stack.push(State::StringToValue);

                for chr in s.chars().rev() {
                    let args_thunks = Box::new([self
                        .program
                        .gc_alloc(ThunkData::new_done(ValueData::from_char(chr)))]);

                    self.state_stack
                        .push(State::FnFallible(Self::do_std_flat_map_string_part));
                    self.execute_call(&func, args_thunks);
                }

                Ok(())
            }
            ValueData::Array(array) => {
                let array = array.view();

                self.array_stack.push(Vec::new());
                self.state_stack.push(State::ArrayToValue);

                for item in array.iter().rev() {
                    let args_thunks = Box::new([item.clone()]);

                    self.state_stack
                        .push(State::FnFallible(Self::do_std_flat_map_array_part));
                    self.execute_call(&func, args_thunks);
                }

                Ok(())
            }
            _ => Err(self.report_error(EvalErrorKind::InvalidStdFuncArgType {
                func_name: "flatMap".into(),
                arg_index: 1,
                expected_types: vec![EvalErrorValueType::String, EvalErrorValueType::Array],
                got_type: EvalErrorValueType::from_value(&arr),
            })),
        }
    }

    fn do_std_flat_map_string_part(&mut self) -> EvalResult<()> {
        let value = self.value_stack.pop().unwrap();

        match value {
            ValueData::Null => Ok(()),
            ValueData::String(s) => {
                self.string_stack.last_mut().unwrap().push_str(&s);
                Ok(())
            }
            _ => Err(self.report_error(EvalErrorKind::Other {
                span: None,
                message: format!(
                    "function must return a string, got {}",
                    EvalErrorValueType::from_value(&value).to_str(),
                ),
            })),
        }
    }

    fn do_std_flat_map_array_part(&mut self) -> EvalResult<()> {
        let value = self.value_stack.pop().unwrap();
        let ValueData::Array(sub_array) = value else {
            return Err(self.report_error(EvalErrorKind::Other {
                span: None,
                message: format!(
                    "function must return an array, got {}",
                    EvalErrorValueType::from_value(&value).to_str(),
                ),
            }));
        };
        let sub_array = sub_array.view();

        self.array_stack
            .last_mut()
            .unwrap()
            .extend(sub_array.iter().cloned());

        Ok(())
    }

    pub(super) fn do_std_filter(&mut self) -> EvalResult<()> {
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

        Ok(())
    }

    pub(super) fn do_std_filter_check(&mut self, item: GcView<ThunkData<'p>>) -> EvalResult<()> {
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

        Ok(())
    }

    pub(super) fn do_std_foldl(&mut self, init: GcView<ThunkData<'p>>) -> EvalResult<()> {
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

            self.check_thunk_args_and_execute_call(&func, &[init, item0.view()], &[], None)?;
        }

        Ok(())
    }

    pub(super) fn do_std_foldl_item(
        &mut self,
        func: GcView<FuncData<'p>>,
        item: GcView<ThunkData<'p>>,
    ) -> EvalResult<()> {
        let acc = self.value_stack.pop().unwrap();
        let acc = self.program.gc_alloc_view(ThunkData::new_done(acc));

        self.check_thunk_args_and_execute_call(&func, &[acc, item], &[], None)?;

        Ok(())
    }

    pub(super) fn do_std_foldr(&mut self, init: GcView<ThunkData<'p>>) -> EvalResult<()> {
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

            self.check_thunk_args_and_execute_call(&func, &[last_item.view(), init], &[], None)?;
        }

        Ok(())
    }

    pub(super) fn do_std_foldr_item(
        &mut self,
        func: GcView<FuncData<'p>>,
        item: GcView<ThunkData<'p>>,
    ) -> EvalResult<()> {
        let acc = self.value_stack.pop().unwrap();
        let acc = self.program.gc_alloc_view(ThunkData::new_done(acc));

        self.check_thunk_args_and_execute_call(&func, &[item, acc], &[], None)?;

        Ok(())
    }

    pub(super) fn do_std_range(&mut self) -> EvalResult<()> {
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

        Ok(())
    }

    pub(super) fn do_std_repeat(&mut self) -> EvalResult<()> {
        let count = self.value_stack.pop().unwrap();
        let value = self.value_stack.pop().unwrap();

        let count = self.expect_std_func_arg_number(count, "repeat", 1)?;

        let Some(count) = float::try_to_i32_exact(count).and_then(|v| usize::try_from(v).ok())
        else {
            return Err(self.report_error(EvalErrorKind::Other {
                span: None,
                message: format!("invalid count value {count}"),
            }));
        };

        match value {
            ValueData::String(s) => {
                self.value_stack
                    .push(ValueData::String(s.repeat(count).into()));
                Ok(())
            }
            ValueData::Array(array) => {
                let array = array.view();
                self.value_stack.push(ValueData::Array(
                    self.program.gc_alloc(
                        std::iter::repeat(&array)
                            .take(count)
                            .flat_map(|a| a.iter())
                            .cloned()
                            .collect(),
                    ),
                ));
                Ok(())
            }
            _ => Err(self.report_error(EvalErrorKind::InvalidStdFuncArgType {
                func_name: "repeat".into(),
                arg_index: 0,
                expected_types: vec![EvalErrorValueType::String, EvalErrorValueType::Array],
                got_type: EvalErrorValueType::from_value(&value),
            })),
        }
    }

    pub(super) fn do_std_slice(&mut self) -> EvalResult<()> {
        let step = self.value_stack.pop().unwrap();
        let end = self.value_stack.pop().unwrap();
        let start = self.value_stack.pop().unwrap();
        let indexable = self.value_stack.pop().unwrap();

        let start = self.expect_std_func_arg_null_or_number(start, "slice", 1)?;
        let end = self.expect_std_func_arg_null_or_number(end, "slice", 2)?;
        let step = self.expect_std_func_arg_null_or_number(step, "slice", 3)?;

        self.do_slice(indexable, start, end, step, true, None)
    }

    pub(super) fn do_std_join(&mut self) -> EvalResult<()> {
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
                Ok(())
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
                Ok(())
            }
            _ => Err(self.report_error(EvalErrorKind::InvalidStdFuncArgType {
                func_name: "join".into(),
                arg_index: 0,
                expected_types: vec![EvalErrorValueType::String, EvalErrorValueType::Array],
                got_type: EvalErrorValueType::from_value(&sep),
            })),
        }
    }

    pub(super) fn do_std_join_str_item(&mut self, sep: Rc<str>) -> EvalResult<()> {
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
        Ok(())
    }

    pub(super) fn do_std_join_str_finish(&mut self) {
        let s = self.string_stack.pop().unwrap();
        self.bool_stack.pop().unwrap();
        self.value_stack.push(ValueData::String(s.into()));
    }

    pub(super) fn do_std_join_array_item(&mut self, sep: GcView<ArrayData<'p>>) -> EvalResult<()> {
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
        Ok(())
    }

    pub(super) fn do_std_join_array_finish(&mut self) {
        let array = self.array_stack.pop().unwrap();
        self.bool_stack.pop().unwrap();
        self.value_stack.push(ValueData::Array(
            self.program.gc_alloc(array.into_boxed_slice()),
        ));
    }

    pub(super) fn do_std_deep_join(&mut self) -> EvalResult<()> {
        match self.value_stack.pop().unwrap() {
            ValueData::String(s) => {
                self.value_stack.push(ValueData::String(s));
                Ok(())
            }
            ValueData::Array(array) => {
                let array = array.view();

                self.string_stack.push(String::new());
                self.state_stack.push(State::StringToValue);

                for item in array.iter().rev() {
                    self.state_stack
                        .push(State::FnFallible(Self::do_std_deep_join_array_item));
                    self.state_stack.push(State::DoThunk(item.view()));
                }

                Ok(())
            }
            value => Err(self.report_error(EvalErrorKind::InvalidStdFuncArgType {
                func_name: "deepJoin".into(),
                arg_index: 0,
                expected_types: vec![EvalErrorValueType::String, EvalErrorValueType::Array],
                got_type: EvalErrorValueType::from_value(&value),
            })),
        }
    }

    fn do_std_deep_join_array_item(&mut self) -> EvalResult<()> {
        match self.value_stack.pop().unwrap() {
            ValueData::String(s) => {
                self.string_stack.last_mut().unwrap().push_str(&s);
                Ok(())
            }
            ValueData::Array(array) => {
                let array = array.view();

                for item in array.iter().rev() {
                    self.state_stack
                        .push(State::FnFallible(Self::do_std_deep_join_array_item));
                    self.state_stack.push(State::DoThunk(item.view()));
                }

                Ok(())
            }
            value => Err(self.report_error(EvalErrorKind::Other {
                span: None,
                message: format!(
                    "array item must be a string or array, got {}",
                    EvalErrorValueType::from_value(&value).to_str(),
                ),
            })),
        }
    }

    pub(super) fn do_std_flatten_arrays(&mut self) -> EvalResult<()> {
        let arrs = self.value_stack.pop().unwrap();
        let arrs = self.expect_std_func_arg_array(arrs, "flattenArrays", 0)?;

        self.array_stack.push(Vec::new());
        self.state_stack.push(State::ArrayToValue);

        for item in arrs.iter().rev() {
            self.state_stack
                .push(State::FnFallible(Self::do_std_flatten_arrays_item));
            self.state_stack.push(State::DoThunk(item.view()));
        }

        Ok(())
    }

    fn do_std_flatten_arrays_item(&mut self) -> EvalResult<()> {
        let item = self.value_stack.pop().unwrap();
        let ValueData::Array(item) = item else {
            return Err(self.report_error(EvalErrorKind::Other {
                span: None,
                message: format!(
                    "array item must be an array, got {}",
                    EvalErrorValueType::from_value(&item).to_str(),
                ),
            }));
        };
        let item = item.view();

        let result_array = self.array_stack.last_mut().unwrap();
        result_array.extend(item.iter().cloned());

        Ok(())
    }

    pub(super) fn do_std_flatten_deep_array(&mut self) {
        let value = self.value_stack.pop().unwrap();

        if let ValueData::Array(array) = value {
            let array = array.view();
            if let Some(item0) = array.first() {
                let item0 = item0.view();
                self.array_stack.push(Vec::new());
                self.state_stack.push(State::ArrayToValue);
                self.state_stack
                    .push(State::StdFlattenDeepArrayItem { array, index: 0 });
                self.state_stack.push(State::DoThunk(item0));
            } else {
                self.value_stack
                    .push(ValueData::Array(Gc::from(&self.program.empty_array)));
            }
        } else {
            self.value_stack.push(ValueData::Array(
                self.program.make_value_array(std::iter::once(value)),
            ));
        }
    }

    pub(super) fn do_std_flatten_deep_array_item(
        &mut self,
        array: GcView<ArrayData<'p>>,
        index: usize,
    ) {
        let item_value = self.value_stack.pop().unwrap();

        let next_index = index + 1;
        if let Some(next_item) = array.get(next_index) {
            let next_item = next_item.view();
            self.state_stack.push(State::StdFlattenDeepArrayItem {
                array: array.clone(),
                index: next_index,
            });
            self.state_stack.push(State::DoThunk(next_item));
        }

        if let ValueData::Array(sub_array) = item_value {
            let sub_array = sub_array.view();
            if let Some(sub_item0) = sub_array.first() {
                let sub_item0 = sub_item0.view();
                self.state_stack.push(State::StdFlattenDeepArrayItem {
                    array: sub_array,
                    index: 0,
                });
                self.state_stack.push(State::DoThunk(sub_item0));
            }
        } else {
            self.array_stack
                .last_mut()
                .unwrap()
                .push(array[index].clone());
        }
    }

    pub(super) fn do_std_reverse(&mut self) -> EvalResult<()> {
        let value = self.value_stack.pop().unwrap();
        let reverse = match value {
            ValueData::String(s) => ValueData::Array(
                self.program
                    .make_value_array(s.chars().rev().map(ValueData::from_char)),
            ),
            ValueData::Array(array) => ValueData::Array(
                self.program
                    .gc_alloc(array.view().iter().rev().cloned().collect()),
            ),
            _ => {
                return Err(self.report_error(EvalErrorKind::InvalidStdFuncArgType {
                    func_name: "reverse".into(),
                    arg_index: 0,
                    expected_types: vec![EvalErrorValueType::String, EvalErrorValueType::Array],
                    got_type: EvalErrorValueType::from_value(&value),
                }));
            }
        };
        self.value_stack.push(reverse);
        Ok(())
    }

    pub(super) fn do_std_sort(&mut self) -> EvalResult<()> {
        let keyf = self.value_stack.pop().unwrap();
        let arr = self.value_stack.pop().unwrap();

        let array = self.expect_std_func_arg_array(arr, "sort", 0)?;
        let keyf = self.expect_std_func_arg_func(keyf, "sort", 1)?;

        let array_len = array.len();
        if array_len <= 1 {
            self.value_stack.push(ValueData::Array(Gc::from(&array)));
        } else {
            let keys: Rc<Vec<_>> = Rc::new((0..array_len).map(|_| OnceCell::new()).collect());

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
                self.check_thunk_args_and_execute_call(&keyf, &[array[i].view()], &[], None)?;
            }
        }

        Ok(())
    }

    pub(super) fn do_std_sort_set_key(
        &mut self,
        keys: Rc<Vec<OnceCell<ValueData<'p>>>>,
        index: usize,
    ) {
        let value = self.value_stack.pop().unwrap();
        keys[index].set(value).ok().unwrap();
    }

    pub(super) fn do_std_sort_compare(
        &mut self,
        keys: Rc<Vec<OnceCell<ValueData<'p>>>>,
        lhs: usize,
        rhs: usize,
    ) {
        self.value_stack.push(keys[lhs].get().unwrap().clone());
        self.value_stack.push(keys[rhs].get().unwrap().clone());
        self.state_stack.push(State::CompareValue);
    }

    pub(super) fn do_std_sort_slice(
        &mut self,
        keys: Rc<Vec<OnceCell<ValueData<'p>>>>,
        sorted: Rc<Vec<Cell<usize>>>,
        range: std::ops::Range<usize>,
    ) {
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

    pub(super) fn do_std_sort_quick_sort_1(
        &mut self,
        keys: Rc<Vec<OnceCell<ValueData<'p>>>>,
        sorted: Rc<Vec<Cell<usize>>>,
        range: std::ops::Range<usize>,
    ) {
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

    pub(super) fn do_std_sort_quick_sort_2(
        &mut self,
        keys: Rc<Vec<OnceCell<ValueData<'p>>>>,
        sorted: Rc<Vec<Cell<usize>>>,
        range: std::ops::Range<usize>,
    ) {
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

    pub(super) fn do_std_sort_merge_prepare(
        &mut self,
        keys: Rc<Vec<OnceCell<ValueData<'p>>>>,
        sorted: Rc<Vec<Cell<usize>>>,
        range: std::ops::Range<usize>,
        mid: usize,
    ) {
        let left = sorted[range.start..mid].iter().map(Cell::get).collect();
        let right = sorted[mid..range.end].iter().map(Cell::get).collect();

        self.state_stack.push(State::StdSortMergePreCompare {
            keys,
            sorted,
            start: range.start,
            unmerged: Rc::new((Cell::new(0), left, Cell::new(0), right)),
        });
    }

    pub(super) fn do_std_sort_merge_pre_compare(
        &mut self,
        keys: Rc<Vec<OnceCell<ValueData<'p>>>>,
        sorted: Rc<Vec<Cell<usize>>>,
        start: usize,
        unmerged: Rc<(Cell<usize>, Box<[usize]>, Cell<usize>, Box<[usize]>)>,
    ) {
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

    pub(super) fn do_std_sort_merge_post_compare(
        &mut self,
        keys: Rc<Vec<OnceCell<ValueData<'p>>>>,
        sorted: Rc<Vec<Cell<usize>>>,
        start: usize,
        unmerged: Rc<(Cell<usize>, Box<[usize]>, Cell<usize>, Box<[usize]>)>,
    ) {
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

    pub(super) fn do_std_sort_finish(
        &mut self,
        orig_array: GcView<ArrayData<'p>>,
        sorted: Rc<Vec<Cell<usize>>>,
    ) {
        let new_array = sorted.iter().map(|i| orig_array[i.get()].clone()).collect();
        self.value_stack
            .push(ValueData::Array(self.program.gc_alloc(new_array)));
    }

    pub(super) fn do_std_uniq(&mut self) -> EvalResult<()> {
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

            self.check_thunk_args_and_execute_call(&keyf, &[array[0].view()], &[], None)?;
        }

        Ok(())
    }

    pub(super) fn do_std_uniq_compare_item(
        &mut self,
        keyf: GcView<FuncData<'p>>,
        item: GcView<ThunkData<'p>>,
        is_last: bool,
    ) -> EvalResult<()> {
        self.state_stack
            .push(State::StdUniqCheckItem { item: item.clone() });
        self.state_stack.push(State::EqualsValue);
        if !is_last {
            self.state_stack.push(State::StdUniqDupValue);
        }
        self.check_thunk_args_and_execute_call(&keyf, &[item], &[], None)?;
        Ok(())
    }

    pub(super) fn do_std_uniq_dup_value(&mut self) {
        let value = self.value_stack.last().unwrap().clone();
        self.value_stack.insert(self.value_stack.len() - 2, value);
    }

    pub(super) fn do_std_uniq_check_item(&mut self, item: GcView<ThunkData<'p>>) {
        let is_equal = self.bool_stack.pop().unwrap();
        if !is_equal {
            let array = self.array_stack.last_mut().unwrap();
            array.push(Gc::from(&item));
        }
    }

    pub(super) fn do_std_all(&mut self) -> EvalResult<()> {
        let arr = self.value_stack.pop().unwrap();

        let array = self.expect_std_func_arg_array(arr, "all", 0)?;
        if array.is_empty() {
            self.value_stack.push(ValueData::Bool(true));
        } else {
            let item_thunk = array[0].view();
            self.state_stack.push(State::StdAllItem { array, index: 0 });
            self.state_stack.push(State::DoThunk(item_thunk));
        }

        Ok(())
    }

    pub(super) fn do_std_all_item(
        &mut self,
        array: GcView<ArrayData<'p>>,
        index: usize,
    ) -> EvalResult<()> {
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

        Ok(())
    }

    pub(super) fn do_std_any(&mut self) -> EvalResult<()> {
        let arr = self.value_stack.pop().unwrap();

        let array = self.expect_std_func_arg_array(arr, "any", 0)?;
        if array.is_empty() {
            self.value_stack.push(ValueData::Bool(false));
        } else {
            let item_thunk = array[0].view();
            self.state_stack.push(State::StdAnyItem { array, index: 0 });
            self.state_stack.push(State::DoThunk(item_thunk));
        }

        Ok(())
    }

    pub(super) fn do_std_any_item(
        &mut self,
        array: GcView<ArrayData<'p>>,
        index: usize,
    ) -> EvalResult<()> {
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

        Ok(())
    }

    pub(super) fn do_std_sum(&mut self) -> EvalResult<()> {
        let arr = self.value_stack.pop().unwrap();

        let array = self.expect_std_func_arg_array(arr, "sum", 0)?;
        if array.is_empty() {
            self.value_stack.push(ValueData::Number(0.0));
        } else {
            let item_thunk = array[0].view();
            self.state_stack.push(State::StdSumItem {
                array,
                index: 0,
                sum: 0.0,
            });
            self.state_stack.push(State::DoThunk(item_thunk));
        }

        Ok(())
    }

    pub(super) fn do_std_sum_item(
        &mut self,
        array: GcView<ArrayData<'p>>,
        index: usize,
        sum: f64,
    ) -> EvalResult<()> {
        let item_value = self.value_stack.pop().unwrap();
        let ValueData::Number(item_value) = item_value else {
            return Err(self.report_error(EvalErrorKind::Other {
                span: None,
                message: format!(
                    "array item {index} must be a number, got {}",
                    EvalErrorValueType::from_value(&item_value).to_str(),
                ),
            }));
        };

        let sum = sum + item_value;
        let index = index + 1;
        if index == array.len() {
            self.value_stack.push(ValueData::Number(sum));
        } else {
            let item_thunk = array[index].view();
            self.state_stack
                .push(State::StdSumItem { array, index, sum });
            self.state_stack.push(State::DoThunk(item_thunk));
        }

        Ok(())
    }

    pub(super) fn do_std_avg(&mut self) -> EvalResult<()> {
        let arr = self.value_stack.pop().unwrap();

        let array = self.expect_std_func_arg_array(arr, "avg", 0)?;
        if array.is_empty() {
            return Err(self.report_error(EvalErrorKind::Other {
                span: None,
                message: "cannot calculate average of empty array".into(),
            }));
        } else {
            let item_thunk = array[0].view();
            self.state_stack.push(State::StdAvgItem {
                array,
                index: 0,
                sum: 0.0,
            });
            self.state_stack.push(State::DoThunk(item_thunk));
        }

        Ok(())
    }

    pub(super) fn do_std_avg_item(
        &mut self,
        array: GcView<ArrayData<'p>>,
        index: usize,
        sum: f64,
    ) -> EvalResult<()> {
        let item_value = self.value_stack.pop().unwrap();
        let ValueData::Number(item_value) = item_value else {
            return Err(self.report_error(EvalErrorKind::Other {
                span: None,
                message: format!(
                    "array item {index} must be a number, got {}",
                    EvalErrorValueType::from_value(&item_value).to_str(),
                ),
            }));
        };

        let sum = sum + item_value;
        let index = index + 1;
        if index == array.len() {
            self.value_stack
                .push(ValueData::Number(sum / (array.len() as f64)));
        } else {
            let item_thunk = array[index].view();
            self.state_stack
                .push(State::StdAvgItem { array, index, sum });
            self.state_stack.push(State::DoThunk(item_thunk));
        }

        Ok(())
    }

    pub(super) fn do_std_min_array(&mut self, on_empty: GcView<ThunkData<'p>>) -> EvalResult<()> {
        let keyf = self.value_stack.pop().unwrap();
        let arr = self.value_stack.pop().unwrap();

        let array = self.expect_std_func_arg_array(arr, "minArray", 0)?;
        let keyf = self.expect_std_func_arg_func(keyf, "minArray", 1)?;

        if array.is_empty() {
            self.state_stack.push(State::DoThunk(on_empty));
        } else if array.len() == 1 {
            self.state_stack.push(State::DoThunk(array[0].view()));
        } else {
            let item0 = array[0].view();
            let item1 = array[1].view();
            self.state_stack.push(State::StdMinArrayCompareItem {
                keyf: keyf.clone(),
                array,
                cur_index: 1,
                max_index: 0,
            });
            self.check_thunk_args_and_execute_call(&keyf, &[item1], &[], None)?;
            self.check_thunk_args_and_execute_call(&keyf, &[item0], &[], None)?;
        }

        Ok(())
    }

    pub(super) fn do_std_min_array_compare_item(
        &mut self,
        keyf: GcView<FuncData<'p>>,
        array: GcView<ArrayData<'p>>,
        cur_index: usize,
        max_index: usize,
    ) -> EvalResult<()> {
        self.value_stack
            .extend_from_within((self.value_stack.len() - 2)..);

        self.state_stack.push(State::StdMinArrayCheckItem {
            keyf: keyf.clone(),
            array,
            cur_index,
            max_index,
        });
        self.state_stack.push(State::CompareValue);

        Ok(())
    }

    pub(super) fn do_std_min_array_check_item(
        &mut self,
        keyf: GcView<FuncData<'p>>,
        array: GcView<ArrayData<'p>>,
        cur_index: usize,
        mut max_index: usize,
    ) -> EvalResult<()> {
        let cmp_ord = self.cmp_ord_stack.pop().unwrap();
        if cmp_ord.is_gt() {
            max_index = cur_index;
            self.value_stack.remove(self.value_stack.len() - 2);
        } else {
            self.value_stack.remove(self.value_stack.len() - 1);
        }

        let cur_index = cur_index + 1;
        if cur_index == array.len() {
            self.value_stack.pop().unwrap();
            self.state_stack
                .push(State::DoThunk(array[max_index].view()));
        } else {
            let item = array[cur_index].view();
            self.state_stack.push(State::StdMinArrayCompareItem {
                keyf: keyf.clone(),
                array,
                cur_index,
                max_index,
            });
            self.check_thunk_args_and_execute_call(&keyf, &[item], &[], None)?;
        }

        Ok(())
    }

    pub(super) fn do_std_max_array(&mut self, on_empty: GcView<ThunkData<'p>>) -> EvalResult<()> {
        let keyf = self.value_stack.pop().unwrap();
        let arr = self.value_stack.pop().unwrap();

        let array = self.expect_std_func_arg_array(arr, "maxArray", 0)?;
        let keyf = self.expect_std_func_arg_func(keyf, "maxArray", 1)?;

        if array.is_empty() {
            self.state_stack.push(State::DoThunk(on_empty));
        } else if array.len() == 1 {
            self.state_stack.push(State::DoThunk(array[0].view()));
        } else {
            let item0 = array[0].view();
            let item1 = array[1].view();
            self.state_stack.push(State::StdMaxArrayCompareItem {
                keyf: keyf.clone(),
                array,
                cur_index: 1,
                max_index: 0,
            });
            self.check_thunk_args_and_execute_call(&keyf, &[item1], &[], None)?;
            self.check_thunk_args_and_execute_call(&keyf, &[item0], &[], None)?;
        }

        Ok(())
    }

    pub(super) fn do_std_max_array_compare_item(
        &mut self,
        keyf: GcView<FuncData<'p>>,
        array: GcView<ArrayData<'p>>,
        cur_index: usize,
        max_index: usize,
    ) -> EvalResult<()> {
        self.value_stack
            .extend_from_within((self.value_stack.len() - 2)..);

        self.state_stack.push(State::StdMaxArrayCheckItem {
            keyf: keyf.clone(),
            array,
            cur_index,
            max_index,
        });
        self.state_stack.push(State::CompareValue);

        Ok(())
    }

    pub(super) fn do_std_max_array_check_item(
        &mut self,
        keyf: GcView<FuncData<'p>>,
        array: GcView<ArrayData<'p>>,
        cur_index: usize,
        mut max_index: usize,
    ) -> EvalResult<()> {
        let cmp_ord = self.cmp_ord_stack.pop().unwrap();
        if cmp_ord.is_lt() {
            max_index = cur_index;
            self.value_stack.remove(self.value_stack.len() - 2);
        } else {
            self.value_stack.remove(self.value_stack.len() - 1);
        }

        let cur_index = cur_index + 1;
        if cur_index == array.len() {
            self.value_stack.pop().unwrap();
            self.state_stack
                .push(State::DoThunk(array[max_index].view()));
        } else {
            let item = array[cur_index].view();
            self.state_stack.push(State::StdMaxArrayCompareItem {
                keyf: keyf.clone(),
                array,
                cur_index,
                max_index,
            });
            self.check_thunk_args_and_execute_call(&keyf, &[item], &[], None)?;
        }

        Ok(())
    }

    pub(super) fn do_std_contains(&mut self, value: GcView<ThunkData<'p>>) -> EvalResult<()> {
        let array = self.value_stack.pop().unwrap();
        let array = self.expect_std_func_arg_array(array, "contains", 0)?;

        if let Some(item0) = array.first() {
            let item0 = item0.view();

            self.state_stack
                .push(State::StdContainsItem { array, index: 0 });
            self.state_stack.push(State::EqualsValue);
            self.state_stack.push(State::DoThunk(item0));
            self.state_stack.push(State::DoThunk(value.clone()));
            self.state_stack.push(State::DoThunk(value));
        } else {
            self.value_stack.push(ValueData::Bool(false));
        }

        Ok(())
    }

    pub(super) fn do_std_contains_item(&mut self, array: GcView<ArrayData<'p>>, index: usize) {
        let equal = self.bool_stack.pop().unwrap();
        if equal {
            *self.value_stack.last_mut().unwrap() = ValueData::Bool(true);
        } else {
            let next_index = index + 1;
            if let Some(next_item) = array.get(next_index) {
                let next_item = next_item.view();
                self.value_stack
                    .push(self.value_stack.last().unwrap().clone());
                self.state_stack.push(State::StdContainsItem {
                    array,
                    index: next_index,
                });
                self.state_stack.push(State::EqualsValue);
                self.state_stack.push(State::DoThunk(next_item));
            } else {
                *self.value_stack.last_mut().unwrap() = ValueData::Bool(false);
            }
        }
    }

    pub(super) fn do_std_remove(&mut self, value: GcView<ThunkData<'p>>) -> EvalResult<()> {
        let array = self.value_stack.pop().unwrap();
        let array = self.expect_std_func_arg_array(array, "contains", 0)?;

        if let Some(item0) = array.first() {
            let item0 = item0.view();

            self.state_stack
                .push(State::StdRemoveCheckItem { array, index: 0 });
            self.state_stack.push(State::EqualsValue);
            self.state_stack.push(State::DoThunk(item0));
            self.state_stack.push(State::DoThunk(value.clone()));
            self.state_stack.push(State::DoThunk(value));
        } else {
            self.value_stack.push(ValueData::Array(Gc::from(&array)));
        }

        Ok(())
    }

    pub(super) fn do_std_remove_check_item(&mut self, array: GcView<ArrayData<'p>>, index: usize) {
        let equal = self.bool_stack.pop().unwrap();
        if equal {
            let new_array = self.program.make_thunk_array(
                array[..index]
                    .iter()
                    .cloned()
                    .chain(array[(index + 1)..].iter().cloned()),
            );
            *self.value_stack.last_mut().unwrap() = ValueData::Array(new_array);
        } else {
            let next_index = index + 1;
            if let Some(next_item) = array.get(next_index) {
                let next_item = next_item.view();
                self.value_stack
                    .push(self.value_stack.last().unwrap().clone());
                self.state_stack.push(State::StdRemoveCheckItem {
                    array,
                    index: next_index,
                });
                self.state_stack.push(State::EqualsValue);
                self.state_stack.push(State::DoThunk(next_item));
            } else {
                *self.value_stack.last_mut().unwrap() = ValueData::Array(Gc::from(&array));
            }
        }
    }

    pub(super) fn do_std_remove_at(&mut self) -> EvalResult<()> {
        let index = self.value_stack.pop().unwrap();
        let array = self.value_stack.pop().unwrap();

        let array = self.expect_std_func_arg_array(array, "containsAt", 0)?;
        let index_f = self.expect_std_func_arg_number(index, "containsAt", 1)?;

        let index = index_f as usize;
        if index_f.trunc() == index_f && index_f >= 0.0 && index < array.len() {
            let new_array = self.program.make_thunk_array(
                array[..index]
                    .iter()
                    .cloned()
                    .chain(array[(index + 1)..].iter().cloned()),
            );
            self.value_stack.push(ValueData::Array(new_array));
        } else {
            self.value_stack.push(ValueData::Array(Gc::from(&array)));
        }

        Ok(())
    }

    pub(super) fn do_std_set(&mut self) -> EvalResult<()> {
        let keyf = self.value_stack.pop().unwrap();
        let arr = self.value_stack.pop().unwrap();

        let array = self.expect_std_func_arg_array(arr, "set", 0)?;
        let keyf = self.expect_std_func_arg_func(keyf, "set", 1)?;

        let array_len = array.len();
        if array_len <= 1 {
            self.value_stack.push(ValueData::Array(Gc::from(&array)));
        } else {
            // Sort and then uniq, reusing the keyF evaluations from sort

            let keys: Rc<Vec<_>> = Rc::new((0..array_len).map(|_| OnceCell::new()).collect());

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
                self.check_thunk_args_and_execute_call(&keyf, &[array[i].view()], &[], None)?;
            }
        }

        Ok(())
    }

    pub(super) fn do_std_set_uniq(
        &mut self,
        orig_array: GcView<ArrayData<'p>>,
        keys: Rc<Vec<OnceCell<ValueData<'p>>>>,
        sorted: Rc<Vec<Cell<usize>>>,
    ) {
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

    pub(super) fn do_std_set_uniq_compare_item(
        &mut self,
        orig_array: GcView<ArrayData<'p>>,
        keys: Rc<Vec<OnceCell<ValueData<'p>>>>,
        sorted: Rc<Vec<Cell<usize>>>,
        index: usize,
    ) {
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

    pub(super) fn do_std_set_uniq_check_item(
        &mut self,
        orig_array: GcView<ArrayData<'p>>,
        sorted: Rc<Vec<Cell<usize>>>,
        index: usize,
    ) {
        let is_equal = self.bool_stack.pop().unwrap();
        if !is_equal {
            let array = self.array_stack.last_mut().unwrap();
            array.push(orig_array[sorted[index].get()].clone());
        }
    }

    pub(super) fn do_std_set_inter(&mut self) -> EvalResult<()> {
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

        Ok(())
    }

    pub(super) fn do_std_set_inter_aux(
        &mut self,
        keyf: GcView<FuncData<'p>>,
        a: GcView<ArrayData<'p>>,
        b: GcView<ArrayData<'p>>,
        mut i: usize,
        mut j: usize,
    ) -> EvalResult<()> {
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

        Ok(())
    }

    pub(super) fn do_std_set_union(&mut self) -> EvalResult<()> {
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

        Ok(())
    }

    pub(super) fn do_std_set_union_aux(
        &mut self,
        keyf: GcView<FuncData<'p>>,
        a: GcView<ArrayData<'p>>,
        b: GcView<ArrayData<'p>>,
        mut i: usize,
        mut j: usize,
    ) -> EvalResult<()> {
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

        Ok(())
    }

    pub(super) fn do_std_set_diff(&mut self) -> EvalResult<()> {
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

        Ok(())
    }

    pub(super) fn do_std_set_diff_aux(
        &mut self,
        keyf: GcView<FuncData<'p>>,
        a: GcView<ArrayData<'p>>,
        b: GcView<ArrayData<'p>>,
        mut i: usize,
        mut j: usize,
    ) -> EvalResult<()> {
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

        Ok(())
    }

    pub(super) fn do_std_set_member(&mut self, x: GcView<ThunkData<'p>>) -> EvalResult<()> {
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

        Ok(())
    }

    pub(super) fn do_std_set_member_slice(
        &mut self,
        keyf: GcView<FuncData<'p>>,
        arr: GcView<ArrayData<'p>>,
        start: usize,
        end: usize,
    ) -> EvalResult<()> {
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

        Ok(())
    }

    pub(super) fn do_std_set_member_check(
        &mut self,
        keyf: GcView<FuncData<'p>>,
        arr: GcView<ArrayData<'p>>,
        start: usize,
        end: usize,
        mid: usize,
    ) -> EvalResult<()> {
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

        Ok(())
    }

    pub(super) fn do_std_base64(&mut self) -> EvalResult<()> {
        let arg = self.value_stack.pop().unwrap();
        match arg {
            ValueData::String(s) => {
                let encoded = encode_base64(s.chars().map(|chr| {
                    u8::try_from(chr).map_err(|_| {
                        self.report_error(EvalErrorKind::Other {
                            span: None,
                            message: "only codepoints up to 255 can be base64 encoded".into(),
                        })
                    })
                }))?;
                self.value_stack.push(ValueData::String(encoded.into()));
                Ok(())
            }
            ValueData::Array(arr) => {
                let arr = arr.view();
                if let Some(first_item) = arr.first() {
                    let first_item = first_item.view();
                    let arr_len = arr.len();
                    self.state_stack.push(State::StdBase64Array {
                        input: arr,
                        bytes: Vec::with_capacity(arr_len),
                    });
                    self.state_stack.push(State::DoThunk(first_item));
                } else if arr.is_empty() {
                    self.value_stack.push(ValueData::String("".into()));
                }
                Ok(())
            }
            _ => Err(self.report_error(EvalErrorKind::InvalidStdFuncArgType {
                func_name: "base64".into(),
                arg_index: 0,
                expected_types: vec![EvalErrorValueType::String, EvalErrorValueType::Array],
                got_type: EvalErrorValueType::from_value(&arg),
            })),
        }
    }

    pub(super) fn do_std_base64_array(
        &mut self,
        input: GcView<ArrayData<'p>>,
        mut bytes: Vec<u8>,
    ) -> EvalResult<()> {
        let item_value = self.value_stack.pop().unwrap();
        let ValueData::Number(item_value) = item_value else {
            return Err(self.report_error(EvalErrorKind::Other {
                span: None,
                message: format!(
                    "array element must be a number, got {}",
                    EvalErrorValueType::from_value(&item_value).to_str(),
                ),
            }));
        };
        let item_value_int = u8::try_from(item_value as i32).map_err(|_| {
            self.report_error(EvalErrorKind::Other {
                span: None,
                message: format!(
                    "only numbers between 0 and 255 can be base64 encoded, got {item_value}",
                ),
            })
        })?;

        bytes.push(item_value_int);
        if bytes.len() == input.len() {
            let encoded = encode_base64(bytes.iter().map(|&b| Ok(b)))?;
            self.value_stack.push(ValueData::String(encoded.into()));
        } else {
            let next_item = input[bytes.len()].view();
            self.state_stack
                .push(State::StdBase64Array { input, bytes });
            self.state_stack.push(State::DoThunk(next_item));
        }

        Ok(())
    }

    pub(super) fn do_std_base64_decode_bytes(&mut self) -> EvalResult<()> {
        let arg = self.value_stack.pop().unwrap();
        let arg = self.expect_std_func_arg_string(arg, "base64DecodeBytes", 0)?;

        let encoded_chars: Vec<_> = arg.chars().collect();
        let decoded = decode_base64(&encoded_chars).map_err(|e| {
            self.report_error(EvalErrorKind::Other {
                span: None,
                message: e,
            })
        })?;

        self.value_stack
            .push(ValueData::Array(self.program.make_value_array(
                decoded.iter().map(|&b| ValueData::Number(b.into())),
            )));

        Ok(())
    }

    pub(super) fn do_std_base64_decode(&mut self) -> EvalResult<()> {
        let arg = self.value_stack.pop().unwrap();
        let arg = self.expect_std_func_arg_string(arg, "base64Decode", 0)?;

        let encoded_chars = arg.chars().collect::<Vec<_>>();
        let decoded = decode_base64(&encoded_chars).map_err(|e| {
            self.report_error(EvalErrorKind::Other {
                span: None,
                message: e,
            })
        })?;
        let decoded: String = decoded.iter().map(|&c| char::from(c)).collect();

        self.value_stack.push(ValueData::String(decoded.into()));

        Ok(())
    }

    pub(super) fn do_std_md5(&mut self) -> EvalResult<()> {
        let arg = self.value_stack.pop().unwrap();
        let arg = self.expect_std_func_arg_string(arg, "md5", 0)?;

        use md5::Digest as _;
        let hash = md5::Md5::digest(arg.as_bytes());

        let hash_string = hash_to_hex_string(&hash);
        self.value_stack.push(ValueData::String(hash_string.into()));

        Ok(())
    }

    pub(super) fn do_std_sha1(&mut self) -> EvalResult<()> {
        let arg = self.value_stack.pop().unwrap();
        let arg = self.expect_std_func_arg_string(arg, "sha1", 0)?;

        use sha1::Digest as _;
        let hash = sha1::Sha1::digest(arg.as_bytes());

        let hash_string = hash_to_hex_string(&hash);
        self.value_stack.push(ValueData::String(hash_string.into()));

        Ok(())
    }

    pub(super) fn do_std_sha256(&mut self) -> EvalResult<()> {
        let arg = self.value_stack.pop().unwrap();
        let arg = self.expect_std_func_arg_string(arg, "sha256", 0)?;

        use sha2::Digest as _;
        let hash = sha2::Sha256::digest(arg.as_bytes());

        let hash_string = hash_to_hex_string(&hash);
        self.value_stack.push(ValueData::String(hash_string.into()));

        Ok(())
    }

    pub(super) fn do_std_sha512(&mut self) -> EvalResult<()> {
        let arg = self.value_stack.pop().unwrap();
        let arg = self.expect_std_func_arg_string(arg, "sha512", 0)?;

        use sha2::Digest as _;
        let hash = sha2::Sha512::digest(arg.as_bytes());

        let hash_string = hash_to_hex_string(&hash);
        self.value_stack.push(ValueData::String(hash_string.into()));

        Ok(())
    }

    pub(super) fn do_std_sha3(&mut self) -> EvalResult<()> {
        let arg = self.value_stack.pop().unwrap();
        let arg = self.expect_std_func_arg_string(arg, "sha3", 0)?;

        use sha3::Digest as _;
        let hash = sha3::Sha3_512::digest(arg.as_bytes());

        let hash_string = hash_to_hex_string(&hash);
        self.value_stack.push(ValueData::String(hash_string.into()));

        Ok(())
    }

    pub(super) fn do_std_native(&mut self) -> EvalResult<()> {
        let arg = self.value_stack.pop().unwrap();
        let name = self.expect_std_func_arg_string(arg, "native", 0)?;
        let value = self
            .program
            .str_interner
            .get_interned(&name)
            .and_then(|name| self.program.native_funcs.get(&name))
            .map_or(ValueData::Null, |func| ValueData::Function(Gc::from(func)));

        self.value_stack.push(value);

        Ok(())
    }

    pub(super) fn do_std_trace(&mut self) -> EvalResult<()> {
        let arg = self.value_stack.pop().unwrap();
        let msg = self.expect_std_func_arg_string(arg, "trace", 0)?;

        let stack = self.get_stack_trace();
        if let Some(callbacks) = self.callbacks.as_mut() {
            callbacks.trace(self.program, &msg, &stack);
        }

        Ok(())
    }

    pub(super) fn do_std_merge_patch_value(&mut self) {
        let patch = self.value_stack.pop().unwrap();
        let target = self.value_stack.pop().unwrap();

        if let ValueData::Object(patch) = patch {
            let patch = patch.view();
            let target = if let ValueData::Object(target) = target {
                Some(target.view())
            } else {
                None
            };

            let patch_fields_order = patch.get_visible_fields_order();
            let patch_fields: FHashSet<_> = patch_fields_order.clone().collect();
            let mut target_fields = FHashSet::default();

            let mut new_object = ObjectData::new_empty();
            if let Some(ref target) = target {
                target_fields = target.get_visible_fields_order().collect();
                for &field_name in target_fields.iter() {
                    if !patch_fields.contains(&field_name) {
                        let field_thunk = self
                            .program
                            .find_object_field_thunk(target, 0, field_name)
                            .unwrap();
                        new_object.self_layer.fields.insert(
                            field_name,
                            ObjectField {
                                base_env: None,
                                visibility: ast::Visibility::Default,
                                expr: None,
                                thunk: OnceCell::from(Gc::from(&field_thunk)),
                            },
                        );
                    }
                }
            }

            self.state_stack.push(State::ObjectToValue);
            self.object_stack.push(new_object);

            for field_name in patch_fields_order.rev() {
                let patch_field_thunk = self
                    .program
                    .find_object_field_thunk(&patch, 0, field_name)
                    .unwrap();

                self.state_stack
                    .push(State::StdMergePatchField { name: field_name });
                self.state_stack.push(State::StdMergePatchValue);
                self.state_stack.push(State::DoThunk(patch_field_thunk));
                if target_fields.contains(&field_name) {
                    let target_field_thunk = self
                        .program
                        .find_object_field_thunk(target.as_ref().unwrap(), 0, field_name)
                        .unwrap();

                    self.state_stack.push(State::DoThunk(target_field_thunk));
                } else {
                    self.value_stack.push(ValueData::Null);
                }
            }

            self.check_object_asserts(&patch);
            if let Some(ref target) = target {
                self.check_object_asserts(target);
            }
        } else {
            self.value_stack.push(patch);
        }
    }

    pub(super) fn do_std_merge_patch_field(&mut self, name: InternedStr<'p>) {
        let value = self.value_stack.pop().unwrap();
        let object = self.object_stack.last_mut().unwrap();
        if !matches!(value, ValueData::Null) {
            object.self_layer.fields.insert(
                name,
                ObjectField {
                    base_env: None,
                    visibility: ast::Visibility::Default,
                    expr: None,
                    thunk: OnceCell::from(self.program.gc_alloc(ThunkData::new_done(value))),
                },
            );
        }
    }

    pub(super) fn do_std_mod(&mut self) -> EvalResult<()> {
        let rhs = self.value_stack.pop().unwrap();
        let lhs = self.value_stack.pop().unwrap();

        match lhs {
            ValueData::Number(lhs) => {
                let rhs = self.expect_std_func_arg_number(rhs, "mod", 1)?;
                if rhs == 0.0 {
                    return Err(self.report_error(EvalErrorKind::DivByZero { span: None }));
                }
                let r = lhs % rhs;
                self.check_number_value(r, None)?;
                self.value_stack.push(ValueData::Number(r));
                Ok(())
            }
            lhs @ ValueData::String(_) => {
                self.value_stack.push(lhs);
                self.value_stack.push(rhs);
                self.state_stack.push(State::StdFormat);
                Ok(())
            }
            _ => Err(self.report_error(EvalErrorKind::InvalidStdFuncArgType {
                func_name: "mod".into(),
                arg_index: 0,
                expected_types: vec![EvalErrorValueType::Number, EvalErrorValueType::String],
                got_type: EvalErrorValueType::from_value(&lhs),
            })),
        }
    }
}

fn hash_to_hex_string(hash: &[u8]) -> String {
    let mut hash_string = String::with_capacity(hash.len() * 2);
    for &byte in hash {
        write!(hash_string, "{byte:02x}").unwrap();
    }
    hash_string
}

fn encode_base64<I>(input: I) -> EvalResult<String>
where
    I: IntoIterator<Item = EvalResult<u8>>,
{
    let encmap = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut input = input.into_iter();
    let mut encoded = String::new();
    loop {
        let Some(b0) = input.next() else {
            break;
        };
        let b0 = b0?;

        encoded.push(char::from(encmap[usize::from(b0 >> 2)]));

        let Some(b1) = input.next() else {
            encoded.push(char::from(encmap[usize::from((b0 & 0b11) << 4)]));
            encoded.push('=');
            encoded.push('=');
            break;
        };
        let b1 = b1?;

        encoded.push(char::from(
            encmap[usize::from(((b0 & 0b11) << 4) | (b1 >> 4))],
        ));

        let Some(b2) = input.next() else {
            encoded.push(char::from(encmap[usize::from((b1 & 0b1111) << 2)]));
            encoded.push('=');
            break;
        };
        let b2 = b2?;

        encoded.push(char::from(
            encmap[usize::from(((b1 & 0b1111) << 2) | (b2 >> 6))],
        ));
        encoded.push(char::from(encmap[usize::from(b2 & 0b111111)]));
    }

    Ok(encoded)
}

fn decode_base64(encoded: &[char]) -> Result<Vec<u8>, String> {
    let mut chunks = encoded.chunks_exact(4);
    if !chunks.remainder().is_empty() {
        return Err("length of base64 string is not a multiple of 4".into());
    }

    let mut decoded = Vec::new();
    if let Some(last_chunk) = chunks.next_back() {
        fn chr_to_index(chr: char) -> Result<u8, String> {
            match chr {
                'A'..='Z' => Ok(chr as u8 - b'A'),
                'a'..='z' => Ok(chr as u8 - b'a' + 26),
                '0'..='9' => Ok(chr as u8 - b'0' + 52),
                '+' => Ok(62),
                '/' => Ok(63),
                _ => Err(format!("invalid base64 character: {chr:?}")),
            }
        }

        for chunk in chunks {
            let i0 = chr_to_index(chunk[0])?;
            let i1 = chr_to_index(chunk[1])?;
            let i2 = chr_to_index(chunk[2])?;
            let i3 = chr_to_index(chunk[3])?;
            decoded.push((i0 << 2) | (i1 >> 4));
            decoded.push((i1 << 4) | (i2 >> 2));
            decoded.push((i2 << 6) | i3);
        }

        let i0 = chr_to_index(last_chunk[0])?;
        let i1 = chr_to_index(last_chunk[1])?;
        decoded.push((i0 << 2) | (i1 >> 4));

        match (last_chunk[2], last_chunk[3]) {
            ('=', '=') => {}
            (e2, '=') => {
                let i2 = chr_to_index(e2)?;
                decoded.push((i1 << 4) | (i2 >> 2));
            }
            (e2, e3) => {
                let i2 = chr_to_index(e2)?;
                let i3 = chr_to_index(e3)?;
                decoded.push((i1 << 4) | (i2 >> 2));
                decoded.push((i2 << 6) | i3);
            }
        }
    }

    Ok(decoded)
}
