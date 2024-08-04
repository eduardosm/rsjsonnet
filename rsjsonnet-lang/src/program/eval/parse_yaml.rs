use std::cell::OnceCell;
use std::collections::HashMap;

use super::{ObjectData, ObjectField, Program, ThunkData, ValueData};
use crate::ast;
use crate::gc::Gc;
use crate::interner::InternedStr;

pub(crate) enum ParseError {
    LibYaml(libyaml_safer::Error),
    Stream,
    EmptyStream,
    Anchor,
    Alias,
    Tag,
    KeyIsObject(libyaml_safer::Mark),
    KeyIsArray(libyaml_safer::Mark),
    NumberOverflow,
    RepeatedFieldName(InternedStr),
}

impl From<libyaml_safer::Error> for ParseError {
    #[inline]
    fn from(e: libyaml_safer::Error) -> Self {
        Self::LibYaml(e)
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::LibYaml(ref e) => write!(f, "{e}"),
            Self::Stream => write!(f, "YAML stream not allowed"),
            Self::EmptyStream => write!(f, "empty stream"),
            Self::Anchor => write!(f, "anchors are not allowed"),
            Self::Alias => write!(f, "aliases are not allowed"),
            Self::Tag => write!(f, "tags are not allowed"),
            Self::KeyIsObject(ref mark) => write!(f, "{mark}: object key is an object"),
            Self::KeyIsArray(ref mark) => write!(f, "{mark}: object key is an array"),
            Self::NumberOverflow => write!(f, "number overflow"),
            Self::RepeatedFieldName(ref name) => {
                write!(f, "repeated field name {:?}", name.value())
            }
        }
    }
}

pub(super) fn parse_yaml(
    program: &mut Program,
    s: &str,
    allow_stream: bool,
) -> Result<ValueData, ParseError> {
    let mut parser = libyaml_safer::Parser::new();
    let mut bytes = s.as_bytes();
    parser.set_input_string(&mut bytes);

    let first_event = parser.parse()?;
    if let libyaml_safer::EventData::StreamStart { .. } = first_event.data {
        drop(first_event);
    } else {
        unreachable!();
    }

    enum StreamKind {
        Empty,
        Single(ValueData),
        Stream(Vec<Gc<ThunkData>>),
    }

    let mut stream_kind = StreamKind::Empty;
    loop {
        let event = parser.parse()?;
        match event.data {
            libyaml_safer::EventData::StreamEnd => match stream_kind {
                StreamKind::Empty => return Err(ParseError::EmptyStream),
                StreamKind::Single(value) => return Ok(value),
                StreamKind::Stream(items) => {
                    if items.is_empty() {
                        return Ok(ValueData::Array(Gc::from(&program.empty_array)));
                    } else {
                        return Ok(ValueData::Array(program.gc_alloc(items.into_boxed_slice())));
                    }
                }
            },
            libyaml_safer::EventData::DocumentStart { implicit, .. } => {
                if !implicit {
                    if !allow_stream {
                        return Err(ParseError::Stream);
                    }
                    match stream_kind {
                        StreamKind::Empty => {
                            stream_kind = StreamKind::Stream(Vec::new());
                        }
                        StreamKind::Single(single_value) => {
                            stream_kind = StreamKind::Stream(vec![
                                program.gc_alloc(ThunkData::new_done(single_value))
                            ]);
                        }
                        StreamKind::Stream(_) => {}
                    }
                }

                let value = parse_yaml_document(program, &mut parser)?;
                match stream_kind {
                    StreamKind::Empty => {
                        stream_kind = StreamKind::Single(value);
                    }
                    StreamKind::Single(_) => unreachable!(),
                    StreamKind::Stream(ref mut items) => {
                        items.push(program.gc_alloc(ThunkData::new_done(value)));
                    }
                }
            }
            _ => unreachable!(),
        }
    }
}

