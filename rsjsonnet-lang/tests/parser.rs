#![warn(
    rust_2018_idioms,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unused_qualifications
)]
#![forbid(unsafe_code)]

use rsjsonnet_lang::arena::Arena;
use rsjsonnet_lang::ast;
use rsjsonnet_lang::interner::StrInterner;
use rsjsonnet_lang::lexer::Lexer;
use rsjsonnet_lang::parser::{ParseError, Parser};
use rsjsonnet_lang::span::{SpanContextId, SpanId, SpanManager};
use rsjsonnet_lang::token::TokenKind;

#[must_use]
struct ParserTest<'a, F: for<'p> FnOnce(&mut Session<'p>, Result<ast::Expr<'p, 'p>, ParseError>)> {
    input: &'a [u8],
    check: F,
}

struct Session<'p> {
    arena: &'p Arena,
    str_interner: StrInterner<'p>,
    span_mgr: SpanManager,
    span_ctx: SpanContextId,
}

impl<F: for<'p> FnOnce(&mut Session<'p>, Result<ast::Expr<'p, 'p>, ParseError>)> ParserTest<'_, F> {
    #[track_caller]
    fn run(self) {
        let arena = Arena::new();
        let str_interner = StrInterner::new();
        let mut span_mgr = SpanManager::new();
        let (span_ctx, _) = span_mgr.insert_source_context(self.input.len());

        let mut session = Session {
            arena: &arena,
            str_interner,
            span_mgr,
            span_ctx,
        };

        let lexer = Lexer::new(
            &arena,
            &arena,
            &session.str_interner,
            &mut session.span_mgr,
            span_ctx,
            self.input,
        );
        let tokens = lexer.lex_to_eof(false).unwrap();
        assert!(matches!(tokens.last().unwrap().kind, TokenKind::EndOfFile));

        let first_span = tokens[0].span;
        let last_span = tokens[tokens.len() - 1].span;
        let (start_ctx, start_pos, _) = session.span_mgr.get_span(first_span);
        let (end_ctx, _, end_pos) = session.span_mgr.get_span(last_span);

        let parser = Parser::new(
            &arena,
            &arena,
            &session.str_interner,
            &mut session.span_mgr,
            tokens,
        );
        let result_ast = parser.parse_root_expr();

        if let Ok(ref result_ast) = result_ast {
            let (expr_span_ctx, expr_start_pos, expr_end_pos) =
                session.span_mgr.get_span(result_ast.span);
            assert_eq!(expr_span_ctx, start_ctx);
            assert_eq!(expr_span_ctx, end_ctx);
            assert_eq!(expr_start_pos, start_pos);
            assert_eq!(expr_end_pos, end_pos);
        }

        (self.check)(&mut session, result_ast);
    }
}

impl<'p> Session<'p> {
    fn intern_span(&mut self, start: usize, end: usize) -> SpanId {
        self.span_mgr.intern_span(self.span_ctx, start, end)
    }

    fn make_ident(&mut self, s: &str, span_start: usize, span_end: usize) -> ast::Ident<'p> {
        ast::Ident {
            value: self.str_interner.intern(self.arena, s),
            span: self.intern_span(span_start, span_end),
        }
    }

    fn make_ident_expr(
        &mut self,
        s: &str,
        span_start: usize,
        span_end: usize,
    ) -> ast::Expr<'p, 'p> {
        let span = self.intern_span(span_start, span_end);
        ast::Expr {
            kind: ast::ExprKind::Ident(ast::Ident {
                value: self.str_interner.intern(self.arena, s),
                span,
            }),
            span,
        }
    }
}

#[test]
fn test_simple_expr() {
    ParserTest {
        input: b"null",
        check: |_, expr| {
            let expr = expr.unwrap();
            assert_eq!(expr.kind, ast::ExprKind::Null);
        },
    }
    .run();

    ParserTest {
        input: b"true",
        check: |_, expr| {
            let expr = expr.unwrap();
            assert_eq!(expr.kind, ast::ExprKind::Bool(true));
        },
    }
    .run();

    ParserTest {
        input: b"false",
        check: |_, expr| {
            let expr = expr.unwrap();
            assert_eq!(expr.kind, ast::ExprKind::Bool(false));
        },
    }
    .run();

    ParserTest {
        input: b"self",
        check: |_, expr| {
            let expr = expr.unwrap();
            assert_eq!(expr.kind, ast::ExprKind::SelfObj);
        },
    }
    .run();

    ParserTest {
        input: b"$",
        check: |_, expr| {
            let expr = expr.unwrap();
            assert_eq!(expr.kind, ast::ExprKind::Dollar);
        },
    }
    .run();

    ParserTest {
        input: b"1.2",
        check: |_, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Number(rsjsonnet_lang::token::Number {
                    digits: "12",
                    exp: -1,
                })
            );
        },
    }
    .run();

    ParserTest {
        input: b"abc",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Ident(session.make_ident("abc", 0, 3))
            );
        },
    }
    .run();

    ParserTest {
        input: b"\"x\"",
        check: |_, expr| {
            let expr = expr.unwrap();
            assert_eq!(expr.kind, ast::ExprKind::String("x"));
        },
    }
    .run();

    ParserTest {
        input: b"|||\n  x\n|||",
        check: |_, expr| {
            let expr = expr.unwrap();
            assert_eq!(expr.kind, ast::ExprKind::TextBlock("x\n"));
        },
    }
    .run();
}

#[test]
fn test_paren_expr() {
    ParserTest {
        input: b"(null)",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Paren(session.arena.alloc(ast::Expr {
                    kind: ast::ExprKind::Null,
                    span: session.intern_span(1, 5),
                }))
            );
        },
    }
    .run();
}

