use std::cell::{Cell, OnceCell, RefCell};
use std::collections::{BTreeSet, HashMap};
use std::rc::Rc;

use super::{ir, Program};
use crate::ast;
use crate::gc::{Gc, GcTrace, GcTraceCtx, GcView};
use crate::interner::{InternedStr, SortedInternedStr};
use crate::span::SpanId;

impl Program {
    #[inline]
    pub(super) fn make_value_array(
        &mut self,
        items: impl IntoIterator<Item = ValueData>,
    ) -> Gc<ArrayData> {
        let items: Box<[_]> = items
            .into_iter()
            .map(|v| self.gc_alloc(ThunkData::new_done(v)))
            .collect();
        if items.is_empty() {
            Gc::from(&self.empty_array)
        } else {
            self.gc_alloc(items)
        }
    }

    #[inline]
    pub(super) fn concat_arrays(
        &mut self,
        lhs: &GcView<ArrayData>,
        rhs: &GcView<ArrayData>,
    ) -> Gc<ArrayData> {
        if lhs.is_empty() {
            Gc::from(rhs)
        } else if rhs.is_empty() {
            Gc::from(lhs)
        } else {
            let mut items = Vec::with_capacity(lhs.len() + rhs.len());
            items.extend(lhs.iter().cloned());
            items.extend(rhs.iter().cloned());
            self.gc_alloc(items.into_boxed_slice())
        }
    }

    #[inline]
    pub(super) fn slice_array(
        &mut self,
        array: &GcView<ArrayData>,
        start: usize,
        end: usize,
        step: usize,
    ) -> Gc<ArrayData> {
        let items: Box<[_]> = array
            .iter()
            .skip(start)
            .take(end - start)
            .step_by(step)
            .cloned()
            .collect();
        if items.is_empty() {
            Gc::from(&self.empty_array)
        } else {
            self.gc_alloc(items)
        }
    }

    fn init_object_env(
        &self,
        object: &GcView<ObjectData>,
        core_i: usize,
        base_env: &Gc<ThunkEnv>,
    ) -> Gc<ThunkEnv> {
        let core = object.get_core(core_i);
        let env = self.gc_alloc_view(ThunkEnv::new());
        let mut env_data = ThunkEnvData::new(Some(base_env.clone()));
        for (local_name, local_value) in core.locals.iter() {
            env_data.set_var(
                local_name.clone(),
                self.gc_alloc(ThunkData::new_pending_expr(
                    local_value.clone(),
                    Gc::from(&env),
                )),
            );
        }
        let top_obj = if core.is_top {
            Gc::from(object)
        } else {
            env_data.get_top_object()
        };
        env_data.set_object(ThunkEnvObject {
            object: Gc::from(object),
            core_i,
            top: top_obj,
        });
        env.set_data(env_data);
        Gc::from(&env)
    }

    fn get_object_core_env(&self, object: &GcView<ObjectData>, core_i: usize) -> Gc<ThunkEnv> {
        let core = object.get_core(core_i);
        core.env
            .get_or_init(|| self.init_object_env(object, core_i, core.base_env.as_ref().unwrap()))
            .clone()
    }

    pub(super) fn find_object_field_thunk(
        &self,
        object: &GcView<ObjectData>,
        core_i: usize,
        name: &InternedStr,
    ) -> Option<GcView<ThunkData>> {
        let (core_i, field) = object.find_field(core_i, name)?;
        let thunk = self.get_object_field_thunk(object, core_i, field).view();
        Some(thunk)
    }

