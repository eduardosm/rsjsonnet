#![warn(
    rust_2018_idioms,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unused_qualifications
)]
#![forbid(unsafe_code)]

use rsjsonnet_lang::interner::{InternedStr, StrInterner};
use rsjsonnet_lang::lexer::{LexError, Lexer};
use rsjsonnet_lang::span::{SpanContextId, SpanId, SpanManager};
use rsjsonnet_lang::token::{Number, STokenKind, Token, TokenKind};

#[must_use]
struct LexerTest<'a, F: FnOnce(&mut Session, Option<LexError>, &[Token])> {
    input: &'a [u8],
    check: F,
}

struct Session {
    str_interner: StrInterner,
    span_mgr: SpanManager,
    span_ctx: SpanContextId,
}

impl<F: FnOnce(&mut Session, Option<LexError>, &[Token])> LexerTest<'_, F> {
    fn run(self) {
        let str_interner = StrInterner::new();
        let mut span_mgr = SpanManager::new();
        let (span_ctx, _) = span_mgr.insert_source_context(self.input.len());

        let mut session = Session {
            str_interner,
            span_mgr,
            span_ctx,
        };

        let mut lexer = Lexer::new(
            &session.str_interner,
            &mut session.span_mgr,
            span_ctx,
            self.input,
        );
        let mut tokens = Vec::new();
        let mut error = None;
        loop {
            match lexer.next_token() {
                Ok(token) => {
                    let is_eof = token.kind == TokenKind::EndOfFile;
                    tokens.push(token);
                    if is_eof {
                        break;
                    }
                }
                Err(e) => {
                    error = Some(e);
                    break;
                }
            }
        }
        (self.check)(&mut session, error, &tokens);
    }
}

impl Session {
    fn intern_span(&mut self, start: usize, end: usize) -> SpanId {
        self.span_mgr.intern_span(self.span_ctx, start, end)
    }

    fn intern_str(&self, s: &str) -> InternedStr {
        self.str_interner.intern(s)
    }
}

fn test_no_error<const N: usize>(
    input: &[u8],
    expected_tokens: impl FnOnce(&mut Session) -> [Token; N],
) {
    LexerTest {
        input,
        check: |session, error, tokens| {
            assert_eq!(error, None);
            assert_eq!(tokens, expected_tokens(session));
        },
    }
    .run();
}

#[test]
fn test_simple() {
    test_no_error(b"", |session| {
        [Token {
            span: session.intern_span(0, 0),
            kind: TokenKind::EndOfFile,
        }]
    });
    test_no_error(b"  ", |session| {
        [
            Token {
                span: session.intern_span(0, 2),
                kind: TokenKind::Whitespace,
            },
            Token {
                span: session.intern_span(2, 2),
                kind: TokenKind::EndOfFile,
            },
        ]
    });

    LexerTest {
        input: b" \xCE\xA9 ",
        check: |session, error, tokens| {
            assert_eq!(
                error,
                Some(LexError::InvalidChar {
                    span: session.intern_span(1, 3),
                    chr: '\u{03A9}',
                }),
            );
            assert_eq!(
                tokens,
                [Token {
                    span: session.intern_span(0, 1),
                    kind: TokenKind::Whitespace,
                }],
            );
        },
    }
    .run();
}

#[test]
fn test_ident() {
    test_no_error(b"hello", |session| {
        [
            Token {
                span: session.intern_span(0, 5),
                kind: TokenKind::Ident(session.intern_str("hello")),
            },
            Token {
                span: session.intern_span(5, 5),
                kind: TokenKind::EndOfFile,
            },
        ]
    });
    test_no_error(b"hello  world", |session| {
        [
            Token {
                span: session.intern_span(0, 5),
                kind: TokenKind::Ident(session.intern_str("hello")),
            },
            Token {
                span: session.intern_span(5, 7),
                kind: TokenKind::Whitespace,
            },
            Token {
                span: session.intern_span(7, 12),
                kind: TokenKind::Ident(session.intern_str("world")),
            },
            Token {
                span: session.intern_span(12, 12),
                kind: TokenKind::EndOfFile,
            },
        ]
    });
}

