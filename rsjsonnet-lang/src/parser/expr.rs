use super::{ExpectedThing, ParseError, Parser};
use crate::ast;
use crate::span::SpanId;
use crate::token::STokenKind;

impl Parser<'_> {
    pub(super) fn parse_expr(&mut self) -> Result<ast::Expr, ParseError> {
        #[derive(Copy, Clone)]
        enum BinOpKind {
            LogicOr,
            LogicAnd,
            BitwiseOr,
            BitwiseXor,
            BitwiseAnd,
            EqCmp,
            OrdCmp,
            Shift,
            Add,
            Mul,
        }

        enum State {
            Parsed(ast::Expr),
            Binary(BinOpKind),
            BinaryRhs(BinOpKind, ast::Expr),
            Unary,
            Primary,
        }

        enum StackItem {
            BinaryLhs(BinOpKind),
            BinaryRhs(BinOpKind, Box<ast::Expr>, ast::BinaryOp),
            Unary(ast::UnaryOp, SpanId),
            Suffix,
            ArrayItem0(SpanId),
            ArrayItemN(SpanId, Vec<ast::Expr>),
            Paren(SpanId),
        }

        impl BinOpKind {
            #[inline]
            fn next_state(self) -> State {
                match self {
                    BinOpKind::LogicOr => State::Binary(BinOpKind::LogicAnd),
                    BinOpKind::LogicAnd => State::Binary(BinOpKind::BitwiseOr),
                    BinOpKind::BitwiseOr => State::Binary(BinOpKind::BitwiseXor),
                    BinOpKind::BitwiseXor => State::Binary(BinOpKind::BitwiseAnd),
                    BinOpKind::BitwiseAnd => State::Binary(BinOpKind::EqCmp),
                    BinOpKind::EqCmp => State::Binary(BinOpKind::OrdCmp),
                    BinOpKind::OrdCmp => State::Binary(BinOpKind::Shift),
                    BinOpKind::Shift => State::Binary(BinOpKind::Add),
                    BinOpKind::Add => State::Binary(BinOpKind::Mul),
                    BinOpKind::Mul => State::Unary,
                }
            }
        }

        const INIT_STATE: State = State::Binary(BinOpKind::LogicOr);

        let mut stack = Vec::new();
        let mut state = INIT_STATE;

        loop {
            match state {
                State::Parsed(expr) => match stack.pop() {
                    None => return Ok(expr),
                    Some(StackItem::BinaryLhs(kind)) => {
                        state = State::BinaryRhs(kind, expr);
                    }
                    Some(StackItem::BinaryRhs(kind, lhs, op)) => {
                        let rhs = Box::new(expr);
                        let span = self.span_mgr.make_surrounding_span(lhs.span, rhs.span);
                        state = State::BinaryRhs(
                            kind,
                            ast::Expr {
                                kind: ast::ExprKind::Binary(lhs, op, rhs),
                                span,
                            },
                        );
                    }
                    Some(StackItem::Unary(op, op_span)) => {
                        let rhs = Box::new(expr);
                        let span = self.span_mgr.make_surrounding_span(op_span, rhs.span);
                        state = State::Parsed(ast::Expr {
                            kind: ast::ExprKind::Unary(op, rhs),
                            span,
                        });
                    }
                    Some(StackItem::Suffix) => {
                        state = State::Parsed(self.parse_suffix_expr(expr)?);
                    }
                    Some(StackItem::ArrayItem0(start_span)) => {
                        let item0 = expr;
                        let has_comma0 = self.eat_simple(STokenKind::Comma, true).is_some();
                        if let Some(comp_spec) = self.maybe_parse_comp_spec()? {
                            let end_span = self.expect_simple(STokenKind::RightBracket, true)?;
                            state = State::Parsed(ast::Expr {
                                kind: ast::ExprKind::ArrayComp(Box::new(item0), comp_spec),
                                span: self.span_mgr.make_surrounding_span(start_span, end_span),
                            });
                        } else if let Some(end_span) =
                            self.eat_simple(STokenKind::RightBracket, true)
                        {
                            state = State::Parsed(ast::Expr {
                                kind: ast::ExprKind::Array(Box::new([item0])),
                                span: self.span_mgr.make_surrounding_span(start_span, end_span),
                            });
                        } else if has_comma0 {
                            stack.push(StackItem::ArrayItemN(start_span, vec![item0]));
                            state = INIT_STATE;
                        } else {
                            return Err(self.report_expected());
                        }
                    }
                    Some(StackItem::ArrayItemN(start_span, mut items)) => {
                        items.push(expr);

                        let has_comma = self.eat_simple(STokenKind::Comma, true).is_some();
                        if let Some(end_span) = self.eat_simple(STokenKind::RightBracket, true) {
                            state = State::Parsed(ast::Expr {
                                kind: ast::ExprKind::Array(items.into_boxed_slice()),
                                span: self.span_mgr.make_surrounding_span(start_span, end_span),
                            });
                        } else if has_comma {
                            stack.push(StackItem::ArrayItemN(start_span, items));
                            state = INIT_STATE;
                        } else {
                            return Err(self.report_expected());
                        }
                    }
                    Some(StackItem::Paren(start_span)) => {
                        let end_span = self.expect_simple(STokenKind::RightParen, true)?;
                        state = State::Parsed(ast::Expr {
                            kind: ast::ExprKind::Paren(Box::new(expr)),
                            span: self.span_mgr.make_surrounding_span(start_span, end_span),
                        });
                    }
                },
                State::Binary(kind) => {
                    stack.push(StackItem::BinaryLhs(kind));
                    state = kind.next_state();
                }
                State::BinaryRhs(kind, lhs) => {
                    let op = match kind {
                        BinOpKind::LogicOr => self
                            .eat_simple(STokenKind::PipePipe, false)
                            .map(|_| ast::BinaryOp::LogicOr),
                        BinOpKind::LogicAnd => self
                            .eat_simple(STokenKind::AmpAmp, false)
                            .map(|_| ast::BinaryOp::LogicAnd),
                        BinOpKind::BitwiseOr => self
                            .eat_simple(STokenKind::Pipe, false)
                            .map(|_| ast::BinaryOp::BitwiseOr),
                        BinOpKind::BitwiseXor => self
                            .eat_simple(STokenKind::Hat, false)
                            .map(|_| ast::BinaryOp::BitwiseXor),
                        BinOpKind::BitwiseAnd => self
                            .eat_simple(STokenKind::Amp, false)
                            .map(|_| ast::BinaryOp::BitwiseAnd),
                        BinOpKind::EqCmp => {
                            if self.eat_simple(STokenKind::EqEq, false).is_some() {
                                Some(ast::BinaryOp::Eq)
                            } else if self.eat_simple(STokenKind::ExclamEq, false).is_some() {
                                Some(ast::BinaryOp::Ne)
                            } else {
                                None
                            }
                        }
                        BinOpKind::OrdCmp => {
                            if self.eat_simple(STokenKind::Lt, false).is_some() {
                                Some(ast::BinaryOp::Lt)
                            } else if self.eat_simple(STokenKind::LtEq, false).is_some() {
                                Some(ast::BinaryOp::Le)
                            } else if self.eat_simple(STokenKind::Gt, false).is_some() {
                                Some(ast::BinaryOp::Gt)
                            } else if self.eat_simple(STokenKind::GtEq, false).is_some() {
                                Some(ast::BinaryOp::Ge)
                            } else if self.eat_simple(STokenKind::In, false).is_some() {
                                if self.peek_simple(STokenKind::Super, 0)
                                    && !self.peek_simple(STokenKind::Dot, 1)
                                    && !self.peek_simple(STokenKind::LeftBracket, 1)
                                {
                                    let super_span =
                                        self.eat_simple(STokenKind::Super, true).unwrap();
                                    let span =
                                        self.span_mgr.make_surrounding_span(lhs.span, super_span);
                                    state = State::BinaryRhs(
                                        kind,
                                        ast::Expr {
                                            kind: ast::ExprKind::InSuper(Box::new(lhs), super_span),
                                            span,
                                        },
                                    );
                                    continue;
                                } else {
                                    Some(ast::BinaryOp::In)
                                }
                            } else {
                                None
                            }
                        }
                        BinOpKind::Shift => {
                            if self.eat_simple(STokenKind::LtLt, false).is_some() {
                                Some(ast::BinaryOp::Shl)
                            } else if self.eat_simple(STokenKind::GtGt, false).is_some() {
                                Some(ast::BinaryOp::Shr)
                            } else {
                                None
                            }
                        }
                        BinOpKind::Add => {
                            if self.eat_simple(STokenKind::Plus, false).is_some() {
                                Some(ast::BinaryOp::Add)
                            } else if self.eat_simple(STokenKind::Minus, false).is_some() {
                                Some(ast::BinaryOp::Sub)
                            } else {
                                None
                            }
                        }
                        BinOpKind::Mul => {
                            if self.eat_simple(STokenKind::Asterisk, false).is_some() {
                                Some(ast::BinaryOp::Mul)
                            } else if self.eat_simple(STokenKind::Slash, false).is_some() {
                                Some(ast::BinaryOp::Div)
                            } else if self.eat_simple(STokenKind::Percent, false).is_some() {
                                Some(ast::BinaryOp::Rem)
                            } else {
                                None
                            }
                        }
                    };

                    if let Some(op) = op {
                        stack.push(StackItem::BinaryRhs(kind, Box::new(lhs), op));
                        state = kind.next_state();
                    } else {
                        self.expected_things.push(ExpectedThing::BinaryOp);
                        state = State::Parsed(lhs);
                    }
                }
                State::Unary => {
                    if let Some(op_span) = self.eat_simple(STokenKind::Plus, false) {
                        stack.push(StackItem::Unary(ast::UnaryOp::Plus, op_span));
                    } else if let Some(op_span) = self.eat_simple(STokenKind::Minus, false) {
                        stack.push(StackItem::Unary(ast::UnaryOp::Minus, op_span));
                    } else if let Some(op_span) = self.eat_simple(STokenKind::Tilde, false) {
                        stack.push(StackItem::Unary(ast::UnaryOp::BitwiseNot, op_span));
                    } else if let Some(op_span) = self.eat_simple(STokenKind::Exclam, false) {
                        stack.push(StackItem::Unary(ast::UnaryOp::LogicNot, op_span));
                    } else {
                        stack.push(StackItem::Suffix);
                        state = State::Primary;
                    }
                }
                State::Primary => {
                    if let Some(expr) = self.parse_maybe_simple_expr() {
                        state = State::Parsed(expr);
                    } else if let Some(start_span) = self.eat_simple(STokenKind::LeftBrace, false) {
                        let (obj_inside, end_span) = self.parse_obj_inside()?;
                        state = State::Parsed(ast::Expr {
                            kind: ast::ExprKind::Object(obj_inside),
                            span: self.span_mgr.make_surrounding_span(start_span, end_span),
                        });
                    } else if let Some(start_span) = self.eat_simple(STokenKind::LeftBracket, false)
                    {
                        if let Some(end_span) = self.eat_simple(STokenKind::RightBracket, true) {
                            state = State::Parsed(ast::Expr {
                                kind: ast::ExprKind::Array(Box::new([])),
                                span: self.span_mgr.make_surrounding_span(start_span, end_span),
                            });
                            continue;
                        }

                        stack.push(StackItem::ArrayItem0(start_span));
                        state = INIT_STATE;
                    } else if let Some(super_span) = self.eat_simple(STokenKind::Super, false) {
                        if self.eat_simple(STokenKind::Dot, true).is_some() {
                            let field_name = self.expect_ident(true)?;
                            let span = self
                                .span_mgr
                                .make_surrounding_span(super_span, field_name.span);
                            state = State::Parsed(ast::Expr {
                                kind: ast::ExprKind::SuperField(super_span, field_name),
                                span,
                            });
                        } else if self.eat_simple(STokenKind::LeftBracket, true).is_some() {
                            let index_expr = Box::new(self.parse_expr()?);
                            let end_span = self.expect_simple(STokenKind::RightBracket, true)?;

                            state = State::Parsed(ast::Expr {
                                kind: ast::ExprKind::SuperIndex(super_span, index_expr),
                                span: self.span_mgr.make_surrounding_span(super_span, end_span),
                            });
                        } else {
                            return Err(self.report_expected());
                        }
                    } else if let Some(start_span) = self.eat_simple(STokenKind::Local, false) {
                        let mut binds = Vec::new();
                        binds.push(self.parse_bind()?);
                        while self.eat_simple(STokenKind::Comma, true).is_some() {
                            binds.push(self.parse_bind()?);
                        }
                        let binds = binds.into_boxed_slice();

                        self.expect_simple(STokenKind::Semicolon, true)?;
                        let inner_expr = Box::new(self.parse_expr()?);

                        let span = self
                            .span_mgr
                            .make_surrounding_span(start_span, inner_expr.span);
                        state = State::Parsed(ast::Expr {
                            kind: ast::ExprKind::Local(binds, inner_expr),
                            span,
                        });
                    } else if let Some(if_span) = self.eat_simple(STokenKind::If, false) {
                        let cond = Box::new(self.parse_expr()?);
                        self.expect_simple(STokenKind::Then, true)?;
                        let then_body = Box::new(self.parse_expr()?);
                        let else_body = if self.eat_simple(STokenKind::Else, true).is_some() {
                            Some(Box::new(self.parse_expr()?))
                        } else {
                            None
                        };

                        let span = self.span_mgr.make_surrounding_span(
                            if_span,
                            else_body.as_ref().map_or(then_body.span, |e| e.span),
                        );
                        state = State::Parsed(ast::Expr {
                            kind: ast::ExprKind::If(cond, then_body, else_body),
                            span,
                        });
                    } else if let Some(start_span) = self.eat_simple(STokenKind::Function, false) {
                        self.expect_simple(STokenKind::LeftParen, true)?;
                        let (params, _) = self.parse_params()?;
                        let params = params.into_boxed_slice();
                        let body = Box::new(self.parse_expr()?);
                        let span = self.span_mgr.make_surrounding_span(start_span, body.span);
                        state = State::Parsed(ast::Expr {
                            kind: ast::ExprKind::Func(params, body),
                            span,
                        });
                    } else if let Some((start_span, assert)) = self.maybe_parse_assert(false)? {
                        self.expect_simple(STokenKind::Semicolon, true)?;
                        let inner_expr = Box::new(self.parse_expr()?);
                        let span = self
                            .span_mgr
                            .make_surrounding_span(start_span, inner_expr.span);
                        state = State::Parsed(ast::Expr {
                            kind: ast::ExprKind::Assert(Box::new(assert), inner_expr),
                            span,
                        });
                    } else if let Some(start_span) = self.eat_simple(STokenKind::Import, false) {
                        let path_expr = Box::new(self.parse_expr()?);
                        let span = self
                            .span_mgr
                            .make_surrounding_span(start_span, path_expr.span);
                        state = State::Parsed(ast::Expr {
                            kind: ast::ExprKind::Import(path_expr),
                            span,
                        });
                    } else if let Some(start_span) = self.eat_simple(STokenKind::Importstr, false) {
                        let path_expr = Box::new(self.parse_expr()?);
                        let span = self
                            .span_mgr
                            .make_surrounding_span(start_span, path_expr.span);
                        state = State::Parsed(ast::Expr {
                            kind: ast::ExprKind::ImportStr(path_expr),
                            span,
                        });
                    } else if let Some(start_span) = self.eat_simple(STokenKind::Importbin, false) {
                        let path_expr = Box::new(self.parse_expr()?);
                        let span = self
                            .span_mgr
                            .make_surrounding_span(start_span, path_expr.span);
                        state = State::Parsed(ast::Expr {
                            kind: ast::ExprKind::ImportBin(path_expr),
                            span,
                        });
                    } else if let Some(start_span) = self.eat_simple(STokenKind::Error, false) {
                        let msg_expr = Box::new(self.parse_expr()?);
                        let span = self
                            .span_mgr
                            .make_surrounding_span(start_span, msg_expr.span);
                        state = State::Parsed(ast::Expr {
                            kind: ast::ExprKind::Error(msg_expr),
                            span,
                        });
                    } else if let Some(start_span) = self.eat_simple(STokenKind::LeftParen, false) {
                        stack.push(StackItem::Paren(start_span));
                        state = INIT_STATE;
                    } else {
                        self.expected_things.push(ExpectedThing::Expr);
                        return Err(self.report_expected());
                    }
                }
            }
        }
    }

    fn parse_suffix_expr(&mut self, primary: ast::Expr) -> Result<ast::Expr, ParseError> {
        let mut lhs = primary;
        loop {
            if self.eat_simple(STokenKind::Dot, true).is_some() {
                let field_name = self.expect_ident(true)?;
                let span = self
                    .span_mgr
                    .make_surrounding_span(lhs.span, field_name.span);
                lhs = ast::Expr {
                    kind: ast::ExprKind::Field(Box::new(lhs), field_name),
                    span,
                };
            } else if self.eat_simple(STokenKind::LeftBracket, true).is_some() {
                lhs = self.parse_index_expr(lhs)?;
            } else if self.eat_simple(STokenKind::LeftParen, true).is_some() {
                let (args, end_span): (Box<[_]>, _) =
                    if let Some(end_span) = self.eat_simple(STokenKind::RightParen, true) {
                        (Box::new([]), end_span)
                    } else {
                        let (args, end_span) = self.parse_args()?;
                        (args.into_boxed_slice(), end_span)
                    };

                let tailstrict_span = self.eat_simple(STokenKind::Tailstrict, true);
                let span = self
                    .span_mgr
                    .make_surrounding_span(lhs.span, tailstrict_span.unwrap_or(end_span));
                lhs = ast::Expr {
                    kind: ast::ExprKind::Call(Box::new(lhs), args, tailstrict_span.is_some()),
                    span,
                };
            } else if let Some(obj_start_span) = self.eat_simple(STokenKind::LeftBrace, true) {
                let (obj_inside, obj_end_span) = self.parse_obj_inside()?;
                let span = self.span_mgr.make_surrounding_span(lhs.span, obj_end_span);
                lhs = ast::Expr {
                    kind: ast::ExprKind::ObjExt(
                        Box::new(lhs),
                        obj_inside,
                        self.span_mgr
                            .make_surrounding_span(obj_start_span, obj_end_span),
                    ),
                    span,
                };
            } else {
                return Ok(lhs);
            }
        }
    }

    fn parse_index_expr(&mut self, lhs: ast::Expr) -> Result<ast::Expr, ParseError> {
        let mut index1 = None;
        let mut index2 = None;
        let mut index3 = None;

        let end_span = if self.eat_simple(STokenKind::Colon, true).is_some() {
            if let Some(end_span) = self.eat_simple(STokenKind::RightBracket, true) {
                end_span
            } else if self.eat_simple(STokenKind::Colon, true).is_some() {
                if let Some(end_span) = self.eat_simple(STokenKind::RightBracket, true) {
                    end_span
                } else {
                    index3 = Some(Box::new(self.parse_expr()?));
                    self.expect_simple(STokenKind::RightBracket, true)?
                }
            } else {
                index2 = Some(Box::new(self.parse_expr()?));
                if let Some(end_span) = self.eat_simple(STokenKind::RightBracket, true) {
                    end_span
                } else if self.eat_simple(STokenKind::Colon, true).is_some() {
                    if let Some(end_span) = self.eat_simple(STokenKind::RightBracket, true) {
                        end_span
                    } else {
                        index3 = Some(Box::new(self.parse_expr()?));
                        self.expect_simple(STokenKind::RightBracket, true)?
                    }
                } else {
                    return Err(self.report_expected());
                }
            }
        } else if self.eat_simple(STokenKind::ColonColon, true).is_some() {
            if let Some(end_span) = self.eat_simple(STokenKind::RightBracket, true) {
                end_span
            } else {
                index3 = Some(Box::new(self.parse_expr()?));
                self.expect_simple(STokenKind::RightBracket, true)?
            }
        } else {
            index1 = Some(Box::new(self.parse_expr()?));
            if let Some(end_span) = self.eat_simple(STokenKind::RightBracket, true) {
                let span = self.span_mgr.make_surrounding_span(lhs.span, end_span);
                return Ok(ast::Expr {
                    kind: ast::ExprKind::Index(Box::new(lhs), index1.unwrap()),
                    span,
                });
            } else if self.eat_simple(STokenKind::Colon, true).is_some() {
                if let Some(end_span) = self.eat_simple(STokenKind::RightBracket, true) {
                    end_span
                } else if self.eat_simple(STokenKind::Colon, true).is_some() {
                    if let Some(end_span) = self.eat_simple(STokenKind::RightBracket, true) {
                        end_span
                    } else {
                        index3 = Some(Box::new(self.parse_expr()?));
                        self.expect_simple(STokenKind::RightBracket, true)?
                    }
                } else {
                    index2 = Some(Box::new(self.parse_expr()?));
                    if let Some(end_span) = self.eat_simple(STokenKind::RightBracket, true) {
                        end_span
                    } else if self.eat_simple(STokenKind::Colon, true).is_some() {
                        if let Some(end_span) = self.eat_simple(STokenKind::RightBracket, true) {
                            end_span
                        } else {
                            index3 = Some(Box::new(self.parse_expr()?));
                            self.expect_simple(STokenKind::RightBracket, true)?
                        }
                    } else {
                        return Err(self.report_expected());
                    }
                }
            } else if self.eat_simple(STokenKind::ColonColon, true).is_some() {
                if let Some(end_span) = self.eat_simple(STokenKind::RightBracket, true) {
                    end_span
                } else {
                    index3 = Some(Box::new(self.parse_expr()?));
                    self.expect_simple(STokenKind::RightBracket, true)?
                }
            } else {
                return Err(self.report_expected());
            }
        };

        let span = self.span_mgr.make_surrounding_span(lhs.span, end_span);
        Ok(ast::Expr {
            kind: ast::ExprKind::Slice(Box::new(lhs), index1, index2, index3),
            span,
        })
    }

    fn parse_maybe_simple_expr(&mut self) -> Option<ast::Expr> {
        if let Some(span) = self.eat_simple(STokenKind::Null, false) {
            Some(ast::Expr {
                kind: ast::ExprKind::Null,
                span,
            })
        } else if let Some(span) = self.eat_simple(STokenKind::False, false) {
            Some(ast::Expr {
                kind: ast::ExprKind::Bool(false),
                span,
            })
        } else if let Some(span) = self.eat_simple(STokenKind::True, false) {
            Some(ast::Expr {
                kind: ast::ExprKind::Bool(true),
                span,
            })
        } else if let Some(span) = self.eat_simple(STokenKind::Self_, false) {
            Some(ast::Expr {
                kind: ast::ExprKind::SelfObj,
                span,
            })
        } else if let Some(span) = self.eat_simple(STokenKind::Dollar, false) {
            Some(ast::Expr {
                kind: ast::ExprKind::Dollar,
                span,
            })
        } else if let Some((s, span)) = self.eat_string(false) {
            Some(ast::Expr {
                kind: ast::ExprKind::String(s),
                span,
            })
        } else if let Some((s, span)) = self.eat_text_block(false) {
            Some(ast::Expr {
                kind: ast::ExprKind::TextBlock(s),
                span,
            })
        } else if let Some((n, span)) = self.eat_number(false) {
            Some(ast::Expr {
                kind: ast::ExprKind::Number(n),
                span,
            })
        } else if let Some(ident) = self.eat_ident(false) {
            let span = ident.span;
            Some(ast::Expr {
                kind: ast::ExprKind::Ident(ident),
                span,
            })
        } else {
            None
        }
    }

    fn maybe_parse_assert(
        &mut self,
        add_to_expected: bool,
    ) -> Result<Option<(SpanId, ast::Assert)>, ParseError> {
        if let Some(start_span) = self.eat_simple(STokenKind::Assert, add_to_expected) {
            let cond = self.parse_expr()?;
            let msg = if self.eat_simple(STokenKind::Colon, true).is_some() {
                Some(self.parse_expr()?)
            } else {
                None
            };
            let end_span = msg.as_ref().map_or(cond.span, |e| e.span);
            let span = self.span_mgr.make_surrounding_span(start_span, end_span);
            Ok(Some((start_span, ast::Assert { cond, msg, span })))
        } else {
            Ok(None)
        }
    }

    fn parse_bind(&mut self) -> Result<ast::Bind, ParseError> {
        let name = self.expect_ident(true)?;
        let params = if let Some(params_start_span) = self.eat_simple(STokenKind::LeftParen, true) {
            let (params, params_end_span) = self.parse_params()?;
            Some((
                params.into_boxed_slice(),
                self.span_mgr
                    .make_surrounding_span(params_start_span, params_end_span),
            ))
        } else {
            None
        };
        self.expect_simple(STokenKind::Eq, true)?;
        let value = self.parse_expr()?;

        Ok(ast::Bind {
            name,
            params,
            value,
        })
    }

    fn parse_obj_inside(&mut self) -> Result<(ast::ObjInside, SpanId), ParseError> {
        if let Some(end_span) = self.eat_simple(STokenKind::RightBrace, true) {
            return Ok((ast::ObjInside::Members(Box::new([])), end_span));
        }

        fn make_comp(
            members: Vec<ast::Member>,
            comp_spec: Box<[ast::CompSpecPart]>,
        ) -> ast::ObjInside {
            let mut locals1 = Vec::new();
            let mut locals2 = Vec::new();
            let mut field = None;
            for member in members {
                match member {
                    ast::Member::Local(local) => {
                        if field.is_none() {
                            locals1.push(local);
                        } else {
                            locals2.push(local);
                        }
                    }
                    ast::Member::Assert(_) => unreachable!(),
                    ast::Member::Field(ast::Field::Value(
                        ast::FieldName::Expr(name, _),
                        false,
                        ast::Visibility::Default,
                        body,
                    )) => {
                        assert!(field.is_none());
                        field = Some((Box::new(name), Box::new(body)));
                    }
                    ast::Member::Field(_) => unreachable!(),
                }
            }

            let (name, body) = field.unwrap();

            ast::ObjInside::Comp {
                locals1: locals1.into_boxed_slice(),
                name,
                body,
                locals2: locals2.into_boxed_slice(),
                comp_spec,
            }
        }

        let mut members = Vec::new();
        let mut can_be_comp = true;
        let mut has_comp_dyn_field = false;
        loop {
            if let Some(local) = self.maybe_parse_obj_local()? {
                members.push(ast::Member::Local(local));
            } else if let Some(field) = self.maybe_parse_field()? {
                match field {
                    ast::Field::Value(
                        ast::FieldName::Expr(..),
                        false,
                        ast::Visibility::Default,
                        _,
                    ) => {
                        if has_comp_dyn_field {
                            can_be_comp = false;
                        } else {
                            has_comp_dyn_field = true;
                        }
                    }
                    _ => can_be_comp = false,
                }
                members.push(ast::Member::Field(field));
            } else if let Some((_, assert)) = self.maybe_parse_assert(true)? {
                can_be_comp = false;
                members.push(ast::Member::Assert(assert));
            } else {
                return Err(self.report_expected());
            }

            if let Some(end_span) = self.eat_simple(STokenKind::RightBrace, true) {
                return Ok((
                    ast::ObjInside::Members(members.into_boxed_slice()),
                    end_span,
                ));
            } else if self.eat_simple(STokenKind::Comma, true).is_some() {
                if let Some(end_span) = self.eat_simple(STokenKind::RightBrace, true) {
                    return Ok((
                        ast::ObjInside::Members(members.into_boxed_slice()),
                        end_span,
                    ));
                } else if can_be_comp && has_comp_dyn_field {
                    if let Some(comp_spec) = self.maybe_parse_comp_spec()? {
                        let end_span = self.expect_simple(STokenKind::RightBrace, true)?;
                        return Ok((make_comp(members, comp_spec), end_span));
                    }
                }
            } else if can_be_comp && has_comp_dyn_field {
                if let Some(comp_spec) = self.maybe_parse_comp_spec()? {
                    let end_span = self.expect_simple(STokenKind::RightBrace, true)?;
                    return Ok((make_comp(members, comp_spec), end_span));
                } else {
                    return Err(self.report_expected());
                }
            } else {
                return Err(self.report_expected());
            }
        }
    }

    fn maybe_parse_field(&mut self) -> Result<Option<ast::Field>, ParseError> {
        if let Some(name) = self.maybe_parse_field_name()? {
            if let Some(params_start_span) = self.eat_simple(STokenKind::LeftParen, true) {
                let (params, params_end_span) = self.parse_params()?;
                let Some(visibility) = self.eat_visibility(true) else {
                    return Err(self.report_expected());
                };
                let params = params.into_boxed_slice();
                let value = self.parse_expr()?;
                Ok(Some(ast::Field::Func(
                    name,
                    params,
                    self.span_mgr
                        .make_surrounding_span(params_start_span, params_end_span),
                    visibility,
                    value,
                )))
            } else {
                let Some((plus, visibility)) = self.eat_plus_visibility(true) else {
                    return Err(self.report_expected());
                };
                let value = self.parse_expr()?;
                Ok(Some(ast::Field::Value(name, plus, visibility, value)))
            }
        } else {
            Ok(None)
        }
    }

    fn maybe_parse_obj_local(&mut self) -> Result<Option<ast::ObjLocal>, ParseError> {
        if self.eat_simple(STokenKind::Local, true).is_some() {
            let bind = self.parse_bind()?;
            Ok(Some(ast::ObjLocal { bind }))
        } else {
            Ok(None)
        }
    }

    fn maybe_parse_comp_spec(&mut self) -> Result<Option<Box<[ast::CompSpecPart]>>, ParseError> {
        if let Some(for_spec) = self.maybe_parse_for_spec()? {
            let mut parts = Vec::new();
            parts.push(ast::CompSpecPart::For(for_spec));
            loop {
                if let Some(for_spec) = self.maybe_parse_for_spec()? {
                    parts.push(ast::CompSpecPart::For(for_spec));
                } else if let Some(if_spec) = self.maybe_parse_if_spec()? {
                    parts.push(ast::CompSpecPart::If(if_spec));
                } else {
                    break;
                }
            }
            Ok(Some(parts.into_boxed_slice()))
        } else {
            Ok(None)
        }
    }

    fn maybe_parse_for_spec(&mut self) -> Result<Option<ast::ForSpec>, ParseError> {
        if self.eat_simple(STokenKind::For, true).is_some() {
            let var = self.expect_ident(true)?;
            self.expect_simple(STokenKind::In, true)?;
            let inner = self.parse_expr()?;
            Ok(Some(ast::ForSpec { var, inner }))
        } else {
            Ok(None)
        }
    }

    fn maybe_parse_if_spec(&mut self) -> Result<Option<ast::IfSpec>, ParseError> {
        if self.eat_simple(STokenKind::If, true).is_some() {
            let cond = self.parse_expr()?;
            Ok(Some(ast::IfSpec { cond }))
        } else {
            Ok(None)
        }
    }

    fn maybe_parse_field_name(&mut self) -> Result<Option<ast::FieldName>, ParseError> {
        if let Some(ident) = self.eat_ident(true) {
            Ok(Some(ast::FieldName::Ident(ident)))
        } else if let Some((s, span)) = self.eat_string(true) {
            let interned = self.str_interner.intern(&s);
            Ok(Some(ast::FieldName::String(interned, span)))
        } else if let Some((s, span)) = self.eat_text_block(true) {
            let interned = self.str_interner.intern(&s);
            Ok(Some(ast::FieldName::String(interned, span)))
        } else if let Some(start_span) = self.eat_simple(STokenKind::LeftBracket, true) {
            let name_expr = self.parse_expr()?;
            let end_span = self.expect_simple(STokenKind::RightBracket, true)?;
            Ok(Some(ast::FieldName::Expr(
                name_expr,
                self.span_mgr.make_surrounding_span(start_span, end_span),
            )))
        } else {
            Ok(None)
        }
    }

    fn parse_args(&mut self) -> Result<(Vec<ast::Arg>, SpanId), ParseError> {
        if let Some(end_span) = self.eat_simple(STokenKind::RightParen, true) {
            return Ok((vec![], end_span));
        }

        let mut args = Vec::new();
        let end_span = loop {
            args.push(self.parse_arg()?);

            if let Some(end_span) = self.eat_simple(STokenKind::RightParen, true) {
                break end_span;
            } else if self.eat_simple(STokenKind::Comma, true).is_some() {
                if let Some(end_span) = self.eat_simple(STokenKind::RightParen, true) {
                    break end_span;
                }
            } else {
                return Err(self.report_expected());
            }
        };

        Ok((args, end_span))
    }

    fn parse_arg(&mut self) -> Result<ast::Arg, ParseError> {
        if self.peek_ident(0) && self.peek_simple(STokenKind::Eq, 1) {
            let name = self.eat_ident(false).unwrap();
            self.eat_simple(STokenKind::Eq, false).unwrap();

            let value = self.parse_expr()?;

            Ok(ast::Arg::Named(name, value))
        } else {
            let value = self.parse_expr()?;
            Ok(ast::Arg::Positional(value))
        }
    }

    fn parse_params(&mut self) -> Result<(Vec<ast::Param>, SpanId), ParseError> {
        if let Some(end_span) = self.eat_simple(STokenKind::RightParen, true) {
            return Ok((vec![], end_span));
        }

        let mut params = Vec::new();
        let end_span = loop {
            let name = self.expect_ident(true)?;
            let default_value = if self.eat_simple(STokenKind::Eq, true).is_some() {
                Some(self.parse_expr()?)
            } else {
                None
            };
            params.push(ast::Param {
                name,
                default_value,
            });

            if let Some(end_span) = self.eat_simple(STokenKind::RightParen, true) {
                break end_span;
            } else if self.eat_simple(STokenKind::Comma, true).is_some() {
                if let Some(end_span) = self.eat_simple(STokenKind::RightParen, true) {
                    break end_span;
                }
            } else {
                return Err(self.report_expected());
            }
        };

        Ok((params, end_span))
    }
}
