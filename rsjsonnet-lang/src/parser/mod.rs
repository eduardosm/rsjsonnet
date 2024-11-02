//! A parser for the Jsonnet language.
//!
//! # Example
//!
//! ```
//! let source = b"local add_one(x) = x + 1; add_one(2)";
//!
//! let arena = rsjsonnet_lang::arena::Arena::new();
//! let ast_arena = rsjsonnet_lang::arena::Arena::new();
//! let str_interner = rsjsonnet_lang::interner::StrInterner::new();
//! let mut span_mgr = rsjsonnet_lang::span::SpanManager::new();
//! let (span_ctx, _) = span_mgr.insert_source_context(source.len());
//!
//! // First, lex the source excluding whitespace and comments.
//! let lexer = rsjsonnet_lang::lexer::Lexer::new(
//!     &arena,
//!     &ast_arena,
//!     &str_interner,
//!     &mut span_mgr,
//!     span_ctx,
//!     source,
//! );
//! let tokens = lexer.lex_to_eof(false).unwrap();
//!
//! // Create the parser.
//! let parser = rsjsonnet_lang::parser::Parser::new(
//!     &arena,
//!     &ast_arena,
//!     &str_interner,
//!     &mut span_mgr,
//!     tokens,
//! );
//!
//! // Parse the source.
//! let ast_root = parser.parse_root_expr().unwrap();
//! ```

use crate::arena::Arena;
use crate::ast;
use crate::interner::StrInterner;
use crate::span::{SpanId, SpanManager};
use crate::token::{Number, STokenKind, Token, TokenKind};

mod error;
mod expr;

pub use error::{ActualToken, ExpectedToken, ParseError};

pub struct Parser<'a, 'p, 'ast> {
    arena: &'p Arena,
    ast_arena: &'ast Arena,
    str_interner: &'a StrInterner<'p>,
    span_mgr: &'a mut SpanManager,
    curr_token: Token<'p, 'ast>,
    rem_tokens: std::vec::IntoIter<Token<'p, 'ast>>,
    expected_things: Vec<ExpectedToken>,
}

impl<'a, 'p, 'ast> Parser<'a, 'p, 'ast> {
    /// Creates a new parser that operates on `tokens`.
    ///
    /// `arena` is used to allocate interned strings and `ast_arena` is used to
    /// allocate most of AST nodes. This allows to free most of the AST while
    /// keeping interned strings.
    ///
    /// `tokens` must not include [whitespace](TokenKind::Whitespace)
    /// or [comments](TokenKind::Comment) and must end with an
    /// [end-of-file](TokenKind::EndOfFile) token.
    pub fn new(
        arena: &'p Arena,
        ast_arena: &'ast Arena,
        str_interner: &'a StrInterner<'p>,
        span_mgr: &'a mut SpanManager,
        tokens: Vec<Token<'p, 'ast>>,
    ) -> Self {
        let mut token_iter = tokens.into_iter();
        let first_token = token_iter.next().expect("passed an empty token slice");
        Self {
            arena,
            ast_arena,
            str_interner,
            span_mgr,
            curr_token: first_token,
            rem_tokens: token_iter,
            expected_things: Vec::new(),
        }
    }

    fn next_token(&mut self) -> Token<'p, 'ast> {
        let next_token = self.rem_tokens.next().unwrap();
        let token = std::mem::replace(&mut self.curr_token, next_token);

        self.expected_things.clear();

        token
    }

    #[cold]
    #[must_use]
    fn report_expected(&mut self) -> ParseError {
        ParseError::Expected {
            span: self.curr_token.span,
            expected: self.expected_things.drain(..).collect(),
            instead: ActualToken::from_token_kind(&self.curr_token.kind),
        }
    }

    #[inline]
    fn eat_eof(&mut self, add_to_expected: bool) -> bool {
        if matches!(self.curr_token.kind, TokenKind::EndOfFile) {
            assert!(self.rem_tokens.as_slice().is_empty());
            self.expected_things.clear();
            true
        } else {
            if add_to_expected {
                self.expected_things.push(ExpectedToken::EndOfFile);
            }
            false
        }
    }

    #[inline]
    fn eat_simple(&mut self, kind: STokenKind, add_to_expected: bool) -> Option<SpanId> {
        if matches!(self.curr_token.kind, TokenKind::Simple(k) if k == kind) {
            let span = self.curr_token.span;
            self.next_token();
            Some(span)
        } else {
            if add_to_expected {
                self.expected_things.push(ExpectedToken::Simple(kind));
            }
            None
        }
    }

    #[inline]
    fn expect_simple(
        &mut self,
        kind: STokenKind,
        add_to_expected: bool,
    ) -> Result<SpanId, ParseError> {
        if let Some(span) = self.eat_simple(kind, add_to_expected) {
            Ok(span)
        } else {
            Err(self.report_expected())
        }
    }