#[test]
fn test_object_expr() {
    ParserTest {
        input: b"{}",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Object(ast::ObjInside::Members(session.arena.alloc([]))),
            );
        },
    }
    .run();

    ParserTest {
        input: b"{a: b}",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Object(ast::ObjInside::Members(session.arena.alloc([
                    ast::Member::Field(ast::Field::Value(
                        ast::FieldName::Ident(session.make_ident("a", 1, 2)),
                        false,
                        ast::Visibility::Default,
                        session.make_ident_expr("b", 4, 5),
                    ),)
                ]))),
            );
        },
    }
    .run();

    ParserTest {
        input: b"{a: b,}",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Object(ast::ObjInside::Members(session.arena.alloc([
                    ast::Member::Field(ast::Field::Value(
                        ast::FieldName::Ident(session.make_ident("a", 1, 2)),
                        false,
                        ast::Visibility::Default,
                        session.make_ident_expr("b", 4, 5),
                    ),)
                ]))),
            );
        },
    }
    .run();

    ParserTest {
        input: b"{a: b, c: d}",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Object(ast::ObjInside::Members(session.arena.alloc([
                    ast::Member::Field(ast::Field::Value(
                        ast::FieldName::Ident(session.make_ident("a", 1, 2)),
                        false,
                        ast::Visibility::Default,
                        session.make_ident_expr("b", 4, 5),
                    )),
                    ast::Member::Field(ast::Field::Value(
                        ast::FieldName::Ident(session.make_ident("c", 7, 8)),
                        false,
                        ast::Visibility::Default,
                        session.make_ident_expr("d", 10, 11),
                    )),
                ]))),
            );
        },
    }
    .run();

    ParserTest {
        input: b"{a: b, c: d,}",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Object(ast::ObjInside::Members(session.arena.alloc([
                    ast::Member::Field(ast::Field::Value(
                        ast::FieldName::Ident(session.make_ident("a", 1, 2)),
                        false,
                        ast::Visibility::Default,
                        session.make_ident_expr("b", 4, 5),
                    )),
                    ast::Member::Field(ast::Field::Value(
                        ast::FieldName::Ident(session.make_ident("c", 7, 8)),
                        false,
                        ast::Visibility::Default,
                        session.make_ident_expr("d", 10, 11),
                    )),
                ]))),
            );
        },
    }
    .run();

    ParserTest {
        input: b"{a(b): c}",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Object(ast::ObjInside::Members(session.arena.alloc([
                    ast::Member::Field(ast::Field::Func(
                        ast::FieldName::Ident(session.make_ident("a", 1, 2)),
                        session.arena.alloc([ast::Param {
                            name: session.make_ident("b", 3, 4),
                            default_value: None,
                        }]),
                        session.intern_span(2, 5),
                        ast::Visibility::Default,
                        session.make_ident_expr("c", 7, 8),
                    ),)
                ]))),
            );
        },
    }
    .run();

    ParserTest {
        input: b"{a(b,): c}",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Object(ast::ObjInside::Members(session.arena.alloc([
                    ast::Member::Field(ast::Field::Func(
                        ast::FieldName::Ident(session.make_ident("a", 1, 2)),
                        session.arena.alloc([ast::Param {
                            name: session.make_ident("b", 3, 4),
                            default_value: None,
                        }]),
                        session.intern_span(2, 6),
                        ast::Visibility::Default,
                        session.make_ident_expr("c", 8, 9),
                    ),)
                ]))),
            );
        },
    }
    .run();

    ParserTest {
        input: b"{a(b, c): d}",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Object(ast::ObjInside::Members(session.arena.alloc([
                    ast::Member::Field(ast::Field::Func(
                        ast::FieldName::Ident(session.make_ident("a", 1, 2)),
                        session.arena.alloc([
                            ast::Param {
                                name: session.make_ident("b", 3, 4),
                                default_value: None,
                            },
                            ast::Param {
                                name: session.make_ident("c", 6, 7),
                                default_value: None,
                            }
                        ]),
                        session.intern_span(2, 8),
                        ast::Visibility::Default,
                        session.make_ident_expr("d", 10, 11),
                    ),)
                ]))),
            );
        },
    }
    .run();

    ParserTest {
        input: b"{local x = a}",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Object(ast::ObjInside::Members(session.arena.alloc([
                    ast::Member::Local(ast::ObjLocal {
                        bind: ast::Bind {
                            name: session.make_ident("x", 7, 8),
                            params: None,
                            value: session.make_ident_expr("a", 11, 12),
                        },
                    },)
                ]))),
            );
        },
    }
    .run();

    ParserTest {
        input: b"{local x = a,}",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Object(ast::ObjInside::Members(session.arena.alloc([
                    ast::Member::Local(ast::ObjLocal {
                        bind: ast::Bind {
                            name: session.make_ident("x", 7, 8),
                            params: None,
                            value: session.make_ident_expr("a", 11, 12),
                        },
                    },)
                ]))),
            );
        },
    }
    .run();

    ParserTest {
        input: b"{local x = a, local y = b}",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Object(ast::ObjInside::Members(session.arena.alloc([
                    ast::Member::Local(ast::ObjLocal {
                        bind: ast::Bind {
                            name: session.make_ident("x", 7, 8),
                            params: None,
                            value: session.make_ident_expr("a", 11, 12),
                        },
                    }),
                    ast::Member::Local(ast::ObjLocal {
                        bind: ast::Bind {
                            name: session.make_ident("y", 20, 21),
                            params: None,
                            value: session.make_ident_expr("b", 24, 25),
                        },
                    }),
                ]))),
            );
        },
    }
    .run();

    ParserTest {
        input: b"{local x = a, local y = b,}",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Object(ast::ObjInside::Members(session.arena.alloc([
                    ast::Member::Local(ast::ObjLocal {
                        bind: ast::Bind {
                            name: session.make_ident("x", 7, 8),
                            params: None,
                            value: session.make_ident_expr("a", 11, 12),
                        },
                    }),
                    ast::Member::Local(ast::ObjLocal {
                        bind: ast::Bind {
                            name: session.make_ident("y", 20, 21),
                            params: None,
                            value: session.make_ident_expr("b", 24, 25),
                        },
                    }),
                ]))),
            );
        },
    }
    .run();

    ParserTest {
        input: b"{local x = a, y: b}",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Object(ast::ObjInside::Members(session.arena.alloc([
                    ast::Member::Local(ast::ObjLocal {
                        bind: ast::Bind {
                            name: session.make_ident("x", 7, 8),
                            params: None,
                            value: session.make_ident_expr("a", 11, 12),
                        },
                    }),
                    ast::Member::Field(ast::Field::Value(
                        ast::FieldName::Ident(session.make_ident("y", 14, 15)),
                        false,
                        ast::Visibility::Default,
                        session.make_ident_expr("b", 17, 18),
                    )),
                ]))),
            );
        },
    }
    .run();

    ParserTest {
        input: b"{local x = a, y: b,}",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Object(ast::ObjInside::Members(session.arena.alloc([
                    ast::Member::Local(ast::ObjLocal {
                        bind: ast::Bind {
                            name: session.make_ident("x", 7, 8),
                            params: None,
                            value: session.make_ident_expr("a", 11, 12),
                        },
                    }),
                    ast::Member::Field(ast::Field::Value(
                        ast::FieldName::Ident(session.make_ident("y", 14, 15)),
                        false,
                        ast::Visibility::Default,
                        session.make_ident_expr("b", 17, 18),
                    )),
                ]))),
            );
        },
    }
    .run();

    ParserTest {
        input: b"{local x = a, assert x == a}",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Object(ast::ObjInside::Members(session.arena.alloc([
                    ast::Member::Local(ast::ObjLocal {
                        bind: ast::Bind {
                            name: session.make_ident("x", 7, 8),
                            params: None,
                            value: session.make_ident_expr("a", 11, 12),
                        },
                    }),
                    ast::Member::Assert(ast::Assert {
                        span: session.intern_span(14, 27),
                        cond: ast::Expr {
                            kind: ast::ExprKind::Binary(
                                session.arena.alloc(session.make_ident_expr("x", 21, 22)),
                                ast::BinaryOp::Eq,
                                session.arena.alloc(session.make_ident_expr("a", 26, 27)),
                            ),
                            span: session.intern_span(21, 27),
                        },
                        msg: None,
                    }),
                ]))),
            );
        },
    }
    .run();

    ParserTest {
        input: b"{[x + y]: 1}",
        check: |_, expr| {
            let expr = expr.unwrap();
            assert!(matches!(
                expr.kind,
                ast::ExprKind::Object(ast::ObjInside::Members(members)) if members.len() == 1
            ));
        },
    }
    .run();

    ParserTest {
        input: b"{[x + y]: 1, f: 2}",
        check: |_, expr| {
            let expr = expr.unwrap();
            assert!(matches!(
                expr.kind,
                ast::ExprKind::Object(ast::ObjInside::Members(members)) if members.len() == 2
            ));
        },
    }
    .run();

    ParserTest {
        input: b"{local x = \"1\", [x + y]: 1}",
        check: |_, expr| {
            let expr = expr.unwrap();
            assert!(matches!(
                expr.kind,
                ast::ExprKind::Object(ast::ObjInside::Members(members)) if members.len() == 2
            ));
        },
    }
    .run();

    ParserTest {
        input: b"{local x = \"1\", [x + y]: 1, local z = \"2\"}",
        check: |_, expr| {
            let expr = expr.unwrap();
            assert!(matches!(
                expr.kind,
                ast::ExprKind::Object(ast::ObjInside::Members(members)) if members.len() == 3
            ));
        },
    }
    .run();

    ParserTest {
        input: b"{local x = \"1\", [x + y]: 1, a: 2}",
        check: |_, expr| {
            let expr = expr.unwrap();
            assert!(matches!(
                expr.kind,
                ast::ExprKind::Object(ast::ObjInside::Members(members)) if members.len() == 3
            ));
        },
    }
    .run();

    ParserTest {
        input: b"{[x + y]: 1 for a in b if c}",
        check: |_, expr| {
            let expr = expr.unwrap();
            assert!(matches!(
                expr.kind,
                ast::ExprKind::Object(ast::ObjInside::Comp { .. })
            ));
        },
    }
    .run();

    ParserTest {
        input: b"{[x + y]: 1, for a in b if c}",
        check: |_, expr| {
            let expr = expr.unwrap();
            assert!(matches!(
                expr.kind,
                ast::ExprKind::Object(ast::ObjInside::Comp { .. })
            ));
        },
    }
    .run();

    ParserTest {
        input: b"{[x + y]: 1 for a in b if c if d}",
        check: |_, expr| {
            let expr = expr.unwrap();
            assert!(matches!(
                expr.kind,
                ast::ExprKind::Object(ast::ObjInside::Comp { .. })
            ));
        },
    }
    .run();

    ParserTest {
        input: b"{local x = \"1\", [x + y]: 1, local z = \"2\" for a in b for c in d}",
        check: |_, expr| {
            let expr = expr.unwrap();
            assert!(matches!(
                expr.kind,
                ast::ExprKind::Object(ast::ObjInside::Comp { .. })
            ));
        },
    }
    .run();
}

