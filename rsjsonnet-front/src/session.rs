use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rsjsonnet_lang::interner::InternedStr;
use rsjsonnet_lang::program::{ImportError, LoadError, NativeError, Program, Thunk, Value};
use rsjsonnet_lang::span::{SourceId, SpanId};

use crate::src_manager::SrcManager;

type NativeFunc = Box<dyn FnMut(&mut Program, &[Value]) -> Result<Value, String>>;

pub struct Session {
    program: Program,
    inner: SessionInner,
}

struct SessionInner {
    src_mgr: SrcManager,
    source_paths: HashMap<SourceId, PathBuf>,
    source_cache: HashMap<PathBuf, Thunk>,
    search_paths: Vec<PathBuf>,
    native_funcs: HashMap<InternedStr, NativeFunc>,
    custom_stack_trace: Vec<String>,
    max_trace: usize,
    #[cfg(feature = "crossterm")]
    colored_output: bool,
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

impl Session {
    pub fn new() -> Self {
        let program = Program::new();
        let (stdlib_src_id, stdlib_data) = program.get_stdlib_source();

        let mut src_mgr = SrcManager::new();
        src_mgr.insert_file(stdlib_src_id, "<stdlib>".into(), stdlib_data.into());

        Self {
            program,
            inner: SessionInner {
                src_mgr,
                source_paths: HashMap::new(),
                source_cache: HashMap::new(),
                search_paths: Vec::new(),
                native_funcs: HashMap::new(),
                custom_stack_trace: Vec::new(),
                max_trace: usize::MAX,
                #[cfg(feature = "crossterm")]
                colored_output: false,
            },
        }
    }

    /// Returns a reference to the underlying `Program`.
    #[must_use]
    #[inline]
    pub fn program(&self) -> &Program {
        &self.program
    }

    /// Returns a mutable reference to the underlying `Program`.
    #[must_use]
    #[inline]
    pub fn program_mut(&mut self) -> &mut Program {
        &mut self.program
    }

    /// Sets the maximum number of stack trace items to print.
    pub fn set_max_trace(&mut self, max_trace: usize) {
        self.inner.max_trace = max_trace;
    }

    #[cfg(feature = "crossterm")]
    pub fn set_colored_output(&mut self, colored_output: bool) {
        self.inner.colored_output = colored_output;
    }

    pub fn add_search_path(&mut self, path: PathBuf) {
        self.inner.search_paths.push(path);
    }

    /// Adds a native function.
    ///
    /// # Example
    ///
    /// ```
    /// let mut session = rsjsonnet_front::Session::new();
    ///
    /// session.add_native_func("MyFunc", &["arg"], |_, [arg]| {
    ///     let Some(arg) = arg.to_string() else {
    ///         return Err("expected a string".into());
    ///     };
    ///
    ///     // Count lowercase vowels.
    ///     let r = arg
    ///         .chars()
    ///         .filter(|chr| matches!(chr, 'a' | 'e' | 'i' | 'o' | 'u'))
    ///         .count();
    ///     Ok(rsjsonnet_lang::program::Value::number(r as f64))
    /// });
    ///
    /// let source = br#"local f = std.native("MyFunc"); f("hello world")"#;
    /// let thunk = session
    ///     .load_virt_file("<example>", source.to_vec())
    ///     .unwrap();
    ///
    /// let result = session.eval_value(&thunk).unwrap();
    ///
    /// assert_eq!(result.as_number(), Some(3.0));
    /// ```
    pub fn add_native_func<const N: usize, F>(
        &mut self,
        name: &str,
        params: &[&str; N],
        mut func: F,
    ) where
        F: FnMut(&mut Program, &[Value; N]) -> Result<Value, String> + 'static,
    {
        let str_interner = self.program.str_interner();
        let name = str_interner.intern(name);
        let params: Vec<_> = params.iter().map(|p| str_interner.intern(p)).collect();

        self.program.register_native_func(name.clone(), &params);
        self.inner.native_funcs.insert(
            name,
            Box::new(move |program, args| func(program, args.try_into().unwrap())),
        );
    }

    /// Loads a file with the provided `data`.
    ///
    /// `repr_path` is used to represent the file in error messages and for
    /// `std.thisFile`.
    ///
    /// In case of failure, the error is printed to stderr and `None` is
    /// returned.
    pub fn load_virt_file(&mut self, repr_path: &str, data: Vec<u8>) -> Option<Thunk> {
        self.inner
            .load_virt_file(&mut self.program, repr_path, data)
    }

    /// Loads a file from the filesystem.
    ///
    /// In case of failure, the error is printed to stderr and `None` is
    /// returned.
    pub fn load_real_file(&mut self, path: &Path) -> Option<Thunk> {
        self.inner.load_real_file(&mut self.program, path)
    }