pub(super) fn parse_yaml_document(
    program: &mut Program,
    parser: &mut libyaml_safer::Parser<'_>,
) -> Result<ValueData, ParseError> {
    enum StackItem {
        Array(Vec<Gc<ThunkData>>),
        Object(HashMap<InternedStr, ObjectField>, InternedStr),
    }

    let mut stack = Vec::new();
    let mut event = parser.parse()?;
    loop {
        let mut value = match event.data {
            libyaml_safer::EventData::Alias { .. } => {
                return Err(ParseError::Alias);
            }
            libyaml_safer::EventData::Scalar {
                anchor,
                tag,
                value,
                style,
                ..
            } => {
                if anchor.is_some() {
                    return Err(ParseError::Anchor);
                }
                if tag.is_some() {
                    return Err(ParseError::Tag);
                }

                scalar_to_value(style, &value)?
            }
            libyaml_safer::EventData::SequenceStart { anchor, tag, .. } => {
                if anchor.is_some() {
                    return Err(ParseError::Anchor);
                }
                if tag.is_some() {
                    return Err(ParseError::Tag);
                }
                event = parser.parse()?;
                if matches!(event.data, libyaml_safer::EventData::SequenceEnd) {
                    ValueData::Array(Gc::from(&program.empty_array))
                } else {
                    stack.push(StackItem::Array(Vec::new()));
                    continue;
                }
            }
            libyaml_safer::EventData::MappingStart { anchor, tag, .. } => {
                if anchor.is_some() {
                    return Err(ParseError::Anchor);
                }
                if tag.is_some() {
                    return Err(ParseError::Tag);
                }
                event = parser.parse()?;
                match event.data {
                    libyaml_safer::EventData::Alias { .. } => {
                        return Err(ParseError::Alias);
                    }
                    libyaml_safer::EventData::SequenceStart { .. } => {
                        return Err(ParseError::KeyIsArray(event.start_mark));
                    }
                    libyaml_safer::EventData::MappingStart { .. } => {
                        return Err(ParseError::KeyIsObject(event.start_mark));
                    }
                    libyaml_safer::EventData::MappingEnd => {
                        ValueData::Object(program.gc_alloc(ObjectData::new_simple(HashMap::new())))
                    }
                    libyaml_safer::EventData::Scalar { value, .. } => {
                        let key = program.str_interner.intern(&value);
                        stack.push(StackItem::Object(HashMap::new(), key));
                        event = parser.parse()?;
                        continue;
                    }
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        };

        event = parser.parse()?;
        loop {
            if let Some(stack_item) = stack.pop() {
                match stack_item {
                    StackItem::Array(mut items) => {
                        items.push(program.gc_alloc(ThunkData::new_done(value)));
                        if matches!(event.data, libyaml_safer::EventData::SequenceEnd) {
                            value = ValueData::Array(program.gc_alloc(items.into_boxed_slice()));
                            event = parser.parse()?;
                        } else {
                            stack.push(StackItem::Array(items));
                            break;
                        }
                    }
                    StackItem::Object(mut fields, current_key) => {
                        match fields.entry(current_key) {
                            std::collections::hash_map::Entry::Vacant(entry) => {
                                entry.insert(ObjectField {
                                    base_env: None,
                                    visibility: ast::Visibility::Default,
                                    expr: None,
                                    thunk: OnceCell::from(
                                        program.gc_alloc(ThunkData::new_done(value)),
                                    ),
                                });
                            }
                            std::collections::hash_map::Entry::Occupied(entry) => {
                                return Err(ParseError::RepeatedFieldName(entry.key().clone()));
                            }
                        }

                        match event.data {
                            libyaml_safer::EventData::Alias { .. } => {
                                return Err(ParseError::Alias);
                            }
                            libyaml_safer::EventData::SequenceStart { .. } => {
                                return Err(ParseError::KeyIsArray(event.start_mark));
                            }
                            libyaml_safer::EventData::MappingStart { .. } => {
                                return Err(ParseError::KeyIsObject(event.start_mark));
                            }
                            libyaml_safer::EventData::MappingEnd => {
                                value = ValueData::Object(
                                    program.gc_alloc(ObjectData::new_simple(fields)),
                                );
                                event = parser.parse()?;
                            }
                            libyaml_safer::EventData::Scalar { value, .. } => {
                                let key = program.str_interner.intern(&value);
                                stack.push(StackItem::Object(fields, key));
                                event = parser.parse()?;
                                break;
                            }
                            _ => unreachable!(),
                        }
                    }
                }
            } else {
                assert!(matches!(
                    event.data,
                    libyaml_safer::EventData::DocumentEnd { .. }
                ));
                return Ok(value);
            }
        }
    }
}

fn scalar_to_value(
    style: libyaml_safer::ScalarStyle,
    value: &str,
) -> Result<ValueData, ParseError> {
    if style == libyaml_safer::ScalarStyle::Plain {
        match value {
            "null" | "Null" | "NULL" | "~" => Ok(ValueData::Null),
            "true" | "True" | "TRUE" => Ok(ValueData::Bool(true)),
            "false" | "False" | "FALSE" => Ok(ValueData::Bool(false)),
            _ => {
                let number = try_parse_number(value)
                    .or_else(|| try_parse_octal_number(value))
                    .or_else(|| try_parse_hex_number(value));
                if let Some(number) = number {
                    if !number.is_finite() {
                        return Err(ParseError::NumberOverflow);
                    }
                    Ok(ValueData::Number(number))
                } else {
                    Ok(ValueData::String(value.into()))
                }
            }
        }
    } else {
        Ok(ValueData::String(value.into()))
    }
}

fn try_parse_number(s: &str) -> Option<f64> {
    enum State {
        Start,
        Sign,
        IntPart,
        Dot,
        LeadingDot,
        FracPart,
        E,
        ESign,
        EDigits,
    }

    let mut state = State::Start;
    let mut iter = s.chars();
    loop {
        match state {
            State::Start => match iter.next() {
                Some('-' | '+') => state = State::Sign,
                Some('0'..='9') => state = State::IntPart,
                Some('.') => state = State::LeadingDot,
                _ => return None,
            },
            State::Sign => match iter.next() {
                Some('0'..='9') => state = State::IntPart,
                Some('.') => state = State::LeadingDot,
                _ => return None,
            },
            State::IntPart => match iter.next() {
                None => break,
                Some('0'..='9') => state = State::IntPart,
                Some('.') => state = State::Dot,
                Some('e' | 'E') => state = State::E,
                Some(_) => return None,
            },
            State::Dot => match iter.next() {
                None => break,
                Some('0'..='9') => state = State::FracPart,
                Some('e' | 'E') => state = State::E,
                _ => return None,
            },
            State::LeadingDot => match iter.next() {
                Some('0'..='9') => state = State::FracPart,
                _ => return None,
            },
            State::FracPart => match iter.next() {
                None => break,
                Some('0'..='9') => state = State::FracPart,
                Some('e' | 'E') => state = State::E,
                Some(_) => return None,
            },
            State::E => match iter.next() {
                Some('-' | '+') => state = State::ESign,
                Some('0'..='9') => state = State::EDigits,
                _ => return None,
            },
            State::ESign => match iter.next() {
                Some('0'..='9') => state = State::EDigits,
                _ => return None,
            },
            State::EDigits => match iter.next() {
                None => break,
                Some('0'..='9') => state = State::EDigits,
                Some(_) => return None,
            },
        }
    }

    assert!(!s.is_empty() && iter.as_str().is_empty());

    let number = s.parse().unwrap();
    Some(number)
}

fn try_parse_octal_number(s: &str) -> Option<f64> {
    let digits = s.strip_prefix("0o")?;
    if digits.is_empty() {
        return None;
    }

    let mut int = 0u128;
    let mut chars = digits.chars().peekable();
    while let Some(chr) = chars.peek() {
        let digit = chr.to_digit(8)?;
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
        let digit = chr.to_digit(8)?;
        number = number.mul_add(8.0, f64::from(digit));
    }

    Some(number)
}

fn try_parse_hex_number(s: &str) -> Option<f64> {
    let digits = s.strip_prefix("0x")?;
    if digits.is_empty() {
        return None;
    }

    let mut int = 0u128;
    let mut chars = digits.chars().peekable();
    while let Some(chr) = chars.peek() {
        let digit = chr.to_digit(16)?;
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
        let digit = chr.to_digit(16)?;
        number = number.mul_add(16.0, f64::from(digit));
    }

    Some(number)
}