#[test]
fn test_array_expr() {
    ParserTest {
        input: b"[]",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(expr.kind, ast::ExprKind::Array(session.arena.alloc([])));
        },
    }
    .run();

    ParserTest {
        input: b"[a]",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Array(session.arena.alloc([session.make_ident_expr("a", 1, 2)])),
            );
        },
    }
    .run();

    ParserTest {
        input: b"[a,]",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Array(session.arena.alloc([session.make_ident_expr("a", 1, 2)])),
            );
        },
    }
    .run();

    ParserTest {
        input: b"[a, b]",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Array(session.arena.alloc([
                    session.make_ident_expr("a", 1, 2),
                    session.make_ident_expr("b", 4, 5),
                ])),
            );
        },
    }
    .run();

    ParserTest {
        input: b"[a, b,]",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Array(session.arena.alloc([
                    session.make_ident_expr("a", 1, 2),
                    session.make_ident_expr("b", 4, 5),
                ])),
            );
        },
    }
    .run();

    ParserTest {
        input: b"[a, b, c]",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Array(session.arena.alloc([
                    session.make_ident_expr("a", 1, 2),
                    session.make_ident_expr("b", 4, 5),
                    session.make_ident_expr("c", 7, 8),
                ])),
            );
        },
    }
    .run();

    ParserTest {
        input: b"[a, b, c,]",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Array(session.arena.alloc([
                    session.make_ident_expr("a", 1, 2),
                    session.make_ident_expr("b", 4, 5),
                    session.make_ident_expr("c", 7, 8),
                ])),
            );
        },
    }
    .run();

    ParserTest {
        input: b"[a for b in c]",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::ArrayComp(
                    session.arena.alloc(session.make_ident_expr("a", 1, 2)),
                    session.arena.alloc([ast::CompSpecPart::For(ast::ForSpec {
                        var: session.make_ident("b", 7, 8),
                        inner: session.make_ident_expr("c", 12, 13),
                    })]),
                ),
            );
        },
    }
    .run();

    ParserTest {
        input: b"[a for b in c if d]",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::ArrayComp(
                    session.arena.alloc(session.make_ident_expr("a", 1, 2)),
                    session.arena.alloc([
                        ast::CompSpecPart::For(ast::ForSpec {
                            var: session.make_ident("b", 7, 8),
                            inner: session.make_ident_expr("c", 12, 13),
                        }),
                        ast::CompSpecPart::If(ast::IfSpec {
                            cond: session.make_ident_expr("d", 17, 18),
                        }),
                    ]),
                ),
            );
        },
    }
    .run();

    ParserTest {
        input: b"[a for b in c if d if e]",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::ArrayComp(
                    session.arena.alloc(session.make_ident_expr("a", 1, 2)),
                    session.arena.alloc([
                        ast::CompSpecPart::For(ast::ForSpec {
                            var: session.make_ident("b", 7, 8),
                            inner: session.make_ident_expr("c", 12, 13),
                        }),
                        ast::CompSpecPart::If(ast::IfSpec {
                            cond: session.make_ident_expr("d", 17, 18),
                        }),
                        ast::CompSpecPart::If(ast::IfSpec {
                            cond: session.make_ident_expr("e", 22, 23),
                        })
                    ]),
                ),
            );
        },
    }
    .run();

    ParserTest {
        input: b"[a, for b in c]",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::ArrayComp(
                    session.arena.alloc(session.make_ident_expr("a", 1, 2)),
                    session.arena.alloc([ast::CompSpecPart::For(ast::ForSpec {
                        var: session.make_ident("b", 8, 9),
                        inner: session.make_ident_expr("c", 13, 14),
                    })]),
                ),
            );
        },
    }
    .run();

    ParserTest {
        input: b"[a, for b in c if d]",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::ArrayComp(
                    session.arena.alloc(session.make_ident_expr("a", 1, 2)),
                    session.arena.alloc([
                        ast::CompSpecPart::For(ast::ForSpec {
                            var: session.make_ident("b", 8, 9),
                            inner: session.make_ident_expr("c", 13, 14),
                        }),
                        ast::CompSpecPart::If(ast::IfSpec {
                            cond: session.make_ident_expr("d", 18, 19),
                        }),
                    ]),
                ),
            );
        },
    }
    .run();

    ParserTest {
        input: b"[a for b in c for d in e]",
        check: |_, expr| {
            let expr = expr.unwrap();
            assert!(matches!(expr.kind, ast::ExprKind::ArrayComp(_, _)));
        },
    }
    .run();

    ParserTest {
        input: b"[a, for b in c for d in e]",
        check: |_, expr| {
            let expr = expr.unwrap();
            assert!(matches!(expr.kind, ast::ExprKind::ArrayComp(_, _)));
        },
    }
    .run();
}