    /// Evaluates a thunk.
    ///
    /// In case of failure, the error is printed to stderr and `None` is
    /// returned.
    pub fn eval_value(&mut self, thunk: &Thunk) -> Option<Value> {
        match self.program.eval_value(thunk, &mut self.inner) {
            Ok(v) => Some(v),
            Err(e) => {
                self.inner.print_eval_error(&self.program, &e);
                None
            }
        }
    }

    /// Evaluates a function call.
    ///
    /// In case of failure, the error is printed to stderr and `None` is
    /// returned.
    pub fn eval_call(
        &mut self,
        func: &Thunk,
        pos_args: &[Thunk],
        named_args: &[(InternedStr, Thunk)],
    ) -> Option<Value> {
        match self
            .program
            .eval_call(func, pos_args, named_args, &mut self.inner)
        {
            Ok(v) => Some(v),
            Err(e) => {
                self.inner.print_eval_error(&self.program, &e);
                None
            }
        }
    }

    /// Marshals a value as JSON.
    ///
    /// In case of failure, the error is printed to stderr and `None` is
    /// returned.
    pub fn manifest_json(&mut self, value: &Value, multiline: bool) -> Option<String> {
        match self.program.manifest_json(value, multiline) {
            Ok(s) => Some(s),
            Err(e) => {
                self.inner.print_eval_error(&self.program, &e);
                None
            }
        }
    }

    pub fn print_error(&self, msg: &str) {
        self.inner.print_error(msg);
    }

    pub fn print_note(&self, msg: &str) {
        self.inner.print_note(msg);
    }

    /// Pushes a custom item that will be included at the end of stack straces.
    pub fn push_custom_stack_trace_item(&mut self, text: String) {
        self.inner.custom_stack_trace.push(text);
    }

    /// Removes the last item pushed with
    /// [`Session::push_custom_stack_trace_item`].
    pub fn pop_custom_stack_trace_item(&mut self) {
        self.inner.custom_stack_trace.pop().unwrap();
    }
}

impl SessionInner {
    fn load_virt_file(
        &mut self,
        program: &mut Program,
        repr_path: &str,
        data: Vec<u8>,
    ) -> Option<Thunk> {
        let (span_ctx, source_id) = program.span_manager_mut().insert_source_context(data.len());

        self.src_mgr
            .insert_file(source_id, repr_path.into(), data.into_boxed_slice());
        let data = self.src_mgr.get_file_data(source_id);

        match program.load_source(span_ctx, data, true, repr_path) {
            Ok(thunk) => Some(thunk),
            Err(ref e) => {
                self.print_load_error(program, e);
                None
            }
        }
    }

    fn load_real_file(&mut self, program: &mut Program, path: &Path) -> Option<Thunk> {
        let norm_path = match path.canonicalize() {
            Ok(p) => p,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                self.print_error(format_args!("file {path:?} does not exist"));
                return None;
            }
            Err(e) => {
                self.print_error(format_args!("failed to canonicalize path {path:?}: {e}"));
                return None;
            }
        };
        if let Some(thunk) = self.source_cache.get(&norm_path) {
            return Some(thunk.clone());
        }

        let data = match std::fs::read(path) {
            Ok(data) => data,
            Err(e) => {
                self.print_error(format_args!("failed to read {path:?}: {e}"));
                return None;
            }
        };

        let repr_path = path.to_string_lossy();

        let (span_ctx, source_id) = program.span_manager_mut().insert_source_context(data.len());
        self.src_mgr.insert_file(
            source_id,
            repr_path.as_ref().into(),
            data.into_boxed_slice(),
        );
        self.source_paths.insert(source_id, path.to_path_buf());
        let data = self.src_mgr.get_file_data(source_id);

        match program.load_source(span_ctx, data, true, &repr_path) {
            Ok(thunk) => {
                self.source_cache.insert(norm_path, thunk.clone());
                Some(thunk)
            }
            Err(ref e) => {
                self.print_load_error(program, e);
                None
            }
        }
    }

    fn find_import(&self, program: &Program, from: SpanId, path: &str) -> Option<PathBuf> {
        let path = Path::new(path);
        if path.is_absolute() {
            if path.exists() {
                Some(path.to_path_buf())
            } else {
                None
            }
        } else {
            let (from_ctx, _, _) = program.span_manager().get_span(from);
            let from_dir_path = match *program.span_manager().get_context(from_ctx) {
                rsjsonnet_lang::span::SpanContext::Source(from_src) => {
                    self.source_paths.get(&from_src).and_then(|p| p.parent())
                }
            };

            for base_path in from_dir_path
                .into_iter()
                .chain(self.search_paths.iter().map(PathBuf::as_path))
            {
                let full_path = base_path.join(path);
                if full_path.exists() {
                    return Some(full_path);
                }
            }
            None
        }
    }

    fn print_rich_message(&self, msg: &[(String, crate::print::TextPartKind)]) {
        #[cfg(feature = "crossterm")]
        if self.colored_output {
            crate::print::output_stderr_colored(msg);
            return;
        }

        crate::print::output_stderr_plain(msg);
    }

