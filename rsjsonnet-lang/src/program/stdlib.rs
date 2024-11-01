use std::cell::OnceCell;
use std::rc::Rc;

use super::{
    ir, BuiltInFunc, FuncData, FuncKind, ObjectData, ObjectField, Program, ThunkData, ThunkState,
    ValueData,
};
use crate::gc::Gc;
use crate::interner::InternedStr;
use crate::span::SpanContextId;
use crate::{ast, FHashMap};

// From jsonnet 0.20.0
pub(super) const STDLIB_DATA: &[u8] = include_bytes!("std.jsonnet");

impl Program {
    pub(super) fn load_stdlib(&mut self, span_ctx: SpanContextId) {
        let stdlib_data = self.stdlib_data;
        let stdlib_thunk = self
            .load_source(span_ctx, stdlib_data, false, "std.jsonnet")
            .expect("failed to load stdlib");
        self.eval_value_internal(&stdlib_thunk)
            .expect("failed to evaluate stdlib");

        let stdlib_obj = match *stdlib_thunk.data.state() {
            ThunkState::Done(ValueData::Object(ref obj)) => obj.view(),
            _ => panic!("stdlib is not an object"),
        };
        let stdlib_extra_obj = self.build_stdlib_extra();

        let ext_stdlib_obj = self.extend_object(&stdlib_obj, &stdlib_extra_obj);
        assert!(self.stdlib_obj.is_none());
        self.stdlib_obj = Some(ext_stdlib_obj.view());

        self.maybe_gc();
    }

