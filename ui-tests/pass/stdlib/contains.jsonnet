std.assertEqual(std.contains([], "x"), false) &&
std.assertEqual(std.contains(["x"], "x"), true) &&
std.assertEqual(std.contains(["x"], "y"), false) &&
std.assertEqual(std.contains(["a", "b", "c"], "x"), false) &&
std.assertEqual(std.contains(["a", "b", "c"], "a"), true) &&
std.assertEqual(std.contains(["a", "b", "c"], "b"), true) &&
std.assertEqual(std.contains(["a", "b", "c"], "c"), true) &&

true
