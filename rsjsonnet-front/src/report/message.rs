use std::collections::HashMap;

use rsjsonnet_lang::span::{SpanId, SpanManager};

use super::TextPartKind;
use crate::src_manager::SrcManager;

pub(super) struct Message {
    pub(super) kind: MessageKind,
    pub(super) message: String,
    pub(super) labels: Vec<MessageLabel>,
}

pub(super) enum MessageKind {
    Note,
    Error,
}

pub(super) struct MessageLabel {
    pub(super) span: SpanId,
    pub(super) kind: LabelKind,
    pub(super) text: String,
}

#[derive(Copy, Clone)]
pub(super) enum LabelKind {
    Note,
    Error,
}

impl Message {
    pub(super) fn render(
        &self,
        span_mgr: &SpanManager,
        src_mgr: &SrcManager,
        out: &mut Vec<(String, TextPartKind)>,
    ) {
        match self.kind {
            MessageKind::Note => {
                put_note_header(&self.message, out);
            }
            MessageKind::Error => {
                put_error_header(&self.message, out);
            }
        }
        put_spans_and_labels(&self.labels, span_mgr, src_mgr, out);
    }
}

pub(super) fn put_note_header<T: std::fmt::Display>(msg: T, out: &mut Vec<(String, TextPartKind)>) {
    out.push(("note".into(), TextPartKind::NoteLabel));
    out.push((": ".into(), TextPartKind::MainMessage));
    out.push((msg.to_string(), TextPartKind::MainMessage));
    out.push(("\n".into(), TextPartKind::Space));
}

pub(super) fn put_error_header<T: std::fmt::Display>(
    msg: T,
    out: &mut Vec<(String, TextPartKind)>,
) {
    out.push(("error".into(), TextPartKind::ErrorLabel));
    out.push((": ".into(), TextPartKind::MainMessage));
    out.push((msg.to_string(), TextPartKind::MainMessage));
    out.push(("\n".into(), TextPartKind::Space));
}

fn put_spans_and_labels(
    labels: &[MessageLabel],
    span_mgr: &SpanManager,
    src_mgr: &SrcManager,
    out: &mut Vec<(String, TextPartKind)>,
) {
    struct SourceData<'a> {
        annots: sourceannot::Annotations<'a, TextPartKind>,
        first_span_line: usize,
        first_span_col: usize,
    }

    let mut by_src = HashMap::new();
    let mut src_order = Vec::new();

    let main_style = get_main_style();
    let margin_line = main_style.margin.unwrap().line_char;

    for label in labels.iter() {
        let (span_ctx, span_start, span_end) = span_mgr.get_span(label.span);
        let rsjsonnet_lang::span::SpanContext::Source(src_id) = *span_mgr.get_context(span_ctx);
        let snippet = src_mgr.get_file_snippet(src_id);
        let (line, col) = snippet.get_line_col(span_start);

        by_src
            .entry(src_id)
            .or_insert_with(|| {
                src_order.push(src_id);
                SourceData {
                    annots: sourceannot::Annotations::new(snippet, main_style),
                    first_span_line: line,
                    first_span_col: col,
                }
            })
            .annots
            .add_annotation(
                span_start..span_end,
                match label.kind {
                    LabelKind::Error => get_error_annot_style(),
                    LabelKind::Note => get_note_annot_style(),
                },
                vec![(
                    label.text.clone(),
                    match label.kind {
                        LabelKind::Error => TextPartKind::ErrorLabel,
                        LabelKind::Note => TextPartKind::NoteLabel,
                    },
                )],
            );
    }

    let max_line_no_width = by_src
        .values()
        .map(|data| data.annots.max_line_no_width())
        .max()
        .unwrap_or(0);

    for &src_id in src_order.iter() {
        let data = &by_src[&src_id];
        let annot_parts = data.annots.render(max_line_no_width, 0, 0);

        let repr_path = src_mgr.get_file_repr_path(src_id);
        let path_ex = format!(
            "{repr_path}:{}:{}",
            data.first_span_line + 1,
            data.first_span_col + 1,
        );

        out.push((" ".repeat(max_line_no_width), TextPartKind::Space));
        out.push(("-->".into(), TextPartKind::Margin));
        out.push((" ".into(), TextPartKind::Space));
        out.push((path_ex, TextPartKind::Path));
        out.push(("\n".into(), TextPartKind::Space));
        out.push((" ".repeat(max_line_no_width + 1), TextPartKind::Space));
        out.push((margin_line.into(), TextPartKind::Margin));
        out.push(("\n".into(), TextPartKind::Space));

        out.extend(annot_parts);
    }
}

fn get_main_style() -> sourceannot::MainStyle<TextPartKind> {
    sourceannot::MainStyle {
        margin: Some(sourceannot::MarginStyle {
            line_char: '|',
            dot_char: ':',
            meta: TextPartKind::Margin,
        }),
        horizontal_char: '_',
        vertical_char: '|',
        top_vertical_char: '/',
        top_corner_char: ' ',
        bottom_corner_char: '|',
        spaces_meta: TextPartKind::Space,
        text_normal_meta: TextPartKind::TextNormal,
        text_alt_meta: TextPartKind::TextAlt,
    }
}

fn get_error_annot_style() -> sourceannot::AnnotStyle<TextPartKind> {
    sourceannot::AnnotStyle {
        caret: '^',
        text_normal_meta: TextPartKind::ErrorTextNormal,
        text_alt_meta: TextPartKind::ErrorTextAlt,
        line_meta: TextPartKind::ErrorLabel,
    }
}

fn get_note_annot_style() -> sourceannot::AnnotStyle<TextPartKind> {
    sourceannot::AnnotStyle {
        caret: '-',
        text_normal_meta: TextPartKind::TextNormal,
        text_alt_meta: TextPartKind::TextAlt,
        line_meta: TextPartKind::NoteLabel,
    }
}
