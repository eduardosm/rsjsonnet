std.assertEqual(std.deepJoin(""), "") &&
std.assertEqual(std.deepJoin("abc"), "abc") &&
std.assertEqual(std.deepJoin([]), "") &&
std.assertEqual(std.deepJoin([[]]), "") &&
std.assertEqual(std.deepJoin([[[]]]), "") &&
std.assertEqual(std.deepJoin([[], [[]]]), "") &&
std.assertEqual(std.deepJoin(["abc"]), "abc") &&
std.assertEqual(std.deepJoin(["a", "b", "c"]), "abc") &&
std.assertEqual(std.deepJoin([["a", ["b"]], [], "c"]), "abc") &&

true
