std.assertEqual(std.map(function(x) error "", []), []) &&
std.assertEqual(std.map(function(x) error "", ""), []) &&

std.assertEqual(std.map(function(x) x * 10, [1, 2, 3]), [10, 20, 30]) &&
std.assertEqual(std.map(function(x) x + ":", "abc"), ["a:", "b:", "c:"]) &&
std.assertEqual(std.map(std.length, [[], [1, 2, 3], [1]]), [0, 3, 1]) &&
std.assertEqual(std.map(std.codepoint, "abc"), [97, 98, 99]) &&
std.assertEqual(std.map(std.codepoint, "🧶🧺🧲🧢"), [129526, 129530, 129522, 129506]) &&

true
