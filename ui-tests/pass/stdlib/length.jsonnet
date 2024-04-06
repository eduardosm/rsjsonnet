std.assertEqual(std.length(""), 0) &&
std.assertEqual(std.length("string"), 6) &&
std.assertEqual(std.length("\uFFFF"), 1) &&
std.assertEqual(std.length("\uD800\uDD56"), 1) &&

std.assertEqual(std.length([]), 0) &&
std.assertEqual(std.length([0]), 1) &&
std.assertEqual(std.length([0, 0, 0]), 3) &&
std.assertEqual(std.length([1, 2, 3, 4]), 4) &&

std.assertEqual(std.length({}), 0) &&
std.assertEqual(std.length({ a:: 1 }), 0) &&
std.assertEqual(std.length({ a: 1 }), 1) &&
std.assertEqual(std.length({ a: 1, b:: 2 }), 1) &&
std.assertEqual(std.length({ a: 1, b: 2 }), 2) &&
std.assertEqual(std.length({ a: 1, b: 2 } + { a: 3 }), 2) &&
std.assertEqual(std.length({ a: 1, b: 2 } + { c: 4 }), 3) &&
std.assertEqual(std.length({ a: 1, b: 2 } + { a: 3, c: 4 }), 3) &&
std.assertEqual(std.length(std), 0) &&

std.assertEqual(std.length(function() null), 0) &&
std.assertEqual(std.length(function(x) null), 1) &&
std.assertEqual(std.length(function(x, y) null), 2) &&
std.assertEqual(std.length(function(x=1, y=2) null), 2) &&

std.assertEqual(std.length(std.type), 1) &&
std.assertEqual(std.length(std.sort), 2) &&

true
