//! A lexer for the Jsonnet language.
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
//! // Create the lexer
//! let lexer = rsjsonnet_lang::lexer::Lexer::new(
//!     &arena,
//!     &ast_arena,
//!     &str_interner,
//!     &mut span_mgr,
//!     span_ctx,
//!     source,
//! );
//!
//! // Lex the whole input.
//! let tokens = lexer.lex_to_eof(false).unwrap();
//! ```

use crate::arena::Arena;
use crate::interner::StrInterner;
use crate::span::{SpanContextId, SpanId, SpanManager};
use crate::token::{Number, STokenKind, Token, TokenKind};

mod error;

pub use error::LexError;

/// The lexer. See the [module-level documentation](self) for more details.
pub struct Lexer<'a, 'p, 'ast> {
    arena: &'p Arena,
    ast_arena: &'ast Arena,
    str_interner: &'a StrInterner<'p>,
    span_mgr: &'a mut SpanManager,
    span_ctx: SpanContextId,
    input: &'a [u8],
    start_pos: usize,
    end_pos: usize,
}

impl<'a, 'p, 'ast> Lexer<'a, 'p, 'ast> {
    /// Create a new lexer that operates on `input`.
    pub fn new(
        arena: &'p Arena,
        ast_arena: &'ast Arena,
        str_interner: &'a StrInterner<'p>,
        span_mgr: &'a mut SpanManager,
        span_ctx: SpanContextId,
        input: &'a [u8],
    ) -> Self {
        Self {
            arena,
            ast_arena,
            str_interner,
            span_mgr,
            span_ctx,
            input,
            start_pos: 0,
            end_pos: 0,
        }
    }

