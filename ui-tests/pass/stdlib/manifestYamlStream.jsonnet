std.assertEqual(
  std.manifestYamlStream([]),
  |||
    ---

    ...
  |||
) &&
std.assertEqual(
  std.manifestYamlStream([], c_document_end=false),
  |||
    ---

  |||
) &&
std.assertEqual(
  std.manifestYamlStream([], c_document_end=true),
  |||
    ---

    ...
  |||
) &&

std.assertEqual(
  std.manifestYamlStream([null, { a: 1, b: 2 }, [3, 4]]),
  |||
    ---
    null
    ---
    "a": 1
    "b": 2
    ---
    - 3
    - 4
    ...
  |||
) &&
std.assertEqual(
  std.manifestYamlStream([null, { a: 1, b: 2 }, [3, 4]], c_document_end=false),
  |||
    ---
    null
    ---
    "a": 1
    "b": 2
    ---
    - 3
    - 4
  |||
) &&
std.assertEqual(
  std.manifestYamlStream([null, { a: 1, b: 2 }, [3, 4]], c_document_end=true),
  |||
    ---
    null
    ---
    "a": 1
    "b": 2
    ---
    - 3
    - 4
    ...
  |||
) &&

std.assertEqual(
  std.manifestYamlStream([{ a: [1 ,2], b: [3, 4] }]),
  |||
    ---
    "a":
    - 1
    - 2
    "b":
    - 3
    - 4
    ...
  |||
) &&
std.assertEqual(
  std.manifestYamlStream([{ a: [1 ,2], b: [3, 4] }], indent_array_in_object=false),
  |||
    ---
    "a":
    - 1
    - 2
    "b":
    - 3
    - 4
    ...
  |||
) &&
std.assertEqual(
  std.manifestYamlStream([{ a: [1 ,2], b: [3, 4] }], indent_array_in_object=true),
  |||
    ---
    "a":
      - 1
      - 2
    "b":
      - 3
      - 4
    ...
  |||
) &&

std.assertEqual(
  std.manifestYamlStream([{ a: 1, b: 2, "null": 3 }]),
  |||
    ---
    "a": 1
    "b": 2
    "null": 3
    ...
  |||
) &&
std.assertEqual(
  std.manifestYamlStream([{ a: 1, b: 2, "null": 3 }], quote_keys=false),
  |||
    ---
    a: 1
    b: 2
    "null": 3
    ...
  |||
) &&
std.assertEqual(
  std.manifestYamlStream([{ a: 1, b: 2, "null": 3 }], quote_keys=true),
  |||
    ---
    "a": 1
    "b": 2
    "null": 3
    ...
  |||
) &&

true