#[test]
fn test_field_expr() {
    ParserTest {
        input: b"a.b",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Field(
                    session.arena.alloc(session.make_ident_expr("a", 0, 1)),
                    session.make_ident("b", 2, 3),
                ),
            );
        },
    }
    .run();

    ParserTest {
        input: b"a.b.c",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Field(
                    session.arena.alloc(ast::Expr {
                        kind: ast::ExprKind::Field(
                            session.arena.alloc(session.make_ident_expr("a", 0, 1)),
                            session.make_ident("b", 2, 3),
                        ),
                        span: session.intern_span(0, 3),
                    }),
                    session.make_ident("c", 4, 5),
                ),
            );
        },
    }
    .run();
}

#[test]
fn test_index_expr() {
    ParserTest {
        input: b"a[b]",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Index(
                    session.arena.alloc(session.make_ident_expr("a", 0, 1)),
                    session.arena.alloc(session.make_ident_expr("b", 2, 3)),
                ),
            );
        },
    }
    .run();

    ParserTest {
        input: b"a[b:c]",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Slice(
                    session.arena.alloc(session.make_ident_expr("a", 0, 1)),
                    Some(session.arena.alloc(session.make_ident_expr("b", 2, 3))),
                    Some(session.arena.alloc(session.make_ident_expr("c", 4, 5))),
                    None
                ),
            );
        },
    }
    .run();

    ParserTest {
        input: b"a[b:c:d]",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Slice(
                    session.arena.alloc(session.make_ident_expr("a", 0, 1)),
                    Some(session.arena.alloc(session.make_ident_expr("b", 2, 3))),
                    Some(session.arena.alloc(session.make_ident_expr("c", 4, 5))),
                    Some(session.arena.alloc(session.make_ident_expr("d", 6, 7))),
                ),
            );
        },
    }
    .run();

    ParserTest {
        input: b"a[b:]",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Slice(
                    session.arena.alloc(session.make_ident_expr("a", 0, 1)),
                    Some(session.arena.alloc(session.make_ident_expr("b", 2, 3))),
                    None,
                    None
                ),
            );
        },
    }
    .run();

    ParserTest {
        input: b"a[b::]",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Slice(
                    session.arena.alloc(session.make_ident_expr("a", 0, 1)),
                    Some(session.arena.alloc(session.make_ident_expr("b", 2, 3))),
                    None,
                    None
                ),
            );
        },
    }
    .run();

    ParserTest {
        input: b"a[b: :]",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Slice(
                    session.arena.alloc(session.make_ident_expr("a", 0, 1)),
                    Some(session.arena.alloc(session.make_ident_expr("b", 2, 3))),
                    None,
                    None
                ),
            );
        },
    }
    .run();

    ParserTest {
        input: b"a[b::d]",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Slice(
                    session.arena.alloc(session.make_ident_expr("a", 0, 1)),
                    Some(session.arena.alloc(session.make_ident_expr("b", 2, 3))),
                    None,
                    Some(session.arena.alloc(session.make_ident_expr("d", 5, 6))),
                ),
            );
        },
    }
    .run();

    ParserTest {
        input: b"a[b: :d]",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Slice(
                    session.arena.alloc(session.make_ident_expr("a", 0, 1)),
                    Some(session.arena.alloc(session.make_ident_expr("b", 2, 3))),
                    None,
                    Some(session.arena.alloc(session.make_ident_expr("d", 6, 7))),
                ),
            );
        },
    }
    .run();

    ParserTest {
        input: b"a[:]",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Slice(
                    session.arena.alloc(session.make_ident_expr("a", 0, 1)),
                    None,
                    None,
                    None
                ),
            );
        },
    }
    .run();

    ParserTest {
        input: b"a[::]",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Slice(
                    session.arena.alloc(session.make_ident_expr("a", 0, 1)),
                    None,
                    None,
                    None
                ),
            );
        },
    }
    .run();

    ParserTest {
        input: b"a[: :]",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Slice(
                    session.arena.alloc(session.make_ident_expr("a", 0, 1)),
                    None,
                    None,
                    None
                ),
            );
        },
    }
    .run();

    ParserTest {
        input: b"a[:c]",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Slice(
                    session.arena.alloc(session.make_ident_expr("a", 0, 1)),
                    None,
                    Some(session.arena.alloc(session.make_ident_expr("c", 3, 4))),
                    None,
                ),
            );
        },
    }
    .run();

    ParserTest {
        input: b"a[:c:]",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Slice(
                    session.arena.alloc(session.make_ident_expr("a", 0, 1)),
                    None,
                    Some(session.arena.alloc(session.make_ident_expr("c", 3, 4))),
                    None,
                ),
            );
        },
    }
    .run();

    ParserTest {
        input: b"a[::d]",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Slice(
                    session.arena.alloc(session.make_ident_expr("a", 0, 1)),
                    None,
                    None,
                    Some(session.arena.alloc(session.make_ident_expr("d", 4, 5))),
                ),
            );
        },
    }
    .run();

    ParserTest {
        input: b"a[: :d]",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Slice(
                    session.arena.alloc(session.make_ident_expr("a", 0, 1)),
                    None,
                    None,
                    Some(session.arena.alloc(session.make_ident_expr("d", 5, 6))),
                ),
            );
        },
    }
    .run();

    ParserTest {
        input: b"a[:c:d]",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Slice(
                    session.arena.alloc(session.make_ident_expr("a", 0, 1)),
                    None,
                    Some(session.arena.alloc(session.make_ident_expr("c", 3, 4))),
                    Some(session.arena.alloc(session.make_ident_expr("d", 5, 6))),
                ),
            );
        },
    }
    .run();
}