    fn print_load_error(&self, program: &Program, error: &LoadError) {
        match error {
            LoadError::Lex(e) => {
                let msg =
                    crate::report::lexer::render_error(e, program.span_manager(), &self.src_mgr);
                self.print_rich_message(&msg);
            }
            LoadError::Parse(e) => {
                let msg =
                    crate::report::parser::render_error(e, program.span_manager(), &self.src_mgr);
                self.print_rich_message(&msg);
            }
            LoadError::Analyze(e) => {
                let msg =
                    crate::report::analyze::render_error(e, program.span_manager(), &self.src_mgr);
                self.print_rich_message(&msg);
            }
        }
    }

    fn print_eval_error(&self, program: &Program, error: &rsjsonnet_lang::program::EvalError) {
        self.print_rich_message(&crate::report::eval::render_error_kind(
            &error.kind,
            program.span_manager(),
            &self.src_mgr,
        ));
        self.print_stack_trace(program, &error.stack_trace);
        eprintln!();
    }

    fn print_error<T: std::fmt::Display>(&self, msg: T) {
        let msg = crate::report::render_error_message(msg);
        self.print_rich_message(&msg);
    }

    fn print_note<T: std::fmt::Display>(&self, msg: T) {
        let msg = crate::report::render_note_message(msg);
        self.print_rich_message(&msg);
    }

    fn print_stack_trace(
        &self,
        program: &Program,
        stack: &[rsjsonnet_lang::program::EvalStackTraceItem],
    ) {
        if stack.len() <= self.max_trace {
            self.print_rich_message(&crate::report::stack_trace::render(
                stack,
                program.span_manager(),
                &self.src_mgr,
            ));
        } else {
            let second_len = self.max_trace / 2;
            let first_len = self.max_trace - second_len;

            self.print_rich_message(&crate::report::stack_trace::render(
                &stack[(stack.len() - first_len)..],
                program.span_manager(),
                &self.src_mgr,
            ));
            self.print_note(format_args!(
                "... {} items hidden ...",
                stack.len() - self.max_trace
            ));
            self.print_rich_message(&crate::report::stack_trace::render(
                &stack[..second_len],
                program.span_manager(),
                &self.src_mgr,
            ));
        }

        for custom_item in self.custom_stack_trace.iter().rev() {
            self.print_note(custom_item);
        }
    }
}

impl rsjsonnet_lang::program::Callbacks for SessionInner {
    fn import(
        &mut self,
        program: &mut Program,
        from: SpanId,
        path: &str,
    ) -> Result<Thunk, ImportError> {
        let Some(full_path) = self.find_import(program, from, path) else {
            self.print_error(format_args!("import {path:?} not found in search path"));
            return Err(ImportError);
        };
        match self.load_real_file(program, &full_path) {
            None => Err(ImportError),
            Some(thunk) => Ok(thunk),
        }
    }

    fn import_str(
        &mut self,
        program: &mut Program,
        from: SpanId,
        path: &str,
    ) -> Result<String, ImportError> {
        let Some(full_path) = self.find_import(program, from, path) else {
            self.print_error(format_args!("import {path:?} not found in search path"));
            return Err(ImportError);
        };
        let data = match std::fs::read(&full_path) {
            Ok(data) => data,
            Err(e) => {
                self.print_error(format_args!("failed to read {full_path:?}: {e}"));
                return Err(ImportError);
            }
        };
        Ok(String::from_utf8_lossy(&data).into_owned())
    }

    fn import_bin(
        &mut self,
        program: &mut Program,
        from: SpanId,
        path: &str,
    ) -> Result<Vec<u8>, ImportError> {
        let Some(full_path) = self.find_import(program, from, path) else {
            self.print_error(format_args!("import {path:?} not found in search path"));
            return Err(ImportError);
        };
        let data = match std::fs::read(&full_path) {
            Ok(data) => data,
            Err(e) => {
                self.print_error(format_args!("failed to read {full_path:?}: {e}"));
                return Err(ImportError);
            }
        };
        Ok(data)
    }

    fn trace(
        &mut self,
        program: &mut Program,
        message: &str,
        stack: &[rsjsonnet_lang::program::EvalStackTraceItem],
    ) {
        self.print_rich_message(&[
            ("TRACE".into(), crate::print::TextPartKind::NoteLabel),
            (": ".into(), crate::print::TextPartKind::MainMessage),
            (message.into(), crate::print::TextPartKind::MainMessage),
            ('\n'.into(), crate::print::TextPartKind::Space),
        ]);
        self.print_stack_trace(program, stack);
        eprintln!();
    }

    fn native_call(
        &mut self,
        program: &mut Program,
        name: &InternedStr,
        args: &[Value],
    ) -> Result<Value, NativeError> {
        let native_func = &mut self.native_funcs.get_mut(name).unwrap();
        match native_func(program, args) {
            Ok(v) => Ok(v),
            Err(e) => {
                self.print_error(format_args!("native function {name:?} failed: {e}"));
                Err(NativeError)
            }
        }
    }
}