    pub(super) fn get_object_field_thunk<'a>(
        &self,
        object: &GcView<ObjectData>,
        core_i: usize,
        field: &'a ObjectField,
    ) -> &'a Gc<ThunkData> {
        field.thunk.get_or_init(|| {
            let expr = field.expr.as_ref().unwrap();
            let env = if let Some(base_env) = field.base_env.as_ref() {
                self.init_object_env(object, core_i, base_env)
            } else {
                self.get_object_core_env(object, core_i)
            };
            self.gc_alloc(ThunkData::new_pending_expr(expr.clone(), env))
        })
    }

    pub(super) fn get_object_assert_env(
        &self,
        object: &GcView<ObjectData>,
        core_i: usize,
        _assert_i: usize,
    ) -> GcView<ThunkEnv> {
        self.get_object_core_env(object, core_i).view()
    }

    pub(super) fn extend_object(&mut self, lhs: &ObjectData, rhs: &ObjectData) -> Gc<ObjectData> {
        let clone_core = |core: &ObjectCore| -> ObjectCore {
            let new_fields = core
                .fields
                .iter()
                .map(|(name, field)| {
                    (
                        name.clone(),
                        ObjectField {
                            base_env: field.base_env.clone(),
                            visibility: field.visibility,
                            expr: field.expr.clone(),
                            thunk: match field.expr {
                                Some(_) => OnceCell::new(),
                                None => field.thunk.clone(),
                            },
                        },
                    )
                })
                .collect();

            let new_asserts = core
                .asserts
                .iter()
                .map(|assert| ObjectAssert {
                    cond: assert.cond.clone(),
                    cond_span: assert.cond_span,
                    msg: assert.msg.clone(),
                    assert_span: assert.assert_span,
                })
                .collect();

            ObjectCore {
                is_top: core.is_top,
                locals: core.locals.clone(),
                base_env: core.base_env.clone(),
                env: OnceCell::new(),
                fields: new_fields,
                asserts: new_asserts,
            }
        };

        let self_core = clone_core(&rhs.self_core);

        let mut super_cores = Vec::with_capacity(lhs.super_cores.len() + rhs.super_cores.len() + 1);
        super_cores.extend(rhs.super_cores.iter().map(clone_core));
        super_cores.push(clone_core(&lhs.self_core));
        super_cores.extend(lhs.super_cores.iter().map(clone_core));

        self.gc_alloc(ObjectData {
            self_core,
            super_cores,
            fields_order: OnceCell::new(),
            asserts_checked: Cell::new(false),
        })
    }
}

pub(super) struct ThunkData {
    state: RefCell<ThunkState>,
}

impl GcTrace for ThunkData {
    fn trace(&self, ctx: &mut GcTraceCtx) {
        self.state.borrow().trace(ctx);
    }
}

impl ThunkData {
    #[inline]
    pub(super) fn new_done(value: ValueData) -> Self {
        Self {
            state: RefCell::new(ThunkState::Done(value)),
        }
    }

    #[inline]
    pub(super) fn new_pending_expr(expr: Rc<ir::Expr>, env: Gc<ThunkEnv>) -> Self {
        Self {
            state: RefCell::new(ThunkState::Pending(PendingThunk::Expr { expr, env })),
        }
    }

    #[inline]
    pub(super) fn new_pending_call(func: Gc<FuncData>, args: Box<[Gc<Self>]>) -> Self {
        Self {
            state: RefCell::new(ThunkState::Pending(PendingThunk::Call { func, args })),
        }
    }

    #[inline]
    pub(super) fn state(&self) -> std::cell::Ref<'_, ThunkState> {
        self.state.borrow()
    }

    #[inline]
    pub(crate) fn switch_state(&self) -> ThunkState {
        let mut state = self.state.borrow_mut();
        match *state {
            ThunkState::Done(ref value) => ThunkState::Done(value.clone()),
            ThunkState::Pending(_) => std::mem::replace(&mut *state, ThunkState::InProgress),
            ThunkState::InProgress => ThunkState::InProgress,
        }
    }

    #[inline]
    pub(super) fn set_done(&self, value: ValueData) {
        let mut state = self.state.borrow_mut();
        assert!(matches!(*state, ThunkState::InProgress));
        *state = ThunkState::Done(value);
    }
}

pub(super) enum ThunkState {
    Done(ValueData),
    Pending(PendingThunk),
    InProgress,
}

impl GcTrace for ThunkState {
    fn trace(&self, ctx: &mut GcTraceCtx) {
        match self {
            Self::Done(value) => value.trace(ctx),
            Self::Pending(pending) => pending.trace(ctx),
            Self::InProgress => {}
        }
    }
}

