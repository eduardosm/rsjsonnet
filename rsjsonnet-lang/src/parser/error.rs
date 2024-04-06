use std::collections::BTreeSet;

use crate::span::SpanId;
use crate::token::{STokenKind, TokenKind};

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExpectedThing {
    EndOfFile,
    Simple(STokenKind),
    Ident,
    Number,
    String,
    TextBlock,
    Expr,
    BinaryOp,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseError {
    Expected {
        span: SpanId,
        expected: BTreeSet<ExpectedThing>,
        instead: TokenKind,
    },
}
