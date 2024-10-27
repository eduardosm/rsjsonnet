//! Jsonnet program state and evaluation.
//!
//! # Example
//!
//! ```
//! let source = b"local add_one(x) = x + 1; add_one(2)";
//!
//! // Implement callbacks
//! struct Callbacks;
//!
//! impl rsjsonnet_lang::program::Callbacks for Callbacks {
//!     fn import(
//!         &mut self,
//!         program: &mut rsjsonnet_lang::program::Program,
//!         from: rsjsonnet_lang::span::SpanId,
//!         path: &str,
//!     ) -> Result<rsjsonnet_lang::program::Thunk, rsjsonnet_lang::program::ImportError> {
//!         unimplemented!();
//!     }
//!
//!     fn import_str(
//!         &mut self,
//!         program: &mut rsjsonnet_lang::program::Program,
//!         from: rsjsonnet_lang::span::SpanId,
//!         path: &str,
//!     ) -> Result<String, rsjsonnet_lang::program::ImportError> {
//!         unimplemented!();
//!     }
//!
//!     fn import_bin(
//!         &mut self,
//!         program: &mut rsjsonnet_lang::program::Program,
//!         from: rsjsonnet_lang::span::SpanId,
//!         path: &str,
//!     ) -> Result<Vec<u8>, rsjsonnet_lang::program::ImportError> {
//!         unimplemented!();
//!     }
//!
//!     fn trace(
//!         &mut self,
//!         program: &mut rsjsonnet_lang::program::Program,
//!         message: &str,
//!         stack: &[rsjsonnet_lang::program::EvalStackTraceItem],
//!     ) {
//!         unimplemented!();
//!     }
//!
//!     fn native_call(
//!         &mut self,
//!         program: &mut rsjsonnet_lang::program::Program,
//!         name: &rsjsonnet_lang::interner::InternedStr,
//!         args: &[rsjsonnet_lang::program::Value],
//!     ) -> Result<rsjsonnet_lang::program::Value, rsjsonnet_lang::program::NativeError> {
//!         unimplemented!();
//!     }
//! }
//!
//! // Create the program state.
//! let mut program = rsjsonnet_lang::program::Program::new();
//! let mut callbacks = Callbacks;
//!
//! // Import the source.
//! let (span_ctx, _) = program
//!     .span_manager_mut()
//!     .insert_source_context(source.len());
//! let thunk = program
//!     .load_source(span_ctx, source, true, "<example>")
//!     .unwrap();
//!
//! // Evaluate it.
//! let value = program.eval_value(&thunk, &mut callbacks).unwrap();
//! assert_eq!(value.as_number(), Some(3.0));
//! ```

use std::cell::OnceCell;
use std::rc::Rc;

use crate::gc::{Gc, GcContext, GcTrace, GcView};
use crate::interner::{InternedStr, StrInterner};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::span::{SourceId, SpanContextId, SpanId, SpanManager};
use crate::{ast, FHashMap};

mod analyze;
mod data;
mod error;
mod eval;
mod ir;
mod stdlib;

use data::{
    ArrayData, BuiltInFunc, FuncData, FuncKind, FuncParams, ObjectAssert, ObjectCore, ObjectData,
    ObjectField, PendingThunk, ThunkData, ThunkEnv, ThunkEnvData, ThunkState, ValueData,
};
pub use error::{AnalyzeError, EvalError, EvalErrorKind, EvalErrorValueType, LoadError};

/// Error type that can be returned by [`Callbacks::import`],
/// [`Callbacks::import_str`] and [`Callbacks::import_bin`].
///
/// It does not carry any additional information.
#[derive(Clone, Debug)]
pub struct ImportError;

/// Error type that can be returned by [`Callbacks::native_call`].
///
/// It does not carry any additional information.
#[derive(Clone, Debug)]
pub struct NativeError;

/// Trait to customize the behavior of operations during evaluation.
///
/// Some [`Program`] methods need to be provided with an implementor of this
/// trait.
pub trait Callbacks {
    /// Called when an `import` expression is evaluated.
    fn import(
        &mut self,
        program: &mut Program,
        from: SpanId,
        path: &str,
    ) -> Result<Thunk, ImportError>;

    /// Called when an `importstr` expression is evaluated.
    fn import_str(
        &mut self,
        program: &mut Program,
        from: SpanId,
        path: &str,
    ) -> Result<String, ImportError>;

