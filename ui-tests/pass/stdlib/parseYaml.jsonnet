std.assertEqual(std.parseYaml("null"), null) &&
std.assertEqual(std.parseYaml("Null"), null) &&
std.assertEqual(std.parseYaml("NULL"), null) &&
std.assertEqual(std.parseYaml("~"), null) &&

std.assertEqual(std.parseYaml("true"), true) &&
std.assertEqual(std.parseYaml("True"), true) &&
std.assertEqual(std.parseYaml("TRUE"), true) &&

std.assertEqual(std.parseYaml("false"), false) &&
std.assertEqual(std.parseYaml("False"), false) &&
std.assertEqual(std.parseYaml("FALSE"), false) &&

std.assertEqual(std.parseYaml("0"), 0) &&
std.assertEqual(std.parseYaml("+0"), 0) &&
std.assertEqual(std.parseYaml("-0"), 0) &&
std.assertEqual(std.parseYaml("00"), 0) &&
std.assertEqual(std.parseYaml("+00"), 0) &&
std.assertEqual(std.parseYaml("-00"), 0) &&
std.assertEqual(std.parseYaml("8"), 8) &&
std.assertEqual(std.parseYaml("+8"), 8) &&
std.assertEqual(std.parseYaml("-8"), -8) &&
std.assertEqual(std.parseYaml("08"), 8) &&
std.assertEqual(std.parseYaml("+08"), 8) &&
std.assertEqual(std.parseYaml("-08"), -8) &&
std.assertEqual(std.parseYaml("32"), 32) &&
std.assertEqual(std.parseYaml("+32"), 32) &&
std.assertEqual(std.parseYaml("-32"), -32) &&

std.assertEqual(std.parseYaml("0.5"), 0.5) &&
std.assertEqual(std.parseYaml("+0.5"), 0.5) &&
std.assertEqual(std.parseYaml("-0.5"), -0.5) &&
std.assertEqual(std.parseYaml("00.5"), 0.5) &&
std.assertEqual(std.parseYaml("+00.5"), 0.5) &&
std.assertEqual(std.parseYaml("-00.5"), -0.5) &&
std.assertEqual(std.parseYaml("8.5"), 8.5) &&
std.assertEqual(std.parseYaml("+8.5"), 8.5) &&
std.assertEqual(std.parseYaml("-8.5"), -8.5) &&
std.assertEqual(std.parseYaml("32.5"), 32.5) &&
std.assertEqual(std.parseYaml("+32.5"), 32.5) &&
std.assertEqual(std.parseYaml("-32.5"), -32.5) &&

std.assertEqual(std.parseYaml("0.25"), 0.25) &&
std.assertEqual(std.parseYaml("+0.25"), 0.25) &&
std.assertEqual(std.parseYaml("-0.25"), -0.25) &&
std.assertEqual(std.parseYaml("8.25"), 8.25) &&
std.assertEqual(std.parseYaml("+8.25"), 8.25) &&
std.assertEqual(std.parseYaml("-8.25"), -8.25) &&
std.assertEqual(std.parseYaml("32.25"), 32.25) &&
std.assertEqual(std.parseYaml("+32.25"), 32.25) &&
std.assertEqual(std.parseYaml("-32.25"), -32.25) &&

std.assertEqual(std.parseYaml(".25"), 0.25) &&
std.assertEqual(std.parseYaml("+.25"), 0.25) &&
std.assertEqual(std.parseYaml("-.25"), -0.25) &&
std.assertEqual(std.parseYaml("8."), 8) &&
std.assertEqual(std.parseYaml("+8."), 8) &&
std.assertEqual(std.parseYaml("-8."), -8) &&

