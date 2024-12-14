std.assertEqual(std.toString(null), "null") &&
std.assertEqual(std.toString(false), "false") &&
std.assertEqual(std.toString(true), "true") &&
std.assertEqual(std.toString(0), "0") &&
std.assertEqual(std.toString(-0), "-0") &&
std.assertEqual(std.toString(1.5), "1.5") &&
std.assertEqual(std.toString(10), "10") &&
std.assertEqual(std.toString(100), "100") &&
std.assertEqual(std.toString(1000), "1000") &&
std.assertEqual(std.toString(10000), "10000") &&
std.assertEqual(std.toString(""), "") &&
std.assertEqual(std.toString("string"), "string") &&
std.assertEqual(std.toString([]), "[ ]") &&
std.assertEqual(std.toString([1]), "[1]") &&
std.assertEqual(std.toString([1, 2]), "[1, 2]") &&
std.assertEqual(std.toString(["\u000F"]), "[\"\\u000f\"]") &&
std.assertEqual(std.toString([[]]), "[[ ]]") &&
std.assertEqual(std.toString({}), "{ }") &&
std.assertEqual(std.toString({ a: 1 }), "{\"a\": 1}") &&
std.assertEqual(std.toString({ a: 1, b: 2 }), "{\"a\": 1, \"b\": 2}") &&
std.assertEqual(std.toString({ a: 1, b:: 2 }), "{\"a\": 1}") &&
std.assertEqual(std.toString({ a: {} }), "{\"a\": { }}") &&

true
