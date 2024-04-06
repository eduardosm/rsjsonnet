use std::rc::Rc;

use super::super::{ArrayData, ObjectData, ThunkData, ValueData};
use super::manifest::ManifestJsonFormat;
use super::{EvalError, EvalErrorKind, EvalErrorValueType, Evaluator, State};
use crate::float;
use crate::gc::GcView;

pub(super) enum FormatPart {
    Literal(String),
    Code(FormatCode),
}

pub(super) struct FormatCode {
    mkey: Option<String>,
    cflags: CFlags,
    fw: Option<FieldWidth>,
    prec: Option<FieldWidth>,
    _len_mod: Option<LenMod>,
    ctype: ConvType,
}

impl FormatCode {
    fn uses_prec(&self) -> bool {
        match self.ctype {
            ConvType::Decimal
            | ConvType::Octal
            | ConvType::HexLower
            | ConvType::HexUpper
            | ConvType::ExpLower
            | ConvType::ExpUpper
            | ConvType::FloatLower
            | ConvType::FloatUpper
            | ConvType::FloatGLower
            | ConvType::FloatGUpper => true,
            ConvType::Char | ConvType::String | ConvType::Percent => false,
        }
    }
}

struct CFlags {
    alt: bool,
    zero: bool,
    left: bool,
    blank: bool,
    plus: bool,
}

enum LenMod {
    H,
    LowerL,
    UpperL,
}

#[derive(PartialEq, Eq)]
enum ConvType {
    Decimal,
    Octal,
    HexLower,
    HexUpper,
    ExpLower,
    ExpUpper,
    FloatLower,
    FloatUpper,
    FloatGLower,
    FloatGUpper,
    Char,
    String,
    Percent,
}

enum FieldWidth {
    Inline(u32),
    External,
}

enum WidthTmp {
    None,
    Inline(u32),
    Thunk(GcView<ThunkData>),
}