#[test]
fn test_keywords() {
    fn test(input: &[u8], kind: STokenKind) {
        LexerTest {
            input,
            check: |session, error, tokens| {
                assert_eq!(error, None);
                assert_eq!(
                    tokens,
                    [
                        Token {
                            span: session.intern_span(0, input.len()),
                            kind: TokenKind::Simple(kind),
                        },
                        Token {
                            span: session.intern_span(input.len(), input.len()),
                            kind: TokenKind::EndOfFile,
                        },
                    ],
                );
            },
        }
        .run();
    }

    test(b"assert", STokenKind::Assert);
    test(b"else", STokenKind::Else);
    test(b"error", STokenKind::Error);
    test(b"false", STokenKind::False);
    test(b"for", STokenKind::For);
    test(b"function", STokenKind::Function);
    test(b"if", STokenKind::If);
    test(b"import", STokenKind::Import);
    test(b"importstr", STokenKind::Importstr);
    test(b"importbin", STokenKind::Importbin);
    test(b"in", STokenKind::In);
    test(b"local", STokenKind::Local);
    test(b"null", STokenKind::Null);
    test(b"tailstrict", STokenKind::Tailstrict);
    test(b"then", STokenKind::Then);
    test(b"self", STokenKind::Self_);
    test(b"super", STokenKind::Super);
    test(b"true", STokenKind::True);
}

#[test]
fn test_symbols() {
    fn test(input: &[u8], kind: STokenKind) {
        LexerTest {
            input,
            check: |session, error, tokens| {
                assert_eq!(error, None);
                assert_eq!(
                    tokens,
                    [
                        Token {
                            span: session.intern_span(0, input.len()),
                            kind: TokenKind::Simple(kind),
                        },
                        Token {
                            span: session.intern_span(input.len(), input.len()),
                            kind: TokenKind::EndOfFile,
                        },
                    ],
                );
            },
        }
        .run();
    }

    test(b"!", STokenKind::Exclam);
    test(b"!=", STokenKind::ExclamEq);
    test(b"$", STokenKind::Dollar);
    test(b"%", STokenKind::Percent);
    test(b"&", STokenKind::Amp);
    test(b"&&", STokenKind::AmpAmp);
    test(b"(", STokenKind::LeftParen);
    test(b")", STokenKind::RightParen);
    test(b"*", STokenKind::Asterisk);
    test(b"+", STokenKind::Plus);
    test(b"+:", STokenKind::PlusColon);
    test(b"+::", STokenKind::PlusColonColon);
    test(b"+:::", STokenKind::PlusColonColonColon);
    test(b",", STokenKind::Comma);
    test(b"-", STokenKind::Minus);
    test(b".", STokenKind::Dot);
    test(b"/", STokenKind::Slash);
    test(b":", STokenKind::Colon);
    test(b"::", STokenKind::ColonColon);
    test(b":::", STokenKind::ColonColonColon);
    test(b";", STokenKind::Semicolon);
    test(b"<", STokenKind::Lt);
    test(b"<<", STokenKind::LtLt);
    test(b"<=", STokenKind::LtEq);
    test(b"=", STokenKind::Eq);
    test(b"==", STokenKind::EqEq);
    test(b">", STokenKind::Gt);
    test(b">=", STokenKind::GtEq);
    test(b">>", STokenKind::GtGt);
    test(b"[", STokenKind::LeftBracket);
    test(b"]", STokenKind::RightBracket);
    test(b"^", STokenKind::Hat);
    test(b"{", STokenKind::LeftBrace);
    test(b"|", STokenKind::Pipe);
    test(b"||", STokenKind::PipePipe);
    test(b"}", STokenKind::RightBrace);
    test(b"~", STokenKind::Tilde);
}

