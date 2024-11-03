use std::fmt::Write as _;
use std::rc::Rc;

use super::super::{ArrayData, ObjectData, ValueData};
use super::{EvalErrorKind, EvalResult, Evaluator, State, TraceItem};
use crate::gc::GcView;
use crate::interner::InternedStr;

pub(super) struct ManifestJsonFormat {
    indent: Box<str>,
    newline: Box<str>,
    key_val_sep: Box<str>,
    item_sep: Box<str>,
    empty_array: Option<Box<str>>,
    empty_object: Option<Box<str>>,
}

impl ManifestJsonFormat {
    pub(super) fn default_to_string() -> Rc<Self> {
        Rc::new(Self {
            indent: "".into(),
            newline: "".into(),
            key_val_sep: ": ".into(),
            item_sep: ", ".into(),
            empty_array: Some("[ ]".into()),
            empty_object: Some("{ }".into()),
        })
    }

    pub(super) fn default_manifest() -> Rc<Self> {
        Rc::new(Self {
            indent: "   ".into(),
            newline: "\n".into(),
            key_val_sep: ": ".into(),
            item_sep: ",".into(),
            empty_array: Some("[ ]".into()),
            empty_object: Some("{ }".into()),
        })
    }

    pub(super) fn for_std_manifest_ex(indent: &str, newline: &str, key_val_sep: &str) -> Rc<Self> {
        Rc::new(Self {
            indent: indent.into(),
            newline: newline.into(),
            key_val_sep: key_val_sep.into(),
            item_sep: ",".into(),
            empty_array: None,
            empty_object: None,
        })
    }
}