    #[inline]
    fn eat_ident(&mut self, add_to_expected: bool) -> Option<ast::Ident<'p>> {
        if let TokenKind::Ident(value) = self.curr_token.kind {
            let span = self.curr_token.span;
            self.next_token();
            Some(ast::Ident { value, span })
        } else {
            if add_to_expected {
                self.expected_things.push(ExpectedToken::Ident);
            }
            None
        }
    }

    #[inline]
    fn expect_ident(&mut self, add_to_expected: bool) -> Result<ast::Ident<'p>, ParseError> {
        if let Some(ident) = self.eat_ident(add_to_expected) {
            Ok(ident)
        } else {
            Err(self.report_expected())
        }
    }

    #[inline]
    fn eat_number(&mut self, add_to_expected: bool) -> Option<(Number<'ast>, SpanId)> {
        if let TokenKind::Number(n) = self.curr_token.kind {
            let span = self.curr_token.span;
            self.next_token();
            Some((n, span))
        } else {
            if add_to_expected {
                self.expected_things.push(ExpectedToken::Number);
            }
            None
        }
    }

    #[inline]
    fn eat_string(&mut self, add_to_expected: bool) -> Option<(&'ast str, SpanId)> {
        if let TokenKind::String(s) = self.curr_token.kind {
            let span = self.curr_token.span;
            self.next_token();
            Some((s, span))
        } else {
            if add_to_expected {
                self.expected_things.push(ExpectedToken::String);
            }
            None
        }
    }

    #[inline]
    fn eat_text_block(&mut self, add_to_expected: bool) -> Option<(&'ast str, SpanId)> {
        if let TokenKind::TextBlock(s) = self.curr_token.kind {
            let span = self.curr_token.span;
            self.next_token();
            Some((s, span))
        } else {
            if add_to_expected {
                self.expected_things.push(ExpectedToken::TextBlock);
            }
            None
        }
    }

    #[inline]
    fn eat_visibility(&mut self, add_to_expected: bool) -> Option<ast::Visibility> {
        if self
            .eat_simple(STokenKind::Colon, add_to_expected)
            .is_some()
        {
            Some(ast::Visibility::Default)
        } else if self
            .eat_simple(STokenKind::ColonColon, add_to_expected)
            .is_some()
        {
            Some(ast::Visibility::Hidden)
        } else if self
            .eat_simple(STokenKind::ColonColonColon, add_to_expected)
            .is_some()
        {
            Some(ast::Visibility::ForceVisible)
        } else {
            None
        }
    }

    #[inline]
    fn eat_plus_visibility(&mut self, add_to_expected: bool) -> Option<(bool, ast::Visibility)> {
        if self
            .eat_simple(STokenKind::Colon, add_to_expected)
            .is_some()
        {
            Some((false, ast::Visibility::Default))
        } else if self
            .eat_simple(STokenKind::ColonColon, add_to_expected)
            .is_some()
        {
            Some((false, ast::Visibility::Hidden))
        } else if self
            .eat_simple(STokenKind::ColonColonColon, add_to_expected)
            .is_some()
        {
            Some((false, ast::Visibility::ForceVisible))
        } else if self
            .eat_simple(STokenKind::PlusColon, add_to_expected)
            .is_some()
        {
            Some((true, ast::Visibility::Default))
        } else if self
            .eat_simple(STokenKind::PlusColonColon, add_to_expected)
            .is_some()
        {
            Some((true, ast::Visibility::Hidden))
        } else if self
            .eat_simple(STokenKind::PlusColonColonColon, add_to_expected)
            .is_some()
        {
            Some((true, ast::Visibility::ForceVisible))
        } else {
            None
        }
    }

    #[inline]
    fn peek_simple(&self, skind: STokenKind, i: usize) -> bool {
        if i == 0 {
            matches!(self.curr_token.kind, TokenKind::Simple(k) if k == skind)
        } else if let Some(Token { kind, .. }) = self.rem_tokens.as_slice().get(i - 1) {
            matches!(*kind, TokenKind::Simple(k) if k == skind)
        } else {
            false
        }
    }

    #[inline]
    fn peek_ident(&self, i: usize) -> bool {
        if i == 0 {
            matches!(self.curr_token.kind, TokenKind::Ident(_))
        } else {
            matches!(
                self.rem_tokens.as_slice().get(i - 1),
                Some(Token {
                    kind: TokenKind::Ident(_),
                    ..
                })
            )
        }
    }

    /// Parses the tokens into an expression.
    pub fn parse_root_expr(mut self) -> Result<ast::Expr<'p, 'ast>, ParseError> {
        let expr = self.parse_expr()?;
        if self.eat_eof(true) {
            Ok(expr)
        } else {
            Err(self.report_expected())
        }
    }
}