std.assertEqual(std.parseYaml("0e2"), 0) &&
std.assertEqual(std.parseYaml("+0e2"), 0) &&
std.assertEqual(std.parseYaml("-0e2"), 0) &&
std.assertEqual(std.parseYaml("8e2"), 800) &&
std.assertEqual(std.parseYaml("+8e2"), 800) &&
std.assertEqual(std.parseYaml("-8e2"), -800) &&
std.assertEqual(std.parseYaml("32e2"), 3200) &&
std.assertEqual(std.parseYaml("+32e2"), 3200) &&
std.assertEqual(std.parseYaml("-32e2"), -3200) &&

std.assertEqual(std.parseYaml("8.5e2"), 850) &&
std.assertEqual(std.parseYaml("8.25e2"), 825) &&

std.assertEqual(std.parseYaml(".25e2"), 25) &&
std.assertEqual(std.parseYaml("+.25e2"), 25) &&
std.assertEqual(std.parseYaml("-.25e2"), -25) &&
std.assertEqual(std.parseYaml("8.e2"), 800) &&
std.assertEqual(std.parseYaml("+8.e2"), 800) &&
std.assertEqual(std.parseYaml("-8.e2"), -800) &&

std.assertEqual(std.parseYaml("8.5e+2"), 850) &&
std.assertEqual(std.parseYaml("25e-2"), 0.25) &&
std.assertEqual(std.parseYaml("0.5e+10"), 5000000000) &&

std.assertEqual(std.parseYaml("0o0"), 0) &&
std.assertEqual(std.parseYaml("0o1"), 1) &&
std.assertEqual(std.parseYaml("0o7"), 7) &&
std.assertEqual(std.parseYaml("0o1234567"), 342391) &&

std.assertEqual(std.parseYaml("0x0"), 0) &&
std.assertEqual(std.parseYaml("0x1"), 1) &&
std.assertEqual(std.parseYaml("0x9"), 9) &&
std.assertEqual(std.parseYaml("0xf"), 15) &&
std.assertEqual(std.parseYaml("0xF"), 15) &&
std.assertEqual(std.parseYaml("0xabcdef"), 11259375) &&
std.assertEqual(std.parseYaml("0xABCDEF"), 11259375) &&

std.assertEqual(
  std.parseYaml("0o123456712345671234567123456712345671234567123456712345671234567"),
  1.281037429041102e+56,
) &&
std.assertEqual(
  std.parseYaml("0x123456789ABCDEF123456789ABCDEF123456789ABCDEF123456789ABCDEF"),
  1.2564245793979622e+71,
) &&
std.assertEqual(
  std.parseYaml("0x123456789abcdef123456789abcdef123456789abcdef123456789abcdef"),
  1.2564245793979622e+71,
) &&

std.assertEqual(std.parseYaml("."), ".") &&
std.assertEqual(std.parseYaml("-."), "-.") &&
std.assertEqual(std.parseYaml("+."), "+.") &&
std.assertEqual(std.parseYaml(".e"), ".e") &&
std.assertEqual(std.parseYaml("-.e"), "-.e") &&
std.assertEqual(std.parseYaml("+.e"), "+.e") &&
std.assertEqual(std.parseYaml(".e1"), ".e1") &&
std.assertEqual(std.parseYaml("-.e1"), "-.e1") &&
std.assertEqual(std.parseYaml("+.e1"), "+.e1") &&

std.assertEqual(std.parseYaml("1e"), "1e") &&
std.assertEqual(std.parseYaml("-1e"), "-1e") &&
std.assertEqual(std.parseYaml("+1e"), "+1e") &&
std.assertEqual(std.parseYaml("1e+"), "1e+") &&
std.assertEqual(std.parseYaml("-1e+"), "-1e+") &&
std.assertEqual(std.parseYaml("+1e+"), "+1e+") &&
std.assertEqual(std.parseYaml("1e-"), "1e-") &&
std.assertEqual(std.parseYaml("-1e-"), "-1e-") &&
std.assertEqual(std.parseYaml("+1e-"), "+1e-") &&

std.assertEqual(std.parseYaml("1.2."), "1.2.") &&
std.assertEqual(std.parseYaml("1.2.3"), "1.2.3") &&