#[test]
fn test_other_op() {
    test_no_error(b"+:/", |session| {
        [
            Token {
                span: session.intern_span(0, 3),
                kind: TokenKind::OtherOp("+:/".into()),
            },
            Token {
                span: session.intern_span(3, 3),
                kind: TokenKind::EndOfFile,
            },
        ]
    });
    test_no_error(b"+:/+", |session| {
        [
            Token {
                span: session.intern_span(0, 3),
                kind: TokenKind::OtherOp("+:/".into()),
            },
            Token {
                span: session.intern_span(3, 4),
                kind: TokenKind::Simple(STokenKind::Plus),
            },
            Token {
                span: session.intern_span(4, 4),
                kind: TokenKind::EndOfFile,
            },
        ]
    });
    test_no_error(b"+//++", |session| {
        [
            Token {
                span: session.intern_span(0, 1),
                kind: TokenKind::Simple(STokenKind::Plus),
            },
            Token {
                span: session.intern_span(1, 5),
                kind: TokenKind::Comment,
            },
            Token {
                span: session.intern_span(5, 5),
                kind: TokenKind::EndOfFile,
            },
        ]
    });
    test_no_error(b"+/*++*/", |session| {
        [
            Token {
                span: session.intern_span(0, 1),
                kind: TokenKind::Simple(STokenKind::Plus),
            },
            Token {
                span: session.intern_span(1, 7),
                kind: TokenKind::Comment,
            },
            Token {
                span: session.intern_span(7, 7),
                kind: TokenKind::EndOfFile,
            },
        ]
    });
    test_no_error(b"+|||\n a\n|||", |session| {
        [
            Token {
                span: session.intern_span(0, 1),
                kind: TokenKind::Simple(STokenKind::Plus),
            },
            Token {
                span: session.intern_span(1, 11),
                kind: TokenKind::TextBlock("a\n".into()),
            },
            Token {
                span: session.intern_span(11, 11),
                kind: TokenKind::EndOfFile,
            },
        ]
    });
}

#[test]
fn test_single_line_comment() {
    test_no_error(b"# comment", |session| {
        [
            Token {
                span: session.intern_span(0, 9),
                kind: TokenKind::Comment,
            },
            Token {
                span: session.intern_span(9, 9),
                kind: TokenKind::EndOfFile,
            },
        ]
    });
    test_no_error(b"# comment\na", |session| {
        [
            Token {
                span: session.intern_span(0, 10),
                kind: TokenKind::Comment,
            },
            Token {
                span: session.intern_span(10, 11),
                kind: TokenKind::Ident(session.intern_str("a")),
            },
            Token {
                span: session.intern_span(11, 11),
                kind: TokenKind::EndOfFile,
            },
        ]
    });
    test_no_error(b"// comment", |session| {
        [
            Token {
                span: session.intern_span(0, 10),
                kind: TokenKind::Comment,
            },
            Token {
                span: session.intern_span(10, 10),
                kind: TokenKind::EndOfFile,
            },
        ]
    });
    test_no_error(b"// comment\na", |session| {
        [
            Token {
                span: session.intern_span(0, 11),
                kind: TokenKind::Comment,
            },
            Token {
                span: session.intern_span(11, 12),
                kind: TokenKind::Ident(session.intern_str("a")),
            },
            Token {
                span: session.intern_span(12, 12),
                kind: TokenKind::EndOfFile,
            },
        ]
    });
}

#[test]
fn test_multi_line_comment() {
    test_no_error(b"/* comment */", |session| {
        [
            Token {
                span: session.intern_span(0, 13),
                kind: TokenKind::Comment,
            },
            Token {
                span: session.intern_span(13, 13),
                kind: TokenKind::EndOfFile,
            },
        ]
    });
    test_no_error(b"/* comment\ncomment */", |session| {
        [
            Token {
                span: session.intern_span(0, 21),
                kind: TokenKind::Comment,
            },
            Token {
                span: session.intern_span(21, 21),
                kind: TokenKind::EndOfFile,
            },
        ]
    });
    test_no_error(b"/* comment */ /* comment 2 */", |session| {
        [
            Token {
                span: session.intern_span(0, 13),
                kind: TokenKind::Comment,
            },
            Token {
                span: session.intern_span(13, 14),
                kind: TokenKind::Whitespace,
            },
            Token {
                span: session.intern_span(14, 29),
                kind: TokenKind::Comment,
            },
            Token {
                span: session.intern_span(29, 29),
                kind: TokenKind::EndOfFile,
            },
        ]
    });

    LexerTest {
        input: b"/* comment",
        check: |session, error, tokens| {
            assert_eq!(
                error,
                Some(LexError::UnfinishedMultilineComment {
                    span: session.intern_span(0, 10),
                }),
            );
            assert_eq!(tokens, []);
        },
    }
    .run();
}

