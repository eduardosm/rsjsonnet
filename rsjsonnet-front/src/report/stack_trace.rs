use rsjsonnet_lang::program::EvalStackTraceItem;
use rsjsonnet_lang::span::SpanManager;

use super::message::{LabelKind, Message, MessageKind, MessageLabel};
use super::TextPartKind;
use crate::src_manager::SrcManager;

#[must_use]
pub(crate) fn render(
    trace: &[EvalStackTraceItem],
    span_mgr: &SpanManager,
    src_mgr: &SrcManager,
) -> Vec<(String, TextPartKind)> {
    let mut out = Vec::new();

    for trace_item in trace.iter().rev() {
        match *trace_item {
            EvalStackTraceItem::Expr { span } => {
                Message {
                    kind: MessageKind::Note,
                    message: "while evaluating this expression".into(),
                    labels: vec![MessageLabel {
                        span,
                        kind: LabelKind::Note,
                        text: String::new(),
                    }],
                }
                .render(span_mgr, src_mgr, &mut out);
            }
            EvalStackTraceItem::Call { span, ref name } => {
                Message {
                    kind: MessageKind::Note,
                    message: if let Some(name) = name {
                        format!("while evaluating call to `{name}`")
                    } else {
                        "while evaluating call to function".into()
                    },
                    labels: span
                        .map(|span| MessageLabel {
                            span,
                            kind: LabelKind::Note,
                            text: String::new(),
                        })
                        .into_iter()
                        .collect(),
                }
                .render(span_mgr, src_mgr, &mut out);
            }
            EvalStackTraceItem::Variable { span, ref name } => {
                Message {
                    kind: MessageKind::Note,
                    message: format!("while evaluating variable `{name}`"),
                    labels: vec![MessageLabel {
                        span,
                        kind: LabelKind::Note,
                        text: String::new(),
                    }],
                }
                .render(span_mgr, src_mgr, &mut out);
            }
            EvalStackTraceItem::ArrayItem { span, index } => {
                Message {
                    kind: MessageKind::Note,
                    message: format!("while evaluating array item {index}"),
                    labels: span
                        .map(|span| MessageLabel {
                            span,
                            kind: LabelKind::Note,
                            text: String::new(),
                        })
                        .into_iter()
                        .collect(),
                }
                .render(span_mgr, src_mgr, &mut out);
            }
            EvalStackTraceItem::ObjectField { span, ref name } => {
                Message {
                    kind: MessageKind::Note,
                    message: format!("while evaluating object field {name:?}"),
                    labels: span
                        .map(|span| MessageLabel {
                            span,
                            kind: LabelKind::Note,
                            text: String::new(),
                        })
                        .into_iter()
                        .collect(),
                }
                .render(span_mgr, src_mgr, &mut out);
            }
            EvalStackTraceItem::ManifestArrayItem { index } => {
                Message {
                    kind: MessageKind::Note,
                    message: format!("while manifesting array item {index}"),
                    labels: vec![],
                }
                .render(span_mgr, src_mgr, &mut out);
            }
            EvalStackTraceItem::ManifestObjectField { ref name } => {
                Message {
                    kind: MessageKind::Note,
                    message: format!("while manifesting object field {name:?}"),
                    labels: vec![],
                }
                .render(span_mgr, src_mgr, &mut out);
            }
            EvalStackTraceItem::Import { span } => {
                Message {
                    kind: MessageKind::Note,
                    message: "while evaluating import".into(),
                    labels: vec![MessageLabel {
                        span,
                        kind: LabelKind::Note,
                        text: String::new(),
                    }],
                }
                .render(span_mgr, src_mgr, &mut out);
            }
        }
    }

    out
}
