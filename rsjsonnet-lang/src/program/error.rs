use super::{EvalStackTraceItem, ValueData};
use crate::ast;
use crate::lexer::LexError;
use crate::parser::ParseError;
use crate::span::SpanId;

#[derive(Clone, Debug)]
pub enum AnalyzeError {
    UnknownVariable {
        span: SpanId,
        name: String,
    },
    SelfOutsideObject {
        self_span: SpanId,
    },
    SuperOutsideObject {
        super_span: SpanId,
    },
    DollarOutsideObject {
        dollar_span: SpanId,
    },
    RepeatedLocalName {
        original_span: SpanId,
        repeated_span: SpanId,
        name: String,
    },
    RepeatedFieldName {
        original_span: SpanId,
        repeated_span: SpanId,
        name: String,
    },
    RepeatedParamName {
        original_span: SpanId,
        repeated_span: SpanId,
        name: String,
    },
    PositionalArgAfterNamed {
        arg_span: SpanId,
    },
    TextBlockAsImportPath {
        span: SpanId,
    },
    ComputedImportPath {
        span: SpanId,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvalError {
    pub stack_trace: Vec<EvalStackTraceItem>,
    pub kind: EvalErrorKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvalErrorKind {
    StackOverflow,
    InfiniteRecursion,
    InvalidIndexedType {
        span: SpanId,
        got_type: EvalErrorValueType,
    },
    InvalidSlicedType {
        span: SpanId,
        got_type: EvalErrorValueType,
    },
    SliceIndexOrStepIsNotNumber {
        span: SpanId,
        got_type: EvalErrorValueType,
    },
    StringIndexIsNotNumber {
        span: SpanId,
        got_type: EvalErrorValueType,
    },
    ArrayIndexIsNotNumber {
        span: SpanId,
        got_type: EvalErrorValueType,
    },
    NumericIndexIsNotValid {
        span: SpanId,
        index: String,
    },
    NumericIndexOutOfRange {
        span: SpanId,
        index: usize,
        length: usize,
    },
    ObjectIndexIsNotString {
        span: SpanId,
        got_type: EvalErrorValueType,
    },
    RepeatedFieldName {
        span: SpanId,
        name: String,
    },
    FieldNameIsNotString {
        span: SpanId,
        got_type: EvalErrorValueType,
    },
    UnknownObjectField {
        span: SpanId,
        field_name: String,
    },
    FieldOfNonObject {
        span: SpanId,
    },
    SuperWithoutSuperObject {
        span: SpanId,
    },
    ForSpecValueIsNotArray {
        span: SpanId,
        got_type: EvalErrorValueType,
    },
    CondIsNotBool {
        span: SpanId,
        got_type: EvalErrorValueType,
    },
    CalleeIsNotFunction {
        span: Option<SpanId>,
        got_type: EvalErrorValueType,
    },
    TooManyCallArgs {
        span: Option<SpanId>,
        num_params: usize,
    },
    UnknownCallParam {
        span: Option<SpanId>,
        param_name: String,
    },
    RepeatedCallParam {
        span: Option<SpanId>,
        param_name: String,
    },
    CallParamNotBound {
        span: Option<SpanId>,
        param_name: String,
    },
    NativeCallFailed,
    InvalidUnaryOpType {
        span: SpanId,
        op: ast::UnaryOp,
        rhs_type: EvalErrorValueType,
    },
    InvalidBinaryOpTypes {
        span: Option<SpanId>,
        op: ast::BinaryOp,
        lhs_type: EvalErrorValueType,
        rhs_type: EvalErrorValueType,
    },
    NumberOverflow {
        span: Option<SpanId>,
    },
    NumberNan {
        span: Option<SpanId>,
    },
    DivByZero {
        span: Option<SpanId>,
    },
    ShiftByNegative {
        span: Option<SpanId>,
    },
    InvalidStdFuncArgType {
        func_name: String,
        arg_index: usize,
        expected_types: Vec<EvalErrorValueType>,
        got_type: EvalErrorValueType,
    },
    AssertFailed {
        span: SpanId,
        message: Option<String>,
    },
    AssertEqualFailed {
        lhs: String,
        rhs: String,
    },
    ExplicitError {
        span: SpanId,
        message: String,
    },
    ImportFailed {
        span: SpanId,
        path: String,
    },
    UnknownExtVar {
        name: String,
    },
    ManifestFunction,
    CompareNullInequality,
    CompareBooleanInequality,
    CompareObjectInequality,
    CompareFunctions,
    CompareDifferentTypesInequality {
        lhs_type: EvalErrorValueType,
        rhs_type: EvalErrorValueType,
    },
    PrimitiveEqualsNonPrimitive {
        got_type: EvalErrorValueType,
    },
    Other {
        span: Option<SpanId>,
        message: String,
    },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum EvalErrorValueType {
    Null,
    Bool,
    Number,
    String,
    Array,
    Object,
    Function,
}

impl EvalErrorValueType {
    pub(super) fn from_value(value: &ValueData) -> Self {
        match value {
            ValueData::Null => Self::Null,
            ValueData::Bool(_) => Self::Bool,
            ValueData::Number(_) => Self::Number,
            ValueData::String(_) => Self::String,
            ValueData::Array(_) => Self::Array,
            ValueData::Object(_) => Self::Object,
            ValueData::Function(_) => Self::Function,
        }
    }

    pub(super) fn to_str(self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Bool => "boolean",
            Self::Number => "number",
            Self::String => "string",
            Self::Array => "array",
            Self::Object => "object",
            Self::Function => "function",
        }
    }
}

#[derive(Debug)]
pub enum LoadError {
    Lex(LexError),
    Parse(ParseError),
    Analyze(AnalyzeError),
}

impl From<LexError> for LoadError {
    #[inline]
    fn from(err: LexError) -> Self {
        Self::Lex(err)
    }
}

impl From<ParseError> for LoadError {
    #[inline]
    fn from(err: ParseError) -> Self {
        Self::Parse(err)
    }
}

impl From<AnalyzeError> for LoadError {
    #[inline]
    fn from(err: AnalyzeError) -> Self {
        Self::Analyze(err)
    }
}