pub(super) enum PendingThunk {
    Expr {
        expr: Rc<ir::Expr>,
        env: Gc<ThunkEnv>,
    },
    Call {
        func: Gc<FuncData>,
        args: Box<[Gc<ThunkData>]>,
    },
}

impl GcTrace for PendingThunk {
    fn trace(&self, ctx: &mut GcTraceCtx) {
        match self {
            Self::Expr { env, .. } => env.trace(ctx),
            Self::Call { func, args } => {
                func.trace(ctx);
                args.trace(ctx);
            }
        }
    }
}

#[derive(Clone)]
pub(super) enum ValueData {
    Null,
    Bool(bool),
    Number(f64),
    String(Rc<str>),
    Array(Gc<ArrayData>),
    Object(Gc<ObjectData>),
    Function(Gc<FuncData>),
}

impl GcTrace for ValueData {
    fn trace(&self, ctx: &mut GcTraceCtx) {
        match self {
            Self::Array(array) => array.trace(ctx),
            Self::Object(object) => object.trace(ctx),
            Self::Function(func) => func.trace(ctx),
            _ => {}
        }
    }
}

impl ValueData {
    pub(super) fn from_char(chr: char) -> Self {
        let mut buf = [0; 4];
        let chr_str: &str = chr.encode_utf8(&mut buf);
        Self::String(chr_str.into())
    }

    #[inline]
    pub(super) fn might_need_deep(&self) -> bool {
        match self {
            Self::Null => false,
            Self::Bool(_) => false,
            Self::Number(_) => false,
            Self::String(_) => false,
            Self::Array(_) => true,
            Self::Object(_) => true,
            Self::Function(_) => false,
        }
    }
}

pub(super) type ArrayData = Box<[Gc<ThunkData>]>;

pub(super) struct ObjectData {
    pub(super) self_core: ObjectCore,
    pub(super) super_cores: Vec<ObjectCore>,
    pub(super) fields_order: OnceCell<Box<[InternedStr]>>,
    pub(super) asserts_checked: Cell<bool>,
}

impl GcTrace for ObjectData {
    fn trace(&self, ctx: &mut GcTraceCtx) {
        self.self_core.trace(ctx);
        self.super_cores.trace(ctx);
    }
}

impl ObjectData {
    #[inline]
    pub(super) fn new_simple(fields: HashMap<InternedStr, ObjectField>) -> Self {
        Self {
            self_core: ObjectCore {
                is_top: false,
                locals: Rc::new(HashMap::new()),
                base_env: None,
                env: OnceCell::new(),
                fields,
                asserts: Vec::new(),
            },
            super_cores: Vec::new(),
            fields_order: OnceCell::new(),
            asserts_checked: Cell::new(true),
        }
    }

    #[inline]
    pub(super) fn get_core(&self, core_i: usize) -> &ObjectCore {
        if core_i == 0 {
            &self.self_core
        } else {
            &self.super_cores[core_i - 1]
        }
    }

    pub(super) fn find_field(
        &self,
        mut core_i: usize,
        name: &InternedStr,
    ) -> Option<(usize, &ObjectField)> {
        if core_i == 0 {
            if let Some(field) = self.self_core.fields.get(name) {
                return Some((0, field));
            }
            core_i += 1;
        }
        for (sub_i, core) in self.super_cores[(core_i - 1)..].iter().enumerate() {
            if let Some(field) = core.fields.get(name) {
                return Some((core_i + sub_i, field));
            }
        }
        None
    }

    pub(super) fn has_field(&self, core_i: usize, name: &InternedStr) -> bool {
        self.find_field(core_i, name).is_some()
    }

    pub(super) fn get_fields_order(&self) -> &[InternedStr] {
        self.fields_order.get_or_init(|| {
            let mut all_names = BTreeSet::new();
            all_names.extend(
                self.self_core
                    .fields
                    .keys()
                    .map(|n| SortedInternedStr(n.clone())),
            );
            for core in self.super_cores.iter() {
                all_names.extend(core.fields.keys().map(|n| SortedInternedStr(n.clone())));
            }
            all_names.into_iter().map(|n| n.0).collect()
        })
    }