    fn build_stdlib_extra(&mut self) -> ObjectData {
        let mut extra_fields = FHashMap::default();

        let mut add_builtin_func =
            |name: InternedStr,
             kind: BuiltInFunc,
             params: Vec<(InternedStr, Option<ir::RcExpr>)>| {
                let prev = extra_fields.insert(
                    name.clone(),
                    ObjectField {
                        base_env: None,
                        visibility: ast::Visibility::Hidden,
                        expr: None,
                        thunk: OnceCell::from(self.gc_alloc(ThunkData::new_done(
                            ValueData::Function(self.gc_alloc(FuncData::new(
                                Rc::new(params),
                                FuncKind::BuiltIn { name, kind },
                            ))),
                        ))),
                    },
                );
                assert!(prev.is_none());
            };

        let mut add_simple = |name: &str, kind: BuiltInFunc, params: &[&str]| {
            add_builtin_func(
                self.intern_str(name),
                kind,
                params.iter().map(|&s| (self.intern_str(s), None)).collect(),
            );
        };

        add_simple("extVar", BuiltInFunc::ExtVar, &["x"]);
        add_simple("type", BuiltInFunc::Type, &["x"]);
        add_simple("isArray", BuiltInFunc::IsArray, &["v"]);
        add_simple("isBoolean", BuiltInFunc::IsBoolean, &["v"]);
        add_simple("isFunction", BuiltInFunc::IsFunction, &["v"]);
        add_simple("isNumber", BuiltInFunc::IsNumber, &["v"]);
        add_simple("isObject", BuiltInFunc::IsObject, &["v"]);
        add_simple("isString", BuiltInFunc::IsString, &["v"]);
        add_simple("length", BuiltInFunc::Length, &["x"]);
        add_simple(
            "objectHasEx",
            BuiltInFunc::ObjectHasEx,
            &["obj", "f", "inc_hidden"],
        );
        add_simple(
            "objectFieldsEx",
            BuiltInFunc::ObjectFieldsEx,
            &["obj", "inc_hidden"],
        );
        add_simple("primitiveEquals", BuiltInFunc::PrimitiveEquals, &["a", "b"]);
        add_simple("equals", BuiltInFunc::Equals, &["a", "b"]);
        add_simple("__compare", BuiltInFunc::Compare, &["v1", "v2"]);
        add_simple(
            "__compare_array",
            BuiltInFunc::CompareArray,
            &["arr1", "arr2"],
        );
        add_simple("exponent", BuiltInFunc::Exponent, &["n"]);
        add_simple("mantissa", BuiltInFunc::Mantissa, &["n"]);
        add_simple("floor", BuiltInFunc::Floor, &["x"]);
        add_simple("ceil", BuiltInFunc::Ceil, &["x"]);
        add_simple("modulo", BuiltInFunc::Modulo, &["a", "b"]);
        add_simple("pow", BuiltInFunc::Pow, &["x", "n"]);
        add_simple("exp", BuiltInFunc::Exp, &["n"]);
        add_simple("log", BuiltInFunc::Log, &["n"]);
        add_simple("sqrt", BuiltInFunc::Sqrt, &["x"]);
        add_simple("sin", BuiltInFunc::Sin, &["x"]);
        add_simple("cos", BuiltInFunc::Cos, &["x"]);
        add_simple("tan", BuiltInFunc::Tan, &["x"]);
        add_simple("asin", BuiltInFunc::Asin, &["x"]);
        add_simple("acos", BuiltInFunc::Acos, &["x"]);
        add_simple("atan", BuiltInFunc::Atan, &["x"]);
        add_simple("assertEqual", BuiltInFunc::AssertEqual, &["a", "b"]);
        add_simple("toString", BuiltInFunc::ToString, &["a"]);
        add_simple("codepoint", BuiltInFunc::Codepoint, &["str"]);
        add_simple("char", BuiltInFunc::Char, &["n"]);
        add_simple("substr", BuiltInFunc::Substr, &["str", "from", "len"]);
        add_simple("findSubstr", BuiltInFunc::FindSubstr, &["pat", "str"]);
        add_simple("startsWith", BuiltInFunc::StartsWith, &["a", "b"]);
        add_simple("endsWith", BuiltInFunc::EndsWith, &["a", "b"]);
        add_simple("split", BuiltInFunc::Split, &["str", "c"]);
        add_simple(
            "splitLimit",
            BuiltInFunc::SplitLimit,
            &["str", "c", "maxsplits"],
        );
        add_simple(
            "splitLimitR",
            BuiltInFunc::SplitLimitR,
            &["str", "c", "maxsplits"],
        );
        add_simple(
            "strReplace",
            BuiltInFunc::StrReplace,
            &["str", "from", "to"],
        );
        add_simple("asciiUpper", BuiltInFunc::AsciiUpper, &["str"]);
        add_simple("asciiLower", BuiltInFunc::AsciiLower, &["str"]);
        add_simple("stringChars", BuiltInFunc::StringChars, &["str"]);
        add_simple("format", BuiltInFunc::Format, &["str", "vals"]);
        add_simple("escapeStringJson", BuiltInFunc::EscapeStringJson, &["str"]);
        add_simple("escapeStringBash", BuiltInFunc::EscapeStringBash, &["str"]);
        add_simple(
            "escapeStringDollars",
            BuiltInFunc::EscapeStringDollars,
            &["str"],
        );
        add_simple("escapeStringXML", BuiltInFunc::EscapeStringXml, &["str"]);
        add_simple("parseInt", BuiltInFunc::ParseInt, &["str"]);
        add_simple("parseOctal", BuiltInFunc::ParseOctal, &["str"]);
        add_simple("parseHex", BuiltInFunc::ParseHex, &["str"]);
        add_simple("parseJson", BuiltInFunc::ParseJson, &["str"]);
        add_simple("parseYaml", BuiltInFunc::ParseYaml, &["str"]);
        add_simple("encodeUTF8", BuiltInFunc::EncodeUtf8, &["str"]);
        add_simple("decodeUTF8", BuiltInFunc::DecodeUtf8, &["arr"]);
        add_simple("manifestIni", BuiltInFunc::ManifestIni, &["ini"]);
        add_simple("manifestPython", BuiltInFunc::ManifestPython, &["v"]);
        add_simple(
            "manifestXmlJsonml",
            BuiltInFunc::ManifestXmlJsonml,
            &["value"],
        );
        add_simple(
            "manifestTomlEx",
            BuiltInFunc::ManifestTomlEx,
            &["value", "indent"],
        );
        add_simple(
            "manifestPythonVars",
            BuiltInFunc::ManifestPythonVars,
            &["conf"],
        );
        add_simple("makeArray", BuiltInFunc::MakeArray, &["sz", "func"]);
        add_simple("member", BuiltInFunc::Member, &["arr", "x"]);
        add_simple("count", BuiltInFunc::Count, &["arr", "x"]);
        add_simple("find", BuiltInFunc::Find, &["value", "find"]);
        add_simple("filter", BuiltInFunc::Filter, &["func", "arr"]);
        add_simple("foldl", BuiltInFunc::Foldl, &["func", "arr", "init"]);
        add_simple("foldr", BuiltInFunc::Foldr, &["func", "arr", "init"]);
        add_simple("range", BuiltInFunc::Range, &["from", "to"]);
        add_simple(
            "slice",
            BuiltInFunc::Slice,
            &["indexable", "index", "end", "step"],
        );
        add_simple("join", BuiltInFunc::Join, &["sep", "arr"]);
        add_simple("reverse", BuiltInFunc::Reverse, &["arr"]);
        add_simple("all", BuiltInFunc::All, &["arr"]);
        add_simple("any", BuiltInFunc::Any, &["arr"]);
        add_simple("base64", BuiltInFunc::Base64, &["input"]);
        add_simple(
            "base64DecodeBytes",
            BuiltInFunc::Base64DecodeBytes,
            &["str"],
        );
        add_simple("base64Decode", BuiltInFunc::Base64Decode, &["str"]);
        add_simple("md5", BuiltInFunc::Md5, &["str"]);
        add_simple("native", BuiltInFunc::Native, &["name"]);
        add_simple("trace", BuiltInFunc::Trace, &["str", "rest"]);
        add_simple("mod", BuiltInFunc::Mod, &["a", "b"]);

        let mut add_with_defaults =
            |name: &str, kind: BuiltInFunc, params: &[(&str, Option<ir::RcExpr>)]| {
                add_builtin_func(
                    self.intern_str(name),
                    kind,
                    params
                        .iter()
                        .map(|(s, e)| (self.intern_str(s), e.clone()))
                        .collect(),
                );
            };

        let identity_func = ir::RcExpr::new(ir::Expr::IdentityFunc);

        add_with_defaults(
            "manifestJsonEx",
            BuiltInFunc::ManifestJsonEx,
            &[
                ("value", None),
                ("indent", None),
                (
                    "newline",
                    Some(ir::RcExpr::new(ir::Expr::String("\n".into()))),
                ),
                (
                    "key_val_sep",
                    Some(ir::RcExpr::new(ir::Expr::String(": ".into()))),
                ),
            ],
        );
        add_with_defaults(
            "manifestYamlDoc",
            BuiltInFunc::ManifestYamlDoc,
            &[
                ("value", None),
                ("indent_array_in_object", Some(self.false_expr.clone())),
                ("quote_keys", Some(self.true_expr.clone())),
            ],
        );
        add_with_defaults(
            "manifestYamlStream",
            BuiltInFunc::ManifestYamlStream,
            &[
                ("value", None),
                ("indent_array_in_object", Some(self.false_expr.clone())),
                ("c_document_end", Some(self.true_expr.clone())),
                ("quote_keys", Some(self.true_expr.clone())),
            ],
        );
        add_with_defaults(
            "sort",
            BuiltInFunc::Sort,
            &[("arr", None), ("keyF", Some(identity_func.clone()))],
        );
        add_with_defaults(
            "uniq",
            BuiltInFunc::Uniq,
            &[("arr", None), ("keyF", Some(identity_func.clone()))],
        );
        add_with_defaults(
            "set",
            BuiltInFunc::Set,
            &[("arr", None), ("keyF", Some(identity_func.clone()))],
        );
        add_with_defaults(
            "setInter",
            BuiltInFunc::SetInter,
            &[
                ("a", None),
                ("b", None),
                ("keyF", Some(identity_func.clone())),
            ],
        );
        add_with_defaults(
            "setUnion",
            BuiltInFunc::SetUnion,
            &[
                ("a", None),
                ("b", None),
                ("keyF", Some(identity_func.clone())),
            ],
        );
        add_with_defaults(
            "setDiff",
            BuiltInFunc::SetDiff,
            &[
                ("a", None),
                ("b", None),
                ("keyF", Some(identity_func.clone())),
            ],
        );
        add_with_defaults(
            "setMember",
            BuiltInFunc::SetMember,
            &[("x", None), ("arr", None), ("keyF", Some(identity_func))],
        );

        ObjectData::new_simple(extra_fields)
    }

    pub(super) fn make_custom_stdlib(&mut self, this_file: &str) -> Gc<ObjectData> {
        let stdlib_obj = self.stdlib_obj.as_ref().unwrap().clone();

        let mut extra_fields = FHashMap::default();
        extra_fields.insert(
            self.intern_str("thisFile"),
            ObjectField {
                base_env: None,
                visibility: ast::Visibility::Hidden,
                expr: None,
                thunk: OnceCell::from(
                    self.gc_alloc(ThunkData::new_done(ValueData::String(this_file.into()))),
                ),
            },
        );

        let extra_obj = ObjectData::new_simple(extra_fields);

        self.extend_object(&stdlib_obj, &extra_obj)
    }
}