#[test]
fn test_number() {
    fn test(input: &[u8], number: Number) {
        LexerTest {
            input,
            check: |session, error, tokens| {
                assert_eq!(error, None);
                assert_eq!(
                    tokens,
                    [
                        Token {
                            span: session.intern_span(0, input.len()),
                            kind: TokenKind::Number(number),
                        },
                        Token {
                            span: session.intern_span(input.len(), input.len()),
                            kind: TokenKind::EndOfFile,
                        },
                    ],
                );
            },
        }
        .run();
    }

    test(
        b"0",
        Number {
            digits: "0".into(),
            exp: 0,
        },
    );
    test(
        b"1",
        Number {
            digits: "1".into(),
            exp: 0,
        },
    );
    test(
        b"12",
        Number {
            digits: "12".into(),
            exp: 0,
        },
    );
    test(
        b"0.0",
        Number {
            digits: "00".into(),
            exp: -1,
        },
    );
    test(
        b"1.0",
        Number {
            digits: "10".into(),
            exp: -1,
        },
    );
    test(
        b"0.1",
        Number {
            digits: "01".into(),
            exp: -1,
        },
    );

    test(
        b"1.2",
        Number {
            digits: "12".into(),
            exp: -1,
        },
    );
    test(
        b"1e10",
        Number {
            digits: "1".into(),
            exp: 10,
        },
    );
    test(
        b"1e+10",
        Number {
            digits: "1".into(),
            exp: 10,
        },
    );
    test(
        b"1e-10",
        Number {
            digits: "1".into(),
            exp: -10,
        },
    );
    test(
        b"1.2e-10",
        Number {
            digits: "12".into(),
            exp: -11,
        },
    );

    LexerTest {
        input: b"01",
        check: |session, error, tokens| {
            assert_eq!(
                error,
                Some(LexError::LeadingZeroInNumber {
                    span: session.intern_span(0, 1),
                }),
            );
            assert_eq!(tokens, []);
        },
    }
    .run();

    LexerTest {
        input: b"1.",
        check: |session, error, tokens| {
            assert_eq!(
                error,
                Some(LexError::MissingFracDigits {
                    span: session.intern_span(1, 2),
                }),
            );
            assert_eq!(tokens, []);
        },
    }
    .run();

    LexerTest {
        input: b"1.e10",
        check: |session, error, tokens| {
            assert_eq!(
                error,
                Some(LexError::MissingFracDigits {
                    span: session.intern_span(1, 2),
                }),
            );
            assert_eq!(tokens, []);
        },
    }
    .run();

    LexerTest {
        input: b"1e",
        check: |session, error, tokens| {
            assert_eq!(
                error,
                Some(LexError::MissingExpDigits {
                    span: session.intern_span(1, 2),
                }),
            );
            assert_eq!(tokens, []);
        },
    }
    .run();

    LexerTest {
        input: b"1e+",
        check: |session, error, tokens| {
            assert_eq!(
                error,
                Some(LexError::MissingExpDigits {
                    span: session.intern_span(1, 3),
                }),
            );
            assert_eq!(tokens, []);
        },
    }
    .run();

    LexerTest {
        input: b"1e-",
        check: |session, error, tokens| {
            assert_eq!(
                error,
                Some(LexError::MissingExpDigits {
                    span: session.intern_span(1, 3),
                }),
            );
            assert_eq!(tokens, []);
        },
    }
    .run();
}

