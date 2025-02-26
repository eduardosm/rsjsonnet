use std::cell::{Cell, OnceCell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use super::{ir, Program};
use crate::gc::{Gc, GcTrace, GcTraceCtx, GcView};
use crate::interner::{InternedStr, SortedInternedStr};
use crate::{ast, FHashMap};

impl<'p> Program<'p> {
    pub(super) fn try_value_from_expr(&self, expr: &'p ir::Expr<'p>) -> Option<ValueData<'p>> {
        match *expr {
            ir::Expr::Null => Some(ValueData::Null),
            ir::Expr::Bool(value) => Some(ValueData::Bool(value)),
            ir::Expr::Number(value, _) if value.is_finite() => Some(ValueData::Number(value)),
            ir::Expr::String(s) => Some(ValueData::String(s.into())),
            ir::Expr::Array([]) => Some(ValueData::Array(Gc::from(&self.empty_array))),
            _ => None,
        }
    }

    pub(super) fn new_pending_expr_thunk(
        &self,
        expr: &'p ir::Expr<'p>,
        env: Gc<ThunkEnv<'p>>,
        func_name: Option<InternedStr<'p>>,
    ) -> Gc<ThunkData<'p>> {
        let thunk = if let ir::Expr::Func { params, body } = *expr {
            ThunkData::new_done(ValueData::Function(self.gc_alloc(FuncData::new(
                params,
                FuncKind::Normal {
                    name: func_name,
                    body,
                    env,
                },
            ))))
        } else if let Some(value) = self.try_value_from_expr(expr) {
            ThunkData::new_done(value)
        } else {
            ThunkData::new_pending_expr(expr, env)
        };
        self.gc_alloc(thunk)
    }

    #[inline]
    pub(super) fn make_value_array(
        &mut self,
        items: impl IntoIterator<Item = ValueData<'p>>,
    ) -> Gc<ArrayData<'p>> {
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
        lhs: &GcView<ArrayData<'p>>,
        rhs: &GcView<ArrayData<'p>>,
    ) -> Gc<ArrayData<'p>> {
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
        array: &GcView<ArrayData<'p>>,
        start: usize,
        end: usize,
        step: usize,
    ) -> Gc<ArrayData<'p>> {
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
        object: &GcView<ObjectData<'p>>,
        layer_i: usize,
        base_env: &Gc<ThunkEnv<'p>>,
    ) -> Gc<ThunkEnv<'p>> {
        let layer = object.get_layer(layer_i);
        let env = self.gc_alloc_view(ThunkEnv::new());
        let mut env_data = ThunkEnvData::new(Some(base_env.clone()));
        for &(local_name, local_value) in layer.locals.iter() {
            env_data.set_var(
                local_name,
                self.new_pending_expr_thunk(local_value, Gc::from(&env), Some(local_name)),
            );
        }
        let top_obj = if layer.is_top {
            Gc::from(object)
        } else {
            env_data.get_top_object()
        };
        env_data.set_object(ThunkEnvObject {
            object: Gc::from(object),
            layer_i,
            top: top_obj,
        });
        env.set_data(env_data);
        Gc::from(&env)
    }

    fn get_object_layer_env(
        &self,
        object: &GcView<ObjectData<'p>>,
        layer_i: usize,
    ) -> Gc<ThunkEnv<'p>> {
        let layer = object.get_layer(layer_i);
        layer
            .env
            .get_or_init(|| self.init_object_env(object, layer_i, layer.base_env.as_ref().unwrap()))
            .clone()
    }

    pub(super) fn find_object_field_thunk(
        &self,
        object: &GcView<ObjectData<'p>>,
        layer_i: usize,
        name: InternedStr<'p>,
    ) -> Option<GcView<ThunkData<'p>>> {
        let (layer_i, field) = object.find_field(layer_i, name)?;
        let thunk = field.thunk.get_or_init(|| {
            let (expr, plus) = field.expr.as_ref().unwrap();
            let env = if let Some(base_env) = field.base_env.as_ref() {
                self.init_object_env(object, layer_i, base_env)
            } else {
                self.get_object_layer_env(object, layer_i)
            };
            let thunk = if *plus {
                ThunkData::new_pending_field_plus(expr, name, env)
            } else {
                ThunkData::new_pending_expr(expr, env)
            };
            self.gc_alloc(thunk)
        });
        Some(thunk.view())
    }

    pub(super) fn get_object_assert_env(
        &self,
        object: &GcView<ObjectData<'p>>,
        layer_i: usize,
        _assert_i: usize,
    ) -> GcView<ThunkEnv<'p>> {
        self.get_object_layer_env(object, layer_i).view()
    }

    pub(super) fn extend_object(
        &mut self,
        lhs: &ObjectData<'p>,
        rhs: &ObjectData<'p>,
    ) -> Gc<ObjectData<'p>> {
        let clone_layer = |layer: &ObjectLayer<'p>| -> ObjectLayer<'p> {
            let new_fields = layer
                .fields
                .iter()
                .map(|(name, field)| {
                    (
                        *name,
                        ObjectField {
                            base_env: field.base_env.clone(),
                            visibility: field.visibility,
                            expr: field.expr,
                            thunk: match field.expr {
                                Some(_) => OnceCell::new(),
                                None => field.thunk.clone(),
                            },
                        },
                    )
                })
                .collect();

            ObjectLayer {
                is_top: layer.is_top,
                locals: layer.locals,
                base_env: layer.base_env.clone(),
                env: OnceCell::new(),
                fields: new_fields,
                asserts: layer.asserts,
            }
        };

        let self_layer = clone_layer(&rhs.self_layer);

        let mut super_layers =
            Vec::with_capacity(lhs.super_layers.len() + rhs.super_layers.len() + 1);
        super_layers.extend(rhs.super_layers.iter().map(clone_layer));
        super_layers.push(clone_layer(&lhs.self_layer));
        super_layers.extend(lhs.super_layers.iter().map(clone_layer));

        self.gc_alloc(ObjectData {
            self_layer,
            super_layers,
            fields_order: OnceCell::new(),
            asserts_checked: Cell::new(false),
        })
    }
}

