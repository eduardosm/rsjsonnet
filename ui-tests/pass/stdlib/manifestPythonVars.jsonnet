std.assertEqual(std.manifestPythonVars({}), "") &&
std.assertEqual(
  std.manifestPythonVars({
    "null": null,
    bool: true,
    number: 1.5,
    string: "string\n",
    array: [1, 2],
    object: { a: 1, b: 2 },
  }),
  |||
    array = [1, 2]
    bool = True
    null = None
    number = 1.5
    object = {"a": 1, "b": 2}
    string = "string\n"
  |||,
) &&

true
