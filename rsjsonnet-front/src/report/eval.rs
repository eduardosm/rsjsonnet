use rsjsonnet_lang::ast;
use rsjsonnet_lang::program::{EvalErrorKind, EvalErrorValueType};
use rsjsonnet_lang::span::SpanManager;

use super::message::{LabelKind, Message, MessageKind, MessageLabel};
use super::TextPartKind;
use crate::src_manager::SrcManager;

#[must_use]
pub(crate) fn render_error_kind(
    error_kind: &EvalErrorKind,
    span_mgr: &SpanManager,
    src_mgr: &SrcManager,
) -> Vec<(String, TextPartKind)> {
    let mut out = Vec::new();

    match *error_kind {
        EvalErrorKind::StackOverflow => {
            Message {
                kind: MessageKind::Error,
                message: "stack overflow".into(),
                labels: vec![],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        EvalErrorKind::InfiniteRecursion => {
            Message {
                kind: MessageKind::Error,
                message: "infinite recursion".into(),
                labels: vec![],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        EvalErrorKind::InvalidIndexedType { span, got_type } => {
            Message {
                kind: MessageKind::Error,
                message: format!("cannot index value of type {}", type_to_string(got_type)),
                labels: vec![MessageLabel {
                    kind: LabelKind::Error,
                    span,
                    text: String::new(),
                }],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        EvalErrorKind::StringIndexIsNotNumber { span, got_type } => {
            Message {
                kind: MessageKind::Error,
                message: format!(
                    "cannot index string with value of type {}",
                    type_to_string(got_type),
                ),
                labels: vec![MessageLabel {
                    kind: LabelKind::Error,
                    span,
                    text: String::new(),
                }],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        EvalErrorKind::ArrayIndexIsNotNumber { span, got_type } => {
            Message {
                kind: MessageKind::Error,
                message: format!(
                    "cannot index array with value of type {}",
                    type_to_string(got_type),
                ),
                labels: vec![MessageLabel {
                    kind: LabelKind::Error,
                    span,
                    text: String::new(),
                }],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        EvalErrorKind::NumericIndexIsNotValid { span, ref index } => {
            Message {
                kind: MessageKind::Error,
                message: format!("{index} is not a valid numeric index value"),
                labels: vec![MessageLabel {
                    kind: LabelKind::Error,
                    span,
                    text: String::new(),
                }],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        EvalErrorKind::NumericIndexOutOfRange {
            span,
            ref index,
            length,
        } => {
            Message {
                kind: MessageKind::Error,
                message: format!("index {index} is out of range for indexable of length {length}"),
                labels: vec![MessageLabel {
                    kind: LabelKind::Error,
                    span,
                    text: String::new(),
                }],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        EvalErrorKind::ObjectIndexIsNotString { span, got_type } => {
            Message {
                kind: MessageKind::Error,
                message: format!(
                    "cannot index object with value of type {}",
                    type_to_string(got_type),
                ),
                labels: vec![MessageLabel {
                    kind: LabelKind::Error,
                    span,
                    text: String::new(),
                }],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        EvalErrorKind::RepeatedFieldName { span, ref name } => {
            Message {
                kind: MessageKind::Error,
                message: format!("repeated field name {name:?}"),
                labels: vec![MessageLabel {
                    kind: LabelKind::Error,
                    span,
                    text: String::new(),
                }],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        EvalErrorKind::FieldNameIsNotString { span, got_type } => {
            Message {
                kind: MessageKind::Error,
                message: format!(
                    "field name must be a string, got {}",
                    type_to_string(got_type),
                ),
                labels: vec![MessageLabel {
                    kind: LabelKind::Error,
                    span,
                    text: String::new(),
                }],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        EvalErrorKind::UnknownObjectField {
            span,
            ref field_name,
        } => {
            Message {
                kind: MessageKind::Error,
                message: format!("unknown field {field_name:?}"),
                labels: vec![MessageLabel {
                    kind: LabelKind::Error,
                    span,
                    text: String::new(),
                }],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        EvalErrorKind::FieldOfNonObject { span } => {
            Message {
                kind: MessageKind::Error,
                message: "attempted to access field of non-object value".into(),
                labels: vec![MessageLabel {
                    kind: LabelKind::Error,
                    span,
                    text: String::new(),
                }],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        EvalErrorKind::SuperWithoutSuperObject { span } => {
            Message {
                kind: MessageKind::Error,
                message: "`super` used without super object".into(),
                labels: vec![MessageLabel {
                    kind: LabelKind::Error,
                    span,
                    text: String::new(),
                }],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        EvalErrorKind::ForSpecValueIsNotArray { span, got_type } => {
            Message {
                kind: MessageKind::Error,
                message: format!(
                    "expected array for comprenhension, got {}",
                    type_to_string(got_type),
                ),
                labels: vec![MessageLabel {
                    kind: LabelKind::Error,
                    span,
                    text: String::new(),
                }],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        EvalErrorKind::CondIsNotBool { span, got_type } => {
            Message {
                kind: MessageKind::Error,
                message: format!(
                    "expected boolean for condition, got {}",
                    type_to_string(got_type),
                ),
                labels: vec![MessageLabel {
                    kind: LabelKind::Error,
                    span,
                    text: String::new(),
                }],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        EvalErrorKind::CalleeIsNotFunction { span, got_type } => {
            Message {
                kind: MessageKind::Error,
                message: format!(
                    "attempted to call a {} instead of a function",
                    type_to_string(got_type),
                ),
                labels: span
                    .map(|span| MessageLabel {
                        kind: LabelKind::Error,
                        span,
                        text: String::new(),
                    })
                    .into_iter()
                    .collect(),
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        EvalErrorKind::TooManyCallArgs { span, num_params } => {
            Message {
                kind: MessageKind::Error,
                message: format!("too many arguments, function has {num_params} parameter(s)"),
                labels: span
                    .map(|span| MessageLabel {
                        kind: LabelKind::Error,
                        span,
                        text: String::new(),
                    })
                    .into_iter()
                    .collect(),
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        EvalErrorKind::UnknownCallParam {
            span,
            ref param_name,
        } => {
            Message {
                kind: MessageKind::Error,
                message: format!("unknown parameter `{param_name}`"),
                labels: span
                    .map(|span| MessageLabel {
                        kind: LabelKind::Error,
                        span,
                        text: String::new(),
                    })
                    .into_iter()
                    .collect(),
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        EvalErrorKind::RepeatedCallParam {
            span,
            ref param_name,
        } => {
            Message {
                kind: MessageKind::Error,
                message: format!("repeated parameter `{param_name}`"),
                labels: span
                    .map(|span| MessageLabel {
                        kind: LabelKind::Error,
                        span,
                        text: String::new(),
                    })
                    .into_iter()
                    .collect(),
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        EvalErrorKind::CallParamNotBound {
            span,
            ref param_name,
        } => {
            Message {
                kind: MessageKind::Error,
                message: format!("parameter `{param_name}` is not bound"),
                labels: span
                    .map(|span| MessageLabel {
                        kind: LabelKind::Error,
                        span,
                        text: String::new(),
                    })
                    .into_iter()
                    .collect(),
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        EvalErrorKind::NativeCallFailed => {
            Message {
                kind: MessageKind::Error,
                message: "native function call failed".into(),
                labels: vec![],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        EvalErrorKind::InvalidUnaryOpType { span, op, rhs_type } => {
            Message {
                kind: MessageKind::Error,
                message: format!(
                    "unary operator `{}` cannot be applied to value of type {}",
                    unary_op_to_string(op),
                    type_to_string(rhs_type),
                ),
                labels: vec![MessageLabel {
                    kind: LabelKind::Error,
                    span,
                    text: String::new(),
                }],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        EvalErrorKind::InvalidBinaryOpTypes {
            span,
            op,
            lhs_type,
            rhs_type,
        } => {
            Message {
                kind: MessageKind::Error,
                message: format!(
                    "binary operator `{}` cannot be applied to values of types {} and {}",
                    binary_op_to_string(op),
                    type_to_string(lhs_type),
                    type_to_string(rhs_type),
                ),
                labels: span
                    .map(|span| MessageLabel {
                        kind: LabelKind::Error,
                        span,
                        text: String::new(),
                    })
                    .into_iter()
                    .collect(),
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        EvalErrorKind::NumberOverflow { span } => {
            Message {
                kind: MessageKind::Error,
                message: "numeric overflow".into(),
                labels: span
                    .map(|span| MessageLabel {
                        kind: LabelKind::Error,
                        span,
                        text: String::new(),
                    })
                    .into_iter()
                    .collect(),
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        EvalErrorKind::NumberNan { span } => {
            Message {
                kind: MessageKind::Error,
                message: "numeric result is NaN".into(),
                labels: span
                    .map(|span| MessageLabel {
                        kind: LabelKind::Error,
                        span,
                        text: String::new(),
                    })
                    .into_iter()
                    .collect(),
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        EvalErrorKind::DivByZero { span } => {
            Message {
                kind: MessageKind::Error,
                message: "division by zero".into(),
                labels: span
                    .map(|span| MessageLabel {
                        kind: LabelKind::Error,
                        span,
                        text: String::new(),
                    })
                    .into_iter()
                    .collect(),
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        EvalErrorKind::ShiftByNegative { span } => {
            Message {
                kind: MessageKind::Error,
                message: "shift by negative amount".into(),
                labels: span
                    .map(|span| MessageLabel {
                        kind: LabelKind::Error,
                        span,
                        text: String::new(),
                    })
                    .into_iter()
                    .collect(),
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        EvalErrorKind::InvalidBuiltInFuncArgType {
            ref func_name,
            arg_index,
            ref expected_types,
            got_type,
        } => {
            let mut expected_str = String::new();
            for (i, expected_type) in expected_types.iter().enumerate() {
                if i != 0 {
                    if i == expected_types.len() - 1 {
                        expected_str.push_str(" or ");
                    } else {
                        expected_str.push_str(", ");
                    }
                }
                expected_str.push_str(type_to_string(*expected_type));
            }
            Message {
                kind: MessageKind::Error,
                message: format!(
                    "argument {arg_index} of function `{func_name}` is expected to be {expected_str}, got {}",
                    type_to_string(got_type),
                ),
                labels: vec![],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        EvalErrorKind::AssertFailed { span, ref message } => {
            Message {
                kind: MessageKind::Error,
                message: if let Some(message) = message {
                    if message.chars().all(|chr| !chr.is_control()) {
                        format!("assertion failed: {message}")
                    } else {
                        format!("assertion failed: {message:?}")
                    }
                } else {
                    "assertion failed".into()
                },
                labels: vec![MessageLabel {
                    kind: LabelKind::Error,
                    span,
                    text: String::new(),
                }],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        EvalErrorKind::AssertEqualFailed { ref lhs, ref rhs } => {
            Message {
                kind: MessageKind::Error,
                message: format!("assertion failed: {lhs} != {rhs}"),
                labels: vec![],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        EvalErrorKind::ExplicitError { span, ref message } => {
            Message {
                kind: MessageKind::Error,
                message: if message.chars().all(|chr| !chr.is_control()) {
                    format!("explicit error: {message}")
                } else {
                    format!("explicit error: {message:?}")
                },
                labels: vec![MessageLabel {
                    kind: LabelKind::Error,
                    span,
                    text: String::new(),
                }],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        EvalErrorKind::ImportFailed { span, ref path } => {
            Message {
                kind: MessageKind::Error,
                message: format!("failed to import {path:?}"),
                labels: vec![MessageLabel {
                    kind: LabelKind::Error,
                    span,
                    text: String::new(),
                }],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        EvalErrorKind::UnknownExtVar { ref name } => {
            Message {
                kind: MessageKind::Error,
                message: format!("unknown external variable {name:?}"),
                labels: vec![],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        EvalErrorKind::ManifestFunction => {
            Message {
                kind: MessageKind::Error,
                message: "functions cannot be manifested".into(),
                labels: vec![],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        EvalErrorKind::CompareNullInequality => {
            Message {
                kind: MessageKind::Error,
                message: "nulls cannot be compared for inequality".into(),
                labels: vec![],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        EvalErrorKind::CompareBooleanInequality => {
            Message {
                kind: MessageKind::Error,
                message: "booleans cannot be compared for inequality".into(),
                labels: vec![],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        EvalErrorKind::CompareObjectInequality => {
            Message {
                kind: MessageKind::Error,
                message: "objects cannot be compared for inequality".into(),
                labels: vec![],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        EvalErrorKind::CompareFunctions => {
            Message {
                kind: MessageKind::Error,
                message: "functions cannot be compared".into(),
                labels: vec![],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        EvalErrorKind::CompareDifferentTypesInequality { lhs_type, rhs_type } => {
            Message {
                kind: MessageKind::Error,
                message: format!(
                    "cannot compare values of different types ({} and {}) for inequality",
                    type_to_string(lhs_type),
                    type_to_string(rhs_type),
                ),
                labels: vec![],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        EvalErrorKind::PrimitiveEqualsNonPrimitive { got_type } => {
            Message {
                kind: MessageKind::Error,
                message: format!(
                    "std.primitiveEquals cannot compare values of type {}",
                    type_to_string(got_type),
                ),
                labels: vec![],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        EvalErrorKind::Other { span, ref message } => {
            Message {
                kind: MessageKind::Error,
                message: message.clone(),
                labels: span
                    .map(|span| MessageLabel {
                        kind: LabelKind::Error,
                        span,
                        text: String::new(),
                    })
                    .into_iter()
                    .collect(),
            }
            .render(span_mgr, src_mgr, &mut out);
        }
    }

    out
}

fn type_to_string(type_: EvalErrorValueType) -> &'static str {
    match type_ {
        EvalErrorValueType::Null => "null",
        EvalErrorValueType::Bool => "boolean",
        EvalErrorValueType::Number => "number",
        EvalErrorValueType::String => "string",
        EvalErrorValueType::Array => "array",
        EvalErrorValueType::Object => "object",
        EvalErrorValueType::Function => "function",
    }
}

fn unary_op_to_string(op: ast::UnaryOp) -> &'static str {
    match op {
        ast::UnaryOp::Minus => "-",
        ast::UnaryOp::Plus => "+",
        ast::UnaryOp::BitwiseNot => "~",
        ast::UnaryOp::LogicNot => "!",
    }
}

fn binary_op_to_string(op: ast::BinaryOp) -> &'static str {
    match op {
        ast::BinaryOp::Add => "+",
        ast::BinaryOp::Sub => "-",
        ast::BinaryOp::Mul => "*",
        ast::BinaryOp::Div => "/",
        ast::BinaryOp::Rem => "%",
        ast::BinaryOp::Shl => "<<",
        ast::BinaryOp::Shr => ">>",
        ast::BinaryOp::Lt => "<",
        ast::BinaryOp::Le => "<=",
        ast::BinaryOp::Gt => ">",
        ast::BinaryOp::Ge => ">=",
        ast::BinaryOp::Eq => "==",
        ast::BinaryOp::Ne => "!=",
        ast::BinaryOp::In => "in",
        ast::BinaryOp::BitwiseAnd => "&",
        ast::BinaryOp::BitwiseOr => "|",
        ast::BinaryOp::BitwiseXor => "^",
        ast::BinaryOp::LogicAnd => "&&",
        ast::BinaryOp::LogicOr => "||",
    }
}