impl Evaluator<'_> {
    pub(super) fn parse_format_codes(&mut self, fmt: &str) -> Result<Vec<FormatPart>, EvalError> {
        let mut parts = Vec::new();
        let mut rem = fmt;
        while !rem.is_empty() {
            let Some(i) = rem.find('%') else {
                parts.push(FormatPart::Literal(rem.into()));
                break;
            };
            if i > 0 {
                parts.push(FormatPart::Literal(rem[..i].into()));
            }
            rem = &rem[(i + 1)..];
            let mkey = self.parse_format_mkey(&mut rem)?;
            let cflags = self.parse_format_cflags(&mut rem)?;
            let fw = self.parse_format_field_width(&mut rem)?;
            let prec = self.parse_format_prec(&mut rem)?;
            let len_mod = self.parse_format_length_modifier(&mut rem)?;
            let conv_type = self.parse_format_conv_type(&mut rem)?;
            parts.push(FormatPart::Code(FormatCode {
                mkey,
                cflags,
                fw,
                prec,
                _len_mod: len_mod,
                ctype: conv_type,
            }));
        }
        Ok(parts)
    }

    fn parse_format_mkey(&mut self, rem: &mut &str) -> Result<Option<String>, EvalError> {
        if let Some(rem_cont) = rem.strip_prefix('(') {
            let Some(end) = rem_cont.find(')') else {
                return Err(self.report_error(EvalErrorKind::Other {
                    span: None,
                    message: "truncated format code".into(),
                }));
            };
            *rem = &rem_cont[(end + 1)..];
            Ok(Some(rem_cont[..end].into()))
        } else {
            Ok(None)
        }
    }

    fn parse_format_cflags(&mut self, rem: &mut &str) -> Result<CFlags, EvalError> {
        let mut cflags = CFlags {
            alt: false,
            zero: false,
            left: false,
            blank: false,
            plus: false,
        };
        loop {
            if let Some(rem_cont) = rem.strip_prefix('#') {
                cflags.alt = true;
                *rem = rem_cont;
            } else if let Some(rem_cont) = rem.strip_prefix('0') {
                cflags.zero = true;
                *rem = rem_cont;
            } else if let Some(rem_cont) = rem.strip_prefix('-') {
                cflags.left = true;
                *rem = rem_cont;
            } else if let Some(rem_cont) = rem.strip_prefix(' ') {
                cflags.blank = true;
                *rem = rem_cont;
            } else if let Some(rem_cont) = rem.strip_prefix('+') {
                cflags.plus = true;
                *rem = rem_cont;
            } else {
                break;
            }
        }
        Ok(cflags)
    }

    fn parse_format_field_width(
        &mut self,
        rem: &mut &str,
    ) -> Result<Option<FieldWidth>, EvalError> {
        if let Some(rem_cont) = rem.strip_prefix('*') {
            *rem = rem_cont;
            Ok(Some(FieldWidth::External))
        } else {
            let mut i = 0;
            let bytes = rem.as_bytes();
            while bytes.get(i).is_some_and(u8::is_ascii_digit) {
                i += 1;
            }
            if i == 0 {
                Ok(None)
            } else {
                let width = rem[..i].parse().map_err(|_| {
                    self.report_error(EvalErrorKind::Other {
                        span: None,
                        message: "format field width is too large".into(),
                    })
                })?;
                *rem = &rem[i..];
                Ok(Some(FieldWidth::Inline(width)))
            }
        }
    }

    fn parse_format_prec(&mut self, rem: &mut &str) -> Result<Option<FieldWidth>, EvalError> {
        if let Some(rem_cont) = rem.strip_prefix('.') {
            if let Some(rem_cont) = rem_cont.strip_prefix('*') {
                *rem = rem_cont;
                Ok(Some(FieldWidth::External))
            } else {
                let mut i = 0;
                let bytes = rem_cont.as_bytes();
                while bytes.get(i).is_some_and(u8::is_ascii_digit) {
                    i += 1;
                }
                if i == 0 {
                    if rem_cont.is_empty() {
                        Err(self.report_error(EvalErrorKind::Other {
                            span: None,
                            message: "truncated format code".into(),
                        }))
                    } else {
                        Err(self.report_error(EvalErrorKind::Other {
                            span: None,
                            message: "missing format precision digits".into(),
                        }))
                    }
                } else {
                    let width = rem_cont[..i].parse().map_err(|_| {
                        self.report_error(EvalErrorKind::Other {
                            span: None,
                            message: "format precision is too large".into(),
                        })
                    })?;
                    *rem = &rem_cont[i..];
                    Ok(Some(FieldWidth::Inline(width)))
                }
            }
        } else {
            Ok(None)
        }
    }

    fn parse_format_length_modifier(
        &mut self,
        rem: &mut &str,
    ) -> Result<Option<LenMod>, EvalError> {
        if let Some(rem_cont) = rem.strip_prefix('h') {
            *rem = rem_cont;
            Ok(Some(LenMod::H))
        } else if let Some(rem_cont) = rem.strip_prefix('l') {
            *rem = rem_cont;
            Ok(Some(LenMod::LowerL))
        } else if let Some(rem_cont) = rem.strip_prefix('L') {
            *rem = rem_cont;
            Ok(Some(LenMod::UpperL))
        } else {
            Ok(None)
        }
    }

    fn parse_format_conv_type(&mut self, rem: &mut &str) -> Result<ConvType, EvalError> {
        let conv_type = match rem.chars().next() {
            None => {
                return Err(self.report_error(EvalErrorKind::Other {
                    span: None,
                    message: "truncated format code".into(),
                }));
            }
            Some('d' | 'i' | 'u') => ConvType::Decimal,
            Some('o') => ConvType::Octal,
            Some('x') => ConvType::HexLower,
            Some('X') => ConvType::HexUpper,
            Some('e') => ConvType::ExpLower,
            Some('E') => ConvType::ExpUpper,
            Some('f') => ConvType::FloatLower,
            Some('F') => ConvType::FloatUpper,
            Some('g') => ConvType::FloatGLower,
            Some('G') => ConvType::FloatGUpper,
            Some('c') => ConvType::Char,
            Some('s') => ConvType::String,
            Some('%') => ConvType::Percent,
            Some(chr) => {
                return Err(self.report_error(EvalErrorKind::Other {
                    span: None,
                    message: format!("invalid format conversion code {chr:?}"),
                }));
            }
        };
        *rem = &rem[1..];
        Ok(conv_type)
    }

    pub(super) fn want_format_array(&mut self, parts: Vec<FormatPart>, array: GcView<ArrayData>) {
        self.string_stack.push(String::new());
        self.state_stack.push(State::StdFormatCodesArray1 {
            parts: Rc::new(parts),
            array,
            part_i: 0,
            array_i: 0,
        });
    }

    pub(super) fn want_format_object(
        &mut self,
        parts: Vec<FormatPart>,
        object: GcView<ObjectData>,
    ) {
        self.string_stack.push(String::new());
        self.state_stack.push(State::StdFormatCodesObject1 {
            parts: Rc::new(parts),
            object,
            part_i: 0,
        });
    }

    pub(super) fn format_codes_array_1(
        &mut self,
        parts: Rc<Vec<FormatPart>>,
        array: GcView<ArrayData>,
        part_i: usize,
        mut array_i: usize,
    ) -> Result<(), EvalError> {
        if part_i >= parts.len() {
            if array_i < array.len() {
                return Err(self.report_error(EvalErrorKind::Other {
                    span: None,
                    message: format!(
                        "too many array items for format: expected {array_i}, got {}",
                        array.len(),
                    ),
                }));
            } else {
                let s = self.string_stack.pop().unwrap();
                self.value_stack.push(ValueData::String(s.into()));
                return Ok(());
            }
        }

        let result = self.string_stack.last_mut().unwrap();
        match parts[part_i] {
            FormatPart::Literal(ref s) => {
                result.push_str(s);
                self.state_stack.push(State::StdFormatCodesArray1 {
                    parts,
                    array,
                    part_i: part_i + 1,
                    array_i,
                });
                Ok(())
            }
            FormatPart::Code(ref code) => {
                let fw = match code.fw {
                    None => WidthTmp::None,
                    Some(FieldWidth::Inline(v)) => WidthTmp::Inline(v),
                    Some(FieldWidth::External) => {
                        if array_i >= array.len() {
                            return Err(self.report_error(EvalErrorKind::Other {
                                span: None,
                                message: format!(
                                    "not enough array items for format, got {}",
                                    array.len(),
                                ),
                            }));
                        }
                        let thunk = array[array_i].view();
                        array_i += 1;
                        WidthTmp::Thunk(thunk)
                    }
                };
                let prec = match code.prec {
                    None => WidthTmp::None,
                    Some(FieldWidth::Inline(v)) => WidthTmp::Inline(v),
                    Some(FieldWidth::External) => {
                        if array_i >= array.len() {
                            return Err(self.report_error(EvalErrorKind::Other {
                                span: None,
                                message: format!(
                                    "not enough array items for format, got {}",
                                    array.len(),
                                ),
                            }));
                        }
                        let thunk = array[array_i].view();
                        array_i += 1;
                        WidthTmp::Thunk(thunk)
                    }
                };

                self.state_stack.push(State::StdFormatCodesArray2 {
                    parts: parts.clone(),
                    array,
                    part_i,
                    array_i,
                });

                if code.uses_prec() {
                    match prec {
                        WidthTmp::None => {}
                        WidthTmp::Inline(v) => {
                            self.state_stack.push(State::PushU32AsValue(v));
                        }
                        WidthTmp::Thunk(thunk) => {
                            self.state_stack.push(State::DoThunk(thunk));
                        }
                    }
                }
                match fw {
                    WidthTmp::None => {}
                    WidthTmp::Inline(v) => {
                        self.state_stack.push(State::PushU32AsValue(v));
                    }
                    WidthTmp::Thunk(thunk) => {
                        self.state_stack.push(State::DoThunk(thunk));
                    }
                }

                Ok(())
            }
        }
    }

    pub(super) fn format_codes_array_2(
        &mut self,
        parts: Rc<Vec<FormatPart>>,
        array: GcView<ArrayData>,
        part_i: usize,
        mut array_i: usize,
    ) -> Result<(), EvalError> {
        let FormatPart::Code(ref code) = parts[part_i] else {
            unreachable!();
        };

        let prec = if code.prec.is_some() && code.uses_prec() {
            let prec = self.value_stack.pop().unwrap();
            let ValueData::Number(prec) = prec else {
                return Err(self.report_error(EvalErrorKind::Other {
                    span: None,
                    message: format!(
                        "format precision must be a number, got {}",
                        EvalErrorValueType::from_value(&prec).to_str(),
                    ),
                }));
            };
            float::try_to_u32(prec)
                .filter(|&v| usize::try_from(v).is_ok())
                .ok_or_else(|| {
                    self.report_error(EvalErrorKind::Other {
                        span: None,
                        message: format!("invalid format precision value: {prec}"),
                    })
                })?
        } else {
            0
        };
        let fw = if code.fw.is_some() {
            let fw = self.value_stack.pop().unwrap();
            let ValueData::Number(fw) = fw else {
                return Err(self.report_error(EvalErrorKind::Other {
                    span: None,
                    message: format!(
                        "format field width must be a number, got {}",
                        EvalErrorValueType::from_value(&fw).to_str(),
                    ),
                }));
            };
            float::try_to_u32(fw)
                .filter(|&v| usize::try_from(v).is_ok())
                .ok_or_else(|| {
                    self.report_error(EvalErrorKind::Other {
                        span: None,
                        message: format!("invalid format field width value: {fw}"),
                    })
                })?
        } else {
            0
        };

        let item = if code.ctype == ConvType::Percent {
            None
        } else {
            let item = array[array_i].view();
            array_i += 1;
            Some(item)
        };

        self.state_stack.push(State::StdFormatCodesArray3 {
            parts: parts.clone(),
            array,
            part_i,
            array_i,
            fw,
        });

        if code.ctype == ConvType::Percent {
            self.string_stack.push("%".into());
        } else {
            self.state_stack.push(State::StdFormatCode {
                parts,
                part_i,
                fw,
                prec,
            });
            self.state_stack.push(State::DoThunk(item.unwrap()));
        }

        Ok(())
    }

    pub(super) fn format_codes_array_3(
        &mut self,
        parts: Rc<Vec<FormatPart>>,
        array: GcView<ArrayData>,
        part_i: usize,
        array_i: usize,
        fw: u32,
    ) -> Result<(), EvalError> {
        let FormatPart::Code(ref code) = parts[part_i] else {
            unreachable!();
        };

        let s = self.string_stack.pop().unwrap();
        let result = self.string_stack.last_mut().unwrap();

        let fw = fw as usize;
        if s.len() < fw {
            let pad_len = fw - s.chars().count();
            if code.cflags.left {
                result.push_str(&s);
                result.extend(std::iter::repeat(' ').take(pad_len));
            } else {
                result.extend(std::iter::repeat(' ').take(pad_len));
                result.push_str(&s);
            }
        } else {
            result.push_str(&s);
        }

        self.state_stack.push(State::StdFormatCodesArray1 {
            parts,
            array,
            part_i: part_i + 1,
            array_i,
        });
        Ok(())
    }

    pub(super) fn format_codes_object_1(
        &mut self,
        parts: Rc<Vec<FormatPart>>,
        object: GcView<ObjectData>,
        part_i: usize,
    ) -> Result<(), EvalError> {
        if part_i >= parts.len() {
            let s = self.string_stack.pop().unwrap();
            self.value_stack.push(ValueData::String(s.into()));
            return Ok(());
        }

        let result = self.string_stack.last_mut().unwrap();
        match parts[part_i] {
            FormatPart::Literal(ref s) => {
                result.push_str(s);
                self.state_stack.push(State::StdFormatCodesObject1 {
                    parts,
                    object,
                    part_i: part_i + 1,
                });
                Ok(())
            }
            FormatPart::Code(ref code) => {
                let fw = match code.fw {
                    None => 0,
                    Some(FieldWidth::Inline(v)) => {
                        if usize::try_from(v).is_err() {
                            return Err(self.report_error(EvalErrorKind::Other {
                                span: None,
                                message: format!("invalid format field width value: {v}"),
                            }));
                        }
                        v
                    }
                    Some(FieldWidth::External) => {
                        return Err(self.report_error(EvalErrorKind::Other {
                            span: None,
                            message: "'*' field width cannot be used with object formatting".into(),
                        }));
                    }
                };
                let prec = match code.prec {
                    None => 0,
                    Some(FieldWidth::Inline(v)) => {
                        if usize::try_from(v).is_err() {
                            return Err(self.report_error(EvalErrorKind::Other {
                                span: None,
                                message: format!("invalid format precision value: {v}"),
                            }));
                        }
                        v
                    }
                    Some(FieldWidth::External) => {
                        return Err(self.report_error(EvalErrorKind::Other {
                            span: None,
                            message: "'*' precision cannot be used with object formatting".into(),
                        }));
                    }
                };

                let item = if code.ctype == ConvType::Percent {
                    None
                } else {
                    let Some(ref mkey) = code.mkey else {
                        return Err(self.report_error(EvalErrorKind::Other {
                            span: None,
                            message: "mapping keys are required with object formatting".into(),
                        }));
                    };
                    let field_name = self.program.str_interner.get_interned(mkey);
                    if let Some(field_thunk) = field_name.and_then(|field_name| {
                        self.program
                            .find_object_field_thunk(&object, 0, &field_name)
                    }) {
                        Some(field_thunk)
                    } else {
                        return Err(self.report_error(EvalErrorKind::Other {
                            span: None,
                            message: format!("missing field {mkey:?} in object formatting"),
                        }));
                    }
                };

                self.state_stack.push(State::StdFormatCodesObject2 {
                    parts: parts.clone(),
                    object: object.clone(),
                    part_i,
                    fw,
                });

                if code.ctype == ConvType::Percent {
                    self.string_stack.push("%".into());
                } else {
                    self.state_stack.push(State::StdFormatCode {
                        parts,
                        part_i,
                        fw,
                        prec,
                    });
                    self.state_stack.push(State::DoThunk(item.unwrap()));
                    self.check_object_asserts(&object);
                }

                Ok(())
            }
        }
    }

    pub(super) fn format_codes_object_2(
        &mut self,
        parts: Rc<Vec<FormatPart>>,
        object: GcView<ObjectData>,
        part_i: usize,
        fw: u32,
    ) -> Result<(), EvalError> {
        let FormatPart::Code(ref code) = parts[part_i] else {
            unreachable!();
        };

        let s = self.string_stack.pop().unwrap();
        let result = self.string_stack.last_mut().unwrap();

        let fw = fw as usize;
        if s.len() < fw {
            let pad_len = fw - s.chars().count();
            if code.cflags.left {
                result.push_str(&s);
                result.extend(std::iter::repeat(' ').take(pad_len));
            } else {
                result.extend(std::iter::repeat(' ').take(pad_len));
                result.push_str(&s);
            }
        } else {
            result.push_str(&s);
        }

        self.state_stack.push(State::StdFormatCodesObject1 {
            parts,
            object,
            part_i: part_i + 1,
        });
        Ok(())
    }

    pub(super) fn format_code(
        &mut self,
        parts: &[FormatPart],
        part_i: usize,
        fw: u32,
        prec: u32,
    ) -> Result<(), EvalError> {
        let FormatPart::Code(ref code) = parts[part_i] else {
            unreachable!();
        };

        let fpprec = if code.prec.is_some() {
            prec as usize
        } else {
            6
        };
        let iprec = if code.prec.is_some() {
            prec as usize
        } else {
            0
        };
        let zp = if code.cflags.zero && !code.cflags.left {
            fw as usize
        } else {
            0
        };

        let value = self.value_stack.pop().unwrap();
        match code.ctype {
            ConvType::Decimal => {
                let ValueData::Number(value) = value else {
                    return Err(self.report_error(EvalErrorKind::Other {
                        span: None,
                        message: format!(
                            "'i' / 'd' formatting requires a number, got {}",
                            EvalErrorValueType::from_value(&value).to_str(),
                        ),
                    }));
                };

                let value = value.trunc();
                let value_abs = value.abs();
                let is_neg = value.is_sign_negative() && value != 0.0;

                let digits_str = value_abs.abs().to_string();
                let s = decorate_digits(
                    &digits_str,
                    is_neg,
                    zp,
                    iprec,
                    code.cflags.plus,
                    code.cflags.blank,
                );

                self.string_stack.push(s);
            }
            ConvType::Octal => {
                let ValueData::Number(value) = value else {
                    return Err(self.report_error(EvalErrorKind::Other {
                        span: None,
                        message: format!(
                            "'o' formatting requires a number, got {}",
                            EvalErrorValueType::from_value(&value).to_str(),
                        ),
                    }));
                };
                let value = value.trunc();
                let neg = value.is_sign_negative() && value != 0.0;
                let mag = value.abs();
                let zero_prefix = if code.cflags.alt { "0" } else { "" };

                let s = self.render_int(
                    neg,
                    mag,
                    zp,
                    iprec,
                    code.cflags.blank,
                    code.cflags.plus,
                    8,
                    zero_prefix,
                );
                self.string_stack.push(s);
            }
            ConvType::HexLower | ConvType::HexUpper => {
                let ValueData::Number(value) = value else {
                    return Err(self.report_error(EvalErrorKind::Other {
                        span: None,
                        message: format!(
                            "'x' / 'X' formatting requires a number, got {}",
                            EvalErrorValueType::from_value(&value).to_str(),
                        ),
                    }));
                };

                let s = self.render_hex(
                    value,
                    zp,
                    iprec,
                    code.cflags.blank,
                    code.cflags.plus,
                    code.cflags.alt,
                    code.ctype == ConvType::HexUpper,
                );
                self.string_stack.push(s);
            }
            ConvType::ExpLower | ConvType::ExpUpper => {
                let ValueData::Number(value) = value else {
                    return Err(self.report_error(EvalErrorKind::Other {
                        span: None,
                        message: format!(
                            "'e' / 'E' formatting requires a number, got {}",
                            EvalErrorValueType::from_value(&value).to_str(),
                        ),
                    }));
                };

                let s = render_float_exp(
                    value,
                    fpprec,
                    zp,
                    code.cflags.plus,
                    code.cflags.blank,
                    code.cflags.alt,
                    false,
                    code.ctype == ConvType::ExpUpper,
                );
                self.string_stack.push(s);
            }
            ConvType::FloatLower | ConvType::FloatUpper => {
                let ValueData::Number(value) = value else {
                    return Err(self.report_error(EvalErrorKind::Other {
                        span: None,
                        message: format!(
                            "'f' / 'F' formatting requires a number, got {}",
                            EvalErrorValueType::from_value(&value).to_str(),
                        ),
                    }));
                };

                let s = render_float_def(
                    value,
                    fpprec,
                    zp,
                    code.cflags.plus,
                    code.cflags.blank,
                    code.cflags.alt,
                    false,
                );
                self.string_stack.push(s);
            }
            ConvType::FloatGLower | ConvType::FloatGUpper => {
                let ValueData::Number(value) = value else {
                    return Err(self.report_error(EvalErrorKind::Other {
                        span: None,
                        message: format!(
                            "'g' / 'G' formatting requires a number, got {}",
                            EvalErrorValueType::from_value(&value).to_str(),
                        ),
                    }));
                };

                let exponent = if value == 0.0 {
                    0.0
                } else {
                    value.abs().log10().floor()
                };

                let s = if exponent < -4.0 || (exponent >= 0.0 && exponent >= (fpprec as f64)) {
                    render_float_exp(
                        value,
                        fpprec.max(1) - 1,
                        zp,
                        code.cflags.plus,
                        code.cflags.blank,
                        code.cflags.alt,
                        !code.cflags.alt,
                        code.ctype == ConvType::FloatGUpper,
                    )
                } else {
                    let value_abs = value.abs();
                    let digits_before_pt = if value_abs < 1.0 {
                        1
                    } else {
                        value_abs.trunc().to_string().len()
                    };

                    let fpprec = fpprec.saturating_sub(digits_before_pt);
                    render_float_def(
                        value,
                        fpprec,
                        zp,
                        code.cflags.plus,
                        code.cflags.blank,
                        code.cflags.alt,
                        !code.cflags.alt,
                    )
                };
                self.string_stack.push(s);
            }
            ConvType::Char => match value {
                ValueData::String(s) => {
                    if s.chars().count() != 1 {
                        return Err(self.report_error(EvalErrorKind::Other {
                            span: None,
                            message: format!(
                                "'c' formatting requires a string of length 1, got {}",
                                s.chars().count(),
                            ),
                        }));
                    }
                    self.string_stack.push((*s).into());
                }
                ValueData::Number(n) => {
                    let Some(chr) = float::try_to_u32(n).and_then(char::from_u32) else {
                        return Err(self.report_error(EvalErrorKind::Other {
                            span: None,
                            message: format!("{n} is not a valid unicode codepoint"),
                        }));
                    };
                    let mut utf8_buf = [0; 4];
                    let utf8 = chr.encode_utf8(&mut utf8_buf);
                    self.string_stack.push(utf8.into());
                }
                _ => {
                    return Err(self.report_error(EvalErrorKind::Other {
                        span: None,
                        message: format!(
                            "'c' formatting requires a string or a number, got {}",
                            EvalErrorValueType::from_value(&value).to_str(),
                        ),
                    }));
                }
            },
            ConvType::String => {
                if let ValueData::String(s) = value {
                    self.string_stack.push((*s).into());
                } else {
                    self.state_stack.push(State::ManifestJson {
                        format: ManifestJsonFormat::default_to_string(),
                        depth: 0,
                    });
                    self.value_stack.push(value);
                    self.string_stack.push(String::new());
                }
            }
            ConvType::Percent => unreachable!(),
        }

        Ok(())
    }

    fn render_int(
        &mut self,
        neg: bool,
        mag: f64,
        min_chars: usize,
        min_digits: usize,
        blank: bool,
        plus: bool,
        radix: u8,
        zero_prefix: &str,
    ) -> String {
        let mut digits_s = String::new();
        if mag == 0.0 {
            digits_s.push('0');
        } else {
            digits_s.push_str(zero_prefix);
            let radix_f = f64::from(radix);
            let mut mag = mag;
            while mag != 0.0 {
                let digit = (mag % radix_f) as u8;
                digits_s.insert(zero_prefix.len(), char::from(b'0' + digit));
                mag = (mag / radix_f).trunc();
            }
        }

        let mut result = String::new();
        if neg {
            result.push('-');
        } else if plus {
            result.push('+');
        } else if blank {
            result.push(' ');
        }

        let pad_len = min_digits
            .saturating_sub(digits_s.len())
            .max(min_chars.saturating_sub(result.len() + digits_s.len()));

        result.extend(std::iter::repeat('0').take(pad_len));
        result.push_str(&digits_s);
        result
    }

    fn render_hex(
        &mut self,
        num: f64,
        min_chars: usize,
        min_digits: usize,
        blank: bool,
        plus: bool,
        add_zerox: bool,
        capitals: bool,
    ) -> String {
        let numerals = if capitals {
            b"0123456789ABCDEF"
        } else {
            b"0123456789abcdef"
        };
        let num = num.trunc();
        let neg = num.is_sign_negative() && num != 0.0;
        let mag = num.abs();

        let mut digits_s = String::new();
        if mag == 0.0 {
            digits_s.push('0');
        } else {
            let radix_f = f64::from(16);
            let mut mag = mag;
            while mag != 0.0 {
                let digit = (mag % radix_f) as u8;
                digits_s.insert(0, char::from(numerals[usize::from(digit)]));
                mag = (mag / radix_f).trunc();
            }
        }

        let mut result = String::new();
        if neg {
            result.push('-');
        } else if plus {
            result.push('+');
        } else if blank {
            result.push(' ');
        }
        if add_zerox {
            result.push_str(if capitals { "0X" } else { "0x" });
        }

        let pad_len = min_digits
            .saturating_sub(digits_s.len())
            .max(min_chars.saturating_sub(result.len() + digits_s.len()));

        result.extend(std::iter::repeat('0').take(pad_len));
        result.push_str(&digits_s);
        result
    }
}