#[test]
fn test_super_expr() {
    ParserTest {
        input: b"super.field",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::SuperField(
                    session.intern_span(0, 5),
                    session.make_ident("field", 6, 11),
                ),
            );
        },
    }
    .run();

    ParserTest {
        input: b"super[index]",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::SuperIndex(
                    session.intern_span(0, 5),
                    session.arena.alloc(session.make_ident_expr("index", 6, 11)),
                ),
            );
        },
    }
    .run();
}

#[test]
fn test_call_expr() {
    ParserTest {
        input: b"a()",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Call(
                    session.arena.alloc(session.make_ident_expr("a", 0, 1)),
                    session.arena.alloc([]),
                    false
                ),
            );
        },
    }
    .run();

    ParserTest {
        input: b"a() tailstrict",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Call(
                    session.arena.alloc(session.make_ident_expr("a", 0, 1)),
                    session.arena.alloc([]),
                    true
                ),
            );
        },
    }
    .run();

    ParserTest {
        input: b"a(b)",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Call(
                    session.arena.alloc(session.make_ident_expr("a", 0, 1)),
                    session
                        .arena
                        .alloc([ast::Arg::Positional(session.make_ident_expr("b", 2, 3))]),
                    false,
                ),
            );
        },
    }
    .run();

    ParserTest {
        input: b"a(b) tailstrict",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Call(
                    session.arena.alloc(session.make_ident_expr("a", 0, 1)),
                    session
                        .arena
                        .alloc([ast::Arg::Positional(session.make_ident_expr("b", 2, 3))]),
                    true,
                ),
            );
        },
    }
    .run();

    ParserTest {
        input: b"a(b,)",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Call(
                    session.arena.alloc(session.make_ident_expr("a", 0, 1)),
                    session
                        .arena
                        .alloc([ast::Arg::Positional(session.make_ident_expr("b", 2, 3))]),
                    false,
                ),
            );
        },
    }
    .run();

    ParserTest {
        input: b"a(b,)",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Call(
                    session.arena.alloc(session.make_ident_expr("a", 0, 1)),
                    session
                        .arena
                        .alloc([ast::Arg::Positional(session.make_ident_expr("b", 2, 3))]),
                    false,
                ),
            );
        },
    }
    .run();

    ParserTest {
        input: b"a(b, c)",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Call(
                    session.arena.alloc(session.make_ident_expr("a", 0, 1)),
                    session.arena.alloc([
                        ast::Arg::Positional(session.make_ident_expr("b", 2, 3)),
                        ast::Arg::Positional(session.make_ident_expr("c", 5, 6)),
                    ]),
                    false
                ),
            );
        },
    }
    .run();

    ParserTest {
        input: b"a(b, c,)",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Call(
                    session.arena.alloc(session.make_ident_expr("a", 0, 1)),
                    session.arena.alloc([
                        ast::Arg::Positional(session.make_ident_expr("b", 2, 3)),
                        ast::Arg::Positional(session.make_ident_expr("c", 5, 6)),
                    ]),
                    false
                ),
            );
        },
    }
    .run();

    ParserTest {
        input: b"a(b = c)",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Call(
                    session.arena.alloc(session.make_ident_expr("a", 0, 1)),
                    session.arena.alloc([ast::Arg::Named(
                        session.make_ident("b", 2, 3),
                        session.make_ident_expr("c", 6, 7),
                    )]),
                    false
                ),
            );
        },
    }
    .run();
}

