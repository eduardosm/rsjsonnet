std.assertEqual(std.objectRemoveKey({}, "x"), {}) &&
std.assertEqual(std.objectRemoveKey({ a: 1, b: 2 }, "x"), { a: 1, b: 2 }) &&
std.assertEqual(std.objectRemoveKey({ a: 1, b: 2 }, "a"), { b: 2 }) &&
std.assertEqual(std.objectRemoveKey({ assert true, a: 1, b: 2 }, "a"), { b: 2 }) &&

true