fn render_float_def(
    value: f64,
    prec: usize,
    zero_pad: usize,
    plus: bool,
    blank: bool,
    ensure_pt: bool,
    trim_zeros: bool,
) -> String {
    let value_abs = value.abs();
    let is_neg = value.is_sign_negative() && value != 0.0;

    let mut digits_str = format!("{value_abs:.prec$}");
    if prec == 0 && ensure_pt {
        digits_str.push('.');
    } else if prec != 0 && trim_zeros {
        let mut trimmed = digits_str.trim_end_matches('0');
        if !ensure_pt {
            trimmed = trimmed.strip_suffix('.').unwrap_or(trimmed);
        }
        digits_str.truncate(trimmed.len());
    };

    decorate_digits(&digits_str, is_neg, zero_pad, 0, plus, blank)
}

fn render_float_exp(
    value: f64,
    prec: usize,
    zero_pad: usize,
    plus: bool,
    blank: bool,
    ensure_pt: bool,
    trim_zeros: bool,
    uppercase: bool,
) -> String {
    let value_abs = value.abs();
    let is_neg = value.is_sign_negative() && value != 0.0;

    let digits_str = format!("{value_abs:.prec$e}");
    let e_pos = digits_str.bytes().position(|chr| chr == b'e').unwrap();
    let mut mant_str = &digits_str[..e_pos];
    if prec != 0 && trim_zeros {
        mant_str = mant_str.trim_end_matches('0');
        if !ensure_pt {
            mant_str = mant_str.strip_suffix('.').unwrap_or(mant_str);
        }
    };
    let exp_str = &digits_str[(e_pos + 1)..];
    let exp_int = exp_str.parse::<i32>().unwrap();
    let dot = if prec == 0 && ensure_pt { "." } else { "" };
    let e_chr = if uppercase { 'E' } else { 'e' };
    let digits_str = format!("{mant_str}{dot}{e_chr}{exp_int:+03}");

    decorate_digits(&digits_str, is_neg, zero_pad, 0, plus, blank)
}

fn decorate_digits(
    digits: &str,
    is_neg: bool,
    min_chars: usize,
    min_digits: usize,
    sign: bool,
    blank: bool,
) -> String {
    let mut s = String::new();
    if is_neg {
        s.push('-');
    } else if sign {
        s.push('+');
    } else if blank {
        s.push(' ');
    }

    let unpadded_len = s.len() + digits.len();
    let pad_len = min_digits
        .saturating_sub(digits.len())
        .max(min_chars.saturating_sub(unpadded_len));

    s.extend(std::iter::repeat('0').take(pad_len));
    s.push_str(digits);

    s
}
