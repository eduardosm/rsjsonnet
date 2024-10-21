std.assertEqual(std.manifestYamlDoc(null), "null") &&
std.assertEqual(std.manifestYamlDoc(true), "true") &&
std.assertEqual(std.manifestYamlDoc(false), "false") &&
std.assertEqual(std.manifestYamlDoc(1.5), "1.5") &&
std.assertEqual(std.manifestYamlDoc(-1.5), "-1.5") &&
std.assertEqual(std.manifestYamlDoc("string"), "\"string\"") &&
std.assertEqual(std.manifestYamlDoc([]), "[]") &&
std.assertEqual(std.manifestYamlDoc({}), "{}") &&
std.assertEqual(std.manifestYamlDoc("multi\nline"), "\"multi\\nline\"") &&
std.assertEqual(
  std.manifestYamlDoc("multi\nline\n") + "\n",
  |||
    |
      multi
      line
  |||,
) &&

std.assertEqual(
  std.manifestYamlDoc([
    null,
    true,
    false,
    1.5,
    -1.5,
    "string",
    [],
    {},
  ]) + "\n",
  |||
    - null
    - true
    - false
    - 1.5
    - -1.5
    - "string"
    - []
    - {}
  |||,
) &&

std.assertEqual(
  std.manifestYamlDoc([
    "\n",
    "multi\nline 1",
    "multi\nline 2\n",
  ]) + "\n",
  |||
    - |
      
    - "multi\nline 1"
    - |
      multi
      line 2
  |||,
) &&

std.assertEqual(
  std.manifestYamlDoc([
    ["multi\nline 1\n"],
    { x: "multi\nline 2\n" },
  ]) + "\n",
  |||
    -
      - |
        multi
        line 1
    - "x": |
        multi
        line 2
  |||,
) &&

std.assertEqual(
  std.manifestYamlDoc({ a: 1, b: [2, 3], c: 4 }) + "\n",
  std.manifestYamlDoc({ a: 1, b: [2, 3], c: 4 }, indent_array_in_object=false) + "\n",
) &&
std.assertEqual(
  std.manifestYamlDoc({ a: 1, b: [2, 3], c: 4 }, indent_array_in_object=true) + "\n",
  |||
    "a": 1
    "b":
      - 2
      - 3
    "c": 4
  |||,
) &&
std.assertEqual(
  std.manifestYamlDoc({ a: 1, b: [2, 3], c: 4 }, indent_array_in_object=false) + "\n",
  |||
    "a": 1
    "b":
    - 2
    - 3
    "c": 4
  |||,
) &&

std.assertEqual(
  std.manifestYamlDoc([[1, 2], [3, 4]]) + "\n",
  |||
    -
      - 1
      - 2
    -
      - 3
      - 4
  |||,
) &&
std.assertEqual(
  std.manifestYamlDoc([[[[]]]]) + "\n",
  |||
    -
      -
        - []
  |||,
) &&
std.assertEqual(
  std.manifestYamlDoc([{ a: 1, b: 2 }, { c: 3, d: 4 }]) + "\n",
  |||
    - "a": 1
      "b": 2
    - "c": 3
      "d": 4
  |||,
) &&
std.assertEqual(
  std.manifestYamlDoc({ a: { b: 1, c: 2 }, d: { e: 3, f: 4 } }) + "\n",
  |||
    "a":
      "b": 1
      "c": 2
    "d":
      "e": 3
      "f": 4
  |||,
) &&
std.assertEqual(
  std.manifestYamlDoc({ a: { b: 1, c: 2 }, d: { e: 3, f: 4 } }, quote_keys=false) + "\n",
  |||
    a:
      b: 1
      c: 2
    d:
      e: 3
      f: 4
  |||,
) &&

