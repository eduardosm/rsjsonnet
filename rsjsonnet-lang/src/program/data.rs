use std::cell::{Cell, OnceCell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use super::{ir, Program};
use crate::gc::{Gc, GcTrace, GcTraceCtx, GcView};
use crate::interner::{InternedStr, SortedInternedStr};
use crate::{ast, FHashMap};

impl Program {
    pub(super) fn try_value_from_expr(&self, expr: &ir::Expr) -> Option<ValueData> {
        match *expr {
            ir::Expr::Null => Some(ValueData::Null),
            ir::Expr::Bool(value) => Some(ValueData::Bool(value)),
            ir::Expr::Number(value, _) if value.is_finite() => Some(ValueData::Number(value)),
            ir::Expr::String(ref s) => Some(ValueData::String(s.clone())),
            ir::Expr::Array(ref items) if items.is_empty() => {
                Some(ValueData::Array(Gc::from(&self.empty_array)))
            }
            _ => None,
        }
    }

    pub(super) fn new_pending_expr_thunk(
        &self,
        expr: ir::RcExpr,
        env: Gc<ThunkEnv>,
        func_name: Option<&InternedStr>,
    ) -> Gc<ThunkData> {
        let thunk = if let ir::Expr::Func {
            ref params,
            ref body,
        } = *expr
        {
            ThunkData::new_done(ValueData::Function(self.gc_alloc(FuncData::new(
                params.clone(),
                FuncKind::Normal {
                    name: func_name.cloned(),
                    body: body.clone(),
                    env,
                },
            ))))
        } else if let Some(value) = self.try_value_from_expr(&expr) {
            ThunkData::new_done(value)
        } else {
            ThunkData::new_pending_expr(expr, env)
        };
        self.gc_alloc(thunk)
    }

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
                self.new_pending_expr_thunk(local_value.clone(), Gc::from(&env), Some(local_name)),
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
        let thunk = field.thunk.get_or_init(|| {
            let (expr, plus) = field.expr.as_ref().unwrap();
            let env = if let Some(base_env) = field.base_env.as_ref() {
                self.init_object_env(object, core_i, base_env)
            } else {
                self.get_object_core_env(object, core_i)
            };
            let thunk = if *plus {
                ThunkData::new_pending_field_plus(expr.clone(), name.clone(), env)
            } else {
                ThunkData::new_pending_expr(expr.clone(), env)
            };
            self.gc_alloc(thunk)
        });
        Some(thunk.view())
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

            ObjectCore {
                is_top: core.is_top,
                locals: core.locals.clone(),
                base_env: core.base_env.clone(),
                env: OnceCell::new(),
                fields: new_fields,
                asserts: core.asserts.clone(),
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
    fn trace<'a>(&self, ctx: &mut impl GcTraceCtx<'a>)
    where
        Self: 'a,
    {
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
    pub(super) fn new_pending_expr(expr: ir::RcExpr, env: Gc<ThunkEnv>) -> Self {
        Self {
            state: RefCell::new(ThunkState::Pending(PendingThunk::Expr { expr, env })),
        }
    }

    #[inline]
    pub(super) fn new_pending_field_plus(
        expr: ir::RcExpr,
        field: InternedStr,
        env: Gc<ThunkEnv>,
    ) -> Self {
        Self {
            state: RefCell::new(ThunkState::Pending(PendingThunk::FieldPlus {
                expr,
                field,
                env,
            })),
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

    #[inline]
    pub(super) fn get_value(&self) -> Option<ValueData> {
        match *self.state.borrow() {
            ThunkState::Done(ref value) => Some(value.clone()),
            _ => None,
        }
    }
}

pub(super) enum ThunkState {
    Done(ValueData),
    Pending(PendingThunk),
    InProgress,
}

impl GcTrace for ThunkState {
    fn trace<'a>(&self, ctx: &mut impl GcTraceCtx<'a>)
    where
        Self: 'a,
    {
        match self {
            Self::Done(value) => value.trace(ctx),
            Self::Pending(pending) => pending.trace(ctx),
            Self::InProgress => {}
        }
    }
}

pub(super) enum PendingThunk {
    Expr {
        expr: ir::RcExpr,
        env: Gc<ThunkEnv>,
    },
    FieldPlus {
        expr: ir::RcExpr,
        field: InternedStr,
        env: Gc<ThunkEnv>,
    },
    Call {
        func: Gc<FuncData>,
        args: Box<[Gc<ThunkData>]>,
    },
}

impl GcTrace for PendingThunk {
    fn trace<'a>(&self, ctx: &mut impl GcTraceCtx<'a>)
    where
        Self: 'a,
    {
        match self {
            Self::Expr { env, .. } => env.trace(ctx),
            Self::FieldPlus { env, .. } => env.trace(ctx),
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
    fn trace<'a>(&self, ctx: &mut impl GcTraceCtx<'a>)
    where
        Self: 'a,
    {
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
    pub(super) fields_order: OnceCell<Box<[(InternedStr, bool)]>>,
    pub(super) asserts_checked: Cell<bool>,
}

impl GcTrace for ObjectData {
    fn trace<'a>(&self, ctx: &mut impl GcTraceCtx<'a>)
    where
        Self: 'a,
    {
        self.self_core.trace(ctx);
        self.super_cores.trace(ctx);
    }
}

impl ObjectData {
    #[inline]
    pub(super) fn new_simple(fields: FHashMap<InternedStr, ObjectField>) -> Self {
        Self {
            self_core: ObjectCore {
                is_top: false,
                locals: Rc::new(Vec::new()),
                base_env: None,
                env: OnceCell::new(),
                fields,
                asserts: Rc::new(Vec::new()),
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

    pub(super) fn get_fields_order(&self) -> &[(InternedStr, bool)] {
        self.fields_order.get_or_init(|| {
            let mut all_fields = BTreeMap::new();
            all_fields.extend(
                self.self_core
                    .fields
                    .iter()
                    .map(|(n, f)| (SortedInternedStr(n.clone()), f.visibility)),
            );
            for core in self.super_cores.iter() {
                for (n, f) in core.fields.iter() {
                    match all_fields.entry(SortedInternedStr(n.clone())) {
                        std::collections::btree_map::Entry::Vacant(entry) => {
                            entry.insert(f.visibility);
                        }
                        std::collections::btree_map::Entry::Occupied(mut entry) => {
                            if *entry.get() == ast::Visibility::Default {
                                *entry.get_mut() = f.visibility;
                            }
                        }
                    }
                }
            }
            all_fields
                .into_iter()
                .map(|(n, vis)| (n.0, vis != ast::Visibility::Hidden))
                .collect()
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
    pub(super) locals: Rc<Vec<(InternedStr, ir::RcExpr)>>,
    pub(super) base_env: Option<Gc<ThunkEnv>>,
    pub(super) env: OnceCell<Gc<ThunkEnv>>,
    pub(super) fields: FHashMap<InternedStr, ObjectField>,
    pub(super) asserts: Rc<Vec<ir::Assert>>,
}

impl GcTrace for ObjectCore {
    fn trace<'a>(&self, ctx: &mut impl GcTraceCtx<'a>)
    where
        Self: 'a,
    {
        self.base_env.trace(ctx);
        self.env.trace(ctx);
        for field in self.fields.values() {
            field.trace(ctx);
        }
    }
}

pub(super) struct ObjectField {
    pub(super) base_env: Option<Gc<ThunkEnv>>,
    pub(super) visibility: ast::Visibility,
    pub(super) expr: Option<(ir::RcExpr, bool)>,
    pub(super) thunk: OnceCell<Gc<ThunkData>>,
}

impl GcTrace for ObjectField {
    fn trace<'a>(&self, ctx: &mut impl GcTraceCtx<'a>)
    where
        Self: 'a,
    {
        self.base_env.trace(ctx);
        self.thunk.trace(ctx);
    }
}

pub(super) struct FuncData {
    pub(super) params: FuncParams,
    pub(super) kind: FuncKind,
}

impl GcTrace for FuncData {
    #[inline]
    fn trace<'a>(&self, ctx: &mut impl GcTraceCtx<'a>)
    where
        Self: 'a,
    {
        self.kind.trace(ctx);
    }
}

impl FuncData {
    pub(super) fn new(
        params_order: Rc<Vec<(InternedStr, Option<ir::RcExpr>)>>,
        kind: FuncKind,
    ) -> Self {
        let mut params_by_name = FHashMap::default();
        for (i, (name, _)) in params_order.iter().enumerate() {
            let prev = params_by_name.insert(name.clone(), i);
            assert!(prev.is_none(), "repeated parameter name: {name:?}");
        }
        Self {
            params: FuncParams {
                order: params_order,
                by_name: params_by_name,
            },
            kind,
        }
    }
}

pub(super) struct FuncParams {
    pub(super) order: Rc<Vec<(InternedStr, Option<ir::RcExpr>)>>,
    pub(super) by_name: FHashMap<InternedStr, usize>,
}

pub(super) enum FuncKind {
    Normal {
        name: Option<InternedStr>,
        body: ir::RcExpr,
        env: Gc<ThunkEnv>,
    },
    BuiltIn {
        name: InternedStr,
        kind: BuiltInFunc,
    },
    Native {
        name: InternedStr,
    },
}

impl GcTrace for FuncKind {
    #[inline]
    fn trace<'a>(&self, ctx: &mut impl GcTraceCtx<'a>)
    where
        Self: 'a,
    {
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
    Split,
    SplitLimit,
    SplitLimitR,
    StrReplace,
    AsciiUpper,
    AsciiLower,
    StringChars,
    Format,
    EscapeStringJson,
    EscapeStringPython,
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
    ManifestIni,
    ManifestPython,
    ManifestPythonVars,
    ManifestJsonEx,
    ManifestYamlDoc,
    ManifestYamlStream,
    ManifestXmlJsonml,
    ManifestTomlEx,
    // Arrays
    MakeArray,
    Member,
    Count,
    Find,
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
    Base64,
    Base64DecodeBytes,
    Base64Decode,
    Md5,
    // Native Functions
    Native,
    // Debugging
    Trace,
    // Other
    Mod,
}

pub(super) struct ThunkEnv {
    data: OnceCell<ThunkEnvData>,
}

impl GcTrace for ThunkEnv {
    fn trace<'a>(&self, ctx: &mut impl GcTraceCtx<'a>)
    where
        Self: 'a,
    {
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
    vars: FHashMap<InternedStr, Gc<ThunkData>>,
    object: Option<ThunkEnvObject>,
}

impl GcTrace for ThunkEnvData {
    #[inline]
    fn trace<'a>(&self, ctx: &mut impl GcTraceCtx<'a>)
    where
        Self: 'a,
    {
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
    fn trace<'a>(&self, ctx: &mut impl GcTraceCtx<'a>)
    where
        Self: 'a,
    {
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
            vars: FHashMap::default(),
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