pub(super) struct ThunkData<'p> {
    state: RefCell<ThunkState<'p>>,
}

impl GcTrace for ThunkData<'_> {
    fn trace<'a>(&self, ctx: &mut impl GcTraceCtx<'a>)
    where
        Self: 'a,
    {
        self.state.borrow().trace(ctx);
    }
}

impl<'p> ThunkData<'p> {
    #[inline]
    pub(super) fn new_done(value: ValueData<'p>) -> Self {
        Self {
            state: RefCell::new(ThunkState::Done(value)),
        }
    }

    #[inline]
    pub(super) fn new_pending_expr(expr: &'p ir::Expr<'p>, env: Gc<ThunkEnv<'p>>) -> Self {
        Self {
            state: RefCell::new(ThunkState::Pending(PendingThunk::Expr { expr, env })),
        }
    }

    #[inline]
    pub(super) fn new_pending_field_plus(
        expr: &'p ir::Expr<'p>,
        field: InternedStr<'p>,
        env: Gc<ThunkEnv<'p>>,
    ) -> Self {
        Self {
            state: RefCell::new(ThunkState::Pending(PendingThunk::FieldPlus {
                expr,
                field,
                env,
            })),
        }
    }

    pub(super) fn new_pending_call(func: Gc<FuncData<'p>>, args: Box<[Gc<Self>]>) -> Self {
        Self {
            state: RefCell::new(ThunkState::Pending(PendingThunk::Call { func, args })),
        }
    }

    #[inline]
    pub(super) fn state(&self) -> std::cell::Ref<'_, ThunkState<'p>> {
        self.state.borrow()
    }

    #[inline]
    pub(crate) fn switch_state(&self) -> ThunkState<'p> {
        let mut state = self.state.borrow_mut();
        match *state {
            ThunkState::Done(ref value) => ThunkState::Done(value.clone()),
            ThunkState::Pending(_) => std::mem::replace(&mut *state, ThunkState::InProgress),
            ThunkState::InProgress => ThunkState::InProgress,
        }
    }

    #[inline]
    pub(super) fn set_done(&self, value: ValueData<'p>) {
        let mut state = self.state.borrow_mut();
        assert!(matches!(*state, ThunkState::InProgress));
        *state = ThunkState::Done(value);
    }

    #[inline]
    pub(super) fn get_value(&self) -> Option<ValueData<'p>> {
        match *self.state.borrow() {
            ThunkState::Done(ref value) => Some(value.clone()),
            _ => None,
        }
    }
}

