std.assertEqual(std.isEmpty(""), true) &&
std.assertEqual(std.isEmpty("string"), false) &&
std.assertEqual(std.isEmpty("\uFFFF"), false) &&
std.assertEqual(std.isEmpty("\uD800\uDD56"), false) &&

std.assertEqual(std.isEmpty([]), true) &&
std.assertEqual(std.isEmpty([0]), false) &&
std.assertEqual(std.isEmpty([0, 0, 0]), false) &&
std.assertEqual(std.isEmpty([1, 2, 3, 4]), false) &&

std.assertEqual(std.isEmpty({}), true) &&
std.assertEqual(std.isEmpty({ a:: 1 }), true) &&
std.assertEqual(std.isEmpty({ a: 1 }), false) &&
std.assertEqual(std.isEmpty({ a: 1, b:: 2 }), false) &&
std.assertEqual(std.isEmpty({ a: 1, b: 2 }), false) &&
std.assertEqual(std.isEmpty({ a: 1, b: 2 } + { a: 3 }), false) &&
std.assertEqual(std.isEmpty({ a: 1, b: 2 } + { c: 4 }), false) &&
std.assertEqual(std.isEmpty({ a: 1, b: 2 } + { a: 3, c: 4 }), false) &&
std.assertEqual(std.isEmpty(std), true) &&

std.assertEqual(std.isEmpty(function() null), true) &&
std.assertEqual(std.isEmpty(function(x) null), false) &&
std.assertEqual(std.isEmpty(function(x, y) null), false) &&
std.assertEqual(std.isEmpty(function(x=1, y=2) null), false) &&

std.assertEqual(std.isEmpty(std.type), false) &&
std.assertEqual(std.isEmpty(std.sort), false) &&

true
