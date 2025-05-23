use std::cell::OnceCell;

use super::{
    ir, BuiltInFunc, FuncData, FuncKind, ObjectData, ObjectField, Program, ThunkData, ValueData,
};
use crate::arena::Arena;
use crate::gc::{Gc, GcContext, GcView};
use crate::interner::{InternedStr, StrInterner};
use crate::span::SpanContextId;
use crate::{ast, FHashMap};

pub(super) const STDLIB_DATA: &[u8] = include_bytes!("std.libsonnet");

impl<'p> Program<'p> {
    pub(super) fn load_stdlib(&mut self, span_ctx: SpanContextId) {
        let stdlib_data = self.stdlib_data;
        let stdlib_thunk = self
            .load_source(span_ctx, stdlib_data, false, "std.libsonnet")
            .expect("failed to load stdlib");
        let stdlib_value = self
            .eval_value_internal(&stdlib_thunk)
            .expect("failed to evaluate stdlib");

        let ValueData::Object(stdlib_obj) = stdlib_value else {
            panic!("stdlib is not an object");
        };
        let stdlib_obj = stdlib_obj.view();

        assert!(self.stdlib_base_obj.is_none());
        self.stdlib_base_obj = Some(stdlib_obj);

        self.maybe_gc();
    }

    pub(super) fn build_stdlib_extra(
        arena: &'p Arena,
        str_interner: &StrInterner<'p>,
        gc_ctx: &GcContext<'p>,
        exprs: &super::Exprs<'p>,
    ) -> FHashMap<InternedStr<'p>, GcView<ThunkData<'p>>> {
        let mut extra_fields = FHashMap::default();

        extra_fields.insert(
            str_interner.intern(arena, "pi"),
            gc_ctx.alloc_view(ThunkData::new_done(ValueData::Number(std::f64::consts::PI))),
        );

        let mut add_builtin_func =
            |name: InternedStr<'p>,
             kind: BuiltInFunc,
             params: Vec<(InternedStr<'p>, Option<&'p ir::Expr<'p>>)>| {
                let prev = extra_fields.insert(
                    name,
                    gc_ctx.alloc_view(ThunkData::new_done(ValueData::Function(gc_ctx.alloc(
                        FuncData::new(arena.alloc_slice(&params), FuncKind::BuiltIn { name, kind }),
                    )))),
                );
                assert!(prev.is_none());
            };

        let mut add_simple = |name: &str, kind: BuiltInFunc, params: &[&str]| {
            add_builtin_func(
                str_interner.intern(arena, name),
                kind,
                params
                    .iter()
                    .map(|&s| (str_interner.intern(arena, s), None))
                    .collect(),
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
        add_simple("prune", BuiltInFunc::Prune, &["a"]);
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
        add_simple(
            "objectRemoveKey",
            BuiltInFunc::ObjectRemoveKey,
            &["obj", "key"],
        );
        add_simple("mapWithKey", BuiltInFunc::MapWithKey, &["func", "obj"]);
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
        add_simple("log2", BuiltInFunc::Log2, &["x"]);
        add_simple("log10", BuiltInFunc::Log10, &["x"]);
        add_simple("sqrt", BuiltInFunc::Sqrt, &["x"]);
        add_simple("sin", BuiltInFunc::Sin, &["x"]);
        add_simple("cos", BuiltInFunc::Cos, &["x"]);
        add_simple("tan", BuiltInFunc::Tan, &["x"]);
        add_simple("asin", BuiltInFunc::Asin, &["x"]);
        add_simple("acos", BuiltInFunc::Acos, &["x"]);
        add_simple("atan", BuiltInFunc::Atan, &["x"]);
        add_simple("atan2", BuiltInFunc::Atan2, &["y", "x"]);
        add_simple("deg2rad", BuiltInFunc::Deg2Rad, &["x"]);
        add_simple("rad2deg", BuiltInFunc::Rad2Deg, &["x"]);
        add_simple("hypot", BuiltInFunc::Hypot, &["a", "b"]);
        add_simple("isEven", BuiltInFunc::IsEven, &["x"]);
        add_simple("isOdd", BuiltInFunc::IsOdd, &["x"]);
        add_simple("isInteger", BuiltInFunc::IsInteger, &["x"]);
        add_simple("isDecimal", BuiltInFunc::IsDecimal, &["x"]);
        add_simple("assertEqual", BuiltInFunc::AssertEqual, &["a", "b"]);
        add_simple("toString", BuiltInFunc::ToString, &["a"]);
        add_simple("codepoint", BuiltInFunc::Codepoint, &["str"]);
        add_simple("char", BuiltInFunc::Char, &["n"]);
        add_simple("substr", BuiltInFunc::Substr, &["str", "from", "len"]);
        add_simple("findSubstr", BuiltInFunc::FindSubstr, &["pat", "str"]);
        add_simple("startsWith", BuiltInFunc::StartsWith, &["a", "b"]);
        add_simple("endsWith", BuiltInFunc::EndsWith, &["a", "b"]);
        add_simple("stripChars", BuiltInFunc::StripChars, &["str", "chars"]);
        add_simple("lstripChars", BuiltInFunc::LStripChars, &["str", "chars"]);
        add_simple("rstripChars", BuiltInFunc::RStripChars, &["str", "chars"]);
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
        add_simple("trim", BuiltInFunc::Trim, &["str"]);
        add_simple(
            "equalsIgnoreCase",
            BuiltInFunc::EqualsIgnoreCase,
            &["str1", "str2"],
        );
        add_simple("asciiUpper", BuiltInFunc::AsciiUpper, &["str"]);
        add_simple("asciiLower", BuiltInFunc::AsciiLower, &["str"]);
        add_simple("stringChars", BuiltInFunc::StringChars, &["str"]);
        add_simple("format", BuiltInFunc::Format, &["str", "vals"]);
        add_simple("escapeStringJson", BuiltInFunc::EscapeStringJson, &["str"]);
        add_simple(
            "escapeStringPython",
            BuiltInFunc::EscapeStringPython,
            &["str"],
        );
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
        add_simple("map", BuiltInFunc::Map, &["func", "arr"]);
        add_simple("mapWithIndex", BuiltInFunc::MapWithIndex, &["func", "arr"]);
        add_simple(
            "filterMap",
            BuiltInFunc::FilterMap,
            &["filter_func", "map_func", "arr"],
        );
        add_simple("flatMap", BuiltInFunc::FlatMap, &["func", "arr"]);
        add_simple("filter", BuiltInFunc::Filter, &["func", "arr"]);
        add_simple("foldl", BuiltInFunc::Foldl, &["func", "arr", "init"]);
        add_simple("foldr", BuiltInFunc::Foldr, &["func", "arr", "init"]);
        add_simple("range", BuiltInFunc::Range, &["from", "to"]);
        add_simple("repeat", BuiltInFunc::Repeat, &["what", "count"]);
        add_simple(
            "slice",
            BuiltInFunc::Slice,
            &["indexable", "index", "end", "step"],
        );
        add_simple("join", BuiltInFunc::Join, &["sep", "arr"]);
        add_simple("deepJoin", BuiltInFunc::DeepJoin, &["arr"]);
        add_simple("flattenArrays", BuiltInFunc::FlattenArrays, &["arrs"]);
        add_simple(
            "flattenDeepArray",
            BuiltInFunc::FlattenDeepArray,
            &["value"],
        );
        add_simple("reverse", BuiltInFunc::Reverse, &["arr"]);
        add_simple("all", BuiltInFunc::All, &["arr"]);
        add_simple("any", BuiltInFunc::Any, &["arr"]);
        add_simple("sum", BuiltInFunc::Sum, &["sum"]);
        add_simple("avg", BuiltInFunc::Avg, &["avg"]);
        add_simple("contains", BuiltInFunc::Contains, &["arr", "elem"]);
        add_simple("remove", BuiltInFunc::Remove, &["arr", "elem"]);
        add_simple("removeAt", BuiltInFunc::RemoveAt, &["arr", "idx"]);
        add_simple("base64", BuiltInFunc::Base64, &["input"]);
        add_simple(
            "base64DecodeBytes",
            BuiltInFunc::Base64DecodeBytes,
            &["str"],
        );
        add_simple("base64Decode", BuiltInFunc::Base64Decode, &["str"]);
        add_simple("md5", BuiltInFunc::Md5, &["str"]);
        add_simple("sha1", BuiltInFunc::Sha1, &["str"]);
        add_simple("sha256", BuiltInFunc::Sha256, &["str"]);
        add_simple("sha512", BuiltInFunc::Sha512, &["str"]);
        add_simple("sha3", BuiltInFunc::Sha3, &["str"]);
        add_simple("mergePatch", BuiltInFunc::MergePatch, &["target", "patch"]);
        add_simple("mod", BuiltInFunc::Mod, &["a", "b"]);
        add_simple("native", BuiltInFunc::Native, &["name"]);
        add_simple("trace", BuiltInFunc::Trace, &["str", "rest"]);

        let mut add_with_defaults =
            |name: &str, kind: BuiltInFunc, params: &[(&str, Option<&'p ir::Expr<'p>>)]| {
                add_builtin_func(
                    str_interner.intern(arena, name),
                    kind,
                    params
                        .iter()
                        .map(|&(s, e)| (str_interner.intern(arena, s), e))
                        .collect(),
                );
            };

        let identity_func = arena.alloc(ir::Expr::IdentityFunc);

        add_with_defaults(
            "manifestJsonEx",
            BuiltInFunc::ManifestJsonEx,
            &[
                ("value", None),
                ("indent", None),
                ("newline", Some(arena.alloc(ir::Expr::String("\n")))),
                ("key_val_sep", Some(arena.alloc(ir::Expr::String(": ")))),
            ],
        );
        add_with_defaults(
            "manifestYamlDoc",
            BuiltInFunc::ManifestYamlDoc,
            &[
                ("value", None),
                ("indent_array_in_object", Some(exprs.false_)),
                ("quote_keys", Some(exprs.true_)),
            ],
        );
        add_with_defaults(
            "manifestYamlStream",
            BuiltInFunc::ManifestYamlStream,
            &[
                ("value", None),
                ("indent_array_in_object", Some(exprs.false_)),
                ("c_document_end", Some(exprs.true_)),
                ("quote_keys", Some(exprs.true_)),
            ],
        );
        add_with_defaults(
            "sort",
            BuiltInFunc::Sort,
            &[("arr", None), ("keyF", Some(identity_func))],
        );
        add_with_defaults(
            "uniq",
            BuiltInFunc::Uniq,
            &[("arr", None), ("keyF", Some(identity_func))],
        );

        let min_max_default_on_empty = arena.alloc(ir::Expr::OtherError {
            msg: arena.alloc_str("expected an array with at least one element"),
        });
        add_with_defaults(
            "minArray",
            BuiltInFunc::MinArray,
            &[
                ("arr", None),
                ("keyF", Some(identity_func)),
                ("onEmpty", Some(min_max_default_on_empty)),
            ],
        );
        add_with_defaults(
            "maxArray",
            BuiltInFunc::MaxArray,
            &[
                ("arr", None),
                ("keyF", Some(identity_func)),
                ("onEmpty", Some(min_max_default_on_empty)),
            ],
        );

        add_with_defaults(
            "set",
            BuiltInFunc::Set,
            &[("arr", None), ("keyF", Some(identity_func))],
        );
        add_with_defaults(
            "setInter",
            BuiltInFunc::SetInter,
            &[("a", None), ("b", None), ("keyF", Some(identity_func))],
        );
        add_with_defaults(
            "setUnion",
            BuiltInFunc::SetUnion,
            &[("a", None), ("b", None), ("keyF", Some(identity_func))],
        );
        add_with_defaults(
            "setDiff",
            BuiltInFunc::SetDiff,
            &[("a", None), ("b", None), ("keyF", Some(identity_func))],
        );
        add_with_defaults(
            "setMember",
            BuiltInFunc::SetMember,
            &[("x", None), ("arr", None), ("keyF", Some(identity_func))],
        );

        extra_fields
    }

    pub(super) fn make_custom_stdlib(&mut self, this_file: &str) -> Gc<ObjectData<'p>> {
        let stdlib_base_obj = self.stdlib_base_obj.as_ref().unwrap().clone();

        let mut extra_fields: FHashMap<_, _> = self
            .stdlib_extra
            .iter()
            .map(|(name, thunk)| {
                (
                    *name,
                    ObjectField {
                        base_env: None,
                        visibility: ast::Visibility::Hidden,
                        expr: None,
                        thunk: OnceCell::from(Gc::from(thunk)),
                    },
                )
            })
            .collect();

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

        self.extend_object(&stdlib_base_obj, &extra_obj)
    }
}