pub(super) enum ThunkState<'p> {
    Done(ValueData<'p>),
    Pending(PendingThunk<'p>),
    InProgress,
}

impl GcTrace for ThunkState<'_> {
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

pub(super) enum PendingThunk<'p> {
    Expr {
        expr: &'p ir::Expr<'p>,
        env: Gc<ThunkEnv<'p>>,
    },
    FieldPlus {
        expr: &'p ir::Expr<'p>,
        field: InternedStr<'p>,
        env: Gc<ThunkEnv<'p>>,
    },
    Call {
        func: Gc<FuncData<'p>>,
        args: Box<[Gc<ThunkData<'p>>]>,
    },
}

impl GcTrace for PendingThunk<'_> {
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
pub(super) enum ValueData<'p> {
    Null,
    Bool(bool),
    Number(f64),
    String(Rc<str>),
    Array(Gc<ArrayData<'p>>),
    Object(Gc<ObjectData<'p>>),
    Function(Gc<FuncData<'p>>),
}

impl GcTrace for ValueData<'_> {
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

impl ValueData<'_> {
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

pub(super) type ArrayData<'p> = Box<[Gc<ThunkData<'p>>]>;

pub(super) struct ObjectData<'p> {
    pub(super) self_layer: ObjectLayer<'p>,
    pub(super) super_layers: Vec<ObjectLayer<'p>>,
    pub(super) fields_order: OnceCell<Box<[(InternedStr<'p>, bool)]>>,
    pub(super) asserts_checked: Cell<bool>,
}

impl GcTrace for ObjectData<'_> {
    fn trace<'a>(&self, ctx: &mut impl GcTraceCtx<'a>)
    where
        Self: 'a,
    {
        self.self_layer.trace(ctx);
        self.super_layers.trace(ctx);
    }
}

impl<'p> ObjectData<'p> {
    #[inline]
    pub(super) fn new_empty() -> Self {
        Self {
            self_layer: ObjectLayer {
                is_top: false,
                locals: &[],
                base_env: None,
                env: OnceCell::new(),
                fields: FHashMap::default(),
                asserts: &[],
            },
            super_layers: Vec::new(),
            fields_order: OnceCell::new(),
            asserts_checked: Cell::new(true),
        }
    }

    #[inline]
    pub(super) fn new_simple(fields: FHashMap<InternedStr<'p>, ObjectField<'p>>) -> Self {
        Self {
            self_layer: ObjectLayer {
                is_top: false,
                locals: &[],
                base_env: None,
                env: OnceCell::new(),
                fields,
                asserts: &[],
            },
            super_layers: Vec::new(),
            fields_order: OnceCell::new(),
            asserts_checked: Cell::new(true),
        }
    }

    #[inline]
    pub(super) fn get_layer(&self, layer_i: usize) -> &ObjectLayer<'p> {
        if layer_i == 0 {
            &self.self_layer
        } else {
            &self.super_layers[layer_i - 1]
        }
    }

    pub(super) fn find_field(
        &self,
        mut layer_i: usize,
        name: InternedStr<'p>,
    ) -> Option<(usize, &ObjectField<'p>)> {
        if layer_i == 0 {
            if let Some(field) = self.self_layer.fields.get(&name) {
                return Some((0, field));
            }
            layer_i += 1;
        }
        for (sub_i, layer) in self.super_layers[(layer_i - 1)..].iter().enumerate() {
            if let Some(field) = layer.fields.get(&name) {
                return Some((layer_i + sub_i, field));
            }
        }
        None
    }

    pub(super) fn has_field(&self, layer_i: usize, name: InternedStr<'p>) -> bool {
        self.find_field(layer_i, name).is_some()
    }

    pub(super) fn get_fields_order(&self) -> &[(InternedStr<'p>, bool)] {
        self.fields_order.get_or_init(|| {
            let mut all_fields = BTreeMap::new();
            all_fields.extend(
                self.self_layer
                    .fields
                    .iter()
                    .map(|(n, f)| (SortedInternedStr(*n), f.visibility)),
            );
            for layer in self.super_layers.iter() {
                for (n, f) in layer.fields.iter() {
                    match all_fields.entry(SortedInternedStr(*n)) {
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

    #[inline]
    pub(super) fn get_visible_fields_order(
        &self,
    ) -> impl DoubleEndedIterator<Item = InternedStr<'p>> + Clone + '_ {
        self.get_fields_order()
            .iter()
            .filter_map(|&(name, visible)| visible.then_some(name))
    }

    pub(super) fn has_visible_field(&self, name: InternedStr<'p>) -> bool {
        let mut found = false;
        if let Some(field) = self.self_layer.fields.get(&name) {
            found = true;
            match field.visibility {
                ast::Visibility::Default => {}
                ast::Visibility::Hidden => return false,
                ast::Visibility::ForceVisible => return true,
            }
        }
        for layer in self.super_layers.iter() {
            if let Some(field) = layer.fields.get(&name) {
                found = true;
                match field.visibility {
                    ast::Visibility::Default => {}
                    ast::Visibility::Hidden => return false,
                    ast::Visibility::ForceVisible => return true,
                }
            }
        }
        found
    }
}

pub(super) struct ObjectLayer<'p> {
    pub(super) is_top: bool,
    pub(super) locals: &'p [(InternedStr<'p>, &'p ir::Expr<'p>)],
    pub(super) base_env: Option<Gc<ThunkEnv<'p>>>,
    pub(super) env: OnceCell<Gc<ThunkEnv<'p>>>,
    pub(super) fields: FHashMap<InternedStr<'p>, ObjectField<'p>>,
    pub(super) asserts: &'p [ir::Assert<'p>],
}

impl GcTrace for ObjectLayer<'_> {
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

pub(super) struct ObjectField<'p> {
    pub(super) base_env: Option<Gc<ThunkEnv<'p>>>,
    pub(super) visibility: ast::Visibility,
    pub(super) expr: Option<(&'p ir::Expr<'p>, bool)>,
    pub(super) thunk: OnceCell<Gc<ThunkData<'p>>>,
}

impl GcTrace for ObjectField<'_> {
    fn trace<'a>(&self, ctx: &mut impl GcTraceCtx<'a>)
    where
        Self: 'a,
    {
        self.base_env.trace(ctx);
        self.thunk.trace(ctx);
    }
}