    /// Called when an `importbin` expression is evaluated.
    fn import_bin(
        &mut self,
        program: &mut Program,
        from: SpanId,
        path: &str,
    ) -> Result<Vec<u8>, ImportError>;

    /// Called when a call to `std.trace` is evaluated.
    fn trace(&mut self, program: &mut Program, message: &str, stack: &[EvalStackTraceItem]);

    /// Called when a function returned by `std.native` is called.
    ///
    /// Native functions must be registered with
    /// [`Program::register_native_func`].
    fn native_call(
        &mut self,
        program: &mut Program,
        name: &InternedStr,
        args: &[Value],
    ) -> Result<Value, NativeError>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvalStackTraceItem {
    Expr {
        span: SpanId,
    },
    Call {
        span: Option<SpanId>,
        name: Option<String>,
    },
    Variable {
        span: SpanId,
        name: String,
    },
    ArrayItem {
        span: Option<SpanId>,
        index: usize,
    },
    ObjectField {
        span: Option<SpanId>,
        name: String,
    },
    CompareArrayItem {
        index: usize,
    },
    CompareObjectField {
        name: String,
    },
    ManifestArrayItem {
        index: usize,
    },
    ManifestObjectField {
        name: String,
    },
    Import {
        span: SpanId,
    },
}

/// Jsonnet program state and evaluator.
///
/// See the [module-level documentation](self) for more information.
pub struct Program {
    str_interner: StrInterner,
    span_mgr: SpanManager,
    gc_ctx: GcContext<'static>,
    objs_after_last_gc: usize,
    max_stack: usize,
    null_expr: ir::RcExpr,
    false_expr: ir::RcExpr,
    true_expr: ir::RcExpr,
    self_obj_expr: ir::RcExpr,
    top_obj_expr: ir::RcExpr,
    stdlib_src_id: SourceId,
    stdlib_data: &'static [u8],
    stdlib_obj: Option<GcView<ObjectData>>,
    empty_array: GcView<ArrayData>,
    identity_func: GcView<FuncData>,
    ext_vars: FHashMap<InternedStr, GcView<ThunkData>>,
    native_funcs: FHashMap<InternedStr, GcView<FuncData>>,
}

impl Default for Program {
    fn default() -> Self {
        Self::new()
    }
}

impl Program {
    /// Creates a new [`Program`].
    pub fn new() -> Self {
        let str_interner = StrInterner::new();
        let gc_ctx = GcContext::new();

        let stdlib_data = stdlib::STDLIB_DATA;
        let mut span_mgr = SpanManager::new();
        let (stdlib_span_ctx, stdlib_src_id) = span_mgr.insert_source_context(stdlib_data.len());

        let empty_array: GcView<ArrayData> = gc_ctx.alloc_view(Box::new([]));
        let identity_func = gc_ctx.alloc_view(FuncData::new(
            Rc::new(vec![(str_interner.intern("x"), None)]),
            FuncKind::BuiltIn {
                name: str_interner.intern("id"),
                kind: BuiltInFunc::Identity,
            },
        ));

        let mut this = Self {
            str_interner,
            span_mgr,
            gc_ctx,
            objs_after_last_gc: 0,
            max_stack: 500,
            null_expr: ir::RcExpr::new(ir::Expr::Null),
            false_expr: ir::RcExpr::new(ir::Expr::Bool(false)),
            true_expr: ir::RcExpr::new(ir::Expr::Bool(true)),
            self_obj_expr: ir::RcExpr::new(ir::Expr::SelfObj),
            top_obj_expr: ir::RcExpr::new(ir::Expr::TopObj),
            stdlib_src_id,
            stdlib_data,
            stdlib_obj: None,
            empty_array,
            identity_func,
            ext_vars: FHashMap::default(),
            native_funcs: FHashMap::default(),
        };
        this.load_stdlib(stdlib_span_ctx);
        this
    }

    #[inline]
    pub fn str_interner(&self) -> &StrInterner {
        &self.str_interner
    }

    #[inline]
    pub fn intern_str(&self, value: &str) -> InternedStr {
        self.str_interner.intern(value)
    }

    #[inline]
    pub fn span_manager(&self) -> &SpanManager {
        &self.span_mgr
    }

    #[inline]
    pub fn span_manager_mut(&mut self) -> &mut SpanManager {
        &mut self.span_mgr
    }

    /// Runs garbage collection unconditionally.
    pub fn gc(&mut self) {
        self.gc_ctx.gc();
        self.str_interner.gc();
        self.objs_after_last_gc = self.gc_ctx.num_objects();
    }

