use std::cell::OnceCell;

use super::{ObjectData, ObjectField, Program, ThunkData, ValueData};
use crate::gc::Gc;
use crate::interner::InternedStr;
use crate::{ast, FHashMap};

pub(super) struct ParseError {
    line: usize,
    column: usize,
    kind: ParseErrorKind,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "line {}, column {}: {}",
            self.line + 1,
            self.column + 1,
            self.kind,
        )
    }
}

enum ParseErrorKind {
    ExpectedValue,
    ExpectedEof,
    Expected1(char),
    Expected2(char, char),
    InvalidNumber,
    NumberOverflow,
    UnfinishedString,
    InvalidChrInString,
    InvalidStringEscape,
    ExpectedObjectKey,
    RepeatedFieldName(InternedStr),
}

impl std::fmt::Display for ParseErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ExpectedValue => write!(f, "expected value"),
            Self::ExpectedEof => write!(f, "expected end-of-file"),
            Self::Expected1(chr) => write!(f, "expected `{chr}`"),
            Self::Expected2(chr1, chr2) => write!(f, "expected `{chr1}` or `{chr2}`"),
            Self::ExpectedObjectKey => write!(f, "expected object key"),
            Self::InvalidNumber => write!(f, "invalid number"),
            Self::NumberOverflow => write!(f, "number overflow"),
            Self::UnfinishedString => write!(f, "unfinished string"),
            Self::InvalidChrInString => write!(f, "invalid character in string"),
            Self::InvalidStringEscape => write!(f, "invalid string escape"),
            Self::RepeatedFieldName(name) => {
                write!(f, "repeated field name {:?}", name.value())
            }
        }
    }
}

pub(super) fn parse_json(program: &mut Program, s: &str) -> Result<ValueData, ParseError> {
    enum StackItem {
        Array(Vec<Gc<ThunkData>>),
        Object(FHashMap<InternedStr, ObjectField>, InternedStr),
    }

    let mut lexer = Lexer {
        line: 0,
        column: 0,
        rem: s,
    };

    let mut stack = Vec::new();
    lexer.skip_spaces();
    loop {
        let mut value = if lexer.eat_str("null") {
            lexer.skip_spaces();
            ValueData::Null
        } else if lexer.eat_str("false") {
            lexer.skip_spaces();
            ValueData::Bool(false)
        } else if lexer.eat_str("true") {
            lexer.skip_spaces();
            ValueData::Bool(true)
        } else if let Some(number) = lexer.lex_number()? {
            lexer.skip_spaces();
            ValueData::Number(number)
        } else if let Some(s) = lexer.lex_string()? {
            lexer.skip_spaces();
            ValueData::String(s.into())
        } else if lexer.eat_char('[') {
            lexer.skip_spaces();
            if lexer.eat_char(']') {
                lexer.skip_spaces();
                ValueData::Array(Gc::from(&program.empty_array))
            } else {
                stack.push(StackItem::Array(Vec::new()));
                continue;
            }
        } else if lexer.eat_char('{') {
            lexer.skip_spaces();
            if lexer.eat_char('}') {
                lexer.skip_spaces();
                ValueData::Object(program.gc_alloc(ObjectData::new_simple(FHashMap::default())))
            } else {
                let first_key = lexer
                    .lex_string()?
                    .ok_or_else(|| lexer.get_error(ParseErrorKind::ExpectedObjectKey))?;
                lexer.skip_spaces();
                if !lexer.eat_char(':') {
                    return Err(lexer.get_error(ParseErrorKind::Expected1(':')));
                }
                lexer.skip_spaces();
                stack.push(StackItem::Object(
                    FHashMap::default(),
                    program.str_interner.intern(&first_key),
                ));
                continue;
            }
        } else {
            return Err(lexer.get_error(ParseErrorKind::ExpectedValue));
        };

        loop {
            if let Some(stack_item) = stack.pop() {
                match stack_item {
                    StackItem::Array(mut items) => {
                        items.push(program.gc_alloc(ThunkData::new_done(value)));
                        if lexer.eat_char(']') {
                            lexer.skip_spaces();
                            value = ValueData::Array(program.gc_alloc(items.into_boxed_slice()));
                        } else if lexer.eat_char(',') {
                            lexer.skip_spaces();
                            stack.push(StackItem::Array(items));
                            break;
                        } else {
                            return Err(lexer.get_error(ParseErrorKind::Expected2(']', ',')));
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
                                return Err(lexer.get_error(ParseErrorKind::RepeatedFieldName(
                                    entry.key().clone(),
                                )));
                            }
                        }

                        if lexer.eat_char('}') {
                            lexer.skip_spaces();
                            value =
                                ValueData::Object(program.gc_alloc(ObjectData::new_simple(fields)));
                        } else if lexer.eat_char(',') {
                            lexer.skip_spaces();
                            let next_key = lexer.lex_string()?.ok_or_else(|| {
                                lexer.get_error(ParseErrorKind::ExpectedObjectKey)
                            })?;
                            lexer.skip_spaces();
                            if !lexer.eat_char(':') {
                                return Err(lexer.get_error(ParseErrorKind::Expected1(':')));
                            }
                            lexer.skip_spaces();
                            stack.push(StackItem::Object(
                                fields,
                                program.str_interner.intern(&next_key),
                            ));
                            break;
                        } else {
                            return Err(lexer.get_error(ParseErrorKind::Expected2('}', ',')));
                        }
                    }
                }
            } else {
                if !lexer.rem.is_empty() {
                    return Err(lexer.get_error(ParseErrorKind::ExpectedEof));
                }
                return Ok(value);
            }
        }
    }
}