local quoted_keys = [
  "",
  "~",
  "1234",
  "+1234",
  "-1234",
  "1_2_3_4",
  "+1_2_3_4",
  "-1_2_3_4",
  "0",
  "+0",
  "-0",
  "00",
  "+00",
  "-00",
  "0b101010",
  "+0b101010",
  "-0b101010",
  "0b10_10_10",
  "+0b10_10_10",
  "-0b10_10_10",
  "0x123ABC",
  "+0x123ABC",
  "-0x123ABC",
  "0x12_3A_BC",
  "+0x12_3A_BC",
  "-0x12_3A_BC",
  "123:45:67.89",
  "+123:45:67.89",
  "-123:45:67.89",
  "0.0",
  "+0.0",
  "-0.0",
  "12.34",
  "+12.34",
  "-12.34",
  "1_2.3_4",
  "+1_2.3_4",
  "-1_2.3_4",
  "+1e2",
  "1e+2",
  "1.2e3",
  "+1.2e3",
  "-1.2e3",
  "1.2e+3",
  "1.2e-3",
  "+1E2",
  "1E+2",
  "1.2E3",
  "+1.2E3",
  "-1.2E3",
  "1.2E+3",
  "1.2E-3",
  ".1",
  "+.1",
  "-.1",
  ".1e2",
  "+.1e2",
  "-.1e2",
  ".1e+2",
  ".1e-2",
  ".1E2",
  "+.1E2",
  "-.1E2",
  ".1E+2",
  ".1E-2",
  ".inf",
  ".Inf",
  ".INF",
  "+.inf",
  "+.Inf",
  "+.INF",
  "-.inf",
  "-.Inf",
  "-.INF",
  ".nan",
  ".NaN",
  ".NAN",
  "1234-45-78",
  "1234-45-78 12:34:56",
  "1234-45-78 12:34:56.78",
  "1234-45-78t12:34:56",
  "1234-45-78t12:34:56.78",
  "1234-45-78T12:34:56",
  "1234-45-78T12:34:56.78",
  "1234-45-78T12:34:56Z",
  "1234-45-78T12:34:56.78Z",
  "1234-45-78T12:34:56+12:34",
  "1234-45-78T12:34:56.78+12:34",
  "1234-45-78T12:34:56-12:34",
  "1234-45-78T12:34:56.78-12:34",
  "1234-45-78T12:34:56 +12:34",
  "1234-45-78T12:34:56.78 +12:34",
  "1234-45-78T12:34:56 -12:34",
  "1234-45-78T12:34:56.78 -12:34",
  "__+1",
  "__-1",
  "null",
  "Null",
  "NULL",
  "false",
  "False",
  "FALSE",
  "true",
  "True",
  "TRUE",
  "n",
  "N",
  "no",
  "No",
  "NO",
  "y",
  "Y",
  "yes",
  "Yes",
  "YES",
  "off",
  "Off",
  "OFF",
  "on",
  "On",
  "ON",
  "0-1-2",
  ".",
  "-",
  "--",
  "---",
  "+",
  " ",
  "a b",
];
std.all([
  local doc = { object: { [key]: "key is " + key } };
  std.assertEqual(
    std.manifestYamlDoc(doc),
    std.manifestYamlDoc(doc, quote_keys=true),
  ) &&
  std.assertEqual(
    std.manifestYamlDoc(doc, quote_keys=false) + "\n",
    |||
      object:
        "%(key)s": "key is %(key)s"
    ||| % { key: key }
  ) &&
  std.assertEqual(
    std.manifestYamlDoc(doc, quote_keys=true) + "\n",
    |||
      "object":
        "%(key)s": "key is %(key)s"
    ||| % { key: key }
  ) &&
  true

  for key in quoted_keys
]) &&

local plain_keys = [
  "abc",
  "a_b_c",
  "a.b.c",
  "a-b-c",
  "a/b/c",
  "0.1.2",
  "0/1/2",
  "0-1-2-3",
  "..",
  "...",
  "1e2",
  "1e-2",
  "-1e2",
  "1E2",
  "1E-2",
  "-1E2",
  "0X123ABC",
  "0B101010",
];
std.all([
  local doc = { object: { [key]: "key is " + key } };
  std.assertEqual(
    std.manifestYamlDoc(doc),
    std.manifestYamlDoc(doc, quote_keys=true),
  ) &&
  std.assertEqual(
    std.manifestYamlDoc(doc, quote_keys=false) + "\n",
    |||
      object:
        %(key)s: "key is %(key)s"
    ||| % { key: key }
  ) &&
  std.assertEqual(
    std.manifestYamlDoc(doc, quote_keys=true) + "\n",
    |||
      "object":
        "%(key)s": "key is %(key)s"
    ||| % { key: key }
  ) &&
  true

  for key in plain_keys
]) &&

local escaped_keys = {
  "\t": "tab",
  "\"": "quote",
  "\\": "backslash",
};
std.assertEqual(
  std.manifestYamlDoc(escaped_keys, quote_keys=true),
  std.manifestYamlDoc(escaped_keys, quote_keys=false),
) &&
std.assertEqual(
  std.manifestYamlDoc(escaped_keys, quote_keys=false) + "\n",
  |||
    "\t": "tab"
    "\"": "quote"
    "\\": "backslash"
  |||,
) &&

true
