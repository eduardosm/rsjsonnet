std.assertEqual(std.remove([], "x"), []) &&
std.assertEqual(std.remove(["x"], "x"), []) &&
std.assertEqual(std.remove(["x"], "y"), ["x"]) &&
std.assertEqual(std.remove(["a", "b", "c"], "x"), ["a", "b", "c"]) &&
std.assertEqual(std.remove(["a", "b", "c"], "a"), ["b", "c"]) &&
std.assertEqual(std.remove(["a", "b", "c"], "b"), ["a", "c"]) &&
std.assertEqual(std.remove(["a", "b", "c"], "c"), ["a", "b"]) &&

true
