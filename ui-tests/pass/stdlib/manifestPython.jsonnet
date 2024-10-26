std.assertEqual(std.manifestPython(null), "None") &&
std.assertEqual(std.manifestPython(true), "True") &&
std.assertEqual(std.manifestPython(false), "False") &&
std.assertEqual(std.manifestPython(0), "0") &&
std.assertEqual(std.manifestPython(-0), "-0") &&
std.assertEqual(std.manifestPython(1.5), "1.5") &&
std.assertEqual(std.manifestPython(-1.5), "-1.5") &&
std.assertEqual(std.manifestPython("string"), '"string"') &&
std.assertEqual(std.manifestPython("\n"), @'"\n"') &&

std.assertEqual(std.manifestPython([]), "[]") &&
std.assertEqual(std.manifestPython([1]), "[1]") &&
std.assertEqual(std.manifestPython([1, 2]), "[1, 2]") &&
std.assertEqual(std.manifestPython([[]]), "[[]]") &&
std.assertEqual(std.manifestPython([{}]), "[{}]") &&

std.assertEqual(std.manifestPython({}), "{}") &&
std.assertEqual(std.manifestPython({ a: 1 }), @'{"a": 1}') &&
std.assertEqual(std.manifestPython({ a: 1, b: 2 }), @'{"a": 1, "b": 2}') &&
std.assertEqual(std.manifestPython({ a: {} }), @'{"a": {}}') &&
std.assertEqual(std.manifestPython({ a: [] }), @'{"a": []}') &&
std.assertEqual(std.manifestPython({ "\n": 1 }), @'{"\n": 1}') &&

true