#[test]
fn test_local_expr() {
    ParserTest {
        input: b"local x = v1; a",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Local(
                    session.arena.alloc([ast::Bind {
                        name: session.make_ident("x", 6, 7),
                        params: None,
                        value: session.make_ident_expr("v1", 10, 12),
                    }]),
                    session.arena.alloc(session.make_ident_expr("a", 14, 15)),
                ),
            );
        },
    }
    .run();

    ParserTest {
        input: b"local x = v1, y = v2; a",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Local(
                    session.arena.alloc([
                        ast::Bind {
                            name: session.make_ident("x", 6, 7),
                            params: None,
                            value: session.make_ident_expr("v1", 10, 12),
                        },
                        ast::Bind {
                            name: session.make_ident("y", 14, 15),
                            params: None,
                            value: session.make_ident_expr("v2", 18, 20),
                        }
                    ]),
                    session.arena.alloc(session.make_ident_expr("a", 22, 23)),
                ),
            );
        },
    }
    .run();

    ParserTest {
        input: b"local x() = 0; x",
        check: |_, expr| {
            let expr = expr.unwrap();
            assert!(matches!(expr.kind, ast::ExprKind::Local(_, _)));
        },
    }
    .run();

    ParserTest {
        input: b"local x(a) = 0; x",
        check: |_, expr| {
            let expr = expr.unwrap();
            assert!(matches!(expr.kind, ast::ExprKind::Local(_, _)));
        },
    }
    .run();

    ParserTest {
        input: b"local x(a = 1) = 0; x",
        check: |_, expr| {
            let expr = expr.unwrap();
            assert!(matches!(expr.kind, ast::ExprKind::Local(_, _)));
        },
    }
    .run();

    ParserTest {
        input: b"local x(a,) = 0; x",
        check: |_, expr| {
            let expr = expr.unwrap();
            assert!(matches!(expr.kind, ast::ExprKind::Local(_, _)));
        },
    }
    .run();

    ParserTest {
        input: b"local x(a, b) = 0; x",
        check: |_, expr| {
            let expr = expr.unwrap();
            assert!(matches!(expr.kind, ast::ExprKind::Local(_, _)));
        },
    }
    .run();

    ParserTest {
        input: b"local x(a, b,) = 0; x",
        check: |_, expr| {
            let expr = expr.unwrap();
            assert!(matches!(expr.kind, ast::ExprKind::Local(_, _)));
        },
    }
    .run();
}

#[test]
fn test_if_expr() {
    ParserTest {
        input: b"if a then b",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::If(
                    session.arena.alloc(session.make_ident_expr("a", 3, 4)),
                    session.arena.alloc(session.make_ident_expr("b", 10, 11)),
                    None,
                )
            );
        },
    }
    .run();

    ParserTest {
        input: b"if a then b else c",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::If(
                    session.arena.alloc(session.make_ident_expr("a", 3, 4)),
                    session.arena.alloc(session.make_ident_expr("b", 10, 11)),
                    Some(session.arena.alloc(session.make_ident_expr("c", 17, 18))),
                )
            );
        },
    }
    .run();
}

#[test]
fn test_obj_ext_expr() {
    ParserTest {
        input: b"a {}",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::ObjExt(
                    session.arena.alloc(session.make_ident_expr("a", 0, 1)),
                    ast::ObjInside::Members(session.arena.alloc([])),
                    session.intern_span(2, 4),
                )
            );
        },
    }
    .run();
}

