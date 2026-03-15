std.assertEqual(std.objectRemoveKey({}, "x"), {}) &&
std.assertEqual(std.objectRemoveKey({ a: 1, b: 2 }, "x"), { a: 1, b: 2 }) &&
std.assertEqual(std.objectRemoveKey({ a: 1, b: 2 }, "a"), { b: 2 }) &&
std.assertEqual(std.objectRemoveKey({ assert true, a: 1, b: 2 }, "a"), { b: 2 }) &&

std.assertEqual(std.length(std.objectRemoveKey({ assert false, x: 1, y: 2 }, "x")), 1) &&

std.assertEqual({ a: 1, b: 2 } + std.objectRemoveKey({ a: 3, b: 4 }, "b"), { a: 3, b: 2 }) &&
std.assertEqual(std.objectRemoveKey({ a: 1, b: 2 }, "b") + { a: 3, b: 4 }, { a: 3, b: 4 }) &&
std.assertEqual(std.objectRemoveKey({ a: 1, b: 2 } + { a: 3, b: 4 }, "b"), { a: 3 }) &&
std.assertEqual({ b: 0 } + std.objectRemoveKey({ a: 1, b: 2 } + { a: 3, b: 4 }, "b"), { a: 3, b: 0 }) &&
std.assertEqual(std.objectRemoveKey({ a: 1, b: 2 } + { a: 3, b: 4 }, "b") + { b: 0 }, { a: 3, b: 0 }) &&
std.assertEqual(std.objectRemoveKey({ a: 1, b: 2 } + { a: 3, b: super.a }, "a"), { b: 1 }) &&
std.assertEqual({ x: 2 } + std.objectRemoveKey({ a: 1, b: super.x }, "a"), { b: 2, x: 2 }) &&
std.assertEqual({ a: 2 } + std.objectRemoveKey({ a: 1, b: super.a }, "a"), { a: 2, b: 2 }) &&

std.assertEqual(std.objectHas({ a: 1, b: 2 } + std.objectRemoveKey({ a: 3, b: 4 }, "b"), "b"), true) &&
std.assertEqual(std.objectHas(std.objectRemoveKey({ a: 1, b: 2 }, "b") + { a: 3, b: 4 }, "b"), true) &&
std.assertEqual(std.objectHas(std.objectRemoveKey({ a: 1, b: 2 } + { a: 3, b: 4 }, "b"), "b"), false) &&
std.assertEqual(std.objectHas({ b: 0 } + std.objectRemoveKey({ a: 1, b: 2 } + { a: 3, b: 4 }, "b"), "b"), true) &&
std.assertEqual(std.objectHas(std.objectRemoveKey({ a: 1, b: 2 } + { a: 3, b: 4 }, "b") + { b: 0 }, "b"), true) &&

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
