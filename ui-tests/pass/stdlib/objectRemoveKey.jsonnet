std.assertEqual(std.objectRemoveKey({}, "x"), {}) &&
std.assertEqual(std.objectRemoveKey({ a: 1, b: 2 }, "x"), { a: 1, b: 2 }) &&
std.assertEqual(std.objectRemoveKey({ a: 1, b: 2 }, "a"), { b: 2 }) &&
std.assertEqual(std.objectRemoveKey({ assert true, a: 1, b: 2 }, "a"), { b: 2 }) &&

// Ensure visibility is preserved

local r = std.objectRemoveKey({ a: 1, b: 2, c:: 3 }, "a");
std.assertEqual(std.objectKeysValues(r), [{ key: "b", value: 2 }]) &&
std.assertEqual(std.objectKeysValuesAll(r), [{ key: "b", value: 2 }, { key: "c", value: 3 }]) &&
std.assertEqual(std.objectKeysValues(r + { c: 4 }), [{ key: "b", value: 2 }]) &&
std.assertEqual(std.objectKeysValuesAll(r + { c: 4 }), [{ key: "b", value: 2 }, { key: "c", value: 4 }]) &&
std.assertEqual(std.objectKeysValues(r + { c::: 4 }), [{ key: "b", value: 2 }, { key: "c", value: 4 }]) &&
std.assertEqual(std.objectKeysValuesAll(r + { c::: 4 }), [{ key: "b", value: 2 }, { key: "c", value: 4 }]) &&

local r = std.objectRemoveKey({ a: 1, b: 2, c::: 3 }, "a");
std.assertEqual(std.objectKeysValues(r), [{ key: "b", value: 2 }, { key: "c", value: 3 }]) &&
std.assertEqual(std.objectKeysValuesAll(r), [{ key: "b", value: 2 }, { key: "c", value: 3 }]) &&
std.assertEqual(std.objectKeysValues({ c:: 4 } + r), [{ key: "b", value: 2 }, { key: "c", value: 3 }]) &&
std.assertEqual(std.objectKeysValuesAll({ c:: 4 } + r), [{ key: "b", value: 2 }, { key: "c", value: 3 }]) &&

true