    /// Runs garbage collection under certain conditions.
    pub fn maybe_gc(&mut self) {
        let num_objects = self.gc_ctx.num_objects();
        if num_objects > 1000 && (num_objects / 2) > self.objs_after_last_gc {
            self.gc();
        }
    }

    /// Sets the maximum call stack size.
    ///
    /// The default is 500.
    pub fn set_max_stack(&mut self, max_stack: usize) {
        self.max_stack = max_stack;
    }

    /// Returns the source of the part of the standard library that
    /// is implemented in Jsonnet.
    pub fn get_stdlib_source(&self) -> (SourceId, &[u8]) {
        (self.stdlib_src_id, self.stdlib_data)
    }

    /// Adds an external variable.
    ///
    /// External variables can be accessed within a Jsonnet program
    /// with the `std.extVar` function.
    pub fn add_ext_var(&mut self, name: InternedStr, thunk: &Thunk) {
        match self.ext_vars.entry(name) {
            std::collections::hash_map::Entry::Occupied(entry) => {
                panic!("external variable {:?} already set", entry.key());
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(thunk.data.clone());
            }
        }
    }

    /// Registers a native function.
    ///
    /// Native functions can be accessed within a Jsonnet program
    /// with the `std.native` function.
    ///
    /// The actual behavior of the native function should be implemented
    /// in [`Callbacks::native_call`].
    pub fn register_native_func(&mut self, name: InternedStr, params: &[InternedStr]) {
        match self.native_funcs.entry(name.clone()) {
            std::collections::hash_map::Entry::Occupied(entry) => {
                panic!("native function {:?} already registered", entry.key());
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                let params_order = params.iter().map(|name| (name.clone(), None)).collect();
                entry.insert(self.gc_ctx.alloc_view(FuncData::new(
                    Rc::new(params_order),
                    FuncKind::Native { name },
                )));
            }
        }
    }

    #[must_use]
    #[inline]
    fn gc_alloc<T: GcTrace + 'static>(&self, data: T) -> Gc<T> {
        self.gc_ctx.alloc(data)
    }

