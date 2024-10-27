#![warn(
    rust_2018_idioms,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unused_qualifications
)]
#![forbid(unsafe_code)]

use rsjsonnet_lang::interner::InternedStr;
use rsjsonnet_lang::program::{
    EvalErrorKind, EvalStackTraceItem, ImportError, NativeError, Program, Thunk, Value,
};
use rsjsonnet_lang::span::SpanId;

pub(crate) struct TestCallbacks;

impl TestCallbacks {
    pub(crate) fn new() -> Self {
        Self
    }

    pub(crate) fn init_native_funcs(&mut self, program: &mut Program) {
        program.register_native_func(program.intern_str("returnNum"), &[]);
        program.register_native_func(
            program.intern_str("isString"),
            &[program.intern_str("value")],
        );
        program.register_native_func(
            program.intern_str("lastItemOfFirst"),
            &[program.intern_str("array")],
        );
        program.register_native_func(program.intern_str("failure"), &[]);
    }
}

impl rsjsonnet_lang::program::Callbacks for TestCallbacks {
    fn import(
        &mut self,
        _program: &mut Program,
        _from: SpanId,
        _path: &str,
    ) -> Result<Thunk, ImportError> {
        unimplemented!();
    }

    fn import_str(
        &mut self,
        _program: &mut Program,
        _from: SpanId,
        _path: &str,
    ) -> Result<String, ImportError> {
        unimplemented!();
    }

    fn import_bin(
        &mut self,
        _program: &mut Program,
        _from: SpanId,
        _path: &str,
    ) -> Result<Vec<u8>, ImportError> {
        unimplemented!();
    }

    fn trace(&mut self, _program: &mut Program, _message: &str, _stack: &[EvalStackTraceItem]) {}

    fn native_call(
        &mut self,
        _program: &mut Program,
        name: &InternedStr,
        args: &[Value],
    ) -> Result<Value, NativeError> {
        match name.value() {
            "returnNum" => {
                assert!(args.is_empty());
                Ok(Value::number(1234.0))
            }
            "isString" => {
                let [arg] = args else {
                    unreachable!();
                };
                Ok(Value::bool(arg.is_string()))
            }
            "lastItemOfFirst" => {
                let [arg] = args else {
                    unreachable!();
                };
                if let Some(root_items) = arg.to_array() {
                    if let Some(first_items) = root_items.first().and_then(Value::to_array) {
                        first_items.last().ok_or(NativeError).cloned()
                    } else {
                        Err(NativeError)
                    }
                } else {
                    Err(NativeError)
                }
            }
            "failure" => Err(NativeError),
            _ => unreachable!(),
        }
    }
}

#[test]
fn test_value_types() {
    #[track_caller]
    pub(crate) fn test(input: &[u8], check: impl FnOnce(Value)) {
        let mut program = Program::new();
        let mut callbacks = TestCallbacks::new();

        let (span_ctx, _) = program
            .span_manager_mut()
            .insert_source_context(input.len());

        let thunk = program
            .load_source(span_ctx, input, true, "test.jsonnet")
            .unwrap();

        let value = program.eval_value(&thunk, &mut callbacks).unwrap();
        check(value);
    }

    test(b"null", |value| {
        assert!(value.is_null());
    });
    test(b"true", |value| {
        assert_eq!(value.as_bool(), Some(true));
    });
    test(b"false", |value| {
        assert_eq!(value.as_bool(), Some(false));
    });
    test(b"1.50", |value| {
        assert_eq!(value.as_number(), Some(1.5));
    });
    test(b"\"string\"", |value| {
        assert_eq!(value.to_string(), Some("string".into()));
    });
    test(b"[]", |value| {
        let items = value.to_array().unwrap();
        assert!(items.is_empty());
    });
    test(b"[true, false]", |value| {
        let items = value.to_array().unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].as_bool(), Some(true));
        assert_eq!(items[1].as_bool(), Some(false));
    });
    test(b"{}", |value| {
        let fields = value.to_object().unwrap();
        assert!(fields.is_empty());
    });
    test(b"{y: true, x: false}", |value| {
        let fields = value.to_object().unwrap();
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].0.value(), "x");
        assert_eq!(fields[0].1.as_bool(), Some(false));
        assert_eq!(fields[1].0.value(), "y");
        assert_eq!(fields[1].1.as_bool(), Some(true));
    });
}

