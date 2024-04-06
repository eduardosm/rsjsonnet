use rsjsonnet_lang::lexer::LexError;
use rsjsonnet_lang::span::SpanManager;

use super::message::{LabelKind, Message, MessageKind, MessageLabel};
use super::TextPartKind;
use crate::src_manager::SrcManager;

#[must_use]
pub(crate) fn render_error(
    error: &LexError,
    span_mgr: &SpanManager,
    src_mgr: &SrcManager,
) -> Vec<(String, TextPartKind)> {
    let mut out = Vec::new();

    match *error {
        LexError::InvalidChar { span, chr } => {
            Message {
                kind: MessageKind::Error,
                message: format!("invalid character U+{:X}", u32::from(chr)),
                labels: vec![MessageLabel {
                    span,
                    kind: LabelKind::Error,
                    text: "invalid character".into(),
                }],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        LexError::InvalidUtf8 { span, seq: _ } => {
            Message {
                kind: MessageKind::Error,
                message: "invalid UTF-8 sequence".into(),
                labels: vec![MessageLabel {
                    span,
                    kind: LabelKind::Error,
                    text: "invalid UTF-8 sequence".into(),
                }],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        LexError::UnfinishedMultilineComment { span } => {
            Message {
                kind: MessageKind::Error,
                message: "unfinished block comment".into(),
                labels: vec![MessageLabel {
                    span,
                    kind: LabelKind::Error,
                    text: "unfinished block comment".into(),
                }],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        LexError::LeadingZeroInNumber { span } => {
            Message {
                kind: MessageKind::Error,
                message: "number has leading zero".into(),
                labels: vec![MessageLabel {
                    span,
                    kind: LabelKind::Error,
                    text: "number has leading zero".into(),
                }],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        LexError::MissingFracDigits { span } => {
            Message {
                kind: MessageKind::Error,
                message: "missing digits in fractional part of number".into(),
                labels: vec![MessageLabel {
                    span,
                    kind: LabelKind::Error,
                    text: "missing fractional digits".into(),
                }],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        LexError::MissingExpDigits { span } => {
            Message {
                kind: MessageKind::Error,
                message: "missing digits in exponent part of number".into(),
                labels: vec![MessageLabel {
                    span,
                    kind: LabelKind::Error,
                    text: "missing exponent digits".into(),
                }],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        LexError::ExpOverflow { span } => {
            Message {
                kind: MessageKind::Error,
                message: "number exponent overflow".into(),
                labels: vec![MessageLabel {
                    span,
                    kind: LabelKind::Error,
                    text: "exponent overflow".into(),
                }],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        LexError::InvalidEscapeInString { span, chr: _ } => {
            Message {
                kind: MessageKind::Error,
                message: "invalid escape character in string".into(),
                labels: vec![MessageLabel {
                    span,
                    kind: LabelKind::Error,
                    text: "invalid escape character".into(),
                }],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        LexError::IncompleteUnicodeEscape { span } => {
            Message {
                kind: MessageKind::Error,
                message: "incomplete unicode escape sequence in string".into(),
                labels: vec![MessageLabel {
                    span,
                    kind: LabelKind::Error,
                    text: "incomplete unicode escape sequence".into(),
                }],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        LexError::InvalidUtf16EscapeSequence { span, cu1, cu2 } => {
            let main_text = if let Some(cu2) = cu2 {
                format!("invalid sequence (\\u{cu1:04X}\\u{cu2:04X}) in UTF-16 escape sequence",)
            } else {
                format!("invalid sequence (\\u{cu1:04X}) in UTF-16 escape sequence",)
            };
            Message {
                kind: MessageKind::Error,
                message: main_text,
                labels: vec![MessageLabel {
                    span,
                    kind: LabelKind::Error,
                    text: "invalid UTF-16 escape sequence".into(),
                }],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        LexError::UnfinishedString { span } => {
            Message {
                kind: MessageKind::Error,
                message: "unfinished string".into(),
                labels: vec![MessageLabel {
                    span,
                    kind: LabelKind::Error,
                    text: "unfinished string".into(),
                }],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        LexError::MissingLineBreakAfterTextBlockStart { span } => {
            Message {
                kind: MessageKind::Error,
                message: "missing line break after `|||` at the beginning of a text block".into(),
                labels: vec![MessageLabel {
                    span,
                    kind: LabelKind::Error,
                    text: "missing line break".into(),
                }],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        LexError::MissingWhitespaceTextBlockStart { span } => {
            Message {
                kind: MessageKind::Error,
                message: "missing whitespace at the beginning of the first line of a text block"
                    .into(),
                labels: vec![MessageLabel {
                    span,
                    kind: LabelKind::Error,
                    text: "missing whitespace".into(),
                }],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        LexError::InvalidTextBlockTermination { span } => {
            Message {
                kind: MessageKind::Error,
                message: "invalid text block termination".into(),
                labels: vec![MessageLabel {
                    span,
                    kind: LabelKind::Error,
                    text: "invalid termination".into(),
                }],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
    }

    out.push(('\n'.into(), TextPartKind::Space));
    out
}
