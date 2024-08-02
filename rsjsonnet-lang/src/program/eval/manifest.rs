use std::fmt::Write as _;
use std::rc::Rc;

use super::super::ValueData;
use super::{EvalError, EvalErrorKind, Evaluator, State, TraceItem};

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

impl Evaluator<'_> {
    pub(super) fn do_manifest_json(
        &mut self,
        format: Rc<ManifestJsonFormat>,
        depth: usize,
    ) -> Result<(), EvalError> {
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

                        self.push_trace_item(TraceItem::ArrayItem {
                            span: None,
                            index: i,
                        });
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
                    .filter(|name| object.field_is_visible(name))
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

                        self.push_trace_item(TraceItem::ObjectField {
                            span: None,
                            name: field_name.clone(),
                        });
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
