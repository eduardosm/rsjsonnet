use crate::interner::InternedStr;
use crate::span::SpanId;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Token<'p, 'ast> {
    pub span: SpanId,
    pub kind: TokenKind<'p, 'ast>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TokenKind<'p, 'ast> {
    EndOfFile,
    Whitespace,
    Comment,

    Simple(STokenKind),
    OtherOp(&'ast str),
    Ident(InternedStr<'p>),
    Number(Number<'ast>),
    String(&'ast str),
    TextBlock(&'ast str),
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Number<'ast> {
    pub digits: &'ast str,
    pub exp: i64,
}
