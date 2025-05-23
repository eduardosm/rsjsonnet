use rsjsonnet_lang::parser::{ActualToken, ExpectedToken, ParseError};
use rsjsonnet_lang::span::SpanManager;
use rsjsonnet_lang::token::STokenKind;

use super::message::{LabelKind, Message, MessageKind, MessageLabel};
use super::TextPartKind;
use crate::src_manager::SrcManager;

#[must_use]
pub(crate) fn render_error(
    error: &ParseError,
    span_mgr: &SpanManager,
    src_mgr: &SrcManager,
) -> Vec<(String, TextPartKind)> {
    let mut out = Vec::new();

    match *error {
        ParseError::Expected {
            span,
            ref expected,
            ref instead,
        } => {
            let mut expected_str = String::new();
            for (i, expected_thing) in expected.iter().enumerate() {
                if i != 0 {
                    if i == expected.len() - 1 {
                        expected_str.push_str(" or ");
                    } else {
                        expected_str.push_str(", ");
                    }
                }
                expected_str.push_str(&expected_to_string(*expected_thing));
            }
            Message {
                kind: MessageKind::Error,
                message: format!(
                    "expected {expected_str} instead of {}",
                    actual_to_string(instead),
                ),
                labels: vec![MessageLabel {
                    span,
                    kind: LabelKind::Error,
                    text: if *instead == ActualToken::EndOfFile {
                        "unexpected end-of-file".into()
                    } else {
                        "unexpected token".into()
                    },
                }],
            }
            .render(span_mgr, src_mgr, &mut out);
        }
    }

    out.push(('\n'.into(), TextPartKind::Space));
    out
}

fn expected_to_string(token: ExpectedToken) -> String {
    match token {
        ExpectedToken::EndOfFile => "end-of-file".into(),
        ExpectedToken::Simple(kind) => format!("`{}`", simple_token_to_str(kind)),
        ExpectedToken::Ident => "identifier".into(),
        ExpectedToken::Number => "number".into(),
        ExpectedToken::String => "string".into(),
        ExpectedToken::TextBlock => "text block".into(),
        ExpectedToken::Expr => "expression".into(),
        ExpectedToken::BinaryOp => "binary operator".into(),
    }
}

fn actual_to_string(token: &ActualToken) -> String {
    match token {
        ActualToken::EndOfFile => "end-of-file".into(),
        ActualToken::Simple(kind) => format!("`{}`", simple_token_to_str(*kind)),
        ActualToken::OtherOp(op) => format!("`{op}`"),
        ActualToken::Ident(_) => "identifier".into(),
        ActualToken::Number => "number".into(),
        ActualToken::String => "string".into(),
        ActualToken::TextBlock => "text block".into(),
    }
}

fn simple_token_to_str(kind: STokenKind) -> &'static str {
    match kind {
        STokenKind::Assert => "assert",
        STokenKind::Else => "else",
        STokenKind::Error => "error",
        STokenKind::False => "false",
        STokenKind::For => "for",
        STokenKind::Function => "function",
        STokenKind::If => "if",
        STokenKind::Import => "import",
        STokenKind::Importstr => "importstr",
        STokenKind::Importbin => "importbin",
        STokenKind::In => "in",
        STokenKind::Local => "local",
        STokenKind::Null => "null",
        STokenKind::Tailstrict => "tailstrict",
        STokenKind::Then => "then",
        STokenKind::Self_ => "self",
        STokenKind::Super => "super",
        STokenKind::True => "true",
        STokenKind::Exclam => "!",
        STokenKind::ExclamEq => "!=",
        STokenKind::Dollar => "$",
        STokenKind::Percent => "%",
        STokenKind::Amp => "&",
        STokenKind::AmpAmp => "&&",
        STokenKind::LeftParen => "(",
        STokenKind::RightParen => ")",
        STokenKind::Asterisk => "*",
        STokenKind::Plus => "+",
        STokenKind::PlusColon => "+:",
        STokenKind::PlusColonColon => "+::",
        STokenKind::PlusColonColonColon => "+:::",
        STokenKind::Comma => ",",
        STokenKind::Minus => "-",
        STokenKind::Dot => ".",
        STokenKind::Slash => "/",
        STokenKind::Colon => ":",
        STokenKind::ColonColon => "::",
        STokenKind::ColonColonColon => ":::",
        STokenKind::Semicolon => ";",
        STokenKind::Lt => "<",
        STokenKind::LtLt => "<<",
        STokenKind::LtEq => "<=",
        STokenKind::Eq => "=",
        STokenKind::EqEq => "==",
        STokenKind::Gt => ">",
        STokenKind::GtEq => ">=",
        STokenKind::GtGt => ">>",
        STokenKind::LeftBracket => "[",
        STokenKind::RightBracket => "]",
        STokenKind::Hat => "^",
        STokenKind::LeftBrace => "{",
        STokenKind::Pipe => "|",
        STokenKind::PipePipe => "||",
        STokenKind::RightBrace => "}",
        STokenKind::Tilde => "~",
    }
}