    #[must_use]
    #[inline]
    fn gc_alloc_view<T: GcTrace + 'static>(&self, data: T) -> GcView<T> {
        self.gc_ctx.alloc_view(data)
    }

    fn insert_thunk_with_value(&mut self, value: ValueData) -> GcView<ThunkData> {
        self.gc_alloc_view(ThunkData::new_done(value))
    }

    /// Creates a thunk with an already evaluated value.
    pub fn value_to_thunk(&mut self, value: &Value) -> Thunk {
        Thunk::new(self.insert_thunk_with_value(value.inner.clone()))
    }

    /// Creates an array value.
    pub fn make_array(&mut self, items: &[Value]) -> Value {
        let array = self.make_value_array(items.iter().map(|item| item.inner.clone()));
        Value::from_value(ValueData::Array(array))
    }

    /// Creates an object value.
    pub fn make_object(&mut self, obj_fields: &[(InternedStr, Value)]) -> Value {
        let mut fields = FHashMap::default();
        for (name, value) in obj_fields.iter() {
            let value_thunk = self.gc_alloc(ThunkData::new_done(value.inner.clone()));
            let prev = fields.insert(
                name.clone(),
                ObjectField {
                    base_env: None,
                    visibility: ast::Visibility::Default,
                    expr: None,
                    thunk: OnceCell::from(value_thunk),
                },
            );
            assert!(prev.is_none(), "repeated field name {name:?}");
        }

        let obj = self.gc_alloc(ObjectData::new_simple(fields));
        Value::from_value(ValueData::Object(obj))
    }

    /// Loads a Jsonnet source into a thunk.
    ///
    /// `with_stdlib` specifies whether the standard library will be available
    /// as `std`. `this_file` will be the value of `std.thisFile`.
    pub fn load_source(
        &mut self,
        span_ctx: SpanContextId,
        input: &[u8],
        with_stdlib: bool,
        this_file: &str,
    ) -> Result<Thunk, LoadError> {
        let lexer = Lexer::new(&self.str_interner, &mut self.span_mgr, span_ctx, input);
        let tokens = lexer.lex_to_eof(false)?;

        let parser = Parser::new(&self.str_interner, &mut self.span_mgr, tokens);
        let root_expr = parser.parse_root_expr()?;

        let env = if with_stdlib {
            let stdlib_obj = self.make_custom_stdlib(this_file);
            let stdlib_thunk =
                self.gc_alloc_view(ThunkData::new_done(ValueData::Object(stdlib_obj)));

            let mut env = FHashMap::default();
            env.insert(self.intern_str("std"), Thunk::new(stdlib_thunk));

            Some(env)
        } else {
            None
        };
        let thunk = self.analyze(&root_expr, env)?;
        Ok(thunk)
    }

    fn analyze(
        &mut self,
        ast: &ast::Expr,
        env: Option<FHashMap<InternedStr, Thunk>>,
    ) -> Result<Thunk, AnalyzeError> {
        let analyze_env = env
            .as_ref()
            .map(|env| env.keys().cloned().collect())
            .unwrap_or_default();
        let ir_expr = analyze::Analyzer::new(self).analyze(ast, analyze_env)?;

        let mut thunk_env_data = ThunkEnvData::new(None);
        if let Some(env) = env {
            for (name, value) in env.iter() {
                thunk_env_data.set_var(name.clone(), Gc::from(&value.data));
            }
        }
        let thunk_env = self.gc_alloc(ThunkEnv::from(thunk_env_data));

        let thunk = self.gc_alloc_view(ThunkData::new_pending_expr(ir_expr, thunk_env));

        Ok(Thunk::new(thunk))
    }

    /// Evaluates a thunk into a value.
    pub fn eval_value(
        &mut self,
        thunk: &Thunk,
        callbacks: &mut dyn Callbacks,
    ) -> Result<Value, EvalError> {
        let output = eval::Evaluator::eval(
            self,
            Some(callbacks),
            eval::EvalInput::Value(thunk.data.clone()),
        )
        .map_err(|e| *e)?;
        assert!(matches!(output, eval::EvalOutput::Nothing));
        Ok(Value::from_thunk(&thunk.data))
    }

    fn eval_value_internal(&mut self, thunk: &Thunk) -> Result<(), EvalError> {
        let output = eval::Evaluator::eval(self, None, eval::EvalInput::Value(thunk.data.clone()))
            .map_err(|e| *e)?;
        assert!(matches!(output, eval::EvalOutput::Nothing));
        Ok(())
    }

    /// Evaluates a function call.
    pub fn eval_call(
        &mut self,
        func: &Thunk,
        pos_args: &[Thunk],
        named_args: &[(InternedStr, Thunk)],
        callbacks: &mut dyn Callbacks,
    ) -> Result<Value, EvalError> {
        let output = eval::Evaluator::eval(
            self,
            Some(callbacks),
            eval::EvalInput::Call(
                func.data.clone(),
                eval::TopLevelArgs {
                    positional: pos_args.iter().map(|thunk| thunk.data.clone()).collect(),
                    named: named_args
                        .iter()
                        .map(|(name, thunk)| (name.clone(), thunk.data.clone()))
                        .collect(),
                },
            ),
        )
        .map_err(|e| *e)?;
        let eval::EvalOutput::Value(value) = output else {
            unreachable!();
        };
        Ok(Value::from_value(value))
    }

    /// Manifests a value as JSON.
    pub fn manifest_json(&mut self, value: &Value, multiline: bool) -> Result<String, EvalError> {
        let thunk = self.insert_thunk_with_value(value.inner.clone());
        let output =
            eval::Evaluator::eval(self, None, eval::EvalInput::ManifestJson(thunk, multiline))
                .map_err(|e| *e)?;
        let eval::EvalOutput::String(s) = output else {
            unreachable!();
        };
        Ok(s)
    }
}

/// A value that might not be evaluated yet.
///
/// Each instance of [`Thunk`] are tied to a [`Program`] instance. Instances of
/// this type will only be valid as long as the [`Program`] they came from has
/// not been dropped.
#[derive(Clone)]
pub struct Thunk {
    data: GcView<ThunkData>,
}

impl Thunk {
    #[inline]
    fn new(data: GcView<ThunkData>) -> Self {
        Self { data }
    }
}

/// A fully evaluated value.
///
/// Each instance of [`Value`] are tied to a [`Program`] instance. Instances of
/// this type will only be valid as long as the [`Program`] they came from has
/// not been dropped.
#[derive(Clone)]
pub struct Value {
    inner: ValueData,
}

impl Value {
    #[inline]
    fn from_value(inner: ValueData) -> Self {
        Self { inner }
    }

    #[inline]
    fn from_thunk(thunk: &ThunkData) -> Self {
        Self {
            inner: match *thunk.state() {
                ThunkState::Done(ref value) => value.clone(),
                _ => panic!("thunk not evaluated"),
            },
        }
    }