pub(super) struct FuncData<'p> {
    pub(super) params: FuncParams<'p>,
    pub(super) kind: FuncKind<'p>,
}

impl GcTrace for FuncData<'_> {
    #[inline]
    fn trace<'a>(&self, ctx: &mut impl GcTraceCtx<'a>)
    where
        Self: 'a,
    {
        self.kind.trace(ctx);
    }
}

impl<'p> FuncData<'p> {
    pub(super) fn new(
        params_order: &'p [(InternedStr<'p>, Option<&'p ir::Expr<'p>>)],
        kind: FuncKind<'p>,
    ) -> Self {
        let mut params_by_name = FHashMap::default();
        for (i, &(name, _)) in params_order.iter().enumerate() {
            let prev = params_by_name.insert(name, i);
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

    pub(super) fn new_identity_func(
        func_name: Option<InternedStr<'p>>,
        param: &'p [(InternedStr<'p>, Option<&'p ir::Expr<'p>>); 1],
    ) -> Self {
        Self::new(param, FuncKind::Identity { name: func_name })
    }
}

pub(super) struct FuncParams<'p> {
    pub(super) order: &'p [(InternedStr<'p>, Option<&'p ir::Expr<'p>>)],
    pub(super) by_name: FHashMap<InternedStr<'p>, usize>,
}

pub(super) enum FuncKind<'p> {
    Identity {
        name: Option<InternedStr<'p>>,
    },
    Normal {
        name: Option<InternedStr<'p>>,
        body: &'p ir::Expr<'p>,
        env: Gc<ThunkEnv<'p>>,
    },
    BuiltIn {
        name: InternedStr<'p>,
        kind: BuiltInFunc,
    },
    Native {
        name: InternedStr<'p>,
    },
}

