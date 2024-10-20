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
    "multi\nline 1",
    "multi\nline 2\n",
  ]) + "\n",
  |||
    - "multi\nline 1"
    - |
      multi
      line 2
  |||,
) &&

std.assertEqual(
  std.manifestYamlDoc([
    "null",
    "NULL",
    "true",
    "TRUE",
    "false",
    "FALSE",
    "",
    "~",
    "1.5",
    "0xABC",
    "0xEFG",
  ]) + "\n",
  |||
    - "null"
    - "NULL"
    - "true"
    - "TRUE"
    - "false"
    - "FALSE"
    - ""
    - "~"
    - "1.5"
    - "0xABC"
    - "0xEFG"
  |||,
) &&

std.assertEqual(
  std.manifestYamlDoc({ a: 1, b: 2, c: 3 }) + "\n",
  |||
    "a": 1
    "b": 2
    "c": 3
  |||,
) &&
std.assertEqual(
  std.manifestYamlDoc({ a: 1, b: 2, c: 3 }, quote_keys=true) + "\n",
  |||
    "a": 1
    "b": 2
    "c": 3
  |||,
) &&
std.assertEqual(
  std.manifestYamlDoc({ a: 1, b: 2, c: 3 }, quote_keys=false) + "\n",
  |||
    a: 1
    b: 2
    c: 3
  |||,
) &&

std.assertEqual(
  std.manifestYamlDoc({
    "null": 1,
    "NULL": 2,
    "true": 3,
    "TRUE": 4,
    "false": 5,
    "FALSE": 6,
    "": 7,
    "~": 8,
  }, quote_keys=false) + "\n",
  |||
    "": 7
    "FALSE": 6
    "NULL": 2
    "TRUE": 4
    "false": 5
    "null": 1
    "true": 3
    "~": 8
  |||,
) &&

std.assertEqual(
  std.manifestYamlDoc({ a: 1, b: [2, 3], c: 4 }) + "\n",
  |||
    "a": 1
    "b":
    - 2
    - 3
    "c": 4
  |||,
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
  std.manifestYamlDoc([{ a: 1, b: 2 }, { c: 3, d: 4 }]) + "\n",
  |||
    - "a": 1
      "b": 2
    - "c": 3
      "d": 4
  |||,
) &&


true
