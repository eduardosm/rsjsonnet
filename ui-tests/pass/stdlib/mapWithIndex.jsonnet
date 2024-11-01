std.assertEqual(std.mapWithIndex(function(i, x) error "", []), []) &&
std.assertEqual(std.mapWithIndex(function(i, x) error "", ""), []) &&

std.assertEqual(std.mapWithIndex(function(i, x) x * 10 + i, [1, 2, 3]), [10, 21, 32]) &&
std.assertEqual(std.mapWithIndex(function(i, x) x + ":" + i, "abc"), ["a:0", "b:1", "c:2"]) &&
std.assertEqual(std.mapWithIndex(function(i, x) [i, x], "🧶🧺🧲🧢"), [[0, "🧶"], [1, "🧺"], [2, "🧲"], [3, "🧢"]]) &&

true
