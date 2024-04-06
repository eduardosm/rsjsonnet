use rsjsonnet_lang::program::AnalyzeError;
use rsjsonnet_lang::span::SpanManager;

use super::message::{LabelKind, Message, MessageKind, MessageLabel};
use super::TextPartKind;
use crate::src_manager::SrcManager;

#[must_use]
pub(crate) fn render_error(
    error: &AnalyzeError,
    span_mgr: &SpanManager,
    src_mgr: &SrcManager,
) -> Vec<(String, TextPartKind)> {
    let mut out = Vec::new();

    match *error {
        AnalyzeError::UnknownVariable { span, ref name } => {
            Message {
                kind: MessageKind::Error,
                message: format!("unknown identifier `{name}`"),
                labels: vec![MessageLabel {
                    span,
                    kind: LabelKind::Error,
                    text: "unknown identifier".into(),
                }],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        AnalyzeError::SelfOutsideObject { self_span } => {
            Message {
                kind: MessageKind::Error,
                message: "`self` outside object".into(),
                labels: vec![MessageLabel {
                    span: self_span,
                    kind: LabelKind::Error,
                    text: "`self` outside object".into(),
                }],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        AnalyzeError::SuperOutsideObject { super_span } => {
            Message {
                kind: MessageKind::Error,
                message: "`super` outside object".into(),
                labels: vec![MessageLabel {
                    span: super_span,
                    kind: LabelKind::Error,
                    text: "`super` outside object".into(),
                }],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        AnalyzeError::DollarOutsideObject { dollar_span } => {
            Message {
                kind: MessageKind::Error,
                message: "`$` outside object".into(),
                labels: vec![MessageLabel {
                    span: dollar_span,
                    kind: LabelKind::Error,
                    text: "`$` outside object".into(),
                }],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        AnalyzeError::RepeatedLocalName {
            original_span,
            repeated_span,
            ref name,
        } => {
            Message {
                kind: MessageKind::Error,
                message: format!("repeated local name `{name}`"),
                labels: vec![
                    MessageLabel {
                        span: repeated_span,
                        kind: LabelKind::Error,
                        text: "repeated local name".into(),
                    },
                    MessageLabel {
                        span: original_span,
                        kind: LabelKind::Note,
                        text: "previously defined here".into(),
                    },
                ],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        AnalyzeError::RepeatedFieldName {
            original_span,
            repeated_span,
            ref name,
        } => {
            Message {
                kind: MessageKind::Error,
                message: format!("repeated field name {name:?}"),
                labels: vec![
                    MessageLabel {
                        span: repeated_span,
                        kind: LabelKind::Error,
                        text: "repeated field name".into(),
                    },
                    MessageLabel {
                        span: original_span,
                        kind: LabelKind::Note,
                        text: "previously defined here".into(),
                    },
                ],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        AnalyzeError::RepeatedParamName {
            original_span,
            repeated_span,
            ref name,
        } => {
            Message {
                kind: MessageKind::Error,
                message: format!("repeated parameter name `{name}`"),
                labels: vec![
                    MessageLabel {
                        span: repeated_span,
                        kind: LabelKind::Error,
                        text: "repeated parameter name".into(),
                    },
                    MessageLabel {
                        span: original_span,
                        kind: LabelKind::Note,
                        text: "previously defined here".into(),
                    },
                ],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        AnalyzeError::PositionalArgAfterNamed { arg_span } => {
            Message {
                kind: MessageKind::Error,
                message: "positional argument after named argument".into(),
                labels: vec![MessageLabel {
                    span: arg_span,
                    kind: LabelKind::Error,
                    text: "positional argument after named argument".into(),
                }],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        AnalyzeError::TextBlockAsImportPath { span } => {
            Message {
                kind: MessageKind::Error,
                message: "import paths cannot be text blocks".into(),
                labels: vec![MessageLabel {
                    span,
                    kind: LabelKind::Error,
                    text: "text blocks path".into(),
                }],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
        AnalyzeError::ComputedImportPath { span } => {
            Message {
                kind: MessageKind::Error,
                message: "import paths cannot be computed".into(),
                labels: vec![MessageLabel {
                    span,
                    kind: LabelKind::Error,
                    text: "computed path".into(),
                }],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
    }

    out.push(('\n'.into(), TextPartKind::Space));
    out
}
