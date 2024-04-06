std.assertEqual(std.parseYaml("null"), null) &&
std.assertEqual(std.parseYaml("true"), true) &&
std.assertEqual(std.parseYaml("false"), false) &&

std.assertEqual(std.parseYaml("0"), 0) &&
std.assertEqual(std.parseYaml("-0"), 0) &&
std.assertEqual(std.parseYaml("8"), 8) &&
std.assertEqual(std.parseYaml("-8"), -8) &&
std.assertEqual(std.parseYaml("32"), 32) &&
std.assertEqual(std.parseYaml("-32"), -32) &&

std.assertEqual(std.parseYaml("0.5"), 0.5) &&
std.assertEqual(std.parseYaml("-0.5"), -0.5) &&
std.assertEqual(std.parseYaml("8.5"), 8.5) &&
std.assertEqual(std.parseYaml("-8.5"), -8.5) &&
std.assertEqual(std.parseYaml("32.5"), 32.5) &&
std.assertEqual(std.parseYaml("-32.5"), -32.5) &&

std.assertEqual(std.parseYaml("0.25"), 0.25) &&
std.assertEqual(std.parseYaml("-0.25"), -0.25) &&
std.assertEqual(std.parseYaml("8.25"), 8.25) &&
std.assertEqual(std.parseYaml("-8.25"), -8.25) &&
std.assertEqual(std.parseYaml("32.25"), 32.25) &&
std.assertEqual(std.parseYaml("-32.25"), -32.25) &&

std.assertEqual(std.parseYaml("0e2"), 0) &&
std.assertEqual(std.parseYaml("-0e2"), 0) &&
std.assertEqual(std.parseYaml("8e2"), 800) &&
std.assertEqual(std.parseYaml("-8e2"), -800) &&
std.assertEqual(std.parseYaml("32e2"), 3200) &&
std.assertEqual(std.parseYaml("-32e2"), -3200) &&

std.assertEqual(std.parseYaml("8.5e2"), 850) &&
std.assertEqual(std.parseYaml("8.25e2"), 825) &&

std.assertEqual(std.parseYaml("8.5e+2"), 850) &&
std.assertEqual(std.parseYaml("25e-2"), 0.25) &&
std.assertEqual(std.parseYaml("0.5e+10"), 5000000000) &&

std.assertEqual(std.parseYaml('"string"'), "string") &&
std.assertEqual(std.parseYaml("'string'"), "string") &&
std.assertEqual(std.parseJson('"\\uABCD"'), "\uABCD") &&
std.assertEqual(std.parseJson('"\\uD800\\uDD56"'), "\uD800\uDD56") &&

std.assertEqual(std.parseYaml('[]'), []) &&
std.assertEqual(std.parseYaml('[1]'), [1]) &&
std.assertEqual(std.parseYaml('[1, 2]'), [1, 2]) &&
std.assertEqual(std.parseYaml('[1, 2, 3]'), [1, 2, 3]) &&
std.assertEqual(
  std.parseYaml(
    |||
      - 1
      - 2
      - 3
    |||
  ),
  [1, 2, 3],
) &&

std.assertEqual(std.parseYaml('{}'), {}) &&
std.assertEqual(std.parseYaml('{"a": 1}'), { a: 1 }) &&
std.assertEqual(std.parseYaml('{"a": 1, "b": 2}'), { a: 1, b: 2 }) &&
std.assertEqual(std.parseYaml('{"a": 1, "b": 2, "c": 3}'), { a: 1, b: 2, c: 3 }) &&
std.assertEqual(
  std.parseYaml(
    |||
      a: 1
      b: 2
      c: 3
    |||
  ),
  { a: 1, b: 2, c: 3 },
) &&

std.assertEqual(
  std.parseYaml(
    |||
      a: null
      "b": [1, 2, 3]
      c:
        - x: 4
        - y: 5
    |||
  ),
  {
    a: null,
    b: [1, 2, 3],
    c: [{ x: 4 }, { y: 5 }],
  },
) &&

std.assertEqual(
  std.parseYaml(
    |||
      - 1
      - 2
      ---
      a: 3
      b: 4
      ...
    |||
  ),
  [
    [1, 2],
    { a: 3, b: 4 },
  ],
) &&

true