impl GcTrace for FuncKind<'_> {
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
    Prune,
    ObjectHasEx,
    ObjectFieldsEx,
    MapWithKey,
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
    Log2,
    Log10,
    Sqrt,
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Atan2,
    Deg2Rad,
    Rad2Deg,
    Hypot,
    IsEven,
    IsOdd,
    IsInteger,
    IsDecimal,
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
    StripChars,
    LStripChars,
    RStripChars,
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
    Map,
    MapWithIndex,
    FilterMap,
    FlatMap,
    Filter,
    Foldl,
    Foldr,
    Range,
    Repeat,
    Slice,
    Join,
    DeepJoin,
    FlattenArrays,
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
    Sha1,
    Sha256,
    Sha512,
    Sha3,
    // JSON Merge Patch
    MergePatch,
    // Other
    Mod,
    // Native Functions
    Native,
    // Debugging
    Trace,
}

pub(super) struct ThunkEnv<'p> {
    data: OnceCell<ThunkEnvData<'p>>,
}

impl GcTrace for ThunkEnv<'_> {
    fn trace<'a>(&self, ctx: &mut impl GcTraceCtx<'a>)
    where
        Self: 'a,
    {
        self.data.trace(ctx);
    }
}

impl<'p> From<ThunkEnvData<'p>> for ThunkEnv<'p> {
    #[inline]
    fn from(data: ThunkEnvData<'p>) -> Self {
        Self {
            data: OnceCell::from(data),
        }
    }
}

impl<'p> ThunkEnv<'p> {
    #[inline]
    pub(super) fn new() -> Self {
        Self {
            data: OnceCell::new(),
        }
    }

    #[inline]
    pub(super) fn set_data(&self, data: ThunkEnvData<'p>) {
        self.data.set(data).ok().expect("env data already set");
    }

    #[inline]
    fn data(&self) -> &ThunkEnvData<'p> {
        self.data.get().expect("env data not set")
    }

    pub(super) fn get_var(&self, name: InternedStr<'p>) -> Gc<ThunkData<'p>> {
        let data = self.data();
        if let Some(var) = data.vars.get(&name) {
            var.clone()
        } else {
            let mut env = data.parent.as_ref().map(Gc::view);
            while let Some(parent) = env {
                let parent = parent.data();
                if let Some(var) = parent.vars.get(&name) {
                    return var.clone();
                }
                env = parent.parent.as_ref().map(Gc::view);
            }
            panic!("variable not found");
        }
    }

    pub(super) fn get_object(&self) -> (Gc<ObjectData<'p>>, usize) {
        let data = self.data();
        let object = data.object.as_ref().unwrap();
        (object.object.clone(), object.layer_i)
    }

    pub(super) fn get_top_object(&self) -> Gc<ObjectData<'p>> {
        let data = self.data();
        let object = data.object.as_ref().unwrap();
        object.top.clone()
    }
}

pub(super) struct ThunkEnvData<'p> {
    parent: Option<Gc<ThunkEnv<'p>>>,
    vars: FHashMap<InternedStr<'p>, Gc<ThunkData<'p>>>,
    object: Option<ThunkEnvObject<'p>>,
}

impl GcTrace for ThunkEnvData<'_> {
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
pub(super) struct ThunkEnvObject<'p> {
    pub(super) object: Gc<ObjectData<'p>>,
    pub(super) layer_i: usize,
    pub(super) top: Gc<ObjectData<'p>>,
}

impl GcTrace for ThunkEnvObject<'_> {
    #[inline]
    fn trace<'a>(&self, ctx: &mut impl GcTraceCtx<'a>)
    where
        Self: 'a,
    {
        self.object.trace(ctx);
        self.top.trace(ctx);
    }
}

impl<'p> ThunkEnvData<'p> {
    #[inline]
    pub(super) fn new(parent: Option<Gc<ThunkEnv<'p>>>) -> Self {
        let object = parent.as_ref().and_then(|p| p.view().data().object.clone());
        Self {
            parent,
            vars: FHashMap::default(),
            object,
        }
    }

    #[inline]
    pub(super) fn set_var(&mut self, name: InternedStr<'p>, thunk: Gc<ThunkData<'p>>) {
        self.vars.insert(name, thunk);
    }

    #[inline]
    pub(super) fn set_object(&mut self, obj_data: ThunkEnvObject<'p>) {
        self.object = Some(obj_data);
    }

    fn get_top_object(&self) -> Gc<ObjectData<'p>> {
        let object = self.object.as_ref().unwrap();
        object.top.clone()
    }
}