#[test]
fn test_func_expr() {
    ParserTest {
        input: b"function() 0",
        check: |_, expr| {
            let expr = expr.unwrap();
            assert!(matches!(expr.kind, ast::ExprKind::Func(_, _)));
        },
    }
    .run();

    ParserTest {
        input: b"function(a) a",
        check: |_, expr| {
            let expr = expr.unwrap();
            assert!(matches!(expr.kind, ast::ExprKind::Func(_, _)));
        },
    }
    .run();

    ParserTest {
        input: b"function(a,) a",
        check: |_, expr| {
            let expr = expr.unwrap();
            assert!(matches!(expr.kind, ast::ExprKind::Func(_, _)));
        },
    }
    .run();

    ParserTest {
        input: b"function(a, b) a + b",
        check: |_, expr| {
            let expr = expr.unwrap();
            assert!(matches!(expr.kind, ast::ExprKind::Func(_, _)));
        },
    }
    .run();
}

#[test]
fn test_assert_expr() {
    ParserTest {
        input: b"assert cond; value",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Assert(
                    session.arena.alloc(ast::Assert {
                        span: session.intern_span(0, 11),
                        cond: session.make_ident_expr("cond", 7, 11),
                        msg: None
                    }),
                    session
                        .arena
                        .alloc(session.make_ident_expr("value", 13, 18)),
                )
            );
        },
    }
    .run();

    ParserTest {
        input: b"assert cond : msg; value",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Assert(
                    session.arena.alloc(ast::Assert {
                        span: session.intern_span(0, 17),
                        cond: session.make_ident_expr("cond", 7, 11),
                        msg: Some(session.make_ident_expr("msg", 14, 17)),
                    }),
                    session
                        .arena
                        .alloc(session.make_ident_expr("value", 19, 24)),
                )
            );
        },
    }
    .run();
}

#[test]
fn test_import_expr() {
    ParserTest {
        input: b"import \"x\"",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Import(session.arena.alloc(ast::Expr {
                    kind: ast::ExprKind::String("x"),
                    span: session.intern_span(7, 10),
                })),
            );
        },
    }
    .run();

    ParserTest {
        input: b"importstr \"x\"",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::ImportStr(session.arena.alloc(ast::Expr {
                    kind: ast::ExprKind::String("x"),
                    span: session.intern_span(10, 13),
                })),
            );
        },
    }
    .run();

    ParserTest {
        input: b"importbin \"x\"",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::ImportBin(session.arena.alloc(ast::Expr {
                    kind: ast::ExprKind::String("x"),
                    span: session.intern_span(10, 13),
                })),
            );
        },
    }
    .run();
}

#[test]
fn test_error_expr() {
    ParserTest {
        input: b"error \"err\"",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Error(session.arena.alloc(ast::Expr {
                    kind: ast::ExprKind::String("err"),
                    span: session.intern_span(6, 11),
                })),
            );
        },
    }
    .run();
}

fn str_to_binop(s: &str) -> ast::BinaryOp {
    match s {
        "+" => ast::BinaryOp::Add,
        "-" => ast::BinaryOp::Sub,
        "*" => ast::BinaryOp::Mul,
        "/" => ast::BinaryOp::Div,
        "%" => ast::BinaryOp::Rem,
        "<<" => ast::BinaryOp::Shl,
        ">>" => ast::BinaryOp::Shr,
        "<" => ast::BinaryOp::Lt,
        "<=" => ast::BinaryOp::Le,
        ">" => ast::BinaryOp::Gt,
        ">=" => ast::BinaryOp::Ge,
        "==" => ast::BinaryOp::Eq,
        "!=" => ast::BinaryOp::Ne,
        "in" => ast::BinaryOp::In,
        "&" => ast::BinaryOp::BitwiseAnd,
        "|" => ast::BinaryOp::BitwiseOr,
        "^" => ast::BinaryOp::BitwiseXor,
        "&&" => ast::BinaryOp::LogicAnd,
        "||" => ast::BinaryOp::LogicOr,
        _ => unreachable!(),
    }
}

