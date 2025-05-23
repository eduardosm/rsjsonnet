use std::cell::{Cell, OnceCell};
use std::rc::Rc;

use super::super::{ir, ArrayData, FuncData, ObjectData, ThunkData, ThunkEnv, ValueData};
use super::format::FormatPart;
use super::{EvalResult, Evaluator, ManifestJsonFormat, TraceItem};
use crate::ast;
use crate::gc::{Gc, GcView};
use crate::interner::InternedStr;
use crate::span::SpanId;

#[must_use]
pub(super) enum State<'a, 'p> {
    FnInfallible(fn(&mut Evaluator<'a, 'p>)),
    FnFallible(fn(&mut Evaluator<'a, 'p>) -> EvalResult<()>),
    // Do not push this directly! Use `Evaluator::push_trace_item` instead.
    TraceItem(TraceItem<'p>),
    // Do not push this directly! Use `Evaluator::delay_trace_item` instead.
    DelayedTraceItem,
    DiscardValue,
    DoThunk(GcView<ThunkData<'p>>),
    GotThunk(GcView<ThunkData<'p>>),
    DeepValue,
    SwapLastValues,
    CoerceToString,
    CoerceAppendToString,
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
    ArrayToValue,
    ObjectToValue,
    ManifestIniSection,
    ManifestIniSectionItem {
        name: InternedStr<'p>,
    },
    ManifestPython,
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
    ManifestTomlPeekSubTable,
    ManifestTomlPeekSubTableArrayItem {
        array: GcView<ArrayData<'p>>,
        index: usize,
    },
    ManifestTomlTable {
        object: GcView<ObjectData<'p>>,
        has_header: bool,
        path: Rc<[InternedStr<'p>]>,
        indent: Rc<str>,
    },
    ManifestTomlValue {
        indent: Rc<str>,
        depth: usize,
        single_line: bool,
    },
    Expr {
        expr: &'p ir::Expr<'p>,
        env: GcView<ThunkEnv<'p>>,
    },
    Error {
        span: SpanId,
    },
    Assert {
        assert_span: SpanId,
        cond_span: SpanId,
        msg_expr: Option<(&'p ir::Expr<'p>, GcView<ThunkEnv<'p>>)>,
    },
    AssertMsg {
        assert_span: SpanId,
    },
    ObjectFixField {
        name: InternedStr<'p>,
        name_span: SpanId,
        plus: bool,
        visibility: ast::Visibility,
        value: &'p ir::Expr<'p>,
        base_env: Option<Gc<ThunkEnv<'p>>>,
    },
    ObjectDynField {
        name_span: SpanId,
        plus: bool,
        visibility: ast::Visibility,
        value: &'p ir::Expr<'p>,
        base_env: Option<Gc<ThunkEnv<'p>>>,
    },
    Field {
        span: SpanId,
        field_name: InternedStr<'p>,
    },
    InitCompSpec {
        var_name: InternedStr<'p>,
        value: &'p ir::Expr<'p>,
        value_span: SpanId,
        env: GcView<ThunkEnv<'p>>,
    },
    GotInitCompSpec {
        var_name: InternedStr<'p>,
        value_span: SpanId,
    },
    ForSpec {
        var_name: InternedStr<'p>,
        value: &'p ir::Expr<'p>,
        value_span: SpanId,
        env: GcView<ThunkEnv<'p>>,
    },
    GotForSpec {
        var_name: InternedStr<'p>,
        value_span: SpanId,
    },
    IfSpec {
        cond: &'p ir::Expr<'p>,
        cond_span: SpanId,
        env: GcView<ThunkEnv<'p>>,
    },
    GotIfSpec {
        cond_span: SpanId,
    },
    ArrayComp {
        item: &'p ir::Expr<'p>,
        env: GcView<ThunkEnv<'p>>,
    },
    ObjectComp {
        expr: &'p ir::Expr<'p>,
        env: GcView<ThunkEnv<'p>>,
    },
    FinishObjectComp,
    Index {
        span: SpanId,
    },
    Slice {
        span: SpanId,
        has_start: bool,
        has_end: bool,
        has_step: bool,
    },
    SuperIndex {
        span: SpanId,
        super_span: SpanId,
        env: GcView<ThunkEnv<'p>>,
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
        rhs: &'p ir::Expr<'p>,
        env: GcView<ThunkEnv<'p>>,
    },
    LogicOr {
        span: SpanId,
        rhs: &'p ir::Expr<'p>,
        env: GcView<ThunkEnv<'p>>,
    },
    InSuper {
        span: SpanId,
        env: GcView<ThunkEnv<'p>>,
    },
    EqualsValue,
    EqualsArray {
        lhs: GcView<ArrayData<'p>>,
        rhs: GcView<ArrayData<'p>>,
        index: usize,
    },
    EqualsObject {
        lhs: GcView<ObjectData<'p>>,
        rhs: GcView<ObjectData<'p>>,
        rem_fields: Vec<InternedStr<'p>>,
    },
    CompareValue,
    CompareArray {
        lhs: GcView<ArrayData<'p>>,
        rhs: GcView<ArrayData<'p>>,
        index: usize,
    },
    CallWithExpr {
        call_expr: &'p ir::Expr<'p>,
        call_env: GcView<ThunkEnv<'p>>,
    },
    TopLevelCall {
        pos_args: Box<[GcView<ThunkData<'p>>]>,
        named_args: Box<[(InternedStr<'p>, GcView<ThunkData<'p>>)]>,
    },
    ExecTailstrictCall {
        func: GcView<FuncData<'p>>,
        args: Box<[Gc<ThunkData<'p>>]>,
    },
    ExecNativeCall {
        name: InternedStr<'p>,
        args: Box<[GcView<ThunkData<'p>>]>,
    },
    If {
        cond_span: SpanId,
        then_body: &'p ir::Expr<'p>,
        else_body: Option<&'p ir::Expr<'p>>,
        env: GcView<ThunkEnv<'p>>,
    },
    StdPruneValue,
    StdPruneArrayItem,
    StdPruneObjectField {
        name: InternedStr<'p>,
    },
    StdFormat,
    StdFormatCodesArray1 {
        parts: Rc<Vec<FormatPart>>,
        array: GcView<ArrayData<'p>>,
        part_i: usize,
        array_i: usize,
    },
    StdFormatCodesArray2 {
        parts: Rc<Vec<FormatPart>>,
        array: GcView<ArrayData<'p>>,
        part_i: usize,
        array_i: usize,
    },
    StdFormatCodesArray3 {
        parts: Rc<Vec<FormatPart>>,
        array: GcView<ArrayData<'p>>,
        part_i: usize,
        array_i: usize,
        fw: u32,
    },
    StdFormatCodesObject1 {
        parts: Rc<Vec<FormatPart>>,
        object: GcView<ObjectData<'p>>,
        part_i: usize,
    },
    StdFormatCodesObject2 {
        parts: Rc<Vec<FormatPart>>,
        object: GcView<ObjectData<'p>>,
        part_i: usize,
        fw: u32,
    },
    StdFormatCode {
        parts: Rc<Vec<FormatPart>>,
        part_i: usize,
        fw: u32,
        prec: u32,
    },
    StdManifestIni,
    StdManifestIniSections,
    StdManifestPython,
    StdManifestPythonVars,
    StdManifestJsonEx,
    StdManifestYamlDoc,
    StdManifestYamlStream,
    StdManifestXmlJsonml,
    StdManifestXmlJsonmlItem0 {
        array: GcView<ArrayData<'p>>,
    },
    StdManifestXmlJsonmlItem1,
    StdManifestXmlJsonmlItemN,
    StdManifestTomlEx,
    StdMember {
        value: GcView<ThunkData<'p>>,
    },
    StdMemberString {
        string: Rc<str>,
    },
    StdMemberArray {
        array: GcView<ArrayData<'p>>,
        index: usize,
    },
    StdCount {
        value: GcView<ThunkData<'p>>,
    },
    StdCountInner {
        array: GcView<ArrayData<'p>>,
        index: usize,
        count: usize,
    },
    StdFind {
        value: GcView<ThunkData<'p>>,
    },
    StdFindInner {
        array: GcView<ArrayData<'p>>,
        index: usize,
    },
    StdFilterMap,
    StdFilterMapCheck {
        item: GcView<ThunkData<'p>>,
        map_func: GcView<FuncData<'p>>,
    },
    StdFilter,
    StdFilterCheck {
        item: GcView<ThunkData<'p>>,
    },
    StdFoldl {
        init: GcView<ThunkData<'p>>,
    },
    StdFoldlItem {
        func: GcView<FuncData<'p>>,
        item: GcView<ThunkData<'p>>,
    },
    StdFoldr {
        init: GcView<ThunkData<'p>>,
    },
    StdFoldrItem {
        func: GcView<FuncData<'p>>,
        item: GcView<ThunkData<'p>>,
    },
    StdJoin,
    StdJoinStrItem {
        sep: Rc<str>,
    },
    StdJoinStrFinish,
    StdJoinArrayItem {
        sep: GcView<ArrayData<'p>>,
    },
    StdJoinArrayFinish,
    StdFlattenDeepArray,
    StdFlattenDeepArrayItem {
        array: GcView<ArrayData<'p>>,
        index: usize,
    },
    StdSort,
    StdSortSetKey {
        keys: Rc<Vec<OnceCell<ValueData<'p>>>>,
        index: usize,
    },
    StdSortCompare {
        keys: Rc<Vec<OnceCell<ValueData<'p>>>>,
        lhs: usize,
        rhs: usize,
    },
    StdSortSlice {
        keys: Rc<Vec<OnceCell<ValueData<'p>>>>,
        sorted: Rc<Vec<Cell<usize>>>,
        range: std::ops::Range<usize>,
    },
    StdSortQuickSort1 {
        keys: Rc<Vec<OnceCell<ValueData<'p>>>>,
        sorted: Rc<Vec<Cell<usize>>>,
        range: std::ops::Range<usize>,
    },
    StdSortQuickSort2 {
        keys: Rc<Vec<OnceCell<ValueData<'p>>>>,
        sorted: Rc<Vec<Cell<usize>>>,
        range: std::ops::Range<usize>,
    },
    StdSortMergePrepare {
        keys: Rc<Vec<OnceCell<ValueData<'p>>>>,
        sorted: Rc<Vec<Cell<usize>>>,
        range: std::ops::Range<usize>,
        mid: usize,
    },
    StdSortMergePreCompare {
        keys: Rc<Vec<OnceCell<ValueData<'p>>>>,
        sorted: Rc<Vec<Cell<usize>>>,
        start: usize,
        unmerged: Rc<(Cell<usize>, Box<[usize]>, Cell<usize>, Box<[usize]>)>,
    },
    StdSortMergePostCompare {
        keys: Rc<Vec<OnceCell<ValueData<'p>>>>,
        sorted: Rc<Vec<Cell<usize>>>,
        start: usize,
        unmerged: Rc<(Cell<usize>, Box<[usize]>, Cell<usize>, Box<[usize]>)>,
    },
    StdSortFinish {
        orig_array: GcView<ArrayData<'p>>,
        sorted: Rc<Vec<Cell<usize>>>,
    },
    StdUniq,
    StdUniqCompareItem {
        keyf: GcView<FuncData<'p>>,
        item: GcView<ThunkData<'p>>,
        is_last: bool,
    },
    StdUniqDupValue,
    StdUniqCheckItem {
        item: GcView<ThunkData<'p>>,
    },
    StdAll,
    StdAllItem {
        array: GcView<ArrayData<'p>>,
        index: usize,
    },
    StdAny,
    StdAnyItem {
        array: GcView<ArrayData<'p>>,
        index: usize,
    },
    StdSum,
    StdSumItem {
        array: GcView<ArrayData<'p>>,
        index: usize,
        sum: f64,
    },
    StdAvg,
    StdAvgItem {
        array: GcView<ArrayData<'p>>,
        index: usize,
        sum: f64,
    },
    StdMinArray {
        on_empty: GcView<ThunkData<'p>>,
    },
    StdMinArrayCompareItem {
        keyf: GcView<FuncData<'p>>,
        array: GcView<ArrayData<'p>>,
        cur_index: usize,
        max_index: usize,
    },
    StdMinArrayCheckItem {
        keyf: GcView<FuncData<'p>>,
        array: GcView<ArrayData<'p>>,
        cur_index: usize,
        max_index: usize,
    },
    StdMaxArray {
        on_empty: GcView<ThunkData<'p>>,
    },
    StdMaxArrayCompareItem {
        keyf: GcView<FuncData<'p>>,
        array: GcView<ArrayData<'p>>,
        cur_index: usize,
        max_index: usize,
    },
    StdMaxArrayCheckItem {
        keyf: GcView<FuncData<'p>>,
        array: GcView<ArrayData<'p>>,
        cur_index: usize,
        max_index: usize,
    },
    StdContains {
        value: GcView<ThunkData<'p>>,
    },
    StdContainsItem {
        array: GcView<ArrayData<'p>>,
        index: usize,
    },
    StdRemove {
        value: GcView<ThunkData<'p>>,
    },
    StdRemoveCheckItem {
        array: GcView<ArrayData<'p>>,
        index: usize,
    },
    StdSet,
    StdSetUniq {
        orig_array: GcView<ArrayData<'p>>,
        keys: Rc<Vec<OnceCell<ValueData<'p>>>>,
        sorted: Rc<Vec<Cell<usize>>>,
    },
    StdSetUniqCompareItem {
        orig_array: GcView<ArrayData<'p>>,
        keys: Rc<Vec<OnceCell<ValueData<'p>>>>,
        sorted: Rc<Vec<Cell<usize>>>,
        index: usize,
    },
    StdSetUniqCheckItem {
        orig_array: GcView<ArrayData<'p>>,
        sorted: Rc<Vec<Cell<usize>>>,
        index: usize,
    },
    StdSetInter,
    StdSetInterAux {
        keyf: GcView<FuncData<'p>>,
        a: GcView<ArrayData<'p>>,
        b: GcView<ArrayData<'p>>,
        i: usize,
        j: usize,
    },
    StdSetUnion,
    StdSetUnionAux {
        keyf: GcView<FuncData<'p>>,
        a: GcView<ArrayData<'p>>,
        b: GcView<ArrayData<'p>>,
        i: usize,
        j: usize,
    },
    StdSetDiff,
    StdSetDiffAux {
        keyf: GcView<FuncData<'p>>,
        a: GcView<ArrayData<'p>>,
        b: GcView<ArrayData<'p>>,
        i: usize,
        j: usize,
    },
    StdSetMember {
        x: GcView<ThunkData<'p>>,
    },
    StdSetMemberSlice {
        keyf: GcView<FuncData<'p>>,
        arr: GcView<ArrayData<'p>>,
        start: usize,
        end: usize,
    },
    StdSetMemberCheck {
        keyf: GcView<FuncData<'p>>,
        arr: GcView<ArrayData<'p>>,
        start: usize,
        end: usize,
        mid: usize,
    },
    StdBase64,
    StdBase64Array {
        input: GcView<ArrayData<'p>>,
        bytes: Vec<u8>,
    },
    StdBase64DecodeBytes,
    StdBase64Decode,
    StdMergePatchValue,
    StdMergePatchField {
        name: InternedStr<'p>,
    },
}