#[test]
fn test_string() {
    fn test(input: &[u8], expected: &str) {
        LexerTest {
            input,
            check: |session, error, tokens| {
                assert_eq!(error, None);
                assert_eq!(
                    tokens,
                    [
                        Token {
                            span: session.intern_span(0, input.len()),
                            kind: TokenKind::String(expected.into()),
                        },
                        Token {
                            span: session.intern_span(input.len(), input.len()),
                            kind: TokenKind::EndOfFile,
                        },
                    ],
                );
            },
        }
        .run();
    }

    // quoted
    test(br#""""#, "");
    test(br#""abcd""#, "abcd");
    test(br"''", "");
    test(br"'abcd'", "abcd");
    test(br"' \n '", " \n ");
    test(br"' \n\t\r\b '", " \n\t\r\u{8} ");
    test(br"' \u0000 '", " \u{0000} ");
    test(br"' \u1234 '", " \u{1234} ");
    test(br"' \uabcd '", " \u{ABCD} ");
    test(br"' \uABCD '", " \u{ABCD} ");
    test(br"' \uffff '", " \u{FFFF} ");
    test(br"' \uFFFF '", " \u{FFFF} ");
    test(br"' \uFFFF\uFFFF '", " \u{FFFF}\u{FFFF} ");
    test(br"' \uD800\uDD56 '", " \u{10156} ");
    test(b"' \xF0\x90\x85\x96 '", " \u{10156} ");
    test(b"' \xFF '", " \u{FFFD} ");
    test(b"\" \xFF \"", " \u{FFFD} ");

    // verbatim
    test(br#"@"""#, "");
    test(br#"@"abcd""#, "abcd");
    test(br#"@"ab\ncd""#, "ab\\ncd");
    test(br#"@"ab""cd""#, "ab\"cd");
    test(br"@''", "");
    test(br"@'ab\ncd'", "ab\\ncd");
    test(br"@'ab''cd'", "ab'cd");
    test(b"@\" \xFF \"", " \u{FFFD} ");

    // error: unfinished
    LexerTest {
        input: b"\"string",
        check: |session, error, tokens| {
            assert_eq!(
                error,
                Some(LexError::UnfinishedString {
                    span: session.intern_span(0, 7),
                }),
            );
            assert_eq!(tokens, []);
        },
    }
    .run();

    LexerTest {
        input: b"\"string\\",
        check: |session, error, tokens| {
            assert_eq!(
                error,
                Some(LexError::UnfinishedString {
                    span: session.intern_span(0, 8),
                }),
            );
            assert_eq!(tokens, []);
        },
    }
    .run();

    LexerTest {
        input: b"@\"string",
        check: |session, error, tokens| {
            assert_eq!(
                error,
                Some(LexError::UnfinishedString {
                    span: session.intern_span(0, 8),
                }),
            );
            assert_eq!(tokens, []);
        },
    }
    .run();

    // error: invalid unicode
    LexerTest {
        input: b"\"\\u\"",
        check: |session, error, tokens| {
            assert_eq!(
                error,
                Some(LexError::IncompleteUnicodeEscape {
                    span: session.intern_span(1, 3),
                }),
            );
            assert_eq!(tokens, []);
        },
    }
    .run();

    LexerTest {
        input: b"\"\\uF\"",
        check: |session, error, tokens| {
            assert_eq!(
                error,
                Some(LexError::IncompleteUnicodeEscape {
                    span: session.intern_span(1, 4),
                }),
            );
            assert_eq!(tokens, []);
        },
    }
    .run();

    LexerTest {
        input: b"\"\\uD800\"",
        check: |session, error, tokens| {
            assert_eq!(
                error,
                Some(LexError::InvalidUtf16EscapeSequence {
                    span: session.intern_span(1, 7),
                    cu1: 0xD800,
                    cu2: None,
                }),
            );
            assert_eq!(tokens, []);
        },
    }
    .run();

    LexerTest {
        input: b"\"\\uD800\\u\"",
        check: |session, error, tokens| {
            assert_eq!(
                error,
                Some(LexError::IncompleteUnicodeEscape {
                    span: session.intern_span(7, 9),
                }),
            );
            assert_eq!(tokens, []);
        },
    }
    .run();

    LexerTest {
        input: b"\"\\uD800\\uF\"",
        check: |session, error, tokens| {
            assert_eq!(
                error,
                Some(LexError::IncompleteUnicodeEscape {
                    span: session.intern_span(7, 10),
                }),
            );
            assert_eq!(tokens, []);
        },
    }
    .run();

    LexerTest {
        input: b"\"\\uD800\\uF000\"",
        check: |session, error, tokens| {
            assert_eq!(
                error,
                Some(LexError::InvalidUtf16EscapeSequence {
                    span: session.intern_span(1, 13),
                    cu1: 0xD800,
                    cu2: Some(0xF000),
                }),
            );
            assert_eq!(tokens, []);
        },
    }
    .run();
}

#[test]
fn test_text_block() {
    fn test(input: &[u8], expected: &str) {
        LexerTest {
            input,
            check: |session, error, tokens| {
                assert_eq!(error, None);
                assert_eq!(
                    tokens,
                    [
                        Token {
                            span: session.intern_span(0, input.len()),
                            kind: TokenKind::TextBlock(expected.into()),
                        },
                        Token {
                            span: session.intern_span(input.len(), input.len()),
                            kind: TokenKind::EndOfFile,
                        },
                    ],
                );
            },
        }
        .run();
    }

    test(b"|||\n \n|||", "\n");
    test(b"|||\n a\n|||", "a\n");
    test(b"|||\n \n a\n|||", "\na\n");
    test(b"||| \n a\n|||", "a\n");
    test(b"|||\t\n a\n|||", "a\n");
    test(b"|||\n\n a\n|||", "\na\n");
    test(b"|||\n\n\n a\n|||", "\n\na\n");
    test(b"|||\n a\n b\n|||", "a\nb\n");
    test(b"|||\n  a\n  b\n|||", "a\nb\n");
    test(b"|||\n a\n\n b\n|||", "a\n\nb\n");
    test(b"|||\n a\n \n b\n|||", "a\n\nb\n");
    test(b"|||\n a\n\n\n b\n|||", "a\n\n\nb\n");
    test(b"|||\n a\n b\n\n|||", "a\nb\n\n");
    test(b"|||\n a\n\t|||", "a\n");
    test(b"|||\n \xFF\n\t|||", "\u{FFFD}\n");

    // error: unfinished
    LexerTest {
        input: b"|||\n a",
        check: |session, error, tokens| {
            assert_eq!(
                error,
                Some(LexError::UnfinishedString {
                    span: session.intern_span(0, 6),
                }),
            );
            assert_eq!(tokens, []);
        },
    }
    .run();

    // error: invalid start
    LexerTest {
        input: b"||| x\n y\n|||",
        check: |session, error, tokens| {
            assert_eq!(
                error,
                Some(LexError::MissingLineBreakAfterTextBlockStart {
                    span: session.intern_span(0, 4),
                }),
            );
            assert_eq!(tokens, []);
        },
    }
    .run();

    LexerTest {
        input: b"|||\nx\ny\n|||",
        check: |session, error, tokens| {
            assert_eq!(
                error,
                Some(LexError::MissingWhitespaceTextBlockStart {
                    span: session.intern_span(4, 4),
                }),
            );
            assert_eq!(tokens, []);
        },
    }
    .run();
}

#[test]
fn test_invalid_utf8() {
    // outside comment or string
    LexerTest {
        input: b" \xFF ",
        check: |session, error, tokens| {
            assert_eq!(
                error,
                Some(LexError::InvalidUtf8 {
                    span: session.intern_span(1, 2),
                    seq: b"\xFF".to_vec(),
                }),
            );
            assert_eq!(
                tokens,
                [Token {
                    span: session.intern_span(0, 1),
                    kind: TokenKind::Whitespace,
                }],
            );
        },
    }
    .run();
}
