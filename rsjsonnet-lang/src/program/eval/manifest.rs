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

    pub(super) fn do_manifest_yaml_doc(
        &mut self,
        indent_array_in_object: bool,
        quote_keys: bool,
        depth: usize,
        parent_is_array: bool,
        parent_is_object: bool,
    ) -> Result<(), EvalError> {
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

                        self.push_trace_item(TraceItem::ArrayItem {
                            span: None,
                            index: i,
                        });
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
                    .filter(|name| object.field_is_visible(name))
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

                        self.push_trace_item(TraceItem::ObjectField {
                            span: None,
                            name: field_name.clone(),
                        });
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