    /// Lexes all the remaining input into a token `Vec`.
    ///
    /// The last token will be an [end-of-file token](TokenKind::EndOfFile).
    ///
    /// If `whitespaces_and_comments` is `false`,
    /// [whitespace](TokenKind::Whitespace) and [comment](TokenKind::Comment)
    /// tokens will be omitted from the output.
    pub fn lex_to_eof(
        mut self,
        whitespaces_and_comments: bool,
    ) -> Result<Vec<Token<'p, 'ast>>, LexError> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token()?;
            let is_eof = token.kind == TokenKind::EndOfFile;
            if whitespaces_and_comments
                || !matches!(token.kind, TokenKind::Whitespace | TokenKind::Comment)
            {
                tokens.push(token);
            }
            if is_eof {
                break;
            }
        }
        Ok(tokens)
    }

    /// Lexes the next token.
    pub fn next_token(&mut self) -> Result<Token<'p, 'ast>, LexError> {
        match self.eat_any_byte() {
            None => Ok(self.commit_token(TokenKind::EndOfFile)),
            Some(b'{') => Ok(self.commit_token(TokenKind::Simple(STokenKind::LeftBrace))),
            Some(b'}') => Ok(self.commit_token(TokenKind::Simple(STokenKind::RightBrace))),
            Some(b'[') => Ok(self.commit_token(TokenKind::Simple(STokenKind::LeftBracket))),
            Some(b']') => Ok(self.commit_token(TokenKind::Simple(STokenKind::RightBracket))),
            Some(b',') => Ok(self.commit_token(TokenKind::Simple(STokenKind::Comma))),
            Some(b'.') => Ok(self.commit_token(TokenKind::Simple(STokenKind::Dot))),
            Some(b'(') => Ok(self.commit_token(TokenKind::Simple(STokenKind::LeftParen))),
            Some(b')') => Ok(self.commit_token(TokenKind::Simple(STokenKind::RightParen))),
            Some(b';') => Ok(self.commit_token(TokenKind::Simple(STokenKind::Semicolon))),
            Some(b'/') => {
                if self.eat_byte(b'/') {
                    self.lex_single_line_comment()
                } else if self.eat_byte(b'*') {
                    self.lex_multi_line_comment()
                } else {
                    Ok(self.lex_operator())
                }
            }
            Some(b'|') => {
                if self.eat_slice(b"||") {
                    self.lex_text_block()
                } else {
                    Ok(self.lex_operator())
                }
            }
            Some(
                b'!' | b'$' | b':' | b'~' | b'+' | b'-' | b'&' | b'^' | b'=' | b'<' | b'>' | b'*'
                | b'%',
            ) => Ok(self.lex_operator()),
            Some(b' ' | b'\t' | b'\n' | b'\r') => {
                while self.eat_byte_if(|byte| matches!(byte, b' ' | b'\t' | b'\n' | b'\r')) {}
                Ok(self.commit_token(TokenKind::Whitespace))
            }
            Some(b'#') => self.lex_single_line_comment(),
            Some(chr @ b'0'..=b'9') => self.lex_number(chr),
            Some(b'_' | b'a'..=b'z' | b'A'..=b'Z') => Ok(self.lex_ident()),
            Some(b'@') => {
                if self.eat_byte(b'\'') {
                    self.lex_verbatim_string(b'\'')
                } else if self.eat_byte(b'"') {
                    self.lex_verbatim_string(b'"')
                } else {
                    let span = self.make_span(self.start_pos, self.end_pos);
                    return Err(LexError::InvalidChar { span, chr: '@' });
                }
            }
            Some(b'\'') => self.lex_quoted_string(b'\''),
            Some(b'"') => self.lex_quoted_string(b'"'),
            Some(byte0) => match self.eat_cont_any_char(byte0) {
                Ok(chr) => {
                    let span = self.make_span(self.start_pos, self.end_pos);
                    Err(LexError::InvalidChar { span, chr })
                }
                Err(_) => {
                    let span = self.make_span(self.start_pos, self.end_pos);
                    Err(LexError::InvalidUtf8 {
                        span,
                        seq: self.input[self.start_pos..self.end_pos].to_vec(),
                    })
                }
            },
        }
    }

    fn lex_single_line_comment(&mut self) -> Result<Token<'p, 'ast>, LexError> {
        while !matches!(self.eat_any_byte(), None | Some(b'\n')) {}
        Ok(self.commit_token(TokenKind::Comment))
    }

    fn lex_multi_line_comment(&mut self) -> Result<Token<'p, 'ast>, LexError> {
        loop {
            if self.eat_slice(b"*/") {
                break;
            } else if self.eat_any_byte().is_none() {
                let span = self.make_span(self.start_pos, self.end_pos);
                return Err(LexError::UnfinishedMultilineComment { span });
            }
        }
        Ok(self.commit_token(TokenKind::Comment))
    }

    #[must_use]
    fn lex_operator(&mut self) -> Token<'p, 'ast> {
        let mut sure_end_pos = self.end_pos;
        loop {
            if self.eat_slice(b"|||") || self.eat_slice(b"//") || self.eat_slice(b"/*") {
                // `|||`, `//` and `/*` cannot appear within an operator
                break;
            }

            let Some(next_byte) = self.eat_any_byte() else {
                break;
            };
            // An multi-byte operator cannot end with '+', '-', '~', '!' or '$'
            if matches!(
                next_byte,
                b':' | b'&' | b'|' | b'^' | b'=' | b'<' | b'>' | b'*' | b'/' | b'%'
            ) {
                sure_end_pos = self.end_pos;
            } else if !matches!(next_byte, b'+' | b'-' | b'~' | b'!' | b'$') {
                break;
            }
        }
        self.end_pos = sure_end_pos;
        let op = &self.input[self.start_pos..self.end_pos];
        match op {
            b":" => self.commit_token(TokenKind::Simple(STokenKind::Colon)),
            b"::" => self.commit_token(TokenKind::Simple(STokenKind::ColonColon)),
            b":::" => self.commit_token(TokenKind::Simple(STokenKind::ColonColonColon)),
            b"+:" => self.commit_token(TokenKind::Simple(STokenKind::PlusColon)),
            b"+::" => self.commit_token(TokenKind::Simple(STokenKind::PlusColonColon)),
            b"+:::" => self.commit_token(TokenKind::Simple(STokenKind::PlusColonColonColon)),
            b"=" => self.commit_token(TokenKind::Simple(STokenKind::Eq)),
            b"$" => self.commit_token(TokenKind::Simple(STokenKind::Dollar)),
            b"*" => self.commit_token(TokenKind::Simple(STokenKind::Asterisk)),
            b"/" => self.commit_token(TokenKind::Simple(STokenKind::Slash)),
            b"%" => self.commit_token(TokenKind::Simple(STokenKind::Percent)),
            b"+" => self.commit_token(TokenKind::Simple(STokenKind::Plus)),
            b"-" => self.commit_token(TokenKind::Simple(STokenKind::Minus)),
            b"<<" => self.commit_token(TokenKind::Simple(STokenKind::LtLt)),
            b">>" => self.commit_token(TokenKind::Simple(STokenKind::GtGt)),
            b"<" => self.commit_token(TokenKind::Simple(STokenKind::Lt)),
            b"<=" => self.commit_token(TokenKind::Simple(STokenKind::LtEq)),
            b">" => self.commit_token(TokenKind::Simple(STokenKind::Gt)),
            b">=" => self.commit_token(TokenKind::Simple(STokenKind::GtEq)),
            b"==" => self.commit_token(TokenKind::Simple(STokenKind::EqEq)),
            b"!=" => self.commit_token(TokenKind::Simple(STokenKind::ExclamEq)),
            b"&" => self.commit_token(TokenKind::Simple(STokenKind::Amp)),
            b"^" => self.commit_token(TokenKind::Simple(STokenKind::Hat)),
            b"|" => self.commit_token(TokenKind::Simple(STokenKind::Pipe)),
            b"&&" => self.commit_token(TokenKind::Simple(STokenKind::AmpAmp)),
            b"||" => self.commit_token(TokenKind::Simple(STokenKind::PipePipe)),
            b"!" => self.commit_token(TokenKind::Simple(STokenKind::Exclam)),
            b"~" => self.commit_token(TokenKind::Simple(STokenKind::Tilde)),
            _ => self.commit_token(TokenKind::OtherOp(
                self.ast_arena.alloc_str(std::str::from_utf8(op).unwrap()),
            )),
        }
    }

    #[must_use]
    fn lex_ident(&mut self) -> Token<'p, 'ast> {
        while self.eat_byte_if(|b| b.is_ascii_alphanumeric() || b == b'_') {}
        let ident_bytes = &self.input[self.start_pos..self.end_pos];
        match ident_bytes {
            b"assert" => self.commit_token(TokenKind::Simple(STokenKind::Assert)),
            b"else" => self.commit_token(TokenKind::Simple(STokenKind::Else)),
            b"error" => self.commit_token(TokenKind::Simple(STokenKind::Error)),
            b"false" => self.commit_token(TokenKind::Simple(STokenKind::False)),
            b"for" => self.commit_token(TokenKind::Simple(STokenKind::For)),
            b"function" => self.commit_token(TokenKind::Simple(STokenKind::Function)),
            b"if" => self.commit_token(TokenKind::Simple(STokenKind::If)),
            b"import" => self.commit_token(TokenKind::Simple(STokenKind::Import)),
            b"importstr" => self.commit_token(TokenKind::Simple(STokenKind::Importstr)),
            b"importbin" => self.commit_token(TokenKind::Simple(STokenKind::Importbin)),
            b"in" => self.commit_token(TokenKind::Simple(STokenKind::In)),
            b"local" => self.commit_token(TokenKind::Simple(STokenKind::Local)),
            b"null" => self.commit_token(TokenKind::Simple(STokenKind::Null)),
            b"tailstrict" => self.commit_token(TokenKind::Simple(STokenKind::Tailstrict)),
            b"then" => self.commit_token(TokenKind::Simple(STokenKind::Then)),
            b"self" => self.commit_token(TokenKind::Simple(STokenKind::Self_)),
            b"super" => self.commit_token(TokenKind::Simple(STokenKind::Super)),
            b"true" => self.commit_token(TokenKind::Simple(STokenKind::True)),
            _ => self.commit_token(TokenKind::Ident(
                self.str_interner
                    .intern(self.arena, std::str::from_utf8(ident_bytes).unwrap()),
            )),
        }
    }

    fn lex_number(&mut self, chr0: u8) -> Result<Token<'p, 'ast>, LexError> {
        let leading_zero = chr0 == b'0';
        let mut digits = String::new();
        digits.push(char::from(chr0));

        while let Some(chr) = self.eat_get_byte_if(|b| b.is_ascii_digit()) {
            if digits.len() == 1 && leading_zero {
                let span = self.make_span(self.end_pos - 2, self.end_pos - 1);
                return Err(LexError::LeadingZeroInNumber { span });
            }
            digits.push(char::from(chr));
        }

        let mut implicit_exp = 0i64;
        if self.eat_byte(b'.') {
            while let Some(chr) = self.eat_get_byte_if(|b| b.is_ascii_digit()) {
                digits.push(char::from(chr));
                implicit_exp -= 1;
            }
            if implicit_exp == 0 {
                let span = self.make_span(self.end_pos - 1, self.end_pos);
                return Err(LexError::MissingFracDigits { span });
            }
        }

        let eff_exp;
        if self.eat_byte_if(|b| matches!(b, b'e' | b'E')) {
            let mut explicit_exp_sign = false;
            let mut explicit_exp = Some(0u64);

            let exp_start = self.end_pos - 1;

            if self.eat_byte(b'+') {
                // explicit +
            } else if self.eat_byte(b'-') {
                explicit_exp_sign = true;
            }

            let mut num_exp_digits = 0;
            while let Some(chr) = self.eat_get_byte_if(|b| b.is_ascii_digit()) {
                explicit_exp = explicit_exp
                    .and_then(|e| e.checked_mul(10))
                    .and_then(|e| e.checked_add(u64::from(chr - b'0')));
                num_exp_digits += 1;
            }

            if num_exp_digits == 0 {
                let span = self.make_span(exp_start, self.end_pos);
                return Err(LexError::MissingExpDigits { span });
            }

            eff_exp = explicit_exp
                .and_then(|e| i64::try_from(e).ok())
                .and_then(|e| {
                    if explicit_exp_sign {
                        implicit_exp.checked_sub(e)
                    } else {
                        implicit_exp.checked_add(e)
                    }
                })
                .ok_or_else(|| {
                    let span = self.make_span(exp_start, self.end_pos);
                    LexError::ExpOverflow { span }
                })?;
        } else {
            eff_exp = implicit_exp;
        }

        Ok(self.commit_token(TokenKind::Number(Number {
            digits: self.ast_arena.alloc_str(&digits),
            exp: eff_exp,
        })))
    }

    fn lex_quoted_string(&mut self, delim: u8) -> Result<Token<'p, 'ast>, LexError> {
        let mut string = String::new();
        loop {
            if self.eat_byte(delim) {
                break;
            } else if self.eat_byte(b'\\') {
                let escape_start = self.end_pos - 1;
                if self.eat_byte(b'"') {
                    string.push('"');
                } else if self.eat_byte(b'\'') {
                    string.push('\'');
                } else if self.eat_byte(b'\\') {
                    string.push('\\');
                } else if self.eat_byte(b'/') {
                    string.push('/');
                } else if self.eat_byte(b'b') {
                    string.push('\u{8}');
                } else if self.eat_byte(b'f') {
                    string.push('\u{C}');
                } else if self.eat_byte(b'n') {
                    string.push('\n');
                } else if self.eat_byte(b'r') {
                    string.push('\r');
                } else if self.eat_byte(b't') {
                    string.push('\t');
                } else if self.eat_byte(b'u') {
                    let hex_from_digit = |b: u8| match b {
                        b'0'..=b'9' => Some(b - b'0'),
                        b'a'..=b'f' => Some(b - b'a' + 10),
                        b'A'..=b'F' => Some(b - b'A' + 10),
                        _ => None,
                    };

                    let eat_codeunit = |this: &mut Self| -> Option<u16> {
                        let d0 = this.eat_map_byte(hex_from_digit)?;
                        let d1 = this.eat_map_byte(hex_from_digit)?;
                        let d2 = this.eat_map_byte(hex_from_digit)?;
                        let d3 = this.eat_map_byte(hex_from_digit)?;
                        Some(
                            (u16::from(d0) << 12)
                                | (u16::from(d1) << 8)
                                | (u16::from(d2) << 4)
                                | u16::from(d3),
                        )
                    };

                    let Some(cu1) = eat_codeunit(self) else {
                        let span = self.make_span(escape_start, self.end_pos);
                        return Err(LexError::IncompleteUnicodeEscape { span });
                    };

                    if matches!(cu1, 0xD800..=0xDFFF) && self.eat_slice(b"\\u") {
                        let Some(cu2) = eat_codeunit(self) else {
                            let span = self.make_span(escape_start + 6, self.end_pos);
                            return Err(LexError::IncompleteUnicodeEscape { span });
                        };
                        if let Ok(chr) = char::decode_utf16([cu1, cu2]).next().unwrap() {
                            string.push(chr);
                        } else {
                            let span = self.make_span(escape_start, self.end_pos);
                            return Err(LexError::InvalidUtf16EscapeSequence {
                                span,
                                cu1,
                                cu2: Some(cu2),
                            });
                        }
                    } else if let Some(chr) = char::from_u32(cu1.into()) {
                        string.push(chr);
                    } else {
                        let span = self.make_span(escape_start, self.end_pos);
                        return Err(LexError::InvalidUtf16EscapeSequence {
                            span,
                            cu1,
                            cu2: None,
                        });
                    }
                } else {
                    match self.eat_any_char() {
                        None => {
                            let span = self.make_span(self.start_pos, self.end_pos);
                            return Err(LexError::UnfinishedString { span });
                        }
                        Some(chr) => {
                            let span = self.make_span(escape_start, self.end_pos);
                            return Err(LexError::InvalidEscapeInString {
                                span,
                                chr: chr.unwrap_or('\u{FFFD}'),
                            });
                        }
                    }
                }
            } else {
                match self.eat_any_char() {
                    None => {
                        let span = self.make_span(self.start_pos, self.end_pos);
                        return Err(LexError::UnfinishedString { span });
                    }
                    Some(Ok(chr)) => string.push(chr),
                    Some(Err(_)) => string.push('\u{FFFD}'),
                }
            }
        }
        Ok(self.commit_token(TokenKind::String(self.ast_arena.alloc_str(&string))))
    }

    fn lex_verbatim_string(&mut self, delim: u8) -> Result<Token<'p, 'ast>, LexError> {
        let mut string = String::new();
        loop {
            if self.eat_byte(delim) {
                if self.eat_byte(delim) {
                    string.push(char::from(delim));
                } else {
                    break;
                }
            } else {
                match self.eat_any_char() {
                    None => {
                        let span = self.make_span(self.start_pos, self.end_pos);
                        return Err(LexError::UnfinishedString { span });
                    }
                    Some(chr) => string.push(chr.unwrap_or('\u{FFFD}')),
                }
            }
        }
        Ok(self.commit_token(TokenKind::String(self.ast_arena.alloc_str(&string))))
    }

    #[inline]
    fn lex_text_block(&mut self) -> Result<Token<'p, 'ast>, LexError> {
        let strip_last_lf = self.eat_byte(b'-');

        let mut string = String::new();
        let mut prefix;
        while self.eat_byte_if(|b| matches!(b, b' ' | b'\t' | b'\r')) {}
        if self.eat_byte(b'\n') {
            loop {
                let prefix_start = self.end_pos;
                while self.eat_byte_if(|b| matches!(b, b' ' | b'\t')) {}
                let prefix_end = self.end_pos;
                prefix = &self.input[prefix_start..prefix_end];
                if self.eat_byte(b'\r') {
                    string.push('\r');
                }
                if prefix.is_empty() {
                    if self.eat_byte(b'\n') {
                        // handle fully empty lines at the beginning:
                        // |||
                        //
                        // →→...
                        // |||
                        string.push('\n');
                        continue;
                    } else {
                        let span = self.make_span(prefix_start, prefix_end);
                        return Err(LexError::MissingWhitespaceTextBlockStart { span });
                    }
                }
                break;
            }
        } else {
            let span = self.make_span(self.start_pos, self.end_pos);
            return Err(LexError::MissingLineBreakAfterTextBlockStart { span });
        }

        'outer: loop {
            while self.eat_byte(b'\n') {
                string.push('\n');
                loop {
                    // Handle fully empty lines
                    if self.eat_byte(b'\n') {
                        string.push('\n');
                    } else if self.eat_slice(b"\r\n") {
                        string.push_str("\r\n");
                    } else {
                        break;
                    }
                }
                if !self.eat_slice(prefix) {
                    let line_start = self.end_pos;
                    while self.eat_byte_if(|b| matches!(b, b' ' | b'\t')) {}
                    if self.eat_slice(b"|||") {
                        break 'outer;
                    } else {
                        let span = self.make_span(line_start, self.end_pos);
                        return Err(LexError::InvalidTextBlockTermination { span });
                    }
                }
            }

            match self.eat_any_char() {
                None => {
                    let span = self.make_span(self.start_pos, self.end_pos);
                    return Err(LexError::UnfinishedString { span });
                }
                Some(chr) => string.push(chr.unwrap_or('\u{FFFD}')),
            }
        }

        let actual_string = if strip_last_lf {
            string.strip_suffix('\n').unwrap()
        } else {
            string.as_str()
        };

        Ok(self.commit_token(TokenKind::TextBlock(
            self.ast_arena.alloc_str(actual_string),
        )))
    }

    #[must_use]
    #[inline]
    fn eat_byte(&mut self, byte: u8) -> bool {
        if matches!(self.input.get(self.end_pos), Some(&b) if b == byte) {
            self.end_pos += 1;
            true
        } else {
            false
        }
    }

    #[must_use]
    #[inline]
    fn eat_byte_if(&mut self, pred: impl FnOnce(u8) -> bool) -> bool {
        if matches!(self.input.get(self.end_pos), Some(&b) if pred(b)) {
            self.end_pos += 1;
            true
        } else {
            false
        }
    }

    #[must_use]
    #[inline]
    fn eat_get_byte_if(&mut self, pred: impl FnOnce(u8) -> bool) -> Option<u8> {
        if let Some(&b) = self.input.get(self.end_pos).filter(|&&b| pred(b)) {
            self.end_pos += 1;
            Some(b)
        } else {
            None
        }
    }

    #[must_use]
    #[inline]
    fn eat_map_byte<R>(&mut self, f: impl FnOnce(u8) -> Option<R>) -> Option<R> {
        if let Some(r) = self.input.get(self.end_pos).and_then(|&b| f(b)) {
            self.end_pos += 1;
            Some(r)
        } else {
            None
        }
    }

    #[must_use]
    #[inline]
    fn eat_slice(&mut self, s: &[u8]) -> bool {
        if self
            .input
            .get(self.end_pos..)
            .is_some_and(|rem| rem.starts_with(s))
        {
            self.end_pos += s.len();
            true
        } else {
            false
        }
    }

    #[must_use]
    #[inline]
    fn decode_cont_char(&self, byte0: u8) -> (usize, Option<char>) {
        // Based on `<Utf8Chunks as Iterator>::next` from Rust libcore
        const TAG_CONT_U8: u8 = 128;
        fn safe_get(xs: &[u8], i: usize) -> u8 {
            *xs.get(i).unwrap_or(&0)
        }

        let mut i = self.end_pos;
        match byte0 {
            0..=0x7F => (i, Some(char::from(byte0))),
            0b11000000..=0b11011111 => {
                let byte1 = safe_get(self.input, i);
                if byte1 & 192 != TAG_CONT_U8 {
                    return (i, None);
                }
                i += 1;

                let cp = (u32::from(byte0 & 0b11111) << 6) | u32::from(byte1 & 0b111111);
                (i, Some(char::from_u32(cp).unwrap()))
            }
            0b11100000..=0b11101111 => {
                let byte1 = safe_get(self.input, i);
                match (byte0, byte1) {
                    (0xE0, 0xA0..=0xBF) => (),
                    (0xE1..=0xEC, 0x80..=0xBF) => (),
                    (0xED, 0x80..=0x9F) => (),
                    (0xEE..=0xEF, 0x80..=0xBF) => (),
                    _ => return (i, None),
                }
                i += 1;
                let byte2 = safe_get(self.input, i);
                if byte2 & 192 != TAG_CONT_U8 {
                    return (i, None);
                }
                i += 1;

                let cp = (u32::from(byte0 & 0b1111) << 12)
                    | (u32::from(byte1 & 0b111111) << 6)
                    | u32::from(byte2 & 0b111111);
                (i, Some(char::from_u32(cp).unwrap()))
            }
            0b11110000..=0b11110111 => {
                let byte1 = safe_get(self.input, i);
                match (byte0, byte1) {
                    (0xF0, 0x90..=0xBF) => (),
                    (0xF1..=0xF3, 0x80..=0xBF) => (),
                    (0xF4, 0x80..=0x8F) => (),
                    _ => return (i, None),
                }
                i += 1;
                let byte2 = safe_get(self.input, i);
                if byte2 & 192 != TAG_CONT_U8 {
                    return (i, None);
                }
                i += 1;
                let byte3 = safe_get(self.input, i);
                if byte3 & 192 != TAG_CONT_U8 {
                    return (i, None);
                }
                i += 1;

                let cp = (u32::from(byte0 & 0b111) << 18)
                    | (u32::from(byte1 & 0b111111) << 12)
                    | (u32::from(byte2 & 0b111111) << 6)
                    | u32::from(byte3 & 0b111111);
                (i, Some(char::from_u32(cp).unwrap()))
            }
            _ => (i, None),
        }
    }

    #[must_use]
    #[inline]
    fn eat_any_byte(&mut self) -> Option<u8> {
        if let Some(&byte) = self.input.get(self.end_pos) {
            self.end_pos += 1;
            Some(byte)
        } else {
            None
        }
    }

    #[inline]
    fn eat_cont_any_char(&mut self, byte0: u8) -> Result<char, usize> {
        let (end_pos, chr) = self.decode_cont_char(byte0);
        if let Some(chr) = chr {
            self.end_pos = end_pos;
            Ok(chr)
        } else {
            let error_len = end_pos - self.end_pos + 1;
            self.end_pos = end_pos;
            Err(error_len)
        }
    }

    #[must_use]
    #[inline]
    fn eat_any_char(&mut self) -> Option<Result<char, usize>> {
        self.eat_any_byte()
            .map(|byte0| self.eat_cont_any_char(byte0))
    }

    #[must_use]
    #[inline]
    fn commit_token(&mut self, kind: TokenKind<'p, 'ast>) -> Token<'p, 'ast> {
        let start_pos = self.start_pos;
        self.start_pos = self.end_pos;
        Token {
            span: self.make_span(start_pos, self.end_pos),
            kind,
        }
    }

    #[must_use]
    #[inline]
    fn make_span(&mut self, start_pos: usize, end_pos: usize) -> SpanId {
        self.span_mgr.intern_span(self.span_ctx, start_pos, end_pos)
    }
}