struct Lexer<'a> {
    line: usize,
    column: usize,
    rem: &'a str,
}

impl Lexer<'_> {
    #[inline]
    fn get_error(&self, kind: ParseErrorKind) -> ParseError {
        ParseError {
            line: self.line,
            column: self.column,
            kind,
        }
    }

    fn skip_spaces(&mut self) {
        loop {
            if let Some(rem) = self.rem.strip_prefix('\t') {
                self.column += 1;
                self.rem = rem;
            } else if let Some(rem) = self.rem.strip_prefix('\n') {
                self.line += 1;
                self.column = 0;
                self.rem = rem;
            } else if let Some(rem) = self.rem.strip_prefix('\r') {
                self.column += 1;
                self.rem = rem;
            } else if let Some(rem) = self.rem.strip_prefix(' ') {
                self.column += 1;
                self.rem = rem;
            } else {
                break;
            }
        }
    }

    #[inline]
    fn eat_char(&mut self, c: char) -> bool {
        if let Some(rem) = self.rem.strip_prefix(c) {
            self.column += 1;
            self.rem = rem;
            true
        } else {
            false
        }
    }

    #[inline]
    fn eat_digit_0_9(&mut self) -> bool {
        let mut chr_iter = self.rem.chars();
        if let Some('0'..='9') = chr_iter.next() {
            self.column += 1;
            self.rem = chr_iter.as_str();
            true
        } else {
            false
        }
    }

    #[inline]
    fn eat_digit_1_9(&mut self) -> bool {
        let mut chr_iter = self.rem.chars();
        if let Some('1'..='9') = chr_iter.next() {
            self.column += 1;
            self.rem = chr_iter.as_str();
            true
        } else {
            false
        }
    }

    #[inline]
    fn eat_str(&mut self, s: &str) -> bool {
        if let Some(rem) = self.rem.strip_prefix(s) {
            self.column += s.len(); // assume ASCII
            self.rem = rem;
            true
        } else {
            false
        }
    }

    #[inline]
    fn eat_any_char(&mut self) -> Option<char> {
        let mut chr_iter = self.rem.chars();
        if let Some(chr) = chr_iter.next() {
            self.column += 1;
            self.rem = chr_iter.as_str();
            Some(chr)
        } else {
            None
        }
    }

    fn lex_number(&mut self) -> Result<Option<f64>, ParseError> {
        enum State {
            Start,
            Minus,
            Zero,
            IntPart,
            Dot,
            FracPart,
            E,
            ESign,
            EDigits,
        }

        let mut state = State::Start;
        let start_column = self.column;
        let init_rem = self.rem;
        loop {
            match state {
                State::Start => {
                    if self.eat_char('-') {
                        state = State::Minus;
                    } else if self.eat_char('0') {
                        state = State::Zero;
                    } else if self.eat_digit_1_9() {
                        state = State::IntPart;
                    } else {
                        break;
                    }
                }
                State::Minus => {
                    if self.eat_char('0') {
                        state = State::Zero;
                    } else if self.eat_digit_1_9() {
                        state = State::IntPart;
                    } else {
                        return Err(ParseError {
                            line: self.line,
                            column: start_column,
                            kind: ParseErrorKind::InvalidNumber,
                        });
                    }
                }
                State::Zero => {
                    if self.eat_digit_0_9() {
                        return Err(ParseError {
                            line: self.line,
                            column: start_column,
                            kind: ParseErrorKind::InvalidNumber,
                        });
                    } else if self.eat_char('.') {
                        state = State::Dot;
                    } else if self.eat_char('e') || self.eat_char('E') {
                        state = State::E;
                    } else {
                        break;
                    }
                }
                State::IntPart => {
                    if self.eat_digit_0_9() {
                        state = State::IntPart;
                    } else if self.eat_char('.') {
                        state = State::Dot;
                    } else if self.eat_char('e') || self.eat_char('E') {
                        state = State::E;
                    } else {
                        break;
                    }
                }
                State::Dot => {
                    if self.eat_digit_0_9() {
                        state = State::FracPart;
                    } else {
                        return Err(ParseError {
                            line: self.line,
                            column: start_column,
                            kind: ParseErrorKind::InvalidNumber,
                        });
                    }
                }
                State::FracPart => {
                    if self.eat_digit_0_9() {
                        state = State::FracPart;
                    } else if self.eat_char('e') || self.eat_char('E') {
                        state = State::E;
                    } else {
                        break;
                    }
                }
                State::E => {
                    if self.eat_char('-') || self.eat_char('+') {
                        state = State::ESign;
                    } else if self.eat_digit_0_9() {
                        state = State::EDigits;
                    } else {
                        return Err(ParseError {
                            line: self.line,
                            column: start_column,
                            kind: ParseErrorKind::InvalidNumber,
                        });
                    }
                }
                State::ESign => {
                    if self.eat_digit_0_9() {
                        state = State::EDigits;
                    } else {
                        return Err(ParseError {
                            line: self.line,
                            column: start_column,
                            kind: ParseErrorKind::InvalidNumber,
                        });
                    }
                }
                State::EDigits => {
                    if self.eat_digit_0_9() {
                        state = State::EDigits;
                    } else {
                        break;
                    }
                }
            }
        }

        let num_len = init_rem.len() - self.rem.len();
        let number = &init_rem[..num_len];
        if number.is_empty() {
            Ok(None)
        } else {
            let number = number.parse::<f64>().unwrap();
            if number.is_finite() {
                Ok(Some(number))
            } else {
                Err(ParseError {
                    line: self.line,
                    column: start_column,
                    kind: ParseErrorKind::NumberOverflow,
                })
            }
        }
    }

    fn lex_string(&mut self) -> Result<Option<String>, ParseError> {
        if !self.eat_char('"') {
            return Ok(None);
        }

        let start_col = self.column;
        let mut string = String::new();
        loop {
            if self.eat_char('"') {
                break;
            }
            let chr_col = self.column;
            let chr = self.eat_any_char().ok_or(ParseError {
                line: self.line,
                column: start_col,
                kind: ParseErrorKind::UnfinishedString,
            })?;
            match chr {
                '\\' => match self.eat_any_char() {
                    None => {
                        return Err(ParseError {
                            line: self.line,
                            column: start_col,
                            kind: ParseErrorKind::UnfinishedString,
                        });
                    }
                    Some('"') => {
                        string.push('"');
                    }
                    Some('\\') => {
                        string.push('\\');
                    }
                    Some('/') => {
                        string.push('/');
                    }
                    Some('b') => {
                        string.push('\u{8}');
                    }
                    Some('f') => {
                        string.push('\u{C}');
                    }
                    Some('n') => {
                        string.push('\n');
                    }
                    Some('r') => {
                        string.push('\r');
                    }
                    Some('t') => {
                        string.push('\t');
                    }
                    Some('u') => {
                        let hex_from_digit = |b: char| match b {
                            '0'..='9' => Some((b as u8) - b'0'),
                            'a'..='f' => Some((b as u8) - b'a' + 10),
                            'A'..='F' => Some((b as u8) - b'A' + 10),
                            _ => None,
                        };

                        let eat_codeunit = |this: &mut Self| -> Option<u16> {
                            let d0 = this.eat_any_char().and_then(hex_from_digit)?;
                            let d1 = this.eat_any_char().and_then(hex_from_digit)?;
                            let d2 = this.eat_any_char().and_then(hex_from_digit)?;
                            let d3 = this.eat_any_char().and_then(hex_from_digit)?;
                            Some(
                                (u16::from(d0) << 12)
                                    | (u16::from(d1) << 8)
                                    | (u16::from(d2) << 4)
                                    | u16::from(d3),
                            )
                        };

                        let Some(cu1) = eat_codeunit(self) else {
                            return Err(ParseError {
                                line: self.line,
                                column: chr_col,
                                kind: ParseErrorKind::InvalidStringEscape,
                            });
                        };

                        if matches!(cu1, 0xD800..=0xDFFF) && self.eat_str("\\u") {
                            let Some(cu2) = eat_codeunit(self) else {
                                return Err(ParseError {
                                    line: self.line,
                                    column: chr_col,
                                    kind: ParseErrorKind::InvalidStringEscape,
                                });
                            };
                            if let Ok(chr) = char::decode_utf16([cu1, cu2]).next().unwrap() {
                                string.push(chr);
                            } else {
                                return Err(ParseError {
                                    line: self.line,
                                    column: chr_col,
                                    kind: ParseErrorKind::InvalidStringEscape,
                                });
                            }
                        } else if let Some(chr) = char::from_u32(cu1.into()) {
                            string.push(chr);
                        } else {
                            return Err(ParseError {
                                line: self.line,
                                column: chr_col,
                                kind: ParseErrorKind::InvalidStringEscape,
                            });
                        }
                    }
                    Some(_) => {
                        return Err(ParseError {
                            line: self.line,
                            column: chr_col,
                            kind: ParseErrorKind::InvalidStringEscape,
                        });
                    }
                },
                chr if !chr.is_ascii_control() => {
                    string.push(chr);
                }
                _ => {
                    return Err(ParseError {
                        line: self.line,
                        column: chr_col,
                        kind: ParseErrorKind::InvalidChrInString,
                    });
                }
            }
        }

        Ok(Some(string))
    }
}
