pub(crate) mod analyze;
pub(crate) mod eval;
pub(crate) mod lexer;
mod message;
pub(crate) mod parser;
pub(crate) mod stack_trace;

use crate::print::TextPartKind;

#[must_use]
pub(crate) fn render_note_message<T: std::fmt::Display>(msg: T) -> Vec<(String, TextPartKind)> {
    let mut out = Vec::new();
    message::put_note_header(msg, &mut out);
    out
}

#[must_use]
pub(crate) fn render_error_message<T: std::fmt::Display>(msg: T) -> Vec<(String, TextPartKind)> {
    let mut out = Vec::new();
    message::put_error_header(msg, &mut out);
    out
}