std.assertEqual(std.parseYaml("-1.2."), "-1.2.") &&
std.assertEqual(std.parseYaml("-1.2.3"), "-1.2.3") &&

std.assertEqual(std.parseYaml("+1.2."), "+1.2.") &&
std.assertEqual(std.parseYaml("+1.2.3"), "+1.2.3") &&

std.assertEqual(std.parseYaml("0o"), "0o") &&
std.assertEqual(std.parseYaml("+0o"), "+0o") &&
std.assertEqual(std.parseYaml("-0o"), "-0o") &&

std.assertEqual(std.parseYaml("0o8"), "0o8") &&
std.assertEqual(std.parseYaml("+0o8"), "+0o8") &&
std.assertEqual(std.parseYaml("-0o8"), "-0o8") &&

std.assertEqual(std.parseYaml("+0o1"), "+0o1") &&
std.assertEqual(std.parseYaml("-0o1"), "-0o1") &&

std.assertEqual(std.parseYaml("0x"), "0x") &&
std.assertEqual(std.parseYaml("+0x"), "+0x") &&
std.assertEqual(std.parseYaml("-0x"), "-0x") &&

std.assertEqual(std.parseYaml("0xG"), "0xG") &&
std.assertEqual(std.parseYaml("+0xG"), "+0xG") &&
std.assertEqual(std.parseYaml("-0xG"), "-0xG") &&

std.assertEqual(std.parseYaml("+0x1"), "+0x1") &&
std.assertEqual(std.parseYaml("-0x1"), "-0x1") &&

std.assertEqual(std.parseYaml('"string"'), "string") &&
std.assertEqual(std.parseYaml("'string'"), "string") &&
std.assertEqual(std.parseYaml('"\\uABCD"'), "\uABCD") &&

std.assertEqual(std.parseYaml(">\n x"), "x\n") &&
std.assertEqual(std.parseYaml(">\n x\n"), "x\n") &&
std.assertEqual(std.parseYaml(">\n x\n y"), "x y\n") &&
std.assertEqual(std.parseYaml(">\n x\n y\n"), "x y\n") &&
std.assertEqual(std.parseYaml(">-\n x"), "x") &&
std.assertEqual(std.parseYaml(">-\n x\n"), "x") &&
std.assertEqual(std.parseYaml(">-\n x\n y"), "x y") &&
std.assertEqual(std.parseYaml(">-\n x\n y\n"), "x y") &&

std.assertEqual(std.parseYaml("|\n x"), "x\n") &&
std.assertEqual(std.parseYaml("|\n x\n"), "x\n") &&
std.assertEqual(std.parseYaml("|\n x\n y"), "x\ny\n") &&
std.assertEqual(std.parseYaml("|\n x\n y\n"), "x\ny\n") &&
std.assertEqual(std.parseYaml("|-\n x"), "x") &&
std.assertEqual(std.parseYaml("|-\n x\n"), "x") &&
std.assertEqual(std.parseYaml("|-\n x\n y"), "x\ny") &&
std.assertEqual(std.parseYaml("|-\n x\n y\n"), "x\ny") &&

std.assertEqual(
  std.parseYaml(|||
    x: |
      a
      b
    y: "a\nb"
  |||),
  { x: "a\nb\n", y: "a\nb" },
) &&

std.assertEqual(
  std.parseYaml(
    |||
      - ""
      - "null"
      - "Null"
      - "NULL"
      - "~"
    |||
  ),
  ["", "null", "Null", "NULL", "~"],
) &&

std.assertEqual(
  std.parseYaml(
    |||
      - "true"
      - "True"
      - "TRUE"
    |||
  ),
  ["true", "True", "TRUE"],
) &&

std.assertEqual(
  std.parseYaml(
    |||
      - "false"
      - "False"
      - "FALSE"
    |||
  ),
  ["false", "False", "FALSE"],
) &&

std.assertEqual(
  std.parseYaml(
    |||
      - ---1
      - ...2
    |||
  ),
  ["---1", "...2"],
) &&

