std.assertEqual(std.removeAt([], 0), []) &&
std.assertEqual(std.removeAt([], -1), []) &&
std.assertEqual(std.removeAt([], 1), []) &&
std.assertEqual(std.removeAt(["x"], 0), []) &&
std.assertEqual(std.removeAt(["x"], -1), ["x"]) &&
std.assertEqual(std.removeAt(["x"], 1), ["x"]) &&
std.assertEqual(std.removeAt(["a", "b", "c"], 0), ["b", "c"]) &&
std.assertEqual(std.removeAt(["a", "b", "c"], 1), ["a", "c"]) &&
std.assertEqual(std.removeAt(["a", "b", "c"], 2), ["a", "b"]) &&

true
