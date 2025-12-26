use std::cell::OnceCell;

use super::{ArrayData, ObjectData, ObjectField, Program, ThunkData, ValueData};
use crate::gc::Gc;
use crate::interner::InternedStr;
use crate::{FHashMap, ast};

pub(crate) enum ParseError<'p> {
    Parser(saphyr_parser::ScanError),
    EmptyStream,
    Tag,
    AnchorContainsItself,
    AnchorFromOtherDoc,
    KeyIsObject(saphyr_parser::Marker),
    KeyIsArray(saphyr_parser::Marker),
    NumberOverflow,
    RepeatedFieldName(InternedStr<'p>),
}

impl From<saphyr_parser::ScanError> for ParseError<'_> {
    #[inline]
    fn from(e: saphyr_parser::ScanError) -> Self {
        Self::Parser(e)
    }
}

impl std::fmt::Display for ParseError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Parser(ref e) => write!(f, "{e}"),
            Self::EmptyStream => write!(f, "empty stream"),
            Self::Tag => write!(f, "tags are not allowed"),
            Self::AnchorContainsItself => write!(f, "anchor contains itself"),
            Self::AnchorFromOtherDoc => {
                write!(f, "alias refers to an anchor from a previous document")
            }
            Self::KeyIsObject(ref marker) => write!(
                f,
                "object key is an object at line {} column {}",
                marker.line(),
                marker.col(),
            ),
            Self::KeyIsArray(ref marker) => write!(
                f,
                "object key is an array at line {} column {}",
                marker.line(),
                marker.col(),
            ),
            Self::NumberOverflow => write!(f, "number overflow"),
            Self::RepeatedFieldName(ref name) => {
                write!(f, "repeated field name {:?}", name.value())
            }
        }
    }
}

