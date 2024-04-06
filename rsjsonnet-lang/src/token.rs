use crate::interner::InternedStr;
use crate::span::SpanId;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Token {
    pub span: SpanId,
    pub kind: TokenKind,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TokenKind {
    EndOfFile,
    Whitespace,
    Comment,

    Simple(STokenKind),
    OtherOp(Box<str>),
    Ident(InternedStr),
    Number(Number),
    String(Box<str>),
    TextBlock(Box<str>),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum STokenKind {
    // Keywords
    Assert,
    Else,
    Error,
    False,
    For,
    Function,
    If,
    Import,
    Importstr,
    Importbin,
    In,
    Local,
    Null,
    Tailstrict,
    Then,
    Self_,
    Super,
    True,

    // Symbols
    Exclam,
    ExclamEq,
    Dollar,
    Percent,
    Amp,
    AmpAmp,
    LeftParen,
    RightParen,
    Asterisk,
    Plus,
    PlusColon,
    PlusColonColon,
    PlusColonColonColon,
    Comma,
    Minus,
    Dot,
    Slash,
    Colon,
    ColonColon,
    ColonColonColon,
    Semicolon,
    Lt,
    LtLt,
    LtEq,
    Eq,
    EqEq,
    Gt,
    GtEq,
    GtGt,
    LeftBracket,
    RightBracket,
    Hat,
    LeftBrace,
    Pipe,
    PipePipe,
    RightBrace,
    Tilde,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Number {
    pub digits: Box<str>,
    pub exp: i64,
}
