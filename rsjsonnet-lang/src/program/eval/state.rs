use std::cell::{Cell, OnceCell};
use std::rc::Rc;

use super::super::{ir, ArrayData, FuncData, ObjectData, ThunkData, ThunkEnv, ValueData};
use super::format::FormatPart;
use super::{ManifestJsonFormat, TraceItem};
use crate::ast;
use crate::gc::{Gc, GcView};
use crate::interner::InternedStr;
use crate::span::SpanId;

#[must_use]
pub(super) enum State {
    // Do not push this directly! Use `Evaluator::push_trace_item` instead.
    TraceItem(TraceItem),
    // Do not push this directly! Use `Evaluator::delay_trace_item` instead.
    DelayedTraceItem,
    DiscardValue,
    DoThunk(GcView<ThunkData>),
    GotThunk(GcView<ThunkData>),
    DeepValue,
    SwapLastValues,
    CoerceToString,
    CoerceToStringValue,
    BoolToValue,
    StringToValue,
    PushU32AsValue(u32),
    CmpOrdToBoolValueIsLt,
    CmpOrdToBoolValueIsLe,
    CmpOrdToBoolValueIsGt,
    CmpOrdToBoolValueIsGe,
    CmpOrdToIntValueThreeWay,
    InvertBool,
    AppendToString(String),
    OutputValue,
    OutputString,
    ArrayToValue,
    ManifestJson {
        format: Rc<ManifestJsonFormat>,
        depth: usize,
    },
    ManifestYamlDoc {
        indent_array_in_object: bool,
        quote_keys: bool,
        depth: usize,
        parent_is_array: bool,
        parent_is_object: bool,
    },
    Expr {
        expr: Rc<ir::Expr>,
        env: GcView<ThunkEnv>,
    },
    Error {
        span: SpanId,
    },
    Assert {
        assert_span: SpanId,
        cond_span: SpanId,
        msg_expr: Option<(Rc<ir::Expr>, GcView<ThunkEnv>)>,
    },
    AssertMsg {
        assert_span: SpanId,
    },
    ObjectFixField {
        name: InternedStr,
        name_span: SpanId,
        plus: bool,
        visibility: ast::Visibility,
        value: Rc<ir::Expr>,
        base_env: Option<Gc<ThunkEnv>>,
    },
    ObjectDynField {
        name_span: SpanId,
        plus: bool,
        visibility: ast::Visibility,
        value: Rc<ir::Expr>,
        base_env: Option<Gc<ThunkEnv>>,
    },
    FinishObject,
    Field {
        span: SpanId,
        field_name: InternedStr,
    },
    InitCompSpec {
        var_name: InternedStr,
        value: Rc<ir::Expr>,
        value_span: SpanId,
        env: GcView<ThunkEnv>,
    },
    GotInitCompSpec {
        var_name: InternedStr,
        value_span: SpanId,
    },
    ForSpec {
        var_name: InternedStr,
        value: Rc<ir::Expr>,
        value_span: SpanId,
        env: GcView<ThunkEnv>,
    },
    GotForSpec {
        var_name: InternedStr,
        value_span: SpanId,
    },
    IfSpec {
        cond: Rc<ir::Expr>,
        cond_span: SpanId,
        env: GcView<ThunkEnv>,
    },
    GotIfSpec {
        cond_span: SpanId,
    },
    ArrayComp {
        item: Rc<ir::Expr>,
        env: GcView<ThunkEnv>,
    },
    ObjectComp {
        expr: Rc<ir::Expr>,
        env: GcView<ThunkEnv>,
    },
    FinishObjectComp,
    Index {
        span: SpanId,
    },
    SuperIndex {
        span: SpanId,
        super_span: SpanId,
        env: GcView<ThunkEnv>,
    },
    UnaryOp {
        span: SpanId,
        op: ast::UnaryOp,
    },
    BinaryOp {
        span: Option<SpanId>,
        op: ast::BinaryOp,
    },
    LogicAnd {
        span: SpanId,
        rhs: Rc<ir::Expr>,
        env: GcView<ThunkEnv>,
    },
    LogicOr {
        span: SpanId,
        rhs: Rc<ir::Expr>,
        env: GcView<ThunkEnv>,
    },
    InSuper {
        span: SpanId,
        env: GcView<ThunkEnv>,
    },
    EqualsValue,
    EqualsArray {
        lhs: GcView<ArrayData>,
        rhs: GcView<ArrayData>,
        index: usize,
    },
    EqualsObject {
        lhs: GcView<ObjectData>,
        rhs: GcView<ObjectData>,
        rem_fields: Vec<InternedStr>,
    },
    CompareValue,
    CompareArray {
        lhs: GcView<ArrayData>,
        rhs: GcView<ArrayData>,
        index: usize,
    },
    CallWithExpr {
        call_expr: Rc<ir::Expr>,
        call_env: GcView<ThunkEnv>,
    },
    TopLevelCall {
        pos_args: Box<[GcView<ThunkData>]>,
        named_args: Box<[(InternedStr, GcView<ThunkData>)]>,
    },
    ExecTailstrictCall {
        func: GcView<FuncData>,
        args: Box<[Gc<ThunkData>]>,
    },
    ExecNativeCall {
        name: InternedStr,
        args: Box<[GcView<ThunkData>]>,
    },
    If {
        cond_span: SpanId,
        then_body: Rc<ir::Expr>,
        else_body: Option<Rc<ir::Expr>>,
        env: GcView<ThunkEnv>,
    },
    StdExtVar,
    StdType,
    StdIsArray,
    StdIsBoolean,
    StdIsFunction,
    StdIsNumber,
    StdIsObject,
    StdIsString,
    StdLength,
    StdObjectHasEx,
    StdObjectFieldsEx,
    StdPrimitiveEquals,
    StdCompareArray,
    StdExponent,
    StdMantissa,
    StdFloor,
    StdCeil,
    StdModulo,
    StdPow,
    StdExp,
    StdLog,
    StdSqrt,
    StdSin,
    StdCos,
    StdTan,
    StdAsin,
    StdAcos,
    StdAtan,
    StdAssertEqual,
    StdAssertEqualCheck,
    StdAssertEqualFail1,
    StdAssertEqualFail2,
    StdCodepoint,
    StdChar,
    StdSubstr,
    StdFindSubstr,
    StdStartsWith,
    StdEndsWith,
    StdSplit,
    StdSplitLimit,
    StdSplitLimitR,
    StdStrReplace,
    StdAsciiUpper,
    StdAsciiLower,
    StdStringChars,
    StdFormat,
    StdFormatCodesArray1 {
        parts: Rc<Vec<FormatPart>>,
        array: GcView<ArrayData>,
        part_i: usize,
        array_i: usize,
    },
    StdFormatCodesArray2 {
        parts: Rc<Vec<FormatPart>>,
        array: GcView<ArrayData>,
        part_i: usize,
        array_i: usize,
    },
    StdFormatCodesArray3 {
        parts: Rc<Vec<FormatPart>>,
        array: GcView<ArrayData>,
        part_i: usize,
        array_i: usize,
        fw: u32,
    },
    StdFormatCodesObject1 {
        parts: Rc<Vec<FormatPart>>,
        object: GcView<ObjectData>,
        part_i: usize,
    },
    StdFormatCodesObject2 {
        parts: Rc<Vec<FormatPart>>,
        object: GcView<ObjectData>,
        part_i: usize,
        fw: u32,
    },
    StdFormatCode {
        parts: Rc<Vec<FormatPart>>,
        part_i: usize,
        fw: u32,
        prec: u32,
    },
    StdEscapeStringJson,
    StdEscapeStringBash,
    StdEscapeStringDollars,
    StdEscapeStringXml,
    StdParseInt,
    StdParseOctal,
    StdParseHex,
    StdParseJson,
    StdParseYaml,
    StdEncodeUtf8,
    StdDecodeUtf8,
    StdDecodeUtf8CheckItem,
    StdDecodeUtf8Finish,
    StdManifestJsonEx,
    StdManifestYamlDoc,
    StdManifestYamlStream,
    StdMakeArray,
    StdCount {
        value: GcView<ThunkData>,
    },
    StdCountInner {
        array: GcView<ArrayData>,
    },
    StdCountCheckItem {
        value: ValueData,
        array: GcView<ArrayData>,
        index: usize,
        count: usize,
    },
    StdFind {
        value: GcView<ThunkData>,
    },
    StdFindInner {
        array: GcView<ArrayData>,
    },
    StdFindCheckItem {
        value: ValueData,
        array: GcView<ArrayData>,
        index: usize,
    },
    StdFilter,
    StdFilterCheck {
        item: GcView<ThunkData>,
    },
    StdFoldl {
        init: GcView<ThunkData>,
    },
    StdFoldlItem {
        func: GcView<FuncData>,
        item: GcView<ThunkData>,
    },
    StdFoldr {
        init: GcView<ThunkData>,
    },
    StdFoldrItem {
        func: GcView<FuncData>,
        item: GcView<ThunkData>,
    },
    StdRange,
    StdSlice,
    StdJoin,
    StdJoinStrItem {
        sep: Rc<str>,
    },
    StdJoinStrFinish,
    StdJoinArrayItem {
        sep: GcView<ArrayData>,
    },
    StdJoinArrayFinish,
    StdReverse,
    StdSort,
    StdSortSetKey {
        keys: Rc<Vec<OnceCell<ValueData>>>,
        index: usize,
    },
    StdSortCompare {
        keys: Rc<Vec<OnceCell<ValueData>>>,
        lhs: usize,
        rhs: usize,
    },
    StdSortSlice {
        keys: Rc<Vec<OnceCell<ValueData>>>,
        sorted: Rc<Vec<Cell<usize>>>,
        range: std::ops::Range<usize>,
    },
    StdSortQuickSort1 {
        keys: Rc<Vec<OnceCell<ValueData>>>,
        sorted: Rc<Vec<Cell<usize>>>,
        range: std::ops::Range<usize>,
    },
    StdSortQuickSort2 {
        keys: Rc<Vec<OnceCell<ValueData>>>,
        sorted: Rc<Vec<Cell<usize>>>,
        range: std::ops::Range<usize>,
    },
    StdSortMergePrepare {
        keys: Rc<Vec<OnceCell<ValueData>>>,
        sorted: Rc<Vec<Cell<usize>>>,
        range: std::ops::Range<usize>,
        mid: usize,
    },
    StdSortMergePreCompare {
        keys: Rc<Vec<OnceCell<ValueData>>>,
        sorted: Rc<Vec<Cell<usize>>>,
        start: usize,
        unmerged: Rc<(Cell<usize>, Box<[usize]>, Cell<usize>, Box<[usize]>)>,
    },
    StdSortMergePostCompare {
        keys: Rc<Vec<OnceCell<ValueData>>>,
        sorted: Rc<Vec<Cell<usize>>>,
        start: usize,
        unmerged: Rc<(Cell<usize>, Box<[usize]>, Cell<usize>, Box<[usize]>)>,
    },
    StdSortFinish {
        orig_array: GcView<ArrayData>,
        sorted: Rc<Vec<Cell<usize>>>,
    },
    StdUniq,
    StdUniqCompareItem {
        keyf: GcView<FuncData>,
        item: GcView<ThunkData>,
        is_last: bool,
    },
    StdUniqDupValue,
    StdUniqCheckItem {
        item: GcView<ThunkData>,
    },
    StdAll,
    StdAllItem {
        array: GcView<ArrayData>,
        index: usize,
    },
    StdAny,
    StdAnyItem {
        array: GcView<ArrayData>,
        index: usize,
    },
    StdSet,
    StdSetUniq {
        orig_array: GcView<ArrayData>,
        keys: Rc<Vec<OnceCell<ValueData>>>,
        sorted: Rc<Vec<Cell<usize>>>,
    },
    StdSetUniqCompareItem {
        orig_array: GcView<ArrayData>,
        keys: Rc<Vec<OnceCell<ValueData>>>,
        sorted: Rc<Vec<Cell<usize>>>,
        index: usize,
    },
    StdSetUniqCheckItem {
        orig_array: GcView<ArrayData>,
        sorted: Rc<Vec<Cell<usize>>>,
        index: usize,
    },
    StdSetInter,
    StdSetInterAux {
        keyf: GcView<FuncData>,
        a: GcView<ArrayData>,
        b: GcView<ArrayData>,
        i: usize,
        j: usize,
    },
    StdSetUnion,
    StdSetUnionAux {
        keyf: GcView<FuncData>,
        a: GcView<ArrayData>,
        b: GcView<ArrayData>,
        i: usize,
        j: usize,
    },
    StdSetDiff,
    StdSetDiffAux {
        keyf: GcView<FuncData>,
        a: GcView<ArrayData>,
        b: GcView<ArrayData>,
        i: usize,
        j: usize,
    },
    StdSetMember {
        x: GcView<ThunkData>,
    },
    StdSetMemberSlice {
        keyf: GcView<FuncData>,
        arr: GcView<ArrayData>,
        start: usize,
        end: usize,
    },
    StdSetMemberCheck {
        keyf: GcView<FuncData>,
        arr: GcView<ArrayData>,
        start: usize,
        end: usize,
        mid: usize,
    },
    StdBase64,
    StdBase64Array {
        input: GcView<ArrayData>,
        bytes: Vec<u8>,
    },
    StdBase64DecodeBytes,
    StdBase64Decode,
    StdMd5,
    StdNative,
    StdTrace,
}