pub(super) fn parse_yaml<'p>(
    program: &mut Program<'p>,
    s: &str,
) -> Result<ValueData<'p>, ParseError<'p>> {
    let mut parser = saphyr_parser::Parser::new_from_str(s);

    let (first_event, _) = parser.next_event().unwrap()?;
    if let saphyr_parser::Event::StreamStart = first_event {
        drop(first_event);
    } else {
        unreachable!();
    }

    enum StreamKind<'p> {
        Empty,
        Single(ValueData<'p>),
        Stream(Vec<Gc<ThunkData<'p>>>),
    }

    let mut stream_kind = StreamKind::Empty;
    loop {
        let (event, _) = parser.next_event().unwrap()?;
        match event {
            saphyr_parser::Event::StreamEnd => break,
            saphyr_parser::Event::DocumentStart(explicit) => {
                let value = parse_yaml_document(program, &mut parser)?;
                match stream_kind {
                    StreamKind::Empty => {
                        if explicit {
                            stream_kind = StreamKind::Stream(vec![
                                program.gc_alloc(ThunkData::new_done(value)),
                            ]);
                        } else {
                            stream_kind = StreamKind::Single(value);
                        }
                    }
                    StreamKind::Single(item0) => {
                        stream_kind = StreamKind::Stream(vec![
                            program.gc_alloc(ThunkData::new_done(item0)),
                            program.gc_alloc(ThunkData::new_done(value)),
                        ]);
                    }
                    StreamKind::Stream(ref mut items) => {
                        items.push(program.gc_alloc(ThunkData::new_done(value)));
                    }
                }
            }
            _ => unreachable!(),
        }
    }

    match stream_kind {
        StreamKind::Empty => Err(ParseError::EmptyStream),
        StreamKind::Single(value) => Ok(value),
        StreamKind::Stream(items) => {
            if items.is_empty() {
                Ok(ValueData::Array(Gc::from(&program.empty_array)))
            } else {
                Ok(ValueData::Array(program.gc_alloc(items.into_boxed_slice())))
            }
        }
    }
}

enum AnchorValue<'p, 'a> {
    Pending,
    Scalar {
        style: saphyr_parser::ScalarStyle,
        value: std::borrow::Cow<'a, str>,
    },
    Array(Gc<ArrayData<'p>>),
    Object(Gc<ObjectData<'p>>),
}

fn parse_yaml_document<'p>(
    program: &mut Program<'p>,
    parser: &mut saphyr_parser::Parser<'_, saphyr_parser::StrInput<'_>>,
) -> Result<ValueData<'p>, ParseError<'p>> {
    enum StackItem<'p> {
        Array {
            anchor_id: usize,
            items: Vec<Gc<ThunkData<'p>>>,
        },
        Object {
            anchor_id: usize,
            fields: FHashMap<InternedStr<'p>, ObjectField<'p>>,
            current_key: InternedStr<'p>,
        },
    }

    let mut anchors = FHashMap::default();
    let mut stack = Vec::new();
    let mut event = parser.next_event().unwrap()?;
    loop {
        let mut value = match event.0 {
            saphyr_parser::Event::Alias(anchor_id) => {
                let Some(value) = anchors.get(&anchor_id) else {
                    return Err(ParseError::AnchorFromOtherDoc);
                };
                match value {
                    AnchorValue::Pending => {
                        return Err(ParseError::AnchorContainsItself);
                    }
                    AnchorValue::Scalar { style, value } => scalar_to_value(*style, value)?,
                    AnchorValue::Array(array) => ValueData::Array(array.clone()),
                    AnchorValue::Object(object) => ValueData::Object(object.clone()),
                }
            }
            saphyr_parser::Event::Scalar(value, style, anchor_id, tag) => {
                if tag.is_some() {
                    return Err(ParseError::Tag);
                }

                let program_value = scalar_to_value(style, &value)?;
                anchors.insert(anchor_id, AnchorValue::Scalar { style, value });

                program_value
            }
            saphyr_parser::Event::SequenceStart(anchor_id, tag) => {
                if tag.is_some() {
                    return Err(ParseError::Tag);
                }
                event = parser.next_event().unwrap()?;
                if matches!(event.0, saphyr_parser::Event::SequenceEnd) {
                    anchors.insert(
                        anchor_id,
                        AnchorValue::Array(Gc::from(&program.empty_array)),
                    );
                    ValueData::Array(Gc::from(&program.empty_array))
                } else {
                    stack.push(StackItem::Array {
                        anchor_id,
                        items: Vec::new(),
                    });
                    anchors.insert(anchor_id, AnchorValue::Pending);
                    continue;
                }
            }
            saphyr_parser::Event::MappingStart(anchor_id, tag) => {
                if tag.is_some() {
                    return Err(ParseError::Tag);
                }
                event = parser.next_event().unwrap()?;
                match event.0 {
                    saphyr_parser::Event::Alias(key_anchor_id) => {
                        anchors.insert(anchor_id, AnchorValue::Pending);
                        let Some(value) = anchors.get(&key_anchor_id) else {
                            return Err(ParseError::AnchorFromOtherDoc);
                        };
                        match value {
                            AnchorValue::Pending => {
                                return Err(ParseError::AnchorContainsItself);
                            }
                            AnchorValue::Scalar { value, .. } => {
                                let key = program.intern_str(value);
                                stack.push(StackItem::Object {
                                    anchor_id,
                                    fields: FHashMap::default(),
                                    current_key: key,
                                });
                                event = parser.next_event().unwrap()?;
                                continue;
                            }
                            AnchorValue::Array(_) => {
                                return Err(ParseError::KeyIsArray(event.1.start));
                            }
                            AnchorValue::Object(_) => {
                                return Err(ParseError::KeyIsObject(event.1.start));
                            }
                        }
                    }
                    saphyr_parser::Event::SequenceStart(..) => {
                        return Err(ParseError::KeyIsArray(event.1.start));
                    }
                    saphyr_parser::Event::MappingStart(..) => {
                        return Err(ParseError::KeyIsObject(event.1.start));
                    }
                    saphyr_parser::Event::MappingEnd => {
                        let object = program.gc_alloc(ObjectData::new_simple(FHashMap::default()));
                        anchors.insert(anchor_id, AnchorValue::Object(object.clone()));
                        ValueData::Object(object)
                    }
                    saphyr_parser::Event::Scalar(value, style, key_anchor_id, tag) => {
                        if tag.is_some() {
                            return Err(ParseError::Tag);
                        }

                        let key = program.intern_str(&value);
                        anchors.insert(key_anchor_id, AnchorValue::Scalar { style, value });
                        stack.push(StackItem::Object {
                            anchor_id,
                            fields: FHashMap::default(),
                            current_key: key,
                        });
                        anchors.insert(anchor_id, AnchorValue::Pending);
                        event = parser.next_event().unwrap()?;
                        continue;
                    }
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        };

        event = parser.next_event().unwrap()?;
        loop {
            if let Some(stack_item) = stack.pop() {
                match stack_item {
                    StackItem::Array {
                        anchor_id,
                        mut items,
                    } => {
                        items.push(program.gc_alloc(ThunkData::new_done(value)));
                        if matches!(event.0, saphyr_parser::Event::SequenceEnd) {
                            let array = program.gc_alloc(items.into_boxed_slice());
                            anchors.insert(anchor_id, AnchorValue::Array(array.clone()));
                            value = ValueData::Array(array);

                            event = parser.next_event().unwrap()?;
                        } else {
                            stack.push(StackItem::Array { anchor_id, items });
                            break;
                        }
                    }
                    StackItem::Object {
                        anchor_id,
                        mut fields,
                        current_key,
                    } => {
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
                                return Err(ParseError::RepeatedFieldName(*entry.key()));
                            }
                        }

                        match event.0 {
                            saphyr_parser::Event::Alias(key_anchor_id) => {
                                let Some(value) = anchors.get(&key_anchor_id) else {
                                    return Err(ParseError::AnchorFromOtherDoc);
                                };
                                match value {
                                    AnchorValue::Pending => {
                                        return Err(ParseError::AnchorContainsItself);
                                    }
                                    AnchorValue::Scalar { value, .. } => {
                                        let key = program.intern_str(value);
                                        stack.push(StackItem::Object {
                                            anchor_id,
                                            fields,
                                            current_key: key,
                                        });
                                        event = parser.next_event().unwrap()?;
                                        break;
                                    }
                                    AnchorValue::Array(_) => {
                                        return Err(ParseError::KeyIsArray(event.1.start));
                                    }
                                    AnchorValue::Object(_) => {
                                        return Err(ParseError::KeyIsObject(event.1.start));
                                    }
                                }
                            }
                            saphyr_parser::Event::SequenceStart(..) => {
                                return Err(ParseError::KeyIsArray(event.1.start));
                            }
                            saphyr_parser::Event::MappingStart(..) => {
                                return Err(ParseError::KeyIsObject(event.1.start));
                            }
                            saphyr_parser::Event::MappingEnd => {
                                let object = program.gc_alloc(ObjectData::new_simple(fields));
                                anchors.insert(anchor_id, AnchorValue::Object(object.clone()));
                                value = ValueData::Object(object);

                                event = parser.next_event().unwrap()?;
                            }
                            saphyr_parser::Event::Scalar(value, style, key_anchor_id, tag) => {
                                if tag.is_some() {
                                    return Err(ParseError::Tag);
                                }

                                let key = program.intern_str(&value);
                                anchors.insert(key_anchor_id, AnchorValue::Scalar { style, value });
                                stack.push(StackItem::Object {
                                    anchor_id,
                                    fields,
                                    current_key: key,
                                });
                                event = parser.next_event().unwrap()?;
                                break;
                            }
                            _ => unreachable!(),
                        }
                    }
                }
            } else {
                assert!(matches!(event.0, saphyr_parser::Event::DocumentEnd));
                return Ok(value);
            }
        }
    }
}

fn scalar_to_value<'p>(
    style: saphyr_parser::ScalarStyle,
    value: &str,
) -> Result<ValueData<'p>, ParseError<'p>> {
    if style == saphyr_parser::ScalarStyle::Plain {
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
    super::parse_num_radix::<8>(digits).ok()
}

fn try_parse_hex_number(s: &str) -> Option<f64> {
    let digits = s.strip_prefix("0x")?;
    super::parse_num_radix::<16>(digits).ok()
}