#[test]
fn test_binary_expr() {
    fn make_binop_2<'p>(
        session: &mut Session<'p>,
        op: ast::BinaryOp,
        op_len: usize,
    ) -> ast::Expr<'p, 'p> {
        let lhs = session.make_ident_expr("a", 0, 1);
        let rhs = session.make_ident_expr("b", op_len + 3, op_len + 4);

        ast::Expr {
            kind: ast::ExprKind::Binary(session.arena.alloc(lhs), op, session.arena.alloc(rhs)),
            span: session.intern_span(0, op_len + 4),
        }
    }

    fn make_binop_3l<'p>(
        session: &mut Session<'p>,
        op1: ast::BinaryOp,
        op1_len: usize,
        op2: ast::BinaryOp,
        op2_len: usize,
    ) -> ast::Expr<'p, 'p> {
        let hs1 = session.make_ident_expr("a", 0, 1);
        let hs2 = session.make_ident_expr("b", op1_len + 3, op1_len + 4);
        let hs3 = session.make_ident_expr("c", op1_len + op2_len + 6, op1_len + op2_len + 7);

        ast::Expr {
            kind: ast::ExprKind::Binary(
                session.arena.alloc(ast::Expr {
                    kind: ast::ExprKind::Binary(
                        session.arena.alloc(hs1),
                        op1,
                        session.arena.alloc(hs2),
                    ),
                    span: session.intern_span(0, op1_len + 4),
                }),
                op2,
                session.arena.alloc(hs3),
            ),
            span: session.intern_span(0, op1_len + op2_len + 7),
        }
    }

    fn make_binop_3r<'p>(
        session: &mut Session<'p>,
        op1: ast::BinaryOp,
        op1_len: usize,
        op2: ast::BinaryOp,
        op2_len: usize,
    ) -> ast::Expr<'p, 'p> {
        let hs1 = session.make_ident_expr("a", 0, 1);
        let hs2 = session.make_ident_expr("b", op1_len + 3, op1_len + 4);
        let hs3 = session.make_ident_expr("c", op1_len + op2_len + 6, op1_len + op2_len + 7);

        ast::Expr {
            kind: ast::ExprKind::Binary(
                session.arena.alloc(hs1),
                op1,
                session.arena.alloc(ast::Expr {
                    kind: ast::ExprKind::Binary(
                        session.arena.alloc(hs2),
                        op2,
                        session.arena.alloc(hs3),
                    ),
                    span: session.intern_span(op1_len + 3, op1_len + op2_len + 7),
                }),
            ),
            span: session.intern_span(0, op1_len + op2_len + 7),
        }
    }

    const OPS: &[&[&str]] = &[
        &["||"],
        &["&&"],
        &["|"],
        &["^"],
        &["&"],
        &["==", "!="],
        &["<", "<=", ">", ">=", "in"],
        &["<<", ">>"],
        &["+", "-"],
        &["*", "/", "%"],
    ];

    for (i, this_ops) in OPS.iter().enumerate() {
        for &this_op_s in this_ops.iter() {
            let this_op = str_to_binop(this_op_s);

            ParserTest {
                input: format!("a {this_op_s} b").as_bytes(),
                check: |session, expr| {
                    let expr = expr.unwrap();
                    assert_eq!(expr, make_binop_2(session, this_op, this_op_s.len()),);
                },
            }
            .run();

            ParserTest {
                input: format!("a {this_op_s} b {this_op_s} c").as_bytes(),
                check: |session, expr| {
                    let expr = expr.unwrap();
                    assert_eq!(
                        expr,
                        make_binop_3l(session, this_op, this_op_s.len(), this_op, this_op_s.len(),),
                    );
                },
            }
            .run();

            for prev_ops in OPS[..i].iter() {
                for &prev_op_s in prev_ops.iter() {
                    let prev_op = str_to_binop(prev_op_s);

                    ParserTest {
                        input: format!("a {this_op_s} b {prev_op_s} c").as_bytes(),
                        check: |session, expr| {
                            let expr = expr.unwrap();
                            assert_eq!(
                                expr,
                                make_binop_3l(
                                    session,
                                    this_op,
                                    this_op_s.len(),
                                    prev_op,
                                    prev_op_s.len(),
                                ),
                            );
                        },
                    }
                    .run();

                    ParserTest {
                        input: format!("a {prev_op_s} b {this_op_s} c").as_bytes(),
                        check: |session, expr| {
                            let expr = expr.unwrap();
                            assert_eq!(
                                expr,
                                make_binop_3r(
                                    session,
                                    prev_op,
                                    prev_op_s.len(),
                                    this_op,
                                    this_op_s.len(),
                                ),
                            );
                        },
                    }
                    .run();
                }
            }

            ParserTest {
                input: format!("+a {this_op_s} -b {this_op_s} !c").as_bytes(),
                check: |_, expr| {
                    let _ = expr.unwrap();
                },
            }
            .run();
        }
    }
}

#[test]
fn test_unary_expr() {
    ParserTest {
        input: b"+a",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Unary(
                    ast::UnaryOp::Plus,
                    session.arena.alloc(session.make_ident_expr("a", 1, 2)),
                ),
            );
        },
    }
    .run();

    ParserTest {
        input: b"-a",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Unary(
                    ast::UnaryOp::Minus,
                    session.arena.alloc(session.make_ident_expr("a", 1, 2)),
                ),
            );
        },
    }
    .run();

    ParserTest {
        input: b"~a",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Unary(
                    ast::UnaryOp::BitwiseNot,
                    session.arena.alloc(session.make_ident_expr("a", 1, 2)),
                ),
            );
        },
    }
    .run();

    ParserTest {
        input: b"!a",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Unary(
                    ast::UnaryOp::LogicNot,
                    session.arena.alloc(session.make_ident_expr("a", 1, 2)),
                ),
            );
        },
    }
    .run();

    ParserTest {
        input: b"!!a",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Unary(
                    ast::UnaryOp::LogicNot,
                    session.arena.alloc(ast::Expr {
                        kind: ast::ExprKind::Unary(
                            ast::UnaryOp::LogicNot,
                            session.arena.alloc(session.make_ident_expr("a", 2, 3)),
                        ),
                        span: session.intern_span(1, 3),
                    }),
                ),
            );
        },
    }
    .run();
}

#[test]
fn test_in_super_expr() {
    ParserTest {
        input: b"\"x\" in super",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::InSuper(
                    session.arena.alloc(ast::Expr {
                        kind: ast::ExprKind::String("x"),
                        span: session.intern_span(0, 3),
                    }),
                    session.intern_span(7, 12),
                ),
            );
        },
    }
    .run();

    ParserTest {
        input: b"\"x\" in super.a",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Binary(
                    session.arena.alloc(ast::Expr {
                        kind: ast::ExprKind::String("x"),
                        span: session.intern_span(0, 3),
                    }),
                    ast::BinaryOp::In,
                    session.arena.alloc(ast::Expr {
                        kind: ast::ExprKind::SuperField(
                            session.intern_span(7, 12),
                            session.make_ident("a", 13, 14),
                        ),
                        span: session.intern_span(7, 14),
                    }),
                )
            );
        },
    }
    .run();

    ParserTest {
        input: b"\"x\" in super[a]",
        check: |session, expr| {
            let expr = expr.unwrap();
            assert_eq!(
                expr.kind,
                ast::ExprKind::Binary(
                    session.arena.alloc(ast::Expr {
                        kind: ast::ExprKind::String("x"),
                        span: session.intern_span(0, 3),
                    }),
                    ast::BinaryOp::In,
                    session.arena.alloc(ast::Expr {
                        kind: ast::ExprKind::SuperIndex(
                            session.intern_span(7, 12),
                            session.arena.alloc(session.make_ident_expr("a", 13, 14)),
                        ),
                        span: session.intern_span(7, 15),
                    }),
                )
            );
        },
    }
    .run();
}
