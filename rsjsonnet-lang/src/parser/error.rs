use std::collections::BTreeSet;

use crate::span::SpanId;
use crate::token::{STokenKind, TokenKind};

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExpectedToken {
    EndOfFile,
    Simple(STokenKind),
    Ident,
    Number,
    String,
    TextBlock,
    Expr,
    BinaryOp,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ActualToken {
    EndOfFile,
    Simple(STokenKind),
    OtherOp(String),
    Ident(String),
    Number,
    String,
    TextBlock,
}

impl ActualToken {
    pub(super) fn from_token_kind(kind: &TokenKind<'_, '_>) -> Self {
        match *kind {
            TokenKind::EndOfFile => Self::EndOfFile,
            TokenKind::Whitespace => unreachable!(),
            TokenKind::Comment => unreachable!(),
            TokenKind::Simple(kind) => Self::Simple(kind),
            TokenKind::OtherOp(op) => Self::OtherOp(op.into()),
            TokenKind::Ident(ident) => Self::Ident(ident.value().into()),
            TokenKind::Number(_) => Self::Number,
            TokenKind::String(_) => Self::String,
            TokenKind::TextBlock(_) => Self::TextBlock,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseError {
    Expected {
        span: SpanId,
        expected: BTreeSet<ExpectedToken>,
        instead: ActualToken,
    },
}