    pub(super) fn field_is_visible(&self, name: &InternedStr) -> bool {
        if let Some(field) = self.self_core.fields.get(name) {
            match field.visibility {
                ast::Visibility::Default => {}
                ast::Visibility::Hidden => return false,
                ast::Visibility::ForceVisible => return true,
            }
        }
        for core in self.super_cores.iter() {
            if let Some(field) = core.fields.get(name) {
                match field.visibility {
                    ast::Visibility::Default => {}
                    ast::Visibility::Hidden => return false,
                    ast::Visibility::ForceVisible => return true,
                }
            }
        }
        true
    }
}

pub(super) struct ObjectCore {
    pub(super) is_top: bool,
    pub(super) locals: Rc<HashMap<InternedStr, Rc<ir::Expr>>>,
    pub(super) base_env: Option<Gc<ThunkEnv>>,
    pub(super) env: OnceCell<Gc<ThunkEnv>>,
    pub(super) fields: HashMap<InternedStr, ObjectField>,
    pub(super) asserts: Vec<ObjectAssert>,
}

impl GcTrace for ObjectCore {
    fn trace(&self, ctx: &mut GcTraceCtx) {
        self.base_env.trace(ctx);
        self.env.trace(ctx);
        for field in self.fields.values() {
            field.trace(ctx);
        }
        self.asserts.trace(ctx);
    }
}

pub(super) struct ObjectField {
    pub(super) base_env: Option<Gc<ThunkEnv>>,
    pub(super) visibility: ast::Visibility,
    pub(super) expr: Option<Rc<ir::Expr>>,
    pub(super) thunk: OnceCell<Gc<ThunkData>>,
}

impl GcTrace for ObjectField {
    fn trace(&self, ctx: &mut GcTraceCtx) {
        self.base_env.trace(ctx);
        self.thunk.trace(ctx);
    }
}

pub(super) struct ObjectAssert {
    pub(super) cond: Rc<ir::Expr>,
    pub(super) cond_span: SpanId,
    pub(super) msg: Option<Rc<ir::Expr>>,
    pub(super) assert_span: SpanId,
}

impl GcTrace for ObjectAssert {
    fn trace(&self, ctx: &mut GcTraceCtx) {
        let _ = ctx;
    }
}

pub(crate) enum FuncData {
    Normal {
        name: Option<InternedStr>,
        params: Rc<ir::FuncParams>,
        body: Rc<ir::Expr>,
        env: Gc<ThunkEnv>,
    },
    BuiltIn {
        name: InternedStr,
        params: Rc<ir::FuncParams>,
        kind: BuiltInFunc,
    },
    Native {
        name: InternedStr,
        params: Rc<ir::FuncParams>,
    },
}