std.assertEqual(std.parseYaml("k---: v"), { "k---": "v" }) &&
std.assertEqual(std.parseYaml("---k: v"), { "---k": "v" }) &&
std.assertEqual(std.parseYaml("k...: v"), { "k...": "v" }) &&
std.assertEqual(std.parseYaml("...k: v"), { "...k": "v" }) &&

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
      - "1"
      - "2.2"
      - "3e3"
    |||
  ),
  ["1", "2.2", "3e3"],
) &&

std.assertEqual(
  std.parseYaml(
    |||
      a: null
      "b": [1, 2, 3]
      c:
        - x: 4
        - y: 5
      d: plain string
    |||
  ),
  {
    a: null,
    b: [1, 2, 3],
    c: [{ x: 4 }, { y: 5 }],
    d: "plain string",
  },
) &&

std.assertEqual(
  std.parseYaml(
    |||
      ---
    |||
  ),
  [
    null,
  ],
) &&

std.assertEqual(
  std.parseYaml(
    |||
      ---
      ---
    |||
  ),
  [
    null,
    null,
  ],
) &&

std.assertEqual(
  std.parseYaml(
    |||
      ---
      - 1
      - 2
    |||
  ),
  [
    [1, 2],
  ],
) &&

std.assertEqual(
  std.parseYaml(
    |||
      - 1
      - 2
      ---
    |||
  ),
  [
    [1, 2],
    null,
  ],
) &&

std.assertEqual(
  std.parseYaml(
    |||
      - 1
      - 2
      ---
      a: 3
      b: 4
    |||
  ),
  [
    [1, 2],
    { a: 3, b: 4 },
  ],
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

std.assertEqual(
  std.parseYaml(
    |||
      ---
      - 1
      - 2
      ---
      a: 3
      b: 4
    |||
  ),
  [
    [1, 2],
    { a: 3, b: 4 },
  ],
) &&

std.assertEqual(
  std.parseYaml(
    |||
      ---
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

std.assertEqual(
  std.parseYaml(
    |||
      a: &x 1
      b: *x
    |||
  ),
  { a: 1, b: 1 },
) &&

std.assertEqual(
  std.parseYaml(
    |||
      a: &x [1, 2]
      b: *x
    |||
  ),
  { a: [1, 2], b: [1, 2] },
) &&

std.assertEqual(
  std.parseYaml(
    |||
      a: &x { c: 1, d: 2 }
      b: *x
    |||
  ),
  { a: { c: 1, d: 2 }, b: { c: 1, d: 2 } },
) &&

std.assertEqual(
  std.parseYaml(
    |||
      &x a: 1
      b: *x
    |||
  ),
  { a: 1, b: "a" },
) &&

std.assertEqual(
  std.parseYaml(
    |||
      a: &x 1
      *x : 2
    |||
  ),
  { a: 1, "1": 2 },
) &&

std.assertEqual(
  std.parseYaml(
    |||
      a: &x 1
      b: &y 2
      c: &z
        *x : *y
        *y : *x
      d: *z
    |||
  ),
  { a: 1, b: 2, c: { "1": 2, "2": 1 }, d: { "1": 2, "2": 1 } },
) &&

std.assertEqual(
  std.parseYaml(
    |||
      arr:
        - obj:
            arr:
              - obj:
                  arr:
                    - text: >-
                        A
                        B
    |||
  ),
  { arr: [{ obj: { arr: [{ obj: { arr: [{ text: "A B" }] } }] } }] },
) &&

# Stress test
local num_repeats = 10000;
std.assertEqual(
  std.parseYaml("[" + std.join(",", std.repeat(["{}"], num_repeats)) + "]"),
  std.repeat([{}], num_repeats)
) &&
std.assertEqual(
  std.parseYaml(std.repeat("- {}\n", num_repeats)),
  std.repeat([{}], num_repeats)
) &&

true