    /// Creates a null value.
    ///
    /// The returned [`Value`] will not be tied to any specific [`Program`]
    /// instance.
    #[inline]
    pub fn null() -> Self {
        Self::from_value(ValueData::Null)
    }

    /// Creates a boolean value.
    ///
    /// The returned [`Value`] will not be tied to any specific [`Program`]
    /// instance.
    #[inline]
    pub fn bool(value: bool) -> Self {
        Self::from_value(ValueData::Bool(value))
    }

    /// Creates a numberic value.
    ///
    /// The returned [`Value`] will not be tied to any specific [`Program`]
    /// instance.
    #[inline]
    pub fn number(value: f64) -> Self {
        Self::from_value(ValueData::Number(value))
    }

    /// Creates a string value.
    ///
    /// The returned [`Value`] will not be tied to any specific [`Program`]
    /// instance.
    #[inline]
    pub fn string(s: &str) -> Self {
        Self::from_value(ValueData::String(s.into()))
    }

    #[must_use]
    pub fn kind(&self) -> ValueKind {
        match self.inner {
            ValueData::Null => ValueKind::Null,
            ValueData::Bool(value) => ValueKind::Bool(value),
            ValueData::Number(value) => ValueKind::Number(value),
            ValueData::String(ref s) => ValueKind::String((**s).into()),
            ValueData::Array(ref array) => ValueKind::Array(Self::extract_array(&array.view())),
            ValueData::Object(ref object) => {
                ValueKind::Object(Self::extract_object(&object.view()))
            }
            ValueData::Function(_) => ValueKind::Function,
        }
    }

    #[must_use]
    #[inline]
    pub fn is_null(&self) -> bool {
        matches!(self.inner, ValueData::Null)
    }

    #[must_use]
    #[inline]
    pub fn is_bool(&self) -> bool {
        matches!(self.inner, ValueData::Bool(_))
    }

    #[must_use]
    #[inline]
    pub fn as_bool(&self) -> Option<bool> {
        if let ValueData::Bool(value) = self.inner {
            Some(value)
        } else {
            None
        }
    }

    #[must_use]
    #[inline]
    pub fn is_number(&self) -> bool {
        matches!(self.inner, ValueData::Number(_))
    }

    #[must_use]
    #[inline]
    pub fn as_number(&self) -> Option<f64> {
        if let ValueData::Number(value) = self.inner {
            Some(value)
        } else {
            None
        }
    }

    #[must_use]
    #[inline]
    pub fn is_string(&self) -> bool {
        matches!(self.inner, ValueData::String(_))
    }

    #[must_use]
    #[inline]
    pub fn to_string(&self) -> Option<String> {
        if let ValueData::String(ref s) = self.inner {
            Some((**s).into())
        } else {
            None
        }
    }

    #[must_use]
    #[inline]
    pub fn is_array(&self) -> bool {
        matches!(self.inner, ValueData::Array(_))
    }

    #[must_use]
    pub fn to_array(&self) -> Option<Vec<Self>> {
        if let ValueData::Array(ref array) = self.inner {
            Some(Self::extract_array(&array.view()))
        } else {
            None
        }
    }

    #[must_use]
    #[inline]
    pub fn is_object(&self) -> bool {
        matches!(self.inner, ValueData::Object(_))
    }

    #[must_use]
    pub fn to_object(&self) -> Option<Vec<(InternedStr, Self)>> {
        if let ValueData::Object(ref object) = self.inner {
            Some(Self::extract_object(&object.view()))
        } else {
            None
        }
    }

    #[must_use]
    #[inline]
    pub fn is_function(&self) -> bool {
        matches!(self.inner, ValueData::Function(_))
    }

    fn extract_array(array: &ArrayData) -> Vec<Self> {
        array
            .iter()
            .map(|item| Self::from_thunk(&item.view()))
            .collect()
    }

    fn extract_object(object: &ObjectData) -> Vec<(InternedStr, Self)> {
        let mut fields = Vec::new();
        for (name, visible) in object.get_fields_order().iter() {
            if *visible {
                let (_, field) = object.find_field(0, name).unwrap();
                let thunk = field.thunk.get().unwrap().clone();
                fields.push((name.clone(), Self::from_thunk(&thunk.view())));
            }
        }
        fields
    }
}

#[derive(Clone)]
pub enum ValueKind {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Value>),
    Object(Vec<(InternedStr, Value)>),
    Function,
}
