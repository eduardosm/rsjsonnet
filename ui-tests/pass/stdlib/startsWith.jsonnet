std.assertEqual(std.startsWith("string", "abc"), false) &&
std.assertEqual(std.startsWith("string", ""), true) &&
std.assertEqual(std.startsWith("string", "str"), true) &&
std.assertEqual(std.startsWith("string", "string"), true) &&
std.assertEqual(std.startsWith("string", "string1"), false) &&
std.assertEqual(std.startsWith("", "string"), false) &&

true
