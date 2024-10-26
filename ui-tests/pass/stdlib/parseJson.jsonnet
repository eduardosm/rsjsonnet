std.assertEqual(std.parseJson("null"), null) &&
std.assertEqual(std.parseJson(" null "), null) &&

std.assertEqual(std.parseJson("true"), true) &&
std.assertEqual(std.parseJson(" true "), true) &&

std.assertEqual(std.parseJson("false"), false) &&
std.assertEqual(std.parseJson(" false "), false) &&

std.assertEqual(std.parseJson("1"), 1) &&
std.assertEqual(std.parseJson(" 1 "), 1) &&

std.assertEqual(std.parseJson("0"), 0) &&
std.assertEqual(std.parseJson("-0"), 0) &&
std.assertEqual(std.parseJson("8"), 8) &&
std.assertEqual(std.parseJson("-8"), -8) &&
std.assertEqual(std.parseJson("32"), 32) &&
std.assertEqual(std.parseJson("-32"), -32) &&

std.assertEqual(std.parseJson("0.5"), 0.5) &&
std.assertEqual(std.parseJson("-0.5"), -0.5) &&
std.assertEqual(std.parseJson("8.5"), 8.5) &&
std.assertEqual(std.parseJson("-8.5"), -8.5) &&
std.assertEqual(std.parseJson("32.5"), 32.5) &&
std.assertEqual(std.parseJson("-32.5"), -32.5) &&

std.assertEqual(std.parseJson("0.25"), 0.25) &&
std.assertEqual(std.parseJson("-0.25"), -0.25) &&
std.assertEqual(std.parseJson("8.25"), 8.25) &&
std.assertEqual(std.parseJson("-8.25"), -8.25) &&
std.assertEqual(std.parseJson("32.25"), 32.25) &&
std.assertEqual(std.parseJson("-32.25"), -32.25) &&

std.assertEqual(std.parseJson("0e2"), 0) &&
std.assertEqual(std.parseJson("-0e2"), 0) &&
std.assertEqual(std.parseJson("8e2"), 800) &&
std.assertEqual(std.parseJson("-8e2"), -800) &&
std.assertEqual(std.parseJson("32e2"), 3200) &&
std.assertEqual(std.parseJson("-32e2"), -3200) &&

std.assertEqual(std.parseJson("8.5e2"), 850) &&
std.assertEqual(std.parseJson("8.25e2"), 825) &&

std.assertEqual(std.parseJson("8.5e+2"), 850) &&
std.assertEqual(std.parseJson("25e-2"), 0.25) &&
std.assertEqual(std.parseJson("0.5e+10"), 5000000000) &&

std.assertEqual(std.parseJson('"string"'), "string") &&
std.assertEqual(std.parseJson(' "string" '), "string") &&

std.assertEqual(std.parseJson('"\\""'), "\"") &&
std.assertEqual(std.parseJson('"\\\\"'), "\\") &&
std.assertEqual(std.parseJson('"\\/"'), "/") &&
std.assertEqual(std.parseJson('"\\b"'), "\b") &&
std.assertEqual(std.parseJson('"\\f"'), "\f") &&
std.assertEqual(std.parseJson('"\\n"'), "\n") &&
std.assertEqual(std.parseJson('"\\r"'), "\r") &&
std.assertEqual(std.parseJson('"\\t"'), "\t") &&
std.assertEqual(std.parseJson('"\\uABCD"'), "\uABCD") &&
std.assertEqual(std.parseJson('"\\uD800\\uDD56"'), "\uD800\uDD56") &&

std.assertEqual(std.parseJson("[]"), []) &&
std.assertEqual(std.parseJson(" [ ] "), []) &&

std.assertEqual(std.parseJson("[1]"), [1]) &&
std.assertEqual(std.parseJson(" [ 1 ] "), [1]) &&

std.assertEqual(std.parseJson("[1,2]"), [1, 2]) &&
std.assertEqual(std.parseJson(" [ 1 , 2 ] "), [1, 2]) &&

std.assertEqual(std.parseJson("[[1, 2], [3, 4], [5, 6]]"), [[1, 2], [3, 4], [5, 6]]) &&

std.assertEqual(std.parseJson("{}"), {}) &&
std.assertEqual(std.parseJson(" { } "), {}) &&

std.assertEqual(std.parseJson('{"a":1}'), { a: 1 }) &&
std.assertEqual(std.parseJson(' { "a" : 1 } '), { a: 1 }) &&

std.assertEqual(std.parseJson('{"a":1,"b":2}'), { a: 1, b: 2 }) &&
std.assertEqual(std.parseJson(' { "a" : 1 , "b" : 2 } '), { a: 1, b: 2 }) &&

std.assertEqual(
  std.parseJson('{"a": {"x": 1, "y": 2}, "b": {"x": 3, "y": 4}}'),
  { a: { x: 1, y: 2 }, b: { x: 3, y: 4 } },
) &&

# Stress test
local num_repeats = 10000;
std.assertEqual(
  std.parseJson("[" + std.join(",", std.repeat(["{}"], num_repeats)) + "]"),
  std.repeat([{}], num_repeats)
) &&

true