impl GcTrace for FuncData {
    #[inline]
    fn trace(&self, ctx: &mut GcTraceCtx) {
        if let Self::Normal { env, .. } = self {
            env.trace(ctx);
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub(super) enum BuiltInFunc {
    Identity,
    // External Variables
    ExtVar,
    // Types and Reflection
    Type,
    IsArray,
    IsBoolean,
    IsFunction,
    IsNumber,
    IsObject,
    IsString,
    Length,
    ObjectHasEx,
    ObjectFieldsEx,
    PrimitiveEquals,
    Equals,
    Compare,
    CompareArray,
    // Mathematical Utilities
    Exponent,
    Mantissa,
    Floor,
    Ceil,
    Modulo,
    Pow,
    Exp,
    Log,
    Sqrt,
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    // Assertions
    AssertEqual,
    // String Manipulation
    ToString,
    Codepoint,
    Char,
    Substr,
    FindSubstr,
    StartsWith,
    EndsWith,
    SplitLimit,
    SplitLimitR,
    StrReplace,
    AsciiUpper,
    AsciiLower,
    StringChars,
    Format,
    EscapeStringJson,
    EscapeStringBash,
    EscapeStringDollars,
    EscapeStringXml,
    // Parsing
    ParseInt,
    ParseOctal,
    ParseHex,
    ParseJson,
    ParseYaml,
    EncodeUtf8,
    DecodeUtf8,
    // Manifestation
    ManifestJsonEx,
    // Arrays
    MakeArray,
    Filter,
    Foldl,
    Foldr,
    Range,
    Slice,
    Join,
    Reverse,
    Sort,
    Uniq,
    All,
    Any,
    // Sets
    Set,
    SetInter,
    SetUnion,
    SetDiff,
    SetMember,
    // Encoding
    Md5,
    // Native Functions
    Native,
    // Debugging
    Trace,
}

pub(super) struct ThunkEnv {
    data: OnceCell<ThunkEnvData>,
}

impl GcTrace for ThunkEnv {
    fn trace(&self, ctx: &mut GcTraceCtx) {
        self.data.trace(ctx);
    }
}

impl From<ThunkEnvData> for ThunkEnv {
    #[inline]
    fn from(data: ThunkEnvData) -> Self {
        Self {
            data: OnceCell::from(data),
        }
    }
}

impl ThunkEnv {
    #[inline]
    pub(super) fn new() -> Self {
        Self {
            data: OnceCell::new(),
        }
    }

    #[inline]
    pub(super) fn set_data(&self, data: ThunkEnvData) {
        self.data.set(data).ok().expect("env data already set");
    }

    #[inline]
    fn data(&self) -> &ThunkEnvData {
        self.data.get().expect("env data not set")
    }

    pub(super) fn get_var(&self, name: &InternedStr) -> Gc<ThunkData> {
        let data = self.data();
        if let Some(var) = data.vars.get(name) {
            var.clone()
        } else {
            let mut env = data.parent.as_ref().map(Gc::view);
            while let Some(parent) = env {
                let parent = parent.data();
                if let Some(var) = parent.vars.get(name) {
                    return var.clone();
                }
                env = parent.parent.as_ref().map(Gc::view);
            }
            panic!("variable not found");
        }
    }

    pub(super) fn get_object(&self) -> (Gc<ObjectData>, usize) {
        let data = self.data();
        let object = data.object.as_ref().unwrap();
        (object.object.clone(), object.core_i)
    }

    pub(super) fn get_top_object(&self) -> Gc<ObjectData> {
        let data = self.data();
        let object = data.object.as_ref().unwrap();
        object.top.clone()
    }
}

pub(super) struct ThunkEnvData {
    parent: Option<Gc<ThunkEnv>>,
    vars: HashMap<InternedStr, Gc<ThunkData>>,
    object: Option<ThunkEnvObject>,
}

impl GcTrace for ThunkEnvData {
    #[inline]
    fn trace(&self, ctx: &mut GcTraceCtx) {
        self.parent.trace(ctx);
        for var in self.vars.values() {
            var.trace(ctx);
        }
        self.object.trace(ctx);
    }
}

#[derive(Clone)]
pub(super) struct ThunkEnvObject {
    pub(super) object: Gc<ObjectData>,
    pub(super) core_i: usize,
    pub(super) top: Gc<ObjectData>,
}

impl GcTrace for ThunkEnvObject {
    #[inline]
    fn trace(&self, ctx: &mut GcTraceCtx) {
        self.object.trace(ctx);
        self.top.trace(ctx);
    }
}

impl ThunkEnvData {
    #[inline]
    pub(super) fn new(parent: Option<Gc<ThunkEnv>>) -> Self {
        let object = parent.as_ref().and_then(|p| p.view().data().object.clone());
        Self {
            parent,
            vars: HashMap::new(),
            object,
        }
    }

    #[inline]
    pub(super) fn set_var(&mut self, name: InternedStr, thunk: Gc<ThunkData>) {
        self.vars.insert(name, thunk);
    }

    #[inline]
    pub(super) fn set_object(&mut self, obj_data: ThunkEnvObject) {
        self.object = Some(obj_data);
    }

    fn get_top_object(&self) -> Gc<ObjectData> {
        let object = self.object.as_ref().unwrap();
        object.top.clone()
    }
}