#[test]
fn test_manifest_single_line() {
    #[track_caller]
    fn test(input: &[u8], expected_result: &str) {
        let mut program = Program::new();
        let mut callbacks = TestCallbacks::new();

        let (span_ctx, _) = program
            .span_manager_mut()
            .insert_source_context(input.len());

        let thunk = program
            .load_source(span_ctx, input, true, "test.jsonnet")
            .unwrap();

        let value = program.eval_value(&thunk, &mut callbacks).unwrap();
        let result = program.manifest_json(&value, false);
        let result = result.unwrap();
        assert_eq!(result, expected_result);
    }

    test(b"null", "null");
    test(b"true", "true");
    test(b"false", "false");
    test(b"0", "0");
    test(b"-0", "-0");
    test(b"1.5", "1.5");
    test(b"\"string\"", "\"string\"");
    test(b"[]", "[ ]");
    test(b"[[]]", "[[ ]]");
    test(b"[[1, 2]]", "[[1, 2]]");
    test(b"[1, 2, 3]", "[1, 2, 3]");
    test(b"{}", "{ }");
    test(b"{a: 1, b: 2}", "{\"a\": 1, \"b\": 2}");
    test(b"{a: 1, b:: 2, c::: 3}", "{\"a\": 1, \"c\": 3}");
    test(b"{a: {}}", "{\"a\": { }}");
    test(b"{a: {b: 1}}", "{\"a\": {\"b\": 1}}");
    test(b"{a:: error \"err\"}", "{ }");
}

#[test]
fn test_manifest_multi_line() {
    #[track_caller]
    fn test(input: &[u8], expected_result: &str) {
        let mut program = Program::new();
        let mut callbacks = TestCallbacks::new();

        let (span_ctx, _) = program
            .span_manager_mut()
            .insert_source_context(input.len());

        let thunk = program
            .load_source(span_ctx, input, true, "test.jsonnet")
            .unwrap();

        let value = program.eval_value(&thunk, &mut callbacks).unwrap();
        let result = program.manifest_json(&value, true);
        let result = result.unwrap();
        assert_eq!(result, expected_result);
    }

    test(b"null", "null");
    test(b"true", "true");
    test(b"false", "false");
    test(b"0", "0");
    test(b"-0", "-0");
    test(b"1.5", "1.5");
    test(b"\"string\"", "\"string\"");
    test(b"[]", "[ ]");
    test(b"[1, 2, 3]", "[\n   1,\n   2,\n   3\n]");
    test(b"[[]]", "[\n   [ ]\n]");
    test(b"[[1, 2]]", "[\n   [\n      1,\n      2\n   ]\n]");
    test(b"{}", "{ }");
    test(b"{a: 1, b: 2}", "{\n   \"a\": 1,\n   \"b\": 2\n}");
    test(b"{a: 1, b:: 2, c::: 3}", "{\n   \"a\": 1,\n   \"c\": 3\n}");
    test(b"{a: {}}", "{\n   \"a\": { }\n}");
    test(b"{a: {b: 1}}", "{\n   \"a\": {\n      \"b\": 1\n   }\n}");
    test(b"{a:: error \"err\"}", r"{ }");
}

#[test]
fn test_native() {
    #[track_caller]
    fn test(input: &[u8], expected: &str) {
        let mut program = Program::new();
        let mut callbacks = TestCallbacks::new();
        callbacks.init_native_funcs(&mut program);

        let (span_ctx, _) = program
            .span_manager_mut()
            .insert_source_context(input.len());

        let root_thunk = program
            .load_source(span_ctx, input, true, "test.jsonnet")
            .unwrap();

        let value = program.eval_value(&root_thunk, &mut callbacks).unwrap();
        let value_str = program.manifest_json(&value, false).unwrap();
        assert_eq!(value_str, expected);
    }

    test(b"std.native(\"returnNum\")()", "1234");
    test(b"std.native(\"isString\")(null)", "false");
    test(b"std.native(\"isString\")(\"str\")", "true");
    test(
        b"std.native(\"lastItemOfFirst\")([[1, 2], [3, 4], [5, 6]])",
        "2",
    );

    test(b"std.native(\"unknown\")", "null");
    test(b"std.isFunction(std.native(\"returnNum\"))", "true");
    test(b"std.length(std.native(\"returnNum\"))", "0");
    test(b"std.length(std.native(\"isString\"))", "1");

    #[track_caller]
    fn test_fail(input: &[u8]) {
        let mut program = Program::new();
        let mut callbacks = TestCallbacks::new();
        callbacks.init_native_funcs(&mut program);

        let (span_ctx, _) = program
            .span_manager_mut()
            .insert_source_context(input.len());

        let thunk = program
            .load_source(span_ctx, input, true, "test.jsonnet")
            .unwrap();

        let error = program.eval_value(&thunk, &mut callbacks).err().unwrap();
        assert_eq!(error.kind, EvalErrorKind::NativeCallFailed);
    }

    test_fail(b"std.native(\"failure\")()");
}
