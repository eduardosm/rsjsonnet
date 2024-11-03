std.assertEqual(std.prune(null), null) &&
std.assertEqual(std.prune(true), true) &&
std.assertEqual(std.prune(false), false) &&
std.assertEqual(std.prune(1.5), 1.5) &&
std.assertEqual(std.prune("string"), "string") &&
std.assertEqual(std.prune([]), []) &&
std.assertEqual(std.prune([1, 2]), [1, 2]) &&
std.assertEqual(std.prune({}), {}) &&
std.assertEqual(std.prune({ a: 1, b: 2 }), { a: 1, b: 2 }) &&
std.assertEqual(std.prune(function(x, y) x + y)(1, 2), 3) &&

std.assertEqual(std.prune([0, 1, null]), [0, 1]) &&
std.assertEqual(std.prune([null, 0, 1]), [0, 1]) &&
std.assertEqual(std.prune([true, false]), [true, false]) &&
std.assertEqual(std.prune([-1, 0, 1, 2, 3]), [-1, 0, 1, 2, 3]) &&
std.assertEqual(std.prune(["", "string"]), ["", "string"]) &&
std.assertEqual(std.prune([0, 1, []]), [0, 1]) &&
std.assertEqual(std.prune([[], 0, 1]), [0, 1]) &&
std.assertEqual(std.prune([0, 1, {}]), [0, 1]) &&
std.assertEqual(std.prune([{}, 0, 1]), [0, 1]) &&

std.assertEqual(std.prune({ a: 1, b: null }), { a: 1 }) &&
std.assertEqual(std.prune({ a: null, b: 1 }), { b: 1 }) &&
std.assertEqual(std.prune({ a: true, b: false }), { a: true, b: false }) &&
std.assertEqual(std.prune({ a: 0, b: 1 }), { a: 0, b: 1 }) &&
std.assertEqual(std.prune({ a: "", b: "string" }), { a: "", b: "string" }) &&
std.assertEqual(std.prune({ a: 1, b: [] }), { a: 1 }) &&
std.assertEqual(std.prune({ a: [], b: 1 }), { b: 1 }) &&
std.assertEqual(std.prune({ a: 1, b: {} }), { a: 1 }) &&
std.assertEqual(std.prune({ a: {}, b: 1 }), { b: 1 }) &&

std.assertEqual(std.prune([[[[[[[[[[]]]]]]]]]]), []) &&
std.assertEqual(std.prune([[[[[[null]]]]]]), []) &&
std.assertEqual(std.prune([[[[[[true]]]]]]), [[[[[[true]]]]]]) &&
std.assertEqual(std.prune([[[[[[false]]]]]]), [[[[[[false]]]]]]) &&

std.assertEqual(std.prune({ a: { a: { a: { a: { a: { a: { a: {} } } } } } } }), {}) &&
std.assertEqual(std.prune({ a: { a: { a: null } } }), {}) &&
std.assertEqual(std.prune({ a: { a: { a: true } } }), { a: { a: { a: true } } }) &&
std.assertEqual(std.prune({ a: { a: { a: false } } }), { a: { a: { a: false } } }) &&

std.assertEqual(std.prune({ a: { x: [], y: null }, b: [null, {}, { x: [[[]]] }] }), {}) &&
std.assertEqual(std.prune([null, [], [null, []], { a: [], b: {}, c: null }]), []) &&

true
