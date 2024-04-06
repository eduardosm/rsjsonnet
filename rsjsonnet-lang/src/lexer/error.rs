use crate::span::SpanId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LexError {
    /// Invalid character that does not represent any Jsonnet token
    InvalidChar { span: SpanId, chr: char },
    /// Invalid UTF-8 sequence outside a string or comment
    InvalidUtf8 { span: SpanId, seq: Vec<u8> },
    /// End-of-file reached before finding closing `*/`
    UnfinishedMultilineComment { span: SpanId },
    /// Leading zero in number
    LeadingZeroInNumber { span: SpanId },
    /// Missing fractional digits after `.` in number
    MissingFracDigits { span: SpanId },
    /// Missing exponent digits after `e` in number
    MissingExpDigits { span: SpanId },
    /// Missing exponent digits after `e` in number
    ExpOverflow { span: SpanId },
    /// Invalid escape sequence in string
    InvalidEscapeInString { span: SpanId, chr: char },
    /// Incomplete Unicode escape sequence (`\uXXXX`) in string
    IncompleteUnicodeEscape { span: SpanId },
    /// Invalid codepoint in Unicode escape sequence (`\uXXXX` or
    /// `\uXXXX\uYYYY`) in string
    InvalidUtf16EscapeSequence {
        span: SpanId,
        cu1: u16,
        cu2: Option<u16>,
    },
    /// File ended before ending a string
    UnfinishedString { span: SpanId },
    /// Missing line break after `|||`
    MissingLineBreakAfterTextBlockStart { span: SpanId },
    /// Missing whitespace at the beginning of the first file of a text block.
    MissingWhitespaceTextBlockStart { span: SpanId },
    /// Text block is not ended correctly.
    InvalidTextBlockTermination { span: SpanId },
}