#[cfg(test)]
mod tests {
    use super::Lexer;
    use crate::arena::Arena;
    use crate::interner::StrInterner;
    use crate::span::SpanManager;

    #[test]
    fn test_decode_valid_utf8() {
        let arena = Arena::new();
        let ast_arena = Arena::new();
        let str_interner = StrInterner::new();
        let mut span_mgr = SpanManager::new();
        let (span_ctx, _) = span_mgr.insert_source_context(4);

        for chr in '\u{0}'..=char::MAX {
            let mut buf = [0; 4];
            let encoded = chr.encode_utf8(&mut buf);

            let mut lexer = Lexer::new(
                &arena,
                &ast_arena,
                &str_interner,
                &mut span_mgr,
                span_ctx,
                encoded.as_bytes(),
            );
            assert_eq!(lexer.eat_any_char(), Some(Ok(chr)));
        }
    }

    #[test]
    fn test_decode_invalid_utf8() {
        let arena = Arena::new();
        let ast_area = Arena::new();
        let str_interner = StrInterner::new();
        let mut span_mgr = SpanManager::new();

        let tests: &[&[u8]] = &[
            // Taken from `from_utf8_error` test in Rust's library/alloc/tests/str.rs
            b"\xFF ",
            b"\x80 ",
            b"\xC1 ",
            b"\xC1",
            b"\xC2",
            b"\xC2 ",
            b"\xC2\xC0",
            b"\xE0",
            b"\xE0\x9F",
            b"\xE0\xA0",
            b"\xE0\xA0\xC0",
            b"\xE0\xA0 ",
            b"\xED\xA0\x80 ",
            b"\xF1",
            b"\xF1\x80",
            b"\xF1\x80\x80",
            b"\xF1 ",
            b"\xF1\x80 ",
            b"\xF1\x80\x80 ",
        ];

        let (span_ctx, _) =
            span_mgr.insert_source_context(tests.iter().fold(0, |l, s| l.max(s.len())));

        for &source in tests.iter() {
            let utf8_error = std::str::from_utf8(source).unwrap_err();
            assert_eq!(utf8_error.valid_up_to(), 0);
            let error_len = utf8_error.error_len().unwrap_or(source.len());

            let mut lexer = Lexer::new(
                &ast_area,
                &arena,
                &str_interner,
                &mut span_mgr,
                span_ctx,
                source,
            );
            assert_eq!(lexer.eat_any_char(), Some(Err(error_len)));
        }
    }
}
