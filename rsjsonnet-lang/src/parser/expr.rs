use super::{ExpectedThing, ParseError, Parser};
use crate::ast;
use crate::span::SpanId;
use crate::token::STokenKind;

impl Parser<'_> {
    pub(super) fn parse_expr(&mut self) -> Result<ast::Expr, ParseError> {
        self.parse_logic_or_expr()
    }

    fn parse_logic_or_expr(&mut self) -> Result<ast::Expr, ParseError> {
        let mut lhs = self.parse_logic_and_expr()?;
        loop {
            if self.eat_simple(STokenKind::PipePipe, false).is_some() {
                let rhs = self.parse_logic_and_expr()?;
                lhs = self.build_binary_expr(lhs, ast::BinaryOp::LogicOr, rhs);
            } else {
                self.expected_things.push(ExpectedThing::BinaryOp);
                return Ok(lhs);
            }
        }
    }

    fn parse_logic_and_expr(&mut self) -> Result<ast::Expr, ParseError> {
        let mut lhs = self.parse_bitwise_or_expr()?;
        loop {
            if self.eat_simple(STokenKind::AmpAmp, false).is_some() {
                let rhs = self.parse_bitwise_or_expr()?;
                lhs = self.build_binary_expr(lhs, ast::BinaryOp::LogicAnd, rhs);
            } else {
                self.expected_things.push(ExpectedThing::BinaryOp);
                return Ok(lhs);
            }
        }
    }

    fn parse_bitwise_or_expr(&mut self) -> Result<ast::Expr, ParseError> {
        let mut lhs = self.parse_bitwise_xor_expr()?;
        loop {
            if self.eat_simple(STokenKind::Pipe, false).is_some() {
                let rhs = self.parse_bitwise_xor_expr()?;
                lhs = self.build_binary_expr(lhs, ast::BinaryOp::BitwiseOr, rhs);
            } else {
                self.expected_things.push(ExpectedThing::BinaryOp);
                return Ok(lhs);
            }
        }
    }

    fn parse_bitwise_xor_expr(&mut self) -> Result<ast::Expr, ParseError> {
        let mut lhs = self.parse_bitwise_and_expr()?;
        loop {
            if self.eat_simple(STokenKind::Hat, false).is_some() {
                let rhs = self.parse_bitwise_and_expr()?;
                lhs = self.build_binary_expr(lhs, ast::BinaryOp::BitwiseXor, rhs);
            } else {
                self.expected_things.push(ExpectedThing::BinaryOp);
                return Ok(lhs);
            }
        }
    }

    fn parse_bitwise_and_expr(&mut self) -> Result<ast::Expr, ParseError> {
        let mut lhs = self.parse_eq_cmp_expr()?;
        loop {
            if self.eat_simple(STokenKind::Amp, false).is_some() {
                let rhs = self.parse_eq_cmp_expr()?;
                lhs = self.build_binary_expr(lhs, ast::BinaryOp::BitwiseAnd, rhs);
            } else {
                self.expected_things.push(ExpectedThing::BinaryOp);
                return Ok(lhs);
            }
        }
    }

    fn parse_eq_cmp_expr(&mut self) -> Result<ast::Expr, ParseError> {
        let mut lhs = self.parse_ord_cmp_expr()?;
        loop {
            if self.eat_simple(STokenKind::EqEq, false).is_some() {
                let rhs = self.parse_ord_cmp_expr()?;
                lhs = self.build_binary_expr(lhs, ast::BinaryOp::Eq, rhs);
            } else if self.eat_simple(STokenKind::ExclamEq, false).is_some() {
                let rhs = self.parse_ord_cmp_expr()?;
                lhs = self.build_binary_expr(lhs, ast::BinaryOp::Ne, rhs);
            } else {
                self.expected_things.push(ExpectedThing::BinaryOp);
                return Ok(lhs);
            }
        }
    }

    fn parse_ord_cmp_expr(&mut self) -> Result<ast::Expr, ParseError> {
        let mut lhs = self.parse_shift_expr()?;
        loop {
            if self.eat_simple(STokenKind::Lt, false).is_some() {
                let rhs = self.parse_shift_expr()?;
                lhs = self.build_binary_expr(lhs, ast::BinaryOp::Lt, rhs);
            } else if self.eat_simple(STokenKind::LtEq, false).is_some() {
                let rhs = self.parse_shift_expr()?;
                lhs = self.build_binary_expr(lhs, ast::BinaryOp::Le, rhs);
            } else if self.eat_simple(STokenKind::Gt, false).is_some() {
                let rhs = self.parse_shift_expr()?;
                lhs = self.build_binary_expr(lhs, ast::BinaryOp::Gt, rhs);
            } else if self.eat_simple(STokenKind::GtEq, false).is_some() {
                let rhs = self.parse_shift_expr()?;
                lhs = self.build_binary_expr(lhs, ast::BinaryOp::Ge, rhs);
            } else if self.eat_simple(STokenKind::In, false).is_some() {
                if self.peek_simple(STokenKind::Super, 0)
                    && !self.peek_simple(STokenKind::Dot, 1)
                    && !self.peek_simple(STokenKind::LeftBracket, 1)
                {
                    let super_span = self.eat_simple(STokenKind::Super, true).unwrap();
                    let span = self.span_mgr.make_surrounding_span(lhs.span, super_span);
                    lhs = ast::Expr {
                        kind: ast::ExprKind::InSuper(Box::new(lhs), super_span),
                        span,
                    };
                } else {
                    let rhs = self.parse_shift_expr()?;
                    lhs = self.build_binary_expr(lhs, ast::BinaryOp::In, rhs);
                }
            } else {
                self.expected_things.push(ExpectedThing::BinaryOp);
                return Ok(lhs);
            }
        }
    }

    fn parse_shift_expr(&mut self) -> Result<ast::Expr, ParseError> {
        let mut lhs = self.parse_add_expr()?;
        loop {
            if self.eat_simple(STokenKind::LtLt, false).is_some() {
                let rhs = self.parse_add_expr()?;
                lhs = self.build_binary_expr(lhs, ast::BinaryOp::Shl, rhs);
            } else if self.eat_simple(STokenKind::GtGt, false).is_some() {
                let rhs = self.parse_add_expr()?;
                lhs = self.build_binary_expr(lhs, ast::BinaryOp::Shr, rhs);
            } else {
                self.expected_things.push(ExpectedThing::BinaryOp);
                return Ok(lhs);
            }
        }
    }

    fn parse_add_expr(&mut self) -> Result<ast::Expr, ParseError> {
        let mut lhs = self.parse_mul_expr()?;
        loop {
            if self.eat_simple(STokenKind::Plus, false).is_some() {
                let rhs = self.parse_mul_expr()?;
                lhs = self.build_binary_expr(lhs, ast::BinaryOp::Add, rhs);
            } else if self.eat_simple(STokenKind::Minus, false).is_some() {
                let rhs = self.parse_mul_expr()?;
                lhs = self.build_binary_expr(lhs, ast::BinaryOp::Sub, rhs);
            } else {
                self.expected_things.push(ExpectedThing::BinaryOp);
                return Ok(lhs);
            }
        }
    }

    fn parse_mul_expr(&mut self) -> Result<ast::Expr, ParseError> {
        let mut lhs = self.parse_unary_expr()?;
        loop {
            if self.eat_simple(STokenKind::Asterisk, false).is_some() {
                let rhs = self.parse_unary_expr()?;
                lhs = self.build_binary_expr(lhs, ast::BinaryOp::Mul, rhs);
            } else if self.eat_simple(STokenKind::Slash, false).is_some() {
                let rhs = self.parse_unary_expr()?;
                lhs = self.build_binary_expr(lhs, ast::BinaryOp::Div, rhs);
            } else if self.eat_simple(STokenKind::Percent, false).is_some() {
                let rhs = self.parse_unary_expr()?;
                lhs = self.build_binary_expr(lhs, ast::BinaryOp::Rem, rhs);
            } else {
                self.expected_things.push(ExpectedThing::BinaryOp);
                return Ok(lhs);
            }
        }
    }

    fn parse_unary_expr(&mut self) -> Result<ast::Expr, ParseError> {
        if let Some(op_span) = self.eat_simple(STokenKind::Plus, false) {
            let rhs = self.parse_unary_expr()?;
            let span = self.span_mgr.make_surrounding_span(op_span, rhs.span);
            Ok(ast::Expr {
                kind: ast::ExprKind::Unary(ast::UnaryOp::Plus, Box::new(rhs)),
                span,
            })
        } else if let Some(op_span) = self.eat_simple(STokenKind::Minus, false) {
            let rhs = self.parse_unary_expr()?;
            let span = self.span_mgr.make_surrounding_span(op_span, rhs.span);
            Ok(ast::Expr {
                kind: ast::ExprKind::Unary(ast::UnaryOp::Minus, Box::new(rhs)),
                span,
            })
        } else if let Some(op_span) = self.eat_simple(STokenKind::Tilde, false) {
            let rhs = self.parse_unary_expr()?;
            let span = self.span_mgr.make_surrounding_span(op_span, rhs.span);
            Ok(ast::Expr {
                kind: ast::ExprKind::Unary(ast::UnaryOp::BitwiseNot, Box::new(rhs)),
                span,
            })
        } else if let Some(op_span) = self.eat_simple(STokenKind::Exclam, false) {
            let rhs = self.parse_unary_expr()?;
            let span = self.span_mgr.make_surrounding_span(op_span, rhs.span);
            Ok(ast::Expr {
                kind: ast::ExprKind::Unary(ast::UnaryOp::LogicNot, Box::new(rhs)),
                span,
            })
        } else {
            self.parse_suffix_expr()
        }
    }

    fn parse_suffix_expr(&mut self) -> Result<ast::Expr, ParseError> {
        let mut lhs = self.parse_primary_expr()?;
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

    fn parse_primary_expr(&mut self) -> Result<ast::Expr, ParseError> {
        if let Some(span) = self.eat_simple(STokenKind::Null, false) {
            Ok(ast::Expr {
                kind: ast::ExprKind::Null,
                span,
            })
        } else if let Some(span) = self.eat_simple(STokenKind::False, false) {
            Ok(ast::Expr {
                kind: ast::ExprKind::Bool(false),
                span,
            })
        } else if let Some(span) = self.eat_simple(STokenKind::True, false) {
            Ok(ast::Expr {
                kind: ast::ExprKind::Bool(true),
                span,
            })
        } else if let Some(span) = self.eat_simple(STokenKind::Self_, false) {
            Ok(ast::Expr {
                kind: ast::ExprKind::SelfObj,
                span,
            })
        } else if let Some(span) = self.eat_simple(STokenKind::Dollar, false) {
            Ok(ast::Expr {
                kind: ast::ExprKind::Dollar,
                span,
            })
        } else if let Some((s, span)) = self.eat_string(false) {
            Ok(ast::Expr {
                kind: ast::ExprKind::String(s),
                span,
            })
        } else if let Some((s, span)) = self.eat_text_block(false) {
            Ok(ast::Expr {
                kind: ast::ExprKind::TextBlock(s),
                span,
            })
        } else if let Some((n, span)) = self.eat_number(false) {
            Ok(ast::Expr {
                kind: ast::ExprKind::Number(n),
                span,
            })
        } else if let Some(start_span) = self.eat_simple(STokenKind::LeftBrace, false) {
            let (obj_inside, end_span) = self.parse_obj_inside()?;
            Ok(ast::Expr {
                kind: ast::ExprKind::Object(obj_inside),
                span: self.span_mgr.make_surrounding_span(start_span, end_span),
            })
        } else if let Some(start_span) = self.eat_simple(STokenKind::LeftBracket, false) {
            if let Some(end_span) = self.eat_simple(STokenKind::RightBracket, true) {
                return Ok(ast::Expr {
                    kind: ast::ExprKind::Array(Box::new([])),
                    span: self.span_mgr.make_surrounding_span(start_span, end_span),
                });
            }

            let item0 = self.parse_expr()?;
            let has_comma0 = self.eat_simple(STokenKind::Comma, true).is_some();
            if let Some(comp_spec) = self.maybe_parse_comp_spec()? {
                let end_span = self.expect_simple(STokenKind::RightBracket, true)?;
                return Ok(ast::Expr {
                    kind: ast::ExprKind::ArrayComp(Box::new(item0), comp_spec),
                    span: self.span_mgr.make_surrounding_span(start_span, end_span),
                });
            } else if let Some(end_span) = self.eat_simple(STokenKind::RightBracket, true) {
                return Ok(ast::Expr {
                    kind: ast::ExprKind::Array(Box::new([item0])),
                    span: self.span_mgr.make_surrounding_span(start_span, end_span),
                });
            } else if has_comma0 {
                let mut items = Vec::new();
                items.push(item0);
                let end_span = loop {
                    items.push(self.parse_expr()?);
                    if let Some(end_span) = self.eat_simple(STokenKind::RightBracket, true) {
                        break end_span;
                    } else if self.eat_simple(STokenKind::Comma, true).is_some() {
                        if let Some(end_span) = self.eat_simple(STokenKind::RightBracket, true) {
                            break end_span;
                        }
                    } else {
                        return Err(self.report_expected());
                    }
                };
                let items = items.into_boxed_slice();

                Ok(ast::Expr {
                    kind: ast::ExprKind::Array(items),
                    span: self.span_mgr.make_surrounding_span(start_span, end_span),
                })
            } else {
                return Err(self.report_expected());
            }
        } else if let Some(super_span) = self.eat_simple(STokenKind::Super, false) {
            if self.eat_simple(STokenKind::Dot, true).is_some() {
                let field_name = self.expect_ident(true)?;
                let span = self
                    .span_mgr
                    .make_surrounding_span(super_span, field_name.span);
                Ok(ast::Expr {
                    kind: ast::ExprKind::SuperField(super_span, field_name),
                    span,
                })
            } else if self.eat_simple(STokenKind::LeftBracket, true).is_some() {
                let index_expr = Box::new(self.parse_expr()?);
                let end_span = self.expect_simple(STokenKind::RightBracket, true)?;

                Ok(ast::Expr {
                    kind: ast::ExprKind::SuperIndex(super_span, index_expr),
                    span: self.span_mgr.make_surrounding_span(super_span, end_span),
                })
            } else {
                return Err(self.report_expected());
            }
        } else if let Some(ident) = self.eat_ident(false) {
            let span = ident.span;
            Ok(ast::Expr {
                kind: ast::ExprKind::Ident(ident),
                span,
            })
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
            Ok(ast::Expr {
                kind: ast::ExprKind::Local(binds, inner_expr),
                span,
            })
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
            Ok(ast::Expr {
                kind: ast::ExprKind::If(cond, then_body, else_body),
                span,
            })
        } else if let Some(start_span) = self.eat_simple(STokenKind::Function, false) {
            self.expect_simple(STokenKind::LeftParen, true)?;
            let (params, _) = self.parse_params()?;
            let params = params.into_boxed_slice();
            let body = Box::new(self.parse_expr()?);
            let span = self.span_mgr.make_surrounding_span(start_span, body.span);
            Ok(ast::Expr {
                kind: ast::ExprKind::Func(params, body),
                span,
            })
        } else if let Some((start_span, assert)) = self.maybe_parse_assert(false)? {
            self.expect_simple(STokenKind::Semicolon, true)?;
            let inner_expr = Box::new(self.parse_expr()?);
            let span = self
                .span_mgr
                .make_surrounding_span(start_span, inner_expr.span);
            Ok(ast::Expr {
                kind: ast::ExprKind::Assert(Box::new(assert), inner_expr),
                span,
            })
        } else if let Some(start_span) = self.eat_simple(STokenKind::Import, false) {
            let path_expr = Box::new(self.parse_expr()?);
            let span = self
                .span_mgr
                .make_surrounding_span(start_span, path_expr.span);
            Ok(ast::Expr {
                kind: ast::ExprKind::Import(path_expr),
                span,
            })
        } else if let Some(start_span) = self.eat_simple(STokenKind::Importstr, false) {
            let path_expr = Box::new(self.parse_expr()?);
            let span = self
                .span_mgr
                .make_surrounding_span(start_span, path_expr.span);
            Ok(ast::Expr {
                kind: ast::ExprKind::ImportStr(path_expr),
                span,
            })
        } else if let Some(start_span) = self.eat_simple(STokenKind::Importbin, false) {
            let path_expr = Box::new(self.parse_expr()?);
            let span = self
                .span_mgr
                .make_surrounding_span(start_span, path_expr.span);
            Ok(ast::Expr {
                kind: ast::ExprKind::ImportBin(path_expr),
                span,
            })
        } else if let Some(start_span) = self.eat_simple(STokenKind::Error, false) {
            let msg_expr = Box::new(self.parse_expr()?);
            let span = self
                .span_mgr
                .make_surrounding_span(start_span, msg_expr.span);
            Ok(ast::Expr {
                kind: ast::ExprKind::Error(msg_expr),
                span,
            })
        } else if let Some(start_span) = self.eat_simple(STokenKind::LeftParen, false) {
            let inner_expr = Box::new(self.parse_expr()?);
            let end_span = self.expect_simple(STokenKind::RightParen, true)?;
            Ok(ast::Expr {
                kind: ast::ExprKind::Paren(inner_expr),
                span: self.span_mgr.make_surrounding_span(start_span, end_span),
            })
        } else {
            self.expected_things.push(ExpectedThing::Expr);
            return Err(self.report_expected());
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
