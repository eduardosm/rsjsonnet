std.assertEqual(std.resolvePath("", "x"), "x") &&
std.assertEqual(std.resolvePath("a", "x"), "x") &&
std.assertEqual(std.resolvePath("a/", "x"), "a/x") &&
std.assertEqual(std.resolvePath("a/b", "x"), "a/x") &&
std.assertEqual(std.resolvePath("a/b/", "x"), "a/b/x") &&
std.assertEqual(std.resolvePath("a/b/c", "x"), "a/b/x") &&

true