impl<'p> Evaluator<'_, 'p> {
    pub(super) fn do_manifest_ini_section(&mut self) -> EvalResult<()> {
        let ValueData::Object(object) = self.value_stack.pop().unwrap() else {
            return Err(self.report_error(EvalErrorKind::Other {
                span: None,
                message: "ini section must be an object".into(),
            }));
        };
        let object = object.view();

        let visible_fields: Vec<_> = object
            .get_fields_order()
            .iter()
            .filter_map(|&(name, visible)| visible.then_some(name))
            .collect();
        for &field_name in visible_fields.iter().rev() {
            let field_thunk = self
                .program
                .find_object_field_thunk(&object, 0, field_name)
                .unwrap();

            self.state_stack
                .push(State::ManifestIniSectionItem { name: field_name });
            self.state_stack.push(State::DoThunk(field_thunk));
        }
        self.check_object_asserts(&object);

        Ok(())
    }

    pub(super) fn do_manifest_ini_section_item(&mut self, name: InternedStr<'p>) -> EvalResult<()> {
        let value = self.value_stack.pop().unwrap();
        if let ValueData::Array(array) = value {
            let array = array.view();
            for item in array.iter().rev() {
                self.state_stack.push(State::AppendToString('\n'.into()));
                self.state_stack.push(State::CoerceAppendToString);
                self.state_stack.push(State::DoThunk(item.view()));
                self.state_stack
                    .push(State::AppendToString(format!("{} = ", name.value())));
            }
        } else {
            self.value_stack.push(value);
            self.state_stack.push(State::AppendToString('\n'.into()));
            self.state_stack.push(State::CoerceAppendToString);
            self.state_stack
                .push(State::AppendToString(format!("{} = ", name.value())));
        }

        Ok(())
    }

    pub(super) fn do_manifest_python(&mut self) -> EvalResult<()> {
        let value = self.value_stack.pop().unwrap();
        let result = self.string_stack.last_mut().unwrap();
        match value {
            ValueData::Null => {
                result.push_str("None");
            }
            ValueData::Bool(b) => {
                result.push_str(if b { "True" } else { "False" });
            }
            ValueData::Number(n) => {
                write!(result, "{n}").unwrap();
            }
            ValueData::String(s) => {
                escape_string_python(&s, result);
            }
            ValueData::Array(array) => {
                let array = array.view();
                if array.is_empty() {
                    result.push_str("[]");
                } else {
                    result.push('[');

                    self.state_stack.push(State::AppendToString(']'.into()));

                    for (i, item) in array.iter().enumerate().rev() {
                        if i != array.len() - 1 {
                            self.state_stack.push(State::AppendToString(", ".into()));
                        }

                        self.push_trace_item(TraceItem::ManifestArrayItem { index: i });
                        self.state_stack.push(State::ManifestPython);
                        self.state_stack.push(State::DoThunk(item.view()));
                        self.delay_trace_item();
                    }
                }
            }
            ValueData::Object(object) => {
                let object = object.view();
                let visible_fields: Vec<_> = object
                    .get_fields_order()
                    .iter()
                    .filter_map(|&(name, visible)| visible.then_some(name))
                    .collect();
                if visible_fields.is_empty() {
                    result.push_str("{}");
                } else {
                    result.push('{');

                    self.state_stack.push(State::AppendToString('}'.into()));

                    for (i, &field_name) in visible_fields.iter().rev().enumerate() {
                        if i != 0 {
                            self.state_stack.push(State::AppendToString(", ".into()));
                        }

                        self.push_trace_item(TraceItem::ManifestObjectField { name: field_name });
                        self.state_stack.push(State::ManifestPython);
                        let field_thunk = self
                            .program
                            .find_object_field_thunk(&object, 0, field_name)
                            .unwrap();
                        self.state_stack.push(State::DoThunk(field_thunk));
                        self.delay_trace_item();
                        self.state_stack.push(State::AppendToString(": ".into()));
                        let mut name_manifested = String::new();
                        escape_string_python(field_name.value(), &mut name_manifested);
                        self.state_stack
                            .push(State::AppendToString(name_manifested));
                    }
                }
                self.check_object_asserts(&object);
            }
            ValueData::Function(_) => {
                return Err(self.report_error(EvalErrorKind::ManifestFunction));
            }
        }

        Ok(())
    }

    pub(super) fn do_manifest_json(
        &mut self,
        format: Rc<ManifestJsonFormat>,
        depth: usize,
    ) -> EvalResult<()> {
        let value = self.value_stack.pop().unwrap();
        let result = self.string_stack.last_mut().unwrap();
        match value {
            ValueData::Null => {
                result.push_str("null");
            }
            ValueData::Bool(b) => {
                result.push_str(if b { "true" } else { "false" });
            }
            ValueData::Number(n) => {
                write!(result, "{n}").unwrap();
            }
            ValueData::String(s) => {
                escape_string_json(&s, result);
            }
            ValueData::Array(array) => {
                let array = array.view();
                if array.is_empty() {
                    if let Some(ref empty_array) = format.empty_array {
                        result.push_str(empty_array);
                    } else {
                        result.push('[');
                        result.push_str(&format.newline);
                        result.push_str(&format.newline);
                        for _ in 0..depth {
                            result.push_str(&format.indent);
                        }
                        result.push(']');
                    }
                } else {
                    result.push('[');
                    result.push_str(&format.newline);

                    let mut tmp = String::new();
                    tmp.push_str(&format.newline);
                    for _ in 0..depth {
                        tmp.push_str(&format.indent);
                    }
                    tmp.push(']');
                    self.state_stack.push(State::AppendToString(tmp));

                    for (i, item) in array.iter().enumerate().rev() {
                        if i != array.len() - 1 {
                            let mut tmp = String::new();
                            tmp.push_str(&format.item_sep);
                            tmp.push_str(&format.newline);
                            self.state_stack.push(State::AppendToString(tmp));
                        }

                        self.push_trace_item(TraceItem::ManifestArrayItem { index: i });
                        self.state_stack.push(State::ManifestJson {
                            format: format.clone(),
                            depth: depth + 1,
                        });
                        self.state_stack.push(State::DoThunk(item.view()));
                        self.delay_trace_item();
                        if !format.indent.is_empty() {
                            self.state_stack
                                .push(State::AppendToString(format.indent.repeat(depth + 1)));
                        }
                    }
                }
            }
            ValueData::Object(object) => {
                let object = object.view();
                let visible_fields: Vec<_> = object
                    .get_fields_order()
                    .iter()
                    .filter_map(|&(name, visible)| visible.then_some(name))
                    .collect();
                if visible_fields.is_empty() {
                    if let Some(ref empty_object) = format.empty_object {
                        result.push_str(empty_object);
                    } else {
                        result.push('{');
                        result.push_str(&format.newline);
                        result.push_str(&format.newline);
                        for _ in 0..depth {
                            result.push_str(&format.indent);
                        }
                        result.push('}');
                    }
                } else {
                    result.push('{');
                    result.push_str(&format.newline);

                    let mut tmp = String::new();
                    tmp.push_str(&format.newline);
                    for _ in 0..depth {
                        tmp.push_str(&format.indent);
                    }
                    tmp.push('}');
                    self.state_stack.push(State::AppendToString(tmp));

                    for (i, &field_name) in visible_fields.iter().rev().enumerate() {
                        if i != 0 {
                            let mut tmp = String::new();
                            tmp.push_str(&format.item_sep);
                            tmp.push_str(&format.newline);
                            self.state_stack.push(State::AppendToString(tmp));
                        }

                        self.push_trace_item(TraceItem::ManifestObjectField { name: field_name });
                        self.state_stack.push(State::ManifestJson {
                            format: format.clone(),
                            depth: depth + 1,
                        });
                        let field_thunk = self
                            .program
                            .find_object_field_thunk(&object, 0, field_name)
                            .unwrap();
                        self.state_stack.push(State::DoThunk(field_thunk));
                        self.delay_trace_item();
                        self.state_stack
                            .push(State::AppendToString((*format.key_val_sep).into()));
                        let mut name_manifested = String::new();
                        escape_string_json(field_name.value(), &mut name_manifested);
                        self.state_stack
                            .push(State::AppendToString(name_manifested));
                        if !format.indent.is_empty() {
                            self.state_stack
                                .push(State::AppendToString(format.indent.repeat(depth + 1)));
                        }
                    }
                }
                self.check_object_asserts(&object);
            }
            ValueData::Function(_) => {
                return Err(self.report_error(EvalErrorKind::ManifestFunction));
            }
        }

        Ok(())
    }

    pub(super) fn do_manifest_yaml_doc(
        &mut self,
        indent_array_in_object: bool,
        quote_keys: bool,
        depth: usize,
        parent_is_array: bool,
        parent_is_object: bool,
    ) -> EvalResult<()> {
        let value = self.value_stack.pop().unwrap();
        let result = self.string_stack.last_mut().unwrap();
        let indent = "  ";
        match value {
            ValueData::Null => {
                if parent_is_array || parent_is_object {
                    result.push(' ');
                }
                result.push_str("null");
            }
            ValueData::Bool(b) => {
                if parent_is_array || parent_is_object {
                    result.push(' ');
                }
                result.push_str(if b { "true" } else { "false" });
            }
            ValueData::Number(n) => {
                if parent_is_array || parent_is_object {
                    result.push(' ');
                }
                write!(result, "{n}").unwrap();
            }
            ValueData::String(s) => {
                if parent_is_array || parent_is_object {
                    result.push(' ');
                }
                if let Some(s) = s.strip_suffix('\n') {
                    let sub_depth = if parent_is_array || parent_is_object {
                        depth
                    } else {
                        depth + 1
                    };
                    result.push('|');
                    for line in s.split('\n') {
                        result.push('\n');
                        for _ in 0..sub_depth {
                            result.push_str(indent);
                        }
                        result.push_str(line);
                    }
                } else {
                    escape_string_json(&s, result);
                }
            }
            ValueData::Array(array) => {
                let array = array.view();
                if array.is_empty() {
                    if parent_is_array || parent_is_object {
                        result.push(' ');
                    }
                    result.push_str("[]");
                } else {
                    // Arrays always start on their own line
                    if parent_is_array || parent_is_object {
                        result.push('\n');
                    }
                    // Allow choosing
                    // ```
                    // x:
                    //   - 1
                    //   - 2
                    // ```
                    // or
                    // ```
                    // x:
                    // - 1
                    // - 2
                    // ```
                    let depth = if parent_is_object && !indent_array_in_object {
                        depth - 1
                    } else {
                        depth
                    };
                    for (i, item) in array.iter().enumerate().rev() {
                        if i != array.len() - 1 {
                            self.state_stack.push(State::AppendToString("\n".into()));
                        }

                        self.push_trace_item(TraceItem::ManifestArrayItem { index: i });
                        self.state_stack.push(State::ManifestYamlDoc {
                            indent_array_in_object,
                            quote_keys,
                            depth: depth + 1,
                            parent_is_array: true,
                            parent_is_object: false,
                        });
                        self.state_stack.push(State::DoThunk(item.view()));
                        self.delay_trace_item();
                        self.state_stack.push(State::AppendToString("-".into()));
                        self.state_stack
                            .push(State::AppendToString(indent.repeat(depth)));
                    }
                }
            }
            ValueData::Object(object) => {
                let object = object.view();
                let visible_fields: Vec<_> = object
                    .get_fields_order()
                    .iter()
                    .filter_map(|&(name, visible)| visible.then_some(name))
                    .collect();
                if visible_fields.is_empty() {
                    if parent_is_array || parent_is_object {
                        result.push(' ');
                    }
                    result.push_str("{}");
                } else {
                    if parent_is_array {
                        // Keep the first field in the same line as the `- ` of the parent array
                        result.push(' ');
                    } else if parent_is_object {
                        // An object nested in another object starts on its own line
                        result.push('\n');
                    }
                    for (i, &field_name) in visible_fields.iter().rev().enumerate() {
                        if i != 0 {
                            self.state_stack.push(State::AppendToString("\n".into()));
                        }

                        self.push_trace_item(TraceItem::ManifestObjectField { name: field_name });
                        self.state_stack.push(State::ManifestYamlDoc {
                            indent_array_in_object,
                            quote_keys,
                            depth: depth + 1,
                            parent_is_array: false,
                            parent_is_object: true,
                        });
                        let field_thunk = self
                            .program
                            .find_object_field_thunk(&object, 0, field_name)
                            .unwrap();
                        self.state_stack.push(State::DoThunk(field_thunk));
                        self.delay_trace_item();
                        self.state_stack.push(State::AppendToString(":".into()));
                        if !quote_keys && is_safe_yaml_plain(field_name.value()) {
                            self.state_stack
                                .push(State::AppendToString(field_name.value().into()));
                        } else {
                            let mut name_manifested = String::new();
                            escape_string_json(field_name.value(), &mut name_manifested);
                            self.state_stack
                                .push(State::AppendToString(name_manifested));
                        }
                        // Skip indent for the first field to keep the first field in the same
                        // line as the `- ` of the parent array
                        if !parent_is_array || i != visible_fields.len() - 1 {
                            self.state_stack
                                .push(State::AppendToString(indent.repeat(depth)));
                        }
                    }
                }
                self.check_object_asserts(&object);
            }
            ValueData::Function(_) => {
                return Err(self.report_error(EvalErrorKind::ManifestFunction));
            }
        }

        Ok(())
    }

    pub(super) fn do_manifest_toml_peek_sub_table(&mut self) {
        if let ValueData::Array(array) = self.value_stack.pop().unwrap() {
            let array = array.view();
            if let Some(item0) = array.first() {
                let item0 = item0.view();
                self.state_stack
                    .push(State::ManifestTomlPeekSubTableArrayItem { array, index: 0 });
                self.state_stack.push(State::DoThunk(item0));
            }
        }
    }

    pub(super) fn do_manifest_toml_peek_sub_table_array_item(
        &mut self,
        array: GcView<ArrayData<'p>>,
        index: usize,
    ) {
        let value = self.value_stack.pop().unwrap();
        if matches!(value, ValueData::Object(_)) {
            let next_index = index + 1;
            if let Some(item) = array.get(next_index) {
                let item = item.view();
                self.state_stack
                    .push(State::ManifestTomlPeekSubTableArrayItem {
                        array,
                        index: next_index,
                    });
                self.state_stack.push(State::DoThunk(item));
            }
        }
    }

    pub(super) fn prepare_manifest_toml_table(
        &mut self,
        object: GcView<ObjectData<'p>>,
        has_header: bool,
        path: Rc<[InternedStr<'p>]>,
        indent: Rc<str>,
    ) {
        self.state_stack.push(State::ManifestTomlTable {
            object: object.clone(),
            has_header,
            path,
            indent,
        });

        let visible_fields: Vec<_> = object
            .get_fields_order()
            .iter()
            .filter_map(|&(name, visible)| visible.then_some(name))
            .collect();
        for &field_name in visible_fields.iter().rev() {
            let field_thunk = self
                .program
                .find_object_field_thunk(&object, 0, field_name)
                .unwrap();

            self.state_stack.push(State::ManifestTomlPeekSubTable);
            self.state_stack.push(State::DoThunk(field_thunk));
        }
        self.check_object_asserts(&object);
    }

    pub(super) fn do_manifest_toml_table(
        &mut self,
        object: GcView<ObjectData<'p>>,
        has_header: bool,
        path: Rc<[InternedStr<'p>]>,
        indent: Rc<str>,
    ) {
        let visible_fields: Vec<_> = object
            .get_fields_order()
            .iter()
            .filter(|&&(_, visible)| visible)
            .map(|&(name, _)| {
                // Check if the field is an object or an array of objects
                let (_, field) = object.find_field(0, name).unwrap();
                let field_value = field.thunk.get().unwrap().view().get_value().unwrap();
                let is_sub_table = match field_value {
                    ValueData::Array(array) => {
                        let array = array.view();
                        !array.is_empty()
                            && array.iter().all(|item| {
                                matches!(item.view().get_value().unwrap(), ValueData::Object(_))
                            })
                    }
                    ValueData::Object(_) => true,
                    _ => false,
                };
                (name, is_sub_table)
            })
            .collect();

        let mut has_sub_tables = false;
        for (i, &(field_name, _)) in visible_fields
            .iter()
            .rev()
            .filter(|&&(_, is_sub_table)| is_sub_table)
            .enumerate()
        {
            if i != 0 {
                self.state_stack.push(State::AppendToString('\n'.into()));
            }

            has_sub_tables = true;

            let (_, field) = object.find_field(0, field_name).unwrap();
            let field_value = field.thunk.get().unwrap().view().get_value().unwrap();

            let sub_path: Rc<[_]> = path
                .iter()
                .copied()
                .chain(std::iter::once(field_name))
                .collect();

            self.push_trace_item(TraceItem::ManifestObjectField { name: field_name });

            match field_value {
                ValueData::Array(array) => {
                    let array = array.view();

                    let mut header = String::new();
                    header.push('\n');
                    for _ in 0..path.len() {
                        header.push_str(&indent);
                    }
                    header.push_str("[[");
                    for part in path.iter() {
                        header.push_str(&escape_key_toml(part.value()));
                        header.push('.');
                    }
                    header.push_str(&escape_key_toml(field_name.value()));
                    header.push_str("]]");

                    for (i, item) in array.iter().enumerate().rev() {
                        if i != array.len() - 1 {
                            self.state_stack.push(State::AppendToString('\n'.into()));
                        }

                        let ValueData::Object(sub_object) = item.view().get_value().unwrap() else {
                            unreachable!();
                        };
                        let sub_object = sub_object.view();

                        self.push_trace_item(TraceItem::ManifestArrayItem { index: i });
                        self.prepare_manifest_toml_table(
                            sub_object,
                            true,
                            sub_path.clone(),
                            indent.clone(),
                        );
                        self.delay_trace_item();
                        self.state_stack.push(State::AppendToString(header.clone()));
                    }
                }
                ValueData::Object(sub_object) => {
                    let sub_object = sub_object.view();
                    self.prepare_manifest_toml_table(sub_object, true, sub_path, indent.clone());

                    let mut header = String::new();
                    header.push('\n');
                    for _ in 0..path.len() {
                        header.push_str(&indent);
                    }
                    header.push('[');
                    for part in path.iter() {
                        header.push_str(&escape_key_toml(part.value()));
                        header.push('.');
                    }
                    header.push_str(&escape_key_toml(field_name.value()));
                    header.push(']');
                    self.state_stack.push(State::AppendToString(header));
                }
                _ => unreachable!(),
            }

            self.delay_trace_item();
        }

        if has_sub_tables {
            self.state_stack.push(State::AppendToString('\n'.into()));
        }

        for (i, &(field_name, _)) in visible_fields
            .iter()
            .rev()
            .filter(|&&(_, is_sub_table)| !is_sub_table)
            .enumerate()
        {
            if i != 0 {
                self.state_stack.push(State::AppendToString('\n'.into()));
            }

            let field_thunk = self
                .program
                .find_object_field_thunk(&object, 0, field_name)
                .unwrap();

            self.push_trace_item(TraceItem::ManifestObjectField { name: field_name });
            self.state_stack.push(State::ManifestTomlValue {
                indent: indent.clone(),
                depth: path.len(),
                single_line: false,
            });
            self.state_stack.push(State::DoThunk(field_thunk));
            self.delay_trace_item();

            let mut tmp = String::new();
            for _ in 0..path.len() {
                tmp.push_str(&indent);
            }
            tmp.push_str(&escape_key_toml(field_name.value()));
            tmp.push_str(" = ");
            self.state_stack.push(State::AppendToString(tmp));
        }

        if has_header && !visible_fields.is_empty() {
            self.state_stack.push(State::AppendToString('\n'.into()));
        }
    }

    pub(super) fn do_manifest_toml_value(
        &mut self,
        indent: Rc<str>,
        depth: usize,
        single_line: bool,
    ) -> EvalResult<()> {
        let value = self.value_stack.pop().unwrap();
        let result = self.string_stack.last_mut().unwrap();
        match value {
            ValueData::Null => {
                return Err(self.report_error(EvalErrorKind::Other {
                    span: None,
                    message: "cannot manifest null in TOML".into(),
                }));
            }
            ValueData::Bool(b) => {
                result.push_str(if b { "true" } else { "false" });
            }
            ValueData::Number(n) => {
                write!(result, "{n}").unwrap();
            }
            ValueData::String(s) => {
                escape_string_toml(&s, result);
            }
            ValueData::Array(array) => {
                let array = array.view();
                if array.is_empty() {
                    result.push_str("[]");
                } else {
                    if single_line {
                        result.push_str("[ ");
                        self.state_stack.push(State::AppendToString(" ]".into()));
                    } else {
                        result.push_str("[\n");

                        let mut tmp = String::new();
                        tmp.push('\n');
                        for _ in 0..depth {
                            tmp.push_str(&indent);
                        }
                        tmp.push(']');
                        self.state_stack.push(State::AppendToString(tmp));
                    }

                    for (i, item) in array.iter().enumerate().rev() {
                        if i != array.len() - 1 {
                            let mut tmp = String::new();
                            if single_line {
                                tmp.push_str(", ");
                            } else {
                                tmp.push_str(",\n");
                            }
                            self.state_stack.push(State::AppendToString(tmp));
                        }

                        self.push_trace_item(TraceItem::ManifestArrayItem { index: i });
                        self.state_stack.push(State::ManifestTomlValue {
                            indent: indent.clone(),
                            depth: depth + 1,
                            single_line: true,
                        });
                        self.state_stack.push(State::DoThunk(item.view()));
                        self.delay_trace_item();
                        if !single_line && !indent.is_empty() {
                            self.state_stack
                                .push(State::AppendToString(indent.repeat(depth + 1)));
                        }
                    }
                }
            }
            ValueData::Object(object) => {
                let object = object.view();
                let visible_fields: Vec<_> = object
                    .get_fields_order()
                    .iter()
                    .filter_map(|&(name, visible)| visible.then_some(name))
                    .collect();
                if visible_fields.is_empty() {
                    result.push_str("{  }");
                } else {
                    result.push_str("{ ");
                    self.state_stack.push(State::AppendToString(" }".into()));

                    for (i, &field_name) in visible_fields.iter().rev().enumerate() {
                        if i != 0 {
                            self.state_stack.push(State::AppendToString(", ".into()));
                        }

                        self.push_trace_item(TraceItem::ManifestObjectField { name: field_name });
                        self.state_stack.push(State::ManifestTomlValue {
                            indent: indent.clone(),
                            depth: depth + 1,
                            single_line: true,
                        });
                        let field_thunk = self
                            .program
                            .find_object_field_thunk(&object, 0, field_name)
                            .unwrap();
                        self.state_stack.push(State::DoThunk(field_thunk));
                        self.delay_trace_item();
                        self.state_stack.push(State::AppendToString(" = ".into()));
                        self.state_stack
                            .push(State::AppendToString(escape_key_toml(field_name.value())));
                    }
                }
                self.check_object_asserts(&object);
            }
            ValueData::Function(_) => {
                return Err(self.report_error(EvalErrorKind::ManifestFunction));
            }
        }

        Ok(())
    }
}

fn is_safe_yaml_plain(s: &str) -> bool {
    // Check empty string and special sequences
    if s.is_empty() || s == "-" || s == "---" {
        return false;
    }

    // Check for unsafe characters
    if s.chars()
        .any(|chr| !matches!(chr, '0'..='9' | 'A'..='Z' | 'a'..='z' | '/' | '_' | '-' | '.'))
    {
        return false;
    }

    // Check for reserved alphabetic values
    let special = [
        "null", "true", "y", "yes", "on", "false", "n", "no", "off", ".nan", ".inf", "+.inf",
        "-.inf",
    ];
    if special
        .iter()
        .any(|&special| s.eq_ignore_ascii_case(special))
    {
        return false;
    }

    // Check for dates, where there are two '-' and the rest are digits
    if s.chars().all(|chr| matches!(chr, '0'..='9' | '-'))
        && s.chars().filter(|&chr| chr == '-').count() == 2
    {
        return false;
    }

    // Check for integers, where there is at most one `-` and the rest are digits or '_'
    if s.chars().all(|chr| matches!(chr, '0'..='9' | '_' | '-'))
        && s.chars().filter(|&chr| chr == '-').count() <= 1
    {
        return false;
    }

    // Check for base-2 integers, which start with `0b` or `-0b` and the rest are digits or '_'
    if (s.starts_with("0b") || s.starts_with("-0b"))
        && s.chars()
            .all(|chr| matches!(chr, '0'..='9' | 'b' | 'B' | '_' | '-'))
        && s.chars().filter(|&chr| chr == '-').count() <= 1
    {
        return false;
    }

    // Check for base-16 integers, which start with `0x` or `-0x` and the rest are digits or '_'
    if (s.starts_with("0x") || s.starts_with("-0x"))
        && s.chars()
            .all(|chr| matches!(chr, '0'..='9' | 'a'..='f' | 'A'..='F' | 'x' | 'X' | '_' | '-'))
        && s.chars().filter(|&chr| chr == '-').count() <= 1
    {
        return false;
    }

    // Check for floats.
    if s.chars()
        .all(|chr| matches!(chr, '0'..='9' | 'e' | 'E' | '_' | '-' | '.'))
        && s.chars().filter(|&chr| chr == '.').count() == 1
        && s.chars().filter(|&chr| chr == '-').count() <= 2
        && s.chars().filter(|&chr| chr == 'e' || chr == 'E').count() <= 1
    {
        return false;
    }

    true
}

fn is_safe_toml_plain(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

fn escape_key_toml(s: &str) -> String {
    if is_safe_toml_plain(s) {
        s.into()
    } else {
        let mut escaped = String::new();
        escape_string_toml(s, &mut escaped);
        escaped
    }
}

pub(super) fn escape_string_json(s: &str, result: &mut String) {
    result.push('"');
    for chr in s.chars() {
        match chr {
            '\u{8}' => result.push_str("\\b"),
            '\t' => result.push_str("\\t"),
            '\n' => result.push_str("\\n"),
            '\u{c}' => result.push_str("\\f"),
            '\r' => result.push_str("\\r"),
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\u{0}'..='\u{19}' | '\u{7F}'..='\u{9F}' => {
                write!(result, "\\u{:04x}", chr as u32).unwrap();
            }
            _ => result.push(chr),
        }
    }
    result.push('"');
}

pub(super) fn escape_string_python(s: &str, result: &mut String) {
    escape_string_json(s, result);
}

fn escape_string_toml(s: &str, result: &mut String) {
    escape_string_json(s, result);
}
